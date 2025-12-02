//! Play queue management.

use std::path::PathBuf;
use super::state::TrackInfo;

/// A single item in the play queue.
#[derive(Debug, Clone)]
pub struct QueueItem {
    /// Path to the audio file
    pub path: PathBuf,
    /// Cached metadata (populated after loading)
    pub info: Option<TrackInfo>,
}

impl QueueItem {
    /// Create a queue item from a file path.
    pub fn from_path(path: PathBuf) -> Self {
        Self { path, info: None }
    }

    /// Create a queue item with metadata.
    pub fn with_info(path: PathBuf, info: TrackInfo) -> Self {
        Self { path, info: Some(info) }
    }

    /// Get the display title.
    pub fn display_title(&self) -> String {
        self.info
            .as_ref()
            .and_then(|i| i.title.clone())
            .unwrap_or_else(|| {
                self.path
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| "Unknown".to_string())
            })
    }
}

/// The play queue with current position tracking.
#[derive(Debug, Clone)]
pub struct PlayQueue {
    /// All items in the queue
    items: Vec<QueueItem>,
    /// Current position in the queue (-1 = not started)
    position: i32,
    /// Shuffle mode enabled
    shuffle: bool,
    /// Repeat mode
    repeat: RepeatMode,
}

impl Default for PlayQueue {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            position: -1, // Not started
            shuffle: false,
            repeat: RepeatMode::Off,
        }
    }
}

/// Repeat mode for the queue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RepeatMode {
    #[default]
    Off,
    /// Repeat entire queue
    All,
    /// Repeat current track
    One,
}

impl PlayQueue {
    /// Create an empty queue.
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if queue is empty.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Get queue length.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Add an item to the end of the queue.
    pub fn add(&mut self, item: QueueItem) {
        self.items.push(item);
    }

    /// Add an item after the current position.
    pub fn add_next(&mut self, item: QueueItem) {
        let insert_pos = if self.position < 0 {
            0
        } else {
            (self.position as usize + 1).min(self.items.len())
        };
        self.items.insert(insert_pos, item);
    }

    /// Clear the queue.
    pub fn clear(&mut self) {
        self.items.clear();
        self.position = -1;
    }

    /// Remove an item at index.
    pub fn remove(&mut self, index: usize) -> Option<QueueItem> {
        if index < self.items.len() {
            let item = self.items.remove(index);
            // Adjust position if needed
            if index as i32 <= self.position {
                self.position = (self.position - 1).max(-1);
            }
            Some(item)
        } else {
            None
        }
    }

    /// Move an item from one position to another.
    pub fn reorder(&mut self, from: usize, to: usize) {
        if from < self.items.len() && to < self.items.len() && from != to {
            let item = self.items.remove(from);
            self.items.insert(to, item);
            
            // Adjust current position
            let pos = self.position as usize;
            if from == pos {
                self.position = to as i32;
            } else if from < pos && to >= pos {
                self.position -= 1;
            } else if from > pos && to <= pos {
                self.position += 1;
            }
        }
    }

    /// Get all items in the queue.
    pub fn items(&self) -> &[QueueItem] {
        &self.items
    }

    /// Get current position (index into items).
    pub fn current_index(&self) -> Option<usize> {
        if self.position >= 0 && (self.position as usize) < self.items.len() {
            Some(self.position as usize)
        } else {
            None
        }
    }

    /// Get count of remaining tracks after current position.
    pub fn remaining_count(&self) -> usize {
        if self.position < 0 {
            self.items.len()
        } else {
            self.items.len().saturating_sub(self.position as usize + 1)
        }
    }

    /// Get current item.
    pub fn current(&self) -> Option<&QueueItem> {
        self.current_index().and_then(|i| self.items.get(i))
    }

    /// Advance to next track and return it.
    pub fn next(&mut self) -> Option<&QueueItem> {
        if self.items.is_empty() {
            return None;
        }

        match self.repeat {
            RepeatMode::One => {
                // Stay on current track
                if self.position < 0 {
                    self.position = 0;
                }
            }
            RepeatMode::All => {
                self.position += 1;
                if self.position as usize >= self.items.len() {
                    self.position = 0;
                }
            }
            RepeatMode::Off => {
                self.position += 1;
                if self.position as usize >= self.items.len() {
                    self.position = self.items.len() as i32 - 1;
                    return None; // End of queue
                }
            }
        }

        self.current()
    }

