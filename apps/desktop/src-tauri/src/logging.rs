//! 有界的本地诊断日志。
//!
//! 只记录固定事件名、错误码与计数；不记录路径、SQL、恢复令牌、Salt、Hook 原文、
//! 工具输入输出或用户内容。日志失败绝不阻断应用启动。

use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use chrono::{SecondsFormat, Utc};

const MAX_LOG_BYTES: u64 = 2 * 1024 * 1024;
const TAIL_BYTES: u64 = 4096;

struct LogWriter {
    path: PathBuf,
    file: Option<File>,
}

impl LogWriter {
    fn open(path: PathBuf) -> io::Result<Self> {
        let file = Self::open_active(&path)?;
        Ok(Self {
            path,
            file: Some(file),
        })
    }

    fn open_active(path: &Path) -> io::Result<File> {
        OpenOptions::new().create(true).append(true).open(path)
    }

    fn append(&mut self, line: &[u8]) -> io::Result<()> {
        let line_size = line.len() as u64;
        if line_size > MAX_LOG_BYTES {
            return Err(io::Error::other("diagnostic event exceeds log size limit"));
        }
        if self.file.is_none() {
            self.file = Some(Self::open_active(&self.path)?);
        }
        let size = self
            .file
            .as_ref()
            .expect("active log is open")
            .metadata()?
            .len();
        if size.saturating_add(line_size) > MAX_LOG_BYTES {
            drop(self.file.take());
            fs::rename(&self.path, self.path.with_extension("log.1"))?;
            self.file = Some(Self::open_active(&self.path)?);
        }
        let file = self.file.as_mut().expect("active log is open");
        file.write_all(line)?;
        file.flush()
    }
}

static LOGGER: OnceLock<Mutex<LogWriter>> = OnceLock::new();

pub fn init(data_directory: &Path) {
    let log_directory = data_directory.join("logs");
    if fs::create_dir_all(&log_directory).is_err() {
        return;
    }
    let path = log_directory.join("xushu.log");
    let previous_was_unclean = previous_run_was_unclean(&path);
    let Ok(writer) = LogWriter::open(path) else {
        return;
    };
    if LOGGER.set(Mutex::new(writer)).is_err() {
        return;
    }

    if previous_was_unclean {
        record("WARN", "previous_run_unclean_exit");
    }
    record(
        "INFO",
        concat!("app_start version=", env!("CARGO_PKG_VERSION")),
    );
}

pub fn install_panic_hook() {
    #[cfg(debug_assertions)]
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        record("ERROR", "panic");
        #[cfg(debug_assertions)]
        default_hook(info);
        #[cfg(not(debug_assertions))]
        let _ = info;
    }));
}

pub fn record(level: &str, event: &str) {
    let Some(logger) = LOGGER.get() else {
        return;
    };
    let Ok(mut logger) = logger.lock() else {
        return;
    };
    let line = format!(
        "{} {level} {event}\n",
        Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
    );
    let _ = logger.append(line.as_bytes());
    #[cfg(debug_assertions)]
    eprintln!("[xushu] {level} {event}");
}

pub fn record_error(context: &str, code: &str) {
    record("ERROR", &format!("{context} code={code}"));
}

fn previous_run_was_unclean(path: &Path) -> bool {
    let Ok(mut file) = File::open(path) else {
        return false;
    };
    let length = file.metadata().map(|meta| meta.len()).unwrap_or(0);
    let start = length.saturating_sub(TAIL_BYTES);
    if file.seek(SeekFrom::Start(start)).is_err() {
        return false;
    }
    let mut tail = Vec::new();
    if file.take(TAIL_BYTES).read_to_end(&mut tail).is_err() || tail.is_empty() {
        return false;
    }
    !last_line_is_clean_exit(&String::from_utf8_lossy(&tail))
}

fn last_line_is_clean_exit(tail: &str) -> bool {
    tail.lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .is_some_and(|line| line.trim_end().ends_with(" INFO app_exit"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_exit_detection_ignores_blank_lines() {
        assert!(last_line_is_clean_exit(
            "2026-01-01T00:00:00.000Z INFO app_start\n2026-01-01T00:00:01.000Z INFO app_exit\n"
        ));
        assert!(last_line_is_clean_exit(
            "2026-01-01T00:00:01.000Z INFO app_exit\n\n"
        ));
        assert!(!last_line_is_clean_exit(
            "2026-01-01T00:00:00.000Z INFO app_start\n"
        ));
        assert!(!last_line_is_clean_exit(""));
    }

    #[test]
    fn a_single_running_session_cannot_grow_the_active_log_past_its_limit() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("xushu.log");
        fs::write(&path, vec![b'a'; MAX_LOG_BYTES as usize - 1]).unwrap();
        let mut logger = LogWriter::open(path.clone()).unwrap();
        logger.append(b"a").unwrap();
        assert_eq!(fs::metadata(&path).unwrap().len(), MAX_LOG_BYTES);
        logger.append(b"INFO app_exit\n").unwrap();
        assert!(fs::metadata(&path).unwrap().len() <= MAX_LOG_BYTES);
        assert_eq!(fs::read(&path).unwrap(), b"INFO app_exit\n");
        assert!(path.with_extension("log.1").is_file());
    }

    #[test]
    fn diagnostic_tail_recognizes_an_unclean_exit_after_a_split_utf8_character() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("xushu.log");
        fs::write(&path, []).unwrap();
        assert!(!previous_run_was_unclean(&path));
        let suffix = "\nINFO app_start\n";
        let prior = format!(
            "已{}{suffix}",
            "a".repeat(TAIL_BYTES as usize - suffix.len() - 1)
        );
        fs::write(&path, prior).unwrap();
        assert!(previous_run_was_unclean(&path));
    }
}
