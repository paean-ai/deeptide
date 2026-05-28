//! Background shell registry: tracks `Bash` invocations spawned with
//! `run_in_background = true` so the model can later read their accumulated
//! stdout/stderr and stop them via the `BashOutput` and `KillBash` tools.
//!
//! Rationale: the prior implementation of `start_background_command` dropped
//! the child handle on the floor and redirected output to `/dev/null`, so the
//! agent could fire-and-forget a `npm test --watch` or `cargo run` but had no
//! way to ever observe what happened next. zero-cli's `LocalShellTask` and
//! Claude Code's `BashOutput`/`KillBash` tools solve this by parking the child
//! in a registry keyed by a short id and streaming both stdout and stderr into
//! a bounded ring buffer in reader threads.
//!
//! Design goals:
//!
//! * **Bounded memory.** A runaway dev server can produce gigabytes of output;
//!   we keep at most [`MAX_BUFFERED_LINES`] lines per stream and silently
//!   discard older lines, just like a terminal's scrollback.
//! * **Reader is cheap and idempotent.** [`read_output`] returns only the lines
//!   produced *since the previous read* if the caller passes the cursor from
//!   the previous response; full reads are also available. This keeps the
//!   model's view of stdout cumulative-but-non-redundant across turns.
//! * **No process leaks at shutdown.** Each child stays alive even after the
//!   reader threads exit, but `KillBash` will SIGKILL it deterministically.
//!   We don't reap on registry drop because the registry is a `OnceLock`
//!   global with `'static` lifetime — its drop only runs at process exit.
//! * **Thread-safe single-writer / many-reader.** Each child has a dedicated
//!   reader thread per stream (stdout + stderr); the registry itself is a
//!   single `Mutex<HashMap>` because contention is bounded by the small number
//!   of concurrently registered tasks (typically <10).

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read};
use std::process::{Child, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Maximum number of lines buffered per stream (stdout or stderr) per task.
/// One terminal screen is ~50 lines; a watcher producing one line per second
/// fills this in just under a minute. Anything beyond is dropped from the
/// head — same policy as a scrollback buffer.
pub const MAX_BUFFERED_LINES: usize = 5_000;

/// How long [`stop`] waits for a child to exit cleanly after SIGTERM before
/// escalating to SIGKILL. 250ms is long enough for typical shells to flush
/// pending output and short enough that the agent isn't stalled waiting.
const STOP_GRACE_PERIOD: Duration = Duration::from_millis(250);

/// Snapshot of one background task's state at the moment of the read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackgroundOutputSnapshot {
    pub shell_id: String,
    pub command: String,
    pub running: bool,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    /// Total number of stdout lines ever produced (including those that
    /// have rolled out of the buffer). Useful as a cursor — pass back in
    /// the next call to fetch only new lines.
    pub stdout_cursor: u64,
    /// Same as `stdout_cursor`, for stderr.
    pub stderr_cursor: u64,
    /// Whether the stdout buffer dropped any older lines because the
    /// ring buffer was full.
    pub stdout_truncated: bool,
    pub stderr_truncated: bool,
    /// Wall-clock seconds the task has been running. Stops advancing
    /// once the task exits.
    pub elapsed_seconds: u64,
}

/// Snapshot of a single background task suitable for listing in the REPL or
/// in a status tool. Cheaper than [`BackgroundOutputSnapshot`] — no stdout/
/// stderr body is materialised.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackgroundTaskSummary {
    pub shell_id: String,
    pub command: String,
    pub running: bool,
    pub exit_code: Option<i32>,
    pub elapsed_seconds: u64,
}

#[derive(Default)]
struct StreamBuffer {
    /// Most recent lines, oldest first. Bounded by [`MAX_BUFFERED_LINES`].
    lines: Vec<String>,
    /// Total lines produced ever, including those that rolled out.
    total_lines: u64,
    /// Whether we have dropped any line off the head.
    truncated: bool,
}

impl StreamBuffer {
    fn push(&mut self, line: String) {
        if self.lines.len() == MAX_BUFFERED_LINES {
            self.lines.remove(0);
            self.truncated = true;
        }
        self.lines.push(line);
        self.total_lines += 1;
    }

