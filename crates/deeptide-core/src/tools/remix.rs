//! Remix tool: download the source of one or more published Paean Apps Square
//! games by hash and scaffold a new project that remixes them.
//!
//! Mirrors the Publish tool's shape (auth, blocking HTTP, native zip handling).
//! Shared infrastructure lives in the parent module (reached via `use super::*`).

use super::*;
use std::io::Cursor;

#[derive(Debug, Default, Clone, Copy)]
pub struct RemixTool;

impl Tool for RemixTool {
    fn name(&self) -> &'static str {
        "Remix"
    }

    fn description(&self) -> &'static str {
        "Download the source of one or more published Paean Apps Square games by hash and scaffold a new game that remixes them."
    }

    fn is_read_only(&self) -> bool {
        false
    }

    fn call(&self, input: serde_json::Value, context: &ToolContext) -> ToolResult {
        let options = match RemixOptions::from_input(&input) {
            Ok(options) => options,
            Err(message) => return ToolResult::error(message),
        };
        match run_remix(context, &options) {
            Ok(text) => ToolResult::text(text),
            Err(message) => ToolResult::error(message),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RemixSourceInput {
    raw: String,
    hash: String,
    role: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RemixOptions {
    sources: Vec<RemixSourceInput>,
    dir: Option<String>,
    title: Option<String>,
    summary: Option<String>,
    category: Option<String>,
    license: Option<String>,
    dry_run: bool,
}

impl RemixOptions {
    fn from_input(input: &serde_json::Value) -> Result<Self, String> {
        let option = |key: &str| {
            input
                .get(key)
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned)
        };

        // Accept `sources: [..]`, a single `source: ".."`, or a space-separated
        // `sources: ".."` string for convenience.
        let mut raw_tokens: Vec<String> = Vec::new();
        if let Some(array) = input.get("sources").and_then(serde_json::Value::as_array) {
            for value in array {
                if let Some(text) = value.as_str() {
                    raw_tokens.push(text.to_owned());
                }
            }
        } else if let Some(text) = input
            .get("sources")
            .or_else(|| input.get("source"))
            .and_then(serde_json::Value::as_str)
        {
            raw_tokens.extend(text.split_whitespace().map(ToOwned::to_owned));
        }

        let mut sources: Vec<RemixSourceInput> = Vec::new();
        for raw in raw_tokens {
            let raw = raw.trim().to_owned();
            if raw.is_empty() {
                continue;
            }
            let (token, role) = split_source_token(&raw);
            let hash = normalize_remix_token(&token)?;
            // Dedupe by hash, keeping the first role seen.
            if let Some(existing) = sources.iter_mut().find(|s| s.hash == hash) {
                if existing.role.is_none() {
                    existing.role = role;
                }
            } else {
                sources.push(RemixSourceInput { raw, hash, role });
            }
        }

        if sources.is_empty() {
            return Err(String::from(
                "Remix needs at least one source. Pass `sources` as an array of published Square app hashKeys (each may be `hashKey`, `hashKey=role`, `hashKey.8x.gg`, or an 8x.gg URL). A *.clide.app play URL is a deployed handle, not necessarily the Square hashKey.",
            ));
        }

        Ok(Self {
            sources,
            dir: option("dir"),
            title: option("title"),
            summary: option("summary"),
            category: option("category"),
            license: option("license"),
            dry_run: input
                .get("dry_run")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false),
        })
    }
}

/// Resolved upstream parent after a metadata lookup.
#[derive(Debug, Clone)]
struct RemixParent {
    hash: String,
    role: Option<String>,
    title: String,
    category: String,
    play_url: String,
    author: String,
}

struct RemixAuth {
    base_url: String,
    token: String,
}

fn run_remix(context: &ToolContext, options: &RemixOptions) -> Result<String, String> {
    let auth = resolve_remix_auth()?;

    // Resolve metadata for every source up front (also validates remixability).
    let mut parents: Vec<RemixParent> = Vec::with_capacity(options.sources.len());
    for source in &options.sources {
        let detail = fetch_app_detail(&auth, &source.hash)?;
        if detail.get("remixable").and_then(serde_json::Value::as_bool) == Some(false) {
            let reason = detail
                .get("remixDisabledReason")
                .and_then(serde_json::Value::as_str)
                .map(|r| format!(": {r}"))
                .unwrap_or_else(|| String::from("."));
            return Err(format!("Source {} is not remixable{reason}", source.hash));
        }
        let str_field = |key: &str| {
            detail
                .get(key)
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_owned()
        };
        let resolved_hash = detail
            .get("hashKey")
            .and_then(serde_json::Value::as_str)
            .filter(|value| !value.is_empty())
            .unwrap_or(source.hash.as_str())
            .to_owned();
        parents.push(RemixParent {
            hash: resolved_hash,
            role: source.role.clone(),
            title: str_field("title"),
            category: str_field("category"),
            play_url: str_field("playUrl"),
            author: str_field("authorName"),
        });
    }

    let title = derive_title(options, &parents);
    let license = options
        .license
        .clone()
        .unwrap_or_else(|| String::from("MIT"));
    let target_dir = derive_target_dir(&context.cwd, options, &title, &parents);
    let target_rel = relative_path_or_dot(&context.cwd, &target_dir);

    if options.dry_run {
        return Ok(render_remix_dry_run(
            &parents,
            &target_rel,
            &title,
            &license,
        ));
    }

    // Guard: don't clobber a non-empty, non-remix target unless the user named it.
    if target_dir.is_dir() {
        let has_other_content = fs::read_dir(&target_dir)
            .map(|entries| {
                entries.flatten().any(|entry| {
                    let name = entry.file_name();
                    let name = name.to_string_lossy();
                    name != ".remix-sources" && name != "clide.json" && name != ".git"
                })
            })
            .unwrap_or(false);
        if has_other_content && options.dir.is_none() && target_dir != context.cwd {
            return Err(format!(
                "Target directory already exists and is not empty: {target_rel}. Pass `dir` to choose another.",
            ));
        }
    }

    let sources_dir = target_dir.join(".remix-sources");
    fs::create_dir_all(&sources_dir)
        .map_err(|error| format!("Failed to create {}: {error}", sources_dir.display()))?;

    let mut downloaded: Vec<(String, usize)> = Vec::new();
    for parent in &parents {
        let remix = remix_app(&auth, &parent.hash)?;
        let zip_bytes = download_workspace_zip(&auth, &remix.workspace_hash_key)?;
        let dest = sources_dir.join(&parent.hash);
        let file_count = extract_zip(&zip_bytes, &dest)?;
        downloaded.push((parent.hash.clone(), file_count));
    }

    // Scaffold project metadata. clide.json is authoritative for Publish.
    let written = write_project_files(&target_dir, options, &parents, &title, &license)?;

    Ok(render_remix_success(
        &target_rel,
        &title,
        &license,
        &downloaded,
        &written,
    ))
}

// ── input parsing helpers ───────────────────────────────────────────────────

fn is_valid_remix_hash(value: &str) -> bool {
    let len = value.len();
    (3..=64).contains(&len)
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

fn is_role_word(value: &str) -> bool {
    let mut chars = value.chars();
    match chars.next() {
        Some(first) if first.is_ascii_alphabetic() => {}
        _ => return false,
    }
    value.len() <= 41
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == ' ' || c == '_' || c == '-')
}

/// Split `hash=role` (URLs keep their full value). A digit-initial value after
/// `=` is not a valid role, so query strings are left intact.
fn split_source_token(raw: &str) -> (String, Option<String>) {
    if let Some(index) = raw.find('=') {
        let role = raw[index + 1..].trim();
        if is_role_word(role) {
            return (raw[..index].trim().to_owned(), Some(role.to_owned()));
        }
    }
    (raw.to_owned(), None)
}

/// Reduce any user-supplied reference to a bare Square app hashKey.
fn normalize_remix_token(token: &str) -> Result<String, String> {
    let mut t = token
        .trim()
        .trim_matches(|c| "<>\"'`".contains(c))
        .trim()
        .to_owned();
    if t.is_empty() {
        return Err(String::from("Empty remix source token"));
    }
    if let Some(pos) = t.find("://") {
        t = t[pos + 3..].to_owned();
    }
    t = t.split_whitespace().next().unwrap_or("").to_owned();
    t = t.split('?').next().unwrap_or("").to_owned();
    t = t.split('#').next().unwrap_or("").to_owned();

    let (host, path) = match t.find('/') {
        Some(index) => (t[..index].to_owned(), t[index + 1..].to_owned()),
        None => (t.clone(), String::new()),
    };
    let lower = host.to_ascii_lowercase();
    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

    let hash = if lower.ends_with(".clide.app") {
        return Err(format!(
            "Invalid remix source: {token} is a clide.app play URL/handle, not necessarily a Square hashKey. Use the app's hashKey from Square or an 8x.gg hash URL."
        ));
    } else if lower == "8x.gg" || lower == "www.8x.gg" || lower == "x.8x.gg" {
        let segs: &[&str] = if segments
            .first()
            .is_some_and(|s| s.eq_ignore_ascii_case("pub"))
        {
            &segments[1..]
        } else {
            &segments[..]
        };
        segs.first().copied().unwrap_or("").to_owned()
    } else if let Some(stripped) = lower.strip_suffix(".8x.gg") {
        host[..stripped.len()].to_owned()
    } else if !host.contains('.') {
        host.clone()
    } else {
        host.split('.').next().unwrap_or("").to_owned()
    };

    if !is_valid_remix_hash(&hash) {
        return Err(format!(
            "Invalid remix source (could not resolve a hashKey): {token}"
        ));
    }
    Ok(hash)
}

fn derive_title(options: &RemixOptions, parents: &[RemixParent]) -> String {
    if let Some(title) = options
        .title
        .as_ref()
        .map(|t| t.trim())
        .filter(|t| !t.is_empty())
    {
        return title.to_owned();
    }
    let names: Vec<&str> = parents
        .iter()
        .map(|p| p.title.trim())
        .filter(|t| !t.is_empty())
        .collect();
    if !names.is_empty() {
        let joined = names.join(" x ");
        return joined.chars().take(80).collect();
    }
    format!(
        "Remix of {}",
        parents
            .iter()
            .map(|p| p.hash.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn slugify(text: &str) -> String {
    let mut out = String::new();
    let mut prev_dash = false;
    for c in text.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
            prev_dash = false;
        } else if !prev_dash && !out.is_empty() {
            out.push('-');
            prev_dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    out.chars().take(48).collect()
}

fn derive_target_dir(
    cwd: &Path,
    options: &RemixOptions,
    title: &str,
    parents: &[RemixParent],
) -> PathBuf {
    if let Some(dir) = &options.dir {
        return cwd.join(dir);
    }
    let mut slug = slugify(title);
    if slug.is_empty() {
        slug = format!(
            "remix-{}",
            parents
                .iter()
                .map(|p| p.hash.chars().take(6).collect::<String>())
                .collect::<Vec<_>>()
                .join("-")
        );
    }
    if slug.is_empty() {
        slug = String::from("remixed-game");
    }
    cwd.join(slug)
}

fn relative_path_or_dot(base: &Path, child: &Path) -> String {
    let rel = child
        .strip_prefix(base)
        .map(|path| path.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|_| child.to_string_lossy().replace('\\', "/"));
    if rel.is_empty() {
        String::from(".")
    } else {
        rel
    }
}

// ── HTTP ────────────────────────────────────────────────────────────────────

fn resolve_remix_auth() -> Result<RemixAuth, String> {
    let token = crate::auth::effective_paean_token()
        .or_else(|| {
            ["PAEAN_AUTH_TOKEN", "PAEAN_API_KEY"]
                .into_iter()
                .find_map(|key| std::env::var(key).ok())
                .map(|value| value.trim().to_owned())
                .filter(|value| !value.is_empty())
        })
        .ok_or_else(|| {
            String::from(
                "Paean login is missing or expired. Run `tide auth login` or set PAEAN_AUTH_TOKEN, then retry Remix.",
            )
        })?;
    let base_url = std::env::var("PAEAN_API_BASE")
        .or_else(|_| std::env::var("ZERO_API_BASE"))
        .or_else(|_| std::env::var("ZERO_CLI_BASE_URL"))
        .or_else(|_| std::env::var("PAEAN_API_BASE_URL"))
        .or_else(|_| std::env::var("CLIDE_API_BASE_URL"))
        .unwrap_or_else(|_| String::from("https://api.paean.ai"));
    Ok(RemixAuth {
        base_url: normalize_remix_base_url(&base_url),
        token,
    })
}

fn normalize_remix_base_url(raw: &str) -> String {
    let mut base = raw.trim().trim_end_matches('/').to_owned();
    if base.ends_with("/zero") {
        base.truncate(base.len() - "/zero".len());
    }
    if base.is_empty() {
        String::from("https://api.paean.ai")
    } else {
        base
    }
}

fn http_client(timeout_secs: u64) -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .build()
        .map_err(|error| format!("Failed to build remix HTTP client: {error}"))
}

/// GET a `{success,data}` JSON envelope and return the `data` payload value.
fn fetch_app_detail(auth: &RemixAuth, hash: &str) -> Result<serde_json::Value, String> {
    let encoded = percent_encode_path_segment(hash);
    let client = http_client(60)?;
    let response = client
        .get(format!("{}/square/apps/{encoded}", auth.base_url))
        .bearer_auth(&auth.token)
        .send()
        .map_err(|error| format!("App lookup failed for {hash}: {error}"))?;
    let value = read_json_envelope(response, &format!("App lookup for {hash}"))?;
    if value
        .get("hashKey")
        .and_then(serde_json::Value::as_str)
        .map(str::is_empty)
        .unwrap_or(true)
    {
        return Err(format!("App not found or not listed: {hash}"));
    }
    Ok(value)
}

#[derive(Debug, Deserialize)]
struct RemixResponse {
    #[serde(rename = "workspaceHashKey")]
    workspace_hash_key: String,
}

fn remix_app(auth: &RemixAuth, hash: &str) -> Result<RemixResponse, String> {
    #[derive(Serialize)]
    struct EmptyRequest {}
    let encoded = percent_encode_path_segment(hash);
    let client = http_client(120)?;
    let response = client
        .post(format!("{}/square/apps/{encoded}/remix", auth.base_url))
        .bearer_auth(&auth.token)
        .json(&EmptyRequest {})
        .send()
        .map_err(|error| format!("Remix request failed for {hash}: {error}"))?;
    let value = read_json_envelope(response, &format!("Remix for {hash}"))?;
    let decoded: RemixResponse = serde_json::from_value(value)
        .map_err(|error| format!("Unexpected remix response for {hash}: {error}"))?;
    if decoded.workspace_hash_key.trim().is_empty() {
        return Err(format!(
            "Remix response missing workspaceHashKey for {hash}"
        ));
    }
    Ok(decoded)
}

fn download_workspace_zip(auth: &RemixAuth, workspace_hash_key: &str) -> Result<Vec<u8>, String> {
    let encoded = percent_encode_path_segment(workspace_hash_key);
    let client = http_client(300)?;
    let response = client
        .get(format!(
            "{}/v2/workspace/{encoded}/export/zip",
            auth.base_url
        ))
        .bearer_auth(&auth.token)
        .send()
        .map_err(|error| format!("Workspace export failed: {error}"))?;
    if matches!(response.status().as_u16(), 401 | 403) {
        return Err(String::from(
            "Paean login is missing or expired. Refresh the token, then retry Remix.",
        ));
    }
    if !response.status().is_success() {
        let status = response.status().as_u16();
        let body = response.text().unwrap_or_default();
        return Err(format!(
            "Workspace export {status}: {}",
            body.trim().chars().take(300).collect::<String>()
        ));
    }
    let bytes = response
        .bytes()
        .map_err(|error| format!("Failed to read workspace export: {error}"))?;
    if bytes.is_empty() {
        return Err(String::from("Workspace export returned an empty archive."));
    }
    Ok(bytes.to_vec())
}

/// Validate the `{success,data}` envelope and return the `data` payload.
fn read_json_envelope(
    response: reqwest::blocking::Response,
    label: &str,
) -> Result<serde_json::Value, String> {
    if matches!(response.status().as_u16(), 401 | 403) {
        return Err(String::from(
            "Paean login is missing or expired. Run `tide auth login` or set PAEAN_AUTH_TOKEN, then retry Remix.",
        ));
    }
    let status = response.status().as_u16();
    let ok = response.status().is_success();
    let body = response
        .text()
        .map_err(|error| format!("Failed to read {label} response: {error}"))?;
    let value: serde_json::Value = serde_json::from_str(&body)
        .map_err(|error| format!("{label} returned non-JSON {status}: {error}"))?;
    if !ok || value.get("success").and_then(serde_json::Value::as_bool) == Some(false) {
        let message = value
            .get("error")
            .or_else(|| value.get("message"))
            .or_else(|| value.get("reason"))
            .and_then(serde_json::Value::as_str)
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| body.trim().chars().take(300).collect());
        return Err(format!("{label} {status}: {message}"));
    }
    Ok(value
        .get("data")
        .filter(|data| !data.is_null())
        .cloned()
        .unwrap_or(value))
}

// ── zip extraction (zip-slip safe) ──────────────────────────────────────────

fn extract_zip(bytes: &[u8], dest: &Path) -> Result<usize, String> {
    fs::create_dir_all(dest)
        .map_err(|error| format!("Failed to create {}: {error}", dest.display()))?;
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes))
        .map_err(|error| format!("Failed to read source archive: {error}"))?;
    let mut file_count = 0usize;
    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|error| format!("Failed to read archive entry {index}: {error}"))?;
        // `enclosed_name` rejects absolute paths and `..` traversal (zip-slip).
        let Some(relative) = entry.enclosed_name() else {
            continue;
        };
        let out_path = dest.join(&relative);
        if entry.is_dir() {
            fs::create_dir_all(&out_path)
                .map_err(|error| format!("Failed to create {}: {error}", out_path.display()))?;
            continue;
        }
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("Failed to create {}: {error}", parent.display()))?;
        }
        let mut out = fs::File::create(&out_path)
            .map_err(|error| format!("Failed to write {}: {error}", out_path.display()))?;
        io::copy(&mut entry, &mut out)
            .map_err(|error| format!("Failed to extract {}: {error}", out_path.display()))?;
        file_count += 1;
    }
    Ok(file_count)
}

