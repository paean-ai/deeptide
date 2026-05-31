//! Interactive fuzzy selection picker for a [`ReplMenu`], à la Claude Code's
//! `/`-command pickers: type to filter, ↑/↓ (or Ctrl-N/Ctrl-P) to move, Enter
//! to select, Esc / Ctrl-C to cancel.
//!
//! Built on the same poll-based raw-mode primitive as the queue editor
//! ([`crate::queue_editor::enter_raw_mode`]). Unix-only; on other platforms (or
//! a non-TTY stdin) [`run`] returns [`PickResult::Unsupported`] so the caller
//! falls back to the numbered-text menu.

use std::io::{self, Write};

use deeptide_core::ReplMenu;

use crate::queue_editor::enter_raw_mode;

/// Outcome of presenting the picker.
pub enum PickResult {
    /// The user chose a row; carries its `action` line to submit.
    Selected(String),
    /// The user dismissed the picker (Esc / Ctrl-C).
    Cancelled,
    /// Raw mode was unavailable (non-TTY / non-Unix); caller should fall back.
    Unsupported,
}

/// Max rows shown at once; the view scrolls to keep the selection visible.
const MAX_VISIBLE: usize = 12;

/// Case-insensitive subsequence match: every char of `query` appears in `text`
/// in order. Empty query matches everything.
fn fuzzy_matches(query: &str, text: &str) -> bool {
    if query.is_empty() {
        return true;
    }
    let mut q = query.chars().map(|c| c.to_ascii_lowercase()).peekable();
    for ch in text.chars().map(|c| c.to_ascii_lowercase()) {
        if q.peek() == Some(&ch) {
            q.next();
        }
    }
    q.peek().is_none()
}

#[cfg(unix)]
pub fn run(menu: &ReplMenu, color: bool) -> PickResult {
    let Some(_guard) = enter_raw_mode() else {
        return PickResult::Unsupported;
    };
    // Also disable ISIG so Ctrl-C reaches us as a byte (clean cancel) instead of
    // killing the REPL. `_guard` still restores the true original termios.
    disable_signals();

    let mut query = String::new();
    let mut selected: usize = 0; // index into the filtered list
    let mut scroll: usize = 0; // first visible filtered row
    let mut prev_lines: usize = 0;

    let result = loop {
        let filtered: Vec<usize> = (0..menu.choices.len())
            .filter(|&i| fuzzy_matches(&query, &menu.choices[i].label))
            .collect();
        if selected >= filtered.len() {
            selected = filtered.len().saturating_sub(1);
        }
        if selected < scroll {
            scroll = selected;
        } else if selected >= scroll + MAX_VISIBLE {
            scroll = selected + 1 - MAX_VISIBLE;
        }

        prev_lines = render(menu, &query, &filtered, selected, scroll, color, prev_lines);

        match read_key() {
            Key::Enter => {
                break match filtered.get(selected) {
                    Some(&i) => PickResult::Selected(menu.choices[i].action.clone()),
                    None => PickResult::Cancelled,
                };
            }
            Key::Cancel => break PickResult::Cancelled,
            Key::Up => selected = selected.saturating_sub(1),
            Key::Down => {
                if selected + 1 < filtered.len() {
                    selected += 1;
                }
            }
            Key::Backspace => {
                query.pop();
                selected = 0;
                scroll = 0;
            }
            Key::Char(c) => {
                query.push(c);
                selected = 0;
                scroll = 0;
            }
            Key::Ignore => {}
            Key::Eof => break PickResult::Cancelled,
        }
    };

    clear_lines(prev_lines);
    result
}

#[cfg(not(unix))]
pub fn run(_menu: &ReplMenu, _color: bool) -> PickResult {
    PickResult::Unsupported
}

/// Draw the picker and return the number of lines it occupies (so the next
/// redraw / final clear can rewind exactly that many).
fn render(
    menu: &ReplMenu,
    query: &str,
    filtered: &[usize],
    selected: usize,
    scroll: usize,
    color: bool,
    prev_lines: usize,
) -> usize {
    let mut out = io::stdout();
    // Rewind over the previous frame and clear downward.
    if prev_lines > 0 {
        let _ = write!(out, "\r\x1b[{prev_lines}A\x1b[J");
    } else {
        let _ = write!(out, "\r\x1b[J");
    }

    let (dim, bold, sel_on, reset) = if color {
        ("\x1b[2m", "\x1b[1m", "\x1b[7m", "\x1b[0m")
    } else {
        ("", "", "", "")
    };

    let mut lines = 0usize;
    let _ = write!(out, "{bold}{}{reset}  {dim}{} match{}{reset}\r\n",
        menu.title,
        filtered.len(),
        if filtered.len() == 1 { "" } else { "es" });
    lines += 1;
    let _ = write!(out, "{dim}search:{reset} {query}\r\n");
    lines += 1;

    let end = (scroll + MAX_VISIBLE).min(filtered.len());
    for (row, &i) in filtered[scroll..end].iter().enumerate() {
        let abs = scroll + row;
        let label = &menu.choices[i].label;
        if abs == selected {
            let _ = write!(out, "{sel_on}› {label}{reset}\r\n");
        } else {
            let _ = write!(out, "  {label}\r\n");
        }
        lines += 1;
    }
    if filtered.is_empty() {
        let _ = write!(out, "{dim}  (no matches){reset}\r\n");
        lines += 1;
    }
    if !menu.footer.is_empty() {
        let _ = write!(out, "{dim}{}{reset}\r\n", menu.footer);
        lines += 1;
    }
    let _ = write!(out, "{dim}↑/↓ move · type to filter · Enter select · Esc cancel{reset}");
    lines += 1; // the hint line (no trailing newline)
    let _ = out.flush();
    lines
}

