//! Offline retrieval-quality benchmark for memory ranking.
//!
//! No network, no API cost — measures MRR and precision@1 of BM25+recency
//! ranking against the old substring baseline on a labeled fixture (query →
//! the entry id that should rank first). Asserts the ranker is materially
//! better, so a regression that flattens ranking back to substring order
//! fails CI.

use deeptide_core::memory_rank::{RankDoc, rank, substring_baseline};

/// A realistic memory corpus (mirrors the kind of entries in MEMORY.md).
fn corpus() -> Vec<RankDoc> {
    let entries = [
        (
            "fc-deploy",
            "FC deployment mapping. api.paeanone.com uses paean-one-service-function in cn-shanghai, not paean-cn-api in cn-hangzhou.",
        ),
        (
            "ring-backend",
            "Ring backend mapping. Ring iOS INTL routes to zero-api; CN routes to api-paeanone-com via Aliyun FC cn-shanghai.",
        ),
        (
            "retention-metric",
            "Retention metric gotcha. Do not use device_sessions.lastActiveAt for retention; it is updated in place and understates by ~10x. Use event-union activation anchoring.",
        ),
        (
            "google-ads-cny",
            "Google Ads is CNY. The PaeanAI ads account is billed in CNY; cost_micros is yuan not dollars, so the report overstates USD ~7x.",
        ),
        (
            "commit-ask",
            "Ask before commit and push. Prior approval does not extend to follow-up commits or sibling repos; ask again every time.",
        ),
        (
            "push-dev-first",
            "Push to dev first, not main. Default branch target for code commits is dev; main is reserved for promoted changes.",
        ),
        (
            "fc-node-runtime",
            "FC Node runtime mismatch. Function Compute runs Node 20 from the console layer; zero-api runs Node 22. ESM-only deps crash ERR_REQUIRE_ESM on FC.",
        ),
        (
            "keychain-tokens",
            "No Keychain tokens. The Mac app stores auth tokens in UserDefaults, not Keychain, though the class is still named KeychainService.",
        ),
        (
            "mac-rebuild",
            "Rebuild Mac app after each change. Run make local and relaunch between iterations or you review stale visuals.",
        ),
        (
            "typography-tokens",
            "Design system token adoption. Prefer PaeanTypography tokens over raw Font.system sizes in SwiftUI.",
        ),
        (
            "cn-target-flag",
            "PaeanAppCN needs CN_TARGET flag. All four CN iOS targets need SWIFT_ACTIVE_COMPILATION_CONDITIONS CN_TARGET or they ship pointing at the INTL backend.",
        ),
        (
            "credits-tier",
            "Credits vs subscription tier. The /credits/status endpoint is authoritative for tier; /subscription/current can lag and show free for premium users.",
        ),
    ];
    let n = entries.len();
    entries
        .iter()
        .enumerate()
        .map(|(i, (id, text))| RankDoc {
            id: (*id).to_owned(),
            text: (*text).to_owned(),
            // oldest first in the array → newest last; spread recency across 0..1.
            recency: i as f64 / (n as f64 - 1.0),
        })
        .collect()
}

/// (query, id that should rank first).
fn labeled_queries() -> Vec<(&'static str, &'static str)> {
    vec![
        ("which branch do I push code to", "push-dev-first"),
        (
            "retention metric understated lastActiveAt",
            "retention-metric",
        ),
        ("where does the Mac app store tokens", "keychain-tokens"),
        ("are ad costs in dollars or yuan", "google-ads-cny"),
        ("CN iOS target compilation flag backend", "cn-target-flag"),
        (
            "node runtime version on function compute",
            "fc-node-runtime",
        ),
        (
            "is subscription tier authoritative or credits",
            "credits-tier",
        ),
        ("do I need to ask before committing", "commit-ask"),
        ("which font tokens to use in SwiftUI", "typography-tokens"),
        ("api.paeanone.com which function and region", "fc-deploy"),
    ]
}

