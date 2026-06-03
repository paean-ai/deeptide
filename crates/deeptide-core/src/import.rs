//! Cross-tool session import: read another agent's local session transcript
//! (Claude Code, Codex) — or deeptide's own — and bring its context into
//! deeptide without starting from scratch.
//!
//! Two consumption modes are served from one normalized transcript:
//!
//!   * **Seed** ([`flatten_for_extraction`]) — flatten the transcript to plain
//!     text, feed it through the dream/capture prompt, and let the model write
//!     durable facts via `MemoryWrite`. Lossy by design; the right fit for a
//!     first-install "stand on prior work" bootstrap.
//!   * **Handoff** ([`build_handoff_block`]) — fold the older turns into a short
//!     note and keep the recent tail near-verbatim, framed as *prior context
//!     from another assistant*. The right fit for a mid-task failover (e.g. a
//!     Claude rate-limit) where the working state matters.
//!
//! Every parser is **defensive**: it reads only the fields it needs and ignores
//! everything else, so an upstream schema change degrades to "fewer turns
//! recovered", never a hard failure. All the pure logic (parsing + the two
//! builders) is deterministic and unit-tested against fixtures — no network.

use std::cmp::Reverse;
use std::path::{Path, PathBuf};

use crate::agent_loop::{ConversationMessage, MessageRole};
use crate::memory::MemorySystem;

/// Cap on how much text we keep from a single tool result / tool-use payload.
/// Tool I/O is bulky and the working tree already holds the real artefacts;
/// we keep enough to orient, not the whole dump.
const TOOL_TEXT_CAP: usize = 400;

/// Which external agent produced a session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceTool {
    Claude,
    Codex,
    Deeptide,
}

impl SourceTool {
    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "claude" | "claude-code" | "cc" => Some(Self::Claude),
            "codex" => Some(Self::Codex),
            "deeptide" | "tide" | "zero" | "self" => Some(Self::Deeptide),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
            Self::Deeptide => "deeptide",
        }
    }
}

/// Normalized role. `System` covers developer/system prompts and assistant
/// "thinking" — kept distinct so callers can drop it from a clean handoff.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportedRole {
    User,
    Assistant,
    System,
}

/// What a turn carries, so consumers can keep human/assistant prose while
/// down-weighting or dropping bulky tool I/O.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurnKind {
    /// Human or assistant natural-language message.
    Message,
    /// An assistant tool invocation (rendered as `name(args-summary)`).
    ToolUse,
    /// A tool result fed back to the model.
    ToolResult,
    /// Assistant chain-of-thought / reasoning.
    Thinking,
}

/// One normalized turn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportedTurn {
    pub role: ImportedRole,
    pub kind: TurnKind,
    pub text: String,
}

impl ImportedTurn {
    fn message(role: ImportedRole, text: String) -> Self {
        Self {
            role,
            kind: TurnKind::Message,
            text,
        }
    }
}

/// A parsed, tool-agnostic transcript.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportedTranscript {
    pub source: SourceTool,
    pub session_id: String,
    pub cwd: Option<String>,
    pub git_branch: Option<String>,
    pub title: Option<String>,
    pub turns: Vec<ImportedTurn>,
}

impl ImportedTranscript {
    /// Count of human/assistant prose turns (excludes tool I/O and thinking),
    /// i.e. the "conversational" depth a user cares about.
    pub fn message_turns(&self) -> usize {
        self.turns
            .iter()
            .filter(|t| t.kind == TurnKind::Message && t.role != ImportedRole::System)
            .count()
    }
}

/// A discovered session on disk, before it's parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionRef {
    pub source: SourceTool,
    pub session_id: String,
    pub path: PathBuf,
    /// Epoch seconds of last modification, for newest-first ordering.
    pub mtime_secs: u64,
}

// ── Discovery ───────────────────────────────────────────────────────────────

/// Slug forms a `cwd` can appear under across tools. Claude/deeptide both map
/// path separators to `-`; Claude keeps a leading `-`, deeptide strips it, so
/// we accept both.
pub fn slug_variants(cwd: &Path) -> Vec<String> {
    let normalized = cwd.to_string_lossy().replace('\\', "/");
    let dashed = normalized.replace('/', "-"); // Claude form: leading '-' kept
    let stripped = dashed.trim_start_matches('-').to_owned(); // deeptide form
    let mut out = vec![dashed];
    if !out.contains(&stripped) {
        out.push(stripped);
    }
    out
}

