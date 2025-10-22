use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use chrono::{DateTime, Utc};
use once_cell::sync::Lazy;
use parking_lot::Mutex as ParkingMutex;
use regex::Regex;
use tokio::fs;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{mpsc, watch, RwLock, Semaphore};
use tokio::time;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::config::{AdvancedSettings, AudioFormat, Config, DownloadSettings};
use crate::error::{DownloadError, HistoryError};
use crate::history::HistoryRepository;

static PROGRESS_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"\[download\]\s+(?:(?P<percent>\d{1,3}(?:\.\d+)?)%)?.*?(?P<downloaded>\d+(?:\.\d+)?)(?P<downloaded_unit>KiB|MiB|GiB|TiB|B|Bytes)?(?:\s*of\s*(?P<total>\d+(?:\.\d+)?)(?P<total_unit>KiB|MiB|GiB|TiB|B|Bytes))?.*?(?:(?P<speed>\d+(?:\.\d+)?)(?P<speed_unit>KiB/s|MiB/s|GiB/s|TiB/s))?.*?(?:ETA\s+(?P<eta>[0-9:]+))?",
    )
    .expect("valid regex" )
});

static DESTINATION_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"Destination:\s+(?P<path>.+)").expect("valid regex"));

#[derive(Debug, Clone)]
pub struct DownloadRequest {
    pub url: String,
    pub output_dir: PathBuf,
    pub format: AudioFormat,
    pub extra_args: Vec<String>,
    pub cookie_file: Option<PathBuf>,
}

impl DownloadRequest {
    pub fn new(url: String, output_dir: PathBuf, format: AudioFormat) -> Self {
        Self {
            url,
            output_dir,
            format,
            extra_args: Vec::new(),
            cookie_file: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum JobStatus {
    Queued,
    Running,
    Succeeded,
    Failed,
    Canceled,
}

impl JobStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            JobStatus::Queued => "Queued",
            JobStatus::Running => "Running",
            JobStatus::Succeeded => "Succeeded",
            JobStatus::Failed => "Failed",
            JobStatus::Canceled => "Canceled",
        }
    }

