//! Relevance ranking for memory retrieval.
//!
//! `MemorySearch` historically returned every memory file whose text contained
//! the query as a lowercase substring, ordered by scope then path — i.e. no
//! relevance signal at all, so the most useful entry could sit tenth. This
//! module ranks candidates with BM25 (the standard sparse-retrieval scorer)
//! plus a mild recency boost, so the agent sees the entry that actually matches
//! first.
//!
//! It is dependency-free and deterministic, so `tests/memory_rank.rs` can
//! measure retrieval quality (MRR / precision@k) offline against a labeled
//! fixture — no API calls, no cost.

/// A retrievable memory document and how recently it was touched.
#[derive(Debug, Clone)]
pub struct RankDoc {
    /// Stable identifier (e.g. file name) returned in results.
    pub id: String,
    /// Full searchable text: title + description + body.
    pub text: String,
    /// Recency in `[0.0, 1.0]`, where `1.0` is the most recently modified
    /// entry and `0.0` the oldest. Used only as a tie-breaking nudge.
    pub recency: f64,
}

/// Split text into lowercase alphanumeric terms, dropping 1-char noise.
pub fn tokenize(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    for ch in text.chars() {
        if ch.is_alphanumeric() {
            cur.extend(ch.to_lowercase());
        } else if !cur.is_empty() {
            if cur.len() > 1 {
                out.push(std::mem::take(&mut cur));
            } else {
                cur.clear();
            }
        }
    }
    if cur.len() > 1 {
        out.push(cur);
    }
    out
}

/// BM25 weight given to the recency nudge. Deliberately tiny: recency is
/// normalized relative to the matched set, so a sub-second mtime gap can span
/// the full `[0, 1]` range. Keeping the weight well below a single matched
/// term's BM25 contribution ensures recency only breaks near-ties and never
/// outranks a clearly better textual match.
const RECENCY_WEIGHT: f64 = 0.1;
const K1: f64 = 1.5;
const B: f64 = 0.75;

/// Rank `docs` against `query`, returning `(index, score)` for the top `k`,
/// best first. Documents that share no query term score 0 and are dropped —
/// matching the old substring behaviour of never returning irrelevant files,
/// but now *ordered* by how well they match.
pub fn rank(query: &str, docs: &[RankDoc], k: usize) -> Vec<(usize, f64)> {
    if docs.is_empty() {
        return Vec::new();
    }
    let query_terms = tokenize(query);
    if query_terms.is_empty() {
        return Vec::new();
    }

    let doc_terms: Vec<Vec<String>> = docs.iter().map(|d| tokenize(&d.text)).collect();
    let doc_len: Vec<f64> = doc_terms.iter().map(|t| t.len() as f64).collect();
    let avg_len = doc_len.iter().sum::<f64>() / doc_len.len() as f64;
    let n = docs.len() as f64;

    // Inverse document frequency per query term, computed once (a term's df is
    // doc-independent, so recomputing it per document was O(D²·Q) for no gain).
    let idf: Vec<f64> = query_terms
        .iter()
        .map(|q| {
            let df = doc_terms
                .iter()
                .filter(|d| d.iter().any(|t| t == q))
                .count() as f64;
            // BM25 idf (with the +1 guard so it never goes negative).
            (1.0 + (n - df + 0.5) / (df + 0.5)).ln()
        })
        .collect();

    let mut scored: Vec<(usize, f64)> = Vec::new();
    for (i, terms) in doc_terms.iter().enumerate() {
        let mut score = 0.0;
        for (qi, q) in query_terms.iter().enumerate() {
            let tf = terms.iter().filter(|t| *t == q).count() as f64;
            if tf == 0.0 {
                continue;
            }
            let denom = tf + K1 * (1.0 - B + B * doc_len[i] / avg_len.max(1.0));
            score += idf[qi] * (tf * (K1 + 1.0)) / denom;
        }
        if score > 0.0 {
            scored.push((i, score + RECENCY_WEIGHT * docs[i].recency));
        }
    }

    // Best score first; stable tie-break on index for determinism.
    scored.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.0.cmp(&b.0))
    });
    scored.truncate(k);
    scored
}

/// Convenience: the substring-only baseline the ranker replaces, kept so the
/// benchmark can measure the improvement against the old behaviour.
pub fn substring_baseline(query: &str, docs: &[RankDoc], k: usize) -> Vec<usize> {
    let needle = query.to_lowercase();
    // The old behaviour matched on the whole query string OR any term; emulate
    // the term-any variant (more generous) and keep input order (scope/path).
    let terms = tokenize(query);
    docs.iter()
        .enumerate()
        .filter(|(_, d)| {
            let hay = d.text.to_lowercase();
            hay.contains(&needle) || terms.iter().any(|t| hay.contains(t))
        })
        .map(|(i, _)| i)
        .take(k)
        .collect()
}
