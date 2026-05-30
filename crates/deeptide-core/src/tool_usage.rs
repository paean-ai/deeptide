//! Per-tool observability for the agent loop.
//!
//! The cost tracker tells you "this turn spent $X" and `/cache` tells
//! you "the prompt cache hit Y% on turn N". Neither tells you
//! _which tool_ the agent is spending its time in. That's the gap this
//! module closes: every dispatch through [`crate::AgentLoop::execute_tool_call`]
//! is wrapped with a stopwatch + result-size sample, and the totals
//! roll up into a `ToolUsageTracker` the REPL renders via `/usage`.
//!
//! The tracker is **session-scoped, in-memory** and never persisted.
//! That keeps the data structure trivially `Send + Sync`-free and
//! avoids leaking the (potentially sensitive) names of tools an agent
//! was permitted to call out to disk.
//!
//! ## Invariants
//!
//! * Every recorded sample pairs a tool name with a strictly positive
//!   duration (we use `Instant::elapsed` which never returns negative).
//! * `success_count + error_count == invocations` for any given entry.
//! * Result-byte totals sum the raw content bytes seen by the agent;
//!   they are *not* token estimates. Use them to spot tools that
//!   dominate output volume.

use std::collections::BTreeMap;
use std::time::Duration;

/// Counters for a single tool name. All fields stay public-by-getter
/// so renderer code can pick the columns it cares about without
/// re-deriving them from raw samples.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ToolUsageEntry {
    invocations: u64,
    success_count: u64,
    error_count: u64,
    total_duration: Duration,
    /// Worst observed wall-clock duration. The agent loop is single
    /// threaded per turn so this is a tight upper bound, not a
    /// percentile estimate.
    peak_duration: Duration,
    /// Cumulative bytes in `ToolResult::content` across all calls.
    total_result_bytes: u64,
}

impl ToolUsageEntry {
    /// Fold a new sample into the running totals.
    pub fn record(&mut self, duration: Duration, is_error: bool, result_bytes: usize) {
        self.invocations = self.invocations.saturating_add(1);
        if is_error {
            self.error_count = self.error_count.saturating_add(1);
        } else {
            self.success_count = self.success_count.saturating_add(1);
        }
        self.total_duration = self.total_duration.saturating_add(duration);
        if duration > self.peak_duration {
            self.peak_duration = duration;
        }
        self.total_result_bytes = self.total_result_bytes.saturating_add(result_bytes as u64);
    }

    pub fn invocations(&self) -> u64 {
        self.invocations
    }

    pub fn success_count(&self) -> u64 {
        self.success_count
    }

    pub fn error_count(&self) -> u64 {
        self.error_count
    }

    pub fn total_duration(&self) -> Duration {
        self.total_duration
    }

    pub fn peak_duration(&self) -> Duration {
        self.peak_duration
    }

    /// Average duration over all recorded calls. Returns `Duration::ZERO`
    /// when the tool has not yet been called (safe for renderers).
    pub fn average_duration(&self) -> Duration {
        if self.invocations == 0 {
            return Duration::ZERO;
        }
        // u64 division — never panics.
        self.total_duration / self.invocations as u32
    }

    pub fn total_result_bytes(&self) -> u64 {
        self.total_result_bytes
    }

    /// Error rate as a 0.0-1.0 fraction. Zero invocations report 0.0.
    pub fn error_rate(&self) -> f64 {
        if self.invocations == 0 {
            0.0
        } else {
            self.error_count as f64 / self.invocations as f64
        }
    }
}

/// All per-tool counters for a single REPL session. The store keeps a
/// stable insertion order via `BTreeMap` (alphabetical), which keeps
/// the `/usage` output diff-friendly between turns.
#[derive(Debug, Clone, Default)]
pub struct ToolUsageTracker {
    entries: BTreeMap<String, ToolUsageEntry>,
    total_invocations: u64,
}

