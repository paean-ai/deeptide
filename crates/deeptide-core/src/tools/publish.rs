//! Publish tool: package a project and upload it to the publish API.
//!
//! Shared infrastructure lives in the parent module (reached via `use super::*`).

use super::*;

#[derive(Debug, Default, Clone, Copy)]
pub struct PublishTool;

impl Tool for PublishTool {
    fn name(&self) -> &'static str {
        "Publish"
    }

    fn description(&self) -> &'static str {
        "Prepare, inspect, or delete a static frontend publish on clide.app."
    }

    fn is_read_only(&self) -> bool {
        false
    }

    fn call(&self, input: serde_json::Value, context: &ToolContext) -> ToolResult {
        let options = match PublishOptions::from_input(&input) {
            Ok(options) => options,
            Err(message) => return ToolResult::error(message),
        };
        if options.status {
            return ToolResult::text(render_publish_status(context));
        }
        if options.delete {
            let handle = options
                .handle
                .clone()
                .or_else(|| load_publish_state(context).and_then(|state| state.handle));
            let Some(handle) = handle.filter(|value| !value.trim().is_empty()) else {
                return ToolResult::error(
                    "No handle specified and no saved `.clide/publish.json` handle found.",
                );
            };
            return match delete_publish(context, &handle) {
                Ok(result) => ToolResult::text(result),
                Err(message) => ToolResult::error(message),
            };
        }

        let publish_dir = match resolve_publish_dir(context, options.dir.as_deref()) {
            Ok(path) => path,
            Err(message) => return ToolResult::error(message),
        };
        if let Err(message) = ensure_clideignore_safety_defaults(&context.cwd) {
            return ToolResult::error(message);
        }
        let patterns = load_clideignore_patterns(&context.cwd);
        let files = match collect_publish_files(&context.cwd, &publish_dir, &patterns) {
            Ok(files) => files,
            Err(message) => return ToolResult::error(message),
        };
        if !files.iter().any(|file| file == "index.html") {
            return ToolResult::error(
                "Publish archive must include top-level `index.html` after `.clideignore` filtering.",
            );
        }
        if files.is_empty() {
            return ToolResult::error("Publish archive would contain no files.");
        }

        let state_handle = load_publish_state(context).and_then(|state| state.handle);
        let handle = if options.random {
            None
        } else {
            options.handle.clone().or(state_handle)
        };
        let plan =
            match build_publish_plan(&context.cwd, &publish_dir, &files, handle, options.random) {
                Ok(plan) => plan,
                Err(message) => return ToolResult::error(message),
            };
        if options.dry_run {
            return ToolResult::text(render_publish_dry_run(&plan));
        }

        match upload_publish(context, &publish_dir, &files, &plan) {
            Ok(result) => ToolResult::text(result),
            Err(message) => ToolResult::error(message),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PublishOptions {
    dir: Option<String>,
    handle: Option<String>,
    random: bool,
    delete: bool,
    dry_run: bool,
    status: bool,
}

impl PublishOptions {
    fn from_input(input: &serde_json::Value) -> Result<Self, String> {
        let option = |key: &str| {
            input
                .get(key)
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned)
        };
        let options = Self {
            dir: option("dir"),
            handle: option("handle"),
            random: input
                .get("random")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false),
            delete: input
                .get("delete")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false),
            dry_run: input
                .get("dry_run")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false),
            status: input
                .get("status")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false),
        };
        if options.random && options.handle.is_some() {
            return Err(String::from("Use either random or handle, not both."));
        }
        if options.delete && options.random {
            return Err(String::from("Use either delete or random, not both."));
        }
        if options.delete && options.dir.is_some() {
            return Err(String::from(
                "dir is only valid when publishing, not deleting.",
            ));
        }
        if options.status
            && (options.delete
                || options.dry_run
                || options.dir.is_some()
                || options.handle.is_some()
                || options.random)
        {
            return Err(String::from(
                "status cannot be combined with publish/delete options.",
            ));
        }
        Ok(options)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct PublishState {
    #[serde(skip_serializing_if = "Option::is_none")]
    handle: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<String>,
    #[serde(rename = "publishDir")]
    #[serde(skip_serializing_if = "Option::is_none")]
    publish_dir: Option<String>,
    #[serde(rename = "fileCount")]
    #[serde(skip_serializing_if = "Option::is_none")]
    file_count: Option<u64>,
    #[serde(rename = "totalBytes")]
    #[serde(skip_serializing_if = "Option::is_none")]
    total_bytes: Option<u64>,
    #[serde(rename = "publishedAt")]
    #[serde(skip_serializing_if = "Option::is_none")]
    published_at: Option<String>,
    #[serde(rename = "deletedAt")]
    #[serde(skip_serializing_if = "Option::is_none")]
    deleted_at: Option<String>,
    #[serde(rename = "lastDeletedHandle")]
    #[serde(skip_serializing_if = "Option::is_none")]
    last_deleted_handle: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PublishPlan {
    publish_dir_relative: String,
    handle_description: String,
    handle: Option<String>,
    random: bool,
    file_count: usize,
    total_bytes: u64,
    has_index: bool,
    sample_files: Vec<String>,
    notes: Vec<String>,
}

const CLIDEIGNORE_FILE: &str = ".clideignore";
const CLIDEIGNORE_HEADER: &str = "# Added by Clide publish safety defaults";
const CLIDEIGNORE_SAFETY_PATTERNS: &[&str] = &[
    ".clide/",
    ".env",
    ".env.*",
    "*.pem",
    "*.key",
    "*.p12",
    "*.pfx",
    "id_rsa*",
    ".npmrc",
    ".aws/",
    ".gcloud/",
    "service-account*.json",
    "secrets/",
    "private/",
    "node_modules/",
    ".git/",
    ".next/cache/",
    "dist/*.map",
    ".vscode/",
    ".idea/",
    "coverage/",
    "*.log",
    ".DS_Store",
    "Thumbs.db",
    "*.tmp",
];

fn render_publish_status(context: &ToolContext) -> String {
    let Some(state) = load_publish_state(context) else {
        return String::from("No saved clide.app publish state at .clide/publish.json");
    };
    let mut lines = vec![String::from("Clide publish status")];
    if let Some(handle) = state.handle {
        lines.push(format!("  Handle:      {handle}"));
    }
    if let Some(url) = state.url {
        lines.push(format!("  URL:         {url}"));
    }
    if let Some(dir) = state.publish_dir {
        lines.push(format!("  Directory:   {dir}"));
    }
    if let Some(file_count) = state.file_count {
        lines.push(format!("  Files:       {file_count}"));
    }
    if let Some(total_bytes) = state.total_bytes {
        lines.push(format!("  Bytes:       {total_bytes}"));
    }
    if let Some(published_at) = state.published_at {
        lines.push(format!("  Published:   {published_at}"));
    }
    if let Some(deleted_at) = state.deleted_at {
        lines.push(format!("  Deleted:     {deleted_at}"));
    }
    if let Some(handle) = state.last_deleted_handle {
        lines.push(format!("  Last deleted handle: {handle}"));
    }
    lines.push(String::from("  State:       .clide/publish.json"));
    lines.join("\n")
}

fn load_publish_state(context: &ToolContext) -> Option<PublishState> {
    let path = context.cwd.join(".clide").join("publish.json");
    let text = fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

fn resolve_publish_dir(
    context: &ToolContext,
    explicit_dir: Option<&str>,
) -> Result<PathBuf, String> {
    if let Some(dir) = explicit_dir {
        let path = context.resolve_path(dir);
        if !path.is_dir() {
            return Err(format!("Publish directory does not exist: {dir}"));
        }
        if !path.join("index.html").is_file() {
            return Err(format!(
                "Publish directory must contain a top-level `index.html`: {dir}"
            ));
        }
        return Ok(path);
    }

    for candidate in ["dist", "build", "out", ".output/public", "public"] {
        let path = context.cwd.join(candidate);
        if path.is_dir() && path.join("index.html").is_file() {
            return Ok(normalize_path(path));
        }
    }
    if context.cwd.join("index.html").is_file() {
        return Ok(normalize_path(context.cwd.clone()));
    }
    if package_json_has_build_script(&context.cwd) {
        return Err(String::from(
            "No built output with top-level `index.html` found. This project has a package.json build script; run the build first, then publish dist, build, out, .output/public, public, or project root.",
        ));
    }
    Err(String::from(
        "No publishable static directory found. Expected top-level `index.html` in `dist`, `build`, `out`, `.output/public`, `public`, or project root.",
    ))
}

fn package_json_has_build_script(project_root: &Path) -> bool {
    let Ok(text) = fs::read_to_string(project_root.join("package.json")) else {
        return false;
    };
    serde_json::from_str::<serde_json::Value>(&text)
        .ok()
        .and_then(|value| value.get("scripts").cloned())
        .and_then(|scripts| scripts.get("build").cloned())
        .is_some()
}

fn ensure_clideignore_safety_defaults(project_root: &Path) -> Result<(), String> {
    let path = project_root.join(CLIDEIGNORE_FILE);
    let exists = path.exists();
    let mut text = if exists {
        fs::read_to_string(&path)
            .map_err(|error| format!("Failed to read .clideignore: {error}"))?
    } else {
        String::from(
            "# .clideignore - files and folders excluded from Clide publish.\n# Gitignore-style: one pattern per line, # for comments.\n# The .clideignore file itself is never published.\n\n",
        )
    };
    let existing = load_clideignore_patterns_from_text(&text)
        .into_iter()
        .map(|pattern| normalize_publish_ignore_pattern(&pattern))
        .collect::<std::collections::BTreeSet<_>>();
    let missing = CLIDEIGNORE_SAFETY_PATTERNS
        .iter()
        .filter(|pattern| !existing.contains(normalize_publish_ignore_pattern(pattern).as_str()))
        .copied()
        .collect::<Vec<_>>();
    if exists && missing.is_empty() {
        return Ok(());
    }
    if !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
    if !text.contains(CLIDEIGNORE_HEADER) {
        if !text.ends_with("\n\n") {
            text.push('\n');
        }
        text.push_str(CLIDEIGNORE_HEADER);
        text.push('\n');
    }
    for pattern in missing {
        text.push_str(pattern);
        text.push('\n');
    }
    fs::write(&path, text).map_err(|error| format!("Failed to write .clideignore: {error}"))
}

fn load_clideignore_patterns(project_root: &Path) -> Vec<String> {
    fs::read_to_string(project_root.join(CLIDEIGNORE_FILE))
        .map(|text| load_clideignore_patterns_from_text(&text))
        .unwrap_or_default()
}

fn load_clideignore_patterns_from_text(text: &str) -> Vec<String> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(normalize_publish_ignore_pattern)
        .filter(|line| !line.is_empty())
        .collect()
}

fn normalize_publish_ignore_pattern(pattern: &str) -> String {
    pattern.trim().trim_end_matches('/').to_owned()
}

fn collect_publish_files(
    project_root: &Path,
    publish_dir: &Path,
    patterns: &[String],
) -> Result<Vec<String>, String> {
    let mut files = Vec::new();
    collect_publish_files_inner(project_root, publish_dir, publish_dir, patterns, &mut files)?;
    files.sort();
    Ok(files)
}

fn collect_publish_files_inner(
    project_root: &Path,
    publish_root: &Path,
    dir: &Path,
    patterns: &[String],
    files: &mut Vec<String>,
) -> Result<(), String> {
    let entries = fs::read_dir(dir).map_err(|error| {
        format!(
            "Failed to read publish directory {}: {error}",
            dir.display()
        )
    })?;
    for entry in entries {
        let entry = entry.map_err(|error| format!("Failed to read directory entry: {error}"))?;
        let path = entry.path();
        let rel_publish = relative_path_string(publish_root, &path);
        let rel_project = relative_path_string(project_root, &path);
        if rel_publish.contains('\n') {
            return Err(format!(
                "Cannot publish files with newline in path: {rel_publish}"
            ));
        }
        if publish_path_is_ignored(&rel_project, &rel_publish, patterns) {
            continue;
        }
        let metadata = entry
            .metadata()
            .map_err(|error| format!("Failed to inspect {}: {error}", path.display()))?;
        if metadata.is_dir() {
            collect_publish_files_inner(project_root, publish_root, &path, patterns, files)?;
        } else if metadata.is_file() {
            files.push(rel_publish);
        }
    }
    Ok(())
}

fn publish_path_is_ignored(rel_project: &str, rel_publish: &str, patterns: &[String]) -> bool {
    if rel_project == CLIDEIGNORE_FILE || rel_publish == CLIDEIGNORE_FILE {
        return true;
    }
    if rel_project == ".clide" || rel_project.starts_with(".clide/") {
        return true;
    }
    patterns.iter().any(|pattern| {
        publish_pattern_matches(pattern, rel_project)
            || publish_pattern_matches(pattern, rel_publish)
    })
}

fn publish_pattern_matches(pattern: &str, relative_path: &str) -> bool {
    let pattern = normalize_publish_ignore_pattern(pattern)
        .trim_matches('/')
        .to_owned();
    let relative_path = relative_path
        .replace('\\', "/")
        .trim_matches('/')
        .to_owned();
    if pattern.is_empty() || relative_path.is_empty() {
        return false;
    }
    if pattern.contains('/') {
        return GlobMatcher::new(&pattern).matches(Path::new(&relative_path))
            || relative_path.starts_with(&(pattern.clone() + "/"))
            || GlobMatcher::new(&(pattern + "/*")).matches(Path::new(&relative_path));
    }
    relative_path
        .split('/')
        .any(|part| matches_glob_segment(part, &pattern))
        || matches_glob_segment(&relative_path, &pattern)
        || relative_path.starts_with(&(pattern + "/"))
}

fn build_publish_plan(
    project_root: &Path,
    publish_dir: &Path,
    files: &[String],
    handle: Option<String>,
    random: bool,
) -> Result<PublishPlan, String> {
    let mut total_bytes = 0u64;
    for file in files {
        let metadata = fs::metadata(publish_dir.join(file))
            .map_err(|error| format!("Failed to inspect {file}: {error}"))?;
        total_bytes = total_bytes.saturating_add(metadata.len());
    }
    let mut notes = Vec::new();
    if random {
        notes.push(String::from(
            "A new random handle will be assigned by the server.",
        ));
    } else if handle.is_none() {
        notes.push(String::from(
            "No saved or explicit handle; server will assign a handle.",
        ));
    } else {
        notes.push(String::from(
            "Existing or explicit handle will be overwritten if it already exists.",
        ));
    }
    if files.iter().any(|file| file.ends_with(".map")) {
        notes.push(String::from(
            "Source maps are included; add a .clideignore rule if they should stay private.",
        ));
    }
    Ok(PublishPlan {
        publish_dir_relative: relative_path_string(project_root, publish_dir),
        handle_description: if random {
            String::from("(random)")
        } else {
            handle
                .clone()
                .unwrap_or_else(|| String::from("(server assigned)"))
        },
        handle,
        random,
        file_count: files.len(),
        total_bytes,
        has_index: files.iter().any(|file| file == "index.html"),
        sample_files: files.iter().take(6).cloned().collect(),
        notes,
    })
}

fn render_publish_dry_run(plan: &PublishPlan) -> String {
    let rel_dir = if plan.publish_dir_relative.is_empty() {
        "."
    } else {
        &plan.publish_dir_relative
    };
    let mut lines = vec![
        String::from("Publish dry run: ready"),
        format!("  Directory:  {rel_dir}"),
        format!("  Handle:     {}", plan.handle_description),
        format!("  Files:      {}", plan.file_count),
        format!("  Bytes:      {}", plan.total_bytes),
        format!(
            "  Index:      {}",
            if plan.has_index { "yes" } else { "no" }
        ),
        String::from("  Ignore:     .clideignore safety defaults present"),
    ];
    if !plan.sample_files.is_empty() {
        lines.push(format!("  Sample:     {}", plan.sample_files.join(", ")));
    }
    if !plan.notes.is_empty() {
        lines.push(String::from("  Notes:"));
        lines.extend(plan.notes.iter().map(|note| format!("    - {note}")));
    }
    lines.join("\n")
}

fn relative_path_string(base: &Path, child: &Path) -> String {
    let base = normalize_path(base.to_path_buf());
    let child = normalize_path(child.to_path_buf());
    child
        .strip_prefix(&base)
        .map(|path| path.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|_| child.to_string_lossy().replace('\\', "/"))
        .trim_start_matches('/')
        .to_owned()
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PublishAuth {
    base_url: String,
    token: String,
}

#[derive(Debug, Deserialize)]
struct PublishUploadResponse {
    success: bool,
    data: Option<PublishResult>,
    error: Option<String>,
    reason: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct PublishResult {
    handle: String,
    #[serde(rename = "assignedHandle")]
    assigned_handle: Option<String>,
    url: String,
    #[serde(rename = "shortUrl")]
    short_url: Option<String>,
    #[serde(rename = "fileCount")]
    file_count: u64,
    #[serde(rename = "totalBytes")]
    total_bytes: u64,
    #[serde(default)]
    overwritten: bool,
    #[serde(rename = "archiveUrl")]
    archive_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DeletePublishResult {
    success: bool,
    handle: Option<String>,
    #[serde(rename = "deletedObjects")]
    deleted_objects: Option<u64>,
    error: Option<String>,
    reason: Option<String>,
}

fn upload_publish(
    context: &ToolContext,
    publish_dir: &Path,
    files: &[String],
    plan: &PublishPlan,
) -> Result<String, String> {
    let auth = resolve_publish_auth()?;
    let zip_path = create_publish_zip(publish_dir, files)?;
    let result = (|| {
        validate_publish_login(&auth)?;
        let response = publish_zip(&auth, &zip_path, publish_dir, plan)?;
        save_publish_state(context, &response, publish_dir)?;
        Ok(render_publish_success(&response, context, publish_dir))
    })();
    let _ = fs::remove_file(&zip_path);
    result
}

fn delete_publish(context: &ToolContext, handle: &str) -> Result<String, String> {
    let auth = resolve_publish_auth()?;
    validate_publish_login(&auth)?;
    let result = unpublish_handle(&auth, handle)?;
    mark_publish_deleted(context, handle)?;
    Ok(render_delete_success(&result, handle))
}

fn resolve_publish_auth() -> Result<PublishAuth, String> {
    let token = ["PAEAN_API_TOKEN", "PAEAN_TOKEN", "CLIDE_API_TOKEN"]
        .into_iter()
        .find_map(|key| std::env::var(key).ok())
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            String::from(
                "Paean login is missing or expired. Set PAEAN_API_TOKEN, PAEAN_TOKEN, or CLIDE_API_TOKEN, then retry Publish.",
            )
        })?;
    let base_url = std::env::var("PAEAN_API_BASE_URL")
        .or_else(|_| std::env::var("CLIDE_API_BASE_URL"))
        .unwrap_or_else(|_| String::from("https://api.paean.ai"));
    Ok(PublishAuth {
        base_url: normalize_publish_base_url(&base_url),
        token,
    })
}

fn normalize_publish_base_url(raw: &str) -> String {
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

fn validate_publish_login(auth: &PublishAuth) -> Result<(), String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|error| format!("Failed to build publish HTTP client: {error}"))?;
    let url = format!("{}/publish/clide/check?handle=", auth.base_url);
    let response = client
        .get(url)
        .bearer_auth(&auth.token)
        .send()
        .map_err(|error| format!("Publish login check failed: {error}"))?;
    if matches!(response.status().as_u16(), 401 | 403) {
        return Err(String::from(
            "Paean login is missing or expired. Refresh the publish token, then retry Publish.",
        ));
    }
    if !response.status().is_success() {
        let status = response.status().as_u16();
        let message = publish_response_message(response);
        return Err(format!(
            "Publish API {status}: {}",
            message.unwrap_or_else(|| String::from("login check failed"))
        ));
    }
    Ok(())
}

fn publish_zip(
    auth: &PublishAuth,
    zip_path: &Path,
    publish_dir: &Path,
    plan: &PublishPlan,
) -> Result<PublishResult, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|error| format!("Failed to build publish HTTP client: {error}"))?;
    let file = fs::File::open(zip_path)
        .map_err(|error| format!("Failed to open publish archive: {error}"))?;
    let file_name = publish_dir
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("site");
    let part = reqwest::blocking::multipart::Part::reader(file)
        .file_name(format!("{file_name}.zip"))
        .mime_str("application/zip")
        .map_err(|error| format!("Failed to prepare publish archive part: {error}"))?;
    let mut form = reqwest::blocking::multipart::Form::new().part("archive", part);
    if !plan.random
        && let Some(handle) = plan.handle.as_ref().filter(|value| !value.is_empty())
    {
        form = form.text("handle", handle.clone());
    }
    let response = client
        .post(format!("{}/publish/clide", auth.base_url))
        .bearer_auth(&auth.token)
        .multipart(form)
        .send()
        .map_err(|error| format!("Publish upload failed: {error}"))?;
    if matches!(response.status().as_u16(), 401 | 403) {
        return Err(String::from(
            "Paean login is missing or expired. Refresh the publish token, then retry Publish.",
        ));
    }
    let status = response.status().as_u16();
    let body = response
        .text()
        .map_err(|error| format!("Failed to read publish response: {error}"))?;
    if !(200..=299).contains(&status) {
        return Err(format!(
            "Publish API {status}: {}",
            parse_publish_error_message(&body).unwrap_or_else(|| body.trim().to_owned())
        ));
    }
    let decoded: PublishUploadResponse = serde_json::from_str(&body)
        .map_err(|error| format!("Unexpected publish response: {error}"))?;
    if !decoded.success {
        return Err(format!(
            "Publish API {status}: {}",
            decoded
                .error
                .or(decoded.reason)
                .unwrap_or_else(|| String::from("Publish failed"))
        ));
    }
    decoded
        .data
        .ok_or_else(|| String::from("Unexpected publish response: missing data"))
}

