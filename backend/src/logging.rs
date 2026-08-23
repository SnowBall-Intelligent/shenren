use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use axum::http::StatusCode;
use chrono::{Duration, NaiveDate, Utc};
use chrono_tz::Tz;
use tracing_subscriber::filter::{LevelFilter, Targets};
use tracing_subscriber::fmt::format::Writer;
use tracing_subscriber::fmt::time::FormatTime;
use tracing_subscriber::layer::{Layer, SubscriberExt};
use tracing_subscriber::util::SubscriberInitExt;

use crate::config::Config;

pub const AUDIT_TARGET: &str = "shenren::audit";
const LOG_ROOT: &str = "data/logs";
const MAX_FILE_BYTES: u64 = 1024 * 1024;
const RETENTION_DAYS: i64 = 30;
const TRUNCATION_MARKER: &[u8] = b"...[TRUNCATED]\n";

pub struct LogGuards {
    _system: Option<tracing_appender::non_blocking::WorkerGuard>,
    _audit: Option<tracing_appender::non_blocking::WorkerGuard>,
}

#[derive(Clone)]
struct LogTimer {
    timezone: Tz,
}

impl FormatTime for LogTimer {
    fn format_time(&self, writer: &mut Writer<'_>) -> fmt::Result {
        write!(
            writer,
            "{}",
            Utc::now()
                .with_timezone(&self.timezone)
                .format("%Y-%m-%d %H:%M:%S%.3f %:z")
        )
    }
}

pub fn init(config: &Config) -> Result<LogGuards, Box<dyn std::error::Error>> {
    if !config.log_enabled {
        tracing_subscriber::registry().try_init()?;
        return Ok(LogGuards {
            _system: None,
            _audit: None,
        });
    }

    let level = parse_level_filter(&config.log_level)?;
    let root = PathBuf::from(LOG_ROOT);
    let system_writer = RollingFileWriter::new(
        root.clone(),
        "system",
        config.log_timezone,
        MAX_FILE_BYTES,
        RETENTION_DAYS,
    )?;
    let audit_writer = RollingFileWriter::new(
        root.join("admin"),
        "audit",
        config.log_timezone,
        MAX_FILE_BYTES,
        RETENTION_DAYS,
    )?;
    let (system_writer, system_guard) =
        tracing_appender::non_blocking::NonBlockingBuilder::default()
            .lossy(false)
            .finish(system_writer);
    let (audit_writer, audit_guard) = tracing_appender::non_blocking::NonBlockingBuilder::default()
        .lossy(false)
        .finish(audit_writer);
    let timer = LogTimer {
        timezone: config.log_timezone,
    };

    let console = tracing_subscriber::fmt::layer()
        .with_timer(timer.clone())
        .with_filter(level);
    let system = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .with_timer(timer.clone())
        .with_writer(system_writer)
        .with_filter(
            Targets::new()
                .with_default(level)
                .with_target(AUDIT_TARGET, LevelFilter::OFF),
        );
    let audit = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .with_timer(timer)
        .with_writer(audit_writer)
        .with_filter(
            Targets::new()
                .with_default(LevelFilter::OFF)
                .with_target(AUDIT_TARGET, level),
        );

    tracing_subscriber::registry()
        .with(console)
        .with(system)
        .with(audit)
        .try_init()?;

    Ok(LogGuards {
        _system: Some(system_guard),
        _audit: Some(audit_guard),
    })
}

fn parse_level_filter(level: &str) -> Result<LevelFilter, String> {
    match level {
        "error" => Ok(LevelFilter::ERROR),
        "warn" => Ok(LevelFilter::WARN),
        "info" => Ok(LevelFilter::INFO),
        "debug" => Ok(LevelFilter::DEBUG),
        "trace" => Ok(LevelFilter::TRACE),
        _ => Err(format!("unsupported log level: {level}")),
    }
}

#[derive(Clone)]
struct RollingFileWriter {
    inner: Arc<Mutex<RollingState>>,
}

struct RollingState {
    directory: PathBuf,
    prefix: String,
    timezone: Tz,
    max_bytes: u64,
    retention_days: i64,
    date: NaiveDate,
    segment: u32,
    bytes_written: u64,
    file: File,
    last_cleanup: NaiveDate,
}

