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

/// Commands that read/exfiltrate file contents. A shell command is only
/// blocked for mentioning a sensitive path when it also invokes one of these.
/// Mirrors Swift's `SensitiveFilePolicy.readCommands`.
const READ_COMMANDS: &[&str] = &[
    "cat", "head", "tail", "less", "more", "sed", "awk", "grep", "egrep", "fgrep", "rg", "ripgrep",
    "strings", "base64", "xxd", "hexdump", "python", "python3", "ruby", "perl", "node", "deno",
    "jq", "yq", "cp", "rsync", "tar", "zip", "gzip", "openssl",
];

/// Shell token separators used to split a command into words/paths.
const SHELL_SEPARATORS: &[char] = &[
    ' ', '\t', '\n', ';', '&', '|', '(', ')', '<', '>', '"', '\'',
];

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

/// Decide whether a shell command should be blocked because it reads a
/// sensitive file that has not been `/open`'d.
///
/// Blocks when the command invokes a read-like command (`cat`, `grep`, …) AND
/// names a resolved sensitive path that is not open. `resolve` maps a raw
/// command token to the absolute path the read tools would use, so opened
/// paths match. Returns the denial message, or `None` to allow.
///
/// This is stricter than Swift's coarse substring pre-filter: it classifies
/// every token by [`is_sensitive`], so `cat secrets.txt` / `head key.pem` are
/// caught too (Swift's `mentionsSensitivePath` substring list misses those).
pub fn shell_command_block_reason(
    command: &str,
    resolve: impl Fn(&str) -> PathBuf,
) -> Option<String> {
    let tokens: Vec<&str> = command
        .split(SHELL_SEPARATORS)
        .filter(|token| !token.is_empty())
        .collect();

    let uses_read_command = tokens
        .iter()
        .any(|token| READ_COMMANDS.contains(&token.to_ascii_lowercase().as_str()));
    if !uses_read_command {
        return None;
    }

    let mentions_blocked = tokens.iter().any(|token| {
        let path = resolve(token);
        is_sensitive(&path) && !is_open(&path)
    });
    if !mentions_blocked {
        return None;
    }

    Some(String::from(
        "Sensitive file access blocked in this shell command (it may hold secrets, \
         credentials, or keys). Run `/open <path>` for each sensitive file you want \
         this session to read, then retry.",
    ))
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

    #[test]
    fn shell_command_blocking_targets_reads_of_unopened_sensitive_paths() {
        let resolve = |token: &str| PathBuf::from(format!("/proj/{token}"));

        // Read-like command touching a sensitive file → blocked.
        assert!(shell_command_block_reason("cat .env", resolve).is_some());
        assert!(shell_command_block_reason("head -n5 secrets.txt", resolve).is_some());
        assert!(shell_command_block_reason("grep KEY config/app.pem", resolve).is_some());
        // Pipelines / chained commands are split on separators.
        assert!(shell_command_block_reason("echo hi && cat .env", resolve).is_some());

        // No read-like command → allowed even if a sensitive path appears.
        assert!(shell_command_block_reason("rm .env", resolve).is_none());
        assert!(shell_command_block_reason("ls -la", resolve).is_none());
        // Read command but no sensitive path → allowed.
        assert!(shell_command_block_reason("cat README.md", resolve).is_none());
    }

    #[test]
    fn shell_command_blocking_respects_opened_paths() {
        // Resolve to a unique dir so the global registry can't be polluted by
        // other tests running in parallel.
        let dir = "/tmp/deeptide-shell-block-unique-abc";
        let resolve = |token: &str| PathBuf::from(format!("{dir}/{token}"));
        assert!(shell_command_block_reason("cat .env", resolve).is_some());

        mark_open(&PathBuf::from(format!("{dir}/.env")));
        assert!(
            shell_command_block_reason("cat .env", resolve).is_none(),
            "once opened, the shell read is allowed"
        );
    }
}
