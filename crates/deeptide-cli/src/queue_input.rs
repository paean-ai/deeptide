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
}
