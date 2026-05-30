//! Integration tests for `/test` and `/lint` autodetection.
//!
//! We never call `--run` from the test suite: it would spawn the
//! detected tool (cargo, npm, …) inside the test fixture's tempdir,
//! which would either fail loudly or, worse, run silently for minutes
//! waiting on the network. The detection path is fully exercised by
//! the dry-run output. Direct unit tests of the runner live in the
//! `project_toolchain` module.

#![allow(clippy::unwrap_used)]

use deeptide_core::{
    AgentBackend, AgentRequest, AgentResponse, AgentUsage, ReplEvent, ReplSession,
};
use std::fs;
use tempfile::tempdir;

struct StaticBackend;

impl AgentBackend for StaticBackend {
    fn respond(&mut self, _request: AgentRequest) -> Result<AgentResponse, String> {
        Ok(AgentResponse {
            content: String::from("ok"),
            usage: Some(AgentUsage::new(1, 1, 0, 0, 0)),
            tool_calls: Vec::new(),
        })
    }
}

fn collect_text(events: Vec<ReplEvent>) -> String {
    events
        .into_iter()
        .filter_map(|event| match event {
            ReplEvent::Output(s) => Some(s),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn make_repl_in(cwd: &std::path::Path) -> ReplSession {
    ReplSession::new(Box::new(StaticBackend)).with_cwd(cwd.to_path_buf())
}

// ── Empty directory ──────────────────────────────────────────────────

#[test]
fn test_on_unknown_directory_reports_no_toolchain() {
    let dir = tempdir().unwrap();
    let mut repl = make_repl_in(dir.path());
    let text = collect_text(repl.submit("/test"));
    assert!(text.contains("No known project toolchain"), "got: {text}",);
}

#[test]
fn lint_on_unknown_directory_reports_no_toolchain() {
    let dir = tempdir().unwrap();
    let mut repl = make_repl_in(dir.path());
    let text = collect_text(repl.submit("/lint"));
    assert!(text.contains("No known project toolchain"), "got: {text}",);
}

// ── Rust autodetection ───────────────────────────────────────────────

#[test]
fn test_on_cargo_workspace_suggests_cargo_test() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("Cargo.toml"), "[workspace]").unwrap();
    let mut repl = make_repl_in(dir.path());
    let text = collect_text(repl.submit("/test"));
    assert!(text.contains("Rust (Cargo)"), "got: {text}");
    assert!(text.contains("cargo test"), "got: {text}");
    assert!(text.contains("--workspace"), "got: {text}");
    assert!(text.contains("/test --run"), "got: {text}");
}

#[test]
fn lint_on_cargo_workspace_suggests_cargo_clippy_dash_d_warnings() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("Cargo.toml"), "[workspace]").unwrap();
    let mut repl = make_repl_in(dir.path());
    let text = collect_text(repl.submit("/lint"));
    assert!(text.contains("cargo clippy"), "got: {text}");
    assert!(text.contains("-D"), "got: {text}");
    assert!(text.contains("warnings"), "got: {text}");
}

// ── Node autodetection ───────────────────────────────────────────────

#[test]
fn test_on_pnpm_repo_suggests_pnpm_test() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("package.json"), "{}").unwrap();
    fs::write(dir.path().join("pnpm-lock.yaml"), "").unwrap();
    let mut repl = make_repl_in(dir.path());
    let text = collect_text(repl.submit("/test"));
    assert!(text.contains("Node"), "got: {text}");
    assert!(text.contains("pnpm test"), "got: {text}");
}

#[test]
fn lint_on_bun_repo_suggests_bun_run_lint() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("package.json"), "{}").unwrap();
    fs::write(dir.path().join("bun.lockb"), "").unwrap();
    let mut repl = make_repl_in(dir.path());
    let text = collect_text(repl.submit("/lint"));
    assert!(text.contains("bun run lint"), "got: {text}");
}

// ── Python / Go / Ruby ───────────────────────────────────────────────

#[test]
fn test_on_pyproject_suggests_pytest() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("pyproject.toml"), "[project]").unwrap();
    let mut repl = make_repl_in(dir.path());
    let text = collect_text(repl.submit("/test"));
    assert!(text.contains("Python"), "got: {text}");
    assert!(text.contains("pytest"), "got: {text}");
}

#[test]
fn lint_on_pyproject_suggests_ruff_check() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("pyproject.toml"), "[project]").unwrap();
    let mut repl = make_repl_in(dir.path());
    let text = collect_text(repl.submit("/lint"));
    assert!(text.contains("ruff check"), "got: {text}");
}

#[test]
fn test_on_go_module_suggests_go_test() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("go.mod"), "module x").unwrap();
    let mut repl = make_repl_in(dir.path());
    let text = collect_text(repl.submit("/test"));
    assert!(text.contains("Go modules"), "got: {text}");
    assert!(text.contains("go test ./..."), "got: {text}");
}

// ── Polyglot repo hints ──────────────────────────────────────────────

#[test]
fn polyglot_repo_picks_first_and_lists_others() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("Cargo.toml"), "[workspace]").unwrap();
    fs::write(dir.path().join("package.json"), "{}").unwrap();
    let mut repl = make_repl_in(dir.path());
    let text = collect_text(repl.submit("/test"));
    assert!(text.contains("cargo test"), "got: {text}");
    assert!(text.contains("Also detected"), "got: {text}");
    assert!(text.contains("Node"), "got: {text}");
}

// ── Help + unknown flag ──────────────────────────────────────────────

#[test]
fn test_help_documents_detection_markers() {
    let dir = tempdir().unwrap();
    let mut repl = make_repl_in(dir.path());
    let text = collect_text(repl.submit("/test --help"));
    for needle in [
        "Cargo.toml",
        "package.json",
        "pyproject.toml",
        "go.mod",
        "Gemfile",
        "--run",
    ] {
        assert!(
            text.contains(needle),
            "expected help text to mention {needle:?}: {text}",
        );
    }
}

#[test]
fn lint_help_documents_detection_markers() {
    let dir = tempdir().unwrap();
    let mut repl = make_repl_in(dir.path());
    let text = collect_text(repl.submit("/lint --help"));
    assert!(text.contains("Cargo.toml"), "got: {text}");
}

#[test]
fn test_rejects_unknown_flag() {
    let dir = tempdir().unwrap();
    let mut repl = make_repl_in(dir.path());
    let text = collect_text(repl.submit("/test --frobnicate"));
    assert!(text.contains("Unknown flag"), "got: {text}");
}
