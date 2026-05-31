//! Automatic memory capture: at the end of a session, ask the model to extract
//! durable, reusable facts worth remembering — so the user no longer has to
//! `/remember` everything by hand (Claude Code's main memory gap).
//!
//! This module owns the two deterministic, offline-testable halves:
//! [`build_capture_prompt`] (what we ask) and [`parse_captured_facts`] (how we
//! read the reply, with confidence filtering and de-dup against existing
//! memory). The network call itself is the caller's job (any `AgentBackend`),
//! and capture is always **proposed for confirmation**, never written silently.
//!
//! Affordability — running this every session only makes sense if it is cheap.
//! `benchmarks/auto_capture_bench.py` measures recall of planted facts, the
//! false-positive rate, and the real ¥ cost per session against the live API.
//! Note that the benchmark exercises a Python mirror of [`build_capture_prompt`]
//! — those numbers characterize *this* prompt/parser, not the agent-driven
//! consolidation that currently runs at session end.
//!
//! **Status: not yet wired.** The live end-of-session pass
//! (`ReplSession::finalize_session`) reuses the existing agent-driven dream
//! consolidation, not this module. These functions are a building block kept
//! ready for the confirmation-based capture flow; until that flow is built they
//! are reachable only from tests and the benchmark.

use serde::Deserialize;

/// A fact the model proposes remembering.
#[derive(Debug, Clone, PartialEq)]
pub struct CandidateFact {
    pub text: String,
    /// One of: user | feedback | project | reference (mirrors MemoryType).
    pub category: String,
    pub confidence: f64,
}

#[derive(Deserialize)]
struct RawFact {
    #[serde(default)]
    text: String,
    #[serde(default)]
    category: String,
    #[serde(default)]
    confidence: f64,
}

/// Minimum confidence to keep a candidate. Below this the model is guessing.
pub const MIN_CONFIDENCE: f64 = 0.6;