/// Resolve `selector` (a session-id prefix, or empty/`--latest`/`latest`) to a
/// concrete session of `source` for `cwd`. Shared by the REPL command and the
/// CLI one-shot path so both pick sessions identically.
pub fn resolve_ref(cwd: &Path, source: SourceTool, selector: &str) -> Result<SessionRef, String> {
    let mut refs: Vec<_> = discover(cwd)
        .into_iter()
        .filter(|r| r.source == source)
        .collect();
    if refs.is_empty() {
        return Err(format!(
            "No {} sessions found for this project.",
            source.label()
        ));
    }
    let sel = selector.trim();
    if sel.is_empty() || sel.eq_ignore_ascii_case("--latest") || sel.eq_ignore_ascii_case("latest")
    {
        return Ok(refs.remove(0)); // discover() is newest-first
    }
    refs.into_iter()
        .find(|r| r.session_id == sel || r.session_id.starts_with(sel))
        .ok_or_else(|| format!("No {} session matching `{sel}`.", source.label()))
}

/// Discover importable sessions from every known tool for `cwd`, newest first.
pub fn discover(cwd: &Path) -> Vec<SessionRef> {
    let mut refs = Vec::new();
    // Claude and deeptide key sessions by a cwd-slug directory, so each reads a
    // single flat dir — no walk to bound. Only Codex shards sessions across a
    // date tree that must be walked, so only `discover_codex` carries depth /
    // entry / symlink guards.
    refs.extend(discover_claude(cwd));
    refs.extend(discover_codex(cwd));
    refs.extend(discover_deeptide(cwd));
    refs.sort_by_key(|r| Reverse(r.mtime_secs));
    refs
}

fn home() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

fn mtime_secs(path: &Path) -> u64 {
    path.metadata()
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn discover_claude(cwd: &Path) -> Vec<SessionRef> {
    let Some(home) = home() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for slug in slug_variants(cwd) {
        let dir = home.join(".claude").join("projects").join(&slug);
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
                continue;
            }
            let id = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or_default()
                .to_owned();
            out.push(SessionRef {
                source: SourceTool::Claude,
                session_id: id,
                mtime_secs: mtime_secs(&path),
                path,
            });
        }
    }
    out
}

fn discover_codex(cwd: &Path) -> Vec<SessionRef> {
    // Codex shards by date (sessions/YYYY/MM/DD/rollout-*.jsonl) and records
    // cwd inside the file, not the path — so we must read each meta line to
    // filter by project. Bounded by walking only the sessions tree.
    let Some(home) = home() else {
        return Vec::new();
    };
    let root = home.join(".codex").join("sessions");
    let target = cwd.to_string_lossy().to_string();
    let mut out = Vec::new();
    // Bound the walk: cap depth and total entries, and never follow symlinks, so
    // a symlink cycle under ~/.codex/sessions can't spin forever and a huge tree
    // can't stall startup (this runs on the first-run onboarding path too).
    const MAX_DEPTH: usize = 8;
    const MAX_ENTRIES: usize = 20_000;
    let mut scanned = 0usize;
    let mut capped = false;
    let mut stack = vec![(root, 0usize)];
    while let Some((dir, depth)) = stack.pop() {
        if scanned >= MAX_ENTRIES {
            capped = true;
            break;
        }
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            scanned += 1;
            if scanned >= MAX_ENTRIES {
                capped = true;
                break;
            }
            // `file_type()` does not traverse symlinks, so a symlinked dir
            // reports as a symlink here and we skip it (no cycle, no escape).
            let Ok(ft) = entry.file_type() else {
                continue;
            };
            if ft.is_symlink() {
                continue;
            }
            let path = entry.path();
            if ft.is_dir() {
                if depth < MAX_DEPTH {
                    stack.push((path, depth + 1));
                }
                continue;
            }
            if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
                continue;
            }
            if codex_session_cwd(&path).as_deref() == Some(target.as_str()) {
                let id = codex_session_id(&path).unwrap_or_else(|| {
                    path.file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or_default()
                        .to_owned()
                });
                out.push(SessionRef {
                    source: SourceTool::Codex,
                    session_id: id,
                    mtime_secs: mtime_secs(&path),
                    path,
                });
            }
        }
    }
    if capped {
        // Don't let a bounded scan masquerade as "nothing else here": say so once
        // on stderr so a user with a very deep Codex history knows discovery was
        // partial (and `/import <id>` can still target a specific session).
        eprintln!(
            "deeptide: stopped scanning ~/.codex/sessions after {MAX_ENTRIES} entries; \
             some older Codex sessions may not appear in discovery."
        );
    }
    out
}

