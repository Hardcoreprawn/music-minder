//! Library scanning handler.

use iced::Task;

use crate::library;

use super::super::messages::Message;
use super::super::state::LoadedState;
use super::load_tracks_task;

/// Handle scan-related messages
pub fn handle_scan(s: &mut LoadedState, msg: &Message) -> Task<Message> {
    match msg {
        Message::ScanPressed => {
            s.is_scanning = true;
            s.scan_count = 0;
            s.status_message = "Scanning...".to_string();
            Task::none()
        }
        Message::ScanStopped => {
            s.is_scanning = false;
            s.status_message = "Scan stopped by user.".to_string();
            load_tracks_task(s.pool.clone())
        }
        Message::ScanFinished => {
            s.is_scanning = false;
            s.status_message = format!("Scan Complete. Processed {} files.", s.scan_count);
            load_tracks_task(s.pool.clone())
        }
        Message::ScanEventReceived(event) => {
            match event {
                library::ScanEvent::Processed(path) => {
                    s.scan_count += 1;
                    s.status_message = format!(
                        "Scanned {} files. Last: {:?}",
                        s.scan_count,
                        path.file_name().unwrap_or_default()
                    );
                }
                library::ScanEvent::Error(path, err) => {
                    s.status_message = format!("Error scanning {:?}: {}", path, err);
                }
            }
            Task::none()
        }
        _ => Task::none(),
    }
}
