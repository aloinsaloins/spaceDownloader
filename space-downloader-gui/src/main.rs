#![windows_subsystem = "windows"]

mod localization;

use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use iced::alignment::{Horizontal, Vertical};
use iced::executor;
use iced::time;
use iced::widget::{button, Column, Container, ProgressBar, Row, Scrollable, Text, TextInput};
use iced::{Element, Length, Subscription, Task, Theme};
use localization::Localizer;
use parking_lot::Mutex;
use space_downloader_core::config::{Config, ThemePreference};
use space_downloader_core::download::{
    DownloadEvent, DownloadRequest, DownloadSummary, DownloaderService, JobHandle, JobStatus,
    ProgressSnapshot,
};
use space_downloader_core::error::SpaceDownloaderError;
use space_downloader_core::history::HistoryRepository;
use space_downloader_core::logging::{LogManager, LogManagerBuilder};
use tokio::sync::{mpsc, watch};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

fn main() -> iced::Result {
    iced::application(
        SpaceDownloaderApp::title,
        SpaceDownloaderApp::update,
        SpaceDownloaderApp::view,
    )
    .subscription(SpaceDownloaderApp::subscription)
    .theme(SpaceDownloaderApp::theme)
    .executor::<executor::Default>()
    .run_with(SpaceDownloaderApp::initialize)
}

enum SpaceDownloaderApp {
    Ready(Box<AppState>),
    Failed(String),
    DownloadingYtDlp {
        downloaded: u64,
        total: u64,
        localizer: Localizer,
    },
}

struct AppState {
    downloader: Arc<DownloaderService>,
    config: Config,
    localizer: Localizer,
    _log_manager: Option<LogManager>,
    url_input: String,
    url_error: Option<String>,
    jobs: HashMap<Uuid, JobTracker>,
    job_order: Vec<Uuid>,
}

#[derive(Debug, Clone)]
enum Message {
    UrlChanged(String),
    StartDownload,
    DownloadQueued(SharedJobResult),
    CancelDownload(Uuid),
    OpenFolder(PathBuf),
    Tick,
    YtDlpDownloadProgress(u64, u64),
    YtDlpDownloadComplete(Result<PathBuf, String>),
    InitializationComplete(Result<Arc<AppInit>, String>),
}

type SharedJobResult = Result<SharedJobHandle, Arc<SpaceDownloaderError>>;

#[derive(Clone)]
struct SharedJobHandle {
    id: Uuid,
    url: String,
    inner: Arc<Mutex<Option<JobHandle>>>,
}

impl SharedJobHandle {
    fn new(handle: JobHandle) -> Self {
        let id = handle.id;
        let url = handle.url.clone();
        Self {
            id,
            url,
            inner: Arc::new(Mutex::new(Some(handle))),
        }
    }

    fn id(&self) -> Uuid {
        self.id
    }

    fn take(&self) -> Option<JobHandle> {
        self.inner.lock().take()
    }
}

impl fmt::Debug for SharedJobHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SharedJobHandle")
            .field("id", &self.id)
            .field("url", &self.url)
            .finish()
    }
}

struct JobTracker {
    id: Uuid,
    url: String,
    status_rx: watch::Receiver<JobStatus>,
    progress_rx: watch::Receiver<Option<ProgressSnapshot>>,
    events_rx: Option<mpsc::Receiver<DownloadEvent>>,
    cancel_token: CancellationToken,
    last_status: JobStatus,
    last_progress: Option<ProgressSnapshot>,
    logs: Vec<String>,
    summary: Option<DownloadSummary>,
    folder_opened: bool,
}

impl JobTracker {
    fn new(handle: JobHandle) -> Self {
        let status_rx = handle.status_receiver();
        let progress_rx = handle.progress_receiver();
        let events_rx = handle.take_events();
        let cancel_token = handle.cancellation_token();
        let last_status = *status_rx.borrow();

        Self {
            id: handle.id,
            url: handle.url,
            status_rx,
            progress_rx,
            events_rx,
            cancel_token,
            last_status,
            last_progress: None,
            logs: Vec::new(),
            summary: None,
            folder_opened: false,
        }
    }

