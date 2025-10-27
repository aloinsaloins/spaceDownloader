# PowerShell script to download yt-dlp and ffmpeg for Windows
# Usage: .\download-dependencies.ps1

param(
    [string]$OutputDir = ".\deps"
)

$ErrorActionPreference = "Stop"

Write-Host "Downloading dependencies to: $OutputDir" -ForegroundColor Green

# Create output directory
New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null

# yt-dlp download
$ytDlpUrl = "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe"
$ytDlpPath = Join-Path $OutputDir "yt-dlp.exe"

Write-Host "`nDownloading yt-dlp..." -ForegroundColor Cyan
try {
    Invoke-WebRequest -Uri $ytDlpUrl -OutFile $ytDlpPath -UseBasicParsing
    Write-Host "yt-dlp downloaded successfully to: $ytDlpPath" -ForegroundColor Green
} catch {
    Write-Host "Failed to download yt-dlp: $_" -ForegroundColor Red
    exit 1
}

# ffmpeg download
$ffmpegUrl = "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-win64-gpl.zip"
$ffmpegZip = Join-Path $OutputDir "ffmpeg.zip"
$ffmpegExtractDir = Join-Path ([System.IO.Path]::GetTempPath()) "ffmpeg-extract-$(Get-Random)"

Write-Host "`nDownloading ffmpeg..." -ForegroundColor Cyan
try {
    Invoke-WebRequest -Uri $ffmpegUrl -OutFile $ffmpegZip -UseBasicParsing
    Write-Host "ffmpeg downloaded successfully" -ForegroundColor Green

    Write-Host "Extracting ffmpeg..." -ForegroundColor Cyan
    Expand-Archive -Path $ffmpegZip -DestinationPath $ffmpegExtractDir -Force

    # Find and copy ffmpeg.exe and ffprobe.exe
    $ffmpegBinDir = Get-ChildItem -Path $ffmpegExtractDir -Recurse -Directory | Where-Object { $_.Name -eq "bin" } | Select-Object -First 1

    if ($ffmpegBinDir) {
        Copy-Item (Join-Path $ffmpegBinDir.FullName "ffmpeg.exe") -Destination $OutputDir -Force
        Copy-Item (Join-Path $ffmpegBinDir.FullName "ffprobe.exe") -Destination $OutputDir -Force
        Write-Host "ffmpeg.exe and ffprobe.exe extracted to: $OutputDir" -ForegroundColor Green
    } else {
        Write-Host "Failed to find ffmpeg binaries in archive" -ForegroundColor Red
        exit 1
    }

    # Cleanup
    Write-Host "Cleaning up temporary files..." -ForegroundColor Cyan
    Remove-Item $ffmpegZip -Force -ErrorAction SilentlyContinue
    if (Test-Path $ffmpegExtractDir) {
        Remove-Item $ffmpegExtractDir -Recurse -Force -ErrorAction SilentlyContinue
    }
    Write-Host "Cleanup completed" -ForegroundColor Green

} catch {
    Write-Host "Failed to download/extract ffmpeg: $_" -ForegroundColor Red
    # Attempt cleanup even on error
    if (Test-Path $ffmpegZip) { Remove-Item $ffmpegZip -Force -ErrorAction SilentlyContinue }
    if (Test-Path $ffmpegExtractDir) { Remove-Item $ffmpegExtractDir -Recurse -Force -ErrorAction SilentlyContinue }
    exit 1
}

Write-Host "`nAll dependencies downloaded successfully!" -ForegroundColor Green
Write-Host "Location: $OutputDir" -ForegroundColor Yellow
Write-Host "`nContents:" -ForegroundColor Yellow
Get-ChildItem $OutputDir | ForEach-Object {
    Write-Host "  - $($_.Name) ($([math]::Round($_.Length/1MB, 2)) MB)" -ForegroundColor White
}
