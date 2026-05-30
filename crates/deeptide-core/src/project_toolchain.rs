//! Detect a project's primary toolchain so `/test` and `/lint` can
//! pick the right command without making the user type it.
//!
//! We deliberately keep the surface narrow: only commands and arguments
//! that are stable across the ecosystem ship as defaults. Users can
//! always pipe `!cargo whatever-they-want` through the bash escape if
//! their workflow diverges. The detection walks the cwd looking for a
//! single marker file per language; we do NOT walk parent directories
//! to find one — interactive coding agents are usually launched from
//! the project root and a recursive search invites surprise.
//!
//! The module is split from the slash-command dispatcher so it can be
//! unit-tested against synthesised directories under `tempfile::tempdir`
//! without spinning up a `ReplSession`.

use std::path::{Path, PathBuf};

/// One detected toolchain. Multiple kinds can coexist (a Next.js app
/// with a Rust backend, say) — callers should prefer the first match
/// in detection order, which favours the "more native" language for
/// the marker file most likely to indicate the test entry point.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectKind {
    Rust,
    Node {
        /// Resolved `package.json` path. Allows the renderer to surface
        /// which scripts the user has available.
        package_json: PathBuf,
        /// Detected package manager. Pinned by lockfile precedence:
        /// `bun.lockb` → bun, `pnpm-lock.yaml` → pnpm, `yarn.lock` →
        /// yarn, otherwise → npm.
        manager: NodePackageManager,
    },
    Python {
        marker: PythonMarker,
    },
    Go,
    Ruby,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodePackageManager {
    Npm,
    Pnpm,
    Yarn,
    Bun,
}

impl NodePackageManager {
    pub fn binary(self) -> &'static str {
        match self {
            NodePackageManager::Npm => "npm",
            NodePackageManager::Pnpm => "pnpm",
            NodePackageManager::Yarn => "yarn",
            NodePackageManager::Bun => "bun",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PythonMarker {
    PyprojectToml,
    SetupPy,
    Requirements,
}

/// A concrete command line the slash command can execute. Carries the
/// argv as separate strings (not a single shell line) so spawn paths
/// can avoid `sh -c` and the security gotchas it brings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolchainCommand {
    pub program: String,
    pub args: Vec<String>,
    /// Human-readable label rendered in the "Detected: …" line.
    pub label: String,
}

impl ToolchainCommand {
    /// Pretty-print the command as a single line, suitable for
    /// inclusion in user-facing output (the user can copy-paste it
    /// into a terminal if the in-REPL execution isn't what they want).
    pub fn display(&self) -> String {
        let mut parts = Vec::with_capacity(self.args.len() + 1);
        parts.push(self.program.clone());
        parts.extend(self.args.iter().cloned());
        parts.join(" ")
    }
}

impl ProjectKind {
    pub fn label(&self) -> &'static str {
        match self {
            ProjectKind::Rust => "Rust (Cargo)",
            ProjectKind::Node { .. } => "Node (package.json)",
            ProjectKind::Python { .. } => "Python",
            ProjectKind::Go => "Go modules",
            ProjectKind::Ruby => "Ruby (Bundler)",
        }
    }

    /// Default test command. `None` means we detected the toolchain
    /// but there isn't a sensible cross-cutting default (e.g. Python
    /// projects without `pytest`).
    pub fn test_command(&self) -> Option<ToolchainCommand> {
        match self {
            ProjectKind::Rust => Some(ToolchainCommand {
                program: String::from("cargo"),
                args: vec![String::from("test"), String::from("--workspace")],
                label: String::from("cargo test"),
            }),
            ProjectKind::Node { manager, .. } => Some(ToolchainCommand {
                program: manager.binary().to_owned(),
                args: vec![String::from("test")],
                label: format!("{} test", manager.binary()),
            }),
            ProjectKind::Python { marker } => match marker {
                PythonMarker::PyprojectToml
                | PythonMarker::SetupPy
                | PythonMarker::Requirements => Some(ToolchainCommand {
                    program: String::from("pytest"),
                    args: Vec::new(),
                    label: String::from("pytest"),
                }),
            },
            ProjectKind::Go => Some(ToolchainCommand {
                program: String::from("go"),
                args: vec![String::from("test"), String::from("./...")],
                label: String::from("go test"),
            }),
            ProjectKind::Ruby => Some(ToolchainCommand {
                program: String::from("bundle"),
                args: vec![
                    String::from("exec"),
                    String::from("rake"),
                    String::from("test"),
                ],
                label: String::from("bundle exec rake test"),
            }),
        }
    }

