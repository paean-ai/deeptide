use std::path::Path;

use crate::memory::MemorySystem;

/// Identity preamble sent on every request.
const AGENT_IDENTITY: &str = "\
You are Deeptide, an AI coding agent. You help users write, debug, and improve code.\n\
You can read and modify files, run shell commands, search the codebase, and call external tools.\n\
When asked to make changes, prefer editing existing files over rewriting them from scratch.\n\
Keep responses concise unless the user asks for detail.";

/// Build the system prompt for a session rooted at `cwd`.
///
/// The prompt is assembled from three optional layers:
/// 1. **Identity** — always present; describes the agent's role and style.
/// 2. **Project guide** — content of the first `CLAUDE.md`, `TIDE.md`,
///    `AGENTS.md`, or `ZERO.md` found in `cwd`.  Truncated to 8 KB.
/// 3. **Memory** — MEMORY.md content loaded by `MemorySystem::load_memory_prompt`.
///
/// Each layer is separated by a blank line.  An empty project guide and empty
/// memory are silently omitted.
pub fn build_system_prompt(cwd: &Path) -> String {
    let mut parts: Vec<String> = vec![AGENT_IDENTITY.to_owned()];

    if let Some(guide) = project_guide(cwd) {
        parts.push(guide);
    }

    let memory = MemorySystem::load_memory_prompt(cwd);
    if !memory.trim().is_empty() {
        parts.push(memory);
    }

    parts.join("\n\n")
}

/// Read the first project guide file found in `cwd`, truncated to 8 KB.
///
/// Checks `CLAUDE.md`, `TIDE.md`, `AGENTS.md`, and `ZERO.md` in that order.
/// Returns `None` if none of them exist or all are empty.
fn project_guide(cwd: &Path) -> Option<String> {
    const MAX_GUIDE_BYTES: usize = 8 * 1024;

    for name in [
        "CLAUDE.md",
        "TIDE.md",
        "AGENTS.md",
        "ZERO.md",
        ".cursorrules",
    ] {
        let path = cwd.join(name);
        let Ok(content) = std::fs::read_to_string(&path) else {
            continue;
        };
        let trimmed = content.trim();
        if trimmed.is_empty() {
            continue;
        }
        let truncated = truncate_bytes(trimmed, MAX_GUIDE_BYTES);
        let header = format!("# Project guide ({name})");
        return Some(format!("{header}\n\n{truncated}"));
    }
    None
}

/// Truncate `s` to at most `max_bytes` bytes, breaking on a character boundary.
fn truncate_bytes(s: &str, max_bytes: usize) -> String {
    if s.len() <= max_bytes {
        return s.to_owned();
    }
    // Walk back from the max-byte boundary to the nearest valid char start.
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}… (truncated)", &s[..end])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_system_prompt_always_contains_identity() {
        let temp = tempfile::tempdir().expect("tempdir");
        let prompt = build_system_prompt(temp.path());
        assert!(
            prompt.contains("You are Deeptide"),
            "prompt should contain identity: {prompt}"
        );
    }

    #[test]
    fn build_system_prompt_includes_claude_md_if_present() {
        let temp = tempfile::tempdir().expect("tempdir");
        std::fs::write(temp.path().join("CLAUDE.md"), "# My Project\nUse tabs.").expect("write");

        let prompt = build_system_prompt(temp.path());
        assert!(
            prompt.contains("Project guide (CLAUDE.md)"),
            "should include guide header: {prompt}"
        );
        assert!(
            prompt.contains("Use tabs."),
            "should include guide body: {prompt}"
        );
    }

    #[test]
    fn build_system_prompt_prefers_claude_md_over_tide_md() {
        let temp = tempfile::tempdir().expect("tempdir");
        std::fs::write(temp.path().join("CLAUDE.md"), "from-claude").expect("write");
        std::fs::write(temp.path().join("TIDE.md"), "from-tide").expect("write");

        let prompt = build_system_prompt(temp.path());
        assert!(prompt.contains("from-claude"), "should prefer CLAUDE.md");
        assert!(
            !prompt.contains("from-tide"),
            "should not include TIDE.md when CLAUDE.md present"
        );
    }

    #[test]
    fn build_system_prompt_falls_back_to_tide_md() {
        let temp = tempfile::tempdir().expect("tempdir");
        std::fs::write(temp.path().join("TIDE.md"), "tide-content").expect("write");

        let prompt = build_system_prompt(temp.path());
        assert!(
            prompt.contains("tide-content"),
            "should fall back to TIDE.md"
        );
    }

    #[test]
    fn build_system_prompt_without_any_guide_is_just_identity() {
        let temp = tempfile::tempdir().expect("tempdir");
        let prompt = build_system_prompt(temp.path());
        // Should not have the guide section header
        assert!(
            !prompt.contains("Project guide"),
            "should have no guide section"
        );
    }

    #[test]
    fn truncate_bytes_leaves_short_strings_intact() {
        assert_eq!(truncate_bytes("hello", 100), "hello");
    }

    #[test]
    fn truncate_bytes_cuts_at_byte_boundary() {
        let s = "a".repeat(20);
        let truncated = truncate_bytes(&s, 10);
        assert!(truncated.starts_with("aaaaaaaaaa"));
        assert!(truncated.contains("truncated"));
    }
}