    /// Go to previous track and return it.
    pub fn previous(&mut self) -> Option<&QueueItem> {
        if self.items.is_empty() {
            return None;
        }

        match self.repeat {
            RepeatMode::One => {
                // Stay on current track
                if self.position < 0 {
                    self.position = 0;
                }
            }
            RepeatMode::All => {
                self.position -= 1;
                if self.position < 0 {
                    self.position = self.items.len() as i32 - 1;
                }
            }
            RepeatMode::Off => {
                self.position -= 1;
                if self.position < 0 {
                    self.position = 0;
                    return None; // Start of queue
                }
            }
        }

        self.current()
    }

    /// Jump to a specific position.
    pub fn jump_to(&mut self, index: usize) -> Option<&QueueItem> {
        if index < self.items.len() {
            self.position = index as i32;
            self.current()
        } else {
            None
        }
    }

    /// Set shuffle mode.
    pub fn set_shuffle(&mut self, enabled: bool) {
        self.shuffle = enabled;
        // TODO: Implement shuffle order
    }

    /// Get shuffle mode.
    pub fn shuffle(&self) -> bool {
        self.shuffle
    }

    /// Cycle repeat mode.
    pub fn cycle_repeat(&mut self) {
        self.repeat = match self.repeat {
            RepeatMode::Off => RepeatMode::All,
            RepeatMode::All => RepeatMode::One,
            RepeatMode::One => RepeatMode::Off,
        };
    }

    /// Set repeat mode.
    pub fn set_repeat(&mut self, mode: RepeatMode) {
        self.repeat = mode;
    }

    /// Get repeat mode.
    pub fn repeat(&self) -> RepeatMode {
        self.repeat
    }

    /// Update metadata for an item.
    pub fn update_info(&mut self, index: usize, info: TrackInfo) {
        if let Some(item) = self.items.get_mut(index) {
            item.info = Some(info);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_item(name: &str) -> QueueItem {
        QueueItem::from_path(PathBuf::from(name))
    }

    #[test]
    fn test_queue_basic() {
        let mut queue = PlayQueue::new();
        assert!(queue.is_empty());
        
        queue.add(make_item("a.mp3"));
        queue.add(make_item("b.mp3"));
        queue.add(make_item("c.mp3"));
        
        assert_eq!(queue.len(), 3);
        assert!(queue.current().is_none()); // Not started yet
        
        // First next() starts playback
        assert_eq!(queue.next().unwrap().path, PathBuf::from("a.mp3"));
        assert_eq!(queue.current_index(), Some(0));
        
        assert_eq!(queue.next().unwrap().path, PathBuf::from("b.mp3"));
        assert_eq!(queue.current_index(), Some(1));
    }

    #[test]
    fn test_queue_repeat_all() {
        let mut queue = PlayQueue::new();
        queue.set_repeat(RepeatMode::All);
        queue.add(make_item("a.mp3"));
        queue.add(make_item("b.mp3"));
        
        queue.next(); // a
        queue.next(); // b
        assert_eq!(queue.next().unwrap().path, PathBuf::from("a.mp3")); // wraps
    }

    #[test]
    fn test_queue_repeat_one() {
        let mut queue = PlayQueue::new();
        queue.set_repeat(RepeatMode::One);
        queue.add(make_item("a.mp3"));
        queue.add(make_item("b.mp3"));
        
        queue.next(); // a
        assert_eq!(queue.next().unwrap().path, PathBuf::from("a.mp3")); // stays
    }

    #[test]
    fn test_queue_add_next() {
        let mut queue = PlayQueue::new();
        queue.add(make_item("a.mp3"));
        queue.add(make_item("c.mp3"));
        queue.next(); // Start playing a
        
        queue.add_next(make_item("b.mp3"));
        
        assert_eq!(queue.items()[0].path, PathBuf::from("a.mp3"));
        assert_eq!(queue.items()[1].path, PathBuf::from("b.mp3"));
        assert_eq!(queue.items()[2].path, PathBuf::from("c.mp3"));
    }
}
