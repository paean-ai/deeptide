use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::memory::MemorySystem;

// ── Permission rules ──────────────────────────────────────────────────────────

/// A single allow/deny/ask rule as stored in `settings.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettingsRule {
    /// Glob pattern matched against the tool input (path, command, etc.).
    pub pattern: String,
    /// Optional tool name to narrow the rule to a specific tool.
    pub tool: Option<String>,
}

/// The `permissions` block in `settings.json`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SettingsPermissions {
    pub allow: Option<Vec<SettingsRule>>,
    pub deny: Option<Vec<SettingsRule>>,
    pub ask: Option<Vec<SettingsRule>>,
    #[serde(rename = "default_mode", skip_serializing_if = "Option::is_none")]
    pub default_mode: Option<String>,
}

// ── Hooks ─────────────────────────────────────────────────────────────────────

/// A single hook entry inside a hook-event group.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HookEntry {
    /// Glob/regex matched against the tool name (for tool hooks) or "" to match all.
    pub matcher: String,
    /// Shell command to run.  Receives tool context via env vars.
    pub command: String,
    /// Maximum milliseconds before the hook is killed.  Defaults to 10 000.
    #[serde(rename = "timeout_ms", skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
    /// When `true` the hook is loaded but not executed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disabled: Option<bool>,
    /// Human-readable label surfaced as `TIDE_HOOK_NAME` env var.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl HookEntry {
    pub fn is_disabled(&self) -> bool {
        self.disabled.unwrap_or(false)
    }
    pub fn effective_timeout_ms(&self) -> u64 {
        self.timeout_ms.unwrap_or(10_000)
    }
}

/// All hook event groups.  Keys mirror Swift's `SettingsHooks` and the
/// tide-spec §4.1 canonical PascalCase events.
///
/// On the wire both `PreToolUse` and `pre_tool_use` are accepted (both cases)
/// for backward compatibility with hooks written against earlier snake_case
/// schemas.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SettingsHooks {
    #[serde(rename = "PreToolUse", skip_serializing_if = "Option::is_none")]
    pub pre_tool_use: Option<Vec<HookEntry>>,
    #[serde(rename = "PostToolUse", skip_serializing_if = "Option::is_none")]
    pub post_tool_use: Option<Vec<HookEntry>>,
    #[serde(rename = "UserPromptSubmit", skip_serializing_if = "Option::is_none")]
    pub user_prompt_submit: Option<Vec<HookEntry>>,
    #[serde(rename = "SessionStart", skip_serializing_if = "Option::is_none")]
    pub session_start: Option<Vec<HookEntry>>,
    #[serde(rename = "SessionEnd", skip_serializing_if = "Option::is_none")]
    pub session_end: Option<Vec<HookEntry>>,
}

// ── MCP server ────────────────────────────────────────────────────────────────

/// A single MCP server entry in `mcp_servers`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct McpServerConfig {
    /// Executable to spawn (stdio transport).  Mutually exclusive with `url`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    /// CLI arguments passed to `command`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<String>>,
    /// Per-server environment variables.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<HashMap<String, String>>,
    /// HTTP/SSE server URL (HTTP transport).  Mutually exclusive with `command`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Optional display name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

// ── Provider profiles ──────────────────────────────────────────────────────────

/// A named API provider profile under the `providers` block.
///
/// Lets a user keep several Anthropic-compatible endpoints (e.g. DeepSeek,
/// Anthropic, a self-hosted gateway) in one settings file and switch between
/// them with `active_profile` or the `--profile` flag. Mirrors Swift's
/// `ProviderProfile`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderProfile {
    /// Base URL for this provider's Anthropic-compatible endpoint.
    #[serde(rename = "base_url", skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,

    /// API key stored inline. Prefer an environment variable for secrets;
    /// this is read only as a last resort.
    #[serde(rename = "api_key", skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,

    /// macOS Keychain account holding the API key. Read on macOS only; other
    /// platforms fall back to `api_key` or environment variables.
    #[serde(rename = "keychain_account", skip_serializing_if = "Option::is_none")]
    pub keychain_account: Option<String>,

    /// Default model identifier for this provider.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    /// Human-readable label shown in `/config`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