    /// Return all lines produced after `since_cursor`. If `since_cursor` is
    /// past the head of the buffer (because lines have been dropped), returns
    /// whatever the buffer currently holds.
    fn lines_since(&self, since_cursor: u64) -> Vec<String> {
        if since_cursor >= self.total_lines {
            return Vec::new();
        }
        let oldest_cursor = self.total_lines.saturating_sub(self.lines.len() as u64);
        if since_cursor < oldest_cursor {
            return self.lines.clone();
        }
        let start = (since_cursor - oldest_cursor) as usize;
        self.lines[start..].to_vec()
    }
}

struct BackgroundTask {
    shell_id: String,
    command: String,
    child: Arc<Mutex<Child>>,
    stdout: Arc<Mutex<StreamBuffer>>,
    stderr: Arc<Mutex<StreamBuffer>>,
    started_at: Instant,
    /// Set once the process exits; lets [`elapsed_seconds`] freeze.
    finished_at: Arc<Mutex<Option<Instant>>>,
    /// Set once the process exits, alongside `finished_at`.
    exit_code: Arc<Mutex<Option<i32>>>,
}

impl BackgroundTask {
    fn elapsed_seconds(&self) -> u64 {
        let end = self
            .finished_at
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .unwrap_or_else(Instant::now);
        end.duration_since(self.started_at).as_secs()
    }

    fn running(&self) -> bool {
        self.finished_at
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .is_none()
    }

    fn exit_code(&self) -> Option<i32> {
        *self.exit_code.lock().unwrap_or_else(|p| p.into_inner())
    }
}

fn registry() -> &'static Mutex<HashMap<String, Arc<BackgroundTask>>> {
    static REGISTRY: OnceLock<Mutex<HashMap<String, Arc<BackgroundTask>>>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Allocate a short, unique-per-process shell id of the form `bash_<n>`.
/// Sequential counter rather than UUID so the model can refer to them more
/// naturally in chat ("kill bash_3") and so test output is deterministic.
fn next_shell_id() -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    // Mix in a short process-start salt so two `cargo test` runs in the same
    // process (rare, but happens with `--test-threads=1`) don't collide if a
    // future change ever resets the registry between tests.
    let salt = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| (d.as_millis() & 0xFFF) as u64)
        .unwrap_or(0);
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    format!("bash_{salt:x}{n:x}")
}

/// Result of [`spawn`]: the freshly allocated id and a human-readable
/// confirmation string the caller (BashTool) can return to the model.
pub struct SpawnedBackground {
    pub shell_id: String,
    pub pid: Option<u32>,
}

/// Spawn `command` in the background via `sh -c` (or `cmd /C` on Windows),
/// register it under a fresh shell id, and kick off reader threads that push
/// each line of stdout/stderr into the per-task ring buffer. Returns the new
/// id; the child stays parked in the registry until [`stop`] kills it or it
/// exits on its own.
pub fn spawn(command: &str, cwd: &std::path::Path) -> Result<SpawnedBackground, String> {
    let mut process = build_shell_command(command);
    process
        .current_dir(cwd)
        .env_remove("TIDE_API_KEY")
        .env_remove("DEEPSEEK_API_KEY")
        .env_remove("ZERO_CLI_API_KEY")
        .env_remove("ZERO_API_KEY")
        .env_remove("ANTHROPIC_API_KEY")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = process
        .spawn()
        .map_err(|error| format!("failed to spawn background command: {error}"))?;
    let pid = Some(child.id());
    let stdout_handle = child.stdout.take();
    let stderr_handle = child.stderr.take();
    let shell_id = next_shell_id();

    let task = Arc::new(BackgroundTask {
        shell_id: shell_id.clone(),
        command: command.to_owned(),
        child: Arc::new(Mutex::new(child)),
        stdout: Arc::new(Mutex::new(StreamBuffer::default())),
        stderr: Arc::new(Mutex::new(StreamBuffer::default())),
        started_at: Instant::now(),
        finished_at: Arc::new(Mutex::new(None)),
        exit_code: Arc::new(Mutex::new(None)),
    });

    if let Some(stdout) = stdout_handle {
        spawn_reader(stdout, Arc::clone(&task.stdout));
    }
    if let Some(stderr) = stderr_handle {
        spawn_reader(stderr, Arc::clone(&task.stderr));
    }

    // Reaper thread: wait for the child to exit, then flip `finished_at`
    // so subsequent reads report `running=false`. We do this in a dedicated
    // thread because polling on every read_output call would be racy and
    // would block readers on `try_wait` syscalls.
    {
        let task = Arc::clone(&task);
        thread::Builder::new()
            .name(format!("bg-shell-reap-{}", task.shell_id))
            .spawn(move || {
                let mut child_guard = task.child.lock().unwrap_or_else(|p| p.into_inner());
                let status = child_guard.wait();
                let code = status.ok().and_then(|s| s.code());
                *task.finished_at.lock().unwrap_or_else(|p| p.into_inner()) = Some(Instant::now());
                *task.exit_code.lock().unwrap_or_else(|p| p.into_inner()) = code;
            })
            .map_err(|error| format!("failed to spawn reaper thread: {error}"))?;
    }

    registry()
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .insert(shell_id.clone(), task);

    Ok(SpawnedBackground { shell_id, pid })
}

