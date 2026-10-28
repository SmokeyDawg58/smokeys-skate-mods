//! PC test journal. Observes the game; never advances or repairs game state.
//! Disk writes run on a separate thread. A bounded queue cannot stall SIM.
use std::{
    collections::{BTreeMap, VecDeque},
    fmt::{self, Write as _},
    fs::{self, File, OpenOptions},
    io::{self, BufWriter, Write},
    path::{Path, PathBuf},
    sync::{
        Mutex, OnceLock,
        atomic::{AtomicU64, Ordering},
        mpsc::{self, Receiver, SyncSender, TrySendError},
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const RECENT: usize = 32768;
const RECENT_BYTES: usize = 16 * 1024 * 1024;
const QUEUE: usize = 4096;
const SEGMENT_BYTES: u64 = 8 * 1024 * 1024;
const SEGMENTS: u64 = 8;
static LOGGER: OnceLock<Logger> = OnceLock::new();

struct Logger {
    directory: PathBuf,
    start: Instant,
    tx: SyncSender<Message>,
    recent: Mutex<Recent>,
    progress: Mutex<BTreeMap<String, Progress>>,
    sequence: AtomicU64,
    dropped: AtomicU64,
    reports: AtomicU64,
    anomalies: Mutex<BTreeMap<&'static str, Instant>>,
}
#[derive(Clone, Default)]
struct Recent {
    lines: VecDeque<String>,
    bytes: usize,
}
impl Recent {
    fn push(&mut self, line: String) {
        while !self.lines.is_empty()
            && (self.lines.len() >= RECENT || self.bytes + line.len() > RECENT_BYTES)
        {
            if let Some(old) = self.lines.pop_front() {
                self.bytes -= old.len();
            }
        }
        if line.len() <= RECENT_BYTES {
            self.bytes += line.len();
            self.lines.push_back(line);
        }
    }
}
#[derive(Clone)]
struct Progress {
    stage: &'static str,
    since: Instant,
}
enum Message {
    Line(String),
    Flush(SyncSender<()>),
    Stop(SyncSender<()>),
}

/// Install before starting the live game. Each run gets its own directory.
/// The guard flushes on normal exit; failure reports also flush independently.
pub fn install(root: &Path) -> io::Result<RunGuard> {
    if LOGGER.get().is_some() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "test journal already installed",
        ));
    }
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let directory = root.join(format!("run-{timestamp}-{}", std::process::id()));
    fs::create_dir_all(&directory)?;
    let writer = Journal::new(&directory, SEGMENT_BYTES, SEGMENTS)?;
    let (tx, rx) = mpsc::sync_channel(QUEUE);
    LOGGER
        .set(Logger {
            directory: directory.clone(),
            start: Instant::now(),
            tx,
            recent: Mutex::new(Recent::default()),
            progress: Mutex::new(BTreeMap::new()),
            sequence: AtomicU64::new(0),
            dropped: AtomicU64::new(0),
            reports: AtomicU64::new(0),
            anomalies: Mutex::new(BTreeMap::new()),
        })
        .map_err(|_| io::Error::other("test journal installation raced"))?;
    std::thread::Builder::new()
        .name("test journal".into())
        .spawn(move || worker(rx, writer))?;
    event(
        "session_start",
        format_args!(
            "schema=1 pid={} unix_ns={timestamp} os={} arch={} rolling_bytes={} recent_records={RECENT}",
            std::process::id(),
            std::env::consts::OS,
            std::env::consts::ARCH,
            SEGMENT_BYTES * SEGMENTS
        ),
    );
    flush();
    Ok(RunGuard { directory })
}

pub struct RunGuard {
    directory: PathBuf,
}
impl RunGuard {
    #[must_use]
    pub fn directory(&self) -> &Path {
        &self.directory
    }
}
impl Drop for RunGuard {
    fn drop(&mut self) {
        event("session_end", format_args!("normal scope exit"));
        if let Some(logger) = LOGGER.get() {
            barrier(logger, true);
        }
    }
}