// ── project scaffolding (clide.json / LICENSE / .clideignore) ────────────────

fn write_project_files(
    target_dir: &Path,
    options: &RemixOptions,
    parents: &[RemixParent],
    title: &str,
    license: &str,
) -> Result<Vec<String>, String> {
    let mut written = Vec::new();

    let manifest_path = target_dir.join("clide.json");
    if !manifest_path.exists() {
        let manifest = build_manifest_json(options, parents, title, license);
        let data = serde_json::to_string_pretty(&manifest)
            .map_err(|error| format!("Failed to encode clide.json: {error}"))?;
        fs::write(&manifest_path, format!("{data}\n"))
            .map_err(|error| format!("Failed to write clide.json: {error}"))?;
        written.push(String::from("clide.json"));
    }

    let license_path = target_dir.join("LICENSE");
    if !license_path.exists() {
        let holder = std::env::var("PAEAN_AUTHOR").unwrap_or_default();
        fs::write(
            &license_path,
            license_body(license, current_year(), &holder),
        )
        .map_err(|error| format!("Failed to write LICENSE: {error}"))?;
        written.push(String::from("LICENSE"));
    }

    let ignore_path = target_dir.join(".clideignore");
    if !ignore_path.exists() {
        fs::write(&ignore_path, clideignore_text())
            .map_err(|error| format!("Failed to write .clideignore: {error}"))?;
        written.push(String::from(".clideignore"));
    }

    Ok(written)
}

