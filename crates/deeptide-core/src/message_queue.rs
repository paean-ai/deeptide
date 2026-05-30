//! Per-session FIFO queue of user messages typed while the agent is busy.
//!
//! Motivation: the REPL drives `repl.submit()` synchronously — once a turn
//! starts, the main thread is busy streaming model tokens and running tools
//! until the turn completes. Without this queue, anything the user types
//! during that window is either lost (raw stdin discarded) or appears
//! garbled inline with the agent's streamed output. With the queue, lines
//! the user typed mid-turn (or explicitly enqueued via `/queue add`) are
//! collected and drained automatically once the agent yields.
//!
//! The queue is intentionally a simple, in-memory data structure with no
//! persistence — between sessions the user starts fresh. Sharing across
//! threads (the CLI's mid-turn poll worker writes into it from the
//! streaming handler thread; the main thread drains it) is the caller's
//! responsibility via `Arc<Mutex<MessageQueue>>`.

use std::collections::VecDeque;
use std::fmt;

/// Strategy for draining queued messages once an agent turn ends.
///
/// Default is `Single`: it preserves the conversational structure of one
/// user message ↔ one agent reply, which is what most LLM backends are
/// trained against and what produces predictable cost/token accounting.
/// `Batch` is opt-in for "I just want the model to see all my follow-ups
/// at once" workflows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum QueueMode {
    /// Pop one message per agent turn. After the turn finishes, the
    /// head of the queue becomes the next user prompt; any remaining
    /// queued messages stay queued for subsequent turns.
    #[default]
    Single,
    /// Drain the entire queue in one shot — concatenate every queued
    /// message with double-newline separators and feed them to the agent
    /// as a single user prompt.
    Batch,
}

impl QueueMode {
    /// Human-readable label for status/help output.
    pub fn label(self) -> &'static str {
        match self {
            Self::Single => "single",
            Self::Batch => "batch",
        }
    }

    /// Parse from a CLI / config string. Accepts canonical lowercase plus
    /// the common synonyms `one` / `all` so users don't have to remember
    /// exact terminology.
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "single" | "one" | "1" => Some(Self::Single),
            "batch" | "all" | "merge" => Some(Self::Batch),
            _ => None,
        }
    }
}

impl fmt::Display for QueueMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// FIFO queue of pending user prompts. Cheap to clone (it's a wrapper
/// over `VecDeque<String>`), but in practice you'll wrap one instance in
/// `Arc<Mutex<…>>` and share it; cloning is here for tests and serde-free
/// debug snapshots.
#[derive(Debug, Clone, Default)]
pub struct MessageQueue {
    items: VecDeque<String>,
    mode: QueueMode,
}

impl MessageQueue {
    /// Empty queue in `QueueMode::Single`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Empty queue with an explicit starting mode.
    pub fn with_mode(mode: QueueMode) -> Self {
        Self {
            items: VecDeque::new(),
            mode,
        }
    }

    /// Push a single message at the tail of the queue. Whitespace-only
    /// input is silently dropped — there's no value queueing an empty
    /// prompt and surfacing one would confuse the drain UX. Returns
    /// `true` if something was actually enqueued.
    pub fn push(&mut self, message: impl Into<String>) -> bool {
        let trimmed = message.into().trim().to_owned();
        if trimmed.is_empty() {
            return false;
        }
        self.items.push_back(trimmed);
        true
    }

    /// Drop everything in the queue, returning the count cleared so the
    /// caller can render a confirmation.
    pub fn clear(&mut self) -> usize {
        let n = self.items.len();
        self.items.clear();
        n
    }

    /// Remove and return the head of the queue (FIFO). `None` when empty.
    pub fn pop_front(&mut self) -> Option<String> {
        self.items.pop_front()
    }

    /// Current queue depth.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// `true` when there's nothing queued.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Read-only borrow of every queued item, in FIFO order. Used by the
    /// `/queue list` renderer and the status-bar segment.
    pub fn items(&self) -> impl Iterator<Item = &str> {
        self.items.iter().map(String::as_str)
    }

    /// Snapshot of every queued item, in FIFO order. Convenience for
    /// callers that want owned strings (e.g. for `Display` formatting
    /// outside the lock).
    pub fn snapshot(&self) -> Vec<String> {
        self.items.iter().cloned().collect()
    }

    /// Current drain strategy.
    pub fn mode(&self) -> QueueMode {
        self.mode
    }

    /// Switch the drain strategy. Existing queued items are unaffected
    /// — the new mode takes effect on the next drain call.
    pub fn set_mode(&mut self, mode: QueueMode) {
        self.mode = mode;
    }

