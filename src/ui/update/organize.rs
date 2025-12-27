//! File organization and undo handlers.

use iced::Task;
use std::path::PathBuf;

use crate::{db, metadata, organizer};

use super::super::messages::Message;
use super::super::state::{LoadedState, OrganizeView};
use super::{load_tracks_task, pick_folder_task};

/// Handle organize-related messages
pub fn handle_organize(s: &mut LoadedState, msg: Message) -> Task<Message> {
    match msg {
        Message::OrganizeDestinationChanged(dest) => {
            s.organize_destination = PathBuf::from(dest);
        }
        Message::OrganizePatternChanged(pattern) => {
            s.organize_pattern = pattern;
        }
        Message::PickOrganizeDestination => {
            return pick_folder_task(Message::OrganizeDestinationPicked);
        }
        Message::OrganizeDestinationPicked(Some(path)) => {
            s.organize_destination = path;
        }
        Message::OrganizePreviewPressed => {
            s.organize_preview.clear();
            s.organize_view = OrganizeView::Preview;
            s.preview_loading = true;
            s.preview_scroll_offset = 0.0;
        }
        Message::OrganizePreviewBatch(batch) => {
            s.organize_preview.extend(batch);
        }
        Message::OrganizePreviewComplete => {
            s.preview_loading = false;
        }
        Message::OrganizeCancelPressed => {
            s.organize_view = OrganizeView::Input;
            s.organize_preview.clear();
            s.preview_loading = false;
        }
        Message::OrganizeConfirmPressed => return start_organize(s),
        Message::OrganizeFileComplete(result) => {
            s.organize_progress += 1;
            if let Err(e) = result {
                s.organize_errors.push(e);
            }
        }
        Message::OrganizeFinished => return finish_organize(s),
        _ => {}
    }
    Task::none()
}

/// Start the organize operation
fn start_organize(s: &mut LoadedState) -> Task<Message> {
    s.organize_view = OrganizeView::Organizing;
    s.organize_progress = 0;
    s.organize_total = s.organize_preview.len();
    s.organize_errors.clear();

    let pool = s.pool.clone();
    let pattern = s.organize_pattern.clone();
    let destination = s.organize_destination.clone();
    let previews = s.organize_preview.clone();

    Task::perform(
        async move {
            let mut undo_log = organizer::UndoLog {
                moves: vec![],
                timestamp: Some(chrono::Utc::now().to_rfc3339()),
            };
            let mut results = vec![];

            for preview in previews {
                let src = preview.source.clone();
                let pat = pattern.clone();
                let dest = destination.clone();

                let res = tokio::task::spawn_blocking(move || {
                    let meta = metadata::read(&src)?;
                    organizer::organize_track(&src, &meta, &pat, &dest).map(|p| (src, p))
                })
                .await;

                match res {
                    Ok(Ok((src, new_path))) => {
                        let path_str = new_path.to_string_lossy().to_string();
                        if let Err(e) =
                            db::update_track_path(&pool, preview.track_id, &path_str).await
                        {
                            results.push(Err(format!("DB error: {}", e)));
                        } else {
                            undo_log.moves.push(organizer::MoveRecord {
                                source: src,
                                destination: new_path,
                                track_id: preview.track_id,
                            });
                            results.push(Ok(()));
                        }
                    }
                    Ok(Err(e)) => results.push(Err(format!("{}: {}", preview.source.display(), e))),
                    Err(e) => results.push(Err(format!("Task error: {}", e))),
                }
            }

            let log = undo_log;
            let _ = tokio::task::spawn_blocking(move || log.save()).await;
            results
        },
        |_| Message::OrganizeFinished,
    )
}

/// Finish the organize operation
fn finish_organize(s: &mut LoadedState) -> Task<Message> {
    let errors = s.organize_errors.len();
    let success = s.organize_total - errors;
    if errors == 0 {
        s.status_message = format!("Organized {} files successfully.", success);
        s.toasts.success(format!("Organized {} files", success));
    } else {
        s.status_message = format!(
            "Organized {} of {} files. {} errors.",
            success, s.organize_total, errors
        );
        s.toasts
            .warning(format!("{} files organized, {} errors", success, errors));
    }
    s.organize_view = OrganizeView::Input;
    s.organize_preview.clear();
    s.can_undo = organizer::UndoLog::has_undo();
    load_tracks_task(s.pool.clone())
}

/// Handle undo-related messages
pub fn handle_undo(s: &mut LoadedState, msg: Message) -> Task<Message> {
    match msg {
        Message::UndoPressed => {
            s.status_message = "Undoing last organize...".to_string();
            let pool = s.pool.clone();
            Task::perform(
                async move {
                    let log = tokio::task::spawn_blocking(organizer::UndoLog::load)
                        .await
                        .map_err(|e| format!("Task error: {}", e))?;

                    let Some(log) = log else {
                        return Err("No undo history available".to_string());
                    };

                    let mut count = 0;
                    for rec in &log.moves {
                        let r = rec.clone();
                        if let Ok(Ok(())) =
                            tokio::task::spawn_blocking(move || organizer::undo_move(&r)).await
                        {
                            let _ = db::update_track_path(
                                &pool,
                                rec.track_id,
                                &rec.source.to_string_lossy(),
                            )
                            .await;
                            count += 1;
                        }
                    }
                    let _ = tokio::task::spawn_blocking(organizer::UndoLog::clear).await;
                    Ok(count)
                },
                Message::UndoComplete,
            )
        }
        Message::UndoComplete(result) => {
            match result {
                Ok(n) => {
                    s.status_message = format!("Undo complete. Restored {} files.", n);
                    s.toasts.success(format!("Restored {} files", n));
                    s.can_undo = false;
                }
                Err(e) => {
                    s.status_message = format!("Undo failed: {}", e);
                    s.toasts.error("Undo failed");
                }
            }
            load_tracks_task(s.pool.clone())
        }
        _ => Task::none(),
    }
}