/// Clear `n` rendered lines, leaving the cursor at the start of the block.
fn clear_lines(n: usize) {
    let mut out = io::stdout();
    if n > 0 {
        // Cursor is on the last (hint) line; rewind to the top and clear down.
        let _ = write!(out, "\r\x1b[{}A\x1b[J", n.saturating_sub(1));
    } else {
        let _ = write!(out, "\r\x1b[J");
    }
    let _ = out.flush();
}

enum Key {
    Up,
    Down,
    Enter,
    Backspace,
    Cancel,
    Char(char),
    Ignore,
    Eof,
}

#[cfg(unix)]
fn read_key() -> Key {
    match read_byte_blocking() {
        None => Key::Eof,
        Some(0x0d) | Some(0x0a) => Key::Enter,
        Some(0x7f) | Some(0x08) => Key::Backspace,
        Some(0x03) | Some(0x07) => Key::Cancel, // Ctrl-C / Ctrl-G
        Some(0x0e) => Key::Down,                // Ctrl-N
        Some(0x10) => Key::Up,                  // Ctrl-P
        Some(0x1b) => {
            // Esc, or the start of an arrow sequence (ESC [ A/B). Peek with a
            // short timeout to disambiguate a bare Esc.
            match read_byte_timeout(60) {
                Some(b'[') | Some(b'O') => match read_byte_timeout(60) {
                    Some(b'A') => Key::Up,
                    Some(b'B') => Key::Down,
                    _ => Key::Ignore,
                },
                _ => Key::Cancel,
            }
        }
        Some(b) if (0x20..=0x7e).contains(&b) => Key::Char(b as char),
        Some(_) => Key::Ignore,
    }
}

#[cfg(unix)]
fn disable_signals() {
    use std::os::fd::AsRawFd;
    let fd = io::stdin().as_raw_fd();
    let mut t: libc::termios = unsafe { std::mem::zeroed() };
    if unsafe { libc::tcgetattr(fd, &mut t) } == 0 {
        t.c_lflag &= !libc::ISIG;
        unsafe {
            libc::tcsetattr(fd, libc::TCSANOW, &t);
        }
    }
}

/// Block (via poll) until one byte is available; `None` on EOF/error.
#[cfg(unix)]
fn read_byte_blocking() -> Option<u8> {
    read_with_timeout(-1)
}

/// Read one byte if it arrives within `ms` milliseconds, else `None`.
#[cfg(unix)]
fn read_byte_timeout(ms: i32) -> Option<u8> {
    read_with_timeout(ms)
}

#[cfg(unix)]
fn read_with_timeout(timeout_ms: i32) -> Option<u8> {
    let mut pfd = libc::pollfd {
        fd: 0,
        events: libc::POLLIN,
        revents: 0,
    };
    loop {
        let r = unsafe { libc::poll(&mut pfd, 1, timeout_ms) };
        if r < 0 {
            if std::io::Error::last_os_error().raw_os_error() == Some(libc::EINTR) {
                continue;
            }
            return None;
        }
        if r == 0 {
            return None; // timed out
        }
        if pfd.revents & libc::POLLIN != 0 {
            let mut byte = [0u8; 1];
            let n = unsafe { libc::read(0, byte.as_mut_ptr() as *mut libc::c_void, 1) };
            if n == 1 {
                return Some(byte[0]);
            }
            if n == 0 {
                return None;
            }
            // n < 0: interrupted/again — retry.
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fuzzy_subsequence_and_case_insensitive() {
        assert!(fuzzy_matches("", "anything"));
        assert!(fuzzy_matches("cld", "[claude] 4604 — continue"));
        assert!(fuzzy_matches("CLAUDE", "[claude] x"));
        assert!(!fuzzy_matches("zzz", "[claude] x"));
        // out-of-order chars don't match a subsequence
        assert!(!fuzzy_matches("dcx", "[claude]"));
    }
}
