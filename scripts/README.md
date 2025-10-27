# Dependency Download Scripts

These scripts automatically download yt-dlp and ffmpeg binaries for bundling with Space Downloader.

## Usage

### Windows (Batch) - Recommended

```batch
REM Download to default location (.\deps)
.\download-dependencies.bat

REM Download to custom location
.\download-dependencies.bat "C:\path\to\output"
```

**Note:** The batch file (`.bat`) is the easiest option for Windows and doesn't require changing execution policies.

### Windows (PowerShell) - Alternative

```powershell
# Download to default location (.\deps)
.\download-dependencies.ps1

# Download to custom location
.\download-dependencies.ps1 -OutputDir "C:\path\to\output"

# If you get an execution policy error, use this instead:
PowerShell -ExecutionPolicy Bypass -File .\download-dependencies.ps1 -OutputDir ".\deps"
```

**Note:** If you encounter a "running scripts is disabled" error, Windows PowerShell execution policy is blocking the script. Use the `-ExecutionPolicy Bypass` option shown above, or run PowerShell as Administrator and execute:
```powershell
Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser
```

### Linux/macOS (Bash)

```bash
# Make script executable
chmod +x download-dependencies.sh

# Download to default location (./deps)
./download-dependencies.sh

# Download to custom location
./download-dependencies.sh /path/to/output
```

## Downloaded Files

### Windows
- `yt-dlp.exe` - Latest release from yt-dlp GitHub
- `ffmpeg.exe` - Latest Windows build from BtbN/FFmpeg-Builds
- `ffprobe.exe` - Included with ffmpeg

### Linux (x86_64)
- `yt-dlp` - Latest release from yt-dlp GitHub
- `ffmpeg` - Latest Linux static build from BtbN/FFmpeg-Builds
- `ffprobe` - Included with ffmpeg

### macOS
- `yt-dlp` - Latest macOS release from yt-dlp GitHub
- `ffmpeg` - Instructions provided for Homebrew installation

## Sources

- **yt-dlp**: https://github.com/yt-dlp/yt-dlp/releases
- **ffmpeg (Windows/Linux)**: https://github.com/BtbN/FFmpeg-Builds/releases
- **ffmpeg (macOS)**: https://ffmpeg.org/download.html or Homebrew

## Notes

- Scripts download the latest versions available
- Downloaded binaries are placed in the specified output directory
- On macOS, ffmpeg installation via Homebrew is recommended
- On Linux ARM architectures, system package manager installation is recommended

## Integration with Build Process

These scripts can be integrated into CI/CD workflows to automatically bundle dependencies:

```yaml
# GitHub Actions example
- name: Download dependencies
  run: |
    pwsh scripts/download-dependencies.ps1 -OutputDir ./bundle
    # or for Linux/macOS
    ./scripts/download-dependencies.sh ./bundle
```