fn unpublish_handle(auth: &PublishAuth, handle: &str) -> Result<DeletePublishResult, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|error| format!("Failed to build publish HTTP client: {error}"))?;
    let encoded = percent_encode_path_segment(handle);
    let response = client
        .delete(format!("{}/publish/{encoded}", auth.base_url))
        .bearer_auth(&auth.token)
        .send()
        .map_err(|error| format!("Publish delete failed: {error}"))?;
    if matches!(response.status().as_u16(), 401 | 403) {
        return Err(String::from(
            "Paean login is missing or expired. Refresh the publish token, then retry Publish.",
        ));
    }
    let status = response.status().as_u16();
    let body = response
        .text()
        .map_err(|error| format!("Failed to read publish delete response: {error}"))?;
    if !(200..=299).contains(&status) {
        return Err(format!(
            "Publish API {status}: {}",
            parse_publish_error_message(&body).unwrap_or_else(|| body.trim().to_owned())
        ));
    }
    let decoded: DeletePublishResult = serde_json::from_str(&body)
        .map_err(|error| format!("Unexpected publish delete response: {error}"))?;
    if !decoded.success {
        return Err(format!(
            "Publish API {status}: {}",
            decoded
                .error
                .or(decoded.reason)
                .unwrap_or_else(|| String::from("Delete failed"))
        ));
    }
    Ok(decoded)
}