fn build_manifest_json(
    options: &RemixOptions,
    parents: &[RemixParent],
    title: &str,
    license: &str,
) -> serde_json::Value {
    let parents_json: Vec<serde_json::Value> = parents
        .iter()
        .map(|parent| {
            let mut entry = serde_json::Map::new();
            entry.insert("hashKey".into(), parent.hash.clone().into());
            entry.insert(
                "role".into(),
                parent.role.clone().unwrap_or_default().into(),
            );
            entry.insert("weight".into(), serde_json::json!(1));
            if !parent.title.is_empty() {
                entry.insert("title".into(), parent.title.clone().into());
            }
            if !parent.play_url.is_empty() {
                entry.insert("playUrl".into(), parent.play_url.clone().into());
            }
            if !parent.category.is_empty() {
                entry.insert("category".into(), parent.category.clone().into());
            }
            if !parent.author.is_empty() {
                entry.insert("author".into(), parent.author.clone().into());
            }
            serde_json::Value::Object(entry)
        })
        .collect();

    let mut manifest = serde_json::Map::new();
    manifest.insert("schemaVersion".into(), serde_json::json!(1));
    manifest.insert("title".into(), title.into());
    if let Some(summary) = options.summary.as_ref().filter(|s| !s.is_empty()) {
        manifest.insert("summary".into(), summary.clone().into());
    }
    if let Some(category) = options.category.as_ref().filter(|c| !c.is_empty()) {
        manifest.insert("category".into(), category.clone().into());
    }
    manifest.insert("tags".into(), serde_json::json!([]));
    manifest.insert("license".into(), license.into());

    let mut remix = serde_json::Map::new();
    if let Some(first) = parents.first() {
        remix.insert("parent".into(), first.hash.clone().into());
    }
    remix.insert("parents".into(), serde_json::Value::Array(parents_json));
    remix.insert("remixedAt".into(), iso_timestamp().into());
    manifest.insert("remix".into(), serde_json::Value::Object(remix));

    serde_json::Value::Object(manifest)
}

