#!/bin/bash
# Shell script to download yt-dlp and ffmpeg for Linux/macOS
# Usage: ./download-dependencies.sh [output-dir]

set -e

OUTPUT_DIR="${1:-./deps}"
OS_TYPE=$(uname -s)

echo "Downloading dependencies to: $OUTPUT_DIR"
echo "Operating System: $OS_TYPE"

# Create output directory
mkdir -p "$OUTPUT_DIR"

# Download yt-dlp
echo ""
echo "Downloading yt-dlp..."
if [ "$OS_TYPE" = "Darwin" ]; then
    # macOS
    YT_DLP_URL="https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp_macos"
    YT_DLP_PATH="$OUTPUT_DIR/yt-dlp"
else
    # Linux
    YT_DLP_URL="https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp"
    YT_DLP_PATH="$OUTPUT_DIR/yt-dlp"
fi

curl -L "$YT_DLP_URL" -o "$YT_DLP_PATH"
chmod +x "$YT_DLP_PATH"
echo "yt-dlp downloaded successfully to: $YT_DLP_PATH"

# Download ffmpeg
echo ""
echo "Downloading ffmpeg..."

if [ "$OS_TYPE" = "Darwin" ]; then
    # macOS - Download from official builds
    echo "For macOS, please install ffmpeg using Homebrew:"
    echo "  brew install ffmpeg"
    echo ""
    echo "Or download from: https://ffmpeg.org/download.html#build-mac"
elif [ "$OS_TYPE" = "Linux" ]; then
    # Linux - Download static build
    ARCH=$(uname -m)

    if [ "$ARCH" = "x86_64" ]; then
        FFMPEG_URL="https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-linux64-gpl.tar.xz"
        FFMPEG_ARCHIVE="$OUTPUT_DIR/ffmpeg.tar.xz"

        curl -L "$FFMPEG_URL" -o "$FFMPEG_ARCHIVE"
        echo "Extracting ffmpeg..."

        tar -xf "$FFMPEG_ARCHIVE" -C "$OUTPUT_DIR"

        # Find and copy binaries
        FFMPEG_BIN_DIR=$(find "$OUTPUT_DIR" -type d -name "bin" | head -n 1)
        if [ -n "$FFMPEG_BIN_DIR" ]; then
            cp "$FFMPEG_BIN_DIR/ffmpeg" "$OUTPUT_DIR/"
            cp "$FFMPEG_BIN_DIR/ffprobe" "$OUTPUT_DIR/"
            chmod +x "$OUTPUT_DIR/ffmpeg"
            chmod +x "$OUTPUT_DIR/ffprobe"
            echo "ffmpeg and ffprobe extracted to: $OUTPUT_DIR"

            # Cleanup
            rm -f "$FFMPEG_ARCHIVE"
            find "$OUTPUT_DIR" -mindepth 1 -maxdepth 1 -type d -exec rm -rf {} +
        else
            echo "Failed to find ffmpeg binaries in archive"
            exit 1
        fi
    else
        echo "Unsupported architecture: $ARCH"
        echo "Please install ffmpeg using your package manager:"
        echo "  sudo apt install ffmpeg  # Debian/Ubuntu"
        echo "  sudo dnf install ffmpeg  # Fedora"
        exit 1
    fi
fi

echo ""
echo "All dependencies downloaded successfully!"
echo "Location: $OUTPUT_DIR"
echo ""
echo "Contents:"
ls -lh "$OUTPUT_DIR"