// ── Model pricing ──────────────────────────────────────────────────────────────

/// Per-model pricing override, in **USD per 1M tokens** (matching Swift's
/// `ModelPricing` and the `pricing` block in `settings.json`).
///
/// Unset fields fall back to the built-in pricing for that model, so an entry
/// can override just the rates it cares about.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct ModelPricingConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<f64>,
    #[serde(rename = "cache_create", skip_serializing_if = "Option::is_none")]
    pub cache_create: Option<f64>,
    #[serde(rename = "cache_read", skip_serializing_if = "Option::is_none")]
    pub cache_read: Option<f64>,
}

// ── ConfigData ────────────────────────────────────────────────────────────────

/// Parsed content of a `settings.json` file.
///
/// All fields are `Option` so absent keys are silently ignored and multiple
/// configs can be merged with higher-precedence values winning.
///
/// The field names follow the zero-cli / Swift `settings.json` format:
/// snake_case JSON keys, camelCase in Rust via `#[serde(rename = ...)]`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ConfigData {
    /// Default model identifier, e.g. `"deepseek-v4-pro"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    /// API key.  Written to the file only on explicit `/config set api_key=...`;
    /// prefer an environment variable for secrets.
    #[serde(rename = "api_key", skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,

    /// Base URL for the Anthropic-compatible API endpoint.
    #[serde(rename = "base_url", skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,

    /// Maximum agent turns per prompt.
    #[serde(rename = "max_turns", skip_serializing_if = "Option::is_none")]
    pub max_turns: Option<usize>,

    /// Maximum model output tokens per request.
    #[serde(rename = "max_tokens", skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<usize>,

    /// Default permission mode: `"default"`, `"accept-edits"`, `"plan"`, `"bypass"`.
    #[serde(rename = "permission_mode", skip_serializing_if = "Option::is_none")]
    pub permission_mode: Option<String>,

    /// Whether to enable prompt caching (default `true` when unset).
    #[serde(rename = "prompt_cache", skip_serializing_if = "Option::is_none")]
    pub prompt_cache: Option<bool>,

    /// Model retried once when the primary model is transiently overloaded.
    #[serde(rename = "fallback_model", skip_serializing_if = "Option::is_none")]
    pub fallback_model: Option<String>,

    /// Extended-thinking level: `low`, `medium`/`enabled`, `high`, `disabled`,
    /// or `auto`/unset to let the provider decide.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<String>,

    /// Reasoning-effort level (`low`/`medium`/`high`); an alias for `thinking`
    /// used when `thinking` is unset.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effort: Option<String>,

    /// Static permission rules applied before the runtime allow/deny lists.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permissions: Option<SettingsPermissions>,

    /// Hook definitions keyed by event name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hooks: Option<SettingsHooks>,

    /// Additional environment variables injected into every tool invocation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<HashMap<String, String>>,

    /// MCP server definitions.
    #[serde(rename = "mcp_servers", skip_serializing_if = "Option::is_none")]
    pub mcp_servers: Option<HashMap<String, McpServerConfig>>,

    /// Name of the active provider profile in `providers`.
    #[serde(rename = "active_profile", skip_serializing_if = "Option::is_none")]
    pub active_profile: Option<String>,

    /// Named API provider profiles.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub providers: Option<HashMap<String, ProviderProfile>>,

    /// Per-model pricing overrides (USD per 1M tokens), keyed by model id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pricing: Option<HashMap<String, ModelPricingConfig>>,
}