fn clideignore_text() -> String {
    [
        "# .clideignore - files and folders excluded from Clide publish.",
        "# Gitignore-style: one pattern per line, # for comments.",
        "# Raw upstream sources are kept locally for remixing but never published.",
        "",
        ".remix-sources/",
        ".clide/",
        "clide.json",
        ".env",
        ".env.*",
        "*.pem",
        "*.key",
        "node_modules/",
        ".git/",
        "*.log",
        ".DS_Store",
        "Thumbs.db",
        "*.tmp",
        "",
    ]
    .join("\n")
}

fn license_body(spdx: &str, year: i32, holder: &str) -> String {
    let id = spdx.trim();
    let who = if holder.trim().is_empty() {
        "the author"
    } else {
        holder.trim()
    };
    match id {
        "MIT" => format!(
            "MIT License\n\nCopyright (c) {year} {who}\n\nPermission is hereby granted, free of charge, to any person obtaining a copy\nof this software and associated documentation files (the \"Software\"), to deal\nin the Software without restriction, including without limitation the rights\nto use, copy, modify, merge, publish, distribute, sublicense, and/or sell\ncopies of the Software, and to permit persons to whom the Software is\nfurnished to do so, subject to the following conditions:\n\nThe above copyright notice and this permission notice shall be included in all\ncopies or substantial portions of the Software.\n\nTHE SOFTWARE IS PROVIDED \"AS IS\", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR\nIMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,\nFITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE\nAUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER\nLIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,\nOUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE\nSOFTWARE.\n"
        ),
        "CC0-1.0" => format!(
            "This work is dedicated to the public domain under CC0 1.0 Universal.\n\nCopyright (c) {year} {who}\n\nSee https://creativecommons.org/publicdomain/zero/1.0/ for the full text.\n"
        ),
        "UNLICENSED" => format!(
            "Copyright (c) {year} {who}\n\nAll rights reserved. Proprietary; do not copy, modify, or distribute without permission.\n"
        ),
        other => format!(
            "{other} License\n\nCopyright (c) {year} {who}\n\nThis work is licensed under the {other} license.\nFull text: https://spdx.org/licenses/{other}.html\n"
        ),
    }
}