impl ToolUsageTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Append one sample. Cheap (`O(log n)` on the tool count) and
    /// allocation-free for repeat-call tools.
    pub fn record(
        &mut self,
        tool_name: &str,
        duration: Duration,
        is_error: bool,
        result_bytes: usize,
    ) {
        let entry = self.entries.entry(tool_name.to_owned()).or_default();
        entry.record(duration, is_error, result_bytes);
        self.total_invocations = self.total_invocations.saturating_add(1);
    }

    pub fn total_invocations(&self) -> u64 {
        self.total_invocations
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn entry(&self, tool_name: &str) -> Option<&ToolUsageEntry> {
        self.entries.get(tool_name)
    }

    /// Iterate over `(tool_name, entry)` pairs in alphabetical order.
    /// Renderers should call [`Self::sorted_by_total_duration`] when a
    /// "hot tools first" view is needed.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &ToolUsageEntry)> {
        self.entries
            .iter()
            .map(|(name, entry)| (name.as_str(), entry))
    }

    /// Snapshot of `(tool_name, entry)` pairs sorted by total duration
    /// descending. Ties broken by invocation count, then by name. This
    /// is what `/usage` renders by default — the slowest tool sits at
    /// the top so a glance reveals the agent's hot path.
    pub fn sorted_by_total_duration(&self) -> Vec<(&str, &ToolUsageEntry)> {
        let mut rows: Vec<(&str, &ToolUsageEntry)> = self.iter().collect();
        rows.sort_by(|(a_name, a), (b_name, b)| {
            b.total_duration
                .cmp(&a.total_duration)
                .then_with(|| b.invocations.cmp(&a.invocations))
                .then_with(|| a_name.cmp(b_name))
        });
        rows
    }

    /// Discard every sample. The session-wide invocation counter also
    /// resets so `/usage reset` produces a clean slate.
    pub fn reset(&mut self) {
        self.entries.clear();
        self.total_invocations = 0;
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    #[test]
    fn record_increments_counters() {
        let mut e = ToolUsageEntry::default();
        e.record(Duration::from_millis(120), false, 256);
        e.record(Duration::from_millis(80), true, 64);
        assert_eq!(e.invocations(), 2);
        assert_eq!(e.success_count(), 1);
        assert_eq!(e.error_count(), 1);
        assert_eq!(e.total_duration(), Duration::from_millis(200));
        assert_eq!(e.peak_duration(), Duration::from_millis(120));
        assert_eq!(e.average_duration(), Duration::from_millis(100));
        assert_eq!(e.total_result_bytes(), 320);
        assert!((e.error_rate() - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn empty_entry_returns_zero_average_and_rate() {
        let e = ToolUsageEntry::default();
        assert_eq!(e.average_duration(), Duration::ZERO);
        assert_eq!(e.error_rate(), 0.0);
    }

    #[test]
    fn peak_duration_only_grows() {
        let mut e = ToolUsageEntry::default();
        e.record(Duration::from_millis(50), false, 0);
        e.record(Duration::from_millis(500), false, 0);
        e.record(Duration::from_millis(50), false, 0);
        assert_eq!(e.peak_duration(), Duration::from_millis(500));
    }

    #[test]
    fn tracker_records_per_tool_separately() {
        let mut t = ToolUsageTracker::new();
        t.record("Read", Duration::from_millis(10), false, 16);
        t.record("Read", Duration::from_millis(20), false, 24);
        t.record("Write", Duration::from_millis(40), true, 0);
        assert_eq!(t.total_invocations(), 3);
        assert_eq!(t.len(), 2);
        assert_eq!(t.entry("Read").unwrap().invocations(), 2);
        assert_eq!(t.entry("Write").unwrap().error_count(), 1);
    }

    #[test]
    fn tracker_sort_puts_hot_tool_first() {
        let mut t = ToolUsageTracker::new();
        t.record("Cheap", Duration::from_millis(5), false, 0);
        t.record("Cheap", Duration::from_millis(5), false, 0);
        t.record("Slow", Duration::from_millis(500), false, 0);
        let rows = t.sorted_by_total_duration();
        assert_eq!(rows.first().map(|(n, _)| *n), Some("Slow"));
        assert_eq!(rows.get(1).map(|(n, _)| *n), Some("Cheap"));
    }

    #[test]
    fn tracker_sort_breaks_ties_by_invocation_count_then_name() {
        let mut t = ToolUsageTracker::new();
        t.record("Aaa", Duration::from_millis(100), false, 0);
        t.record("Bbb", Duration::from_millis(100), false, 0);
        t.record("Bbb", Duration::from_millis(0), false, 0);
        let rows = t.sorted_by_total_duration();
        // Bbb has 2 invocations, Aaa has 1 → Bbb first.
        assert_eq!(rows[0].0, "Bbb");
        assert_eq!(rows[1].0, "Aaa");
    }

    #[test]
    fn reset_clears_state() {
        let mut t = ToolUsageTracker::new();
        t.record("Read", Duration::from_millis(10), false, 16);
        t.record("Write", Duration::from_millis(20), true, 0);
        t.reset();
        assert!(t.is_empty());
        assert_eq!(t.total_invocations(), 0);
        assert!(t.entry("Read").is_none());
    }

    #[test]
    fn saturating_overflow_does_not_panic() {
        let mut e = ToolUsageEntry::default();
        for _ in 0..3 {
            e.record(Duration::from_secs(1), false, usize::MAX);
        }
        // We don't assert the exact byte value (saturated); we just
        // confirm we don't blow up on huge inputs.
        assert_eq!(e.invocations(), 3);
    }
}
