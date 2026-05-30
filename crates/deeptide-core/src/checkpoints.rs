//! In-session conversation checkpoints. Snapshots of the
//! `ConversationMessage` list with a short ID, optional human label,
//! and creation timestamp. Used by `/checkpoint` + `/rewind` so users
//! can punctually rewind multi-turn mistakes without restarting the
//! session or losing cost-tracker telemetry.
//!
//! Storage policy is intentionally in-memory only — checkpoints live
//! for the lifetime of the REPL and a FIFO cap (`MAX_CHECKPOINTS`)
//! prevents an interactive user from accidentally pinning hundreds of
//! megabytes of transcript over a long session. For cross-session
//! survival, users can still use `/sessions` + `/resume` (the existing
//! on-disk session store).

use crate::agent_loop::ConversationMessage;

/// FIFO upper bound on the number of checkpoints retained per session.
/// Each snapshot clones the entire transcript so the memory footprint
/// grows linearly with both session length and checkpoint count; this
/// ceiling keeps a worst-case session bounded at ~20× one transcript.
pub const MAX_CHECKPOINTS: usize = 20;

/// One named snapshot of the conversation state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Checkpoint {
    /// 8-character timestamp-derived hex string used to address this
    /// checkpoint from the command line. Stable for the lifetime of
    /// the REPL; not guaranteed to be globally unique.
    pub id: String,
    /// Optional human label supplied via `/checkpoint <label>`. Empty
    /// when the user omitted it.
    pub label: String,
    /// RFC3339 creation timestamp.
    pub created_at: String,
    /// Number of messages captured. Convenience copy of
    /// `messages.len()` so the list view doesn't have to walk every
    /// snapshot.
    pub message_count: usize,
    /// Model active when the checkpoint was taken. Informational —
    /// restoring doesn't change the current model.
    pub model: String,
    /// Captured transcript. Cloned at save time so subsequent edits to
    /// the live transcript don't leak in.
    pub messages: Vec<ConversationMessage>,
}

impl Checkpoint {
    /// One-line summary suitable for `/checkpoints` listing. Format:
    /// `<id>  <created_at>  msgs=<N>  [label]`. The label is omitted
    /// (rather than rendered as empty brackets) when unset.
    pub fn summarise(&self, index: usize) -> String {
        let label_part = if self.label.is_empty() {
            String::new()
        } else {
            format!("  {}", self.label)
        };
        format!(
            "  {:>2}. {}  {}  msgs={}{label_part}",
            index + 1,
            self.id,
            self.created_at,
            self.message_count,
        )
    }
}

/// FIFO bounded queue of checkpoints. Pushing past `MAX_CHECKPOINTS`
/// drops the oldest entry; explicit `restore` / `drop` lookups support
/// matching by index (1-based), exact ID, ID prefix, or exact label.
#[derive(Debug, Clone, Default)]
pub struct CheckpointStore {
    items: Vec<Checkpoint>,
}

/// How a CLI-supplied selector resolved against the store. Lets the
/// caller produce specific error messages instead of a generic
/// "couldn't find checkpoint".
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectorOutcome {
    Found(usize),
    Empty,
    NotFound,
    Ambiguous { matches: Vec<String> },
}

