use std::path::{Path, PathBuf};
use std::time::Duration;

use futures_util::StreamExt;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tokio::time::timeout;

use crate::config::AdvancedSettings;
use crate::error::DependencyError;

#[derive(Debug, Clone)]
pub struct DependencyCheck {
    pub binary: String,
    pub available: bool,
    pub version: Option<String>,
    pub path: Option<PathBuf>,
    pub error: Option<String>,
}

impl DependencyCheck {
    pub fn missing(binary: &str, error: Option<String>) -> Self {
        Self {
            binary: binary.to_string(),
            available: false,
            version: None,
            path: None,
            error,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DependencyStatus {
    pub yt_dlp: DependencyCheck,
    pub ffmpeg: DependencyCheck,
}

impl DependencyStatus {
    pub fn all_available(&self) -> bool {
        self.yt_dlp.available && self.ffmpeg.available
    }
}

pub async fn check_dependencies(
    settings: &AdvancedSettings,
) -> Result<DependencyStatus, DependencyError> {
    let yt_dlp = check_binary(
        settings.yt_dlp_path.to_str().unwrap_or("yt-dlp"),
        &["--version"],
    )
    .await?;

    let ffmpeg = check_binary("ffmpeg", &["-version"]).await?;

    Ok(DependencyStatus { yt_dlp, ffmpeg })
}

async fn check_binary(binary: &str, args: &[&str]) -> Result<DependencyCheck, DependencyError> {
    let candidate = PathBuf::from(binary);
    let resolved = resolve_binary(&candidate);

    if resolved.is_none() {
        return Ok(DependencyCheck::missing(
            binary,
            Some("command not found".to_string()),
        ));
    }

    let command_path = resolved.unwrap();
    let mut command = Command::new(&command_path);
    command.args(args).kill_on_drop(true);

    match timeout(Duration::from_secs(5), command.output()).await {
        Err(_) => Ok(DependencyCheck::missing(
            binary,
            Some("version check timed out".to_string()),
        )),
        Ok(Err(error)) => {
            if error.kind() == std::io::ErrorKind::NotFound {
                Ok(DependencyCheck::missing(
                    binary,
                    Some("command not found".to_string()),
                ))
            } else {
                Err(DependencyError::Spawn {
                    binary: binary.to_string(),
                    source: error,
                })
            }
        }
        Ok(Ok(output)) => {
            if !output.status.success() {
                return Ok(DependencyCheck {
                    binary: binary.to_string(),
                    available: false,
                    version: None,
                    path: Some(command_path.clone()),
                    error: Some(String::from_utf8_lossy(&output.stderr).to_string()),
                });
            }

            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let version_text = if stdout.trim().is_empty() {
                stderr.trim()
            } else {
                stdout.trim()
            };

            Ok(DependencyCheck {
                binary: binary.to_string(),
                available: true,
                version: parse_version(version_text).map(|s| s.to_string()),
                path: Some(command_path),
                error: None,
            })
        }
    }
}

/// Get the directory where the current executable is located
pub fn get_executable_dir() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|exe_path| exe_path.parent().map(|p| p.to_path_buf()))
}

/// Resolve binary path with the following priority:
/// 1. If candidate is an absolute/relative path, check if it exists
/// 2. Check in the same directory as the executable
/// 3. Check in the dependencies directory (for downloaded binaries)
/// 4. Check in PATH
pub fn resolve_binary(candidate: &Path) -> Option<PathBuf> {
    // If candidate is a multi-component path, treat it as absolute/relative path
    if candidate.components().count() > 1 {
        if candidate.exists() {
            return Some(candidate.to_path_buf());
        } else {
            return None;
        }
    }

    // Get the binary filename
    let binary_name = candidate.file_name()?;

    // Priority 1: Check in executable directory
    if let Some(exe_dir) = get_executable_dir() {
        // Windows: check both with and without .exe extension
        #[cfg(target_os = "windows")]
        {
            let with_exe = exe_dir.join(format!("{}.exe", binary_name.to_string_lossy()));
            if with_exe.exists() {
                return Some(with_exe);
            }
        }

        let bundled_path = exe_dir.join(binary_name);
        if bundled_path.exists() {
            return Some(bundled_path);
        }
    }

    // Priority 2: Check in dependencies directory (for downloaded binaries)
    if let Some(deps_dir) = get_dependencies_dir() {
        #[cfg(target_os = "windows")]
        {
            let with_exe = deps_dir.join(format!("{}.exe", binary_name.to_string_lossy()));
            if with_exe.exists() {
                return Some(with_exe);
            }
        }

        let downloaded_path = deps_dir.join(binary_name);
        if downloaded_path.exists() {
            return Some(downloaded_path);
        }
    }

    // Priority 3: Check in PATH
    which::which(candidate).ok()
}

fn parse_version(text: &str) -> Option<&str> {
    if text.is_empty() {
        None
    } else if let Some(first_line) = text.lines().next() {
        Some(first_line)
    } else {
        None
    }
}

/// Get the directory for storing downloaded dependencies
pub fn get_dependencies_dir() -> Option<PathBuf> {
    directories::ProjectDirs::from("com", "space-downloader", "space-downloader")
        .map(|proj_dirs| proj_dirs.data_dir().join("bin"))
}

/// Get the expected path for yt-dlp in the dependencies directory
pub fn get_ytdlp_path() -> Option<PathBuf> {
    get_dependencies_dir().map(|dir| {
        #[cfg(target_os = "windows")]
        {
            dir.join("yt-dlp.exe")
        }
        #[cfg(not(target_os = "windows"))]
        {
            dir.join("yt-dlp")
        }
    })
}

/// Download progress callback
pub type DownloadProgressCallback = Box<dyn Fn(u64, u64) + Send + Sync>;

/// Download yt-dlp from GitHub releases
pub async fn download_ytdlp(
    progress_callback: Option<DownloadProgressCallback>,
) -> Result<PathBuf, DependencyError> {
    let dest_path = get_ytdlp_path().ok_or_else(|| DependencyError::SaveFailed {
        binary: "yt-dlp".to_string(),
        path: PathBuf::from("unknown"),
        source: std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "could not determine dependencies directory",
        ),
    })?;

