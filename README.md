# 🎵 Music Minder

A fast, native music library manager built with Rust. Scan, organize, enrich metadata, and play your entire collection with a beautiful, responsive interface.

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.75+-orange.svg)](https://www.rust-lang.org/)

## ✨ Features

- **📂 Smart Library Scanning** - Recursively scan directories for MP3, FLAC, OGG, WAV, and M4A files. Handles 10,000+ tracks with virtualized scrolling.

- **🏷️ Metadata Enrichment** - Audio fingerprinting via AcoustID, MusicBrainz lookups, and automatic cover art from Cover Art Archive.

- **📁 File Organization** - Pattern-based organization (Artist/Album/Track) with preview, undo support, and batch operations.

- **🎧 Audio Playback** - Low-latency playback with queue management, volume control, and real-time visualization (spectrum, waveform, VU meter).

- **🎛️ OS Integration** - Media key support (play/pause/next/prev), system overlay with track info, and Bluetooth/headphone button controls via Windows SMTC / Linux MPRIS / macOS MediaPlayer.

- **⚡ Native Performance** - Built with Rust for minimal memory usage and maximum speed. No Electron, no web views.

## 📸 Screenshots

### Coming soon

## 🚀 Installation

### Download

Download the latest release for your platform from the [Releases page](https://github.com/Hardcoreprawn/music-minder/releases).

### Build from Source

Requirements:

- Rust 1.75+
- On Windows: Visual Studio Build Tools
- On Linux: `libasound2-dev libdbus-1-dev pkg-config`

```bash
git clone https://github.com/Hardcoreprawn/music-minder.git
cd music-minder
cargo build --release
```

The binary will be at `target/release/music-minder` (or `.exe` on Windows).

## 🎮 Usage

### GUI Mode (default)

```bash
music-minder
```

### CLI Commands

```bash
# Scan a directory for music files
music-minder scan /path/to/music

# Identify a track using audio fingerprinting
music-minder identify track.mp3

# Enrich metadata for files in a directory
music-minder enrich /path/to/music --write

# Preview file organization without moving
music-minder organize /path/to/music --preview
```

## 🛠️ Tech Stack

| Component | Technology |
|-----------|------------|
| Language | Rust 2024 Edition |
| GUI | [Iced](https://iced.rs/) 0.13 |
| Audio Decode | [Symphonia](https://github.com/pdeljanov/Symphonia) |
| Audio Output | [CPAL](https://github.com/RustAudio/cpal) (WASAPI/CoreAudio/ALSA) |
| Database | SQLite via [SQLx](https://github.com/launchbadge/sqlx) |
| Async Runtime | [Tokio](https://tokio.rs/) |
| Metadata | [Lofty](https://github.com/Serial-ATA/lofty-rs) |
| Media Controls | [Souvlaki](https://github.com/Sinono3/souvlaki) |

## 📋 Roadmap

See [ROADMAP.md](docs/ROADMAP.md) for the full development roadmap.

### Current Phase: System Integration & Polish

- [x] OS media controls (SMTC/MPRIS)
- [x] Unified playback architecture
- [ ] Keyboard shortcuts
- [ ] Search/filter library

**Upcoming:**

- Equalizer with presets
- Gapless playback
- Playlist management
- Scrobbling (Last.fm / ListenBrainz)

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## � Code Signing Policy

Free code signing provided by [SignPath.io](https://about.signpath.io/), certificate by [SignPath Foundation](https://signpath.org/)

- **Committers / Reviewers / Approvers:** [@Hardcoreprawn](https://github.com/Hardcoreprawn)

**Privacy:** This program will not transfer any information to other networked systems unless specifically requested by the user (metadata enrichment via MusicBrainz/AcoustID APIs).

## �🙏 Acknowledgments

- [Symphonia](https://github.com/pdeljanov/Symphonia) for excellent audio decoding
- [Iced](https://iced.rs/) for the beautiful GUI framework
- [MusicBrainz](https://musicbrainz.org/) and [AcoustID](https://acoustid.org/) for metadata services
- [Cover Art Archive](https://coverartarchive.org/) for album artwork