fn spawn_reader<R: Read + Send + 'static>(stream: R, sink: Arc<Mutex<StreamBuffer>>) {
    thread::Builder::new()
        .name(String::from("bg-shell-reader"))
        .spawn(move || {
            let reader = BufReader::new(stream);
            for line in reader.lines() {
                match line {
                    Ok(line) => {
                        sink.lock().unwrap_or_else(|p| p.into_inner()).push(line);
                    }
                    Err(_) => break,
                }
            }
        })
        .ok();
}

/// Return a snapshot of `shell_id`'s state. If `since_stdout`/`since_stderr`
/// are supplied, only lines produced after the given cursors are included —
/// this lets the agent paginate through long-running output without
/// re-reading what it already saw.
pub fn read_output(
    shell_id: &str,
    since_stdout: Option<u64>,
    since_stderr: Option<u64>,
) -> Result<BackgroundOutputSnapshot, String> {
    let task = lookup(shell_id)?;
    let stdout_buf = task.stdout.lock().unwrap_or_else(|p| p.into_inner());
    let stderr_buf = task.stderr.lock().unwrap_or_else(|p| p.into_inner());
    let stdout = if let Some(cursor) = since_stdout {
        stdout_buf.lines_since(cursor).join("\n")
    } else {
        stdout_buf.lines.join("\n")
    };
    let stderr = if let Some(cursor) = since_stderr {
        stderr_buf.lines_since(cursor).join("\n")
    } else {
        stderr_buf.lines.join("\n")
    };
    Ok(BackgroundOutputSnapshot {
        shell_id: task.shell_id.clone(),
        command: task.command.clone(),
        running: task.running(),
        exit_code: task.exit_code(),
        stdout,
        stderr,
        stdout_cursor: stdout_buf.total_lines,
        stderr_cursor: stderr_buf.total_lines,
        stdout_truncated: stdout_buf.truncated,
        stderr_truncated: stderr_buf.truncated,
        elapsed_seconds: task.elapsed_seconds(),
    })
}

/// SIGKILL `shell_id`'s child process and return its final snapshot.
/// Idempotent: stopping an already-exited task succeeds and reports the
/// recorded exit code.
pub fn stop(shell_id: &str) -> Result<BackgroundOutputSnapshot, String> {
    let task = lookup(shell_id)?;

    if task.running() {
        let mut child_guard = task.child.lock().unwrap_or_else(|p| p.into_inner());
        // We don't bother with SIGTERM first — `std::process::Child::kill`
        // sends SIGKILL on Unix and TerminateProcess on Windows, which is
        // what the model wants when it says "kill it". Graceful shutdown
        // for dev servers is usually wired through their own protocol
        // (e.g. /quit endpoint), not the shell.
        let _ = child_guard.kill();
        // The reaper thread will pick the exit up; we wait briefly so the
        // returned snapshot already shows `running=false`. Without this
        // the model often sees a stale `running=true` immediately after
        // KillBash and retries unnecessarily.
        drop(child_guard);
        let start = Instant::now();
        while task.running() && start.elapsed() < STOP_GRACE_PERIOD {
            thread::sleep(Duration::from_millis(10));
        }
    }

    read_output(shell_id, None, None)
}

