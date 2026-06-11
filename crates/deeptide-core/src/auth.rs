use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::memory::MemorySystem;

/// Production Paean inference gateway. It speaks the Anthropic Messages shape.
pub const PAEAN_API_BASE_URL: &str = "https://api.paean.ai/zero";
pub const PAEAN_WEB_BASE_URL: &str = "https://one.paean.ai";

/// Browser-login credentials compatible with zero-cli's
/// `~/.zero/credentials.json`, stored canonically under Deeptide's shared
/// `~/.config/tide/credentials.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaeanCredentials {
    pub token: String,
    #[serde(rename = "apiKey", skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    #[serde(rename = "userId", skip_serializing_if = "Option::is_none")]
    pub user_id: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(rename = "loginTime")]
    pub login_time: String,
}

impl PaeanCredentials {
    pub fn display_identity(&self) -> &str {
        self.email.as_deref().unwrap_or("(email unavailable)")
    }
}

pub fn paean_credentials_path() -> PathBuf {
    MemorySystem::tide_config_dir().join("credentials.json")
}

pub fn legacy_zero_credentials_path() -> Option<PathBuf> {
    home_dir().map(|home| home.join(".zero").join("credentials.json"))
}

pub fn load_paean_credentials() -> Option<PaeanCredentials> {
    read_credentials(&paean_credentials_path())
        .or_else(|| legacy_zero_credentials_path().and_then(|path| read_credentials(&path)))
}

pub fn save_paean_credentials(credentials: &PaeanCredentials) -> Result<(), String> {
    let path = paean_credentials_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
    }
    let body = serde_json::to_string_pretty(credentials)
        .map_err(|error| format!("cannot serialize credentials: {error}"))?;
    std::fs::write(&path, body)
        .map_err(|error| format!("cannot write {}: {error}", path.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
    }
    Ok(())
}

pub fn clear_paean_credentials() -> Result<(), String> {
    let mut first_error = None;
    for path in [
        Some(paean_credentials_path()),
        legacy_zero_credentials_path(),
    ]
    .into_iter()
    .flatten()
    {
        if path.exists()
            && let Err(error) = std::fs::remove_file(&path)
            && first_error.is_none()
        {
            first_error = Some(format!("cannot remove {}: {error}", path.display()));
        }
    }
    match first_error {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

/// First Paean token from explicit env or stored login credentials.
pub fn effective_paean_token() -> Option<String> {
    env_first_non_empty(&["PAEAN_API_TOKEN", "PAEAN_TOKEN", "CLIDE_API_TOKEN"])
        .or_else(|| load_paean_credentials().map(|credentials| credentials.token))
}

pub fn has_paean_token() -> bool {
    effective_paean_token().is_some()
}

fn read_credentials(path: &PathBuf) -> Option<PaeanCredentials> {
    let raw = std::fs::read_to_string(path).ok()?;
    let credentials: PaeanCredentials = serde_json::from_str(&raw).ok()?;
    if credentials.token.trim().is_empty() {
        None
    } else {
        Some(credentials)
    }
}

fn env_first_non_empty(names: &[&str]) -> Option<String> {
    names
        .iter()
        .filter_map(|name| std::env::var(name).ok())
        .find(|value| !value.trim().is_empty())
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::PaeanCredentials;

    #[test]
    fn credentials_parse_zero_cli_shape() {
        let raw = r#"{
          "token": "tok",
          "userId": 42,
          "email": "u@example.com",
          "loginTime": "2026-06-03T00:00:00Z"
        }"#;
        let credentials: PaeanCredentials = serde_json::from_str(raw).expect("parse");
        assert_eq!(credentials.token, "tok");
        assert_eq!(credentials.user_id, Some(42));
        assert_eq!(credentials.display_identity(), "u@example.com");
    }
}
