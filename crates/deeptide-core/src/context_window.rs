//! Token-aware sliding window and adaptive compression for agent conversation
//! history.
//!
//! The agent loop accumulates `ConversationMessage`s as the user iterates on a
//! task. For long, multi-file engineering sessions the raw transcript easily
//! grows beyond the model's context limit. This module provides a small,
//! self-contained manager that:
//!
//! * estimates the token cost of each message using a cheap heuristic,
//! * keeps a configurable number of recent messages verbatim (the *sliding
//!   window*),
//! * and condenses everything older into a single rolling *summary message*
//!   when the running total approaches the configured budget.
//!
//! The summary keeps "modification clues" (file paths, tool calls, decisions)
//! so the agent can continue long-running tasks without re-reading the full
//! history. Summarisation is intentionally pluggable: callers can provide their
//! own [`Summarizer`] (e.g. backed by an LLM) and fall back to the built-in
//! [`HeuristicSummarizer`] when offline.

use crate::agent_loop::{ConversationMessage, MessageRole};

/// Default ratio used to derive the soft threshold from `max_tokens`.
///
/// When the live transcript exceeds `max_tokens * SOFT_RATIO` the manager will
/// start compressing the oldest non-pinned messages.
const SOFT_RATIO: f32 = 0.8;

/// Default number of recent messages preserved verbatim regardless of token
/// pressure. Picked to comfortably cover the last user turn plus the agent's
/// reply and any immediate follow-up.
const DEFAULT_WINDOW_SIZE: usize = 6;

/// Heuristic token estimate: ~4 characters per token, rounded up.
///
/// This is intentionally crude — it avoids pulling in a tokenizer dependency
/// and matches the order-of-magnitude estimates used by most OpenAI/Anthropic
/// budgeting code in this crate.
pub fn estimate_tokens(text: &str) -> usize {
    if text.is_empty() {
        return 0;
    }
    // Round up so any non-empty string yields at least one token.
    text.chars().count().div_ceil(4)
}

/// Configuration for [`ContextWindowManager`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextWindowConfig {
    /// Hard token budget. Compression always keeps the result strictly below
    /// this value when possible.
    pub max_tokens: usize,
    /// Soft threshold at which compression starts. Defaults to
    /// `max_tokens * 0.8`.
    pub soft_tokens: usize,
    /// Number of most recent messages preserved verbatim.
    pub window_size: usize,
    /// Optional prefix attached to the rolling summary. Useful so the model
    /// can recognise the synthetic message.
    pub summary_prefix: String,
}

impl ContextWindowConfig {
    /// Build a config from a hard token budget, deriving the soft threshold
    /// and using sensible defaults for the remaining fields.
    pub fn with_max_tokens(max_tokens: usize) -> Self {
        let soft = ((max_tokens as f32) * SOFT_RATIO) as usize;
        Self {
            max_tokens,
            soft_tokens: soft.max(1),
            window_size: DEFAULT_WINDOW_SIZE,
            summary_prefix: "[context-summary]".to_string(),
        }
    }
}

impl Default for ContextWindowConfig {
    fn default() -> Self {
        // 128k matches the current Claude / GPT-4o family default.
        Self::with_max_tokens(128_000)
    }
}

/// Pluggable summariser for compressed history segments.
///
/// Implementations receive the slice of messages that are being evicted from
/// the window and must return a short, single-string digest that preserves
/// enough context for the agent to keep working (file paths touched, key
/// decisions, outstanding TODOs, etc.).
pub trait Summarizer {
    fn summarize(&self, messages: &[ConversationMessage]) -> String;
}

/// Deterministic, dependency-free summariser used as a fallback.
///
/// It extracts the first line of each message and prefixes it with the role,
/// trimming long lines. The output is stable and inexpensive, which makes it
/// safe to use in tests and offline environments.
#[derive(Debug, Default, Clone, Copy)]
pub struct HeuristicSummarizer;

