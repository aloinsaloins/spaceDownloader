use std::io;
use std::path::PathBuf;

use directories::ProjectDirs;
use once_cell::sync::Lazy;
use tracing::Level;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::layer::{Layer, SubscriberExt};
use tracing_subscriber::{fmt, util::SubscriberInitExt, EnvFilter};

use crate::config::{LogLevel, LogSettings};

static DEFAULT_LOG_DIR: Lazy<PathBuf> = Lazy::new(|| {
    #[cfg(target_os = "macos")]
    {
        // macOS: ~/Library/Application Support/com.space-downloader.space-downloader/logs
        ProjectDirs::from("com", "space-downloader", "space-downloader")
            .map(|dirs| dirs.data_dir().join("logs"))
            .unwrap_or_else(|| PathBuf::from("logs"))
    }
    #[cfg(target_os = "windows")]
    {
        // Windows: %APPDATA%\space-downloader\space-downloader\logs
        ProjectDirs::from("", "space-downloader", "space-downloader")
            .map(|dirs| dirs.data_dir().join("logs"))
            .unwrap_or_else(|| PathBuf::from("logs"))
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        // Linux: ~/.local/share/space-downloader/logs
        ProjectDirs::from("", "", "space-downloader")
            .map(|dirs| dirs.data_dir().join("logs"))
            .unwrap_or_else(|| PathBuf::from("logs"))
    }
});

pub struct LogManager {
    _guard: Option<tracing_appender::non_blocking::WorkerGuard>,
    level: Level,
    log_dir: PathBuf,
}

impl LogManager {
    pub fn builder() -> LogManagerBuilder {
        LogManagerBuilder::default()
    }

    pub fn level(&self) -> Level {
        self.level
    }

    pub fn log_dir(&self) -> &PathBuf {
        &self.log_dir
    }
}

pub struct LogManagerBuilder {
    level: Level,
    enable_file: bool,
    log_dir: PathBuf,
    enable_stdout: bool,
}

impl Default for LogManagerBuilder {
    fn default() -> Self {
        Self {
            level: Level::INFO,
            enable_file: true,
            log_dir: DEFAULT_LOG_DIR.clone(),
            enable_stdout: true,
        }
    }
}

impl LogManagerBuilder {
    pub fn with_settings(mut self, settings: &LogSettings) -> Self {
        self.level = level_from_config(&settings.level);
        self.enable_file = settings.enabled;
        self
    }

    pub fn level(mut self, level: Level) -> Self {
        self.level = level;
        self
    }

    pub fn log_dir<P: Into<PathBuf>>(mut self, path: P) -> Self {
        self.log_dir = path.into();
        self
    }

    pub fn enable_file(mut self, enable: bool) -> Self {
        self.enable_file = enable;
        self
    }

    pub fn enable_stdout(mut self, enable: bool) -> Self {
        self.enable_stdout = enable;
        self
    }

    pub fn build(self) -> std::io::Result<LogManager> {
        if self.enable_file {
            std::fs::create_dir_all(&self.log_dir)?;
        }

        let level_filter = EnvFilter::from_default_env().add_directive(self.level.into());

        let (file_writer, file_guard) = if self.enable_file {
            let file_appender =
                tracing_appender::rolling::daily(&self.log_dir, "space_downloader.log");
            tracing_appender::non_blocking(file_appender)
        } else {
            tracing_appender::non_blocking(io::sink())
        };

        let file_layer = fmt::layer()
            .with_writer(file_writer)
            .with_ansi(false)
            .with_filter(if self.enable_file {
                LevelFilter::TRACE
            } else {
                LevelFilter::OFF
            });

        let stdout_layer = fmt::layer()
            .with_target(true)
            .with_filter(if self.enable_stdout {
                LevelFilter::TRACE
            } else {
                LevelFilter::OFF
            });

        tracing_subscriber::registry()
            .with(level_filter)
            .with(file_layer)
            .with(stdout_layer)
            .init();

        Ok(LogManager {
            _guard: if self.enable_file {
                Some(file_guard)
            } else {
                None
            },
            level: self.level,
            log_dir: self.log_dir,
        })
    }
}

fn level_from_config(level: &LogLevel) -> Level {
    match level {
        LogLevel::Error => Level::ERROR,
        LogLevel::Warn => Level::WARN,
        LogLevel::Info => Level::INFO,
        LogLevel::Debug => Level::DEBUG,
    }
}