    /// Consume queued messages according to the configured mode and
    /// return the prompt string the caller should submit, or `None`
    /// when the queue was empty.
    ///
    /// `QueueMode::Single` pops exactly one entry (FIFO); the rest stay
    /// queued for future turns. `QueueMode::Batch` drains the entire
    /// queue, joining every item with a `\n\n` separator — the empty
    /// line between messages is what most chat formats use to mark a
    /// soft boundary, and it stops the model from concatenating two
    /// unrelated thoughts into one sentence.
    pub fn drain_next(&mut self) -> Option<String> {
        match self.mode {
            QueueMode::Single => self.items.pop_front(),
            QueueMode::Batch => {
                if self.items.is_empty() {
                    return None;
                }
                let drained: Vec<String> = self.items.drain(..).collect();
                Some(drained.join("\n\n"))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_single_mode_and_empty() {
        let q = MessageQueue::new();
        assert!(q.is_empty());
        assert_eq!(q.len(), 0);
        assert_eq!(q.mode(), QueueMode::Single);
    }

    #[test]
    fn push_rejects_empty_and_whitespace() {
        let mut q = MessageQueue::new();
        assert!(!q.push(""));
        assert!(!q.push("   \t\n"));
        assert_eq!(q.len(), 0);
    }

    #[test]
    fn push_trims_surrounding_whitespace() {
        let mut q = MessageQueue::new();
        assert!(q.push("  hello world  \n"));
        assert_eq!(q.snapshot(), vec!["hello world".to_owned()]);
    }

    #[test]
    fn drain_next_single_pops_head_fifo() {
        let mut q = MessageQueue::new();
        q.push("first");
        q.push("second");
        q.push("third");

        assert_eq!(q.drain_next().as_deref(), Some("first"));
        assert_eq!(q.len(), 2);
        assert_eq!(q.drain_next().as_deref(), Some("second"));
        assert_eq!(q.drain_next().as_deref(), Some("third"));
        assert_eq!(q.drain_next(), None);
    }

    #[test]
    fn drain_next_batch_joins_with_blank_line() {
        let mut q = MessageQueue::with_mode(QueueMode::Batch);
        q.push("alpha");
        q.push("beta");
        q.push("gamma");

        let drained = q.drain_next().expect("batch drain non-empty");
        assert_eq!(drained, "alpha\n\nbeta\n\ngamma");
        assert!(q.is_empty(), "batch drain must empty the queue");
    }

    #[test]
    fn drain_next_batch_returns_none_when_empty() {
        let mut q = MessageQueue::with_mode(QueueMode::Batch);
        assert_eq!(q.drain_next(), None);
    }

    #[test]
    fn drain_next_single_with_single_item_matches_batch() {
        // A single-item queue should produce identical drain output in
        // either mode — important for predictable UX when the user toggles
        // mode with one pending message.
        let mut single = MessageQueue::with_mode(QueueMode::Single);
        let mut batch = MessageQueue::with_mode(QueueMode::Batch);
        single.push("hello");
        batch.push("hello");
        assert_eq!(single.drain_next(), batch.drain_next());
    }

    #[test]
    fn set_mode_does_not_disturb_existing_items() {
        let mut q = MessageQueue::new();
        q.push("a");
        q.push("b");
        q.set_mode(QueueMode::Batch);
        assert_eq!(q.len(), 2);
        assert_eq!(q.drain_next().as_deref(), Some("a\n\nb"));
    }

    #[test]
    fn clear_returns_count() {
        let mut q = MessageQueue::new();
        q.push("x");
        q.push("y");
        q.push("z");
        assert_eq!(q.clear(), 3);
        assert!(q.is_empty());
        assert_eq!(q.clear(), 0, "clear on empty must be a no-op returning 0");
    }

    #[test]
    fn queue_mode_parse_accepts_canonical_and_synonyms() {
        assert_eq!(QueueMode::parse("single"), Some(QueueMode::Single));
        assert_eq!(QueueMode::parse("ONE"), Some(QueueMode::Single));
        assert_eq!(QueueMode::parse("1"), Some(QueueMode::Single));
        assert_eq!(QueueMode::parse("  batch "), Some(QueueMode::Batch));
        assert_eq!(QueueMode::parse("all"), Some(QueueMode::Batch));
        assert_eq!(QueueMode::parse("merge"), Some(QueueMode::Batch));
        assert_eq!(QueueMode::parse("nonsense"), None);
        assert_eq!(QueueMode::parse(""), None);
    }

    #[test]
    fn queue_mode_label_and_display_are_consistent() {
        assert_eq!(QueueMode::Single.label(), "single");
        assert_eq!(QueueMode::Batch.label(), "batch");
        assert_eq!(format!("{}", QueueMode::Single), "single");
        assert_eq!(format!("{}", QueueMode::Batch), "batch");
    }

    #[test]
    fn snapshot_preserves_order() {
        let mut q = MessageQueue::new();
        q.push("one");
        q.push("two");
        q.push("three");
        let snap = q.snapshot();
        assert_eq!(snap, vec!["one", "two", "three"]);
        // snapshot must not have mutated the queue
        assert_eq!(q.len(), 3);
    }

    #[test]
    fn items_iterator_is_borrowed_view() {
        let mut q = MessageQueue::new();
        q.push("alpha");
        q.push("beta");
        let collected: Vec<&str> = q.items().collect();
        assert_eq!(collected, vec!["alpha", "beta"]);
        assert_eq!(q.len(), 2, "items() must not consume");
    }
}