fn create_publish_zip(publish_dir: &Path, files: &[String]) -> Result<PathBuf, String> {
    let tmp_dir = std::env::temp_dir().join(format!(
        "deeptide-publish-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0)
    ));
    fs::create_dir_all(&tmp_dir)
        .map_err(|error| format!("Failed to create publish archive directory: {error}"))?;
    let zip_path = tmp_dir.join("site.zip");
    let file = fs::File::create(&zip_path)
        .map_err(|error| format!("Failed to create publish archive: {error}"))?;
    let mut writer = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o644);
    for relative in files {
        let path = publish_dir.join(relative);
        writer
            .start_file(relative.replace('\\', "/"), options)
            .map_err(|error| format!("Failed to add {relative} to publish archive: {error}"))?;
        let mut input = fs::File::open(&path)
            .map_err(|error| format!("Failed to read publish file {relative}: {error}"))?;
        io::copy(&mut input, &mut writer)
            .map_err(|error| format!("Failed to write {relative} to publish archive: {error}"))?;
    }
    writer
        .finish()
        .map_err(|error| format!("Failed to finalize publish archive: {error}"))?;
    Ok(zip_path)
}

fn save_publish_state(
    context: &ToolContext,
    result: &PublishResult,
    publish_dir: &Path,
) -> Result<(), String> {
    let handle = result
        .assigned_handle
        .clone()
        .unwrap_or_else(|| result.handle.clone());
    let state = PublishState {
        handle: Some(handle),
        url: Some(result.url.clone()),
        publish_dir: Some({
            let rel = relative_path_string(&context.cwd, publish_dir);
            if rel.is_empty() {
                String::from(".")
            } else {
                rel
            }
        }),
        file_count: Some(result.file_count),
        total_bytes: Some(result.total_bytes),
        published_at: Some(format_cron_datetime(std::time::SystemTime::now())),
        deleted_at: None,
        last_deleted_handle: None,
    };
    write_publish_state(context, &state)
}

