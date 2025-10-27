# Dependency Download Scripts

These scripts automatically download yt-dlp and ffmpeg binaries for bundling with Space Downloader.

## Usage

### Windows (PowerShell)

```powershell
# Download to default location (.\deps)
.\download-dependencies.ps1

# Download to custom location
.\download-dependencies.ps1 -OutputDir "C:\path\to\output"
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