/// Build the extraction prompt. `existing_titles` are the current memory index
/// titles so the model can skip facts already on file.
pub fn build_capture_prompt(transcript: &str, existing_titles: &[String]) -> String {
    let known = if existing_titles.is_empty() {
        "(none yet)".to_owned()
    } else {
        existing_titles
            .iter()
            .map(|t| format!("- {t}"))
            .collect::<Vec<_>>()
            .join("\n")
    };
    format!(
        "You maintain a coding agent's long-term memory. Read the session \
transcript and extract only DURABLE, REUSABLE facts worth remembering for \
future sessions: stable user preferences, project conventions, environment \
constraints, and corrections the user made.

Do NOT capture: one-off task details, transient state, file contents, \
anything already covered by existing memory, or anything you are unsure is \
durable.

Existing memory (do not duplicate these):
{known}

Return ONLY a JSON array, no prose. Each item:
  {{\"text\": \"<concise fact>\", \"category\": \"user|feedback|project|reference\", \"confidence\": 0.0-1.0}}
Return [] if nothing is worth keeping.

Transcript:
{transcript}"
    )
}

/// Parse the model reply into candidate facts: tolerate code fences and
/// surrounding prose, drop empty/low-confidence items, and de-dup against
/// existing memory titles by lexical overlap.
pub fn parse_captured_facts(reply: &str, existing_titles: &[String]) -> Vec<CandidateFact> {
    let Some(json) = extract_json_array(reply) else {
        return Vec::new();
    };
    let raw: Vec<RawFact> = serde_json::from_str(json).unwrap_or_default();

    let mut out: Vec<CandidateFact> = Vec::new();
    for f in raw {
        let text = f.text.trim().to_owned();
        if text.is_empty() || f.confidence < MIN_CONFIDENCE {
            continue;
        }
        // Skip if it lexically duplicates an existing memory title or an
        // already-accepted candidate. Existing entries are short *titles*, so
        // use the overlap coefficient (a title contained in the longer fact
        // scores ~1.0); Jaccard's union denominator would understate that.
        // Candidate-vs-candidate stays Jaccard — both are full facts of
        // comparable length.
        let dup_existing = existing_titles
            .iter()
            .any(|t| crate::memory_hygiene::overlap_coefficient(t, &text) >= 0.6);
        let dup_self = out
            .iter()
            .any(|c| crate::memory_hygiene::jaccard(&c.text, &text) >= 0.6);
        if dup_existing || dup_self {
            continue;
        }
        let category = normalize_category(&f.category);
        out.push(CandidateFact {
            text,
            category,
            confidence: f.confidence.clamp(0.0, 1.0),
        });
    }
    out
}

fn normalize_category(raw: &str) -> String {
    match raw.trim().to_ascii_lowercase().as_str() {
        "user" => "user",
        "feedback" => "feedback",
        "reference" => "reference",
        _ => "project",
    }
    .to_owned()
}

/// Pull the first top-level JSON array out of a reply that may be fenced or
/// wrapped in prose. Returns the `[...]` slice, or `None` if absent.
fn extract_json_array(reply: &str) -> Option<&str> {
    let start = reply.find('[')?;
    // Walk to the matching close bracket, respecting string literals.
    let bytes = reply.as_bytes();
    let mut depth = 0i32;
    let mut in_str = false;
    let mut escaped = false;
    for (i, &b) in bytes.iter().enumerate().skip(start) {
        if in_str {
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == b'"' {
                in_str = false;
            }
            continue;
        }
        match b {
            b'"' => in_str = true,
            b'[' => depth += 1,
            b']' => {
                depth -= 1;
                if depth == 0 {
                    return reply.get(start..=i);
                }
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_includes_transcript_and_existing_titles() {
        let p = build_capture_prompt("USER: use pnpm", &["Build is pnpm".to_owned()]);
        assert!(p.contains("USER: use pnpm"));
        assert!(p.contains("Build is pnpm"));
        assert!(p.contains("JSON array"));
    }

    #[test]
    fn parses_fenced_json_and_filters_low_confidence() {
        let reply = "Sure, here you go:\n```json\n[\
            {\"text\":\"Use pnpm not npm\",\"category\":\"project\",\"confidence\":0.9},\
            {\"text\":\"Maybe likes dark mode\",\"category\":\"user\",\"confidence\":0.3}\
        ]\n```";
        let facts = parse_captured_facts(reply, &[]);
        assert_eq!(facts.len(), 1, "low-confidence item dropped: {facts:?}");
        assert_eq!(facts[0].text, "Use pnpm not npm");
        assert_eq!(facts[0].category, "project");
    }

    #[test]
    fn dedups_against_existing_memory() {
        let reply = "[{\"text\":\"Build tool is pnpm not npm\",\"category\":\"project\",\"confidence\":0.95}]";
        let facts = parse_captured_facts(reply, &["Build tool is pnpm, never npm".to_owned()]);
        assert!(
            facts.is_empty(),
            "should skip fact already in memory: {facts:?}"
        );
    }

    #[test]
    fn dedups_short_title_contained_in_long_fact() {
        // The realistic case: existing memory is a short *title*, the candidate
        // is a longer fact that restates it. Jaccard would score ~0.3 (union
        // dominated by the long side) and let the duplicate through; the overlap
        // coefficient scores ~1.0 and correctly drops it.
        let reply = "[{\"text\":\"This project always uses the pnpm package manager; never npm or yarn.\",\"category\":\"project\",\"confidence\":0.95}]";
        let facts = parse_captured_facts(reply, &["pnpm package manager".to_owned()]);
        assert!(
            facts.is_empty(),
            "short title contained in the longer fact should dedup: {facts:?}"
        );
    }

    #[test]
    fn empty_or_garbage_reply_yields_nothing() {
        assert!(parse_captured_facts("nothing worth keeping", &[]).is_empty());
        assert!(parse_captured_facts("[]", &[]).is_empty());
        assert!(parse_captured_facts("", &[]).is_empty());
    }

    #[test]
    fn unknown_category_falls_back_to_project() {
        let reply = "[{\"text\":\"x y z fact\",\"category\":\"weird\",\"confidence\":0.8}]";
        let facts = parse_captured_facts(reply, &[]);
        assert_eq!(facts[0].category, "project");
    }
}