impl RollingFileWriter {
    fn new(
        directory: PathBuf,
        prefix: &str,
        timezone: Tz,
        max_bytes: u64,
        retention_days: i64,
    ) -> io::Result<Self> {
        fs::create_dir_all(&directory)?;
        let date = Utc::now().with_timezone(&timezone).date_naive();
        cleanup_expired(&directory, prefix, date, retention_days)?;
        let (file, segment, bytes_written) =
            open_latest_segment(&directory, prefix, date, max_bytes)?;
        Ok(Self {
            inner: Arc::new(Mutex::new(RollingState {
                directory,
                prefix: prefix.to_string(),
                timezone,
                max_bytes,
                retention_days,
                date,
                segment,
                bytes_written,
                file,
                last_cleanup: date,
            })),
        })
    }
}

impl Write for RollingFileWriter {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        let original_len = buffer.len();
        let result = self
            .inner
            .lock()
            .map_err(|_| io::Error::other("log writer lock poisoned"))
            .and_then(|mut state| state.write_record(buffer));
        if let Err(error) = result {
            eprintln!("log file write failed: {error}");
        }
        Ok(original_len)
    }

    fn flush(&mut self) -> io::Result<()> {
        match self.inner.lock() {
            Ok(mut state) => {
                if let Err(error) = state.file.flush() {
                    eprintln!("log file flush failed: {error}");
                }
            }
            Err(_) => eprintln!("log file flush failed: log writer lock poisoned"),
        }
        Ok(())
    }
}

impl RollingState {
    fn write_record(&mut self, buffer: &[u8]) -> io::Result<()> {
        let today = Utc::now().with_timezone(&self.timezone).date_naive();
        self.write_record_for_date(buffer, today)
    }

    fn write_record_for_date(&mut self, buffer: &[u8], today: NaiveDate) -> io::Result<()> {
        if today != self.date {
            self.open_date(today)?;
        }
        if today != self.last_cleanup {
            cleanup_expired(&self.directory, &self.prefix, today, self.retention_days)?;
            self.last_cleanup = today;
        }

        let payload = truncate_record(buffer, self.max_bytes as usize);
        if self.bytes_written > 0
            && self.bytes_written + u64::try_from(payload.len()).unwrap_or(u64::MAX)
                > self.max_bytes
        {
            self.open_segment(self.segment.saturating_add(1))?;
        }
        self.file.write_all(&payload)?;
        self.bytes_written += payload.len() as u64;
        Ok(())
    }

    fn open_date(&mut self, date: NaiveDate) -> io::Result<()> {
        let (file, segment, bytes_written) =
            open_latest_segment(&self.directory, &self.prefix, date, self.max_bytes)?;
        self.date = date;
        self.segment = segment;
        self.bytes_written = bytes_written;
        self.file = file;
        Ok(())
    }

    fn open_segment(&mut self, segment: u32) -> io::Result<()> {
        let path = segment_path(&self.directory, &self.prefix, self.date, segment);
        self.file = OpenOptions::new().create(true).append(true).open(path)?;
        self.segment = segment;
        self.bytes_written = self.file.metadata()?.len();
        Ok(())
    }
}

fn truncate_record(buffer: &[u8], max_bytes: usize) -> Vec<u8> {
    if buffer.len() <= max_bytes {
        return buffer.to_vec();
    }
    let keep = max_bytes.saturating_sub(TRUNCATION_MARKER.len());
    let mut payload = buffer[..keep].to_vec();
    payload.extend_from_slice(TRUNCATION_MARKER);
    payload
}

fn open_latest_segment(
    directory: &Path,
    prefix: &str,
    date: NaiveDate,
    max_bytes: u64,
) -> io::Result<(File, u32, u64)> {
    let latest = fs::read_dir(directory)?
        .filter_map(Result::ok)
        .filter_map(|entry| parse_log_filename(prefix, &entry.file_name().to_string_lossy()))
        .filter(|(file_date, _)| *file_date == date)
        .map(|(_, segment)| segment)
        .max()
        .unwrap_or(1);
    let latest_path = segment_path(directory, prefix, date, latest);
    let latest_len = latest_path.metadata().map(|meta| meta.len()).unwrap_or(0);
    let segment = if latest_len >= max_bytes {
        latest.saturating_add(1)
    } else {
        latest
    };
    let path = segment_path(directory, prefix, date, segment);
    let file = OpenOptions::new().create(true).append(true).open(path)?;
    let bytes_written = file.metadata()?.len();
    Ok((file, segment, bytes_written))
}

