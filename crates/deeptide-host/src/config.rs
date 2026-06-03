//! Config-driven [`BackendParams`] resolution for non-clap front-ends (the GUI).
//!
//! The CLI resolves backend params from clap flags (which themselves absorb
//! layered config). A windowed front-end has no flags, so it resolves the same
//! params directly from the shared `ConfigStore` (the very same
//! `~/.config/tide/settings.json` + `.deeptide/settings.json` the CLI reads) and
//! the standard credential env vars. Keeping this in `deeptide-host` means the
//! GUI and CLI agree on precedence without duplicating logic.

use std::path::Path;

use deeptide_core::config::ConfigStore;

use crate::backend::{BackendParams, CloudCredential};

/// First non-empty value among `names` in the process environment.
fn env_first(names: &[&str]) -> Option<String> {
    names
        .iter()
        .filter_map(|name| std::env::var(name).ok())
        .find(|value| !value.trim().is_empty())
}

/// Resolve an API credential from config then the standard env vars, mirroring
/// the CLI's precedence (canonical Deeptide names first, then the legacy
/// `ZERO_*` / `ANTHROPIC_*` fallbacks).
fn resolve_credential(config_api_key: Option<&str>) -> Option<CloudCredential> {
    config_api_key
        .map(ToOwned::to_owned)
        .filter(|key| !key.trim().is_empty())
        .or_else(|| {
            env_first(&[
                "DEEPTIDE_API_KEY",
                "ZERO_API_KEY",
                "ZERO_CLI_API_KEY",
                "ANTHROPIC_API_KEY",
            ])
        })
        .map(CloudCredential::ApiKey)
}

/// Build [`BackendParams`] for `cwd` from the layered `settings.json` plus env.
///
/// Provider selection: there is no `settings.json` field for the provider preset
/// (it's a CLI flag concept), so we default to `anthropic` and honour a
/// `DEEPTIDE_PROVIDER` env override — enough to point the GUI at deepseek /
/// ollama / etc. until a provider picker lands. Explicit base-URL / model in
/// config (or their env vars) still override the preset default via the host
/// builder.
pub fn resolve_backend_params(cwd: &Path) -> BackendParams {
    let config = ConfigStore::load(cwd);
    BackendParams {
        provider: env_first(&["DEEPTIDE_PROVIDER"]).unwrap_or_else(|| String::from("anthropic")),
        base_url_override: config.base_url.clone().or_else(|| {
            env_first(&[
                "DEEPTIDE_BASE_URL",
                "ZERO_CLI_BASE_URL",
                "ANTHROPIC_BASE_URL",
            ])
        }),
        model_override: config
            .model
            .clone()
            .or_else(|| env_first(&["DEEPTIDE_MODEL", "ZERO_CLI_MODEL", "ANTHROPIC_MODEL"])),
        credential: resolve_credential(config.api_key.as_deref()),
        max_output_tokens: config.max_tokens.unwrap_or(65_536),
        enable_prompt_caching: config.prompt_cache.unwrap_or(true),
        // The GUI always installs a streaming handler, so streaming is on.
        stream: true,
        fallback_model: config.fallback_model.clone(),
        thinking_label: config.thinking.clone().or_else(|| config.effort.clone()),
        // Project system prompt (CLAUDE.md / memory) resolution is a later phase.
        system_prompt: None,
    }
}