    /// Default lint command.
    pub fn lint_command(&self) -> Option<ToolchainCommand> {
        match self {
            ProjectKind::Rust => Some(ToolchainCommand {
                program: String::from("cargo"),
                args: vec![
                    String::from("clippy"),
                    String::from("--workspace"),
                    String::from("--all-targets"),
                    String::from("--"),
                    String::from("-D"),
                    String::from("warnings"),
                ],
                label: String::from("cargo clippy"),
            }),
            ProjectKind::Node { manager, .. } => Some(ToolchainCommand {
                program: manager.binary().to_owned(),
                args: vec![String::from("run"), String::from("lint")],
                label: format!("{} run lint", manager.binary()),
            }),
            ProjectKind::Python { .. } => Some(ToolchainCommand {
                program: String::from("ruff"),
                args: vec![String::from("check"), String::from(".")],
                label: String::from("ruff check"),
            }),
            ProjectKind::Go => Some(ToolchainCommand {
                program: String::from("go"),
                args: vec![String::from("vet"), String::from("./...")],
                label: String::from("go vet"),
            }),
            ProjectKind::Ruby => Some(ToolchainCommand {
                program: String::from("bundle"),
                args: vec![String::from("exec"), String::from("rubocop")],
                label: String::from("bundle exec rubocop"),
            }),
        }
    }
}

/// Inspect `cwd` and return the toolchains we recognise, in detection
/// order. The order reflects "compiled languages tend to dictate the
/// test runner" — Rust before Node when both `Cargo.toml` and
/// `package.json` exist (common in WASM bindings, embedded JS, etc.).
pub fn detect_toolchains(cwd: &Path) -> Vec<ProjectKind> {
    let mut found = Vec::new();

    if cwd.join("Cargo.toml").exists() {
        found.push(ProjectKind::Rust);
    }
    if let Some(package_json) = exists(cwd, "package.json") {
        let manager = detect_node_manager(cwd);
        found.push(ProjectKind::Node {
            package_json,
            manager,
        });
    }
    if exists(cwd, "pyproject.toml").is_some() {
        found.push(ProjectKind::Python {
            marker: PythonMarker::PyprojectToml,
        });
    } else if exists(cwd, "setup.py").is_some() {
        found.push(ProjectKind::Python {
            marker: PythonMarker::SetupPy,
        });
    } else if exists(cwd, "requirements.txt").is_some() {
        found.push(ProjectKind::Python {
            marker: PythonMarker::Requirements,
        });
    }
    if exists(cwd, "go.mod").is_some() {
        found.push(ProjectKind::Go);
    }
    if exists(cwd, "Gemfile").is_some() {
        found.push(ProjectKind::Ruby);
    }

    found
}

fn exists(cwd: &Path, name: &str) -> Option<PathBuf> {
    let candidate = cwd.join(name);
    candidate.exists().then_some(candidate)
}