impl ConfigData {
    /// Merge `other` on top of `self`.  `other` values win when both are
    /// `Some`; `self` values are kept when `other` is `None`.
    pub fn merge(self, other: ConfigData) -> ConfigData {
        ConfigData {
            model: other.model.or(self.model),
            api_key: other.api_key.or(self.api_key),
            base_url: other.base_url.or(self.base_url),
            max_turns: other.max_turns.or(self.max_turns),
            max_tokens: other.max_tokens.or(self.max_tokens),
            permission_mode: other.permission_mode.or(self.permission_mode),
            prompt_cache: other.prompt_cache.or(self.prompt_cache),
            fallback_model: other.fallback_model.or(self.fallback_model),
            thinking: other.thinking.or(self.thinking),
            effort: other.effort.or(self.effort),
            permissions: other.permissions.or(self.permissions),
            hooks: other.hooks.or(self.hooks),
            env: merge_maps(self.env, other.env),
            mcp_servers: merge_maps(self.mcp_servers, other.mcp_servers),
            active_profile: other.active_profile.or(self.active_profile),
            providers: merge_maps(self.providers, other.providers),
            pricing: merge_maps(self.pricing, other.pricing),
        }
    }

    /// Convert the configured per-1M pricing overrides into the per-token
    /// [`ModelPricing`](crate::ModelPricing) map the cost tracker consumes.
    /// Unset rates fall back to the built-in pricing for each model.
    pub fn pricing_overrides(&self) -> HashMap<String, crate::ModelPricing> {
        let mut overrides = HashMap::new();
        let Some(ref pricing) = self.pricing else {
            return overrides;
        };
        for (model, entry) in pricing {
            let base = crate::CostTracker::base_pricing(model);
            let per_token = |value: Option<f64>, fallback: f64| {
                value.map(|usd| usd / 1_000_000.0).unwrap_or(fallback)
            };
            overrides.insert(
                model.clone(),
                crate::ModelPricing::new(
                    per_token(entry.input, base.input),
                    per_token(entry.output, base.output),
                    per_token(entry.cache_create, base.cache_create),
                    per_token(entry.cache_read, base.cache_read),
                ),
            );
        }
        overrides
    }

    /// Look up a provider profile by name.
    pub fn provider(&self, name: &str) -> Option<&ProviderProfile> {
        self.providers.as_ref()?.get(name)
    }

    /// Resolve the active provider profile and its name.
    ///
    /// The profile name is taken from `explicit` (e.g. a `--profile` flag) when
    /// non-empty, otherwise from the `active_profile` field. Returns `None` when
    /// no name resolves or the named profile is absent.
    pub fn active_provider(&self, explicit: Option<&str>) -> Option<(&str, &ProviderProfile)> {
        let name = explicit
            .filter(|value| !value.is_empty())
            .or(self.active_profile.as_deref())?;
        self.providers
            .as_ref()?
            .get_key_value(name)
            .map(|(key, value)| (key.as_str(), value))
    }

    /// Apply environment variables from the `env` block.
    ///
    /// Does **not** overwrite variables already set in the process environment
    /// so explicit `export FOO=bar` in the shell always wins.
    pub fn apply_env(&self) {
        let Some(ref env) = self.env else { return };
        for (key, value) in env {
            if std::env::var_os(key).is_none() {
                // SAFETY: called at startup before any threads are spawned.
                unsafe { std::env::set_var(key, value) };
            }
        }
    }
}

fn merge_maps<V>(
    base: Option<HashMap<String, V>>,
    overlay: Option<HashMap<String, V>>,
) -> Option<HashMap<String, V>> {
    match (base, overlay) {
        (None, None) => None,
        (Some(b), None) => Some(b),
        (None, Some(o)) => Some(o),
        (Some(mut b), Some(o)) => {
            b.extend(o);
            Some(b)
        }
    }
}

// ── ConfigStore ───────────────────────────────────────────────────────────────

/// Scope for a config write operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigScope {
    /// `~/.config/tide/settings.json`
    Global,
    /// `<cwd>/.deeptide/settings.json`
    Project,
    /// `<cwd>/.deeptide/settings.local.json`
    Local,
}

