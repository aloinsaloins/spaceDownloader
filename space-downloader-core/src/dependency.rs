use std::path::{Path, PathBuf};
use std::time::Duration;

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

/// Resolve binary path with the following priority:
/// 1. If candidate is an absolute/relative path, check if it exists
/// 2. Check in PATH (for Homebrew-installed binaries)
pub fn resolve_binary(candidate: &Path) -> Option<PathBuf> {
    // If candidate is a multi-component path, treat it as absolute/relative path
    if candidate.components().count() > 1 {
        if candidate.exists() {
            return Some(candidate.to_path_buf());
        } else {
            return None;
        }
    }

    // Check in PATH (for Homebrew-installed binaries)
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
