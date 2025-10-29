use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("failed to read config file {path:?}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse config file {path:?}: {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },
    #[error("failed to serialize config: {source}")]
    Serialize {
        #[source]
        source: toml::ser::Error,
    },
}

#[derive(Debug, Error)]
pub enum DependencyError {
    #[error("failed to launch dependency check for {binary}: {source}")]
    Spawn {
        binary: String,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to download {binary}: {source}")]
    Download {
        binary: String,
        #[source]
        source: reqwest::Error,
    },
    #[error("failed to save {binary} to {path:?}: {source}")]
    SaveFailed {
        binary: String,
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to make {binary} executable at {path:?}: {source}")]
    ChmodFailed {
        binary: String,
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

#[derive(Debug, Error)]
pub enum HistoryError {
    #[error("failed to initialize history database at {path:?}: {source}")]
    Initialize {
        path: PathBuf,
        #[source]
        source: rusqlite::Error,
    },
    #[error("failed to access history storage at {path:?}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to execute history query: {source}")]
    Query {
        #[source]
        source: rusqlite::Error,
    },
}

#[derive(Debug, Error)]
pub enum DownloadError {
    #[error("invalid url: {0}")]
    InvalidUrl(String),
    #[error("missing dependency: {0}")]
    MissingDependency(String),
    #[error("failed to spawn yt-dlp process: {source}")]
    Spawn {
        #[source]
        source: std::io::Error,
    },
    #[error("download command failed with status {status:?}: {stderr}")]
    CommandFailed { status: Option<i32>, stderr: String },
    #[error("download canceled")]
    Canceled,
    #[error("download timed out after {0} seconds")]
    Timeout(u64),
    #[error("io error: {source}")]
    Io {
        #[source]
        source: std::io::Error,
    },
    #[error("command execution error: {source}")]
    Join {
        #[source]
        source: tokio::task::JoinError,
    },
}

#[derive(Debug, Error)]
pub enum SpaceDownloaderError {
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error(transparent)]
    Dependency(#[from] DependencyError),
    #[error(transparent)]
    Download(#[from] DownloadError),
    #[error(transparent)]
    History(#[from] HistoryError),
}

pub type Result<T> = std::result::Result<T, SpaceDownloaderError>;
