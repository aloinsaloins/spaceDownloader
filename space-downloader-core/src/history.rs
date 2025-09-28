use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, Row};
use uuid::Uuid;

use crate::config::{AudioFormat, ParseAudioFormatError};
use crate::download::JobStatus;
use crate::error::HistoryError;

const DEFAULT_DB_PATH: &str = "history/history.db";

#[derive(Clone)]
pub struct HistoryRepository {
    path: PathBuf,
}

impl HistoryRepository {
    pub fn open(path: Option<PathBuf>) -> Result<Self, HistoryError> {
        let resolved = path.unwrap_or_else(|| PathBuf::from(DEFAULT_DB_PATH));
        if let Some(parent) = resolved.parent() {
            fs::create_dir_all(parent).map_err(|source| HistoryError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        let repo = Self { path: resolved };
        repo.initialize()?;
        Ok(repo)
    }

    fn initialize(&self) -> Result<(), HistoryError> {
        let connection = self.connection()?;
        connection
            .execute_batch(
                "PRAGMA journal_mode=WAL;
                 CREATE TABLE IF NOT EXISTS downloads (
                     id INTEGER PRIMARY KEY AUTOINCREMENT,
                     job_id TEXT NOT NULL,
                     url TEXT NOT NULL,
                     format TEXT NOT NULL,
                     title TEXT,
                     uploader TEXT,
                     status TEXT NOT NULL,
                     started_at TEXT NOT NULL,
                     ended_at TEXT,
                     file_path TEXT,
                     error_code TEXT,
                     error_message TEXT
                 );
                 CREATE INDEX IF NOT EXISTS idx_downloads_job_id ON downloads(job_id);",
            )
            .map_err(|source| HistoryError::Initialize {
                path: self.path.clone(),
                source,
            })?;
        Ok(())
    }

    pub fn record_queued(
        &self,
        job_id: Uuid,
        url: &str,
        format: AudioFormat,
    ) -> Result<i64, HistoryError> {
        let connection = self.connection()?;
        connection
            .execute(
                "INSERT INTO downloads (job_id, url, format, status, started_at) VALUES (?, ?, ?, ?, ?)",
                params![
                    job_id.to_string(),
                    url,
                    format.to_string(),
                    JobStatus::Queued.as_str(),
                    Utc::now().to_rfc3339(),
                ],
            )
            .map_err(|source| HistoryError::Query { source })?;
        Ok(connection.last_insert_rowid())
    }

    pub fn update_metadata(
        &self,
        job_id: Uuid,
        title: Option<&str>,
        uploader: Option<&str>,
    ) -> Result<(), HistoryError> {
        let connection = self.connection()?;
        connection
            .execute(
                "UPDATE downloads SET title = COALESCE(?, title), uploader = COALESCE(?, uploader) WHERE job_id = ?",
                params![title, uploader, job_id.to_string()],
            )
            .map_err(|source| HistoryError::Query { source })?;
        Ok(())
    }

    pub fn mark_completed(
        &self,
        job_id: Uuid,
        status: JobStatus,
        file_path: Option<&Path>,
        error_code: Option<&str>,
        error_message: Option<&str>,
    ) -> Result<(), HistoryError> {
        let connection = self.connection()?;
        connection
            .execute(
                "UPDATE downloads
                 SET status = ?,
                     ended_at = ?,
                     file_path = ?,
                     error_code = ?,
                     error_message = ?
                 WHERE job_id = ?",
                params![
                    status.as_str(),
                    Utc::now().to_rfc3339(),
                    file_path.map(|p| p.to_string_lossy().to_string()),
                    error_code,
                    error_message,
                    job_id.to_string(),
                ],
            )
            .map_err(|source| HistoryError::Query { source })?;
        Ok(())
    }

    pub fn recent(&self, limit: usize) -> Result<Vec<DownloadHistoryEntry>, HistoryError> {
        let connection = self.connection()?;
        let mut statement = connection
            .prepare(
                "SELECT id, job_id, url, format, title, uploader, status, started_at, ended_at, file_path, error_code, error_message
                 FROM downloads
                 ORDER BY started_at DESC
                 LIMIT ?",
            )
            .map_err(|source| HistoryError::Query { source })?;

        let mut rows = statement
            .query(params![limit as i64])
            .map_err(|source| HistoryError::Query { source })?;

        let mut entries = Vec::new();
        while let Some(row) = rows
            .next()
            .map_err(|source| HistoryError::Query { source })?
        {
            entries.push(map_entry(row)?);
        }

        Ok(entries)
    }

    fn connection(&self) -> Result<Connection, HistoryError> {
        Connection::open(&self.path).map_err(|source| HistoryError::Initialize {
            path: self.path.clone(),
            source,
        })
    }
}

fn map_entry(row: &Row<'_>) -> Result<DownloadHistoryEntry, HistoryError> {
    let started_at: String = row
        .get("started_at")
        .map_err(|source| HistoryError::Query { source })?;
    let started_at = DateTime::parse_from_rfc3339(&started_at)
        .map_err(|source| HistoryError::Query {
            source: rusqlite::Error::FromSqlConversionFailure(
                0,
                rusqlite::types::Type::Text,
                Box::new(source),
            ),
        })?
        .with_timezone(&Utc);

    let ended_at: Option<String> = row
        .get("ended_at")
        .map_err(|source| HistoryError::Query { source })?;
    let ended_at = match ended_at {
        Some(value) => Some(
            DateTime::parse_from_rfc3339(&value)
                .map_err(|source| HistoryError::Query {
                    source: rusqlite::Error::FromSqlConversionFailure(
                        0,
                        rusqlite::types::Type::Text,
                        Box::new(source),
                    ),
                })?
                .with_timezone(&Utc),
        ),
        None => None,
    };

    let format_text: String = row
        .get("format")
        .map_err(|source| HistoryError::Query { source })?;
    let format = AudioFormat::from_str(&format_text).map_err(|ParseAudioFormatError(value)| {
        HistoryError::Query {
            source: rusqlite::Error::FromSqlConversionFailure(
                0,
                rusqlite::types::Type::Text,
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("invalid audio format {value}"),
                )),
            ),
        }
    })?;

