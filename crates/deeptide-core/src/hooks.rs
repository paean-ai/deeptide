//! Lifecycle hook execution.
//!
//! Mirrors the Swift implementation's `HookEngine`: users configure shell
//! commands under the `hooks` block of `settings.json` that run at lifecycle
//! events. A `PreToolUse` hook that exits non-zero **blocks** the tool call;
//! other events are observational.
//!
//! Each hook runs through the platform shell (`sh -c` / `cmd /C`) in the
//! session's working directory, receiving context via environment variables
//! (`TIDE_EVENT`, `TIDE_INPUT_JSON`, `TIDE_CWD`, … plus `DEEPTIDE_*` aliases).
//! A per-hook timeout bounds runaway commands.

use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use crate::config::{HookEntry, SettingsHooks};

/// A lifecycle event that can trigger hooks. Matches the events modeled by
/// [`SettingsHooks`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookEvent {
    PreToolUse,
    PostToolUse,
    UserPromptSubmit,
    SessionStart,
    SessionEnd,
    PreCompact,
    /// Fires once when the agent finishes responding to a user turn (the agent
    /// loop returns). Useful for turn-end automation: auto-format, auto-commit,
    /// notify. Observational — the result does not block.
    Stop,
    /// Fires once when a sub-agent (the `Agent` tool) finishes its own loop.
    SubagentStop,
}

impl HookEvent {
    pub fn as_str(self) -> &'static str {
        match self {
            HookEvent::PreToolUse => "PreToolUse",
            HookEvent::PostToolUse => "PostToolUse",
            HookEvent::UserPromptSubmit => "UserPromptSubmit",
            HookEvent::SessionStart => "SessionStart",
            HookEvent::SessionEnd => "SessionEnd",
            HookEvent::PreCompact => "PreCompact",
            HookEvent::Stop => "Stop",
            HookEvent::SubagentStop => "SubagentStop",
        }
    }
}

/// Result of running a single hook command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HookOutcome {
    pub name: String,
    pub matcher: String,
    pub exit_code: i32,
    /// `true` when the hook exited non-zero (or failed/timed out). For
    /// `PreToolUse` this means the tool call should be denied.
    pub blocked: bool,
    /// Combined stdout+stderr, trimmed. Surfaced as the block reason.
    pub output: String,
}

/// Runs configured hooks for lifecycle events.
#[derive(Debug, Clone, Default)]
pub struct HookEngine {
    hooks: SettingsHooks,
    cwd: std::path::PathBuf,
    session_id: String,
}