fn discover_deeptide(cwd: &Path) -> Vec<SessionRef> {
    let dir = MemorySystem::project_memory_dir(cwd)
        .parent()
        .map(|p| p.join("sessions"))
        .unwrap_or_else(|| PathBuf::from("sessions"));
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
            continue;
        }
        let id = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_owned();
        out.push(SessionRef {
            source: SourceTool::Deeptide,
            session_id: id,
            mtime_secs: mtime_secs(&path),
            path,
        });
    }
    out
}

/// Read only the first line of a Codex rollout to pull its cwd.
fn codex_session_cwd(path: &Path) -> Option<String> {
    let line = first_line(path)?;
    let obj: serde_json::Value = serde_json::from_str(&line).ok()?;
    obj.get("payload")?
        .get("cwd")
        .and_then(|v| v.as_str())
        .map(ToOwned::to_owned)
}

fn codex_session_id(path: &Path) -> Option<String> {
    let line = first_line(path)?;
    let obj: serde_json::Value = serde_json::from_str(&line).ok()?;
    obj.get("payload")?
        .get("id")
        .and_then(|v| v.as_str())
        .map(ToOwned::to_owned)
}

fn first_line(path: &Path) -> Option<String> {
    use std::io::{BufRead, BufReader, Read};
    let file = std::fs::File::open(path).ok()?;
    let mut line = String::new();
    // Cap the read so a corrupt/newline-free file can't buffer unbounded memory
    // just to extract the meta cwd. Codex meta lines are well under this; a line
    // longer than the cap simply won't parse and the session is skipped.
    BufReader::new(file.take(256 * 1024))
        .read_line(&mut line)
        .ok()?;
    Some(line)
}

// ── Parsing ─────────────────────────────────────────────────────────────────

/// Parse a session file at `path`, dispatching on `source`.
pub fn parse_file(path: &Path, source: SourceTool) -> Result<ImportedTranscript, String> {
    let raw = std::fs::read_to_string(path)
        .map_err(|e| format!("cannot read session file {}: {e}", path.display()))?;
    let fallback_id = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("imported")
        .to_owned();
    Ok(match source {
        SourceTool::Claude => parse_claude(&raw, &fallback_id),
        SourceTool::Codex => parse_codex(&raw, &fallback_id),
        SourceTool::Deeptide => parse_deeptide(&raw, &fallback_id),
    })
}

/// Pull plain text out of an Anthropic-style content value (string, or an
/// array of blocks), joining `text`/`thinking` blocks.
fn text_from_content(content: &serde_json::Value) -> String {
    if let Some(s) = content.as_str() {
        return s.to_owned();
    }
    let Some(arr) = content.as_array() else {
        return String::new();
    };
    let mut parts = Vec::new();
    for block in arr {
        if let Some(t) = block.get("text").and_then(|v| v.as_str()) {
            parts.push(t.to_owned());
        }
    }
    parts.join("\n")
}

pub(crate) fn truncate(text: &str, cap: usize) -> String {
    // `cap` is a display-column budget; width-aware so CJK session previews in
    // the import picker are cut on the right boundary and stay aligned.
    crate::width::truncate_to_width(text.trim(), cap)
}

