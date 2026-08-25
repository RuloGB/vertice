//! The application's file-based diagnostic log: the ONLY module in this
//! workspace that names `log` or `chrono`, and the second (and last) module
//! CA-16 permits to write (`freshness/cache.rs` is the first). Owns the log
//! file's location, its fixed-column line format, the in-memory size
//! counter, and size-bounded rotation with one retained predecessor
//! (design §6, §7).
//!
//! `app_data_dir: &Path` is always received as a parameter, mirroring
//! `freshness::cache::store_path` — this module never resolves it itself,
//! never reads the process environment directly, and contains no literal
//! absolute path. That is
//! what keeps it liftable into a future shared crate untouched, and what
//! keeps `tests/read_only_audit.rs`'s per-module proof obligations
//! satisfiable (design §3, §10).

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use chrono::SecondsFormat;
use log::{Level, LevelFilter, Log, Metadata, Record};

/// The current log file's name.
pub const FILE_NAME: &str = "vertice.log";
/// The single retained predecessor's name, written by rotation.
pub const ROTATED_FILE_NAME: &str = "vertice.log.1";
/// The size threshold that triggers rotation before the next write (design
/// §7): 1 MiB.
pub const MAX_BYTES: u64 = 1024 * 1024;

/// The absolute path of the current log file, a child of `app_data_dir` —
/// never a literal path (mirrors `freshness::cache::store_path`).
pub fn log_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join(FILE_NAME)
}

/// The absolute path of the single retained predecessor file, a child of
/// `app_data_dir`.
fn rotated_log_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join(ROTATED_FILE_NAME)
}

/// Failure to initialise the sink: the directory could not be created, or
/// the log file could not be opened. Reported exactly once, on stderr, by
/// the caller (design §12, D5 class 2) — this type carries no further
/// behaviour of its own.
#[derive(Debug)]
pub struct InitError(io::Error);

impl std::fmt::Display for InitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "could not initialize the application log: {}", self.0)
    }
}

impl std::error::Error for InitError {}

/// Construct the sink and install it as the global `log` implementation.
/// Idempotent by `log`'s own contract: a second call returns `Err` because
/// `set_boxed_logger` only ever succeeds once per process (design §6).
pub fn init(app_data_dir: &Path) -> Result<(), InitError> {
    let sink = FileSink::open(app_data_dir).map_err(InitError)?;
    log::set_boxed_logger(Box::new(sink))
        .map(|()| log::set_max_level(LevelFilter::Info))
        .map_err(|err| InitError(io::Error::other(err)))
}

/// One open log file plus the number of bytes written to it since it was
/// last (re)opened — tracked in memory so the per-line size check is an
/// integer comparison, never a `metadata()` syscall (design §6).
struct LogFile {
    file: File,
    written: u64,
}

/// The testable sink, deliberately separable from the global `log`
/// installation: unit tests exercise rotation and format against a temp
/// directory with no global logger involved (FRD §10's stubbed-
/// `app_data_dir()` seam).
pub(crate) struct FileSink {
    path: PathBuf,
    rotated: PathBuf,
    state: Mutex<LogFile>,
}

impl FileSink {
    /// Open (creating if necessary) the current log file under
    /// `app_data_dir`, creating the directory first if it does not yet
    /// exist (mirrors `freshness::cache.rs`'s own directory-creation, but
    /// deliberately not shared — sharing would put a write primitive in a
    /// third module and require a third audit exception; design §9).
    pub(crate) fn open(app_data_dir: &Path) -> io::Result<Self> {
        fs::create_dir_all(app_data_dir)?;
        let path = log_path(app_data_dir);
        let rotated = rotated_log_path(app_data_dir);
        let file = OpenOptions::new().create(true).append(true).open(&path)?;
        let written = file.metadata()?.len();
        Ok(Self {
            path,
            rotated,
            state: Mutex::new(LogFile { file, written }),
        })
    }

