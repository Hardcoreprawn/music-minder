//! Audio Device Diagnostics
//!
//! Enumerates audio devices and their capabilities.
//! Uses Windows Multimedia APIs for device enumeration.

/// Audio device information
#[derive(Debug, Clone)]
pub struct AudioDeviceInfo {
    /// Device name
    pub name: String,
    /// Device type (output, input)
    pub device_type: AudioDeviceType,
    /// Is this the default device?
    pub is_default: bool,
    /// Supported sample rates (if known)
    pub sample_rates: Vec<u32>,
    /// Number of channels
    pub channels: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioDeviceType {
    Output,
    Input,
}

impl AudioDeviceInfo {
    /// Enumerate all audio devices
    #[cfg(windows)]
    pub fn enumerate() -> Vec<Self> {
        let mut devices = Vec::new();

        // Enumerate output devices using waveOut
        devices.extend(enumerate_wave_out_devices());

        // Enumerate input devices using waveIn
        devices.extend(enumerate_wave_in_devices());

        devices
    }

    #[cfg(not(windows))]
    pub fn enumerate() -> Vec<Self> {
        Vec::new()
    }
}

#[cfg(windows)]
fn enumerate_wave_out_devices() -> Vec<AudioDeviceInfo> {
    use std::mem::MaybeUninit;

    #[repr(C)]
    #[allow(non_snake_case, clippy::upper_case_acronyms)]
    struct WAVEOUTCAPSW {
        wMid: u16,
        wPid: u16,
        vDriverVersion: u32,
        szPname: [u16; 32],
        dwFormats: u32,
        wChannels: u16,
        wReserved1: u16,
        dwSupport: u32,
    }

    #[link(name = "winmm")]
    unsafe extern "system" {
        fn waveOutGetNumDevs() -> u32;
        fn waveOutGetDevCapsW(uDeviceID: usize, pwoc: *mut WAVEOUTCAPSW, cbwoc: u32) -> u32;
    }

    let mut devices = Vec::new();

    unsafe {
        let num_devs = waveOutGetNumDevs();

        for i in 0..num_devs {
            let mut caps = MaybeUninit::<WAVEOUTCAPSW>::uninit();

            if waveOutGetDevCapsW(
                i as usize,
                caps.as_mut_ptr(),
                std::mem::size_of::<WAVEOUTCAPSW>() as u32,
            ) == 0
            {
                let caps = caps.assume_init();

                // Convert name from UTF-16
                let name_len = caps.szPname.iter().position(|&c| c == 0).unwrap_or(32);
                let name = String::from_utf16_lossy(&caps.szPname[..name_len]);

                // Parse supported sample rates from dwFormats
                let sample_rates = parse_wave_formats(caps.dwFormats);

                devices.push(AudioDeviceInfo {
                    name,
                    device_type: AudioDeviceType::Output,
                    is_default: i == 0, // First device is typically default
                    sample_rates,
                    channels: Some(caps.wChannels as u32),
                });
            }
        }
    }

    devices
}

#[cfg(windows)]
fn enumerate_wave_in_devices() -> Vec<AudioDeviceInfo> {
    use std::mem::MaybeUninit;

    #[repr(C)]
    #[allow(non_snake_case, clippy::upper_case_acronyms)]
    struct WAVEINCAPSW {
        wMid: u16,
        wPid: u16,
        vDriverVersion: u32,
        szPname: [u16; 32],
        dwFormats: u32,
        wChannels: u16,
    }

    #[link(name = "winmm")]
    unsafe extern "system" {
        fn waveInGetNumDevs() -> u32;
        fn waveInGetDevCapsW(uDeviceID: usize, pwic: *mut WAVEINCAPSW, cbwic: u32) -> u32;
    }

    let mut devices = Vec::new();

    unsafe {
        let num_devs = waveInGetNumDevs();

        for i in 0..num_devs {
            let mut caps = MaybeUninit::<WAVEINCAPSW>::uninit();

            if waveInGetDevCapsW(
                i as usize,
                caps.as_mut_ptr(),
                std::mem::size_of::<WAVEINCAPSW>() as u32,
            ) == 0
            {
                let caps = caps.assume_init();

                let name_len = caps.szPname.iter().position(|&c| c == 0).unwrap_or(32);
                let name = String::from_utf16_lossy(&caps.szPname[..name_len]);

                let sample_rates = parse_wave_formats(caps.dwFormats);

                devices.push(AudioDeviceInfo {
                    name,
                    device_type: AudioDeviceType::Input,
                    is_default: i == 0,
                    sample_rates,
                    channels: Some(caps.wChannels as u32),
                });
            }
        }
    }

    devices
}

#[cfg(windows)]
fn parse_wave_formats(formats: u32) -> Vec<u32> {
    // WAVE_FORMAT_* flags indicate supported sample rates
    let mut rates = Vec::new();

    // 11.025 kHz
    if formats & 0x00000001 != 0
        || formats & 0x00000002 != 0
        || formats & 0x00000004 != 0
        || formats & 0x00000008 != 0
    {
        rates.push(11025);
    }

    // 22.05 kHz
    if formats & 0x00000010 != 0
        || formats & 0x00000020 != 0
        || formats & 0x00000040 != 0
        || formats & 0x00000080 != 0
    {
        rates.push(22050);
    }

    // 44.1 kHz
    if formats & 0x00000100 != 0
        || formats & 0x00000200 != 0
        || formats & 0x00000400 != 0
        || formats & 0x00000800 != 0
    {
        rates.push(44100);
    }

    // 48 kHz
    if formats & 0x00001000 != 0
        || formats & 0x00002000 != 0
        || formats & 0x00004000 != 0
        || formats & 0x00008000 != 0
    {
        rates.push(48000);
    }

    // 96 kHz
    if formats & 0x00010000 != 0
        || formats & 0x00020000 != 0
        || formats & 0x00040000 != 0
        || formats & 0x00080000 != 0
    {
        rates.push(96000);
    }

    rates
}

/// Get the default audio output device name
#[cfg(windows)]
pub fn get_default_output_device() -> Option<String> {
    AudioDeviceInfo::enumerate()
        .into_iter()
        .find(|d| d.device_type == AudioDeviceType::Output && d.is_default)
        .map(|d| d.name)
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[test]
    #[cfg(windows)]
    fn test_enumerate_audio_devices() {
        let devices = AudioDeviceInfo::enumerate();

        println!("Found {} audio devices:", devices.len());
        for device in &devices {
            let type_str = match device.device_type {
                AudioDeviceType::Output => "Output",
                AudioDeviceType::Input => "Input",
            };
            let default_str = if device.is_default { " [DEFAULT]" } else { "" };

            println!("  {} ({}){}", device.name, type_str, default_str);
            if !device.sample_rates.is_empty() {
                println!("    Sample rates: {:?}", device.sample_rates);
            }
            if let Some(ch) = device.channels {
                println!("    Channels: {}", ch);
            }
        }
    }

    #[test]
    #[cfg(windows)]
    fn test_get_default_output() {
        let default = get_default_output_device();
        assert!(default.is_some());
        println!("Default output: {}", default.unwrap());
    }
}