impl HookEngine {
    /// An engine with no configured hooks; every `run` is a no-op.
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn new(hooks: SettingsHooks, cwd: impl Into<std::path::PathBuf>) -> Self {
        Self {
            hooks,
            cwd: cwd.into(),
            session_id: String::new(),
        }
    }

    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = session_id.into();
        self
    }

    fn entries(&self, event: HookEvent) -> &[HookEntry] {
        let group = match event {
            HookEvent::PreToolUse => &self.hooks.pre_tool_use,
            HookEvent::PostToolUse => &self.hooks.post_tool_use,
            HookEvent::UserPromptSubmit => &self.hooks.user_prompt_submit,
            HookEvent::SessionStart => &self.hooks.session_start,
            HookEvent::SessionEnd => &self.hooks.session_end,
            HookEvent::PreCompact => &self.hooks.pre_compact,
            HookEvent::Stop => &self.hooks.stop,
            HookEvent::SubagentStop => &self.hooks.subagent_stop,
        };
        group.as_deref().unwrap_or(&[])
    }

    /// Whether any enabled hook is configured for `event`.
    pub fn has_hooks(&self, event: HookEvent) -> bool {
        self.entries(event).iter().any(|entry| !entry.is_disabled())
    }

    /// Run every enabled hook for `event` whose matcher is `*` or equals
    /// `tool`, in declaration order. Returns one outcome per hook run.
    pub fn run(
        &self,
        event: HookEvent,
        tool: Option<&str>,
        input_json: Option<&str>,
    ) -> Vec<HookOutcome> {
        let payload = make_input_payload(tool, input_json);
        self.entries(event)
            .iter()
            .filter(|entry| !entry.is_disabled() && matcher_matches(&entry.matcher, tool))
            .map(|entry| self.run_one(event, entry, tool, input_json, &payload))
            .collect()
    }

    /// Convenience for `PreToolUse`: returns a denial reason if any hook blocks
    /// the tool, otherwise `None`.
    pub fn pre_tool_block_reason(&self, tool: &str, input_json: Option<&str>) -> Option<String> {
        self.run(HookEvent::PreToolUse, Some(tool), input_json)
            .into_iter()
            .find(|outcome| outcome.blocked)
            .map(|outcome| {
                let name = if outcome.name.is_empty() {
                    outcome.matcher
                } else {
                    outcome.name
                };
                let detail = if outcome.output.is_empty() {
                    format!("exit code {}", outcome.exit_code)
                } else {
                    outcome.output
                };
                format!("Blocked by PreToolUse hook '{name}': {detail}")
            })
    }

    fn run_one(
        &self,
        event: HookEvent,
        entry: &HookEntry,
        tool: Option<&str>,
        input_json: Option<&str>,
        payload: &str,
    ) -> HookOutcome {
        let name = entry.name.clone().unwrap_or_else(|| entry.matcher.clone());
        let mut command = shell_command(&entry.command);
        command
            .current_dir(&self.cwd)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .env("TIDE_EVENT", event.as_str())
            .env("TIDE_INPUT_JSON", payload)
            .env("TIDE_CWD", &self.cwd)
            .env("TIDE_SESSION_ID", &self.session_id)
            .env("TIDE_HOOK_NAME", &name)
            .env("TIDE_SPEC_VERSION", "0.1")
            // Legacy aliases for hooks written against pre-spec deeptide.
            .env("DEEPTIDE_EVENT", event.as_str())
            .env("DEEPTIDE_TOOL", tool.unwrap_or(""))
            .env("DEEPTIDE_INPUT_JSON", input_json.unwrap_or(""))
            .env("DEEPTIDE_CWD", &self.cwd);

        let child = match command.spawn() {
            Ok(child) => child,
            Err(error) => {
                return HookOutcome {
                    name,
                    matcher: entry.matcher.clone(),
                    exit_code: -1,
                    blocked: true,
                    output: format!("hook failed to start: {error}"),
                };
            }
        };

        // Wait for completion on a worker thread so a per-hook timeout can be
        // enforced. On timeout the child is left to finish in the background;
        // the outcome is reported as blocked.
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let _ = tx.send(child.wait_with_output());
        });

        match rx.recv_timeout(Duration::from_millis(entry.effective_timeout_ms())) {
            Ok(Ok(output)) => {
                let exit_code = output.status.code().unwrap_or(-1);
                let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
                text.push_str(&String::from_utf8_lossy(&output.stderr));
                HookOutcome {
                    name,
                    matcher: entry.matcher.clone(),
                    exit_code,
                    blocked: exit_code != 0,
                    output: text.trim().to_owned(),
                }
            }
            Ok(Err(error)) => HookOutcome {
                name,
                matcher: entry.matcher.clone(),
                exit_code: -1,
                blocked: true,
                output: format!("hook error: {error}"),
            },
            Err(_) => HookOutcome {
                name,
                matcher: entry.matcher.clone(),
                exit_code: -1,
                blocked: true,
                output: format!("hook timed out after {}ms", entry.effective_timeout_ms()),
            },
        }
    }
}

fn matcher_matches(matcher: &str, tool: Option<&str>) -> bool {
    matcher == "*" || tool.is_some_and(|tool| matcher == tool)
}

/// Build the `TIDE_INPUT_JSON` payload: `{"tool_name": …, "input": …}`.
fn make_input_payload(tool: Option<&str>, input_json: Option<&str>) -> String {
    let mut obj = serde_json::Map::new();
    if let Some(tool) = tool.filter(|tool| !tool.is_empty()) {
        obj.insert(
            String::from("tool_name"),
            serde_json::Value::String(tool.to_owned()),
        );
    }
    if let Some(raw) = input_json.filter(|raw| !raw.is_empty()) {
        match serde_json::from_str::<serde_json::Value>(raw) {
            Ok(parsed) => {
                obj.insert(String::from("input"), parsed);
            }
            Err(_) => {
                obj.insert(
                    String::from("input_raw"),
                    serde_json::Value::String(raw.to_owned()),
                );
            }
        }
    }
    if obj.is_empty() {
        return String::from("{}");
    }
    serde_json::to_string(&serde_json::Value::Object(obj)).unwrap_or_else(|_| String::from("{}"))
}

