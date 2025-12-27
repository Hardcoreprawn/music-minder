//! Icon system using Font Awesome 6 Free (Solid).
//!
//! Font Awesome provides consistent, professional icons across all platforms.
//! Icons are referenced by their Unicode codepoints.

use iced::Font;
use iced::font::{Family, Weight};
use iced::widget::Text;

// ============================================================================
// Font Definition
// ============================================================================

/// Font Awesome font bytes for loading at startup
pub const ICON_FONT_BYTES: &[u8] = include_bytes!("../../assets/fa-solid-900.ttf");

/// Font Awesome 6 Free Solid - specify family name and weight
/// The font file fa-solid-900.ttf has weight 900 (Black)
/// Try "Font Awesome 6 Free" as family (Solid is the weight variant)
pub const ICON_FONT: Font = Font {
    family: Family::Name("Font Awesome 6 Free"),
    weight: Weight::Black,
    ..Font::DEFAULT
};

// ============================================================================
// Player Control Icons (Font Awesome codepoints)
// ============================================================================

/// Play icon - fa-play (U+F04B)
pub const PLAY: char = '\u{f04b}';

/// Pause icon - fa-pause (U+F04C)
pub const PAUSE: char = '\u{f04c}';

/// Stop icon - fa-stop (U+F04D)
pub const STOP: char = '\u{f04d}';

/// Skip backward - fa-backward-step (U+F048)
pub const SKIP_BACK: char = '\u{f048}';

/// Skip forward - fa-forward-step (U+F051)
pub const SKIP_FORWARD: char = '\u{f051}';

/// Shuffle - fa-shuffle (U+F074)
pub const SHUFFLE: char = '\u{f074}';

/// Repeat - fa-repeat (U+F363)
pub const REPEAT: char = '\u{f363}';

/// Repeat one - fa-rotate-left with 1 (we'll use repeat)
pub const REPEAT_ONE: char = '\u{f363}';

// ============================================================================
// Volume Icons
// ============================================================================

/// Volume high - fa-volume-high (U+F028)
pub const VOLUME_HIGH: char = '\u{f028}';

/// Volume low - fa-volume-low (U+F027)
pub const VOLUME_LOW: char = '\u{f027}';

/// Volume off/mute - fa-volume-xmark (U+F6A9)
pub const VOLUME_MUTE: char = '\u{f6a9}';

/// Volume (medium) - fa-volume (older: fa-volume-down)
pub const VOLUME_MED: char = '\u{f027}';

// ============================================================================
// Navigation Icons
// ============================================================================

/// Music/Library - fa-music (U+F001)
pub const MUSIC: char = '\u{f001}';

/// Folder - fa-folder (U+F07B)
pub const FOLDER: char = '\u{f07b}';

/// Folder open - fa-folder-open (U+F07C)
pub const FOLDER_OPEN: char = '\u{f07c}';

/// Floppy disk/Save - fa-floppy-disk (U+F0C7)
pub const FLOPPY: char = '\u{f0c7}';

/// File export - fa-file-export (U+F56E)
pub const FILE_EXPORT: char = '\u{f56e}';

/// List/Library - fa-list (U+F03A)
pub const LIST: char = '\u{f03a}';

/// Gear/Settings - fa-gear (U+F013)
pub const GEAR: char = '\u{f013}';

/// Sliders/Settings - fa-sliders (U+F1DE)
pub const SLIDERS: char = '\u{f1de}';

/// Wand/Magic/Enrich - fa-wand-magic-sparkles (U+E2CA)
pub const WAND: char = '\u{e2ca}';

/// Headphones - fa-headphones (U+F025)
pub const HEADPHONES: char = '\u{f025}';

/// Speaker - fa-volume-high works as speaker indicator
pub const SPEAKER: char = '\u{f028}';

/// Compact disc - fa-compact-disc (U+F51F)
pub const DISC: char = '\u{f51f}';

