//! Live end-to-end checks for the dream-consolidation persistence path, against
//! DeepSeek's real Anthropic-compatible API.
//!
//! The bug these guard: the dream prompt used to tell the agent to "save memory
//! in `.deeptide/MEMORY.md`", but the loader only reads the canonical
//! `…/projects/<slug>/memory/MEMORY.md` (and the legacy `cwd/.deeptide/memory/`
//! subdir) — so a literal write to `.deeptide/MEMORY.md` was silent data loss.
//! The prompt now routes persistence through the `MemoryWrite` tool, which lands
//! in the canonical dir. These tests drive *real* consolidations and assert the
//! kept facts round-trip back into the system-prompt memory block.
//!
//! `#[ignore]`d like `routing_live.rs`. They mutate the process-global
//! `TIDE_CONFIG_DIR`, so run single-threaded:
//!
//! ```sh
//! DEEPSEEK_API_KEY=sk-... cargo test -p deeptide-core \
//!   --test dream_consolidation_live -- --ignored --nocapture --test-threads=1
//! ```

use std::path::Path;

use deeptide_core::memory::MemorySystem;
use deeptide_core::permissions::PermissionMode;
use deeptide_core::{AnthropicBackend, AnthropicConfig, ReplEvent, ReplSession, SystemMessage};

/// DeepSeek's Anthropic-compatible base; `/v1/messages` is appended by the backend.
const DEEPSEEK_ANTHROPIC_BASE: &str = "https://api.deepseek.com/anthropic";
const DEEPSEEK_MODEL: &str = "deepseek-chat";

fn require_key() -> String {
    std::env::var("DEEPSEEK_API_KEY").expect("DEEPSEEK_API_KEY must be set")
}

/// Point the whole memory store at a throwaway dir. SAFETY (edition 2024):
/// these tests run `--test-threads=1`, so no other thread reads the env here.
fn isolate_config(config_dir: &Path) {
    unsafe {
        std::env::set_var("TIDE_CONFIG_DIR", config_dir);
    }
}

fn build_live_repl(key: &str, cwd: &Path) -> ReplSession {
    let mut config = AnthropicConfig::new(DEEPSEEK_ANTHROPIC_BASE, key, DEEPSEEK_MODEL);
    config.max_tokens = 1024;
    let backend = AnthropicBackend::new(config).expect("build AnthropicBackend");
    ReplSession::new(Box::new(backend))
        .with_cwd(cwd)
        .with_model(DEEPSEEK_MODEL)
        // No interactive approver in a test; auto-approve the MemoryWrite call.
        .with_permission_mode(PermissionMode::Bypass)
        .with_max_turns(16)
}

