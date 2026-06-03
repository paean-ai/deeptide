use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::agent_loop::{ConversationMessage, MessageRole};
use crate::memory::MemorySystem;

// ── Session ID ────────────────────────────────────────────────────────────────

/// Process-global counter mixed into session IDs to guarantee uniqueness even
/// when multiple IDs are generated within the same clock tick.
static SESSION_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Generate a unique session ID: `YYYY-MM-DDTHH-MM-SS-microseconds-XXXXXX`.
///
/// The format mirrors the Swift reference (`newSessionID()` in SessionStore.swift):
/// an ISO8601-derived timestamp with colons/dots replaced by dashes so it is a
/// valid filename on all platforms, followed by a 6-char base-36 suffix.  The
/// suffix is derived by hashing the current nanosecond counter with a
/// monotonically increasing process-local counter, so IDs are unique even when
/// generated in tight loops on a low-resolution clock.
pub fn new_session_id() -> String {
    let now = OffsetDateTime::now_utc();
    let count = SESSION_COUNTER.fetch_add(1, Ordering::Relaxed);
    let ns = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as u64)
        .unwrap_or(0);
    // Mix counter and nanos with a Knuth multiplicative hash to spread bits.
    let mixed = ns.wrapping_add(count.wrapping_mul(2_654_435_761));
    let suffix = base36((mixed % 2_176_782_336) as u32); // 36^6 = 2,176,782,336
    format!(
        "{:04}-{:02}-{:02}T{:02}-{:02}-{:02}-{:06}-{suffix}",
        now.year(),
        now.month() as u8,
        now.day(),
        now.hour(),
        now.minute(),
        now.second(),
        now.microsecond()
    )
}

fn base36(mut n: u32) -> String {
    const DIGITS: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    let mut buf = [0u8; 6];
    for slot in buf.iter_mut().rev() {
        *slot = DIGITS[(n % 36) as usize];
        n /= 36;
    }
    std::str::from_utf8(&buf).unwrap_or("000000").to_owned()
}

// ── Session types ─────────────────────────────────────────────────────────────

/// Compact index record stored in the session-locations index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionEntry {
    pub session_id: String,
    pub cwd: String,
    pub model: String,
    pub started_at: String,
    pub updated_at: String,
    pub preview: String,
    pub message_count: usize,
}

impl SessionEntry {
    pub fn short_id(&self) -> &str {
        // Last 12 chars are the microseconds + suffix, which is unique enough
        // for display. Show first 24 chars (date + time) + "..." if too long.
        &self.session_id
    }
}

/// A line in a persisted session JSONL file.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum SessionLine {
    Meta {
        session_id: String,
        cwd: String,
        model: String,
        started_at: String,
    },
    Message {
        role: String,
        content: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        timestamp: Option<String>,
    },
}

// ── SessionStore ──────────────────────────────────────────────────────────────

pub struct SessionStore;

impl SessionStore {
    /// Project-scoped session directory: `~/.config/tide/projects/<slug>/sessions/`.
    pub fn session_dir(cwd: &Path) -> PathBuf {
        MemorySystem::tide_config_dir()
            .join("projects")
            .join(MemorySystem::project_slug(cwd))
            .join("sessions")
    }

    /// Path to the JSONL transcript file for a session.
    pub fn session_path(cwd: &Path, session_id: &str) -> PathBuf {
        Self::session_dir(cwd).join(format!("{session_id}.jsonl"))
    }

    /// Path to the global cross-project session index.
    fn index_path() -> PathBuf {
        MemorySystem::tide_config_dir().join("session-locations.json")
    }

