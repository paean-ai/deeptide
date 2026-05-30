//! Build script: captures git provenance so `deeptide-rs --version` is
//! actually meaningful (commit hash + commit date + dirty-tree flag),
//! not just the static `0.1.0` from Cargo.toml.
//!
//! All emitted variables are optional — when the crate is built outside
//! a git checkout (e.g. from a crates.io tarball or a release archive)
//! every probe falls back to `"unknown"` and the resulting binary still
//! builds & runs, just with a less informative `--version`.

use std::path::Path;
use std::process::Command;

fn main() {
    let workspace_root = locate_workspace_root();

    let git_hash = probe_git(&workspace_root, &["rev-parse", "--short=12", "HEAD"])
        .unwrap_or_else(|| String::from("unknown"));

    // A "-dirty" suffix on the hash gives a strong visual hint that the
    // installed binary doesn't correspond to any pushed commit. We never
    // mark "unknown" as dirty because the dirty probe would have run from
    // the same workspace and we can't trust its result either.
    let dirty_suffix = if git_hash == "unknown" {
        String::new()
    } else if probe_git(&workspace_root, &["status", "--porcelain"])
        .map(|out| !out.is_empty())
        .unwrap_or(false)
    {
        String::from("-dirty")
    } else {
        String::new()
    };

    // Commit date in YYYY-MM-DD UTC. Using the commit date (not the build
    // wall-clock) means two CI builds of the same commit produce identical
    // version strings, which is what you want for reproducibility-style
    // debugging — "are we running the same code as origin/deeptide-rs HEAD?"
    let git_date = probe_git(
        &workspace_root,
        &["log", "-1", "--format=%cd", "--date=short"],
    )
    .unwrap_or_else(|| String::from("unknown"));

    let git_branch = probe_git(&workspace_root, &["rev-parse", "--abbrev-ref", "HEAD"])
        .unwrap_or_else(|| String::from("unknown"));

    // Toolchain stamp — useful when triaging "works on my machine" builds
    // across Rust versions. Cargo always sets RUSTC for build scripts, but
    // we still defensively fall back to a constant.
    let rustc_version = std::env::var("RUSTC")
        .ok()
        .and_then(|rustc| {
            Command::new(rustc)
                .arg("--version")
                .output()
                .ok()
                .and_then(|out| {
                    if out.status.success() {
                        String::from_utf8(out.stdout)
                            .ok()
                            .map(|s| s.trim().to_owned())
                    } else {
                        None
                    }
                })
        })
        .unwrap_or_else(|| String::from("unknown rustc"));

    println!("cargo:rustc-env=DEEPTIDE_GIT_HASH={git_hash}{dirty_suffix}");
    println!("cargo:rustc-env=DEEPTIDE_GIT_DATE={git_date}");
    println!("cargo:rustc-env=DEEPTIDE_GIT_BRANCH={git_branch}");
    println!("cargo:rustc-env=DEEPTIDE_RUSTC={rustc_version}");

    // Re-run the build script when HEAD moves so a fresh checkout or a
    // local commit refreshes the embedded hash without `cargo clean`.
    // We point at the resolved .git directory; if it doesn't exist we
    // simply emit nothing and Cargo treats the script as fully cached
    // until any other input changes (which is the desired behaviour for
    // crates.io-style builds where no git metadata exists).
    if let Some(git_dir) = workspace_root.as_ref().map(|root| root.join(".git"))
        && git_dir.exists()
    {
        // .git/HEAD captures branch / detached-HEAD changes; .git/index
        // captures staging changes that affect the "-dirty" probe.
        println!("cargo:rerun-if-changed={}", git_dir.join("HEAD").display());
        println!("cargo:rerun-if-changed={}", git_dir.join("index").display());
    }
    // Always honour an explicit override knob — useful for distro packagers
    // who want to inject a known hash without running git.
    println!("cargo:rerun-if-env-changed=DEEPTIDE_GIT_HASH_OVERRIDE");
    if let Ok(override_hash) = std::env::var("DEEPTIDE_GIT_HASH_OVERRIDE") {
        println!("cargo:rustc-env=DEEPTIDE_GIT_HASH={override_hash}");
    }
}

/// Walk upward from `CARGO_MANIFEST_DIR` looking for the workspace root
/// (the directory containing the workspace `Cargo.toml` with the `.git/`
/// folder alongside it). Returns `None` if we can't locate either, in
/// which case all git probes fall back to "unknown".
fn locate_workspace_root() -> Option<std::path::PathBuf> {
    let manifest_dir = std::env::var_os("CARGO_MANIFEST_DIR")?;
    let mut current = Path::new(&manifest_dir).to_path_buf();
    for _ in 0..6 {
        if current.join(".git").exists() {
            return Some(current);
        }
        if !current.pop() {
            break;
        }
    }
    None
}

/// Run a git subcommand inside `workspace_root`, returning its trimmed
/// stdout on a clean zero exit. Any failure mode (no git binary, not a
/// repo, non-zero exit, non-UTF8 output) collapses to `None` so the
/// caller can substitute a sensible default.
fn probe_git(workspace_root: &Option<std::path::PathBuf>, args: &[&str]) -> Option<String> {
    let root = workspace_root.as_ref()?;
    let output = Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8(output.stdout).ok()?.trim().to_owned();
    if stdout.is_empty() {
        None
    } else {
        Some(stdout)
    }
}