fn id_at(docs: &[RankDoc], idx: usize) -> &str {
    &docs[idx].id
}

/// Reciprocal rank of the gold id in a ranked id list (0 if absent).
fn reciprocal_rank(ranked_ids: &[&str], gold: &str) -> f64 {
    ranked_ids
        .iter()
        .position(|id| *id == gold)
        .map(|p| 1.0 / (p as f64 + 1.0))
        .unwrap_or(0.0)
}

#[test]
fn bm25_beats_substring_baseline_on_labeled_queries() {
    let docs = corpus();
    let queries = labeled_queries();

    let mut bm25_mrr = 0.0;
    let mut base_mrr = 0.0;
    let mut bm25_p1 = 0usize;
    let mut base_p1 = 0usize;

    for (query, gold) in &queries {
        let ranked: Vec<&str> = rank(query, &docs, docs.len())
            .into_iter()
            .map(|(i, _)| id_at(&docs, i))
            .collect();
        let base: Vec<&str> = substring_baseline(query, &docs, docs.len())
            .into_iter()
            .map(|i| id_at(&docs, i))
            .collect();

        bm25_mrr += reciprocal_rank(&ranked, gold);
        base_mrr += reciprocal_rank(&base, gold);
        if ranked.first() == Some(gold) {
            bm25_p1 += 1;
        }
        if base.first() == Some(gold) {
            base_p1 += 1;
        }
    }

    let q = queries.len() as f64;
    bm25_mrr /= q;
    base_mrr /= q;

    println!(
        "\n=== memory retrieval quality ({} queries) ===",
        queries.len()
    );
    println!(
        "baseline (substring)  MRR={base_mrr:.3}  precision@1={base_p1}/{}",
        queries.len()
    );
    println!(
        "ranked   (BM25+rec)   MRR={bm25_mrr:.3}  precision@1={bm25_p1}/{}",
        queries.len()
    );

    assert!(
        bm25_mrr > base_mrr + 0.15,
        "BM25 MRR ({bm25_mrr:.3}) not materially better than substring ({base_mrr:.3})"
    );
    assert!(
        bm25_p1 >= base_p1,
        "BM25 precision@1 ({bm25_p1}) regressed below substring ({base_p1})"
    );
    // BM25 should put the right entry first most of the time.
    assert!(
        bm25_p1 * 2 >= queries.len(),
        "BM25 precision@1 too low: {bm25_p1}/{}",
        queries.len()
    );
}

#[test]
fn irrelevant_query_returns_nothing() {
    let docs = corpus();
    let ranked = rank("zzz quux nonexistent topic", &docs, 10);
    assert!(ranked.is_empty(), "should not surface unrelated entries");
}

/// A document that overlaps the query only on stopwords (`the`/`is`/`on`) must
/// not surface — even with maximal recency — while the document sharing the one
/// content word does. Without query-side stopword filtering the stopword-only
/// doc would score > 0 and leak in (this test fails on that regression).
#[test]
fn stopword_only_overlap_does_not_surface() {
    let docs = vec![
        RankDoc {
            id: "content-match".to_owned(),
            text: "Vectorize embeddings live in the index.".to_owned(),
            recency: 0.0,
        },
        RankDoc {
            id: "stopword-only".to_owned(),
            text: "The build is on the runner over there.".to_owned(),
            recency: 1.0,
        },
    ];
    let ids: Vec<&str> = rank("what is the vectorize on the disk", &docs, 10)
        .into_iter()
        .map(|(i, _)| docs[i].id.as_str())
        .collect();
    assert_eq!(
        ids,
        vec!["content-match"],
        "stopword-only overlap must not surface a document; got {ids:?}"
    );
}

#[test]
fn ranking_is_deterministic() {
    let docs = corpus();
    let a = rank("which branch do I push to", &docs, 5);
    let b = rank("which branch do I push to", &docs, 5);
    assert_eq!(a, b);
}