fn cleanup_expired(
    directory: &Path,
    prefix: &str,
    today: NaiveDate,
    retention_days: i64,
) -> io::Result<()> {
    let cutoff = today - Duration::days(retention_days);
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let filename = entry.file_name();
        let filename = filename.to_string_lossy();
        if let Some((date, _)) = parse_log_filename(prefix, &filename) {
            if date < cutoff {
                fs::remove_file(entry.path())?;
            }
        }
    }
    Ok(())
}

fn segment_path(directory: &Path, prefix: &str, date: NaiveDate, segment: u32) -> PathBuf {
    directory.join(format!(
        "{prefix}-{}-{segment:03}.log",
        date.format("%Y-%m-%d")
    ))
}

fn parse_log_filename(prefix: &str, filename: &str) -> Option<(NaiveDate, u32)> {
    let rest = filename.strip_prefix(&format!("{prefix}-"))?;
    let rest = rest.strip_suffix(".log")?;
    if rest.len() < 12 {
        return None;
    }
    let (date, segment) = rest.split_at(10);
    let segment = segment.strip_prefix('-')?.parse().ok()?;
    Some((NaiveDate::parse_from_str(date, "%Y-%m-%d").ok()?, segment))
}

pub fn redact_sensitive(value: &str) -> String {
    const SENSITIVE_MARKERS: [&str; 8] = [
        "password",
        "secret",
        "token",
        "cookie",
        "authorization",
        "api_key",
        "apikey",
        "srk_",
    ];
    let lower = value.to_ascii_lowercase();
    if SENSITIVE_MARKERS
        .iter()
        .any(|marker| lower.contains(marker))
    {
        "[REDACTED]".to_string()
    } else {
        value
            .chars()
            .filter(|c| !c.is_control())
            .take(256)
            .collect()
    }
}

#[allow(clippy::too_many_arguments)]
pub fn emit_audit(
    actor_id: Option<i64>,
    username: Option<&str>,
    role: Option<&str>,
    action: &str,
    resource: &str,
    resource_id: Option<&str>,
    status: StatusCode,
    client_ip: std::net::IpAddr,
) {
    let actor_id = actor_id
        .map(|id| id.to_string())
        .unwrap_or_else(|| "-".into());
    let username = username.map(redact_sensitive).unwrap_or_else(|| "-".into());
    let role = role.map(redact_sensitive).unwrap_or_else(|| "-".into());
    let resource_id = resource_id
        .map(redact_sensitive)
        .unwrap_or_else(|| "-".into());
    let outcome = if status.is_success() {
        "success"
    } else {
        "failure"
    };

    if status.is_server_error() {
        tracing::error!(
            target: AUDIT_TARGET,
            admin_id = %actor_id,
            username = %username,
            role = %role,
            action,
            resource,
            resource_id = %resource_id,
            outcome,
            status = status.as_u16(),
            client_ip = %client_ip,
            "admin operation"
        );
    } else if status.is_success() {
        tracing::info!(
            target: AUDIT_TARGET,
            admin_id = %actor_id,
            username = %username,
            role = %role,
            action,
            resource,
            resource_id = %resource_id,
            outcome,
            status = status.as_u16(),
            client_ip = %client_ip,
            "admin operation"
        );
    } else {
        tracing::warn!(
            target: AUDIT_TARGET,
            admin_id = %actor_id,
            username = %username,
            role = %role,
            action,
            resource,
            resource_id = %resource_id,
            outcome,
            status = status.as_u16(),
            client_ip = %client_ip,
            "admin operation"
        );
    }
}

#[derive(Default)]
struct AuditFields {
    username: Option<String>,
    resource_id: Option<String>,
}

#[derive(Clone, Default)]
pub struct AuditContext {
    fields: Arc<Mutex<AuditFields>>,
}

impl AuditContext {
    pub fn set_username(&self, username: &str) {
        if let Ok(mut fields) = self.fields.lock() {
            fields.username = Some(redact_sensitive(username.trim()));
        }
    }

    pub fn username(&self) -> Option<String> {
        self.fields
            .lock()
            .ok()
            .and_then(|fields| fields.username.clone())
    }

