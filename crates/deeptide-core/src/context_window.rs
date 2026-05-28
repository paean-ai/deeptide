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
    // Round up. `+ 3` guarantees a non-zero estimate for short strings.
    (text.chars().count() + 3) / 4
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
        if !self.should_compress(messages) {
            return CompressionReport {
                compressed_messages: 0,
                tokens_after: self.estimate_total(messages),
                did_compress: false,
            };
        }

        let split_at = messages.len().saturating_sub(self.config.window_size);
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
        assert!(history[0].content.contains("TAG:2"));
    }
}