#[must_use]
pub fn enabled() -> bool {
    LOGGER.get().is_some()
}

/// Independent of the console quiet policy. JSONL uses a string detail field so
/// errors, nonfinite floats and platform debug values remain valid JSON.
pub fn event(kind: &str, detail: fmt::Arguments<'_>) {
    let Some(logger) = LOGGER.get() else {
        return;
    };
    let line = logger.line(kind, &detail.to_string());
    logger.record(line);
}
impl Logger {
    fn record(&self, line: String) {
        if let Ok(mut recent) = self.recent.lock() {
            recent.push(line.clone());
        }
        if let Err(TrySendError::Full(_) | TrySendError::Disconnected(_)) =
            self.tx.try_send(Message::Line(line))
        {
            self.dropped.fetch_add(1, Ordering::Relaxed);
        }
    }
}
impl Logger {
    fn line(&self, kind: &str, detail: &str) -> String {
        let thread = std::thread::current();
        let identity = format!("{}:{:?}", thread.name().unwrap_or("unnamed"), thread.id());
        format!(
            "{{\"seq\":{},\"ms\":{:.3},\"thread\":{},\"kind\":{},\"detail\":{}}}\n",
            self.sequence.fetch_add(1, Ordering::Relaxed),
            self.start.elapsed().as_secs_f64() * 1000.,
            quoted(&identity),
            quoted(kind),
            quoted(detail)
        )
    }
}

/// A scoped breadcrumb. Watchdog reporting does not take game/session locks.
/// Only explicitly active work is monitored: waiting for input is not a hang.
pub fn span(stage: &'static str) -> StageGuard {
    let mut guard = StageGuard {
        thread: None,
        previous: None,
    };
    if let Some(logger) = LOGGER.get() {
        let current = std::thread::current();
        let identity = format!("{}:{:?}", current.name().unwrap_or("unnamed"), current.id());
        if let Ok(mut progress) = logger.progress.lock() {
            guard.previous = progress.insert(
                identity.clone(),
                Progress {
                    stage,
                    since: Instant::now(),
                },
            );
            guard.thread = Some(identity);
        }
    }
    guard
}
/// Timed breadcrumb for a source-integration phase. The eight-millisecond
/// threshold controls diagnostic output only; it never controls simulation.
pub fn timed_span(stage: &'static str) -> TimedStageGuard {
    TimedStageGuard {
        _stage: span(stage),
        stage,
        started: enabled().then(Instant::now),
    }
}
pub struct TimedStageGuard {
    _stage: StageGuard,
    stage: &'static str,
    started: Option<Instant>,
}
impl Drop for TimedStageGuard {
    fn drop(&mut self) {
        if let Some(started) = self.started {
            let elapsed = started.elapsed();
            if elapsed >= Duration::from_millis(8) {
                event("slow_stage", format_args!("stage={} duration_ms={:.3}",
                    self.stage, elapsed.as_secs_f64() * 1000.));
            }
        }
    }
}

pub struct StageGuard {
    thread: Option<String>,
    previous: Option<Progress>,
}
impl Drop for StageGuard {
    fn drop(&mut self) {
        if let (Some(logger), Some(thread)) = (LOGGER.get(), &self.thread)
            && let Ok(mut progress) = logger.progress.lock()
        {
            if let Some(previous) = self.previous.take() {
                progress.insert(thread.clone(), previous);
            } else {
                progress.remove(thread);
            }
        }
    }
}

