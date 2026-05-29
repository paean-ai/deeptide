//! Offline benchmark for memory hygiene (dedup + staleness). No network, no
//! API cost — scores duplicate-detection precision/recall on a labeled corpus
//! and dead-reference detection against a real temp tree.

use std::collections::HashSet;

use deeptide_core::memory_hygiene::{
    dead_references, find_duplicates, is_stale_by_age, referenced_paths,
};

/// Corpus where (0,1) and (3,4) are intentional **lexical** near-duplicates —
/// the realistic case the detector targets: the same note saved twice with a
/// small edit / copy-paste-with-tweak. Semantic-only paraphrase is explicitly
/// out of scope (see `known_limitation_semantic_paraphrase_is_missed`); that
/// needs embeddings.
fn corpus() -> Vec<String> {
    vec![
        // 0 / 1 — lexical near-dup (one word changed).
        "Build tool is pnpm; never use npm or yarn in this repo.".to_owned(),
        "Build tool is pnpm; do not use npm or yarn in this repo.".to_owned(),
        // 2 — distinct.
        "Mac app stores auth tokens in UserDefaults, not Keychain.".to_owned(),
        // 3 / 4 — lexical near-dup (reordered + minor edit).
        "Google Ads account is billed in CNY, so cost_micros is yuan not dollars.".to_owned(),
        "The Google Ads account is billed in CNY; cost_micros is yuan, not dollars.".to_owned(),
        // 5 — distinct.
        "Function Compute runs Node 20 while zero-api runs Node 22.".to_owned(),
    ]
}

fn gold_duplicate_pairs() -> HashSet<(usize, usize)> {
    [(0, 1), (3, 4)].into_iter().collect()
}

#[test]
fn dedup_precision_recall_on_labeled_pairs() {
    let entries = corpus();
    let gold = gold_duplicate_pairs();
    let threshold = 0.30;

    let found: HashSet<(usize, usize)> = find_duplicates(&entries, threshold)
        .into_iter()
        .map(|p| (p.a, p.b))
        .collect();

    let tp = found.intersection(&gold).count() as f64;
    let precision = if found.is_empty() {
        1.0
    } else {
        tp / found.len() as f64
    };
    let recall = tp / gold.len() as f64;

    println!("\n=== dedup quality (threshold {threshold}) ===");
    println!("found pairs : {found:?}");
    println!("gold pairs  : {gold:?}");
    println!("precision={precision:.2}  recall={recall:.2}");

    assert!(
        (recall - 1.0).abs() < f64::EPSILON,
        "missed a known duplicate pair: found {found:?}, gold {gold:?}"
    );
    assert!(
        precision >= 1.0 - f64::EPSILON,
        "flagged a non-duplicate: {found:?}"
    );
}

/// Documents the boundary: a purely *semantic* paraphrase (same meaning, few
/// shared words) is NOT caught by lexical Jaccard. This is expected — catching
/// it needs embeddings, tracked as future work. The test pins the limitation so
/// it's a conscious scope decision, not a silent miss.
#[test]
fn known_limitation_semantic_paraphrase_is_missed() {
    let entries = vec![
        "Push code to the dev branch first, not main.".to_owned(),
        "Default branch for commits is dev; main is reserved for promotion.".to_owned(),
    ];
    assert!(
        find_duplicates(&entries, 0.30).is_empty(),
        "lexical Jaccard is not expected to catch semantic paraphrase; \
         if this now passes, great — update the scope docs"
    );
}

#[test]
fn distinct_entries_are_not_flagged_as_duplicates() {
    // Two clearly different facts must stay below threshold.
    let entries = vec![
        "Mac app stores tokens in UserDefaults.".to_owned(),
        "Function Compute runs Node 20.".to_owned(),
    ];
    assert!(find_duplicates(&entries, 0.30).is_empty());
}

#[test]
fn dead_reference_detection_against_real_tree() {
    let temp = tempfile::tempdir().expect("tempdir");
    // A file that exists, and one that doesn't.
    std::fs::create_dir_all(temp.path().join("crates/core/src")).expect("mkdir");
    std::fs::write(temp.path().join("crates/core/src/lib.rs"), "// real").expect("write");

    let entry = "See `crates/core/src/lib.rs` for the entry point; legacy logic \
                 lived in `crates/old/src/gone.rs` which was deleted.";

    let paths = referenced_paths(entry);
    assert!(
        paths.contains(&"crates/core/src/lib.rs".to_owned()),
        "should extract the live path: {paths:?}"
    );
    assert!(
        paths.contains(&"crates/old/src/gone.rs".to_owned()),
        "should extract the dead path: {paths:?}"
    );

    let dead = dead_references(entry, temp.path());
    assert_eq!(
        dead,
        vec!["crates/old/src/gone.rs".to_owned()],
        "only the missing file should be flagged stale"
    );
}

#[test]
fn referenced_paths_ignores_urls_and_prose() {
    let text = "Visit https://example.com/docs for details about the v4 API. \
                It is 3.5 GB on disk and runs at 24 tok/s.";
    assert!(
        referenced_paths(text).is_empty(),
        "URLs / numbers / prose must not be mistaken for file paths: {:?}",
        referenced_paths(text)
    );
}

#[test]
fn staleness_by_age() {
    let now = 1_000_000_000.0;
    let sixty_days = 60.0 * 86_400.0;
    assert!(
        is_stale_by_age(now - sixty_days - 1.0, now, 60.0),
        "older than 60d → stale"
    );
    assert!(
        !is_stale_by_age(now - 1.0, now, 60.0),
        "just touched → fresh"
    );
    assert!(
        !is_stale_by_age(0.0, now, 60.0),
        "unknown mtime → not flagged"
    );
}
