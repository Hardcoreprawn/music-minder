//! Icon constants using Unicode symbols that work in default fonts.
//! 
//! Uses geometric shapes and symbols from Unicode that render in most system fonts.

use iced::widget::text;

// ============================================================================
// Player Control Icons - Using Unicode geometric shapes
// ============================================================================

/// Play icon (triangle pointing right) - BLACK RIGHT-POINTING TRIANGLE
pub const PLAY: &str = "▶";

/// Pause icon (two vertical bars) - DOUBLE VERTICAL BAR  
pub const PAUSE: &str = "⏸";

/// Stop icon (square) - BLACK SQUARE
pub const STOP: &str = "■";

/// Skip to previous track - BLACK LEFT-POINTING DOUBLE TRIANGLE WITH VERTICAL BAR
pub const SKIP_BACK: &str = "⏮";

/// Skip to next track - BLACK RIGHT-POINTING DOUBLE TRIANGLE WITH VERTICAL BAR
pub const SKIP_FORWARD: &str = "⏭";

/// Volume icon - SPEAKER WITH THREE SOUND WAVES
pub const VOLUME_UP: &str = "🔊";

/// Volume muted - SPEAKER WITH CANCELLATION STROKE
pub const VOLUME_MUTE: &str = "🔇";

// ============================================================================
// Navigation Icons  
// ============================================================================

/// Folder/Library icon - CARD FILE BOX
pub const FOLDER: &str = "📁";
pub const COLLECTION: &str = "📚";

/// Music icon - MUSICAL NOTE
pub const MUSIC: &str = "♪";
pub const MUSIC_NOTE: &str = "♫";

/// Gear/Settings icon - GEAR
pub const GEAR: &str = "⚙";

// ============================================================================
// Action Icons
// ============================================================================

/// Plus icon - HEAVY PLUS SIGN
pub const PLUS: &str = "+";

/// Check mark - CHECK MARK
pub const CHECK: &str = "✓";

/// X mark - MULTIPLICATION X
pub const X: &str = "✗";

/// Caret right - BLACK RIGHT-POINTING SMALL TRIANGLE
pub const CARET_RIGHT: &str = "▸";

// ============================================================================
// Status Icons
// ============================================================================

/// Success - CHECK MARK
pub const CHECK_CIRCLE: &str = "✓";

/// Error - CROSS MARK
pub const X_CIRCLE: &str = "✗";

/// Warning - WARNING SIGN
pub const EXCLAMATION_CIRCLE: &str = "⚠";
pub const EXCLAMATION_TRIANGLE: &str = "⚠";

/// Info - INFORMATION SOURCE
pub const INFO_CIRCLE: &str = "ℹ";

// ============================================================================
// Helper Functions - Return regular text, no special font needed
// ============================================================================

/// Create an icon text element
pub fn icon(s: &str) -> iced::widget::Text<'static> {
    text(s.to_string())
}

/// Create an icon text element with specific size  
pub fn icon_sized(s: &str, size: u16) -> iced::widget::Text<'static> {
    text(s.to_string()).size(size)
}

// Keep the font bytes for potential future use, but don't load it
pub const ICON_FONT_BYTES: &[u8] = &[];
