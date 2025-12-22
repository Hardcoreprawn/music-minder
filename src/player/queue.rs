//! Play queue management.

use super::state::TrackInfo;
use rand::seq::SliceRandom;
use std::path::PathBuf;

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
        Self {
            path,
            info: Some(info),
        }
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
    /// Shuffled indices (maps shuffle position → item index)
    shuffle_order: Vec<usize>,
    /// Current position in shuffle_order when shuffling
    shuffle_position: i32,
    /// Repeat mode
    repeat: RepeatMode,
}

impl Default for PlayQueue {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            position: -1, // Not started
            shuffle: false,
            shuffle_order: Vec::new(),
            shuffle_position: -1,
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
        let new_index = self.items.len();
        self.items.push(item);
        // Add new item to shuffle order (at random position if shuffling)
        if self.shuffle {
            if self.shuffle_order.is_empty() {
                self.shuffle_order.push(new_index);
            } else {
                // Insert at random position after current
                let insert_after = if self.shuffle_position < 0 {
                    0
                } else {
                    self.shuffle_position as usize + 1
                };
                let insert_pos = if insert_after >= self.shuffle_order.len() {
                    self.shuffle_order.len()
                } else {
                    let mut rng = rand::rng();
                    rand::Rng::random_range(&mut rng, insert_after..=self.shuffle_order.len())
                };
                self.shuffle_order.insert(insert_pos, new_index);
            }
        }
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
        self.shuffle_order.clear();
        self.shuffle_position = -1;
    }

    /// Remove an item at index.
    pub fn remove(&mut self, index: usize) -> Option<QueueItem> {
        if index < self.items.len() {
            let item = self.items.remove(index);
            // Adjust position if needed
            if index as i32 <= self.position {
                self.position = (self.position - 1).max(-1);
            }
            // Update shuffle order: remove this index and adjust all indices > removed
            if self.shuffle {
                if let Some(shuffle_idx) = self.shuffle_order.iter().position(|&i| i == index) {
                    self.shuffle_order.remove(shuffle_idx);
                    if (shuffle_idx as i32) <= self.shuffle_position {
                        self.shuffle_position = (self.shuffle_position - 1).max(-1);
                    }
                }
                // Decrement all indices greater than the removed one
                for idx in &mut self.shuffle_order {
                    if *idx > index {
                        *idx -= 1;
                    }
                }
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

    /// Move an item up one position. Returns the new index if moved.
    ///
    /// If shuffle is enabled, this reorders the shuffle sequence instead.
    pub fn move_up(&mut self, index: usize) -> Option<usize> {
        if index == 0 {
            return None; // Already at top
        }

        if self.shuffle && !self.shuffle_order.is_empty() {
            self.reorder_shuffle(index, index - 1)
        } else if index < self.items.len() {
            self.reorder(index, index - 1);
            Some(index - 1)
        } else {
            None
        }
    }

    /// Move an item down one position. Returns the new index if moved.
    ///
    /// If shuffle is enabled, this reorders the shuffle sequence instead.
    pub fn move_down(&mut self, index: usize) -> Option<usize> {
        let max_index = if self.shuffle && !self.shuffle_order.is_empty() {
            self.shuffle_order.len().saturating_sub(1)
        } else {
            self.items.len().saturating_sub(1)
        };

        if index >= max_index {
            return None; // Already at bottom
        }

        if self.shuffle && !self.shuffle_order.is_empty() {
            self.reorder_shuffle(index, index + 1)
        } else {
            self.reorder(index, index + 1);
            Some(index + 1)
        }
    }

    /// Reorder within the shuffle sequence.
    ///
    /// When shuffle is enabled, the user sees tracks in shuffle order.
    /// This method reorders the shuffle_order array so the visual order changes.
    /// The underlying items array is not modified.
    ///
    /// `from` and `to` are indices into shuffle_order, not items.
    pub fn reorder_shuffle(&mut self, from: usize, to: usize) -> Option<usize> {
        if !self.shuffle || self.shuffle_order.is_empty() {
            return None;
        }

        if from >= self.shuffle_order.len() || to >= self.shuffle_order.len() || from == to {
            return None;
        }

        // Swap positions in shuffle_order
        let item_idx = self.shuffle_order.remove(from);
        self.shuffle_order.insert(to, item_idx);

        // Adjust shuffle_position if needed
        let pos = self.shuffle_position as usize;
        if from == pos {
            self.shuffle_position = to as i32;
        } else if from < pos && to >= pos {
            self.shuffle_position -= 1;
        } else if from > pos && to <= pos {
            self.shuffle_position += 1;
        }

        Some(to)
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
    pub fn skip_forward(&mut self) -> Option<&QueueItem> {
        if self.items.is_empty() {
            return None;
        }

        if self.shuffle && !self.shuffle_order.is_empty() {
            return self.skip_forward_shuffle();
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

    /// Skip forward in shuffle mode.
    fn skip_forward_shuffle(&mut self) -> Option<&QueueItem> {
        match self.repeat {
            RepeatMode::One => {
                // Stay on current track
                if self.shuffle_position < 0 {
                    self.shuffle_position = 0;
                    self.position = self.shuffle_order[0] as i32;
                }
            }
            RepeatMode::All => {
                self.shuffle_position += 1;
                if self.shuffle_position as usize >= self.shuffle_order.len() {
                    // Reshuffle for next loop
                    self.generate_shuffle_order();
                    self.shuffle_position = 0;
                }
                self.position = self.shuffle_order[self.shuffle_position as usize] as i32;
            }
            RepeatMode::Off => {
                self.shuffle_position += 1;
                if self.shuffle_position as usize >= self.shuffle_order.len() {
                    self.shuffle_position = self.shuffle_order.len() as i32 - 1;
                    return None; // End of shuffle
                }
                self.position = self.shuffle_order[self.shuffle_position as usize] as i32;
            }
        }

        self.current()
    }

    /// Go to previous track and return it.
    pub fn previous(&mut self) -> Option<&QueueItem> {
        if self.items.is_empty() {
            return None;
        }

        if self.shuffle && !self.shuffle_order.is_empty() {
            return self.previous_shuffle();
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

    /// Go to previous track in shuffle mode.
    fn previous_shuffle(&mut self) -> Option<&QueueItem> {
        match self.repeat {
            RepeatMode::One => {
                // Stay on current track
                if self.shuffle_position < 0 {
                    self.shuffle_position = 0;
                    self.position = self.shuffle_order[0] as i32;
                }
            }
            RepeatMode::All => {
                self.shuffle_position -= 1;
                if self.shuffle_position < 0 {
                    self.shuffle_position = self.shuffle_order.len() as i32 - 1;
                }
                self.position = self.shuffle_order[self.shuffle_position as usize] as i32;
            }
            RepeatMode::Off => {
                self.shuffle_position -= 1;
                if self.shuffle_position < 0 {
                    self.shuffle_position = 0;
                    return None; // Start of shuffle
                }
                self.position = self.shuffle_order[self.shuffle_position as usize] as i32;
            }
        }

        self.current()
    }

    /// Jump to a specific position.
    pub fn jump_to(&mut self, index: usize) -> Option<&QueueItem> {
        if index < self.items.len() {
            self.position = index as i32;
            // Update shuffle position to match
            if self.shuffle
                && !self.shuffle_order.is_empty()
                && let Some(shuffle_pos) = self.shuffle_order.iter().position(|&i| i == index)
            {
                self.shuffle_position = shuffle_pos as i32;
            }
            self.current()
        } else {
            None
        }
    }

    /// Set shuffle mode.
    pub fn set_shuffle(&mut self, enabled: bool) {
        self.shuffle = enabled;
        if enabled {
            self.generate_shuffle_order();
        } else {
            self.shuffle_order.clear();
            self.shuffle_position = -1;
        }
    }

    /// Generate a new shuffle order, keeping current track first if playing.
    fn generate_shuffle_order(&mut self) {
        let len = self.items.len();
        if len == 0 {
            self.shuffle_order.clear();
            self.shuffle_position = -1;
            return;
        }

        // Create indices
        let mut indices: Vec<usize> = (0..len).collect();

        // Shuffle using Fisher-Yates
        let mut rng = rand::rng();
        indices.shuffle(&mut rng);

        // If we're currently playing, move that track to front of shuffle
        if self.position >= 0 {
            let current_idx = self.position as usize;
            if let Some(pos) = indices.iter().position(|&i| i == current_idx) {
                indices.remove(pos);
                indices.insert(0, current_idx);
            }
            self.shuffle_position = 0;
        } else {
            self.shuffle_position = -1;
        }

        self.shuffle_order = indices;
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

        // First skip_forward() starts playback
        assert_eq!(queue.skip_forward().unwrap().path, PathBuf::from("a.mp3"));
        assert_eq!(queue.current_index(), Some(0));

        assert_eq!(queue.skip_forward().unwrap().path, PathBuf::from("b.mp3"));
        assert_eq!(queue.current_index(), Some(1));
    }

    #[test]
    fn test_queue_repeat_all() {
        let mut queue = PlayQueue::new();
        queue.set_repeat(RepeatMode::All);
        queue.add(make_item("a.mp3"));
        queue.add(make_item("b.mp3"));

        queue.skip_forward(); // a
        queue.skip_forward(); // b
        assert_eq!(queue.skip_forward().unwrap().path, PathBuf::from("a.mp3")); // wraps
    }

    #[test]
    fn test_queue_repeat_one() {
        let mut queue = PlayQueue::new();
        queue.set_repeat(RepeatMode::One);
        queue.add(make_item("a.mp3"));
        queue.add(make_item("b.mp3"));

        queue.skip_forward(); // a
        assert_eq!(queue.skip_forward().unwrap().path, PathBuf::from("a.mp3")); // stays
    }

    #[test]
    fn test_queue_add_next() {
        let mut queue = PlayQueue::new();
        queue.add(make_item("a.mp3"));
        queue.add(make_item("c.mp3"));
        queue.skip_forward(); // Start playing a

        queue.add_next(make_item("b.mp3"));

        assert_eq!(queue.items()[0].path, PathBuf::from("a.mp3"));
        assert_eq!(queue.items()[1].path, PathBuf::from("b.mp3"));
        assert_eq!(queue.items()[2].path, PathBuf::from("c.mp3"));
    }

    #[test]
    fn test_shuffle_visits_all_tracks() {
        let mut queue = PlayQueue::new();
        for i in 0..10 {
            queue.add(make_item(&format!("{}.mp3", i)));
        }
        queue.set_shuffle(true);

        // Collect all tracks visited in shuffle order
        let mut visited = std::collections::HashSet::new();
        for _ in 0..10 {
            let item = queue.skip_forward();
            if let Some(item) = item {
                visited.insert(item.path.clone());
            }
        }

        // Should have visited all 10 tracks
        assert_eq!(visited.len(), 10);
    }

    #[test]
    fn test_shuffle_keeps_current_first() {
        let mut queue = PlayQueue::new();
        queue.add(make_item("a.mp3"));
        queue.add(make_item("b.mp3"));
        queue.add(make_item("c.mp3"));

        // Start playing track 1 (b.mp3)
        queue.skip_forward(); // a
        queue.skip_forward(); // b
        assert_eq!(queue.current_index(), Some(1));

        // Enable shuffle - current track should stay current
        queue.set_shuffle(true);
        assert_eq!(queue.current_index(), Some(1));
        assert_eq!(queue.shuffle_order[0], 1); // Current track is first in shuffle
    }

    #[test]
    fn test_shuffle_previous_works() {
        let mut queue = PlayQueue::new();
        queue.add(make_item("a.mp3"));
        queue.add(make_item("b.mp3"));
        queue.add(make_item("c.mp3"));
        queue.set_shuffle(true);

        // Navigate forward twice
        let first = queue.skip_forward().unwrap().path.clone();
        let second = queue.skip_forward().unwrap().path.clone();

        // Go back
        let back = queue.previous().unwrap().path.clone();
        assert_eq!(back, first);

        // Forward again should return to second
        let again = queue.skip_forward().unwrap().path.clone();
        assert_eq!(again, second);
    }

    #[test]
    fn test_shuffle_disable_clears_order() {
        let mut queue = PlayQueue::new();
        queue.add(make_item("a.mp3"));
        queue.add(make_item("b.mp3"));
        queue.set_shuffle(true);
        assert!(!queue.shuffle_order.is_empty());

        queue.set_shuffle(false);
        assert!(queue.shuffle_order.is_empty());
        assert!(!queue.shuffle());
    }

    #[test]
    fn test_shuffle_with_repeat_all() {
        let mut queue = PlayQueue::new();
        queue.add(make_item("a.mp3"));
        queue.add(make_item("b.mp3"));
        queue.set_shuffle(true);
        queue.set_repeat(RepeatMode::All);

        // Play through entire queue
        queue.skip_forward();
        queue.skip_forward();

        // Third skip should wrap and reshuffle
        let third = queue.skip_forward();
        assert!(third.is_some()); // Should still return a track
    }

    #[test]
    fn test_jump_to_updates_shuffle_position() {
        let mut queue = PlayQueue::new();
        queue.add(make_item("a.mp3"));
        queue.add(make_item("b.mp3"));
        queue.add(make_item("c.mp3"));
        queue.set_shuffle(true);
        queue.skip_forward(); // Start playing

        // Jump to specific track
        queue.jump_to(2);
        assert_eq!(queue.current_index(), Some(2));

        // Shuffle position should be updated to match
        let shuffle_pos = queue.shuffle_position as usize;
        assert_eq!(queue.shuffle_order[shuffle_pos], 2);
    }

    #[test]
    fn test_move_up() {
        let mut queue = PlayQueue::new();
        queue.add(make_item("a.mp3"));
        queue.add(make_item("b.mp3"));
        queue.add(make_item("c.mp3"));

        // Move item at index 1 up to index 0
        let new_idx = queue.move_up(1);
        assert_eq!(new_idx, Some(0));
        assert_eq!(queue.items()[0].path, PathBuf::from("b.mp3"));
        assert_eq!(queue.items()[1].path, PathBuf::from("a.mp3"));

        // Move at index 0 should return None
        assert_eq!(queue.move_up(0), None);
    }

    #[test]
    fn test_move_down() {
        let mut queue = PlayQueue::new();
        queue.add(make_item("a.mp3"));
        queue.add(make_item("b.mp3"));
        queue.add(make_item("c.mp3"));

        // Move item at index 0 down to index 1
        let new_idx = queue.move_down(0);
        assert_eq!(new_idx, Some(1));
        assert_eq!(queue.items()[0].path, PathBuf::from("b.mp3"));
        assert_eq!(queue.items()[1].path, PathBuf::from("a.mp3"));

        // Move at last index should return None
        assert_eq!(queue.move_down(2), None);
    }

    #[test]
    fn test_move_preserves_current_playing() {
        let mut queue = PlayQueue::new();
        queue.add(make_item("a.mp3"));
        queue.add(make_item("b.mp3"));
        queue.add(make_item("c.mp3"));
        queue.skip_forward(); // Playing a (index 0)
        queue.skip_forward(); // Playing b (index 1)
        assert_eq!(queue.current_index(), Some(1));

        // Move the currently playing track up
        queue.move_up(1);
        // Current index should follow the item
        assert_eq!(queue.current_index(), Some(0));
        assert_eq!(queue.current().unwrap().path, PathBuf::from("b.mp3"));
    }

    #[test]
    fn test_move_in_shuffle_mode() {
        let mut queue = PlayQueue::new();
        queue.add(make_item("a.mp3"));
        queue.add(make_item("b.mp3"));
        queue.add(make_item("c.mp3"));
        queue.set_shuffle(true);

        // Get the original shuffle order
        let orig_first = queue.shuffle_order[0];
        let orig_second = queue.shuffle_order[1];

        // Move shuffle position 1 up to 0
        let new_idx = queue.move_up(1);
        assert_eq!(new_idx, Some(0));

        // The shuffle order should be swapped
        assert_eq!(queue.shuffle_order[0], orig_second);
        assert_eq!(queue.shuffle_order[1], orig_first);

        // The underlying items should NOT be changed
        assert_eq!(queue.items()[0].path, PathBuf::from("a.mp3"));
        assert_eq!(queue.items()[1].path, PathBuf::from("b.mp3"));
        assert_eq!(queue.items()[2].path, PathBuf::from("c.mp3"));
    }
}
