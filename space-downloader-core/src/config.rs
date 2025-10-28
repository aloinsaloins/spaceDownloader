use std::fs;
use std::path::{Path, PathBuf};

use directories::ProjectDirs;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

use crate::error::ConfigError;

static DEFAULT_PROJECT_DIRS: Lazy<Option<ProjectDirs>> = Lazy::new(|| {
    #[cfg(target_os = "macos")]
    {
        // macOS: com.space-downloader.space-downloader
        ProjectDirs::from("com", "space-downloader", "space-downloader")
    }
    #[cfg(target_os = "windows")]
    {
        // Windows: space-downloader\space-downloader
        ProjectDirs::from("", "space-downloader", "space-downloader")
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        // Linux: space-downloader
        ProjectDirs::from("", "", "space-downloader")
    }
});

pub const CONFIG_RELATIVE_PATH: &str = "space_downloader.toml";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub general: GeneralSettings,
    #[serde(default)]
    pub download: DownloadSettings,
    #[serde(default)]
    pub advanced: AdvancedSettings,
    #[serde(default)]
    pub logging: LogSettings,
}

impl Config {
    pub fn load_or_default(path: Option<&Path>) -> Result<(Self, PathBuf), ConfigError> {
        let resolved_path = path
            .map(|p| p.to_path_buf())
            .unwrap_or_else(default_config_path);

        if resolved_path.exists() {
            let content = fs::read_to_string(&resolved_path).map_err(|source| ConfigError::Io {
                path: resolved_path.clone(),
                source,
            })?;
            let config =
                toml::from_str::<Config>(&content).map_err(|source| ConfigError::Parse {
                    path: resolved_path.clone(),
                    source,
                })?;
            Ok((config, resolved_path))
        } else {
            if let Some(parent) = resolved_path.parent() {
                fs::create_dir_all(parent).map_err(|source| ConfigError::Io {
                    path: parent.to_path_buf(),
                    source,
                })?;
            }
            let config = Config::default();
            config.save(&resolved_path)?;
            Ok((config, resolved_path))
        }
    }

    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), ConfigError> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|source| ConfigError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }

        let serialized =
            toml::to_string_pretty(self).map_err(|source| ConfigError::Serialize { source })?;
        fs::write(path, serialized).map_err(|source| ConfigError::Io {
            path: path.to_path_buf(),
            source,
        })
    }

    pub fn merge_download(&mut self, download: DownloadSettings) {
        self.download = download;
    }

    pub fn merge_general(&mut self, general: GeneralSettings) {
        self.general = general;
    }

    pub fn merge_advanced(&mut self, advanced: AdvancedSettings) {
        self.advanced = advanced;
    }

    pub fn merge_logging(&mut self, logging: LogSettings) {
        self.logging = logging;
    }
}

fn default_config_path() -> PathBuf {
    if let Some(project_dirs) = DEFAULT_PROJECT_DIRS.as_ref() {
        project_dirs.config_dir().join("space_downloader.toml")
    } else {
        PathBuf::from(CONFIG_RELATIVE_PATH)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ThemePreference {
    Light,
    Dark,
    #[default]
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralSettings {
    pub output_dir: PathBuf,
    pub language: String,
    pub theme: ThemePreference,
}

impl Default for GeneralSettings {
    fn default() -> Self {
        Self {
            output_dir: default_download_dir(),
            language: default_language(),
            theme: ThemePreference::System,
        }
    }
}

fn default_download_dir() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        // macOS: Use ~/Downloads
        if let Some(home) = dirs::home_dir() {
            return home.join("Downloads");
        }
    }

    #[cfg(target_os = "windows")]
    {
        // Windows: Use executable directory
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                return exe_dir.to_path_buf();
            }
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        // Linux: Use ~/Downloads or current directory
        if let Some(home) = dirs::home_dir() {
            let downloads = home.join("Downloads");
            if downloads.exists() {
                return downloads;
            }
        }
    }

    // Fallback to current directory
    PathBuf::from(".")
}

fn default_language() -> String {
    match std::env::var("LANG") {
        Ok(value) => {
            if value.starts_with("ja") {
                "ja-JP".to_string()
            } else {
                "en-US".to_string()
            }
        }
        Err(_) => "en-US".to_string(),
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum AudioFormat {
    #[default]
    M4a,
    Mp3,
    Opus,
}

impl std::fmt::Display for AudioFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            AudioFormat::M4a => "m4a",
            AudioFormat::Mp3 => "mp3",
            AudioFormat::Opus => "opus",
        };
        write!(f, "{}", text)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadSettings {
    pub format: AudioFormat,
    pub max_retries: u8,
    pub timeout_sec: u64,
    pub concurrency: usize,
}

impl DownloadSettings {
    pub fn effective_concurrency(&self) -> usize {
        self.concurrency.clamp(1, 3)
    }
}

impl Default for DownloadSettings {
    fn default() -> Self {
        Self {
            format: AudioFormat::M4a,
            max_retries: 3,
            timeout_sec: 0,
            concurrency: 1,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedSettings {
    pub yt_dlp_path: PathBuf,
    pub cookie_file: Option<PathBuf>,
    pub extra_args: Vec<String>,
    pub save_logs: bool,
}

impl Default for AdvancedSettings {
    fn default() -> Self {
        Self {
            yt_dlp_path: PathBuf::from("yt-dlp"),
            cookie_file: None,
            extra_args: Vec::new(),
            save_logs: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Error,
    Warn,
    #[default]
    Info,
    Debug,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogSettings {
    pub enabled: bool,
    pub level: LogLevel,
}

impl Default for LogSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            level: LogLevel::Info,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_roundtrip() {
        let config = Config::default();
        let toml = toml::to_string(&config).unwrap();
        let restored: Config = toml::from_str(&toml).unwrap();
        assert_eq!(restored.download.concurrency, 1);
        assert_eq!(restored.advanced.extra_args.len(), 0);
    }
}
#[derive(Debug, Clone)]
pub struct ParseAudioFormatError(pub String);

impl std::str::FromStr for AudioFormat {
    type Err = ParseAudioFormatError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "m4a" => Ok(AudioFormat::M4a),
            "mp3" => Ok(AudioFormat::Mp3),
            "opus" => Ok(AudioFormat::Opus),
            other => Err(ParseAudioFormatError(other.to_string())),
        }
    }
}
impl std::fmt::Display for ThemePreference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let label = match self {
            ThemePreference::Light => "Light",
            ThemePreference::Dark => "Dark",
            ThemePreference::System => "System",
        };
        write!(f, "{}", label)
    }
}
impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let label = match self {
            LogLevel::Error => "error",
            LogLevel::Warn => "warn",
            LogLevel::Info => "info",
            LogLevel::Debug => "debug",
        };
        write!(f, "{}", label)
    }
}