impl CheckpointStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Checkpoint> {
        self.items.iter()
    }

    /// Push a fresh checkpoint. If we're already at `MAX_CHECKPOINTS`,
    /// the OLDEST snapshot is evicted first (FIFO).
    pub fn push(&mut self, checkpoint: Checkpoint) {
        if self.items.len() >= MAX_CHECKPOINTS {
            self.items.remove(0);
        }
        self.items.push(checkpoint);
    }

    /// Drop every checkpoint. Returns the count cleared.
    pub fn clear(&mut self) -> usize {
        let n = self.items.len();
        self.items.clear();
        n
    }

    /// Resolve a CLI selector against the store. The grammar accepts:
    ///
    /// * empty → newest checkpoint (only when the store is non-empty)
    /// * `latest` / `last` / `top` → newest
    /// * a 1-based decimal index (the position shown by `/checkpoints`)
    /// * an exact 8-char ID
    /// * a non-empty ID prefix (at least 2 chars; matches one or
    ///   produces an Ambiguous outcome)
    /// * an exact label match (case-sensitive)
    pub fn resolve(&self, selector: &str) -> SelectorOutcome {
        if self.items.is_empty() {
            return SelectorOutcome::Empty;
        }
        let trimmed = selector.trim();
        if trimmed.is_empty() || matches!(trimmed, "latest" | "last" | "top") {
            return SelectorOutcome::Found(self.items.len() - 1);
        }

        if let Ok(n) = trimmed.parse::<usize>()
            && n >= 1
            && n <= self.items.len()
        {
            return SelectorOutcome::Found(n - 1);
        }

        // Exact ID match takes precedence over prefix.
        if let Some((i, _)) = self.items.iter().enumerate().find(|(_, c)| c.id == trimmed) {
            return SelectorOutcome::Found(i);
        }

        // Exact label match.
        if !trimmed.is_empty()
            && let Some((i, _)) = self
                .items
                .iter()
                .enumerate()
                .find(|(_, c)| !c.label.is_empty() && c.label == trimmed)
        {
            return SelectorOutcome::Found(i);
        }

        // ID prefix (minimum 2 chars to avoid 1-char accidents).
        if trimmed.len() >= 2 {
            let hits: Vec<(usize, &Checkpoint)> = self
                .items
                .iter()
                .enumerate()
                .filter(|(_, c)| c.id.starts_with(trimmed))
                .collect();
            match hits.len() {
                0 => {}
                1 => return SelectorOutcome::Found(hits[0].0),
                _ => {
                    return SelectorOutcome::Ambiguous {
                        matches: hits.iter().map(|(_, c)| c.id.clone()).collect(),
                    };
                }
            }
        }

        SelectorOutcome::NotFound
    }

    /// Borrow a checkpoint by its zero-based index. Returns `None` if
    /// the index is out of range.
    pub fn get(&self, idx: usize) -> Option<&Checkpoint> {
        self.items.get(idx)
    }

    /// Remove the checkpoint at `idx` and return it. `None` for an
    /// out-of-range index.
    pub fn remove(&mut self, idx: usize) -> Option<Checkpoint> {
        if idx >= self.items.len() {
            return None;
        }
        Some(self.items.remove(idx))
    }
}