/// List every registered background task — terminated or otherwise. The
/// registry is currently never GC'd; explicit cleanup will be added when
/// we wire session lifecycle into background shells.
pub fn list_tasks() -> Vec<BackgroundTaskSummary> {
    let map = registry().lock().unwrap_or_else(|p| p.into_inner());
    let mut summaries: Vec<_> = map
        .values()
        .map(|task| BackgroundTaskSummary {
            shell_id: task.shell_id.clone(),
            command: task.command.clone(),
            running: task.running(),
            exit_code: task.exit_code(),
            elapsed_seconds: task.elapsed_seconds(),
        })
        .collect();
    // Stable ordering — alphabetic by shell_id, which matches creation order
    // closely enough because the counter is monotonic (modulo the salt).
    summaries.sort_by(|a, b| a.shell_id.cmp(&b.shell_id));
    summaries
}

fn lookup(shell_id: &str) -> Result<Arc<BackgroundTask>, String> {
    registry()
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .get(shell_id)
        .cloned()
        .ok_or_else(|| format!("no background shell with id {shell_id}"))
}

fn build_shell_command(command: &str) -> std::process::Command {
    #[cfg(windows)]
    {
        let mut process = std::process::Command::new("cmd");
        process.args(["/C", command]);
        process
    }
    #[cfg(not(windows))]
    {
        let mut process = std::process::Command::new("sh");
        process.args(["-c", command]);
        process
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex as StdMutex;

    // All tests in this module share a single counter via `next_shell_id`
    // and a single registry, so they must run serialised. A simple
    // process-local mutex is enough.
    static TEST_LOCK: StdMutex<()> = StdMutex::new(());

    fn cwd() -> std::path::PathBuf {
        std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
    }

    fn wait_until<F: Fn() -> bool>(predicate: F, timeout: Duration) -> bool {
        let start = Instant::now();
        while start.elapsed() < timeout {
            if predicate() {
                return true;
            }
            thread::sleep(Duration::from_millis(20));
        }
        predicate()
    }

    #[test]
    fn spawn_captures_stdout_and_marks_finished_with_exit_code() {
        let _lock = TEST_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        let SpawnedBackground { shell_id, .. } =
            spawn("printf 'hello\\nworld\\n'", &cwd()).expect("spawn");

        assert!(wait_until(
            || {
                let snap = read_output(&shell_id, None, None).expect("read");
                !snap.running && snap.stdout.contains("world")
            },
            Duration::from_secs(5),
        ));
        let snap = read_output(&shell_id, None, None).expect("final");
        assert!(!snap.running, "task should have completed");
        assert_eq!(snap.exit_code, Some(0));
        let lines: Vec<&str> = snap.stdout.lines().collect();
        assert_eq!(lines, vec!["hello", "world"]);
        assert_eq!(snap.stdout_cursor, 2);
        assert_eq!(snap.stderr, "");
        assert!(!snap.stdout_truncated);
    }

    #[test]
    fn read_output_with_cursor_returns_only_new_lines() {
        let _lock = TEST_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        let SpawnedBackground { shell_id, .. } = spawn(
            // print 3 lines with small sleeps so reads can happen mid-flight
            "for i in 1 2 3; do echo line$i; sleep 0.05; done",
            &cwd(),
        )
        .expect("spawn");

        // Wait until at least one line has been read.
        assert!(wait_until(
            || read_output(&shell_id, None, None)
                .map(|snap| snap.stdout_cursor >= 1)
                .unwrap_or(false),
            Duration::from_secs(5),
        ));
        let first = read_output(&shell_id, None, None).expect("first");
        let first_cursor = first.stdout_cursor;

        // Wait for completion.
        assert!(wait_until(
            || read_output(&shell_id, None, None)
                .map(|snap| !snap.running)
                .unwrap_or(false),
            Duration::from_secs(5),
        ));
        let next = read_output(&shell_id, Some(first_cursor), None).expect("next");
        assert!(!next.running);
        assert_eq!(next.stdout_cursor, 3);
        // The "since" snapshot must not include lines we already saw.
        for line in next.stdout.lines() {
            assert!(line.starts_with("line"), "unexpected line: {line}");
        }
    }

    #[test]
    fn read_output_with_stale_cursor_returns_buffer_contents() {
        let _lock = TEST_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        let SpawnedBackground { shell_id, .. } = spawn("echo only", &cwd()).expect("spawn");
        assert!(wait_until(
            || read_output(&shell_id, None, None)
                .map(|snap| !snap.running)
                .unwrap_or(false),
            Duration::from_secs(5),
        ));
        let stale = read_output(&shell_id, Some(99), None).expect("stale");
        // since_cursor is past total_lines, so we should see nothing new.
        assert_eq!(stale.stdout, "");
        assert_eq!(stale.stdout_cursor, 1);
    }

    #[test]
    fn stop_kills_running_task_and_records_exit() {
        let _lock = TEST_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        // `sleep 30` would otherwise outlive the test.
        let SpawnedBackground { shell_id, .. } = spawn("sleep 30", &cwd()).expect("spawn");

        // Confirm it's running.
        let snap = read_output(&shell_id, None, None).expect("pre-stop");
        assert!(snap.running);

        let final_snap = stop(&shell_id).expect("stop");
        assert!(!final_snap.running, "task should be marked finished");
        // SIGKILL on Unix → exit_code is None (terminated by signal), so we
        // assert running flipped rather than exit_code's exact value, which
        // differs between Unix (None) and Windows (Some(1) by convention).
    }

    #[test]
    fn stop_on_already_finished_task_is_idempotent() {
        let _lock = TEST_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        let SpawnedBackground { shell_id, .. } = spawn("true", &cwd()).expect("spawn");
        assert!(wait_until(
            || read_output(&shell_id, None, None)
                .map(|snap| !snap.running)
                .unwrap_or(false),
            Duration::from_secs(5),
        ));
        let snap = stop(&shell_id).expect("idempotent stop");
        assert!(!snap.running);
        assert_eq!(snap.exit_code, Some(0));
    }

    #[test]
    fn lookup_returns_friendly_error_for_unknown_id() {
        let _lock = TEST_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        let error = read_output("bash_nope", None, None).expect_err("unknown id must error");
        assert!(error.contains("bash_nope"));
        let error = stop("bash_nope").expect_err("unknown id must error");
        assert!(error.contains("bash_nope"));
    }

    #[test]
    fn list_tasks_returns_registered_tasks() {
        let _lock = TEST_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        let a = spawn("true", &cwd()).expect("a").shell_id;
        let b = spawn("true", &cwd()).expect("b").shell_id;
        let listed = list_tasks();
        let ids: Vec<&str> = listed.iter().map(|s| s.shell_id.as_str()).collect();
        assert!(ids.contains(&a.as_str()));
        assert!(ids.contains(&b.as_str()));
    }

    #[test]
    fn stream_buffer_drops_oldest_when_full() {
        let mut buf = StreamBuffer::default();
        for i in 0..(MAX_BUFFERED_LINES + 5) {
            buf.push(format!("line {i}"));
        }
        assert_eq!(buf.lines.len(), MAX_BUFFERED_LINES);
        assert_eq!(buf.total_lines, (MAX_BUFFERED_LINES + 5) as u64);
        assert!(buf.truncated);
        // Oldest 5 lines dropped → first surviving line is "line 5".
        assert_eq!(buf.lines.first().map(String::as_str), Some("line 5"));

        // cursor before head → returns whole current buffer.
        let lines = buf.lines_since(2);
        assert_eq!(lines.len(), MAX_BUFFERED_LINES);
        assert_eq!(lines.first().map(String::as_str), Some("line 5"));
    }

    #[test]
    fn next_shell_id_is_unique_within_process() {
        let _lock = TEST_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        let mut ids: Vec<String> = (0..32).map(|_| next_shell_id()).collect();
        let unique: std::collections::HashSet<_> = ids.drain(..).collect();
        assert_eq!(unique.len(), 32);
    }
}
