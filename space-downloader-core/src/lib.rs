pub mod config;
pub mod dependency;
pub mod download;
pub mod error;
pub mod history;
pub mod logging;

pub use config::{
    AdvancedSettings, AudioFormat, Config, DownloadSettings, GeneralSettings, LogSettings,
};
pub use dependency::{DependencyCheck, DependencyStatus};
pub use download::{
    DownloadEvent, DownloadRequest, DownloadSummary, DownloaderService, JobHandle, JobState,
    JobStatus, ProgressSnapshot,
};
pub use error::{ConfigError, DependencyError, DownloadError, HistoryError, SpaceDownloaderError};
pub use history::{DownloadHistoryEntry, HistoryRepository};
pub use logging::{LogManager, LogManagerBuilder};

pub type Result<T> = std::result::Result<T, SpaceDownloaderError>;