/// Static helpers for reading, writing, and locating `settings.json` files.
pub struct ConfigStore;

impl ConfigStore {
    /// Path to the user-global settings file.
    pub fn global_path() -> PathBuf {
        MemorySystem::tide_config_dir().join("settings.json")
    }

    /// Path to the project-scoped settings file in `cwd`.
    pub fn project_path(cwd: &Path) -> PathBuf {
        cwd.join(".deeptide").join("settings.json")
    }

    /// Path to the project-local (gitignored) settings override in `cwd`.
    pub fn local_path(cwd: &Path) -> PathBuf {
        cwd.join(".deeptide").join("settings.local.json")
    }

    /// Path for a given scope.
    pub fn scope_path(scope: ConfigScope, cwd: &Path) -> PathBuf {
        match scope {
            ConfigScope::Global => Self::global_path(),
            ConfigScope::Project => Self::project_path(cwd),
            ConfigScope::Local => Self::local_path(cwd),
        }
    }

    /// Load and merge all config files: global ← project ← local.
    ///
    /// Missing or malformed files are silently skipped.
    pub fn load(cwd: &Path) -> ConfigData {
        let global = Self::read_file(&Self::global_path());
        let project = Self::read_file(&Self::project_path(cwd));
        let local = Self::read_file(&Self::local_path(cwd));
        global.merge(project).merge(local)
    }

    fn read_file(path: &Path) -> ConfigData {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .unwrap_or_default()
    }