    // Create parent directory if it doesn't exist
    if let Some(parent) = dest_path.parent() {
        tokio::fs::create_dir_all(parent).await.map_err(|e| {
            DependencyError::SaveFailed {
                binary: "yt-dlp".to_string(),
                path: parent.to_path_buf(),
                source: e,
            }
        })?;
    }

    // Determine download URL based on platform
    let download_url = get_ytdlp_download_url();

    tracing::info!("Downloading yt-dlp from {}", download_url);

    // Download the file
    let client = reqwest::Client::new();
    let response = client
        .get(download_url)
        .send()
        .await
        .map_err(|e| DependencyError::Download {
            binary: "yt-dlp".to_string(),
            source: e,
        })?;

    let total_size = response.content_length().unwrap_or(0);
    let mut downloaded: u64 = 0;

    // Create temp file
    let temp_path = dest_path.with_extension("tmp");
    let mut file = File::create(&temp_path)
        .await
        .map_err(|e| DependencyError::SaveFailed {
            binary: "yt-dlp".to_string(),
            path: temp_path.clone(),
            source: e,
        })?;

    // Stream the download
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| DependencyError::Download {
            binary: "yt-dlp".to_string(),
            source: e,
        })?;

        file.write_all(&chunk)
            .await
            .map_err(|e| DependencyError::SaveFailed {
                binary: "yt-dlp".to_string(),
                path: temp_path.clone(),
                source: e,
            })?;

        downloaded += chunk.len() as u64;

        if let Some(ref callback) = progress_callback {
            callback(downloaded, total_size);
        }
    }

    file.flush().await.map_err(|e| DependencyError::SaveFailed {
        binary: "yt-dlp".to_string(),
        path: temp_path.clone(),
        source: e,
    })?;

    drop(file);

    // Move temp file to final location
    tokio::fs::rename(&temp_path, &dest_path)
        .await
        .map_err(|e| DependencyError::SaveFailed {
            binary: "yt-dlp".to_string(),
            path: dest_path.clone(),
            source: e,
        })?;

    // Make executable on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        // On macOS, add Python3 shebang if it doesn't exist (for Python script version)
        #[cfg(target_os = "macos")]
        {
            let content = tokio::fs::read(&dest_path)
                .await
                .map_err(|e| DependencyError::SaveFailed {
                    binary: "yt-dlp".to_string(),
                    path: dest_path.clone(),
                    source: e,
                })?;

            // Check if shebang exists
            if content.len() < 2 || &content[0..2] != b"#!" {
                // Prepend Python3 shebang
                let shebang = b"#!/usr/bin/env python3\n";
                let mut new_content = Vec::with_capacity(shebang.len() + content.len());
                new_content.extend_from_slice(shebang);
                new_content.extend_from_slice(&content);

                tokio::fs::write(&dest_path, new_content)
                    .await
                    .map_err(|e| DependencyError::SaveFailed {
                        binary: "yt-dlp".to_string(),
                        path: dest_path.clone(),
                        source: e,
                    })?;
            }
        }

        let mut perms = tokio::fs::metadata(&dest_path)
            .await
            .map_err(|e| DependencyError::ChmodFailed {
                binary: "yt-dlp".to_string(),
                path: dest_path.clone(),
                source: e,
            })?
            .permissions();
        perms.set_mode(0o755);
        tokio::fs::set_permissions(&dest_path, perms)
            .await
            .map_err(|e| DependencyError::ChmodFailed {
                binary: "yt-dlp".to_string(),
                path: dest_path.clone(),
                source: e,
            })?;
    }

    tracing::info!("yt-dlp downloaded successfully to {:?}", dest_path);

    Ok(dest_path)
}

/// Get the download URL for yt-dlp based on platform
/// On macOS, we download the Python script version to avoid code signing issues
fn get_ytdlp_download_url() -> &'static str {
    #[cfg(target_os = "macos")]
    {
        // Use Python script version on macOS to avoid code signing issues with PyInstaller binaries
        "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp"
    }
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp_linux"
    }
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    {
        "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp_linux_aarch64"
    }
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    {
        "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe"
    }
    #[cfg(not(any(
        target_os = "macos",
        all(target_os = "linux", any(target_arch = "x86_64", target_arch = "aarch64")),
        all(target_os = "windows", target_arch = "x86_64")
    )))]
    {
        compile_error!("Unsupported platform for yt-dlp download")
    }
}
