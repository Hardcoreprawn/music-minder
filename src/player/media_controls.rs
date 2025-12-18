//! OS Media Controls integration via souvlaki.
//!
//! This module provides integration with platform-specific media controls:
//! - Windows: System Media Transport Controls (SMTC)
//! - Linux: MPRIS D-Bus interface
//! - macOS: MediaCenter / Now Playing
//!
//! Features:
//! - Media key support (play/pause/next/prev from keyboard)
//! - System overlay with track info + album art
//! - Bluetooth/headphone button controls

use souvlaki::{
    MediaControlEvent, MediaControls, MediaMetadata, MediaPlayback, PlatformConfig, SeekDirection,
};
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::Duration;

/// Commands that can be received from OS media controls.
#[derive(Debug, Clone)]
pub enum MediaControlCommand {
    /// Play or resume playback
    Play,
    /// Pause playback
    Pause,
    /// Toggle play/pause
    Toggle,
    /// Stop playback
    Stop,
    /// Skip to next track
    Next,
    /// Skip to previous track
    Previous,
    /// Seek to absolute position
    Seek(Duration),
    /// Seek relative (forward/backward by small amount)
    SeekRelative(SeekDirection),
}

/// Metadata to display in OS media controls.
#[derive(Debug, Clone, Default)]
pub struct MediaControlsMetadata {
    /// Track title
    pub title: Option<String>,
    /// Artist name
    pub artist: Option<String>,
    /// Album name
    pub album: Option<String>,
    /// Track duration
    pub duration: Option<Duration>,
    /// Path to cover art image (for system overlay)
    pub cover_path: Option<PathBuf>,
}

impl MediaControlsMetadata {
    /// Create metadata with just a title.
    pub fn with_title(title: impl Into<String>) -> Self {
        Self {
            title: Some(title.into()),
            ..Default::default()
        }
    }

    /// Set the artist.
    pub fn artist(mut self, artist: impl Into<String>) -> Self {
        self.artist = Some(artist.into());
        self
    }

    /// Set the album.
    pub fn album(mut self, album: impl Into<String>) -> Self {
        self.album = Some(album.into());
        self
    }

    /// Set the duration.
    pub fn duration(mut self, duration: Duration) -> Self {
        self.duration = Some(duration);
        self
    }

    /// Set the cover art path.
    pub fn cover(mut self, path: PathBuf) -> Self {
        self.cover_path = Some(path);
        self
    }
}

/// Handle to the OS media controls.
///
/// This runs on a separate thread and communicates via channels.
pub struct MediaControlsHandle {
    /// Sender for updating the controls
    update_tx: Sender<MediaControlsUpdate>,
    /// Receiver for commands from the OS
    command_rx: Receiver<MediaControlCommand>,
}

/// Updates that can be sent to the media controls.
#[derive(Debug, Clone)]
pub enum MediaControlsUpdate {
    /// Update the metadata displayed
    Metadata(MediaControlsMetadata),
    /// Update the playback state
    PlaybackState(MediaPlaybackState),
    /// Update the current position
    Position(Duration),
    /// Shutdown the media controls
    Shutdown,
}

/// Playback state for media controls.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaPlaybackState {
    Playing,
    Paused,
    Stopped,
}

impl MediaControlsHandle {
    /// Initialize OS media controls.
    ///
    /// Returns `None` if media controls are not available on this platform.
    pub fn new() -> Option<Self> {
        let (update_tx, update_rx) = channel::<MediaControlsUpdate>();
        let (command_tx, command_rx) = channel::<MediaControlCommand>();

        // Spawn the media controls thread
        match std::thread::Builder::new()
            .name("media-controls".into())
            .spawn(move || {
                tracing::info!("Media controls thread started");
                match run_media_controls(update_rx, command_tx) {
                    Ok(()) => tracing::info!("Media controls thread ended normally"),
                    Err(e) => tracing::error!("Media controls thread error: {}", e),
                }
            }) {
            Ok(_) => {
                tracing::info!("Media controls thread spawned successfully");
                Some(Self {
                    update_tx,
                    command_rx,
                })
            }
            Err(e) => {
                tracing::error!("Failed to spawn media controls thread: {}", e);
                None
            }
        }
    }