/// Compact disc (alias for easter eggs)
pub const COMPACT_DISC: char = '\u{f51f}';

/// Record vinyl - fa-record-vinyl (U+F8D9)
pub const RECORD_VINYL: char = '\u{f8d9}';

/// Radio - fa-radio (U+F8D7)
pub const RADIO: char = '\u{f8d7}';

/// Guitar - fa-guitar (U+F7A6)
pub const GUITAR: char = '\u{f7a6}';

/// Drum - fa-drum (U+F569)
pub const DRUM: char = '\u{f569}';

/// Microphone - fa-microphone (U+F130)
pub const MICROPHONE: char = '\u{f130}';

/// Music note - fa-music (same as MUSIC but clearer name)
pub const MUSIC_NOTE: char = '\u{f001}';

/// Speaker high (for audio settings) - same as VOLUME_HIGH
pub const SPEAKER_HIGH: char = '\u{f028}';

/// Moon (dark theme) - fa-moon (U+F186)
pub const MOON: char = '\u{f186}';

/// Palette (appearance) - fa-palette (U+F53F)
pub const PALETTE: char = '\u{f53f}';

/// Info circle - fa-circle-info (U+F05A)
pub const INFO: char = '\u{f05a}';

/// Sparkle/Star - fa-star (U+F005)
pub const SPARKLE: char = '\u{f005}';

/// Star (char version for easter eggs)
pub const STAR: char = '\u{f005}';

/// Heart - fa-heart (U+F004)
pub const HEART: char = '\u{f004}';

/// Fire - fa-fire (U+F06D)
pub const FIRE: char = '\u{f06d}';

/// Rocket - fa-rocket (U+F135)
pub const ROCKET: char = '\u{f135}';

/// Gift - fa-gift (U+F06B)
pub const GIFT: char = '\u{f06b}';

/// Hand pointer - fa-hand-pointer (U+F25A)
pub const HAND_POINTER: char = '\u{f25a}';

/// Face smile - fa-face-smile (U+F118)
pub const FACE_SMILE: char = '\u{f118}';

/// Face grin - fa-face-grin (U+F580)
pub const FACE_GRIN: char = '\u{f580}';

/// Wand sparkles (char version for easter eggs)
pub const WAND_SPARKLES: char = '\u{e2ca}';

/// Database - fa-database (U+F1C0)
pub const DATABASE: char = '\u{f1c0}';

/// Refresh/Sync - fa-arrows-rotate (U+F021)
pub const REFRESH: char = '\u{f021}';

/// Check circle - fa-circle-check (U+F058)  
pub const CHECK_CIRCLE: char = '\u{f058}';

/// Warning/Triangle - fa-triangle-exclamation (U+F071)
pub const WARNING: char = '\u{f071}';

/// Wand/Enrich (string version) - fa-wand-magic-sparkles (U+E2CA)
pub const WAND_STR: &str = "\u{e2ca}";

/// Empty circle (string version) - fa-circle (U+F111)
pub const CIRCLE_STR: &str = "\u{f111}";

/// Folder (string version) - fa-folder (U+F07B)
pub const FOLDER_STR: &str = "\u{f07b}";

/// Gear (string version) - fa-gear (U+F013)
pub const GEAR_STR: &str = "\u{f013}";

// ============================================================================
// Action Icons
// ============================================================================

/// Plus - fa-plus (U+2B)
pub const PLUS: char = '\u{2b}';

/// Minus - fa-minus (U+F068)
pub const MINUS: char = '\u{f068}';

/// X/Close - fa-xmark (U+F00D)
pub const XMARK: char = '\u{f00d}';

/// Check - fa-check (U+F00C)
pub const CHECK: char = '\u{f00c}';

/// Search - fa-magnifying-glass (U+F002)
pub const SEARCH: char = '\u{f002}';

/// Ellipsis vertical - fa-ellipsis-vertical (U+F142)
pub const ELLIPSIS_V: char = '\u{f142}';