fn render_events(events: &[ReplEvent]) -> String {
    events
        .iter()
        .filter_map(|event| match event {
            ReplEvent::Output(text) => Some(text.clone()),
            // Surface system events (tool summaries/bodies, notices) so the
            // consolidation's MemoryWrite/MemorySearch calls are visible in
            // `--nocapture` output. Assertions read the store directly, but the
            // model's own answer text still arrives as `Output`.
            ReplEvent::System(message) => Some(render_system(message)),
            ReplEvent::Exit => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_system(message: &SystemMessage) -> String {
    match message {
        SystemMessage::ToolBatch {
            label,
            failed_count,
        } => format!("[tools] {label} (failed: {failed_count})"),
        SystemMessage::Tool {
            name,
            summary,
            body,
            ..
        } => match body {
            Some(body) => format!("[tool] {name}: {summary}\n{body}"),
            None => format!("[tool] {name}: {summary}"),
        },
        SystemMessage::Compaction {
            compressed_messages,
            tokens_after,
        } => format!("[compaction] {compressed_messages} folded, {tokens_after} tokens left"),
        SystemMessage::Notice(text) => format!("[notice] {text}"),
    }
}

/// Dump the canonical memory store (index + every shard body) for visibility,
/// and return it lowercased so callers can assert on content.
fn dump_memory_store(label: &str, project_dir: &Path) -> String {
    let dir = MemorySystem::project_memory_dir(project_dir);
    let mut rendered = format!(
        "\n===== OUTPUT: memory store after {label} =====\ndir: {}\n",
        dir.display()
    );

    let index = dir.join("MEMORY.md");
    match std::fs::read_to_string(&index) {
        Ok(text) => rendered.push_str(&format!("[MEMORY.md]\n{text}\n")),
        Err(_) => rendered.push_str("[MEMORY.md] (absent)\n"),
    }

    if let Ok(entries) = std::fs::read_dir(&dir) {
        let mut shards: Vec<_> = entries
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| {
                path.extension().and_then(|ext| ext.to_str()) == Some("md")
                    && path.file_name().and_then(|name| name.to_str()) != Some("MEMORY.md")
            })
            .collect();
        shards.sort();
        rendered.push_str(&format!("shard count: {}\n", shards.len()));
        for shard in shards {
            let name = shard
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or_default();
            let body = std::fs::read_to_string(&shard).unwrap_or_default();
            rendered.push_str(&format!("--- {name} ---\n{body}\n"));
        }
    }
    rendered.push_str("===== end =====\n");
    eprintln!("{rendered}");
    rendered.to_lowercase()
}

/// The dead path the OLD prompt named — nothing loads it, so memory must never
/// land here regardless of how consolidation was triggered.
fn assert_not_in_dead_path(project_dir: &Path) {
    let dead = project_dir.join(".deeptide").join("MEMORY.md");
    assert!(
        !dead.exists(),
        "consolidation wrote to the dead .deeptide/MEMORY.md path (nothing loads it)"
    );
}

/// Assert the canonical index exists under the isolated config dir and that the
/// system-prompt memory block (canonical index → expanded bodies) contains the
/// expected token — i.e. the fact actually round-trips back into the prompt.
fn assert_round_trips(config_dir: &Path, project_dir: &Path, expect_token: &str) -> String {
    let index = MemorySystem::project_memory_index(project_dir);
    assert!(
        index.starts_with(config_dir),
        "index {index:?} is not under the isolated config dir {config_dir:?}"
    );
    assert!(
        index.exists(),
        "canonical MEMORY.md was not created at {index:?} — consolidation did not \
         persist through MemoryWrite"
    );
    let loaded = MemorySystem::load_memory_prompt(project_dir).to_lowercase();
    assert!(
        loaded.contains(&expect_token.to_lowercase()),
        "system-prompt memory block is missing {expect_token:?}; got:\n{loaded}"
    );
    loaded
}

/// Manual `/dream run` trigger: facts stated in the session must be persisted to
/// the canonical store and round-trip into the system prompt.
#[test]
#[ignore = "live network + DEEPSEEK_API_KEY"]
fn dream_run_persists_to_canonical_and_round_trips() {
    let key = require_key();
    let config_dir = tempfile::tempdir().expect("config dir");
    let project_dir = tempfile::tempdir().expect("project dir");
    isolate_config(config_dir.path());

    let mut repl = build_live_repl(&key, project_dir.path());

    let seed = "Remember these durable project conventions: this repo ALWAYS uses pnpm, \
                never npm or yarn; and the CN backend runs on Aliyun Function Compute in \
                the cn-shanghai region.";
    eprintln!("\n===== INPUT (seed turn) =====\n{seed}\n===== INPUT (command) =====\n/dream run");
    let _ = repl.submit(seed);
    let events = repl.submit("/dream run");
    eprintln!(
        "\n===== OUTPUT (/dream run events) =====\n{}",
        render_events(&events)
    );

    let store = dump_memory_store("/dream run", project_dir.path());
    assert_not_in_dead_path(project_dir.path());
    assert_round_trips(config_dir.path(), project_dir.path(), "pnpm");
    assert!(
        store.contains("pnpm") || store.contains("cn-shanghai"),
        "neither durable fact landed in the store"
    );
}

/// The REAL shipping trigger: `finalize_session()` is what `/exit` and Ctrl-D
/// call. Drive it directly (not `/dream run`) to prove the path users actually
/// hit persists to the canonical store.
#[test]
#[ignore = "live network + DEEPSEEK_API_KEY"]
fn session_end_finalize_consolidates_via_real_model() {
    let key = require_key();
    let config_dir = tempfile::tempdir().expect("config dir");
    let project_dir = tempfile::tempdir().expect("project dir");
    isolate_config(config_dir.path());

    let mut repl = build_live_repl(&key, project_dir.path());

    let seed = "For future sessions: deployments must always run `make verify` before \
                `make ship`, and the production database is read-only from the app — all \
                writes go through the queue.";
    eprintln!(
        "\n===== INPUT (seed turn) =====\n{seed}\n===== INPUT (trigger) =====\nfinalize_session() [== /exit / Ctrl-D]"
    );
    let _ = repl.submit(seed);

    let events = repl.finalize_session();
    eprintln!(
        "\n===== OUTPUT (finalize_session events) =====\n{}",
        render_events(&events)
    );
    assert!(
        !events.is_empty(),
        "finalize_session did not run a consolidation pass (it should, after a user turn)"
    );

    dump_memory_store("finalize_session", project_dir.path());
    assert_not_in_dead_path(project_dir.path());
    // "make verify" and "queue" are the distinctive durable tokens.
    let loaded = assert_round_trips(config_dir.path(), project_dir.path(), "make verify");
    assert!(
        loaded.contains("queue") || loaded.contains("read-only"),
        "the second durable fact (read-only db / queue) did not round-trip; got:\n{loaded}"
    );

    // Idempotent: a second teardown call must not run again / duplicate.
    assert!(
        repl.finalize_session().is_empty(),
        "finalize_session must fire at most once per session"
    );
}

/// Precision: durable conventions are kept, one-off task details are dropped.
#[test]
#[ignore = "live network + DEEPSEEK_API_KEY"]
fn consolidation_keeps_durable_drops_ephemeral() {
    let key = require_key();
    let config_dir = tempfile::tempdir().expect("config dir");
    let project_dir = tempfile::tempdir().expect("project dir");
    isolate_config(config_dir.path());

    let mut repl = build_live_repl(&key, project_dir.path());

    // Neutral session notes mixing one durable convention with two clearly
    // one-off task details — let the dream prompt's own filter decide.
    let seed = "Session notes: we always use pnpm in this repo, never npm. \
                I just fixed the typo on line 42 of header.tsx. \
                I restarted the staging server because it was stuck a minute ago.";
    eprintln!("\n===== INPUT (seed turn) =====\n{seed}\n===== INPUT (command) =====\n/dream run");
    let _ = repl.submit(seed);
    let events = repl.submit("/dream run");
    eprintln!(
        "\n===== OUTPUT (/dream run events) =====\n{}",
        render_events(&events)
    );

    let store = dump_memory_store("/dream run (precision)", project_dir.path());
    assert_not_in_dead_path(project_dir.path());

    // HARD gate — this is what the PR actually changes: the durable convention
    // is kept and round-trips through the canonical path.
    assert_round_trips(config_dir.path(), project_dir.path(), "pnpm");

    // SOFT signal — whether the prompt's filter drops one-off task details is a
    // model-judgment call (e.g. deepseek-chat sometimes generalizes "I restarted
    // staging" into a durable ops rule), so it's logged, not gated. Precision is
    // measured statistically in `benchmarks/auto_capture_bench.py`; gating this
    // single live run on model nondeterminism would make the suite flaky.
    for (token, label) in [("line 42", "typo fix"), ("staging", "staging restart")] {
        if store.contains(token) {
            eprintln!(
                "[precision] note: ephemeral '{label}' detail was persisted as memory \
                 (model judgment; not a hard failure)"
            );
        }
    }
}

/// Full loop across two separate sessions: session 1 consolidates a fact to the
/// canonical store; a *fresh* session 2 (whose system prompt is rebuilt from
/// that store) recalls it when asked. This is the strongest end-to-end proof
/// that consolidated memory both lands correctly and feeds the next session's
/// prompt — the whole point of the path fix.
#[test]
#[ignore = "live network + DEEPSEEK_API_KEY"]
fn memory_survives_across_sessions_via_real_model() {
    let key = require_key();
    let config_dir = tempfile::tempdir().expect("config dir");
    let project_dir = tempfile::tempdir().expect("project dir");
    isolate_config(config_dir.path());

    // --- Session 1: state a distinctive fact, then consolidate. ---
    let seed = "Durable convention for this repo: the package manager is pnpm \
                (never npm or yarn). Please remember this for future sessions.";
    eprintln!("\n===== SESSION 1 INPUT =====\n{seed}\n+ /dream run");
    {
        let mut session1 = build_live_repl(&key, project_dir.path());
        let _ = session1.submit(seed);
        let events = session1.submit("/dream run");
        eprintln!("\n===== SESSION 1 OUTPUT =====\n{}", render_events(&events));
    }

    // Persisted to the canonical store between sessions.
    dump_memory_store("session 1", project_dir.path());
    assert_not_in_dead_path(project_dir.path());
    assert_round_trips(config_dir.path(), project_dir.path(), "pnpm");

    // --- Session 2: brand-new ReplSession over the same store. Its system
    // prompt is rebuilt from the now-populated memory, so the model should
    // recall the fact without being told again. ---
    let mut session2 = build_live_repl(&key, project_dir.path());
    let question =
        "Which package manager should I use in this repo? Reply with just the tool name.";
    eprintln!("\n===== SESSION 2 INPUT =====\n{question}");
    let answer = render_events(&session2.submit(question)).to_lowercase();
    eprintln!("\n===== SESSION 2 OUTPUT =====\n{answer}");

    assert!(
        answer.contains("pnpm"),
        "session 2 did not recall the fact consolidated in session 1; got:\n{answer}"
    );
    assert!(
        !answer.contains("npm install") && !answer.contains("use npm "),
        "session 2 recommended npm despite the stored pnpm convention; got:\n{answer}"
    );
}