    /// Persist the full conversation transcript for the given session.
    ///
    /// Writes atomically via a `.tmp` sibling then rename.  Errors are silenced
    /// because session persistence must never crash the REPL.
    pub fn save(
        cwd: &Path,
        session_id: &str,
        model: &str,
        started_at: &str,
        messages: &[ConversationMessage],
    ) {
        let dir = Self::session_dir(cwd);
        if std::fs::create_dir_all(&dir).is_err() {
            return;
        }

        let path = dir.join(format!("{session_id}.jsonl"));
        let tmp = path.with_extension("jsonl.tmp");

        let mut lines: Vec<String> = Vec::with_capacity(messages.len() + 1);

        let meta = SessionLine::Meta {
            session_id: session_id.to_owned(),
            cwd: cwd.display().to_string(),
            model: model.to_owned(),
            started_at: started_at.to_owned(),
        };
        if let Ok(s) = serde_json::to_string(&meta) {
            lines.push(s);
        }

        let ts = OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .unwrap_or_default();
        for msg in messages {
            // Skip tool-result-only messages (empty content with tool_results)
            if msg.content.is_empty() && msg.tool_calls.is_empty() {
                continue;
            }
            let role = match msg.role {
                MessageRole::User => "user",
                MessageRole::Assistant => "assistant",
            };
            let line = SessionLine::Message {
                role: role.to_owned(),
                content: msg.content.clone(),
                timestamp: Some(ts.clone()),
            };
            if let Ok(s) = serde_json::to_string(&line) {
                lines.push(s);
            }
        }

        let content = lines.join("\n") + "\n";
        if std::fs::write(&tmp, content).is_ok() {
            let _ = std::fs::rename(&tmp, &path);
        }

        Self::upsert_index(cwd, session_id, model, started_at, messages);
    }

    fn upsert_index(
        cwd: &Path,
        session_id: &str,
        model: &str,
        started_at: &str,
        messages: &[ConversationMessage],
    ) {
        let index_path = Self::index_path();
        let mut index: BTreeMap<String, serde_json::Value> = index_path
            .exists()
            .then(|| std::fs::read_to_string(&index_path).ok())
            .flatten()
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .unwrap_or_default();

        let preview = messages
            .iter()
            .find(|m| m.role == MessageRole::User && !m.content.is_empty())
            .map(|m| truncate_chars(&m.content, 60))
            .unwrap_or_default();

        let now = OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .unwrap_or_default();

        let message_count = messages.iter().filter(|m| !m.content.is_empty()).count();

        index.insert(
            session_id.to_owned(),
            serde_json::json!({
                "session_id": session_id,
                "cwd": cwd.display().to_string(),
                "model": model,
                "started_at": started_at,
                "updated_at": now,
                "preview": preview,
                "message_count": message_count,
            }),
        );

        if let Some(parent) = index_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(serialized) = serde_json::to_string_pretty(&index) {
            let _ = std::fs::write(&index_path, serialized);
        }
    }

    /// List saved sessions for a project directory, newest first.
    ///
    /// Scans the project session directory rather than relying on the global
    /// index so that the result is always consistent with what is on disk and
    /// there are no concurrent-write race conditions.
    pub fn list(cwd: &Path) -> Vec<SessionEntry> {
        let dir = Self::session_dir(cwd);
        if !dir.exists() {
            return Vec::new();
        }

        let Ok(read_dir) = std::fs::read_dir(&dir) else {
            return Vec::new();
        };

        let mut entries: Vec<SessionEntry> = read_dir
            .flatten()
            .filter_map(|de| {
                let path = de.path();
                if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
                    return None;
                }
                read_session_meta(&path)
            })
            .collect();

        entries.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        entries
    }

    /// Load the conversation messages from a session JSONL file.
    ///
    /// Looks up the session in the index to find its exact file path, then reads
    /// the JSONL and reconstructs the `ConversationMessage` list.  Returns an
    /// error string on any failure.
    pub fn load(cwd: &Path, session_id: &str) -> Result<Vec<ConversationMessage>, String> {
        let path = locate_session_file(cwd, session_id)?;

        let raw =
            std::fs::read_to_string(&path).map_err(|e| format!("Cannot read session file: {e}"))?;

        let mut messages = Vec::new();
        for line in raw.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let obj: serde_json::Value =
                serde_json::from_str(line).map_err(|e| format!("Malformed session line: {e}"))?;

            let ty = obj.get("type").and_then(|v| v.as_str()).unwrap_or("");
            if ty != "message" {
                continue;
            }

            let role = match obj.get("role").and_then(|v| v.as_str()) {
                Some("user") => MessageRole::User,
                Some("assistant") => MessageRole::Assistant,
                _ => continue,
            };
            let content = obj
                .get("content")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_owned();

            messages.push(ConversationMessage {
                role,
                content,
                tool_calls: Vec::new(),
                tool_results: Vec::new(),
            });
        }

        Ok(messages)
    }
}