/// Generate a fresh checkpoint ID from a timestamp. Uses the lower 32
/// bits of the Unix-nanos representation (rendered as zero-padded
/// hex). Collisions in the same millisecond are extremely unlikely in
/// an interactive REPL — if two snapshots ever do collide, the
/// selector logic falls back to label / index lookup.
pub fn fresh_checkpoint_id() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    format!("{:08x}", (nanos as u32) ^ ((nanos >> 32) as u32))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use crate::agent_loop::MessageRole;

    fn mk_msg(text: &str) -> ConversationMessage {
        ConversationMessage {
            role: MessageRole::User,
            content: text.to_owned(),
            tool_calls: Vec::new(),
            tool_results: Vec::new(),
        }
    }

    fn mk_cp(id: &str, label: &str, msgs: usize) -> Checkpoint {
        Checkpoint {
            id: id.to_owned(),
            label: label.to_owned(),
            created_at: String::from("2026-01-01T00:00:00Z"),
            message_count: msgs,
            model: String::from("test-model"),
            messages: (0..msgs).map(|i| mk_msg(&format!("m{i}"))).collect(),
        }
    }

    #[test]
    fn empty_store_resolve_returns_empty() {
        let store = CheckpointStore::new();
        assert_eq!(store.resolve(""), SelectorOutcome::Empty);
        assert_eq!(store.resolve("anything"), SelectorOutcome::Empty);
    }

    #[test]
    fn empty_selector_picks_latest() {
        let mut store = CheckpointStore::new();
        store.push(mk_cp("aaaa1111", "", 1));
        store.push(mk_cp("bbbb2222", "", 2));
        assert_eq!(store.resolve(""), SelectorOutcome::Found(1));
        assert_eq!(store.resolve("latest"), SelectorOutcome::Found(1));
        assert_eq!(store.resolve("last"), SelectorOutcome::Found(1));
    }

    #[test]
    fn index_selector_is_one_based() {
        let mut store = CheckpointStore::new();
        store.push(mk_cp("aaaa1111", "", 1));
        store.push(mk_cp("bbbb2222", "", 2));
        assert_eq!(store.resolve("1"), SelectorOutcome::Found(0));
        assert_eq!(store.resolve("2"), SelectorOutcome::Found(1));
        // Out-of-range index falls through to other resolution paths.
        assert_eq!(store.resolve("99"), SelectorOutcome::NotFound);
        assert_eq!(store.resolve("0"), SelectorOutcome::NotFound);
    }

    #[test]
    fn exact_id_selector() {
        let mut store = CheckpointStore::new();
        store.push(mk_cp("aaaa1111", "", 1));
        store.push(mk_cp("bbbb2222", "", 2));
        assert_eq!(store.resolve("bbbb2222"), SelectorOutcome::Found(1));
    }

    #[test]
    fn id_prefix_selector_when_unique() {
        let mut store = CheckpointStore::new();
        store.push(mk_cp("aaaa1111", "", 1));
        store.push(mk_cp("bbbb2222", "", 2));
        assert_eq!(store.resolve("bb"), SelectorOutcome::Found(1));
        assert_eq!(store.resolve("bbbb22"), SelectorOutcome::Found(1));
    }

    #[test]
    fn id_prefix_selector_reports_ambiguity() {
        let mut store = CheckpointStore::new();
        store.push(mk_cp("aaaa1111", "", 1));
        store.push(mk_cp("aaaa2222", "", 2));
        match store.resolve("aaaa") {
            SelectorOutcome::Ambiguous { matches } => {
                assert_eq!(matches.len(), 2);
            }
            other => panic!("expected Ambiguous, got {other:?}"),
        }
    }

    #[test]
    fn id_prefix_requires_at_least_two_chars() {
        let mut store = CheckpointStore::new();
        store.push(mk_cp("aaaa1111", "", 1));
        // single char doesn't kick prefix logic.
        assert_eq!(store.resolve("a"), SelectorOutcome::NotFound);
    }

    #[test]
    fn label_selector_is_exact_case_sensitive() {
        let mut store = CheckpointStore::new();
        store.push(mk_cp("aaaa1111", "before-refactor", 1));
        store.push(mk_cp("bbbb2222", "after-fix", 2));
        assert_eq!(store.resolve("before-refactor"), SelectorOutcome::Found(0));
        // Wrong case → no match.
        assert_eq!(store.resolve("Before-Refactor"), SelectorOutcome::NotFound);
    }

    #[test]
    fn push_fifo_evicts_oldest_at_cap() {
        let mut store = CheckpointStore::new();
        for n in 0..MAX_CHECKPOINTS + 5 {
            store.push(mk_cp(&format!("{n:08x}"), "", 1));
        }
        assert_eq!(store.len(), MAX_CHECKPOINTS);
        let first_id = &store.iter().next().unwrap().id;
        assert_eq!(first_id, &format!("{:08x}", 5));
    }

    #[test]
    fn clear_returns_count_and_resets() {
        let mut store = CheckpointStore::new();
        store.push(mk_cp("aaaa1111", "", 1));
        store.push(mk_cp("bbbb2222", "", 1));
        assert_eq!(store.clear(), 2);
        assert!(store.is_empty());
        assert_eq!(store.clear(), 0);
    }

    #[test]
    fn remove_by_index_returns_entry() {
        let mut store = CheckpointStore::new();
        store.push(mk_cp("aaaa1111", "", 1));
        store.push(mk_cp("bbbb2222", "", 2));
        let removed = store.remove(0).expect("removed");
        assert_eq!(removed.id, "aaaa1111");
        assert_eq!(store.len(), 1);
        assert!(store.remove(5).is_none());
    }

    #[test]
    fn summarise_format_includes_index_id_and_label_when_present() {
        let cp = mk_cp("aaaa1111", "pre-fix", 7);
        let line = cp.summarise(0);
        assert!(line.contains("aaaa1111"));
        assert!(line.contains("msgs=7"));
        assert!(line.contains("pre-fix"));
    }

    #[test]
    fn summarise_omits_label_when_empty() {
        let cp = mk_cp("aaaa1111", "", 3);
        let line = cp.summarise(0);
        assert!(
            !line.ends_with("  "),
            "trailing whitespace not allowed: {line}"
        );
    }

    #[test]
    fn fresh_id_is_8_hex_chars() {
        let id = fresh_checkpoint_id();
        assert_eq!(id.len(), 8);
        assert!(id.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