    /// Update the displayed metadata.
    pub fn set_metadata(&self, metadata: MediaControlsMetadata) {
        let _ = self.update_tx.send(MediaControlsUpdate::Metadata(metadata));
    }

    /// Update the playback state.
    pub fn set_playback_state(&self, state: MediaPlaybackState) {
        let _ = self
            .update_tx
            .send(MediaControlsUpdate::PlaybackState(state));
    }

    /// Update the current playback position.
    pub fn set_position(&self, position: Duration) {
        let _ = self.update_tx.send(MediaControlsUpdate::Position(position));
    }

    /// Try to receive a command from the OS (non-blocking).
    pub fn try_recv_command(&self) -> Option<MediaControlCommand> {
        self.command_rx.try_recv().ok()
    }

    /// Shutdown the media controls.
    pub fn shutdown(&self) {
        let _ = self.update_tx.send(MediaControlsUpdate::Shutdown);
    }
}

impl Drop for MediaControlsHandle {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// Run the media controls event loop on a dedicated thread.
fn run_media_controls(
    update_rx: Receiver<MediaControlsUpdate>,
    command_tx: Sender<MediaControlCommand>,
) -> Result<(), String> {
    tracing::debug!("Setting up platform-specific media controls config");

    // Platform-specific configuration
    #[cfg(target_os = "windows")]
    let hwnd = {
        // On Windows, we need a window handle. Create a hidden window.
        // Note: SMTC requires a real window, not a message-only window (HWND_MESSAGE).
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;
        use std::ptr;

        unsafe {
            // Get module handle
            let h_instance =
                windows_sys::Win32::System::LibraryLoader::GetModuleHandleW(ptr::null());
            tracing::debug!("Got module handle: {:?}", h_instance);

            // Create a unique class name to avoid conflicts
            let class_name: Vec<u16> = OsStr::new("MusicMinderSMTC")
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();

            // Register window class with proper window procedure
            let wc = windows_sys::Win32::UI::WindowsAndMessaging::WNDCLASSEXW {
                cbSize: std::mem::size_of::<windows_sys::Win32::UI::WindowsAndMessaging::WNDCLASSEXW>(
                ) as u32,
                style: 0,
                lpfnWndProc: Some(windows_sys::Win32::UI::WindowsAndMessaging::DefWindowProcW),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: h_instance,
                hIcon: ptr::null_mut(),
                hCursor: ptr::null_mut(),
                hbrBackground: ptr::null_mut(),
                lpszMenuName: ptr::null(),
                lpszClassName: class_name.as_ptr(),
                hIconSm: ptr::null_mut(),
            };

            let class_atom = windows_sys::Win32::UI::WindowsAndMessaging::RegisterClassExW(&wc);
            if class_atom == 0 {
                // Class might already be registered from a previous run
                tracing::debug!("Window class registration returned 0 (may already exist)");
            } else {
                tracing::debug!("Registered window class, atom: {}", class_atom);
            }

            // Create a hidden window (not message-only, but with size 0,0 and not visible)
            // This is required for SMTC to work properly
            let hwnd = windows_sys::Win32::UI::WindowsAndMessaging::CreateWindowExW(
                0, // Extended style
                class_name.as_ptr(),
                class_name.as_ptr(),
                0, // Style: no visible style flags
                0,
                0,
                0,
                0,               // Position and size (0,0 to make it effectively invisible)
                ptr::null_mut(), // No parent (top-level window, but hidden)
                ptr::null_mut(), // No menu
                h_instance,
                ptr::null(),
            );

            if hwnd.is_null() {
                let error = windows_sys::Win32::Foundation::GetLastError();
                return Err(format!(
                    "Failed to create window for media controls (error code: {})",
                    error
                ));
            }

            tracing::info!("Created hidden HWND for SMTC: {:?}", hwnd);
            Some(hwnd)
        }
    };

    #[cfg(target_os = "windows")]
    let config = PlatformConfig {
        dbus_name: "music_minder",
        display_name: "Music Minder",
        hwnd,
    };

    #[cfg(not(target_os = "windows"))]
    let config = PlatformConfig {
        dbus_name: "music_minder",
        display_name: "Music Minder",
        hwnd: None,
    };

    let mut controls = MediaControls::new(config)
        .map_err(|e| format!("Failed to create media controls: {:?}", e))?;

    // Set up event handler
    let tx = command_tx.clone();
    controls
        .attach(move |event: MediaControlEvent| {
            tracing::debug!("SMTC event received: {:?}", event);
            let cmd = match event {
                MediaControlEvent::Play => MediaControlCommand::Play,
                MediaControlEvent::Pause => MediaControlCommand::Pause,
                MediaControlEvent::Toggle => MediaControlCommand::Toggle,
                MediaControlEvent::Stop => MediaControlCommand::Stop,
                MediaControlEvent::Next => MediaControlCommand::Next,
                MediaControlEvent::Previous => MediaControlCommand::Previous,
                MediaControlEvent::Seek(dir) => MediaControlCommand::SeekRelative(dir),
                MediaControlEvent::SeekBy(dir, _dur) => {
                    // Convert relative seek to absolute if needed
                    // For now, treat as relative
                    MediaControlCommand::SeekRelative(dir)
                }
                MediaControlEvent::SetPosition(pos) => MediaControlCommand::Seek(pos.0),
                MediaControlEvent::SetVolume(_) => return, // Volume handled separately
                MediaControlEvent::OpenUri(_) => return,   // Not supported
                MediaControlEvent::Raise => return,        // Window management
                MediaControlEvent::Quit => return,         // App quit
            };
            if let Err(e) = tx.send(cmd) {
                tracing::warn!("Failed to send media control command: {}", e);
            }
        })
        .map_err(|e| format!("Failed to attach event handler: {:?}", e))?;

    // Set initial metadata so SMTC knows we're a media app
    controls
        .set_metadata(MediaMetadata {
            title: Some("Music Minder"),
            artist: Some("Ready to play"),
            album: None,
            duration: None,
            cover_url: None,
        })
        .map_err(|e| format!("Failed to set initial metadata: {:?}", e))?;

    // Windows SMTC quirk: We need to cycle through states to fully activate the session.
    // The first button press often just "wakes up" the session without triggering an event.
    // By setting Playing then Paused, we ensure the session is fully active.
    controls
        .set_playback(MediaPlayback::Playing { progress: None })
        .map_err(|e| format!("Failed to set initial playing state: {:?}", e))?;

    // Small delay to let Windows process the state change
    std::thread::sleep(std::time::Duration::from_millis(50));

    // Now set to Paused - this ensures buttons are responsive from the first press
    controls
        .set_playback(MediaPlayback::Paused { progress: None })
        .map_err(|e| format!("Failed to set playback state: {:?}", e))?;

    tracing::info!("Media controls initialized");

    // Event loop - pump Windows messages frequently for responsive media keys
    loop {
        // Pump Windows message queue FIRST to process media key events
        // This is critical - events won't be received without pumping
        #[cfg(target_os = "windows")]
        for _ in 0..5 {
            pump_windows_messages();
            std::thread::sleep(std::time::Duration::from_millis(2));
        }

        // Check for updates with a short timeout
        match update_rx.recv_timeout(std::time::Duration::from_millis(10)) {
            Ok(MediaControlsUpdate::Metadata(meta)) => {
                tracing::debug!(
                    "SMTC thread received metadata: {:?} - {:?}",
                    meta.artist,
                    meta.title
                );
                let cover_url = meta
                    .cover_path
                    .as_ref()
                    .map(|p| format!("file://{}", p.to_string_lossy().replace('\\', "/")));

                let metadata = MediaMetadata {
                    title: meta.title.as_deref(),
                    artist: meta.artist.as_deref(),
                    album: meta.album.as_deref(),
                    duration: meta.duration,
                    cover_url: cover_url.as_deref(),
                };

                if let Err(e) = controls.set_metadata(metadata) {
                    tracing::warn!("Failed to set SMTC metadata: {:?}", e);
                } else {
                    tracing::debug!("SMTC metadata updated successfully");
                }
            }
            Ok(MediaControlsUpdate::PlaybackState(state)) => {
                tracing::debug!("SMTC thread received playback state: {:?}", state);
                let playback = match state {
                    MediaPlaybackState::Playing => MediaPlayback::Playing { progress: None },
                    MediaPlaybackState::Paused => MediaPlayback::Paused { progress: None },
                    MediaPlaybackState::Stopped => MediaPlayback::Stopped,
                };

                if let Err(e) = controls.set_playback(playback) {
                    tracing::debug!("Failed to set playback state: {:?}", e);
                }
            }
            Ok(MediaControlsUpdate::Position(_pos)) => {
                // Update position for seek bar in system UI
                // Note: souvlaki combines this with playback state
                // For now we'll update on next PlaybackState change
            }
            Ok(MediaControlsUpdate::Shutdown) => {
                tracing::info!("Media controls shutting down");
                break;
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                // Continue pumping messages
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                // Channel closed, exit
                break;
            }
        }
    }

    Ok(())
}

/// Pump the Windows message queue to process media key events.
#[cfg(target_os = "windows")]
fn pump_windows_messages() {
    use std::mem::MaybeUninit;
    use std::ptr;

    unsafe {
        let mut msg = MaybeUninit::uninit();
        // Process all pending messages without blocking
        while windows_sys::Win32::UI::WindowsAndMessaging::PeekMessageW(
            msg.as_mut_ptr(),
            ptr::null_mut(), // All windows
            0,
            0,
            windows_sys::Win32::UI::WindowsAndMessaging::PM_REMOVE,
        ) != 0
        {
            let msg = msg.assume_init_ref();
            windows_sys::Win32::UI::WindowsAndMessaging::TranslateMessage(msg);
            windows_sys::Win32::UI::WindowsAndMessaging::DispatchMessageW(msg);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata_builder() {
        let meta = MediaControlsMetadata::with_title("Test Song")
            .artist("Test Artist")
            .album("Test Album")
            .duration(Duration::from_secs(180));

        assert_eq!(meta.title.as_deref(), Some("Test Song"));
        assert_eq!(meta.artist.as_deref(), Some("Test Artist"));
        assert_eq!(meta.album.as_deref(), Some("Test Album"));
        assert_eq!(meta.duration, Some(Duration::from_secs(180)));
    }
}

// ============================================================================
// Defensive Tests - Verify souvlaki API contracts used by this module
// ============================================================================

#[cfg(test)]
mod souvlaki_api_tests {
    use souvlaki::{
        MediaControlEvent, MediaControls, MediaMetadata, MediaPlayback, PlatformConfig,
        SeekDirection,
    };
    use std::time::Duration;

    /// Verify PlatformConfig can be constructed with our fields
    #[test]
    fn test_platform_config_construction() {
        let config = PlatformConfig {
            dbus_name: "test_app",
            display_name: "Test App",
            hwnd: None,
        };

        // Verify the fields we set
        assert_eq!(config.dbus_name, "test_app");
        assert_eq!(config.display_name, "Test App");
        assert!(config.hwnd.is_none());
    }

    /// Verify MediaMetadata can be constructed with our fields
    #[test]
    fn test_media_metadata_construction() {
        let metadata = MediaMetadata {
            title: Some("Test Title"),
            artist: Some("Test Artist"),
            album: Some("Test Album"),
            duration: Some(Duration::from_secs(180)),
            cover_url: Some("file:///path/to/cover.jpg"),
        };

        assert_eq!(metadata.title, Some("Test Title"));
        assert_eq!(metadata.artist, Some("Test Artist"));
        assert_eq!(metadata.album, Some("Test Album"));
        assert_eq!(metadata.duration, Some(Duration::from_secs(180)));
        assert_eq!(metadata.cover_url, Some("file:///path/to/cover.jpg"));
    }

    /// Verify MediaPlayback enum variants we use
    #[test]
    fn test_media_playback_variants() {
        // We use these three variants
        let _playing = MediaPlayback::Playing { progress: None };
        let _paused = MediaPlayback::Paused { progress: None };
        let _stopped = MediaPlayback::Stopped;

        // Verify with progress
        let _playing_with_progress = MediaPlayback::Playing {
            progress: Some(souvlaki::MediaPosition(Duration::from_secs(30))),
        };
    }

    /// Verify MediaControlEvent variants we handle
    #[test]
    fn test_media_control_event_variants() {
        // We match on these event variants in our event handler
        fn handle_event(event: MediaControlEvent) -> &'static str {
            match event {
                MediaControlEvent::Play => "play",
                MediaControlEvent::Pause => "pause",
                MediaControlEvent::Toggle => "toggle",
                MediaControlEvent::Stop => "stop",
                MediaControlEvent::Next => "next",
                MediaControlEvent::Previous => "previous",
                MediaControlEvent::Seek(_) => "seek",
                MediaControlEvent::SeekBy(_, _) => "seek_by",
                MediaControlEvent::SetPosition(_) => "set_position",
                MediaControlEvent::SetVolume(_) => "set_volume",
                MediaControlEvent::OpenUri(_) => "open_uri",
                MediaControlEvent::Raise => "raise",
                MediaControlEvent::Quit => "quit",
            }
        }

        // Verify each variant we use
        assert_eq!(handle_event(MediaControlEvent::Play), "play");
        assert_eq!(handle_event(MediaControlEvent::Pause), "pause");
        assert_eq!(handle_event(MediaControlEvent::Toggle), "toggle");
        assert_eq!(handle_event(MediaControlEvent::Stop), "stop");
        assert_eq!(handle_event(MediaControlEvent::Next), "next");
        assert_eq!(handle_event(MediaControlEvent::Previous), "previous");
    }

    /// Verify SeekDirection enum exists and we can use it
    #[test]
    fn test_seek_direction_exists() {
        // SeekDirection is used in SeekBy and Seek events
        let _forward = SeekDirection::Forward;
        let _backward = SeekDirection::Backward;

        // Verify Debug impl (we use {:?} in debug logging)
        let debug_str = format!("{:?}", SeekDirection::Forward);
        assert!(!debug_str.is_empty());
    }

    /// Verify MediaPosition wrapper type exists
    #[test]
    fn test_media_position_exists() {
        use souvlaki::MediaPosition;

        let pos = MediaPosition(Duration::from_secs(60));

        // MediaPosition is a newtype around Duration
        assert_eq!(pos.0, Duration::from_secs(60));
    }

    /// Verify MediaControls can be created (compile-time check)
    /// Note: We can't actually create controls without a real window handle on Windows
    #[test]
    fn test_media_controls_new_signature() {
        // This test verifies the API signature exists
        // MediaControls::new(config) -> Result<MediaControls, Error>
        fn check_new_signature<F: Fn(PlatformConfig) -> Result<MediaControls, souvlaki::Error>>(
            _f: F,
        ) {
        }
        check_new_signature(MediaControls::new);
    }

    /// Verify the Error type we handle
    #[test]
    fn test_error_type_exists() {
        // We use {:?} formatting on errors
        // souvlaki::Error should implement Debug
        fn check_debug<T: std::fmt::Debug>() {}
        check_debug::<souvlaki::Error>();
    }
}

// ============================================================================
// Defensive Tests - Verify windows-sys API contracts used by this module
// ============================================================================

#[cfg(all(test, target_os = "windows"))]
mod windows_sys_api_tests {
    use std::ptr;

    /// Verify WNDCLASSEXW struct fields we use exist and have expected types
    #[test]
    fn test_wndclassexw_struct_fields() {
        use windows_sys::Win32::UI::WindowsAndMessaging::WNDCLASSEXW;

        // Construct a WNDCLASSEXW with all fields we use
        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: 0,
            lpfnWndProc: None, // Some(DefWindowProcW) in real code
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: ptr::null_mut(),
            hIcon: ptr::null_mut(),
            hCursor: ptr::null_mut(),
            hbrBackground: ptr::null_mut(),
            lpszMenuName: ptr::null(),
            lpszClassName: ptr::null(),
            hIconSm: ptr::null_mut(),
        };

        // Verify field access compiles
        assert_eq!(wc.cbSize, std::mem::size_of::<WNDCLASSEXW>() as u32);
        assert_eq!(wc.style, 0);
    }

    /// Verify RegisterClassExW function signature
    #[test]
    fn test_register_class_ex_w_signature() {
        use windows_sys::Win32::UI::WindowsAndMessaging::{RegisterClassExW, WNDCLASSEXW};

        // Verify the function accepts a pointer to WNDCLASSEXW and returns u16
        fn check_signature(_f: unsafe extern "system" fn(*const WNDCLASSEXW) -> u16) {}
        check_signature(RegisterClassExW);
    }

    /// Verify CreateWindowExW parameters we use exist
    #[test]
    fn test_create_window_ex_w_exists() {
        use windows_sys::Win32::UI::WindowsAndMessaging::CreateWindowExW;

        // Verify the function exists by taking its address
        // All HANDLE types are now *mut c_void in windows-sys 0.59+
        let _fn_ptr: unsafe extern "system" fn(
            u32,                      // dwExStyle
            *const u16,               // lpClassName
            *const u16,               // lpWindowName
            u32,                      // dwStyle
            i32,                      // x
            i32,                      // y
            i32,                      // nWidth
            i32,                      // nHeight
            *mut core::ffi::c_void,   // hWndParent (HWND)
            *mut core::ffi::c_void,   // hMenu (HMENU)
            *mut core::ffi::c_void,   // hInstance (HMODULE)
            *const core::ffi::c_void, // lpParam
        ) -> *mut core::ffi::c_void = CreateWindowExW;
    }

    /// Verify message pump functions exist
    #[test]
    fn test_message_pump_functions_exist() {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            DispatchMessageW, MSG, PM_REMOVE, PeekMessageW, TranslateMessage,
        };

        // Verify constants
        let _pm_remove: u32 = PM_REMOVE;

        // Verify MSG struct can be created
        let _msg: MSG = unsafe { std::mem::zeroed() };

        // Verify function signatures with extern "system"
        let _peek: unsafe extern "system" fn(
            *mut MSG,
            *mut core::ffi::c_void,
            u32,
            u32,
            u32,
        ) -> i32 = PeekMessageW;
        let _translate: unsafe extern "system" fn(*const MSG) -> i32 = TranslateMessage;
        let _dispatch: unsafe extern "system" fn(*const MSG) -> isize = DispatchMessageW;
    }

    /// Verify GetModuleHandleW exists
    #[test]
    fn test_get_module_handle_w_exists() {
        use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;

        // Verify the function exists with extern "system" calling convention
        let _fn_ptr: unsafe extern "system" fn(*const u16) -> *mut core::ffi::c_void =
            GetModuleHandleW;
    }

    /// Verify GetLastError exists
    #[test]
    fn test_get_last_error_exists() {
        use windows_sys::Win32::Foundation::GetLastError;

        // Verify the function exists and returns u32
        let _fn_ptr: unsafe extern "system" fn() -> u32 = GetLastError;
    }

    /// Verify DefWindowProcW exists for window procedure
    #[test]
    fn test_def_window_proc_w_exists() {
        use windows_sys::Win32::UI::WindowsAndMessaging::DefWindowProcW;

        // Verify the function exists with expected signature
        let _fn_ptr: unsafe extern "system" fn(
            *mut core::ffi::c_void, // hwnd (HWND)
            u32,                    // msg
            usize,                  // wparam (WPARAM)
            isize,                  // lparam (LPARAM)
        ) -> isize = DefWindowProcW;
    }
}