/// Save the recent ring immediately, including the original panic/error and
/// active stages. Uses try_lock in a panic hook to avoid poisoned game locks.
pub fn failure(kind: &str, detail: &str) -> Option<PathBuf> {
    let logger = LOGGER.get()?;
    event(kind, format_args!("{detail}"));
    // Release both journal locks before any filesystem operation. A slow disk
    // in the watchdog must never hold the producer's history lock.
    let progress = logger.progress.try_lock().ok().map(|map| map.clone());
    let recent = logger.recent.try_lock().ok().map(|lines| lines.clone());
    let path = logger.directory.join(format!(
        "failure-{:02}.jsonl",
        logger.reports.fetch_add(1, Ordering::Relaxed) % 8
    ));
    let result = (|| -> io::Result<()> {
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&path)?;
        file.write_all(logger.line("failure", detail).as_bytes())?;
        if let Some(progress) = progress {
            for (thread, value) in progress.iter() {
                file.write_all(
                    logger
                        .line(
                            "active_stage",
                            &format!(
                                "thread={thread} stage={} duration_ms={:.3}",
                                value.stage,
                                value.since.elapsed().as_secs_f64() * 1000.
                            ),
                        )
                        .as_bytes(),
                )?;
            }
        }
        if let Some(recent) = recent {
            for line in &recent.lines {
                file.write_all(line.as_bytes())?;
            }
        }
        file.sync_all()
    })();
    if let Err(error) = result {
        std::eprintln!("Could not save failure history {}: {error}", path.display());
        return None;
    }
    Some(path)
}

/// Repeated invalid output is retained in events, but cannot cause a disk dump
/// on every simulation tick. Report each anomaly kind at most once per 30 s.
pub fn anomaly(kind: &'static str, detail: &str) {
    let Some(logger) = LOGGER.get() else {
        return;
    };
    let report = logger.anomalies.try_lock().ok().is_some_and(|mut reports| {
        if reports
            .get(kind)
            .is_some_and(|last| last.elapsed() < Duration::from_secs(30))
        {
            return false;
        }
        reports.insert(kind, Instant::now());
        true
    });
    if report {
        let _ = failure(kind, detail);
    }
}

/// Durable checkpoints, called off the hot path (startup, errors, shutdown).
pub fn flush() {
    if let Some(logger) = LOGGER.get() {
        barrier(logger, false);
    }
}

fn barrier(logger: &Logger, stop: bool) {
    let (tx, rx) = mpsc::sync_channel(1);
    let mut message = if stop {
        Message::Stop(tx)
    } else {
        Message::Flush(tx)
    };
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        match logger.tx.try_send(message) {
            Ok(()) => {
                let _ = rx.recv_timeout(deadline.saturating_duration_since(Instant::now()));
                return;
            }
            Err(TrySendError::Full(returned)) => {
                if Instant::now() >= deadline {
                    return;
                }
                message = returned;
                std::thread::sleep(Duration::from_millis(1));
            }
            Err(TrySendError::Disconnected(_)) => return,
        }
    }
}

fn worker(rx: Receiver<Message>, mut writer: Journal) {
    let mut heartbeat = Instant::now();
    let mut warnings: BTreeMap<String, Instant> = BTreeMap::new();
    loop {
        let result = match rx.recv_timeout(Duration::from_millis(250)) {
            Ok(Message::Line(line)) => writer.write(&line),
            Ok(Message::Flush(done)) => {
                let result = writer.sync();
                let _ = done.try_send(());
                result
            }
            Ok(Message::Stop(done)) => {
                let _ = writer.sync();
                let _ = done.try_send(());
                return;
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                let _ = writer.sync();
                return;
            }
            Err(mpsc::RecvTimeoutError::Timeout) => Ok(()),
        };
        if let Err(error) = result {
            std::eprintln!("Test journal disk write failed: {error}");
            return;
        }
        if heartbeat.elapsed() >= Duration::from_secs(1) {
            heartbeat = Instant::now();
            if let Some(logger) = LOGGER.get() {
                let dropped = logger.dropped.swap(0, Ordering::Relaxed);
                let progress = logger
                    .progress
                    .lock()
                    .map(|map| map.clone())
                    .unwrap_or_default();
                let mut active = String::new();
                for (thread, value) in &progress {
                    let duration = value.since.elapsed();
                    let _ = write!(
                        active,
                        "[{thread} {} {:.0}ms] ",
                        value.stage,
                        duration.as_secs_f64() * 1000.
                    );
                    if duration >= Duration::from_secs(10)
                        && warnings
                            .get(thread)
                            .is_none_or(|last| last.elapsed() >= Duration::from_secs(30))
                    {
                        let detail = format!(
                            "thread={thread} stage={} stalled_ms={} (slow work or possible hang)",
                            value.stage,
                            duration.as_millis()
                        );
                        event("stage_stalled", format_args!("{detail}"));
                        let _ = failure("watchdog", &detail);
                        warnings.insert(thread.clone(), Instant::now());
                    }
                }
                warnings.retain(|thread, _| progress.contains_key(thread));
                // Write directly: a saturated producer queue must not suppress
                // the heartbeat or its explicit lost-record count.
                let _ = writer.write(&logger.line(
                    "heartbeat",
                    &format!("dropped_records={dropped} active={active}"),
                ));
            }
            if let Err(error) = writer.sync() {
                std::eprintln!("Test journal flush failed: {error}");
                return;
            }
        }
    }
}