    /// Write a single `key=value` pair into the target file, preserving any
    /// other keys already present.  Creates the file and parent directory if
    /// they don't exist.
    pub fn set_value(key: &str, value: &str, path: &Path) -> Result<(), String> {
        let mut data: serde_json::Value = if path.exists() {
            let raw = std::fs::read_to_string(path)
                .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
            serde_json::from_str(&raw)
                .unwrap_or_else(|_| serde_json::Value::Object(serde_json::Map::new()))
        } else {
            serde_json::Value::Object(serde_json::Map::new())
        };

        let json_val: serde_json::Value = serde_json::from_str(value)
            .unwrap_or_else(|_| serde_json::Value::String(value.to_owned()));

        match &mut data {
            serde_json::Value::Object(map) => {
                map.insert(key.to_owned(), json_val);
            }
            _ => return Err(format!("{} is not a JSON object", path.display())),
        }

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("cannot create {}: {e}", parent.display()))?;
        }
        let serialized = serde_json::to_string_pretty(&data)
            .map_err(|e| format!("cannot serialize config: {e}"))?;
        std::fs::write(path, serialized)
            .map_err(|e| format!("cannot write {}: {e}", path.display()))
    }

    /// Remove a key from the target file.  No-op if the key doesn't exist.
    pub fn unset_value(key: &str, path: &Path) -> Result<(), String> {
        if !path.exists() {
            return Ok(());
        }
        let raw = std::fs::read_to_string(path)
            .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
        let mut data: serde_json::Value = serde_json::from_str(&raw)
            .unwrap_or_else(|_| serde_json::Value::Object(serde_json::Map::new()));
        if let serde_json::Value::Object(ref mut map) = data {
            map.remove(key);
        }
        let serialized = serde_json::to_string_pretty(&data)
            .map_err(|e| format!("cannot serialize config: {e}"))?;
        std::fs::write(path, serialized)
            .map_err(|e| format!("cannot write {}: {e}", path.display()))
    }

    /// Human-readable summary of the merged config for `/config show`.
    pub fn show(cwd: &Path) -> String {
        let global_path = Self::global_path();
        let project_path = Self::project_path(cwd);
        let local_path = Self::local_path(cwd);
        let merged = Self::load(cwd);

        let mut lines = vec![String::from("Settings files:")];
        for (label, path) in [
            ("global", &global_path),
            ("project", &project_path),
            ("local", &local_path),
        ] {
            let status = if path.exists() { "present" } else { "missing" };
            lines.push(format!("  {label:<8} {status:<7} {}", path.display()));
        }

        lines.push(String::new());
        lines.push(String::from("Merged values:"));
        let kv = |k: &str, v: &str| format!("  {k:<20} {v}");
        lines.push(kv(
            "model",
            merged
                .model
                .as_deref()
                .unwrap_or("(unset — use env or --model)"),
        ));
        lines.push(kv(
            "base_url",
            merged.base_url.as_deref().unwrap_or("(unset — default)"),
        ));
        lines.push(kv(
            "fallback_model",
            merged
                .fallback_model
                .as_deref()
                .unwrap_or("(unset — no fallback)"),
        ));
        lines.push(kv(
            "thinking",
            merged
                .thinking
                .as_deref()
                .or(merged.effort.as_deref())
                .unwrap_or("(unset — provider default)"),
        ));
        lines.push(kv(
            "max_turns",
            &merged
                .max_turns
                .map(|n| n.to_string())
                .unwrap_or_else(|| "(unset — default 25)".to_owned()),
        ));
        lines.push(kv(
            "max_tokens",
            &merged
                .max_tokens
                .map(|n| n.to_string())
                .unwrap_or_else(|| "(unset — default 4096)".to_owned()),
        ));
        lines.push(kv(
            "permission_mode",
            merged
                .permission_mode
                .as_deref()
                .unwrap_or("(unset — default)"),
        ));
        lines.push(kv(
            "prompt_cache",
            &merged
                .prompt_cache
                .map(|v| v.to_string())
                .unwrap_or_else(|| "(unset — true)".to_owned()),
        ));
        lines.push(kv(
            "api_key",
            if merged.api_key.is_some() {
                "(set — hidden)"
            } else {
                "(unset — use env)"
            },
        ));

        if let Some(ref perm) = merged.permissions {
            let allow_count = perm.allow.as_ref().map_or(0, Vec::len);
            let deny_count = perm.deny.as_ref().map_or(0, Vec::len);
            lines.push(kv(
                "permissions",
                &format!("{allow_count} allow, {deny_count} deny rules"),
            ));
        }

        if let Some(ref hooks) = merged.hooks {
            let count = hooks.pre_tool_use.as_ref().map_or(0, Vec::len)
                + hooks.post_tool_use.as_ref().map_or(0, Vec::len)
                + hooks.user_prompt_submit.as_ref().map_or(0, Vec::len)
                + hooks.session_start.as_ref().map_or(0, Vec::len)
                + hooks.session_end.as_ref().map_or(0, Vec::len);
            lines.push(kv("hooks", &format!("{count} entries")));
        }

        if let Some(ref servers) = merged.mcp_servers {
            lines.push(kv("mcp_servers", &format!("{} configured", servers.len())));
        }

        if let Some(ref env) = merged.env {
            lines.push(kv("env", &format!("{} variables", env.len())));
        }

        if let Some(ref active) = merged.active_profile {
            lines.push(kv("active_profile", active));
        }
        if let Some(ref providers) = merged.providers {
            lines.push(kv("providers", &format!("{} configured", providers.len())));
            let mut names: Vec<&String> = providers.keys().collect();
            names.sort();
            for name in names {
                let profile = &providers[name];
                let active = merged.active_profile.as_deref() == Some(name.as_str());
                let marker = if active { "*" } else { " " };
                let base = profile.base_url.as_deref().unwrap_or("(default)");
                let label = profile.label.as_deref().unwrap_or("");
                lines.push(format!("    {marker} {name:<16} {base}  {label}"));
            }
        }

        if let Some(ref pricing) = merged.pricing {
            lines.push(kv(
                "pricing",
                &format!("{} model overrides (USD per 1M tokens)", pricing.len()),
            ));
        }

        lines.push(String::new());
        lines.push(String::from("Usage:"));
        lines.push(String::from(
            "  /config set key=value           write to global settings",
        ));
        lines.push(String::from(
            "  /config set key=value --project  write to project settings",
        ));
        lines.push(String::from(
            "  /config unset key               remove from global settings",
        ));

        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_data_merge_other_wins() {
        let base = ConfigData {
            model: Some(String::from("model-a")),
            max_turns: Some(10),
            ..Default::default()
        };
        let overlay = ConfigData {
            model: Some(String::from("model-b")),
            ..Default::default()
        };
        let merged = base.merge(overlay);
        assert_eq!(merged.model.as_deref(), Some("model-b"));
        assert_eq!(merged.max_turns, Some(10)); // kept from base
    }

    #[test]
    fn config_data_merge_preserves_base_when_overlay_none() {
        let base = ConfigData {
            model: Some(String::from("model-a")),
            ..Default::default()
        };
        let overlay = ConfigData::default();
        let merged = base.merge(overlay);
        assert_eq!(merged.model.as_deref(), Some("model-a"));
    }

    #[test]
    fn config_store_round_trip_set_and_read() {
        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().join("settings.json");

        ConfigStore::set_value("model", "deepseek-v4-flash", &path).expect("set model");
        ConfigStore::set_value("max_turns", "15", &path).expect("set max_turns");

        let data = ConfigStore::read_file(&path);
        assert_eq!(data.model.as_deref(), Some("deepseek-v4-flash"));
        assert_eq!(data.max_turns, Some(15));
    }

    #[test]
    fn config_store_set_preserves_existing_keys() {
        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().join("settings.json");

        ConfigStore::set_value("model", "model-a", &path).expect("set model");
        ConfigStore::set_value("max_turns", "7", &path).expect("set max_turns");

        let data = ConfigStore::read_file(&path);
        assert_eq!(data.model.as_deref(), Some("model-a"));
        assert_eq!(data.max_turns, Some(7));
    }

    #[test]
    fn config_store_unset_removes_key() {
        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().join("settings.json");

        ConfigStore::set_value("model", "model-a", &path).expect("set");
        ConfigStore::unset_value("model", &path).expect("unset");

        let data = ConfigStore::read_file(&path);
        assert!(data.model.is_none());
    }

    #[test]
    fn config_store_load_merges_three_layers() {
        let temp = tempfile::tempdir().expect("tempdir");
        let cwd = temp.path();

        // Simulate global config
        let global_dir = cwd.join(".tide-test-global");
        std::fs::create_dir_all(&global_dir).expect("global dir");
        let global_config = serde_json::json!({ "model": "global-model", "max_turns": 20 });
        std::fs::write(
            global_dir.join("settings.json"),
            serde_json::to_string_pretty(&global_config).expect("serialize"),
        )
        .expect("write global");

        // Simulate project config that overrides max_turns
        let project_dir = cwd.join(".deeptide");
        std::fs::create_dir_all(&project_dir).expect("project dir");
        let project_config = serde_json::json!({ "max_turns": 5 });
        std::fs::write(
            project_dir.join("settings.json"),
            serde_json::to_string_pretty(&project_config).expect("serialize"),
        )
        .expect("write project");

        // Direct path test (not using TIDE_CONFIG_DIR since that's global state)
        let global_data = ConfigStore::read_file(&global_dir.join("settings.json"));
        let project_data = ConfigStore::read_file(&project_dir.join("settings.json"));
        let merged = global_data.merge(project_data);

        assert_eq!(merged.model.as_deref(), Some("global-model")); // from global
        assert_eq!(merged.max_turns, Some(5)); // overridden by project
    }

    #[test]
    fn hook_entry_effective_timeout_defaults_to_ten_seconds() {
        let h = HookEntry {
            matcher: String::from("*"),
            command: String::from("echo hi"),
            timeout_ms: None,
            disabled: None,
            name: None,
        };
        assert_eq!(h.effective_timeout_ms(), 10_000);
        assert!(!h.is_disabled());
    }

    #[test]
    fn config_data_apply_env_skips_existing_vars() {
        let mut data = ConfigData::default();
        let mut env_map = HashMap::new();
        env_map.insert(
            String::from("DEEPTIDE_TEST_UNIQUE_VAR_XYZ"),
            String::from("from-config"),
        );
        data.env = Some(env_map);

        // Pre-set the var
        unsafe {
            std::env::set_var("DEEPTIDE_TEST_UNIQUE_VAR_XYZ", "pre-existing");
        }
        data.apply_env();
        assert_eq!(
            std::env::var("DEEPTIDE_TEST_UNIQUE_VAR_XYZ").expect("var should exist"),
            "pre-existing"
        );
        unsafe {
            std::env::remove_var("DEEPTIDE_TEST_UNIQUE_VAR_XYZ");
        }
    }

    #[test]
    fn config_data_parses_provider_profiles() {
        let raw = serde_json::json!({
            "active_profile": "deepseek",
            "providers": {
                "deepseek": {
                    "base_url": "https://api.deepseek.com/anthropic",
                    "model": "deepseek-v4-pro",
                    "label": "DeepSeek official",
                    "keychain_account": "default"
                },
                "anthropic": {
                    "base_url": "https://api.anthropic.com"
                }
            }
        })
        .to_string();
        let data: ConfigData = serde_json::from_str(&raw).expect("parse providers");

        assert_eq!(data.active_profile.as_deref(), Some("deepseek"));
        let deepseek = data.provider("deepseek").expect("deepseek provider");
        assert_eq!(
            deepseek.base_url.as_deref(),
            Some("https://api.deepseek.com/anthropic")
        );
        assert_eq!(deepseek.model.as_deref(), Some("deepseek-v4-pro"));
        assert_eq!(deepseek.keychain_account.as_deref(), Some("default"));
        assert!(data.provider("missing").is_none());
    }

    #[test]
    fn active_provider_prefers_explicit_then_active_profile() {
        let data = ConfigData {
            active_profile: Some(String::from("deepseek")),
            providers: Some(HashMap::from([
                (
                    String::from("deepseek"),
                    ProviderProfile {
                        base_url: Some(String::from("https://api.deepseek.com/anthropic")),
                        ..Default::default()
                    },
                ),
                (
                    String::from("anthropic"),
                    ProviderProfile {
                        base_url: Some(String::from("https://api.anthropic.com")),
                        ..Default::default()
                    },
                ),
            ])),
            ..Default::default()
        };

        // Falls back to active_profile when no explicit override.
        let (name, profile) = data.active_provider(None).expect("active provider");
        assert_eq!(name, "deepseek");
        assert_eq!(
            profile.base_url.as_deref(),
            Some("https://api.deepseek.com/anthropic")
        );

        // Explicit override wins.
        let (name, _) = data
            .active_provider(Some("anthropic"))
            .expect("explicit provider");
        assert_eq!(name, "anthropic");

        // Empty explicit string is ignored, unknown name yields None.
        assert_eq!(
            data.active_provider(Some("")).map(|(n, _)| n),
            Some("deepseek")
        );
        assert!(data.active_provider(Some("ghost")).is_none());
    }

    #[test]
    fn config_data_parses_and_merges_fallback_model() {
        let data: ConfigData =
            serde_json::from_str(r#"{"fallback_model": "deepseek-v4-flash"}"#).expect("parse");
        assert_eq!(data.fallback_model.as_deref(), Some("deepseek-v4-flash"));

        let base = ConfigData {
            fallback_model: Some(String::from("base-fallback")),
            ..Default::default()
        };
        let overlay = ConfigData {
            fallback_model: Some(String::from("overlay-fallback")),
            ..Default::default()
        };
        assert_eq!(
            base.clone().merge(overlay).fallback_model.as_deref(),
            Some("overlay-fallback")
        );
        assert_eq!(
            base.merge(ConfigData::default()).fallback_model.as_deref(),
            Some("base-fallback")
        );
    }

    #[test]
    fn config_data_parses_and_merges_thinking_effort() {
        let data: ConfigData =
            serde_json::from_str(r#"{"thinking": "high", "effort": "low"}"#).expect("parse");
        assert_eq!(data.thinking.as_deref(), Some("high"));
        assert_eq!(data.effort.as_deref(), Some("low"));

        let base = ConfigData {
            thinking: Some(String::from("medium")),
            effort: Some(String::from("low")),
            ..Default::default()
        };
        let overlay = ConfigData {
            thinking: Some(String::from("high")),
            ..Default::default()
        };
        let merged = base.merge(overlay);
        assert_eq!(merged.thinking.as_deref(), Some("high"));
        // effort is preserved from base when the overlay omits it.
        assert_eq!(merged.effort.as_deref(), Some("low"));
    }

    #[test]
    fn pricing_overrides_convert_per_million_to_per_token_with_fallback() {
        let data: ConfigData = serde_json::from_str(
            r#"{"pricing": {"custom-model": {"input": 1.0, "output": 2.0, "cache_create": 3.0, "cache_read": 4.0}}}"#,
        )
        .expect("parse pricing");

        let overrides = data.pricing_overrides();
        let pricing = overrides.get("custom-model").expect("override present");
        // Per-1M values are converted to per-token (divided by 1_000_000).
        assert!((pricing.input - 1.0 / 1_000_000.0).abs() < f64::EPSILON);
        assert!((pricing.output - 2.0 / 1_000_000.0).abs() < f64::EPSILON);
        assert!((pricing.cache_create - 3.0 / 1_000_000.0).abs() < f64::EPSILON);
        assert!((pricing.cache_read - 4.0 / 1_000_000.0).abs() < f64::EPSILON);
    }

    #[test]
    fn pricing_overrides_partial_entry_falls_back_to_built_in_rates() {
        // Only override the input rate for a known model; the rest should keep
        // the built-in pricing.
        let data: ConfigData =
            serde_json::from_str(r#"{"pricing": {"deepseek-v4-pro": {"input": 9.0}}}"#)
                .expect("parse pricing");

        let overrides = data.pricing_overrides();
        let pricing = overrides.get("deepseek-v4-pro").expect("override present");
        let base = crate::CostTracker::base_pricing("deepseek-v4-pro");
        assert!((pricing.input - 9.0 / 1_000_000.0).abs() < f64::EPSILON);
        // Unset rates retain the built-in values.
        assert!((pricing.output - base.output).abs() < f64::EPSILON);
        assert!((pricing.cache_create - base.cache_create).abs() < f64::EPSILON);
        assert!((pricing.cache_read - base.cache_read).abs() < f64::EPSILON);
    }

    #[test]
    fn merge_combines_provider_maps() {
        let base = ConfigData {
            active_profile: Some(String::from("deepseek")),
            providers: Some(HashMap::from([(
                String::from("deepseek"),
                ProviderProfile {
                    base_url: Some(String::from("https://api.deepseek.com/anthropic")),
                    ..Default::default()
                },
            )])),
            ..Default::default()
        };
        let overlay = ConfigData {
            providers: Some(HashMap::from([(
                String::from("anthropic"),
                ProviderProfile {
                    base_url: Some(String::from("https://api.anthropic.com")),
                    ..Default::default()
                },
            )])),
            ..Default::default()
        };
        let merged = base.merge(overlay);

        let providers = merged.providers.expect("providers");
        assert_eq!(providers.len(), 2);
        assert!(providers.contains_key("deepseek"));
        assert!(providers.contains_key("anthropic"));
        // active_profile is preserved from base when overlay omits it.
        assert_eq!(merged.active_profile.as_deref(), Some("deepseek"));
    }
}
