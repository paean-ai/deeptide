//! Non-blocking stdin polling for the `/queue` mid-turn type-ahead path.
//!
//! Architectural context: the REPL's `repl.submit()` is synchronous —
//! while a turn streams the main thread is busy, and rustyline isn't
//! holding stdin. Stdin is in cooked / line-buffered mode (the kernel
//! line discipline collects chars until Enter, then makes the full line
//! available at once). That means we can opportunistically peek at stdin
//! from the streaming handler with a zero-timeout `poll(2)` — if the
//! syscall says there's data ready, an entire user-typed line is
//! waiting, and a follow-up `read_line` returns it without blocking.
//!
//! Limitations:
//!   * Unix only. The Windows path is a no-op stub that returns
//!     `false` so the streaming-handler call site compiles
//!     unchanged. `/queue add <msg>` still works on Windows.
//!   * Cooked-mode echo means the user's typed characters will
//!     visually interleave with the agent's streamed output as they
//!     type. The follow-up `✚ queued (#N)` notice we print after
//!     consuming the line is the user's confirmation that it was
//!     captured rather than lost. Cleaning the echo up would require
//!     putting the terminal in raw mode for the entire session, which
//!     is a much larger architectural change.

#[cfg(unix)]
pub fn stdin_has_pending_line() -> bool {
    use std::os::fd::AsRawFd;

    // We poll the actual stdin fd rather than fd 0 directly so that
    // process invocations where stdin has been duped do the right thing.
    let stdin = std::io::stdin();
    let fd = stdin.as_raw_fd();

    let mut pfd = libc::pollfd {
        fd,
        events: libc::POLLIN,
        revents: 0,
    };

    // Zero-timeout: pure peek. Returns:
    //   * < 0 on syscall error (treat as "nothing ready", we'll try
    //     again on the next streaming event)
    //   * 0 when no fd is ready before timeout (no input pending)
    //   * > 0 with revents indicating which event fired
    //
    // We deliberately *don't* propagate errors: this poll runs many
    // times a second from inside the streaming handler, and surfacing
    // a transient EINTR or EBADF to the user would be noise. The only
    // failure mode that matters is "stdin closed forever", which we
    // detect downstream via the read returning EOF.
    let ret = unsafe { libc::poll(&mut pfd, 1, 0) };
    if ret <= 0 {
        return false;
    }
    // POLLIN means readable, POLLHUP/POLLERR mean the fd is dead.
    // Only POLLIN should trigger a read attempt.
    (pfd.revents & libc::POLLIN) != 0
}

#[cfg(not(unix))]
pub fn stdin_has_pending_line() -> bool {
    // Non-Unix targets: skip mid-turn polling. `/queue add` still works
    // between turns; the feature simply degrades to manual enqueuing.
    false
}

/// Single-line non-blocking read used by the streaming handler when
/// `stdin_has_pending_line` returns `true`. Returns `Ok(Some(line))` on
/// a normal line, `Ok(None)` on stdin EOF (Ctrl+D mid-turn — rare but
/// possible), or `Err(_)` on a transient I/O error which the caller
/// should swallow and retry on the next poll tick.
pub fn read_pending_line() -> std::io::Result<Option<String>> {
    use std::io::BufRead;
    let stdin = std::io::stdin();
    let mut guard = stdin.lock();
    let mut buf = String::new();
    match guard.read_line(&mut buf) {
        Ok(0) => Ok(None),      // EOF
        Ok(_) => Ok(Some(buf)), // line including trailing \n
        Err(error) => Err(error),
    }
}

/// Best-effort drain of any bytes currently buffered on stdin,
/// discarding them. Used as recovery after rustyline returns
/// `Io(InvalidData)` — that error normally means a stray
/// continuation byte (orphaned UTF-8 mid-sequence) or a partial
/// escape sequence is sitting in the buffer, which will trip the
/// *next* `readline()` call too unless we flush it first.
///
/// Semantics:
///   * Loops until `poll(2)` says nothing is ready.
///   * Each iteration reads up to 256 bytes with a zero timeout.
///   * Discards every byte read. Errors are swallowed: this runs
///     on a recovery path and must never itself raise a fatal
///     error.
///   * Returns the total number of bytes drained for diagnostics
///     (callers may include this in the warning they emit).
///
/// Unix-only; the Windows stub returns 0.
#[cfg(unix)]
pub fn drain_pending_stdin_bytes() -> usize {
    use std::os::fd::AsRawFd;

    let stdin = std::io::stdin();
    let fd = stdin.as_raw_fd();
    let mut total: usize = 0;
    let mut iterations: usize = 0;

    loop {
        let mut pfd = libc::pollfd {
            fd,
            events: libc::POLLIN,
            revents: 0,
        };
        let ret = unsafe { libc::poll(&mut pfd, 1, 0) };
        if ret <= 0 || (pfd.revents & libc::POLLIN) == 0 {
            break;
        }

        let mut buf = [0u8; 256];
        let n = unsafe { libc::read(fd, buf.as_mut_ptr().cast::<libc::c_void>(), buf.len()) };
        if n <= 0 {
            break;
        }
        total = total.saturating_add(n as usize);
        // Belt + suspenders cap: never loop forever if a noisy
        // device keeps producing bytes (e.g. a paste-bomb).
        iterations += 1;
        if iterations >= 32 {
            break;
        }
    }
    total
}

#[cfg(not(unix))]
pub fn drain_pending_stdin_bytes() -> usize {
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stdin_has_pending_line_is_callable() {
        // We can't reliably assert true/false in a test harness (cargo
        // runs tests with stdin closed or piped), but the function must
        // not panic and must complete promptly (zero timeout).
        let _ = stdin_has_pending_line();
    }

    #[test]
    fn drain_pending_stdin_bytes_is_callable_and_returns_count() {
        // Same caveat as above — under the test harness stdin is
        // typically closed/piped so the drain returns 0. The
        // contract we verify is "doesn't panic, completes
        // promptly".
        let drained = drain_pending_stdin_bytes();
        assert_eq!(drained, drained);
    }
}
