# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Space Downloader - A desktop GUI application for downloading Twitter/X Space audio archives using yt-dlp.

- **Target OS**: Windows 10/11, macOS 13+, Linux x86_64
- **Tech Stack**: Rust + iced GUI framework
- **Audio Extraction**: yt-dlp (must be installed separately)
- **Architecture**: Workspace with core library and GUI binary

## Architecture

### Workspace Structure
```
spaceDownloader/
├─ space-downloader-core/    # Core library with business logic
│  └─ src/
│     ├─ config.rs           # Configuration management
│     ├─ dependency.rs       # yt-dlp/ffmpeg dependency checking
│     ├─ download.rs         # Download service and job management
│     ├─ error.rs           # Error types
│     ├─ history.rs         # Download history (SQLite)
│     └─ logging.rs         # Logging utilities
└─ space-downloader-gui/     # GUI application
   └─ src/
      ├─ main.rs             # Main app and UI logic
      └─ localization.rs    # i18n support

```

### Core Components

1. **DownloaderService** (`space-downloader-core`)
   - Manages download queue and concurrent job execution
   - Executes yt-dlp commands asynchronously
   - Provides progress tracking via watch channels
   - Supports cancellation via CancellationToken

2. **GUI Application** (`space-downloader-gui`)
   - Built with iced 0.13 framework
   - Message-driven architecture
   - Real-time progress updates via subscription
   - Supports multiple concurrent downloads

### Data Flow
1. User inputs URL → GUI validates and creates DownloadRequest
2. Request queued in DownloaderService → Returns JobHandle
3. Background task executes yt-dlp → Sends progress via channels
4. GUI polls channels on tick (500ms) → Updates UI
5. Completion/failure → Updates history database

## Development Commands

### Build & Run
```bash
# Development mode
cargo run --package space-downloader-gui

# Release build
cargo build --release --package space-downloader-gui

# Run tests
cargo test --workspace

# Format code
cargo fmt

# Lint
cargo clippy -- -D warnings
```

### Dependencies
External tools required (not bundled):
- `yt-dlp` - For downloading media (macOS: install via Homebrew: `brew install yt-dlp`)
- `ffmpeg` - For audio conversion (macOS: install via Homebrew: `brew install ffmpeg`)

**Important for macOS**: The application requires Homebrew-installed `yt-dlp` and will not automatically download binaries. This avoids code signing issues with bundled binaries.

## Key Implementation Details

### Download Process
- Command: `yt-dlp --extract-audio --audio-format {format} --audio-quality 0 --write-info-json --no-playlist --output "{output}" "{url}"`
- Progress parsing: Regex extracts percentage, speed, ETA from stdout
- Error handling: Non-zero exit codes and stderr captured

### Configuration
Platform-specific configuration file locations:
- **macOS**: `~/Library/Application Support/com.space-downloader.space-downloader/space_downloader.toml`
- **Windows**: `%APPDATA%\space-downloader\space-downloader\space_downloader.toml`
- **Linux**: `~/.config/space-downloader/space_downloader.toml`

Settings: Output directory, audio format, retry count, concurrency limit
- Can be modified via GUI settings screen (future feature)

### History Storage
Platform-specific SQLite database locations:
- **macOS**: `~/Library/Application Support/com.space-downloader.space-downloader/history.db`
- **Windows**: `%APPDATA%\space-downloader\space-downloader\history.db`
- **Linux**: `~/.local/share/space-downloader/history.db`

Tracks: URL, title, status, timestamps, file paths, error messages

### Logging
Platform-specific log file locations:
- **macOS**: `~/Library/Application Support/com.space-downloader.space-downloader/logs/`
- **Windows**: `%APPDATA%\space-downloader\space-downloader\logs\`
- **Linux**: `~/.local/share/space-downloader/logs/`

### Default Download Directory
Platform-specific default download locations:
- **macOS**: `~/Downloads`
- **Windows**: Executable directory
- **Linux**: `~/Downloads` (if exists, otherwise current directory)

### Localization
- Uses fluent for i18n
- Supported languages: en-US, ja-JP
- Language files in `locales/` directory

## Current Status

### Implemented
- ✅ Core download service with yt-dlp integration
- ✅ Job queue management with concurrent execution
- ✅ Progress tracking and cancellation
- ✅ Basic GUI with URL input and download display
- ✅ Real-time progress updates
- ✅ Download history persistence
- ✅ Configuration management
- ✅ Error handling framework

### TODO
- [ ] Settings screen UI
- [ ] History list UI
- [ ] Dependency check on startup
- [ ] Drag & drop URL support
- [ ] OS notifications on completion
- [ ] Retry logic for failed downloads
- [ ] Cookie file support for authentication
- [ ] Log rotation
- [ ] Installer/packaging scripts

## Testing Approach

### Unit Tests
- Mock yt-dlp responses for download logic
- Test configuration loading/saving
- Test progress parsing regex
- Test history database operations

### Integration Tests
- End-to-end download with mock server
- GUI state transitions
- Error recovery scenarios

### Manual Testing
- Test on Windows, macOS, Linux
- Test with various Space URLs
- Test cancellation at different stages
- Test with missing dependencies

## Common Issues & Solutions

### yt-dlp not found
- Ensure yt-dlp is installed: `pip install yt-dlp`
- Add to PATH or specify full path in settings

### ffmpeg not found
- Install ffmpeg from system package manager
- Windows: Use ffmpeg builds from gyan.dev

### Download fails with 403
- May need authentication cookies
- Export cookies from browser, specify file in settings

## Code Style Guidelines

- Use `rustfmt` for formatting
- Follow Rust API guidelines
- Keep async boundaries clear
- Use `tracing` for logging
- Handle errors with `thiserror`
- Validate user input at GUI layer