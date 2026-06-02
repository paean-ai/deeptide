//! Live end-to-end check for `--as memory` session import against DeepSeek's
//! real API: plant a fake Claude session, distil it via the import consolidation
//! pass, and assert the durable fact lands in deeptide's canonical memory store
//! (reachable by future sessions). Mirrors `dream_consolidation_live`.
//!
//! `#[ignore]`d; mutates process-global env, so run single-threaded:
//!
//! ```sh
//! DEEPSEEK_API_KEY=sk-… cargo test -p deeptide-core \
//!   --test import_sessions_live -- --ignored --nocapture --test-threads=1
//! ```

use std::path::Path;

use deeptide_core::memory::MemorySystem;
use deeptide_core::permissions::PermissionMode;
use deeptide_core::{AnthropicBackend, AnthropicConfig, ReplEvent, ReplSession};

const DEEPSEEK_ANTHROPIC_BASE: &str = "https://api.deepseek.com/anthropic";
const DEEPSEEK_MODEL: &str = "deepseek-chat";

fn require_key() -> String {
    std::env::var("DEEPSEEK_API_KEY").expect("DEEPSEEK_API_KEY must be set")
}

fn claude_slug(cwd: &Path) -> String {
    cwd.to_string_lossy().replace('/', "-")
}

#[test]
#[ignore = "live network + DEEPSEEK_API_KEY"]
fn import_as_memory_distills_a_claude_session_to_canonical_store() {
    let key = require_key();
    let home = tempfile::tempdir().expect("home");
    let config_dir = tempfile::tempdir().expect("config dir");
    let project = tempfile::tempdir().expect("project");

    // SAFETY: single-threaded test run; no concurrent readers of these env vars.
    unsafe {
        std::env::set_var("HOME", home.path());
        std::env::set_var("TIDE_CONFIG_DIR", config_dir.path());
    }

    // Plant a Claude session carrying a distinctive durable fact.
    let slug = claude_slug(project.path());
    let claude_dir = home.path().join(".claude").join("projects").join(&slug);
    std::fs::create_dir_all(&claude_dir).expect("claude dir");
    let session = r#"{"type":"ai-title","aiTitle":"Set up the CN deploy","sessionId":"deadbeef"}
{"type":"user","message":{"role":"user","content":"Important convention for this repo: the CN backend deploys to Aliyun Function Compute in the cn-shanghai region. Remember this."}}
{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"Noted — CN backend deploys to Aliyun FC, cn-shanghai."}]}}
{"type":"user","message":{"role":"user","content":"Also we always use pnpm here, never npm."}}
{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"Understood, pnpm only."}]}}
"#;
    std::fs::write(claude_dir.join("deadbeef.jsonl"), session).expect("write session");

    let mut config = AnthropicConfig::new(DEEPSEEK_ANTHROPIC_BASE, &key, DEEPSEEK_MODEL);
    config.max_tokens = 1024;
    let backend = AnthropicBackend::new(config).expect("backend");
    let mut repl = ReplSession::new(Box::new(backend))
        .with_cwd(project.path())
        .with_model(DEEPSEEK_MODEL)
        .with_permission_mode(PermissionMode::Bypass)
        .with_max_turns(16);

    eprintln!("\n===== INPUT: /import claude --as memory =====");
    let events = repl.run_import("claude --as memory");
    let rendered: String = events
        .into_iter()
        .filter_map(|e| match e {
            ReplEvent::Output(t) => Some(t),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n");
    eprintln!("\n===== OUTPUT =====\n{rendered}");

    // The distilled facts must land in the canonical memory store and round-trip
    // back into the system prompt a future session would load.
    let dir = MemorySystem::project_memory_dir(project.path());
    let mut store = String::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for e in entries.flatten() {
            store.push_str(&std::fs::read_to_string(e.path()).unwrap_or_default());
            store.push('\n');
        }
    }
    let store = store.to_lowercase();
    eprintln!("\n===== STORE under {} =====\n{store}", dir.display());

    assert!(
        store.contains("cn-shanghai") || store.contains("aliyun") || store.contains("pnpm"),
        "no durable fact from the imported session landed in the canonical store:\n{store}"
    );
    let loaded = MemorySystem::load_memory_prompt(project.path()).to_lowercase();
    assert!(
        loaded.contains("cn-shanghai") || loaded.contains("aliyun") || loaded.contains("pnpm"),
        "imported facts did not round-trip into the system-prompt memory block:\n{loaded}"
    );
}