/// Bars/Menu - fa-bars (U+F0C9)
pub const BARS: char = '\u{f0c9}';

/// Arrow up - fa-arrow-up (U+F062)
pub const ARROW_UP: char = '\u{f062}';

/// Arrow down - fa-arrow-down (U+F063)
pub const ARROW_DOWN: char = '\u{f063}';

/// Chevron right - fa-chevron-right (U+F054)
pub const CHEVRON_RIGHT: char = '\u{f054}';

/// Chevron down - fa-chevron-down (U+F078)
pub const CHEVRON_DOWN: char = '\u{f078}';

/// Chevron left - fa-chevron-left (U+F053)
pub const CHEVRON_LEFT: char = '\u{f053}';

/// Arrow rotate right - fa-arrow-rotate-right (U+F01E)
pub const ARROW_ROTATE: char = '\u{f01e}';

/// Grip vertical - fa-grip-vertical (U+F58E) - for drag handles
pub const GRIP_VERTICAL: char = '\u{f58e}';

// ============================================================================
// Diagnostic/System Icons
// ============================================================================

/// Microchip - fa-microchip (U+F2DB)
pub const CHIP: char = '\u{f2db}';

/// Clock - fa-clock (U+F017)
pub const CLOCK: char = '\u{f017}';

/// Gauge high - fa-gauge-high (U+F625)
pub const GAUGE: char = '\u{f625}';

/// Memory - fa-memory (U+F538)
pub const MEMORY: char = '\u{f538}';

/// Bolt - fa-bolt (U+F0E7)
pub const BOLT: char = '\u{f0e7}';

/// Lightbulb - fa-lightbulb (U+F0EB)
pub const LIGHTBULB: char = '\u{f0eb}';

// ============================================================================
// Status Icons
// ============================================================================

/// Circle check - fa-circle-check (U+F058)
pub const CIRCLE_CHECK: char = '\u{f058}';

/// Circle X - fa-circle-xmark (U+F057)
pub const CIRCLE_XMARK: char = '\u{f057}';

/// Circle exclamation - fa-circle-exclamation (U+F06A)
pub const CIRCLE_EXCLAIM: char = '\u{f06a}';

/// Circle info - fa-circle-info (U+F05A)
pub const CIRCLE_INFO: char = '\u{f05a}';

/// Empty circle - fa-circle (U+F111)
pub const CIRCLE: char = '\u{f111}';

/// Spinner - fa-spinner (U+F110)
pub const SPINNER: char = '\u{f110}';

/// Circle notch (another spinner) - fa-circle-notch (U+F1CE)  
pub const CIRCLE_NOTCH: char = '\u{f1ce}';

/// Sync (rotating arrows) - fa-sync (U+F021)
pub const SYNC: char = '\u{f021}';

/// Eye - fa-eye (U+F06E)
pub const EYE: char = '\u{f06e}';

/// Eye slash - fa-eye-slash (U+F070)
pub const EYE_SLASH: char = '\u{f070}';

// ============================================================================
// Spinner Animation Frames
// ============================================================================

/// Simple ASCII spinner animation frames - universally supported
const ASCII_SPINNER: [char; 4] = ['|', '/', '—', '\\'];

/// Get a spinner character based on animation tick
/// Call with the current animation_tick to get a smoothly rotating frame
/// At 60fps tick rate, changes frame every 10 ticks (~6fps animation)
pub fn spinner_frame(tick: u32) -> char {
    let frame = (tick / 10) as usize % ASCII_SPINNER.len();
    ASCII_SPINNER[frame]
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Create an icon text element with the icon font
pub fn icon(c: char) -> Text<'static> {
    Text::new(c.to_string()).font(ICON_FONT)
}

/// Create an icon text element with specific size
pub fn icon_sized(c: char, size: u16) -> Text<'static> {
    Text::new(c.to_string()).font(ICON_FONT).size(size)
}