    fn poll(&mut self) -> Option<PathBuf> {
        let mut folder_to_open = None;

        if let Some(events_rx) = self.events_rx.as_mut() {
            while let Ok(event) = events_rx.try_recv() {
                match event {
                    DownloadEvent::Status(status) => {
                        self.last_status = status;
                    }
                    DownloadEvent::Progress(progress) => {
                        self.last_progress = Some(progress);
                    }
                    DownloadEvent::LogLine(line) => {
                        self.logs.push(line);
                        if self.logs.len() > 100 {
                            self.logs.remove(0);
                        }
                    }
                    DownloadEvent::Completed(summary) => {
                        self.summary = Some(summary.clone());
                        self.last_status = summary.status;

                        // Auto-open folder on completion
                        if !self.folder_opened && summary.status == JobStatus::Succeeded {
                            if let Some(file_path) = &summary.file_path {
                                if let Some(parent) = file_path.parent() {
                                    folder_to_open = Some(parent.to_path_buf());
                                    self.folder_opened = true;
                                }
                            }
                        }
                    }
                    DownloadEvent::Failed(message) => {
                        self.last_status = JobStatus::Failed;
                        self.logs.push(message);
                        if self.logs.len() > 100 {
                            self.logs.remove(0);
                        }
                    }
                }
            }
        }

        let status = *self.status_rx.borrow();
        if status != self.last_status {
            self.last_status = status;
        }

        if let Some(progress) = self.progress_rx.borrow().clone() {
            self.last_progress = Some(progress);
        }

        folder_to_open
    }

    fn is_finished(&self) -> bool {
        matches!(
            self.last_status,
            JobStatus::Succeeded | JobStatus::Failed | JobStatus::Canceled
        )
    }

    fn cancel(&self) {
        self.cancel_token.cancel();
    }

    fn view(&self, localizer: &Localizer) -> Element<'_, Message> {
        let mut column = Column::new()
            .spacing(6)
            .push(Text::new(self.url.clone()).size(14))
            .push(Text::new(format_status(self.last_status, localizer)).size(12));

        if let Some(progress) = &self.last_progress {
            if let Some(percent) = progress.percent {
                column = column.push(ProgressBar::new(
                    0.0..=1.0,
                    (percent / 100.0).clamp(0.0, 1.0),
                ));
            }

            if let Some(progress_text) = format_progress(progress) {
                column = column.push(Text::new(progress_text).size(12));
            }
        }

        if let Some(summary) = &self.summary {
            if let Some(path) = &summary.file_path {
                column = column.push(Text::new(path.to_string_lossy().to_string()).size(12));
            }
        }

        if let Some(last) = self.logs.last() {
            column = column.push(Text::new(last.clone()).size(12));
        }

        // Button row for actions
        let mut button_row = Row::new().spacing(8);

        if !self.is_finished() {
            button_row = button_row.push(
                button(Text::new(localizer.text("button-cancel")))
                    .on_press(Message::CancelDownload(self.id)),
            );
        } else if let Some(summary) = &self.summary {
            // Show "Open Folder" button if download completed successfully
            if matches!(self.last_status, JobStatus::Succeeded) {
                if let Some(file_path) = &summary.file_path {
                    if let Some(parent) = file_path.parent() {
                        button_row = button_row.push(
                            button(Text::new(localizer.text("job-open-folder")))
                                .on_press(Message::OpenFolder(parent.to_path_buf())),
                        );
                    }
                }
            }
        }

        column = column.push(button_row);

        Container::new(column)
            .padding(12)
            .width(Length::Fill)
            .into()
    }
}

struct AppInit {
    downloader: Arc<DownloaderService>,
    config: Config,
    log_manager: Option<LogManager>,
}