#[cfg(unix)]
fn shell_command(command: &str) -> Command {
    let mut c = Command::new("/bin/sh");
    c.arg("-c").arg(command);
    c
}

#[cfg(windows)]
fn shell_command(command: &str) -> Command {
    let mut c = Command::new("cmd");
    c.arg("/C").arg(command);
    c
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::HookEntry;

    fn hook(matcher: &str, command: &str) -> HookEntry {
        HookEntry {
            matcher: matcher.to_owned(),
            command: command.to_owned(),
            timeout_ms: Some(5_000),
            disabled: None,
            name: None,
        }
    }

    #[test]
    fn matcher_matches_wildcard_and_exact() {
        assert!(matcher_matches("*", Some("Read")));
        assert!(matcher_matches("Read", Some("Read")));
        assert!(!matcher_matches("Read", Some("Write")));
        assert!(!matcher_matches("Read", None));
    }

    #[test]
    fn input_payload_wraps_tool_and_parsed_input() {
        let payload = make_input_payload(Some("Read"), Some(r#"{"file_path":"a.txt"}"#));
        assert!(payload.contains("\"tool_name\":\"Read\""));
        assert!(payload.contains("\"input\""));
        assert!(payload.contains("a.txt"));
        assert_eq!(make_input_payload(None, None), "{}");
    }

    #[cfg(unix)]
    #[test]
    fn runs_matching_hooks_and_reports_block_on_nonzero_exit() {
        let hooks = SettingsHooks {
            pre_tool_use: Some(vec![
                hook("Read", "exit 2"),
                hook("Write", "exit 0"),
                hook("*", "exit 0"),
            ]),
            ..Default::default()
        };
        let engine = HookEngine::new(hooks, std::env::temp_dir());

        // Read matches the `Read` hook (exit 2 -> block) and the `*` hook (ok).
        let outcomes = engine.run(HookEvent::PreToolUse, Some("Read"), None);
        assert_eq!(outcomes.len(), 2);
        assert!(outcomes.iter().any(|o| o.blocked && o.matcher == "Read"));

        let reason = engine.pre_tool_block_reason("Read", None);
        assert!(reason.is_some(), "Read should be blocked");
        // Write matches only its own (exit 0) and `*` (exit 0) -> not blocked.
        assert!(engine.pre_tool_block_reason("Write", None).is_none());
    }

    #[cfg(unix)]
    #[test]
    fn passes_event_and_tool_via_environment() {
        let dir = tempfile::tempdir().expect("tempdir");
        let out = dir.path().join("hook-env.txt");
        let hooks = SettingsHooks {
            pre_tool_use: Some(vec![hook(
                "*",
                &format!(
                    "printf '%s %s' \"$TIDE_EVENT\" \"$DEEPTIDE_TOOL\" > {}",
                    out.display()
                ),
            )]),
            ..Default::default()
        };
        let engine = HookEngine::new(hooks, dir.path());
        let outcomes = engine.run(HookEvent::PreToolUse, Some("Bash"), None);
        assert_eq!(outcomes.len(), 1);
        assert!(!outcomes[0].blocked);

        let recorded = std::fs::read_to_string(&out).expect("hook wrote env file");
        assert_eq!(recorded, "PreToolUse Bash");
    }

    #[cfg(unix)]
    #[test]
    fn disabled_hooks_are_skipped() {
        let mut entry = hook("*", "exit 1");
        entry.disabled = Some(true);
        let hooks = SettingsHooks {
            pre_tool_use: Some(vec![entry]),
            ..Default::default()
        };
        let engine = HookEngine::new(hooks, std::env::temp_dir());
        assert!(
            engine
                .run(HookEvent::PreToolUse, Some("Read"), None)
                .is_empty()
        );
        assert!(!engine.has_hooks(HookEvent::PreToolUse));
    }
}