/// Strip the `<wrapper>…</wrapper>` system-context envelopes both Claude and
/// Codex inject around real user prompts (command caveats, environment dumps).
/// Best-effort: removes leading angle-bracket-tag blocks, keeps the human text.
fn strip_envelopes(text: &str) -> String {
    let mut s = text.trim();
    // Drop leading `<tag>…</tag>` or `<tag …>` blocks repeatedly.
    loop {
        if !s.starts_with('<') {
            break;
        }
        // Find the matching close of the first tag's region: if there's a
        // closing `</…>`, cut past it; otherwise cut past the first `>`.
        if let Some(close) = s
            .find("</")
            .and_then(|i| s[i..].find('>').map(|j| i + j + 1))
        {
            s = s[close..].trim_start();
        } else if let Some(gt) = s.find('>') {
            s = s[gt + 1..].trim_start();
        } else {
            break;
        }
    }
    s.to_owned()
}

pub fn parse_claude(raw: &str, fallback_id: &str) -> ImportedTranscript {
    let mut session_id = None;
    let mut cwd = None;
    let mut git_branch = None;
    let mut title = None;
    let mut turns = Vec::new();

    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(obj) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        // Opportunistically capture metadata wherever it appears.
        if session_id.is_none() {
            session_id = obj
                .get("sessionId")
                .and_then(|v| v.as_str())
                .map(ToOwned::to_owned);
        }
        if cwd.is_none() {
            cwd = obj
                .get("cwd")
                .and_then(|v| v.as_str())
                .map(ToOwned::to_owned);
        }
        if git_branch.is_none() {
            git_branch = obj
                .get("gitBranch")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(ToOwned::to_owned);
        }
        let ty = obj.get("type").and_then(|v| v.as_str()).unwrap_or("");
        if ty == "ai-title" && title.is_none() {
            title = obj
                .get("aiTitle")
                .and_then(|v| v.as_str())
                .map(ToOwned::to_owned);
            continue;
        }
        if ty != "user" && ty != "assistant" {
            continue;
        }
        let Some(message) = obj.get("message") else {
            continue;
        };
        let content = message.get("content").unwrap_or(&serde_json::Value::Null);

        // String content on a user line = a real human prompt.
        if ty == "user"
            && let Some(s) = content.as_str()
        {
            let cleaned = strip_envelopes(s);
            if !cleaned.is_empty() {
                turns.push(ImportedTurn::message(ImportedRole::User, cleaned));
            }
            continue;
        }

        // Otherwise walk the block array.
        if let Some(blocks) = content.as_array() {
            for block in blocks {
                let bt = block.get("type").and_then(|v| v.as_str()).unwrap_or("");
                match bt {
                    "text" => {
                        let text = block.get("text").and_then(|v| v.as_str()).unwrap_or("");
                        let role = if ty == "user" {
                            ImportedRole::User
                        } else {
                            ImportedRole::Assistant
                        };
                        let cleaned = if role == ImportedRole::User {
                            strip_envelopes(text)
                        } else {
                            text.trim().to_owned()
                        };
                        if !cleaned.is_empty() {
                            turns.push(ImportedTurn::message(role, cleaned));
                        }
                    }
                    "thinking" => {
                        let text = block.get("thinking").and_then(|v| v.as_str()).unwrap_or("");
                        if !text.trim().is_empty() {
                            turns.push(ImportedTurn {
                                role: ImportedRole::Assistant,
                                kind: TurnKind::Thinking,
                                text: truncate(text, TOOL_TEXT_CAP),
                            });
                        }
                    }
                    "tool_use" => {
                        let name = block.get("name").and_then(|v| v.as_str()).unwrap_or("tool");
                        let input = block.get("input");
                        let summary = input
                            .map(|i| truncate(&i.to_string(), TOOL_TEXT_CAP))
                            .unwrap_or_default();
                        turns.push(ImportedTurn {
                            role: ImportedRole::Assistant,
                            kind: TurnKind::ToolUse,
                            text: format!("{name}({summary})"),
                        });
                    }
                    "tool_result" => {
                        let text = truncate(
                            &text_from_content(
                                block.get("content").unwrap_or(&serde_json::Value::Null),
                            ),
                            TOOL_TEXT_CAP,
                        );
                        if !text.is_empty() {
                            turns.push(ImportedTurn {
                                role: ImportedRole::User,
                                kind: TurnKind::ToolResult,
                                text,
                            });
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    ImportedTranscript {
        source: SourceTool::Claude,
        session_id: session_id.unwrap_or_else(|| fallback_id.to_owned()),
        cwd,
        git_branch,
        title,
        turns,
    }
}

pub fn parse_codex(raw: &str, fallback_id: &str) -> ImportedTranscript {
    let mut session_id = None;
    let mut cwd = None;
    let mut turns = Vec::new();

    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(obj) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        let ty = obj.get("type").and_then(|v| v.as_str()).unwrap_or("");
        let payload = obj.get("payload").unwrap_or(&serde_json::Value::Null);
        if ty == "session_meta" {
            session_id = payload
                .get("id")
                .and_then(|v| v.as_str())
                .map(ToOwned::to_owned);
            cwd = payload
                .get("cwd")
                .and_then(|v| v.as_str())
                .map(ToOwned::to_owned);
            continue;
        }
        if ty != "response_item" {
            continue;
        }
        let pt = payload.get("type").and_then(|v| v.as_str()).unwrap_or("");
        match pt {
            "message" => {
                let role = payload.get("role").and_then(|v| v.as_str()).unwrap_or("");
                let text =
                    text_from_content(payload.get("content").unwrap_or(&serde_json::Value::Null));
                let (role, cleaned) = match role {
                    "user" => (ImportedRole::User, strip_envelopes(&text)),
                    "assistant" => (ImportedRole::Assistant, text.trim().to_owned()),
                    _ => (ImportedRole::System, strip_envelopes(&text)),
                };
                if !cleaned.is_empty() && role != ImportedRole::System {
                    turns.push(ImportedTurn::message(role, cleaned));
                }
            }
            "function_call" | "custom_tool_call" => {
                let name = payload
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("tool");
                let args = payload
                    .get("arguments")
                    .map(|a| truncate(&a.to_string(), TOOL_TEXT_CAP))
                    .unwrap_or_default();
                turns.push(ImportedTurn {
                    role: ImportedRole::Assistant,
                    kind: TurnKind::ToolUse,
                    text: format!("{name}({args})"),
                });
            }
            "function_call_output" | "custom_tool_call_output" => {
                let out = payload
                    .get("output")
                    .map(text_from_content)
                    .unwrap_or_default();
                let out = truncate(&out, TOOL_TEXT_CAP);
                if !out.is_empty() {
                    turns.push(ImportedTurn {
                        role: ImportedRole::User,
                        kind: TurnKind::ToolResult,
                        text: out,
                    });
                }
            }
            _ => {}
        }
    }

    ImportedTranscript {
        source: SourceTool::Codex,
        session_id: session_id.unwrap_or_else(|| fallback_id.to_owned()),
        cwd,
        git_branch: None,
        title: None,
        turns,
    }
}

pub fn parse_deeptide(raw: &str, fallback_id: &str) -> ImportedTranscript {
    let mut session_id = None;
    let mut cwd = None;
    let mut turns = Vec::new();

    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(obj) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        let ty = obj.get("type").and_then(|v| v.as_str()).unwrap_or("");
        if ty == "meta" {
            session_id = obj
                .get("session_id")
                .and_then(|v| v.as_str())
                .map(ToOwned::to_owned);
            cwd = obj
                .get("cwd")
                .and_then(|v| v.as_str())
                .map(ToOwned::to_owned);
            continue;
        }
        if ty != "message" {
            continue;
        }
        let role = match obj.get("role").and_then(|v| v.as_str()) {
            Some("user") => ImportedRole::User,
            Some("assistant") => ImportedRole::Assistant,
            _ => continue,
        };
        let text = obj
            .get("content")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .trim()
            .to_owned();
        if !text.is_empty() {
            turns.push(ImportedTurn::message(role, text));
        }
    }

    ImportedTranscript {
        source: SourceTool::Deeptide,
        session_id: session_id.unwrap_or_else(|| fallback_id.to_owned()),
        cwd,
        git_branch: None,
        title: None,
        turns,
    }
}

// ── Builders ────────────────────────────────────────────────────────────────

/// Render the conversational turns (human + assistant prose only) to plain text
/// for the seed/extraction prompt. Tool I/O and thinking are dropped — the
/// extractor wants decisions and intent, not the play-by-play.
pub fn flatten_for_extraction(transcript: &ImportedTranscript) -> String {
    let mut out = String::new();
    for turn in &transcript.turns {
        if turn.kind != TurnKind::Message {
            continue;
        }
        let tag = match turn.role {
            ImportedRole::User => "USER",
            ImportedRole::Assistant => "ASSISTANT",
            ImportedRole::System => continue,
        };
        out.push_str(tag);
        out.push_str(": ");
        out.push_str(turn.text.trim());
        out.push_str("\n\n");
    }
    out.trim_end().to_owned()
}

/// Build a single "context handoff" block for a live session: a framing header,
/// a one-line note about omitted older turns, then the last `recent_n`
/// conversational turns near-verbatim (plus the most recent tool actions for
/// orientation). Phrased as *prior context from another assistant* so the model
/// treats it as reference, not its own history.
pub fn build_handoff_block(transcript: &ImportedTranscript, recent_n: usize) -> String {
    let source = transcript.source.label();
    let mut lines = vec![format!(
        "[context handoff — the following is prior context from a previous \
         {source} session on this project, authored by another assistant. \
         Treat it as reference and continue the work; do not attribute these \
         turns to yourself.]"
    )];
    if let Some(title) = &transcript.title {
        lines.push(format!("Session: {title}"));
    }
    if let Some(branch) = &transcript.git_branch {
        lines.push(format!("Git branch: {branch}"));
    }

    // Conversational turns only for the verbatim tail; tool actions are
    // appended compactly so the agent knows what was touched.
    let convo: Vec<&ImportedTurn> = transcript
        .turns
        .iter()
        .filter(|t| t.kind == TurnKind::Message && t.role != ImportedRole::System)
        .collect();
    let total = convo.len();
    let start = total.saturating_sub(recent_n);
    if start > 0 {
        lines.push(format!(
            "({start} earlier turn{} omitted — run `/import {source} --as memory` \
             for the full durable context.)",
            if start == 1 { "" } else { "s" }
        ));
    }
    lines.push(String::new());
    for turn in &convo[start..] {
        let tag = match turn.role {
            ImportedRole::User => "USER",
            ImportedRole::Assistant => "ASSISTANT",
            ImportedRole::System => continue,
        };
        lines.push(format!("{tag}: {}", turn.text.trim()));
    }

    // The last few tool actions, for "what was just being done" orientation.
    let recent_tools: Vec<&ImportedTurn> = transcript
        .turns
        .iter()
        .filter(|t| t.kind == TurnKind::ToolUse)
        .collect();
    let tool_start = recent_tools.len().saturating_sub(5);
    if !recent_tools[tool_start..].is_empty() {
        lines.push(String::new());
        lines.push(String::from("Recent tool actions:"));
        for t in &recent_tools[tool_start..] {
            lines.push(format!("  - {}", t.text.trim()));
        }
    }

    lines.join("\n")
}

/// Wrap [`build_handoff_block`] output as a single user-role message ready to
/// **prepend** to a live `agent_loop`. Prepending (not replacing) keeps it a
/// stable conversation prefix — friendly to DeepSeek's prefix cache.
pub fn handoff_message(transcript: &ImportedTranscript, recent_n: usize) -> ConversationMessage {
    ConversationMessage {
        role: MessageRole::User,
        content: build_handoff_block(transcript, recent_n),
        tool_calls: Vec::new(),
        tool_results: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CLAUDE: &str = r#"
{"type":"ai-title","aiTitle":"Fix the auth flow","sessionId":"s1"}
{"type":"user","sessionId":"s1","cwd":"/repo","gitBranch":"main","message":{"role":"user","content":"<local-command-caveat>ignore me</local-command-caveat>Use pnpm, never npm."}}
{"type":"assistant","message":{"role":"assistant","content":[{"type":"thinking","thinking":"let me think"},{"type":"text","text":"Got it, pnpm it is."},{"type":"tool_use","name":"Bash","input":{"command":"pnpm install"}}]}}
{"type":"user","message":{"role":"user","content":[{"type":"tool_result","content":"done"}]}}
{"type":"system","content":"noise"}
"#;

    const CODEX: &str = r#"
{"type":"session_meta","payload":{"id":"c1","cwd":"/repo"}}
{"type":"response_item","payload":{"type":"message","role":"developer","content":[{"type":"input_text","text":"<permissions instructions>noise</permissions instructions>"}]}}
{"type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"<environment_context><cwd>/repo</cwd></environment_context>Deploy to cn-shanghai."}]}}
{"type":"response_item","payload":{"type":"function_call","name":"shell","arguments":{"cmd":"ls"}}}
{"type":"response_item","payload":{"type":"message","role":"assistant","content":[{"type":"output_text","text":"Deploying now."}]}}
"#;

    #[test]
    fn claude_parser_extracts_human_assistant_and_tools() {
        let t = parse_claude(CLAUDE, "fallback");
        assert_eq!(t.source, SourceTool::Claude);
        assert_eq!(t.session_id, "s1");
        assert_eq!(t.cwd.as_deref(), Some("/repo"));
        assert_eq!(t.git_branch.as_deref(), Some("main"));
        assert_eq!(t.title.as_deref(), Some("Fix the auth flow"));
        // Envelope stripped from the human prompt.
        let user = t
            .turns
            .iter()
            .find(|t| t.role == ImportedRole::User && t.kind == TurnKind::Message)
            .expect("a human user message");
        assert_eq!(user.text, "Use pnpm, never npm.");
        assert!(!user.text.contains("ignore me"));
        // Assistant text + a tool_use + a tool_result all captured.
        assert!(
            t.turns
                .iter()
                .any(|t| t.kind == TurnKind::ToolUse && t.text.starts_with("Bash("))
        );
        assert!(
            t.turns
                .iter()
                .any(|t| t.kind == TurnKind::ToolResult && t.text == "done")
        );
        assert!(t.turns.iter().any(|t| t.kind == TurnKind::Thinking));
    }

    #[test]
    fn codex_parser_strips_envelopes_and_keeps_prose() {
        let t = parse_codex(CODEX, "fallback");
        assert_eq!(t.source, SourceTool::Codex);
        assert_eq!(t.session_id, "c1");
        assert_eq!(t.cwd.as_deref(), Some("/repo"));
        let user = t
            .turns
            .iter()
            .find(|t| t.role == ImportedRole::User && t.kind == TurnKind::Message)
            .expect("a human user message");
        assert_eq!(user.text, "Deploy to cn-shanghai.");
        assert!(
            t.turns
                .iter()
                .any(|t| t.role == ImportedRole::Assistant && t.text == "Deploying now.")
        );
        // The developer/system envelope produced no user/assistant message.
        assert_eq!(t.message_turns(), 2);
    }

    #[test]
    fn malformed_lines_are_skipped_not_fatal() {
        let raw = "not json\n{\"type\":\"user\",\"message\":{\"role\":\"user\",\"content\":\"hi\"}}\n{bad";
        let t = parse_claude(raw, "fb");
        assert_eq!(t.message_turns(), 1);
        assert_eq!(t.session_id, "fb");
    }

    #[test]
    fn flatten_keeps_prose_drops_tools() {
        let t = parse_claude(CLAUDE, "fb");
        let flat = flatten_for_extraction(&t);
        assert!(flat.contains("USER: Use pnpm, never npm."));
        assert!(flat.contains("ASSISTANT: Got it, pnpm it is."));
        assert!(
            !flat.contains("Bash("),
            "tool calls must not leak into the seed text: {flat}"
        );
        assert!(!flat.contains("done"), "tool results must not leak: {flat}");
    }

    #[test]
    fn handoff_block_frames_and_keeps_recent_tail() {
        let t = parse_claude(CLAUDE, "fb");
        let block = build_handoff_block(&t, 1);
        assert!(block.contains("context handoff"));
        assert!(block.contains("another assistant"));
        // recent_n = 1 keeps the last conversational turn (assistant text)
        // and notes the earlier one was omitted.
        assert!(block.contains("1 earlier turn"));
        assert!(block.contains("ASSISTANT: Got it, pnpm it is."));
        // Tool orientation line present.
        assert!(block.contains("Recent tool actions:"));
        assert!(block.contains("Bash("));
    }

    #[test]
    fn slug_variants_cover_claude_and_deeptide_forms() {
        let v = slug_variants(Path::new("/Users/me/proj"));
        assert!(v.contains(&"-Users-me-proj".to_owned()), "{v:?}");
        assert!(v.contains(&"Users-me-proj".to_owned()), "{v:?}");
    }
}