impl Clone for AppInit {
    fn clone(&self) -> Self {
        Self {
            downloader: self.downloader.clone(),
            config: self.config.clone(),
            log_manager: None, // LogManager is not cloneable, so we set it to None
        }
    }
}

impl std::fmt::Debug for AppInit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppInit")
            .field("config", &self.config)
            .finish()
    }
}

impl SpaceDownloaderApp {
    fn initialize() -> (Self, Task<Message>) {
        let (config, _) = match Config::load_or_default(None) {
            Ok(cfg) => cfg,
            Err(err) => {
                return (
                    SpaceDownloaderApp::Failed(format!("Failed to load config: {}", err)),
                    Task::none(),
                )
            }
        };

        let localizer = Localizer::new(&config.general.language);

        (
            SpaceDownloaderApp::DownloadingYtDlp {
                downloaded: 0,
                total: 0,
                localizer,
            },
            Task::perform(async_initialize(config), |result| {
                Message::InitializationComplete(result.map(Arc::new))
            }),
        )
    }

    fn title(&self) -> String {
        match self {
            SpaceDownloaderApp::Failed(_) => "Space Downloader".into(),
            SpaceDownloaderApp::Ready(state) => state.localizer.text("app-title"),
            SpaceDownloaderApp::DownloadingYtDlp { localizer, .. } => {
                localizer.text("app-title")
            }
        }
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match self {
            SpaceDownloaderApp::Failed(_) => Task::none(),
            SpaceDownloaderApp::Ready(state) => state.update(message),
            SpaceDownloaderApp::DownloadingYtDlp {
                downloaded,
                total,
                localizer: _,
            } => match message {
                Message::YtDlpDownloadProgress(d, t) => {
                    *downloaded = d;
                    *total = t;
                    Task::none()
                }
                Message::YtDlpDownloadComplete(Ok(_)) => {
                    // Re-initialize after download complete
                    Task::none()
                }
                Message::YtDlpDownloadComplete(Err(error)) => {
                    *self = SpaceDownloaderApp::Failed(error);
                    Task::none()
                }
                Message::InitializationComplete(result) => match result {
                    Ok(init) => {
                        let init = Arc::try_unwrap(init).unwrap_or_else(|arc| (*arc).clone());
                        *self = SpaceDownloaderApp::Ready(Box::new(AppState::from(init)));
                        Task::none()
                    }
                    Err(error) => {
                        *self = SpaceDownloaderApp::Failed(error);
                        Task::none()
                    }
                }
                _ => Task::none(),
            },
        }
    }

    fn view(&self) -> Element<'_, Message> {
        match self {
            SpaceDownloaderApp::Failed(error) => Container::new(Text::new(error.clone()))
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Horizontal::Center)
                .align_y(Vertical::Center)
                .into(),
            SpaceDownloaderApp::Ready(state) => state.view(),
            SpaceDownloaderApp::DownloadingYtDlp {
                downloaded,
                total,
                localizer: _,
            } => {
                let progress = if *total > 0 {
                    *downloaded as f32 / *total as f32
                } else {
                    0.0
                };

                let size_text = if *total > 0 {
                    format!(
                        "{:.1} MB / {:.1} MB",
                        *downloaded as f64 / 1024.0 / 1024.0,
                        *total as f64 / 1024.0 / 1024.0
                    )
                } else {
                    "Initializing...".to_string()
                };

                Container::new(
                    Column::new()
                        .spacing(16)
                        .align_x(Horizontal::Center)
                        .push(Text::new("Downloading yt-dlp...").size(24))
                        .push(ProgressBar::new(0.0..=1.0, progress))
                        .push(Text::new(size_text)),
                )
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Horizontal::Center)
                .align_y(Vertical::Center)
                .padding(48)
                .into()
            }
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        match self {
            SpaceDownloaderApp::Ready(_) => {
                time::every(Duration::from_millis(500)).map(|_| Message::Tick)
            }
            SpaceDownloaderApp::Failed(_) => Subscription::none(),
            SpaceDownloaderApp::DownloadingYtDlp { .. } => Subscription::none(),
        }
    }

    fn theme(&self) -> Theme {
        match self {
            SpaceDownloaderApp::Ready(state) => match state.config.general.theme {
                ThemePreference::Light => Theme::Light,
                ThemePreference::Dark => Theme::Dark,
                ThemePreference::System => Theme::default(),
            },
            SpaceDownloaderApp::Failed(_) => Theme::default(),
            SpaceDownloaderApp::DownloadingYtDlp { .. } => Theme::default(),
        }
    }
}