    Ok(DownloadHistoryEntry {
        id: row
            .get("id")
            .map_err(|source| HistoryError::Query { source })?,
        job_id: row
            .get::<_, String>("job_id")
            .map_err(|source| HistoryError::Query { source })?
            .parse()
            .map_err(|err| HistoryError::Query {
                source: rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Text,
                    Box::new(err),
                ),
            })?,
        url: row
            .get("url")
            .map_err(|source| HistoryError::Query { source })?,
        format,
        title: row
            .get("title")
            .map_err(|source| HistoryError::Query { source })?,
        uploader: row
            .get("uploader")
            .map_err(|source| HistoryError::Query { source })?,
        status: JobStatus::from_str(
            &row.get::<_, String>("status")
                .map_err(|source| HistoryError::Query { source })?,
        ),
        started_at,
        ended_at,
        file_path: row
            .get::<_, Option<String>>("file_path")
            .map_err(|source| HistoryError::Query { source })?
            .map(PathBuf::from),
        error_code: row
            .get("error_code")
            .map_err(|source| HistoryError::Query { source })?,
        error_message: row
            .get("error_message")
            .map_err(|source| HistoryError::Query { source })?,
    })
}

#[derive(Debug, Clone)]
pub struct DownloadHistoryEntry {
    pub id: i64,
    pub job_id: Uuid,
    pub url: String,
    pub format: AudioFormat,
    pub title: Option<String>,
    pub uploader: Option<String>,
    pub status: JobStatus,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub file_path: Option<PathBuf>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
}

impl std::fmt::Debug for HistoryRepository {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HistoryRepository")
            .field("path", &self.path)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn initialize_and_store_history() {
        let dir = tempdir().unwrap();
        let repo = HistoryRepository::open(Some(dir.path().join("history.db"))).unwrap();
        let job_id = Uuid::new_v4();
        repo.record_queued(job_id, "https://example.com/space", AudioFormat::M4a)
            .unwrap();
        repo.mark_completed(job_id, JobStatus::Succeeded, None, None, None)
            .unwrap();
        let entries = repo.recent(10).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].status, JobStatus::Succeeded);
    }
}
