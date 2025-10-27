@echo off
REM Batch script to download yt-dlp and ffmpeg for Windows
REM Usage: download-dependencies.bat [output-dir]
REM Default output directory is .\deps

setlocal enabledelayedexpansion

set "OUTPUT_DIR=%~1"
if "%OUTPUT_DIR%"=="" set "OUTPUT_DIR=.\deps"

echo ========================================
echo Downloading dependencies to: %OUTPUT_DIR%
echo ========================================
echo.

REM Create output directory
if not exist "%OUTPUT_DIR%" mkdir "%OUTPUT_DIR%"

REM Download yt-dlp
echo [1/2] Downloading yt-dlp...
powershell -NoProfile -ExecutionPolicy Bypass -Command "& { Invoke-WebRequest -Uri 'https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe' -OutFile '%OUTPUT_DIR%\yt-dlp.exe' -UseBasicParsing }"
if errorlevel 1 (
    echo ERROR: Failed to download yt-dlp
    exit /b 1
)
echo SUCCESS: yt-dlp downloaded

REM Download ffmpeg
echo.
echo [2/2] Downloading ffmpeg...
set "FFMPEG_ZIP=%OUTPUT_DIR%\ffmpeg.zip"
set "TEMP_EXTRACT=%TEMP%\ffmpeg-extract-%RANDOM%"

powershell -NoProfile -ExecutionPolicy Bypass -Command "& { Invoke-WebRequest -Uri 'https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-win64-gpl.zip' -OutFile '%FFMPEG_ZIP%' -UseBasicParsing }"
if errorlevel 1 (
    echo ERROR: Failed to download ffmpeg
    exit /b 1
)
echo SUCCESS: ffmpeg archive downloaded

echo Extracting ffmpeg...
powershell -NoProfile -ExecutionPolicy Bypass -Command "& { Expand-Archive -Path '%FFMPEG_ZIP%' -DestinationPath '%TEMP_EXTRACT%' -Force }"
if errorlevel 1 (
    echo ERROR: Failed to extract ffmpeg
    exit /b 1
)

REM Find and copy ffmpeg.exe and ffprobe.exe
for /r "%TEMP_EXTRACT%" %%F in (ffmpeg.exe) do (
    copy /Y "%%F" "%OUTPUT_DIR%\ffmpeg.exe" >nul
    copy /Y "%%~dpFffprobe.exe" "%OUTPUT_DIR%\ffprobe.exe" >nul
    goto :found
)
:found

REM Cleanup
del /f /q "%FFMPEG_ZIP%" >nul 2>&1
rmdir /s /q "%TEMP_EXTRACT%" >nul 2>&1

echo SUCCESS: ffmpeg and ffprobe extracted

echo.
echo ========================================
echo All dependencies downloaded successfully!
echo Location: %OUTPUT_DIR%
echo ========================================
echo.
echo Contents:
dir /b "%OUTPUT_DIR%"

endlocal