    pub fn from_str(value: &str) -> JobStatus {
        match value {
            "Queued" => JobStatus::Queued,
            "Running" => JobStatus::Running,
            "Succeeded" => JobStatus::Succeeded,
            "Canceled" => JobStatus::Canceled,
            _ => JobStatus::Failed,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProgressSnapshot {
    pub percent: Option<f32>,
    pub downloaded_bytes: Option<u64>,
    pub total_bytes: Option<u64>,
    pub speed_bytes_per_sec: Option<u64>,
    pub eta: Option<Duration>,
}

impl Default for ProgressSnapshot {
    fn default() -> Self {
        Self {
            percent: None,
            downloaded_bytes: None,
            total_bytes: None,
            speed_bytes_per_sec: None,
            eta: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DownloadSummary {
    pub id: Uuid,
    pub url: String,
    pub status: JobStatus,
    pub title: Option<String>,
    pub uploader: Option<String>,
    pub file_path: Option<PathBuf>,
    pub completed_at: DateTime<Utc>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone)]
pub enum DownloadEvent {
    Status(JobStatus),
    Progress(ProgressSnapshot),
    LogLine(String),
    Completed(DownloadSummary),
    Failed(String),
}

#[derive(Debug, Clone)]
pub struct JobState {
    pub id: Uuid,
    pub url: String,
    pub status: JobStatus,
    pub progress: Option<ProgressSnapshot>,
}

pub struct JobHandle {
    pub id: Uuid,
    pub url: String,
    status_rx: watch::Receiver<JobStatus>,
    progress_rx: watch::Receiver<Option<ProgressSnapshot>>,
    events_rx: ParkingMutex<Option<mpsc::Receiver<DownloadEvent>>>,
    cancel_token: CancellationToken,
}

impl JobHandle {
    pub fn status_receiver(&self) -> watch::Receiver<JobStatus> {
        self.status_rx.clone()
    }

    pub fn progress_receiver(&self) -> watch::Receiver<Option<ProgressSnapshot>> {
        self.progress_rx.clone()
    }

    pub fn take_events(&self) -> Option<mpsc::Receiver<DownloadEvent>> {
        self.events_rx.lock().take()
    }

    pub fn cancel(&self) {
        self.cancel_token.cancel();
    }

    pub fn cancellation_token(&self) -> CancellationToken {
        self.cancel_token.clone()
    }
}

pub struct DownloaderService {
    inner: Arc<DownloaderInner>,
}

struct DownloaderInner {
    config: RwLock<Config>,
    history: HistoryRepository,
    semaphore: RwLock<Arc<Semaphore>>,
}

struct JobRuntime {
    id: Uuid,
    request: DownloadRequest,
    status_tx: watch::Sender<JobStatus>,
    progress_tx: watch::Sender<Option<ProgressSnapshot>>,
    events_tx: mpsc::Sender<DownloadEvent>,
    cancel_token: CancellationToken,
    download_settings: DownloadSettings,
    advanced_settings: AdvancedSettings,
    history: HistoryRepository,
    history_row_id: ParkingMutex<Option<i64>>,
}

impl DownloaderService {
    pub fn new(config: Config, history: HistoryRepository) -> Self {
        let concurrency = config.download.effective_concurrency();
        let semaphore = Arc::new(Semaphore::new(concurrency));
        Self {
            inner: Arc::new(DownloaderInner {
                config: RwLock::new(config),
                history,
                semaphore: RwLock::new(semaphore),
            }),
        }
    }

    pub async fn update_config(&self, config: Config) {
        let concurrency = config.download.effective_concurrency();
        {
            let mut current = self.inner.config.write().await;
            *current = config;
        }
        let mut semaphore = self.inner.semaphore.write().await;
        *semaphore = Arc::new(Semaphore::new(concurrency));
    }

    pub async fn queue(&self, mut request: DownloadRequest) -> Result<JobHandle, DownloadError> {
        url::Url::parse(&request.url)
            .map_err(|_| DownloadError::InvalidUrl(request.url.clone()))?;

        let config = self.inner.config.read().await.clone();
        let download_settings = config.download.clone();
        let advanced_settings = config.advanced.clone();

        if request.output_dir.as_os_str().is_empty() {
            request.output_dir = config.general.output_dir.clone();
        }

        if request.extra_args.is_empty() {
            request.extra_args = advanced_settings.extra_args.clone();
        }

        if request.cookie_file.is_none() {
            request.cookie_file = advanced_settings.cookie_file.clone();
        }

        fs::create_dir_all(&request.output_dir)
            .await
            .map_err(|source| DownloadError::Io { source })?;

        let job_id = Uuid::new_v4();
        let (status_tx, status_rx) = watch::channel(JobStatus::Queued);
        let (progress_tx, progress_rx) = watch::channel::<Option<ProgressSnapshot>>(None);
        let (events_tx, events_rx) = mpsc::channel(128);
        let cancel_token = CancellationToken::new();

        let history = self.inner.history.clone();
        let handle_url = request.url.clone();
        let history_url = handle_url.clone();
        let history_format = request.format;
        let history_row = tokio::task::spawn_blocking(move || {
            history.record_queued(job_id, &history_url, history_format)
        })
        .await
        .map_err(|source| DownloadError::Join { source })?
        .map_err(download_error_from_history)?;

        let job = Arc::new(JobRuntime {
            id: job_id,
            request,
            status_tx,
            progress_tx,
            events_tx,
            cancel_token: cancel_token.clone(),
            download_settings,
            advanced_settings,
            history: self.inner.history.clone(),
            history_row_id: ParkingMutex::new(Some(history_row)),
        });

        let semaphore = { self.inner.semaphore.read().await.clone() };
        let job_for_task = job.clone();

        tokio::spawn(async move {
            let permit = tokio::select! {
                permit = semaphore.acquire_owned() => {
                    match permit {
                        Ok(permit) => permit,
                        Err(error) => {
                            error!("download job {} failed to start: {error}", job_for_task.id);
                            job_for_task.status_tx.send_replace(JobStatus::Failed);
                            let message = "failed to acquire download slot".to_string();
                            job_for_task
                                .events_tx
                                .send(DownloadEvent::Failed(message.clone()))
                                .await
                                .ok();
                            finalize_history(&job_for_task, JobStatus::Failed, None, Some(message)).await;
                            return;
                        }
                    }
                }
                _ = job_for_task.cancel_token.cancelled() => {
                    job_for_task.status_tx.send_replace(JobStatus::Canceled);
                    job_for_task
                        .events_tx
                        .send(DownloadEvent::Status(JobStatus::Canceled))
                        .await
                        .ok();
                    finalize_history(
                        &job_for_task,
                        JobStatus::Canceled,
                        None,
                        Some("download canceled".to_string()),
                    )
                    .await;
                    return;
                }
            };

            if job_for_task.cancel_token.is_cancelled() {
                job_for_task.status_tx.send_replace(JobStatus::Canceled);
                job_for_task
                    .events_tx
                    .send(DownloadEvent::Status(JobStatus::Canceled))
                    .await
                    .ok();
                finalize_history(
                    &job_for_task,
                    JobStatus::Canceled,
                    None,
                    Some("download canceled".to_string()),
                )
                .await;
                drop(permit);
                return;
            }

            if let Err(error) = run_job(job_for_task.clone()).await {
                error!("download job {} failed: {error}", job_for_task.id);
            }

            drop(permit);
        });

        Ok(JobHandle {
            id: job_id,
            url: handle_url,
            status_rx,
            progress_rx,
            events_rx: ParkingMutex::new(Some(events_rx)),
            cancel_token,
        })
    }
}

async fn run_job(job: Arc<JobRuntime>) -> Result<(), DownloadError> {
    info!("starting download job {}", job.id);
    job.status_tx.send_replace(JobStatus::Running);
    job.events_tx
        .send(DownloadEvent::Status(JobStatus::Running))
        .await
        .ok();

    match execute_download(job.clone()).await {
        Ok(summary) => {
            job.status_tx.send_replace(JobStatus::Succeeded);
            job.events_tx
                .send(DownloadEvent::Completed(summary.clone()))
                .await
                .ok();
            finalize_history(
                &job,
                JobStatus::Succeeded,
                summary.file_path.as_deref(),
                None,
            )
            .await;

            if summary.title.is_some() || summary.uploader.is_some() {
                let history = job.history.clone();
                let title = summary.title.clone();
                let uploader = summary.uploader.clone();
                let job_id = job.id;
                tokio::task::spawn_blocking(move || {
                    history.update_metadata(job_id, title.as_deref(), uploader.as_deref())
                })
                .await
                .ok();
            }

            info!("download job {} succeeded", job.id);
            Ok(())
        }
        Err(error) => {
            let status = if matches!(error, DownloadError::Canceled) {
                JobStatus::Canceled
            } else {
                JobStatus::Failed
            };
            job.status_tx.send_replace(status);
            let message = error_message(&error);
            let event = if status == JobStatus::Canceled {
                DownloadEvent::Status(JobStatus::Canceled)
            } else {
                DownloadEvent::Failed(message.clone())
            };
            job.events_tx.send(event).await.ok();
            finalize_history(&job, status, None, Some(message.clone())).await;
            if status == JobStatus::Canceled {
                warn!("download job {} canceled", job.id);
            } else {
                error!("download job {} failed: {message}", job.id);
            }
            Err(error)
        }
    }
}

async fn execute_download(job: Arc<JobRuntime>) -> Result<DownloadSummary, DownloadError> {
    let mut command = build_command(&job);
    command.stdout(std::process::Stdio::piped());
    command.stderr(std::process::Stdio::piped());

    let mut child = command
        .spawn()
        .map_err(|source| DownloadError::Spawn { source })?;
    let stderr = child.stderr.take().ok_or_else(|| DownloadError::Spawn {
        source: std::io::Error::new(std::io::ErrorKind::Other, "missing stderr"),
    })?;
    let mut stderr_lines = BufReader::new(stderr).lines();
    let mut stderr_buffer = String::new();
    let mut destination_path: Option<PathBuf> = None;

    loop {
        tokio::select! {
            _ = job.cancel_token.cancelled() => {
                warn!("cancel request received for job {}", job.id);
                terminate_child(&mut child).await?;
                return Err(DownloadError::Canceled);
            }
            line = stderr_lines.next_line() => {
                match line {
                    Ok(Some(line)) => {
                        if !stderr_buffer.is_empty() {
                            stderr_buffer.push('\n');
                        }
                        stderr_buffer.push_str(&line);
                        handle_process_line(&job, &line, &mut destination_path).await;
                    }
                    Ok(None) => break,
                    Err(source) => return Err(DownloadError::Io { source }),
                }
            }
        }
    }

    let timeout = job.download_settings.timeout_sec;
    let status = if timeout > 0 {
        match time::timeout(Duration::from_secs(timeout), child.wait()).await {
            Ok(result) => result.map_err(|source| DownloadError::Io { source })?,
            Err(_) => {
                let _ = terminate_child(&mut child).await;
                return Err(DownloadError::Timeout(timeout));
            }
        }
    } else {
        child
            .wait()
            .await
            .map_err(|source| DownloadError::Io { source })?
    };
    if !status.success() {
        return Err(DownloadError::CommandFailed {
            status: status.code(),
            stderr: stderr_buffer,
        });
    }

    let metadata = tokio::task::spawn_blocking({
        let output_dir = job.request.output_dir.clone();
        move || read_latest_metadata(&output_dir)
    })
    .await
    .map_err(|source| DownloadError::Join { source })?;

    let summary = DownloadSummary {
        id: job.id,
        url: job.request.url.clone(),
        status: JobStatus::Succeeded,
        title: metadata.as_ref().and_then(|m| m.title.clone()),
        uploader: metadata.as_ref().and_then(|m| m.uploader.clone()),
        file_path: metadata
            .as_ref()
            .and_then(|m| m.file_path.clone())
            .or(destination_path.clone()),
        completed_at: Utc::now(),
        error_message: None,
    };

    Ok(summary)
}

async fn finalize_history(
    job: &JobRuntime,
    status: JobStatus,
    file_path: Option<&Path>,
    error_message: Option<String>,
) {
    let history = job.history.clone();
    let job_id = job.id;
    let path = file_path.map(|p| p.to_path_buf());
    let error_code = match status {
        JobStatus::Succeeded => None,
        JobStatus::Canceled => Some("Canceled".to_string()),
        JobStatus::Failed => Some("Failed".to_string()),
        _ => None,
    };

    let row_id = {
        let mut guard = job.history_row_id.lock();
        guard.take()
    };

    if row_id.is_some() {
        tokio::task::spawn_blocking(move || {
            let _ = history.mark_completed(
                job_id,
                status,
                path.as_deref(),
                error_code.as_deref(),
                error_message.as_deref(),
            );
        })
        .await
        .ok();
    }
}

fn build_command(job: &JobRuntime) -> Command {
    let mut command = Command::new(&job.advanced_settings.yt_dlp_path);
    
    // Hide command window on Windows
    #[cfg(target_os = "windows")]
    {
        #[allow(unused_imports)]
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    
    command.arg("--extract-audio");
    command
        .arg("--audio-format")
        .arg(job.request.format.to_string());
    command.arg("--audio-quality").arg("0");
    command.arg("--write-info-json");
    command.arg("--no-playlist");
    command.arg("--progress");
    command.arg("--newline");

    let output_template = job.request.output_dir.join("%(title)s.%(ext)s");
    command
        .arg("--output")
        .arg(&output_template);

    if let Some(cookie) = &job.request.cookie_file {
        command.arg("--cookies").arg(cookie);
    }

    for extra in &job.request.extra_args {
        command.arg(extra);
    }

    command.arg(&job.request.url);
    command
}

async fn terminate_child(child: &mut Child) -> Result<(), DownloadError> {
    #[cfg(windows)]
    {
        child
            .kill()
            .await
            .map_err(|source| DownloadError::Io { source })?;
    }
    #[cfg(not(windows))]
    {
        child
            .start_kill()
            .map_err(|source| DownloadError::Io { source })?;
    }
    Ok(())
}

async fn handle_process_line(job: &JobRuntime, line: &str, destination: &mut Option<PathBuf>) {
    debug!("yt-dlp: {line}");
    job.events_tx
        .send(DownloadEvent::LogLine(line.to_string()))
        .await
        .ok();

    if let Some(captures) = DESTINATION_RE.captures(line) {
        if let Some(path_match) = captures.name("path") {
            *destination = Some(PathBuf::from(path_match.as_str()));
        }
    }

    if let Some(progress) = parse_progress(line) {
        job.progress_tx.send_replace(Some(progress.clone()));
        job.events_tx
            .send(DownloadEvent::Progress(progress))
            .await
            .ok();
    }
}

fn parse_progress(line: &str) -> Option<ProgressSnapshot> {
    let captures = PROGRESS_RE.captures(line)?;
    let mut snapshot = ProgressSnapshot::default();
    snapshot.percent = captures
        .name("percent")
        .and_then(|m| m.as_str().parse::<f32>().ok());
    snapshot.downloaded_bytes = captures.name("downloaded").and_then(|m| {
        parse_bytes(
            m.as_str(),
            captures.name("downloaded_unit").map(|u| u.as_str()),
        )
    });
    snapshot.total_bytes = captures
        .name("total")
        .and_then(|m| parse_bytes(m.as_str(), captures.name("total_unit").map(|u| u.as_str())));
    snapshot.speed_bytes_per_sec = captures
        .name("speed")
        .and_then(|m| parse_speed(m.as_str(), captures.name("speed_unit").map(|u| u.as_str())));
    snapshot.eta = captures.name("eta").and_then(|m| parse_eta(m.as_str()));
    Some(snapshot)
}

fn parse_bytes(value: &str, unit: Option<&str>) -> Option<u64> {
    let number = value.parse::<f64>().ok()?;
    let multiplier = match unit.unwrap_or("Bytes") {
        "KiB" => 1024.0,
        "MiB" => 1024.0 * 1024.0,
        "GiB" => 1024.0 * 1024.0 * 1024.0,
        "TiB" => 1024.0 * 1024.0 * 1024.0 * 1024.0,
        "B" | "Bytes" => 1.0,
        other if other.eq_ignore_ascii_case("kb") => 1000.0,
        other if other.eq_ignore_ascii_case("mb") => 1000.0 * 1000.0,
        other if other.eq_ignore_ascii_case("gb") => 1000.0 * 1000.0 * 1000.0,
        _ => 1.0,
    };
    Some((number * multiplier) as u64)
}
fn parse_speed(value: &str, unit: Option<&str>) -> Option<u64> {
    let unit = unit.map(|u| u.trim_end_matches("/s"));
    parse_bytes(value, unit)
}

fn parse_eta(value: &str) -> Option<Duration> {
    if value.is_empty() {
        return None;
    }
    let parts: Vec<&str> = value.split(':').collect();
    let mut seconds = 0u64;
    for part in parts {
        seconds = seconds
            .checked_mul(60)?
            .checked_add(part.parse::<u64>().ok()?)?;
    }
    Some(Duration::from_secs(seconds))
}

fn error_message(error: &DownloadError) -> String {
    match error {
        DownloadError::InvalidUrl(url) => format!("invalid url: {url}"),
        DownloadError::MissingDependency(dep) => format!("missing dependency: {dep}"),
        DownloadError::Spawn { source } => format!("failed to spawn command: {source}"),
        DownloadError::CommandFailed { status, stderr } => {
            format!("command failed (status {status:?}): {stderr}")
        }
        DownloadError::Canceled => "download canceled".to_string(),
        DownloadError::Timeout(seconds) => format!("download timed out after {seconds} seconds"),
        DownloadError::Io { source } => format!("io error: {source}"),
        DownloadError::Join { source } => format!("task join error: {source}"),
    }
}

struct DownloadMetadata {
    title: Option<String>,
    uploader: Option<String>,
    file_path: Option<PathBuf>,
}

fn read_latest_metadata(output_dir: &Path) -> Option<DownloadMetadata> {
    let entries = std::fs::read_dir(output_dir).ok()?;
    let mut newest: Option<(SystemTime, PathBuf)> = None;

    for entry in entries.flatten() {
        let path = entry.path();
        if !is_info_json(&path) {
            continue;
        }
        let metadata = entry.metadata().ok()?;
        let modified = metadata.modified().ok()?;
        if newest
            .as_ref()
            .map(|(time, _)| &modified > time)
            .unwrap_or(true)
        {
            newest = Some((modified, path));
        }
    }

    let (_, info_path) = newest?;
    let info_content = std::fs::read_to_string(&info_path).ok()?;
    let value: serde_json::Value = serde_json::from_str(&info_content).ok()?;

    let title = value
        .get("title")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let uploader = value
        .get("uploader")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let ext = value
        .get("ext")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let base_name = info_path
        .file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| name.strip_suffix(".info.json"))?;

    let ext = ext.unwrap_or_else(|| "m4a".to_string());
    let candidate = info_path
        .parent()
        .unwrap_or(output_dir)
        .join(format!("{base_name}.{ext}"));
    let file_path = if candidate.exists() {
        Some(candidate)
    } else {
        None
    };

    Some(DownloadMetadata {
        title,
        uploader,
        file_path,
    })
}

fn is_info_json(path: &Path) -> bool {
    match path.file_name().and_then(|name| name.to_str()) {
        Some(name) => name.ends_with(".info.json"),
        None => false,
    }
}
fn download_error_from_history(error: HistoryError) -> DownloadError {
    DownloadError::Io {
        source: io::Error::new(io::ErrorKind::Other, error.to_string()),
    }
}
unsafe impl Send for JobRuntime {}
unsafe impl Sync for JobRuntime {}
