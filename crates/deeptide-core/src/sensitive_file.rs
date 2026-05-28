//! Sensitive-file read policy.
//!
//! Mirrors the Swift implementation's `SensitiveFilePolicy`: files that
//! commonly hold secrets (`.env`, credentials, private keys, etc.) are blocked
//! from the file-read tools so the agent cannot inadvertently surface their
//! contents to the model. The user can grant a specific file read access for
//! the current session with the `/open <path>` command.
//!
//! The set of opened paths is held in a process-global registry (one process =
//! one REPL session), the same pattern used by [`crate::background_shell`], so
//! no per-tool session state needs to be threaded through `ToolContext`.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

/// Exact (case-insensitive) file names treated as sensitive.
const EXACT_NAMES: &[&str] = &[
    ".env",
    ".npmrc",
    ".pypirc",
    ".netrc",
    ".pgpass",
    "credentials",
    "credential",
    "secrets",
    "secret",
    "id_rsa",
    "id_dsa",
    "id_ecdsa",
    "id_ed25519",
];

/// File extensions (case-insensitive, without the dot) treated as sensitive.
const SENSITIVE_EXTENSIONS: &[&str] = &["pem", "key", "p12", "pfx", "crt", "cer"];

/// Whether `path`'s file name marks it as likely holding secrets.
///
/// Matches Swift's `SensitiveFilePolicy.isSensitive`: exact names, the `.env.`
/// prefix (e.g. `.env.local`), any name containing `secret`/`credential`, or a
/// known credential/key extension.
pub fn is_sensitive(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    let lower = name.to_ascii_lowercase();
    if EXACT_NAMES.contains(&lower.as_str()) {
        return true;
    }
    if lower.starts_with(".env.") {
        return true;
    }
    if lower.contains("secret") || lower.contains("credential") {
        return true;
    }
    if let Some(ext) = path.extension().and_then(|e| e.to_str())
        && SENSITIVE_EXTENSIONS.contains(&ext.to_ascii_lowercase().as_str())
    {
        return true;
    }
    false
}

fn opened_paths() -> &'static Mutex<HashSet<PathBuf>> {
    static OPENED: OnceLock<Mutex<HashSet<PathBuf>>> = OnceLock::new();
    OPENED.get_or_init(|| Mutex::new(HashSet::new()))
}

/// Grant the current session read access to a specific sensitive file
/// (invoked by the `/open` command).
pub fn mark_open(path: &Path) {
    opened_paths()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .insert(path.to_path_buf());
}

/// Whether `path` has been explicitly opened this session.
pub fn is_open(path: &Path) -> bool {
    opened_paths()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .contains(path)
}

/// Whether a file-read tool may read `path`: non-sensitive files are always
/// allowed; sensitive files only after an explicit `/open`.
pub fn is_allowed(path: &Path) -> bool {
    !is_sensitive(path) || is_open(path)
}

/// Message returned when a sensitive file is read without being opened first.
pub fn denial_message(path: &Path) -> String {
    let display = path.display();
    format!(
        "{display} is a sensitive file (it may hold secrets, credentials, or keys). \
         Run `/open {display}` to allow this session to read it, then retry."
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn classifies_sensitive_names_and_extensions() {
        for name in [
            ".env",
            ".env.local",
            ".env.production",
            ".npmrc",
            ".netrc",
            "credentials",
            "secrets",
            "my-secret-notes.txt",
            "aws_credential_dump",
            "server.pem",
            "tls.key",
            "cert.p12",
            "bundle.pfx",
            "ca.crt",
            "id_rsa",
            "id_ed25519",
        ] {
            assert!(
                is_sensitive(&PathBuf::from(format!("/proj/{name}"))),
                "{name} should be sensitive"
            );
        }
    }

    #[test]
    fn ordinary_files_are_not_sensitive() {
        for name in [
            "main.rs",
            "README.md",
            "notes.txt",
            "config.toml",
            "data.json",
        ] {
            assert!(
                !is_sensitive(&PathBuf::from(format!("/proj/{name}"))),
                "{name} should not be sensitive"
            );
        }
    }

    #[test]
    fn classification_is_case_insensitive() {
        assert!(is_sensitive(&PathBuf::from("/proj/Server.PEM")));
        assert!(is_sensitive(&PathBuf::from("/proj/.ENV")));
        assert!(is_sensitive(&PathBuf::from("/proj/My-SECRET.txt")));
    }

    #[test]
    fn opening_grants_read_access_to_a_sensitive_file() {
        // Use a unique path so the process-global registry can't be affected
        // by other tests running in parallel.
        let path = PathBuf::from("/tmp/deeptide-sensitive-test-unique-xyz/.env");
        assert!(is_sensitive(&path));
        assert!(!is_allowed(&path), "sensitive file blocked before /open");

        mark_open(&path);
        assert!(is_open(&path));
        assert!(is_allowed(&path), "sensitive file allowed after /open");

        // A different sensitive file remains blocked.
        let other = PathBuf::from("/tmp/deeptide-sensitive-test-unique-xyz/other.key");
        assert!(!is_allowed(&other));
    }

    #[test]
    fn denial_message_points_at_open_command() {
        let msg = denial_message(&PathBuf::from("/proj/.env"));
        assert!(msg.contains("/open"));
        assert!(msg.contains(".env"));
    }
}