fn current_year() -> i32 {
    time::OffsetDateTime::now_utc().year()
}

fn iso_timestamp() -> String {
    let now = time::OffsetDateTime::now_utc();
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        now.year(),
        u8::from(now.month()),
        now.day(),
        now.hour(),
        now.minute(),
        now.second()
    )
}

// ── rendering ────────────────────────────────────────────────────────────────

fn render_remix_dry_run(
    parents: &[RemixParent],
    target_rel: &str,
    title: &str,
    license: &str,
) -> String {
    let mut lines = vec![
        String::from("Remix plan (dry run — no downloads, no files written):"),
        format!("  Target:   {target_rel}"),
        format!("  Title:    {title}"),
        format!("  License:  {license}"),
        format!("  Sources:  {}", parents.len()),
    ];
    for parent in parents {
        let role = parent
            .role
            .as_ref()
            .map(|r| format!(" [{r}]"))
            .unwrap_or_default();
        let name = if parent.title.is_empty() {
            String::new()
        } else {
            format!(" — {}", parent.title)
        };
        lines.push(format!("    - {}{role}{name}", parent.hash));
    }
    lines.push(String::from(
        "Remixing records lineage for each source (credits the upstream creators).",
    ));
    lines.join("\n")
}

fn render_remix_success(
    target_rel: &str,
    title: &str,
    license: &str,
    downloaded: &[(String, usize)],
    written: &[String],
) -> String {
    let mut lines = vec![
        format!("Remixed into: {target_rel}"),
        format!("  Title:    {title}"),
        format!("  License:  {license}"),
        format!("  Sources:  {}", downloaded.len()),
    ];
    for (hash, files) in downloaded {
        lines.push(format!(
            "    - {hash} → .remix-sources/{hash}/ ({files} files)"
        ));
    }
    if !written.is_empty() {
        lines.push(format!("  Wrote:    {}", written.join(", ")));
    }
    lines.push(String::from(
        "Build the new game in the target directory using the downloaded sources under .remix-sources/, add top-level favicon.svg and 800x400 banner.jpg when possible, then run the Publish tool from there.",
    ));
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_every_reference_form() {
        assert_eq!(
            normalize_remix_token("aB3dEf12").expect("remix test"),
            "aB3dEf12"
        );
        assert_eq!(
            normalize_remix_token("aB3dEf12.8x.gg").expect("remix test"),
            "aB3dEf12"
        );
        assert_eq!(
            normalize_remix_token("https://8x.gg/aB3dEf12").expect("remix test"),
            "aB3dEf12"
        );
        assert_eq!(
            normalize_remix_token("https://x.8x.gg/pub/aB3dEf12/").expect("remix test"),
            "aB3dEf12"
        );
        assert!(normalize_remix_token("ab").is_err());
        assert!(normalize_remix_token("../etc/passwd").is_err());
        assert!(normalize_remix_token("aB3dEf12.clide.app").is_err());
        assert!(normalize_remix_token("https://aB3dEf12.clide.app/play?x=1#f").is_err());
        assert!(normalize_remix_token("").is_err());
    }

    #[test]
    fn splits_role_but_not_query_strings() {
        assert_eq!(
            split_source_token("hash1=gameplay"),
            (String::from("hash1"), Some(String::from("gameplay")))
        );
        assert_eq!(split_source_token("hash1"), (String::from("hash1"), None));
        assert_eq!(
            split_source_token("https://h.clide.app/?ref=1"),
            (String::from("https://h.clide.app/?ref=1"), None)
        );
    }

    #[test]
    fn parses_sources_array_and_dedupes() {
        let options = RemixOptions::from_input(&serde_json::json!({
            "sources": ["hash1aaa=gameplay", "hash2bbb.8x.gg", "hash1aaa"],
            "dir": "newgame",
            "dry_run": true,
        }))
        .expect("remix test");
        assert_eq!(options.sources.len(), 2);
        assert_eq!(options.sources[0].hash, "hash1aaa");
        assert_eq!(options.sources[0].role.as_deref(), Some("gameplay"));
        assert_eq!(options.sources[1].hash, "hash2bbb");
        assert!(options.dry_run);
    }

    #[test]
    fn requires_at_least_one_source() {
        assert!(RemixOptions::from_input(&serde_json::json!({})).is_err());
    }

    #[test]
    fn license_and_manifest_render() {
        assert!(license_body("MIT", 2026, "Ada").contains("MIT License"));
        assert!(license_body("Apache-2.0", 2026, "").contains("spdx.org/licenses/Apache-2.0"));
        let parents = vec![RemixParent {
            hash: String::from("h1"),
            role: Some(String::from("gameplay")),
            title: String::from("Galaxy"),
            category: String::from("arcade"),
            play_url: String::new(),
            author: String::new(),
        }];
        let options = RemixOptions {
            sources: vec![],
            dir: None,
            title: Some(String::from("New")),
            summary: None,
            category: None,
            license: None,
            dry_run: false,
        };
        let manifest = build_manifest_json(&options, &parents, "New", "MIT");
        assert_eq!(manifest["remix"]["parent"], "h1");
        assert_eq!(manifest["remix"]["parents"][0]["role"], "gameplay");
    }
}