impl AppState {
    fn from(init: AppInit) -> Self {
        let localizer = Localizer::new(&init.config.general.language);
        Self {
            downloader: init.downloader,
            config: init.config,
            localizer,
            _log_manager: init.log_manager,
            url_input: String::new(),
            url_error: None,
            jobs: HashMap::new(),
            job_order: Vec::new(),
        }
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::UrlChanged(value) => {
                self.url_input = value;
                self.url_error = None;
                Task::none()
            }
            Message::StartDownload => self.start_download(),
            Message::DownloadQueued(result) => {
                match result {
                    Ok(shared) => {
                        if let Some(handle) = shared.take() {
                            let id = shared.id();
                            let tracker = JobTracker::new(handle);
                            self.job_order.push(id);
                            self.jobs.insert(id, tracker);
                            self.url_input.clear();
                            self.url_error = None;
                        }
                    }
                    Err(error) => {
                        self.url_error = Some(error.to_string());
                    }
                }
                Task::none()
            }
            Message::CancelDownload(id) => {
                if let Some(job) = self.jobs.get(&id) {
                    job.cancel();
                }
                Task::none()
            }
            Message::OpenFolder(path) => {
                if let Err(e) = open_folder_in_explorer(&path) {
                    tracing::error!("Failed to open folder: {}", e);
                }
                Task::none()
            }
            Message::Tick => {
                for id in &self.job_order {
                    if let Some(job) = self.jobs.get_mut(id) {
                        if let Some(folder_path) = job.poll() {
                            // Auto-open folder on completion
                            if let Err(e) = open_folder_in_explorer(&folder_path) {
                                tracing::error!("Failed to auto-open folder: {}", e);
                            }
                        }
                    }
                }
                Task::none()
            }
            Message::YtDlpDownloadProgress(_, _)
            | Message::YtDlpDownloadComplete(_)
            | Message::InitializationComplete(_) => {
                // These messages are handled in the top-level update
                Task::none()
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let input_row = Row::new()
            .spacing(8)
            .push(
                TextInput::new(&self.localizer.text("input-url-label"), &self.url_input)
                    .padding(8)
                    .width(Length::Fill)
                    .on_input(Message::UrlChanged),
            )
            .push(
                button(Text::new(self.localizer.text("button-download")))
                    .on_press(Message::StartDownload),
            );

        let mut column = Column::new().spacing(16).push(input_row);

        if let Some(error) = &self.url_error {
            column = column.push(Text::new(error.clone()));
        }

        column = column.push(Text::new(self.localizer.text("download-active")).size(16));

        let mut jobs_list = Column::new().spacing(8);
        for id in &self.job_order {
            if let Some(job) = self.jobs.get(id) {
                jobs_list = jobs_list.push(job.view(&self.localizer));
            }
        }

        if self.job_order.is_empty() {
            column = column.push(Text::new(self.localizer.text("history-empty")));
        } else {
            column = column.push(Scrollable::new(jobs_list).height(Length::Fill));
        }

        Container::new(column.padding(16))
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn start_download(&mut self) -> Task<Message> {
        let url = self.url_input.trim();
        if url.is_empty() {
            self.url_error = Some(self.localizer.text("error-invalid-url"));
            return Task::none();
        }

        let request = build_download_request(&self.config, url.to_string());
        let downloader = self.downloader.clone();
        Task::perform(queue_download(downloader, request), Message::DownloadQueued)
    }
}

async fn async_initialize(config: Config) -> Result<AppInit, String> {
    use space_downloader_core::dependency::{check_dependencies, download_ytdlp};

    // Check if yt-dlp is available
    let deps = check_dependencies(&config.advanced)
        .await
        .map_err(|err| format!("Failed to check dependencies: {}", err))?;

    // Download yt-dlp if not available
    if !deps.yt_dlp.available {
        tracing::info!("yt-dlp not found, downloading...");
        download_ytdlp(None)
            .await
            .map_err(|err| format!("Failed to download yt-dlp: {}", err))?;
        tracing::info!("yt-dlp download completed");
    }

    // Continue with normal initialization
    let history = HistoryRepository::open(None).map_err(|err| err.to_string())?;
    let downloader = Arc::new(DownloaderService::new(config.clone(), history));
    let log_manager = initialize_logger(&config.logging).map_err(|err| err.to_string())?;

    Ok(AppInit {
        downloader,
        config,
        log_manager,
    })
}

fn initialize_logger(
    settings: &space_downloader_core::config::LogSettings,
) -> std::io::Result<Option<LogManager>> {
    if !settings.enabled {
        return Ok(None);
    }

    let manager = LogManagerBuilder::default()
        .with_settings(settings)
        .enable_stdout(false)
        .build()?;

    Ok(Some(manager))
}

fn build_download_request(config: &Config, url: String) -> DownloadRequest {
    let mut request = DownloadRequest::new(
        url,
        config.general.output_dir.clone(),
        config.download.format,
    );

    request.extra_args = config.advanced.extra_args.clone();
    request.cookie_file = config.advanced.cookie_file.clone();
    request
}

async fn queue_download(
    downloader: Arc<DownloaderService>,
    request: DownloadRequest,
) -> SharedJobResult {
    downloader
        .queue(request)
        .await
        .map(SharedJobHandle::new)
        .map_err(|err| Arc::new(SpaceDownloaderError::from(err)))
}

fn format_status(status: JobStatus, localizer: &Localizer) -> String {
    let key = match status {
        JobStatus::Queued => "status-queued",
        JobStatus::Running => "status-running",
        JobStatus::Succeeded => "status-succeeded",
        JobStatus::Failed => "status-failed",
        JobStatus::Canceled => "status-canceled",
    };
    localizer.text(key)
}

fn format_progress(progress: &ProgressSnapshot) -> Option<String> {
    let mut parts = Vec::new();

    if let Some(percent) = progress.percent {
        parts.push(format!("{percent:.1}%"));
    }

    if let Some(downloaded) = progress.downloaded_bytes {
        if let Some(total) = progress.total_bytes {
            parts.push(format!(
                "{}/{}",
                format_bytes(downloaded),
                format_bytes(total)
            ));
        } else {
            parts.push(format!("{} downloaded", format_bytes(downloaded)));
        }
    }

    if let Some(speed) = progress.speed_bytes_per_sec {
        parts.push(format!("{} /s", format_bytes(speed)));
    }

    if let Some(eta) = progress.eta {
        parts.push(format!("ETA {}", format_eta(eta)));
    }

    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" • "))
    }
}

fn format_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;

    let value = bytes as f64;

    if value >= GB {
        format!("{:.2} GB", value / GB)
    } else if value >= MB {
        format!("{:.2} MB", value / MB)
    } else if value >= KB {
        format!("{:.2} KB", value / KB)
    } else {
        format!("{bytes} B")
    }
}

fn format_eta(duration: Duration) -> String {
    let secs = duration.as_secs();
    let minutes = secs / 60;
    let seconds = secs % 60;
    if minutes > 0 {
        format!("{}m {:02}s", minutes, seconds)
    } else {
        format!("{}s", seconds)
    }
}

fn open_folder_in_explorer(path: &std::path::Path) -> std::io::Result<()> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer").arg(path).spawn()?;
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open").arg(path).spawn()?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open").arg(path).spawn()?;
    }

    Ok(())
}