/// Read a minimal `SessionEntry` from the first (meta) line of a JSONL session file.
/// Returns `None` if the file cannot be read or doesn't have a valid meta header.
fn read_session_meta(path: &std::path::Path) -> Option<SessionEntry> {
    use std::io::{BufRead, BufReader};

    let file = std::fs::File::open(path).ok()?;
    let mut reader = BufReader::new(file);
    let mut first_line = String::new();
    reader.read_line(&mut first_line).ok()?;

    let meta: serde_json::Value = serde_json::from_str(first_line.trim()).ok()?;
    if meta.get("type").and_then(|v| v.as_str()) != Some("meta") {
        return None;
    }

    let session_id = meta.get("session_id")?.as_str()?.to_owned();
    let cwd = meta.get("cwd")?.as_str()?.to_owned();
    let model = meta
        .get("model")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_owned();
    let started_at = meta
        .get("started_at")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_owned();

    // Get mtime as the updated_at proxy
    let updated_at = path
        .metadata()
        .and_then(|m| m.modified())
        .map(|t| {
            let secs = t
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            // Format as a sortable string
            let odt = OffsetDateTime::from_unix_timestamp(secs as i64)
                .unwrap_or(OffsetDateTime::UNIX_EPOCH);
            odt.format(&Rfc3339).unwrap_or_default()
        })
        .unwrap_or_default();

    // Count non-meta, non-empty lines as the message count and grab preview
    let mut count = 0usize;
    let mut preview = String::new();
    for line in reader.lines().map_while(Result::ok) {
        let line = line.trim().to_owned();
        if line.is_empty() {
            continue;
        }
        if let Ok(obj) = serde_json::from_str::<serde_json::Value>(&line) {
            let ty = obj.get("type").and_then(|v| v.as_str()).unwrap_or("");
            let role = obj.get("role").and_then(|v| v.as_str()).unwrap_or("");
            if ty == "message" {
                count += 1;
                if preview.is_empty() && role == "user" {
                    preview = obj
                        .get("content")
                        .and_then(|v| v.as_str())
                        .map(|c| truncate_chars(c, 60))
                        .unwrap_or_default();
                }
            }
        }
    }

    Some(SessionEntry {
        session_id,
        cwd,
        model,
        started_at,
        updated_at,
        preview,
        message_count: count,
    })
}

/// Locate the JSONL file for a session ID.  First tries the canonical
/// project-scoped path; falls back to the global index for cross-project lookup.
fn locate_session_file(cwd: &Path, session_id: &str) -> Result<PathBuf, String> {
    let canonical = SessionStore::session_path(cwd, session_id);
    if canonical.exists() {
        return Ok(canonical);
    }

    // Try global index
    let index_path = SessionStore::index_path();
    let raw = std::fs::read_to_string(&index_path)
        .map_err(|_| format!("Session not found: {session_id}"))?;
    let index: BTreeMap<String, serde_json::Value> =
        serde_json::from_str(&raw).map_err(|_| format!("Session not found: {session_id}"))?;

    let entry = index
        .get(session_id)
        .ok_or_else(|| format!("Session not found: {session_id}"))?;

    let stored_cwd = entry
        .get("cwd")
        .and_then(|v| v.as_str())
        .ok_or_else(|| format!("Malformed index entry for {session_id}"))?;

    let stored_path = SessionStore::session_path(Path::new(stored_cwd), session_id);
    if stored_path.exists() {
        Ok(stored_path)
    } else {
        Err(format!("Session file missing: {session_id}"))
    }
}

