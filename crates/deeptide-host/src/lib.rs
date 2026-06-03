//! `deeptide-host` — the shared agent-assembly layer.
//!
//! Both the terminal CLI (`deeptide-cli`) and the native GUI (`deeptide-gui`)
//! must build the model backend, resolve provider presets, and wire the agent
//! loop *identically* — otherwise the two front-ends would drift (e.g. a
//! session that talks to DeepSeek in the CLI but Anthropic in the GUI). This
//! crate owns that protocol-neutral wiring so there is a single source of truth,
//! leaving each front-end to own only its own I/O (terminal vs. windowed).

pub mod backend;
pub mod provider;