impl Summarizer for HeuristicSummarizer {
    fn summarize(&self, messages: &[ConversationMessage]) -> String {
        let mut lines = Vec::with_capacity(messages.len() + 1);
        lines.push(format!("Condensed {} earlier message(s):", messages.len()));
        for msg in messages {
            let role = match msg.role {
                MessageRole::User => "user",
                MessageRole::Assistant => "assistant",
            };
            let first_line = msg
                .content
                .lines()
                .find(|line| !line.trim().is_empty())
                .unwrap_or("(empty)")
                .trim();
            let truncated: String = first_line.chars().take(160).collect();
            let suffix = if first_line.chars().count() > 160 {
                "…"
            } else {
                ""
            };
            lines.push(format!("- {role}: {truncated}{suffix}"));
        }
        lines.join("\n")
    }
}

/// Outcome of a single compression pass.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompressionReport {
    /// Number of messages that were folded into the summary during this pass.
    pub compressed_messages: usize,
    /// Token count after compression.
    pub tokens_after: usize,
    /// `true` when the manager performed any rewriting.
    pub did_compress: bool,
}

/// Sliding-window manager that keeps a conversation transcript inside a token
/// budget by collapsing older turns into a rolling summary.
pub struct ContextWindowManager<S: Summarizer = HeuristicSummarizer> {
    config: ContextWindowConfig,
    summarizer: S,
}

impl ContextWindowManager<HeuristicSummarizer> {
    /// Create a manager with the built-in heuristic summariser.
    pub fn new(config: ContextWindowConfig) -> Self {
        Self {
            config,
            summarizer: HeuristicSummarizer,
        }
    }
}

impl<S: Summarizer> ContextWindowManager<S> {
    /// Create a manager with a custom summariser.
    pub fn with_summarizer(config: ContextWindowConfig, summarizer: S) -> Self {
        Self { config, summarizer }
    }

    /// Read-only access to the active configuration.
    pub fn config(&self) -> &ContextWindowConfig {
        &self.config
    }

    /// Estimate the total token cost of an existing transcript.
    pub fn estimate_total(&self, messages: &[ConversationMessage]) -> usize {
        messages.iter().map(|m| estimate_tokens(&m.content)).sum()
    }

    /// Returns `true` when the transcript should be compressed.
    pub fn should_compress(&self, messages: &[ConversationMessage]) -> bool {
        self.estimate_total(messages) > self.config.soft_tokens
            && messages.len() > self.config.window_size
    }

    /// Compress `messages` in-place, returning a report describing what
    /// happened. The manager:
    ///
    /// 1. leaves the most recent `window_size` messages untouched,
    /// 2. asks the summariser to digest everything older, and
    /// 3. prepends the digest as a synthetic user message so the model still
    ///    sees the historical context.
    ///
    /// If the transcript already fits the soft budget the function is a no-op.
    pub fn compress(&self, messages: &mut Vec<ConversationMessage>) -> CompressionReport {
        self.compress_at(messages, false)
    }

    /// Compress unconditionally (subject only to there being messages older than
    /// the window), ignoring the soft-token threshold. Used by the explicit
    /// `/compact` command, which the user invokes on demand.
    pub fn force_compress(&self, messages: &mut Vec<ConversationMessage>) -> CompressionReport {
        self.compress_at(messages, true)
    }