fn truncate_chars(s: &str, max: usize) -> String {
    let s = s.replace('\n', " ");
    // Session/checkpoint preview lines: budget by display width so a CJK
    // preview is cut on the right column boundary (the ellipsis differs from
    // the previous `...`, but the listings are display-only).
    crate::width::truncate_to_width(&s, max)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_id_has_expected_shape() {
        let id = new_session_id();
        // YYYY-MM-DDTHH-MM-SS-microseconds-XXXXXX  (24 + 1 + 6 + 1 + 6 = 38 chars minimum)
        assert!(id.len() >= 30, "session ID too short: {id}");
        assert!(!id.contains(':'), "colons must be replaced with dashes");
        assert!(!id.contains('.'), "dots must not appear in session ID");
        // All chars should be alphanumeric, dash, or T
        assert!(
            id.chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == 'T'),
            "unexpected chars in session ID: {id}"
        );
    }

    #[test]
    fn session_ids_are_unique() {
        let ids: Vec<_> = (0..10).map(|_| new_session_id()).collect();
        let unique: std::collections::HashSet<_> = ids.iter().collect();
        assert_eq!(ids.len(), unique.len(), "session IDs must be unique");
    }

    #[test]
    fn session_store_save_and_load_round_trip() {
        let temp = tempfile::tempdir().expect("tempdir");
        let cwd = temp.path();
        let id = new_session_id();
        let started = "2024-01-01T00:00:00Z";

        let messages = vec![
            ConversationMessage::user("hello"),
            ConversationMessage::assistant("hi there"),
            ConversationMessage::user("what is 2+2?"),
            ConversationMessage::assistant("4"),
        ];

        SessionStore::save(cwd, &id, "deepseek-v4-flash", started, &messages);

        let loaded = SessionStore::load(cwd, &id).expect("should load");
        assert_eq!(loaded.len(), 4);
        assert_eq!(loaded[0].content, "hello");
        assert_eq!(loaded[1].content, "hi there");
        assert_eq!(loaded[2].content, "what is 2+2?");
        assert_eq!(loaded[3].content, "4");
        assert!(matches!(loaded[0].role, MessageRole::User));
        assert!(matches!(loaded[1].role, MessageRole::Assistant));
    }

    #[test]
    fn session_store_list_returns_project_sessions() {
        let temp = tempfile::tempdir().expect("tempdir");
        let cwd = temp.path();
        let id1 = new_session_id();
        let id2 = new_session_id();

        let msgs = vec![ConversationMessage::user("test")];
        SessionStore::save(cwd, &id1, "model-a", "2024-01-01T00:00:00Z", &msgs);
        SessionStore::save(cwd, &id2, "model-b", "2024-01-01T00:01:00Z", &msgs);

        let entries = SessionStore::list(cwd);
        assert_eq!(entries.len(), 2);
        // Both sessions should belong to this project
        let cwds: Vec<_> = entries.iter().map(|e| e.cwd.as_str()).collect();
        for c in cwds {
            assert_eq!(c, cwd.display().to_string());
        }
    }

    #[test]
    fn session_store_load_missing_returns_error() {
        let temp = tempfile::tempdir().expect("tempdir");
        let result = SessionStore::load(temp.path(), "nonexistent-session-id");
        assert!(result.is_err());
        let err = result.expect_err("should be an error");
        assert!(err.contains("not found"), "error: {err}");
    }

    #[test]
    fn session_store_preview_uses_first_user_message() {
        let temp = tempfile::tempdir().expect("tempdir");
        let cwd = temp.path();
        let id = new_session_id();

        let msgs = vec![
            ConversationMessage::user("write me a function that adds two numbers"),
            ConversationMessage::assistant(
                "Sure, here it is: fn add(a: i32, b: i32) -> i32 { a + b }",
            ),
        ];
        SessionStore::save(cwd, &id, "model", "2024-01-01T00:00:00Z", &msgs);

        let entries = SessionStore::list(cwd);
        assert_eq!(entries.len(), 1);
        assert!(
            entries[0].preview.contains("write me a function"),
            "preview: {}",
            entries[0].preview
        );
    }
}
