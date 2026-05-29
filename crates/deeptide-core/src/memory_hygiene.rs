//! Memory hygiene: find near-duplicate and stale memory entries.
//!
//! A file-based memory index grows monotonically and rots — the same fact gets
//! written twice in different words, and entries keep naming files, flags, or
//! symbols that no longer exist. Both quietly poison retrieval (and waste the
//! cached-core budget). This module provides the detectors; surfacing/pruning
//! is left to a command so removal stays user-confirmed.
//!
//! Dependency-free and deterministic, so `tests/memory_hygiene.rs` can score
//! precision/recall offline with no API cost.

use std::collections::HashSet;
use std::path::Path;

use crate::memory_rank::tokenize;

/// Jaccard similarity of the two texts' term sets, in `[0.0, 1.0]`.
pub fn jaccard(a: &str, b: &str) -> f64 {
    let sa: HashSet<String> = tokenize(a).into_iter().collect();
    let sb: HashSet<String> = tokenize(b).into_iter().collect();
    if sa.is_empty() && sb.is_empty() {
        return 1.0;
    }
    let inter = sa.intersection(&sb).count() as f64;
    let union = sa.union(&sb).count() as f64;
    if union == 0.0 { 0.0 } else { inter / union }
}

/// A detected near-duplicate pair (indices into the input slice) and their
/// similarity.
#[derive(Debug, Clone, PartialEq)]
pub struct DuplicatePair {
    pub a: usize,
    pub b: usize,
    pub similarity: f64,
}

/// Find entry pairs whose term-set Jaccard similarity is `>= threshold`.
/// Returns each unordered pair once (`a < b`), highest similarity first.
pub fn find_duplicates(entries: &[String], threshold: f64) -> Vec<DuplicatePair> {
    let mut out = Vec::new();
    for i in 0..entries.len() {
        for j in (i + 1)..entries.len() {
            let sim = jaccard(&entries[i], &entries[j]);
            if sim >= threshold {
                out.push(DuplicatePair {
                    a: i,
                    b: j,
                    similarity: sim,
                });
            }
        }
    }
    out.sort_by(|x, y| {
        y.similarity
            .partial_cmp(&x.similarity)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(x.a.cmp(&y.a))
            .then(x.b.cmp(&y.b))
    });
    out
}

/// Extract path-like references from a memory entry: backtick-quoted spans and
/// bare tokens that look like a relative path with a file extension
/// (`Sources/Foo.swift`, `crates/x/src/y.rs`). Best-effort and conservative —
/// only flags things that clearly denote a file, to keep dead-reference
/// detection low-false-positive.
pub fn referenced_paths(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();

    // 1) backtick-quoted spans that contain a '/' and a '.'.
    let mut chars = text.char_indices().peekable();
    while let Some((_, ch)) = chars.next() {
        if ch == '`' {
            let mut span = String::new();
            for (_, c) in chars.by_ref() {
                if c == '`' {
                    break;
                }
                span.push(c);
            }
            let span = span.trim();
            if looks_like_path(span) && seen.insert(span.to_owned()) {
                out.push(span.to_owned());
            }
        }
    }

    // 2) bare whitespace-delimited tokens that look like a path.
    for raw in text.split(|c: char| c.is_whitespace() || c == '(' || c == ')' || c == ',') {
        let tok = raw.trim_matches(|c: char| {
            !c.is_alphanumeric() && c != '/' && c != '.' && c != '_' && c != '-'
        });
        if looks_like_path(tok) && seen.insert(tok.to_owned()) {
            out.push(tok.to_owned());
        }
    }
    out
}

/// A token denotes a file path if it has a directory separator and a short
/// trailing extension. Excludes URLs and bare domains.
fn looks_like_path(token: &str) -> bool {
    if token.len() < 3 || token.contains("://") || token.starts_with('/') {
        return false;
    }
    if !token.contains('/') {
        return false;
    }
    match token.rsplit('/').next().and_then(|f| f.rsplit_once('.')) {
        Some((stem, ext)) => {
            !stem.is_empty()
                && (1..=5).contains(&ext.len())
                && ext.chars().all(|c| c.is_ascii_alphanumeric())
        }
        None => false,
    }
}

/// Of the paths a memory entry references, the ones that no longer exist under
/// `root`. A non-empty result means the entry is stale by dead reference.
pub fn dead_references(text: &str, root: &Path) -> Vec<String> {
    referenced_paths(text)
        .into_iter()
        .filter(|p| !root.join(p).exists())
        .collect()
}

/// Whether an entry is stale by age: untouched for more than `max_age_days`.
pub fn is_stale_by_age(mtime_secs: f64, now_secs: f64, max_age_days: f64) -> bool {
    if mtime_secs <= 0.0 {
        return false; // unknown mtime → don't guess
    }
    (now_secs - mtime_secs) > max_age_days * 86_400.0
}