    pub fn set_resource_id(&self, resource_id: impl ToString) {
        if let Ok(mut fields) = self.fields.lock() {
            fields.resource_id = Some(redact_sensitive(&resource_id.to_string()));
        }
    }

    pub fn resource_id(&self) -> Option<String> {
        self.fields
            .lock()
            .ok()
            .and_then(|fields| fields.resource_id.clone())
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::Write;
    use std::path::PathBuf;

    use chrono::{Duration, Utc};
    use chrono_tz::UTC;
    use uuid::Uuid;

    use super::{cleanup_expired, parse_log_filename, redact_sensitive, RollingFileWriter};

    fn test_dir(label: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("shenren-{label}-{}", Uuid::new_v4()));
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn redacts_key_and_secret_like_values() {
        assert_eq!(redact_sensitive("srk_very-secret-value"), "[REDACTED]");
        assert_eq!(redact_sensitive("Authorization: Bearer abc"), "[REDACTED]");
        assert_eq!(redact_sensitive("plain-admin"), "plain-admin");
    }

    #[test]
    fn parses_only_owned_log_names() {
        let parsed = parse_log_filename("system", "system-2026-08-23-001.log").unwrap();
        assert_eq!(parsed.1, 1);
        assert!(parse_log_filename("system", "unrelated.log").is_none());
        assert!(parse_log_filename("system", "audit-2026-08-23-001.log").is_none());
    }

    #[test]
    fn rotates_before_exceeding_limit_and_truncates_large_records() {
        let dir = test_dir("rotation");
        let mut writer = RollingFileWriter::new(dir.clone(), "system", UTC, 64, 30).unwrap();
        writer.write_all(&[b'a'; 48]).unwrap();
        writer.write_all(&[b'b'; 48]).unwrap();
        writer.write_all(&[b'c'; 128]).unwrap();
        writer.flush().unwrap();

        let mut sizes = fs::read_dir(&dir)
            .unwrap()
            .map(|entry| entry.unwrap().metadata().unwrap().len())
            .collect::<Vec<_>>();
        sizes.sort_unstable();
        assert_eq!(sizes.len(), 3);
        assert!(sizes.into_iter().all(|size| size <= 64));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn cleanup_only_removes_expired_owned_files() {
        let dir = test_dir("retention");
        let today = Utc::now().date_naive();
        let old = today - Duration::days(31);
        let recent = today - Duration::days(2);
        fs::write(
            dir.join(format!("system-{}-001.log", old.format("%Y-%m-%d"))),
            b"old",
        )
        .unwrap();
        fs::write(
            dir.join(format!("system-{}-001.log", recent.format("%Y-%m-%d"))),
            b"recent",
        )
        .unwrap();
        fs::write(dir.join("keep.txt"), b"keep").unwrap();

        cleanup_expired(&dir, "system", today, 30).unwrap();

        assert!(!dir
            .join(format!("system-{}-001.log", old.format("%Y-%m-%d")))
            .exists());
        assert!(dir
            .join(format!("system-{}-001.log", recent.format("%Y-%m-%d")))
            .exists());
        assert!(dir.join("keep.txt").exists());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn reopens_latest_segment_and_rotates_on_date_change() {
        let dir = test_dir("restart-date");
        let today = Utc::now().date_naive();
        {
            let mut writer = RollingFileWriter::new(dir.clone(), "system", UTC, 128, 30).unwrap();
            writer.write_all(b"first\n").unwrap();
            writer.flush().unwrap();
        }
        {
            let mut writer = RollingFileWriter::new(dir.clone(), "system", UTC, 128, 30).unwrap();
            writer.write_all(b"second\n").unwrap();
            writer.flush().unwrap();
            let mut state = writer.inner.lock().unwrap();
            state
                .write_record_for_date(b"tomorrow\n", today + Duration::days(1))
                .unwrap();
            state.file.flush().unwrap();
        }

        let today_file = dir.join(format!("system-{}-001.log", today.format("%Y-%m-%d")));
        let tomorrow_file = dir.join(format!(
            "system-{}-001.log",
            (today + Duration::days(1)).format("%Y-%m-%d")
        ));
        assert_eq!(fs::read_to_string(today_file).unwrap(), "first\nsecond\n");
        assert_eq!(fs::read_to_string(tomorrow_file).unwrap(), "tomorrow\n");
        fs::remove_dir_all(dir).unwrap();
    }
}