struct Journal {
    directory: PathBuf,
    file: BufWriter<File>,
    segment: u64,
    bytes: u64,
    limit: u64,
    retained: u64,
}
impl Journal {
    fn new(directory: &Path, limit: u64, retained: u64) -> io::Result<Self> {
        let file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(directory.join("events-000000.jsonl"))?;
        Ok(Self {
            directory: directory.into(),
            file: BufWriter::new(file),
            segment: 0,
            bytes: 0,
            limit,
            retained,
        })
    }
    fn write(&mut self, line: &str) -> io::Result<()> {
        if self.bytes != 0 && self.bytes + line.len() as u64 > self.limit {
            self.sync()?;
            self.segment += 1;
            self.file = BufWriter::new(
                OpenOptions::new().create_new(true).write(true).open(
                    self.directory
                        .join(format!("events-{:06}.jsonl", self.segment)),
                )?,
            );
            self.bytes = 0;
            if self.segment >= self.retained {
                fs::remove_file(
                    self.directory
                        .join(format!("events-{:06}.jsonl", self.segment - self.retained)),
                )?;
            }
        }
        self.file.write_all(line.as_bytes())?;
        self.bytes += line.len() as u64;
        Ok(())
    }
    fn sync(&mut self) -> io::Result<()> {
        self.file.flush()?;
        self.file.get_ref().sync_data()
    }
}