fn mark_publish_deleted(context: &ToolContext, handle: &str) -> Result<(), String> {
    let Some(mut state) = load_publish_state(context) else {
        return Ok(());
    };
    if state.handle.as_deref() != Some(handle) {
        return Ok(());
    }
    state.last_deleted_handle = Some(handle.to_owned());
    state.deleted_at = Some(format_cron_datetime(std::time::SystemTime::now()));
    state.handle = None;
    write_publish_state(context, &state)
}

fn write_publish_state(context: &ToolContext, state: &PublishState) -> Result<(), String> {
    let dir = context.cwd.join(".clide");
    fs::create_dir_all(&dir).map_err(|error| format!("Failed to create .clide: {error}"))?;
    let data = serde_json::to_string_pretty(state)
        .map_err(|error| format!("Failed to encode publish state: {error}"))?;
    fs::write(dir.join("publish.json"), format!("{data}\n"))
        .map_err(|error| format!("Failed to write .clide/publish.json: {error}"))
}

fn render_publish_success(
    result: &PublishResult,
    context: &ToolContext,
    publish_dir: &Path,
) -> String {
    let handle = result
        .assigned_handle
        .as_deref()
        .unwrap_or(result.handle.as_str());
    let rel_dir = {
        let rel = relative_path_string(&context.cwd, publish_dir);
        if rel.is_empty() {
            String::from(".")
        } else {
            rel
        }
    };
    [
        format!("Published: {}", result.url),
        format!("  Handle:     {handle}"),
        format!("  Directory:  {rel_dir}"),
        format!("  Files:      {}", result.file_count),
        format!("  Bytes:      {}", result.total_bytes),
        format!(
            "  Overwrote:  {}",
            if result.overwritten { "yes" } else { "no" }
        ),
        String::from("  State:      .clide/publish.json"),
    ]
    .join("\n")
}

fn render_delete_success(result: &DeletePublishResult, fallback_handle: &str) -> String {
    let handle = result.handle.as_deref().unwrap_or(fallback_handle);
    let mut lines = vec![format!("Deleted remote publish: {handle}.clide.app")];
    if let Some(count) = result.deleted_objects {
        lines.push(format!("  Deleted objects: {count}"));
    }
    lines.push(String::from("  State: .clide/publish.json"));
    lines.join("\n")
}

fn publish_response_message(response: reqwest::blocking::Response) -> Option<String> {
    response
        .text()
        .ok()
        .and_then(|body| {
            parse_publish_error_message(&body).or_else(|| Some(body.trim().to_owned()))
        })
        .filter(|message| !message.is_empty())
}

fn parse_publish_error_message(body: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|value| {
            value
                .get("error")
                .or_else(|| value.get("reason"))
                .and_then(serde_json::Value::as_str)
                .map(ToOwned::to_owned)
        })
}