    /// Write one already-formatted line, rotating first if it would push
    /// the current file at or above [`MAX_BYTES`] (design §7). Infallible
    /// by contract (D5 class 1): a write failure — file removed underneath
    /// the sink, disk full, permissions — is swallowed, never panics,
    /// never propagates.
    pub(crate) fn write_line(&self, line: &str) {
        let bytes = line.as_bytes();
        let n = bytes.len() as u64;

        let mut guard = self.state.lock().unwrap_or_else(|err| err.into_inner());

        if guard.written > 0 && guard.written + n > MAX_BYTES {
            self.rotate(&mut guard);
        }

        if guard.file.write_all(bytes).is_ok() {
            guard.written += n;
        }
        // Preserve evidence over throughput: no error surfaces beyond this
        // point (design §7, §12).
    }

    /// Rotate = flush, drop the handle, rename current -> predecessor
    /// (overwriting any existing predecessor — a single atomic replace on
    /// both POSIX `rename` and Win32 `MoveFileEx(REPLACE_EXISTING)`),
    /// reopen a fresh empty current file. If any step fails, the current
    /// handle is left usable and the next line is still written to it
    /// (design §7: rotation failing must never drop a line).
    fn rotate(&self, guard: &mut LogFile) {
        if guard.file.flush().is_err() {
            return;
        }
        if fs::rename(&self.path, &self.rotated).is_err() {
            return;
        }
        match OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&self.path)
        {
            Ok(fresh) => {
                guard.file = fresh;
                guard.written = 0;
            }
            Err(_) => {
                // The rename succeeded but reopening failed: leave the
                // stale handle in place rather than losing the sink
                // entirely. The next line's `write_all` will surface the
                // failure and be swallowed by `write_line`, self-healing on
                // a subsequent successful rotation.
            }
        }
    }
}

impl Log for FileSink {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let timestamp = current_timestamp();
        let file = record.file().unwrap_or("<unknown>");
        let line = record.line().unwrap_or(0);
        let formatted = format_line(
            &timestamp,
            record.level(),
            file,
            line,
            &record.args().to_string(),
        );
        self.write_line(&formatted);
    }

    fn flush(&self) {
        let mut guard = self.state.lock().unwrap_or_else(|err| err.into_inner());
        let _ = guard.file.flush();
    }
}

/// The current local time, formatted as one whitespace-free RFC 3339 token
/// with an explicit offset and millisecond precision (design §5):
/// `2026-08-24T14:03:11.482+02:00`. Since chrono 0.4.20 `Local` parses the
/// platform zone itself rather than calling `localtime_r`, so this is sound
/// to call from any thread (design §5's residual caveat: safe unless a
/// concurrent mutation of the `TZ` environment variable races it, which
/// this workspace contains no instance of).
fn current_timestamp() -> String {
    chrono::Local::now().to_rfc3339_opts(SecondsFormat::Millis, false)
}