fn detect_node_manager(cwd: &Path) -> NodePackageManager {
    if cwd.join("bun.lockb").exists() {
        NodePackageManager::Bun
    } else if cwd.join("pnpm-lock.yaml").exists() {
        NodePackageManager::Pnpm
    } else if cwd.join("yarn.lock").exists() {
        NodePackageManager::Yarn
    } else {
        NodePackageManager::Npm
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn detects_rust_workspace() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("Cargo.toml"), "[workspace]").unwrap();
        let kinds = detect_toolchains(dir.path());
        assert_eq!(kinds.len(), 1);
        assert!(matches!(kinds[0], ProjectKind::Rust));
        let cmd = kinds[0].test_command().unwrap();
        assert_eq!(cmd.program, "cargo");
        assert_eq!(cmd.args[0], "test");
    }

    #[test]
    fn detects_node_npm_default() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("package.json"), "{}").unwrap();
        let kinds = detect_toolchains(dir.path());
        match &kinds[0] {
            ProjectKind::Node { manager, .. } => assert_eq!(*manager, NodePackageManager::Npm),
            other => panic!("expected Node, got {other:?}"),
        }
    }

    #[test]
    fn detects_node_pnpm_via_lockfile() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("package.json"), "{}").unwrap();
        fs::write(dir.path().join("pnpm-lock.yaml"), "").unwrap();
        match &detect_toolchains(dir.path())[0] {
            ProjectKind::Node { manager, .. } => assert_eq!(*manager, NodePackageManager::Pnpm),
            other => panic!("expected Node, got {other:?}"),
        }
    }

    #[test]
    fn detects_node_yarn_via_lockfile() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("package.json"), "{}").unwrap();
        fs::write(dir.path().join("yarn.lock"), "").unwrap();
        match &detect_toolchains(dir.path())[0] {
            ProjectKind::Node { manager, .. } => assert_eq!(*manager, NodePackageManager::Yarn),
            other => panic!("expected Node, got {other:?}"),
        }
    }

    #[test]
    fn detects_node_bun_via_lockfile() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("package.json"), "{}").unwrap();
        fs::write(dir.path().join("bun.lockb"), "").unwrap();
        match &detect_toolchains(dir.path())[0] {
            ProjectKind::Node { manager, .. } => assert_eq!(*manager, NodePackageManager::Bun),
            other => panic!("expected Node, got {other:?}"),
        }
    }

    #[test]
    fn detects_python_pyproject() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("pyproject.toml"), "[project]").unwrap();
        match &detect_toolchains(dir.path())[0] {
            ProjectKind::Python { marker } => assert_eq!(*marker, PythonMarker::PyprojectToml),
            other => panic!("expected Python, got {other:?}"),
        }
    }

    #[test]
    fn detects_python_setup_py_when_pyproject_missing() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("setup.py"), "from setuptools import setup").unwrap();
        match &detect_toolchains(dir.path())[0] {
            ProjectKind::Python { marker } => assert_eq!(*marker, PythonMarker::SetupPy),
            other => panic!("expected Python, got {other:?}"),
        }
    }

    #[test]
    fn detects_go_modules() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("go.mod"), "module x").unwrap();
        let kinds = detect_toolchains(dir.path());
        assert_eq!(kinds.len(), 1);
        assert_eq!(kinds[0], ProjectKind::Go);
        let cmd = kinds[0].lint_command().unwrap();
        assert_eq!(cmd.program, "go");
        assert_eq!(cmd.args, vec![String::from("vet"), String::from("./...")]);
    }

    #[test]
    fn detects_ruby_via_gemfile() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("Gemfile"), "source 'https://rubygems.org'").unwrap();
        assert_eq!(detect_toolchains(dir.path())[0], ProjectKind::Ruby);
    }

    #[test]
    fn detection_returns_empty_for_unknown_directory() {
        let dir = tempdir().unwrap();
        assert!(detect_toolchains(dir.path()).is_empty());
    }

    #[test]
    fn detection_returns_multiple_kinds_for_polyglot() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("Cargo.toml"), "[workspace]").unwrap();
        fs::write(dir.path().join("package.json"), "{}").unwrap();
        let kinds = detect_toolchains(dir.path());
        assert_eq!(kinds.len(), 2);
        // Rust first per stated detection order.
        assert_eq!(kinds[0], ProjectKind::Rust);
        assert!(matches!(kinds[1], ProjectKind::Node { .. }));
    }

    #[test]
    fn toolchain_command_display_joins_argv_with_spaces() {
        let cmd = ToolchainCommand {
            program: String::from("cargo"),
            args: vec![String::from("test"), String::from("--workspace")],
            label: String::from("cargo test"),
        };
        assert_eq!(cmd.display(), "cargo test --workspace");
    }

    #[test]
    fn rust_lint_command_is_clippy_dash_d_warnings() {
        let cmd = ProjectKind::Rust.lint_command().unwrap();
        assert_eq!(cmd.program, "cargo");
        assert!(cmd.args.iter().any(|a| a == "clippy"));
        // Must include `-D warnings` to fail on lints.
        assert!(cmd.args.iter().any(|a| a == "-D"));
        assert!(cmd.args.iter().any(|a| a == "warnings"));
    }
}