fn quoted(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if ch < ' ' => {
                let _ = write!(out, "\\u{:04x}", u32::from(ch));
            }
            ch => out.push(ch),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_journal_rotates_complete_records_and_keeps_newest_segments() {
        let root = std::env::temp_dir().join(format!(
            "skate-test-journal-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        let mut journal = Journal::new(&root, 20, 2).unwrap();
        for i in 0..5 {
            journal.write(&format!("{{\"record\":{i}}}\n")).unwrap();
        }
        journal.sync().unwrap();
        drop(journal);
        assert_eq!(fs::read_dir(&root).unwrap().count(), 2);
        assert_eq!(
            fs::read_to_string(root.join("events-000003.jsonl")).unwrap(),
            "{\"record\":3}\n"
        );
        assert_eq!(
            fs::read_to_string(root.join("events-000004.jsonl")).unwrap(),
            "{\"record\":4}\n"
        );
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn test_journal_escapes_error_text_and_non_ascii_without_broken_json() {
        assert_eq!(
            quoted("\"C:\\test\"\n\r\t\0 ðŸ›¹"),
            "\"\\\"C:\\\\test\\\"\\n\\r\\t\\u0000 ðŸ›¹\""
        );
    }
    #[test]
    fn test_journal_flushes_quiet_events_and_saves_active_failure_history() {
        let root = std::env::temp_dir().join(format!("skate-journal-run-{}", std::process::id()));
        let run = install(&root).unwrap();
        let directory = run.directory().to_owned();
        let stage = span("test_physics");
        event("raw_input", format_args!("buttons=32768 sticks=[1,-2]"));
        // The logger's watchdog keeps running while its observed game thread
        // makes no progress. No game/window needs to be launched for this check.
        std::thread::sleep(Duration::from_millis(11_200));
        let failure = failure("returned_error", "visual \"cloth\"\nfailed").unwrap();
        let report = fs::read_to_string(failure).unwrap();
        assert!(report.contains("test_physics"));
        assert!(report.contains("raw_input"));
        assert!(report.contains("visual \\\"cloth\\\"\\nfailed"));
        drop(stage);
        flush();
        drop(run);
        let journal = fs::read_to_string(directory.join("events-000000.jsonl")).unwrap();
        assert!(journal.contains("session_start"));
        assert!(journal.contains("session_end"));
        assert!(journal.contains("returned_error"));
        assert!(journal.contains("stage_stalled"));
        assert!(journal.contains("heartbeat"));
        for line in journal.lines() {
            assert!(line.starts_with('{') && line.ends_with('}'));
        }
        fs::remove_dir_all(directory).unwrap();
        let _ = fs::remove_dir(root);
    }

    #[test]
    fn test_journal_full_disk_queue_keeps_memory_history_without_waiting() {
        let (tx, _unread) = mpsc::sync_channel(1);
        let logger = Logger {
            directory: PathBuf::new(),
            start: Instant::now(),
            tx,
            recent: Mutex::new(Recent::default()),
            progress: Mutex::new(BTreeMap::new()),
            sequence: AtomicU64::new(0),
            dropped: AtomicU64::new(0),
            reports: AtomicU64::new(0),
            anomalies: Mutex::new(BTreeMap::new()),
        };
        for index in 0..100 {
            logger.record(logger.line("frame", &format!("frame={index}")));
        }
        assert_eq!(logger.dropped.load(Ordering::Relaxed), 99);
        assert_eq!(logger.recent.lock().unwrap().lines.len(), 100);
        assert!(
            logger
                .recent
                .lock()
                .unwrap()
                .lines
                .back()
                .unwrap()
                .contains("frame=99")
        );
    }

    #[test]
    fn test_journal_recent_history_enforces_byte_and_record_bounds() {
        let mut recent = Recent::default();
        for _ in 0..4 {
            recent.push("x".repeat(RECENT_BYTES / 3));
        }
        assert_eq!(recent.lines.len(), 3);
        assert!(recent.bytes <= RECENT_BYTES);
        recent = Recent::default();
        for _ in 0..RECENT + 10 {
            recent.push("x".into());
        }
        assert_eq!(recent.lines.len(), RECENT);
        assert_eq!(recent.bytes, RECENT);
    }

    #[test]
    fn test_journal_shutdown_retries_a_full_queue_then_flushes_in_order() {
        let (tx, rx) = mpsc::sync_channel(1);
        tx.send(Message::Line("last frame".into())).unwrap();
        let logger = Logger {
            directory: PathBuf::new(),
            start: Instant::now(),
            tx,
            recent: Mutex::new(Recent::default()),
            progress: Mutex::new(BTreeMap::new()),
            sequence: AtomicU64::new(0),
            dropped: AtomicU64::new(0),
            reports: AtomicU64::new(0),
            anomalies: Mutex::new(BTreeMap::new()),
        };
        let consumer = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(20));
            assert!(matches!(rx.recv().unwrap(), Message::Line(line) if line == "last frame"));
            match rx.recv().unwrap() {
                Message::Stop(done) => done.send(()).unwrap(),
                _ => panic!("shutdown barrier absent"),
            }
        });
        barrier(&logger, true);
        consumer.join().unwrap();
    }
}