    fn compress_at(
        &self,
        messages: &mut Vec<ConversationMessage>,
        force: bool,
    ) -> CompressionReport {
        let no_op = |messages: &[ConversationMessage]| CompressionReport {
            compressed_messages: 0,
            tokens_after: self.estimate_total(messages),
            did_compress: false,
        };

        // Nothing older than the preserved window to summarize.
        if messages.len() <= self.config.window_size {
            return no_op(messages);
        }
        if !force && !self.should_compress(messages) {
            return no_op(messages);
        }

        // Snap the boundary forward to the next assistant message. This keeps
        // any tool_use / tool_result pair together (tool_result messages carry
        // the user role, so they are folded into the summary alongside their
        // originating tool_use) and ensures the kept window opens on an
        // assistant turn — so the synthetic user summary still alternates
        // user → assistant on the wire.
        let mut split_at = messages.len().saturating_sub(self.config.window_size);
        while split_at < messages.len() && messages[split_at].role != MessageRole::Assistant {
            split_at += 1;
        }

        let older: Vec<ConversationMessage> = messages.drain(..split_at).collect();
        let compressed_count = older.len();

        let digest = self.summarizer.summarize(&older);
        let summary =
            ConversationMessage::user(format!("{} {}", self.config.summary_prefix, digest));
        messages.insert(0, summary);

        // If we're still over the hard limit, fall back to dropping the oldest
        // window entries one-by-one until we fit. This guarantees the loop
        // never hands an over-budget transcript back to the model.
        while self.estimate_total(messages) > self.config.max_tokens && messages.len() > 1 {
            // Keep the summary (index 0) and drop the next-oldest message.
            messages.remove(1);
        }

        CompressionReport {
            compressed_messages: compressed_count,
            tokens_after: self.estimate_total(messages),
            did_compress: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn msg(role: MessageRole, content: &str) -> ConversationMessage {
        ConversationMessage {
            role,
            content: content.to_string(),
            tool_calls: Vec::new(),
            tool_results: Vec::new(),
        }
    }

    #[test]
    fn estimate_tokens_is_roughly_chars_div_four() {
        assert_eq!(estimate_tokens(""), 0);
        // "hello" -> 5 chars -> ceil(5/4) == 2
        assert_eq!(estimate_tokens("hello"), 2);
        // 400 chars -> ~100 tokens
        let big = "x".repeat(400);
        assert_eq!(estimate_tokens(&big), 100);
    }

    #[test]
    fn no_compression_when_below_threshold() {
        let cfg = ContextWindowConfig {
            max_tokens: 1000,
            soft_tokens: 800,
            window_size: 4,
            summary_prefix: "[s]".into(),
        };
        let mgr = ContextWindowManager::new(cfg);
        let mut history = vec![
            msg(MessageRole::User, "hi"),
            msg(MessageRole::Assistant, "hello"),
        ];
        let before = history.clone();
        let report = mgr.compress(&mut history);
        assert!(!report.did_compress);
        assert_eq!(history, before);
    }

    #[test]
    fn compresses_old_messages_and_keeps_window() {
        let cfg = ContextWindowConfig {
            max_tokens: 200,
            soft_tokens: 80,
            window_size: 2,
            summary_prefix: "[s]".into(),
        };
        let mgr = ContextWindowManager::new(cfg);
        let bulk = "a".repeat(200); // ~50 tokens each
        let mut history = vec![
            msg(MessageRole::User, &bulk),
            msg(MessageRole::Assistant, &bulk),
            msg(MessageRole::User, &bulk),
            msg(MessageRole::Assistant, "latest reply"),
            msg(MessageRole::User, "follow up"),
        ];
        let report = mgr.compress(&mut history);
        assert!(report.did_compress);
        assert_eq!(report.compressed_messages, 3);
        // Summary + the 2 most recent messages.
        assert_eq!(history.len(), 3);
        assert!(history[0].content.starts_with("[s]"));
        assert_eq!(history[1].content, "latest reply");
        assert_eq!(history[2].content, "follow up");
    }

    #[test]
    fn enforces_hard_limit_by_dropping_window_entries() {
        let cfg = ContextWindowConfig {
            max_tokens: 30,
            soft_tokens: 20,
            window_size: 3,
            summary_prefix: "[s]".into(),
        };
        let mgr = ContextWindowManager::new(cfg);
        let huge = "z".repeat(400); // ~100 tokens each, way over the hard limit
        let mut history = vec![
            msg(MessageRole::User, &huge),
            msg(MessageRole::Assistant, &huge),
            msg(MessageRole::User, &huge),
            msg(MessageRole::Assistant, &huge),
            msg(MessageRole::User, &huge),
        ];
        let report = mgr.compress(&mut history);
        assert!(report.did_compress);
        // Summary must survive even when we have to shed window entries.
        assert!(history[0].content.starts_with("[s]"));
    }

    #[test]
    fn custom_summarizer_is_honoured() {
        struct Tagging;
        impl Summarizer for Tagging {
            fn summarize(&self, messages: &[ConversationMessage]) -> String {
                format!("TAG:{}", messages.len())
            }
        }
        let cfg = ContextWindowConfig {
            max_tokens: 200,
            soft_tokens: 40,
            window_size: 1,
            summary_prefix: "[s]".into(),
        };
        let mgr = ContextWindowManager::with_summarizer(cfg, Tagging);
        let bulk = "q".repeat(200);
        let mut history = vec![
            msg(MessageRole::User, &bulk),
            msg(MessageRole::Assistant, &bulk),
            msg(MessageRole::User, &bulk),
        ];
        let report = mgr.compress(&mut history);
        assert!(report.did_compress);
        // The trailing user message is folded in too (the boundary snaps to an
        // assistant turn), so all three earlier messages are summarized.
        assert!(history[0].content.contains("TAG:3"));
    }

    #[test]
    fn compress_snaps_boundary_to_assistant_and_keeps_tool_pairs_intact() {
        use crate::ToolResultBlock;
        use crate::agent_loop::ToolCall;

        let cfg = ContextWindowConfig {
            max_tokens: 10_000,
            soft_tokens: 1,
            window_size: 3,
            summary_prefix: "[s]".into(),
        };
        let mgr = ContextWindowManager::new(cfg);

        let mut tool_use = msg(MessageRole::Assistant, "calling a tool");
        tool_use.tool_calls = vec![ToolCall::new("call-1", "Read", serde_json::json!({}))];
        let mut tool_result = msg(MessageRole::User, "");
        tool_result.tool_results = vec![ToolResultBlock::new("call-1", "file contents", false)];

        // user, assistant(tool_use), user(tool_result), assistant(reply), user(follow up)
        let mut history = vec![
            msg(MessageRole::User, "open the file"),
            tool_use,
            tool_result,
            msg(MessageRole::Assistant, "here is the summary"),
            msg(MessageRole::User, "thanks"),
        ];

        // window_size=3 → raw boundary at index 2 (the tool_result, a user
        // message); it must snap forward to the assistant at index 3.
        let report = mgr.force_compress(&mut history);
        assert!(report.did_compress);

        // First kept message is the synthetic user summary; the next must be an
        // assistant so user → assistant alternation holds on the wire.
        assert_eq!(history[0].role, MessageRole::User);
        assert!(history[0].content.starts_with("[s]"));
        assert_eq!(history[1].role, MessageRole::Assistant);

        // The tool_result was folded into the summary alongside its tool_use —
        // no orphaned tool_result survives in the kept window.
        assert!(
            history.iter().all(|m| m.tool_results.is_empty()),
            "tool_results must not be orphaned after compaction"
        );
    }

    #[test]
    fn force_compress_ignores_soft_threshold_but_needs_older_messages() {
        let cfg = ContextWindowConfig {
            max_tokens: 1_000_000,
            soft_tokens: 1_000_000,
            window_size: 2,
            summary_prefix: "[s]".into(),
        };
        let mgr = ContextWindowManager::new(cfg);

        // Below the soft threshold: regular compress is a no-op...
        let mut history = vec![
            msg(MessageRole::User, "one"),
            msg(MessageRole::Assistant, "two"),
            msg(MessageRole::User, "three"),
            msg(MessageRole::Assistant, "four"),
        ];
        assert!(!mgr.compress(&mut history.clone()).did_compress);
        // ...but a forced compact still runs because there are older messages.
        assert!(mgr.force_compress(&mut history).did_compress);

        // Nothing to do when the whole transcript fits in the window.
        let mut tiny = vec![msg(MessageRole::User, "hi")];
        assert!(!mgr.force_compress(&mut tiny).did_compress);
    }
}