/// Fixed-column plain-text line format (design §6, application-logging
/// spec): `ts␣␣LEVEL␣␣file:line␣␣msg\n`. `LEVEL` is left-padded to 5
/// characters. Embedded newlines in `message` are replaced with a space so
/// one event is always one line. Pure — no I/O, no clock read.
pub(crate) fn format_line(
    timestamp: &str,
    level: Level,
    file: &str,
    line: u32,
    message: &str,
) -> String {
    let flattened_message = message.replace('\n', " ");
    format!("{timestamp}  {level:<5}  {file}:{line}  {flattened_message}\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static UNIQUE: AtomicU64 = AtomicU64::new(0);

    fn temp_app_data_dir(label: &str) -> PathBuf {
        let unique = UNIQUE.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "vertice-logging-test-{label}-{}-{unique}",
            std::process::id()
        ))
    }

    #[test]
    fn format_line_has_the_fixed_column_shape_with_one_trailing_newline() {
        let line = format_line(
            "2026-08-24T14:03:11.482+02:00",
            Level::Info,
            "src/commands.rs",
            49,
            "scan finished",
        );

        assert_eq!(
            line,
            "2026-08-24T14:03:11.482+02:00  INFO   src/commands.rs:49  scan finished\n"
        );
        assert_eq!(line.matches('\n').count(), 1);
        assert!(line.ends_with('\n'));
    }

    #[test]
    fn format_line_left_pads_every_level_to_five_characters() {
        let warn_line = format_line("ts", Level::Warn, "f.rs", 1, "m");
        assert!(warn_line.starts_with("ts  WARN   f.rs:1"));

        let error_line = format_line("ts", Level::Error, "f.rs", 1, "m");
        assert!(error_line.starts_with("ts  ERROR  f.rs:1"));
    }

    #[test]
    fn format_line_replaces_interior_newlines_in_the_message_with_a_space() {
        let line = format_line("ts", Level::Info, "f.rs", 1, "first\nsecond");

        assert_eq!(line.matches('\n').count(), 1);
        assert!(line.contains("first second"));
    }

    #[test]
    fn current_timestamp_parses_as_rfc3339_with_a_non_empty_offset_and_no_space() {
        let token = current_timestamp();

        assert!(!token.contains(' '));
        let parsed = chrono::DateTime::parse_from_rfc3339(&token)
            .expect("the timestamp token must parse as RFC 3339");
        assert_ne!(parsed.offset().local_minus_utc(), i32::MIN); // offset is always present/typed
    }

    #[test]
    fn fresh_install_has_exactly_one_log_file() {
        let app_data_dir = temp_app_data_dir("fresh-install");
        let sink = FileSink::open(&app_data_dir).expect("sink must open against a fresh dir");

        sink.write_line("one line\n");

        assert!(log_path(&app_data_dir).is_file());
        assert!(!rotated_log_path(&app_data_dir).is_file());
    }

    #[test]
    fn writing_past_max_bytes_rotates_leaving_exactly_two_files_with_no_line_duplicated_or_torn() {
        let app_data_dir = temp_app_data_dir("rotation");
        let sink = FileSink::open(&app_data_dir).expect("sink must open against a fresh dir");

        // Each line is ~100 bytes; MAX_BYTES is 1 MiB, so write comfortably
        // past it with synthetic long lines (design §16's accepted
        // alternative to a test-only limit override).
        let long_message = "x".repeat(200_000);
        for i in 0..10 {
            sink.write_line(&format!("line-{i}-{long_message}\n"));
        }

        let current = fs::read_to_string(log_path(&app_data_dir)).expect("current file readable");
        let predecessor =
            fs::read_to_string(rotated_log_path(&app_data_dir)).expect("predecessor file readable");

        assert!(rotated_log_path(&app_data_dir).is_file());
        assert!(log_path(&app_data_dir).is_file());

        // No file has three log files' worth of siblings.
        let file_count = fs::read_dir(&app_data_dir).unwrap().count();
        assert_eq!(file_count, 2);

        // The newest line is whole, in the current file.
        assert!(current.contains("line-9-"));
        assert!(current.ends_with('\n'));
        // The predecessor holds earlier lines whole, not truncated.
        assert!(predecessor.ends_with('\n'));
        // No line appears in both files.
        for line in current.lines() {
            if !line.is_empty() {
                assert!(!predecessor.contains(line));
            }
        }
    }

    #[test]
    fn write_line_keeps_returning_even_after_the_underlying_file_is_removed() {
        let app_data_dir = temp_app_data_dir("removed-file");
        let sink = FileSink::open(&app_data_dir).expect("sink must open against a fresh dir");

        fs::remove_file(log_path(&app_data_dir)).expect("test setup removal must succeed");

        // Must not panic, must simply return.
        sink.write_line("a line after the file was removed\n");
        sink.write_line("another line\n");
    }

    #[test]
    fn init_against_an_uncreatable_directory_returns_err_and_does_not_panic() {
        // A file, not a directory, at the would-be app_data_dir path: any
        // attempt to `create_dir_all` onto it must fail.
        let parent = temp_app_data_dir("init-failure-parent");
        fs::create_dir_all(&parent).expect("test setup parent dir must be creatable");
        let blocked_path = parent.join("blocked-by-a-file");
        fs::write(&blocked_path, b"not a directory").expect("test setup file must be writable");

        let result = init(&blocked_path.join("nested"));

        assert!(result.is_err());
    }
}
