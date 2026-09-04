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
        "Prepare, inspect, or publish a static frontend to Clide hosting, optionally with an Apps Square listing."
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
        let patterns = if options.dry_run {
            effective_clideignore_patterns(&context.cwd)
        } else {
            if let Err(message) = ensure_clideignore_safety_defaults(&context.cwd) {
                return ToolResult::error(message);
            }
            load_clideignore_patterns(&context.cwd)
        };
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

        let prior_state = load_publish_state(context);
        let prior_hosting_handle = prior_state
            .as_ref()
            .filter(|state| state.mode.as_deref() == Some("hosting-only"))
            .and_then(|state| state.handle.clone());
        if options.hosting_only
            && prior_hosting_handle.is_some()
            && options.handle.is_some()
            && prior_hosting_handle != options.handle
        {
            return ToolResult::error(
                "This project already has a saved hosting-only handle. Refusing to create a second site implicitly; rename or delete the existing deployment explicitly first.",
            );
        }
        let effective_handle = if options.hosting_only {
            options.handle.clone().or(prior_hosting_handle)
        } else {
            options.handle.clone()
        };
        let metadata = match build_publish_metadata(&context.cwd, &options) {
            Ok(metadata) => metadata,
            Err(message) => return ToolResult::error(message),
        };
        let plan = match build_publish_plan(
            &context.cwd,
            &publish_dir,
            &files,
            metadata,
            &options,
            effective_handle.clone(),
        ) {
            Ok(plan) => plan,
            Err(message) => return ToolResult::error(message),
        };
        if options.dry_run {
            return ToolResult::text(render_publish_dry_run(&plan));
        }
        if plan.runtime_compatibility.status == "blocked" {
            return ToolResult::error(format!(
                "Publish blocked: {} Use the project's Worker deployment workflow, or set allow_static_only only after the user accepts that backend/API features will not work.",
                plan.runtime_compatibility
                    .reason
                    .as_deref()
                    .unwrap_or("server runtime detected")
            ));
        }

        match upload_publish(context, &publish_dir, &files, &plan, &options) {
            Ok(result) => ToolResult::text(result),
            Err(message) => ToolResult::error(message),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PublishOptions {
    dir: Option<String>,
    handle: Option<String>,
    title: Option<String>,
    summary: Option<String>,
    category: Option<String>,
    tags: Vec<String>,
    allow_secrets: bool,
    allow_static_only: bool,
    hosting_only: bool,
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
            title: option("title"),
            summary: option("summary"),
            category: option("category"),
            tags: input
                .get("tags")
                .and_then(serde_json::Value::as_array)
                .map(|values| {
                    values
                        .iter()
                        .filter_map(serde_json::Value::as_str)
                        .flat_map(|value| value.split(','))
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .map(|value| value.to_ascii_lowercase())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default(),
            allow_secrets: input
                .get("allow_secrets")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false),
            allow_static_only: input
                .get("allow_static_only")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false),
            hosting_only: input
                .get("hosting_only")
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
        if input
            .get("random")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false)
        {
            return Err(String::from(
                "random is deprecated; omit handle to let the server assign a subdomain.",
            ));
        }
        if options.delete && options.dir.is_some() {
            return Err(String::from(
                "dir is only valid when publishing, not deleting.",
            ));
        }
        if options.delete
            && (options.title.is_some()
                || options.summary.is_some()
                || options.category.is_some()
                || !options.tags.is_empty()
                || options.allow_secrets
                || options.allow_static_only
                || options.hosting_only)
        {
            return Err(String::from(
                "publish-mode, metadata, and safety override options are only valid when publishing, not deleting.",
            ));
        }
        if options.status
            && (options.delete
                || options.dry_run
                || options.dir.is_some()
                || options.handle.is_some()
                || options.title.is_some()
                || options.summary.is_some()
                || options.category.is_some()
                || !options.tags.is_empty()
                || options.allow_secrets
                || options.allow_static_only
                || options.hosting_only)
        {
            return Err(String::from(
                "status cannot be combined with publish, delete, metadata, or safety override options.",
            ));
        }
        Ok(options)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct PublishState {
    #[serde(skip_serializing_if = "Option::is_none")]
    mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    handle: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<String>,
    #[serde(rename = "playUrl")]
    #[serde(skip_serializing_if = "Option::is_none")]
    play_url: Option<String>,
    #[serde(rename = "archiveUrl")]
    #[serde(skip_serializing_if = "Option::is_none")]
    archive_url: Option<String>,
    #[serde(rename = "workspaceHashKey")]
    #[serde(skip_serializing_if = "Option::is_none")]
    workspace_hash_key: Option<String>,
    #[serde(rename = "squareAppHashKey")]
    #[serde(skip_serializing_if = "Option::is_none")]
    square_app_hash_key: Option<String>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    category: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PublishPlan {
    publish_dir_relative: String,
    metadata: PublishMetadata,
    file_count: usize,
    total_bytes: u64,
    has_index: bool,
    sample_files: Vec<String>,
    secret_findings: Vec<PublishSecretFinding>,
    asset_warnings: Vec<String>,
    notes: Vec<String>,
    hosting_only: bool,
    effective_handle: Option<String>,
    runtime_compatibility: RuntimeCompatibility,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RuntimeCompatibility {
    status: String,
    config_file: Option<String>,
    features: Vec<String>,
    reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PublishMetadata {
    title: String,
    summary: String,
    category: String,
    tags: Vec<String>,
    remix_of_hash_key: Option<String>,
    remix_of_hash_keys: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PublishSecretFinding {
    file: String,
    kind: String,
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
    if let Some(mode) = state.mode {
        lines.push(format!("  Mode:        {mode}"));
    }
    if let Some(handle) = state.handle {
        lines.push(format!("  Handle:      {handle}"));
    }
    if let Some(url) = state.url {
        lines.push(format!("  URL:         {url}"));
    }
    if let Some(play_url) = state.play_url {
        lines.push(format!("  Play URL:    {play_url}"));
    }
    if let Some(archive_url) = state.archive_url {
        lines.push(format!("  Archive:     {archive_url}"));
    }
    if let Some(workspace) = state.workspace_hash_key {
        lines.push(format!("  Workspace:   {workspace}"));
    }
    if let Some(square_app) = state.square_app_hash_key {
        lines.push(format!("  Square app:  {square_app}"));
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
    if let Some(title) = state.title {
        lines.push(format!("  Title:       {title}"));
    }
    if let Some(category) = state.category {
        lines.push(format!("  Category:    {category}"));
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

fn detect_server_runtime(project_root: &Path, allow_static_only: bool) -> RuntimeCompatibility {
    for name in ["wrangler.jsonc", "wrangler.json", "wrangler.toml"] {
        let path = project_root.join(name);
        let Ok(source) = fs::read_to_string(&path) else {
            continue;
        };
        let checks = [
            ("Worker entrypoint", ["\"main\"", "main ="]),
            ("D1 binding", ["\"d1_databases\"", "[[d1_databases]]"]),
            ("R2 binding", ["\"r2_buckets\"", "[[r2_buckets]]"]),
            ("KV binding", ["\"kv_namespaces\"", "[[kv_namespaces]]"]),
            (
                "Durable Object binding",
                ["\"durable_objects\"", "[durable_objects]"],
            ),
            ("Service binding", ["\"services\"", "[[services]]"]),
        ];
        let features = checks
            .iter()
            .filter(|(_, needles)| needles.iter().any(|needle| source.contains(needle)))
            .map(|(label, _)| (*label).to_owned())
            .collect::<Vec<_>>();
        if !features.is_empty() {
            return RuntimeCompatibility {
                status: if allow_static_only {
                    String::from("static-only-acknowledged")
                } else {
                    String::from("blocked")
                },
                config_file: Some(name.to_owned()),
                features,
                reason: Some(String::from(
                    "Clide static hosting uploads browser assets only; it does not deploy Worker code or provision D1/R2/KV/Durable Object/service bindings.",
                )),
            };
        }
    }
    RuntimeCompatibility {
        status: String::from("static-compatible"),
        config_file: None,
        features: Vec::new(),
        reason: None,
    }
}

fn package_json_has_build_script(project_root: &Path) -> bool {
    read_package_json(project_root)
        .ok()
        .and_then(|value| value.get("scripts").cloned())
        .and_then(|scripts| scripts.get("build").cloned())
        .is_some()
}

fn read_package_json(project_root: &Path) -> Result<serde_json::Value, String> {
    let text = fs::read_to_string(project_root.join("package.json"))
        .map_err(|error| format!("Failed to read package.json: {error}"))?;
    serde_json::from_str::<serde_json::Value>(&text)
        .map_err(|error| format!("Failed to parse package.json: {error}"))
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

fn effective_clideignore_patterns(project_root: &Path) -> Vec<String> {
    let mut patterns = load_clideignore_patterns(project_root);
    for pattern in CLIDEIGNORE_SAFETY_PATTERNS {
        let normalized = normalize_publish_ignore_pattern(pattern);
        if !patterns.contains(&normalized) {
            patterns.push(normalized);
        }
    }
    patterns
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
    metadata: PublishMetadata,
    options: &PublishOptions,
    effective_handle: Option<String>,
) -> Result<PublishPlan, String> {
    let mut total_bytes = 0u64;
    for file in files {
        let metadata = fs::metadata(publish_dir.join(file))
            .map_err(|error| format!("Failed to inspect {file}: {error}"))?;
        total_bytes = total_bytes.saturating_add(metadata.len());
    }
    let secret_findings = scan_publish_secrets(publish_dir, files);
    let asset_warnings = publish_asset_warnings(publish_dir, files, !options.hosting_only);
    if !options.allow_secrets && !secret_findings.is_empty() {
        return Err(render_secret_block_message(&secret_findings));
    }
    let mut notes = Vec::new();
    notes.push(if options.hosting_only {
        String::from("Hosting is public at *.clide.app; no Paean workspace or Apps Square listing will be created.")
    } else {
        String::from("Publishing is public: the app is listed in Paean Apps Square and reachable at *.clide.app.")
    });
    if files.iter().any(|file| file.ends_with(".map")) {
        notes.push(String::from(
            "Source maps are included; add a .clideignore rule if they should stay private.",
        ));
    }
    Ok(PublishPlan {
        publish_dir_relative: relative_path_string(project_root, publish_dir),
        metadata,
        file_count: files.len(),
        total_bytes,
        has_index: files.iter().any(|file| file == "index.html"),
        sample_files: files.iter().take(6).cloned().collect(),
        secret_findings,
        asset_warnings,
        notes,
        hosting_only: options.hosting_only,
        effective_handle,
        runtime_compatibility: detect_server_runtime(project_root, options.allow_static_only),
    })
}

fn render_publish_dry_run(plan: &PublishPlan) -> String {
    let rel_dir = if plan.publish_dir_relative.is_empty() {
        "."
    } else {
        &plan.publish_dir_relative
    };
    let mut lines = vec![
        format!(
            "Publish dry run: {}",
            if plan.runtime_compatibility.status == "blocked" {
                "blocked"
            } else {
                "ready"
            }
        ),
        format!("  Directory:  {rel_dir}"),
        format!(
            "  Mode:       {}",
            if plan.hosting_only {
                "hosting-only"
            } else {
                "square"
            }
        ),
        format!(
            "  Target:     {}",
            if plan.hosting_only {
                "Clide hosting only (*.clide.app; no Apps Square listing)"
            } else {
                "Paean Apps Square public listing + *.clide.app"
            }
        ),
        format!("  Title:      {}", plan.metadata.title),
        format!("  Category:   {}", plan.metadata.category),
        format!("  Files:      {}", plan.file_count),
        format!("  Bytes:      {}", plan.total_bytes),
        format!(
            "  Index:      {}",
            if plan.has_index { "yes" } else { "no" }
        ),
        String::from("  Ignore:     .clideignore safety defaults present"),
        format!(
            "  Secrets:    {}",
            if plan.secret_findings.is_empty() {
                "passed"
            } else {
                "allowed by flag"
            }
        ),
    ];
    if !plan.metadata.summary.is_empty() {
        lines.push(format!("  Summary:    {}", plan.metadata.summary));
    }
    if let Some(handle) = &plan.effective_handle {
        lines.push(format!("  Handle:     {handle}"));
        lines.push(format!("  URL:        https://{handle}.clide.app/"));
    }
    lines.push(format!(
        "  Runtime:    {}",
        plan.runtime_compatibility.status
    ));
    if let Some(config_file) = &plan.runtime_compatibility.config_file {
        lines.push(format!(
            "  Detected:   {} ({})",
            config_file,
            plan.runtime_compatibility.features.join(", ")
        ));
    }
    if let Some(reason) = &plan.runtime_compatibility.reason {
        lines.push(format!("  Runtime note: {reason}"));
    }
    if !plan.metadata.tags.is_empty() {
        lines.push(format!("  Tags:       {}", plan.metadata.tags.join(", ")));
    }
    if !plan.sample_files.is_empty() {
        lines.push(format!("  Sample:     {}", plan.sample_files.join(", ")));
    }
    if !plan.metadata.remix_of_hash_keys.is_empty() {
        lines.push(format!(
            "  Remix API:  remixOfHashKey={}, remixOfHashKeys={}",
            plan.metadata
                .remix_of_hash_key
                .as_deref()
                .unwrap_or_default(),
            plan.metadata.remix_of_hash_keys.join(", ")
        ));
    }
    if !plan.asset_warnings.is_empty() {
        lines.push(String::from("  Asset warnings:"));
        lines.extend(
            plan.asset_warnings
                .iter()
                .map(|warning| format!("    - {warning}")),
        );
    }
    if !plan.notes.is_empty() {
        lines.push(String::from("  Notes:"));
        lines.extend(plan.notes.iter().map(|note| format!("    - {note}")));
    }
    lines.join("\n")
}

fn build_publish_metadata(
    project_root: &Path,
    options: &PublishOptions,
) -> Result<PublishMetadata, String> {
    let package = read_package_json(project_root).ok();
    let manifest = read_clide_json(project_root).ok();
    let title = options
        .title
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .or_else(|| manifest_string(&manifest, "title"))
        .or_else(|| {
            package
                .as_ref()
                .and_then(|value| value.get("name"))
                .and_then(serde_json::Value::as_str)
                .map(|name| {
                    name.trim()
                        .rsplit_once('/')
                        .map(|(_, local)| local)
                        .unwrap_or(name.trim())
                        .to_owned()
                })
        })
        .or_else(|| {
            project_root
                .file_name()
                .and_then(|name| name.to_str())
                .map(ToOwned::to_owned)
        })
        .filter(|value| !value.is_empty())
        .ok_or_else(|| String::from("Could not determine publish title."))?;
    let summary = options
        .summary
        .clone()
        .or_else(|| manifest_string(&manifest, "summary"))
        .or_else(|| {
            package
                .as_ref()
                .and_then(|value| value.get("description"))
                .and_then(serde_json::Value::as_str)
                .map(ToOwned::to_owned)
        })
        .unwrap_or_default();
    let manifest_tags = manifest_tags(&manifest);
    let remix_hashes = manifest_remix_hashes(&manifest);
    Ok(PublishMetadata {
        title,
        summary,
        category: options
            .category
            .clone()
            .or_else(|| manifest_string(&manifest, "category"))
            .unwrap_or_else(|| String::from("custom")),
        tags: if options.tags.is_empty() {
            dedupe_tags(&manifest_tags)
        } else {
            dedupe_tags(&options.tags)
        },
        remix_of_hash_key: remix_hashes.first().cloned(),
        remix_of_hash_keys: remix_hashes,
    })
}

fn read_clide_json(project_root: &Path) -> Result<serde_json::Value, String> {
    let text = fs::read_to_string(project_root.join("clide.json"))
        .map_err(|error| format!("Failed to read clide.json: {error}"))?;
    serde_json::from_str::<serde_json::Value>(&text)
        .map_err(|error| format!("Failed to parse clide.json: {error}"))
}

fn manifest_string(manifest: &Option<serde_json::Value>, key: &str) -> Option<String> {
    manifest
        .as_ref()
        .and_then(|value| value.get(key))
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn manifest_tags(manifest: &Option<serde_json::Value>) -> Vec<String> {
    manifest
        .as_ref()
        .and_then(|value| value.get("tags"))
        .and_then(serde_json::Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(|value| value.to_ascii_lowercase())
                .collect()
        })
        .unwrap_or_default()
}

fn manifest_remix_hashes(manifest: &Option<serde_json::Value>) -> Vec<String> {
    let mut hashes = Vec::new();
    let Some(remix) = manifest.as_ref().and_then(|value| value.get("remix")) else {
        return hashes;
    };
    if let Some(parent) = remix
        .get("parent")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        hashes.push(parent.to_owned());
    }
    if let Some(parents) = remix.get("parents").and_then(serde_json::Value::as_array) {
        for parent in parents {
            let hash = parent
                .get("hashKey")
                .or_else(|| parent.get("hash"))
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty());
            if let Some(hash) = hash {
                hashes.push(hash.to_owned());
            }
        }
    }
    dedupe_tags(&hashes)
}

fn dedupe_tags(tags: &[String]) -> Vec<String> {
    let mut seen = std::collections::BTreeSet::new();
    let mut out = Vec::new();
    for tag in tags {
        if seen.insert(tag.clone()) {
            out.push(tag.clone());
        }
    }
    out
}

fn scan_publish_secrets(publish_dir: &Path, files: &[String]) -> Vec<PublishSecretFinding> {
    files
        .iter()
        .filter(|file| publish_file_is_secret_scannable(file, &publish_dir.join(file)))
        .filter_map(|file| {
            let text = fs::read_to_string(publish_dir.join(file)).ok()?;
            publish_secret_kind(&text).map(|kind| PublishSecretFinding {
                file: file.clone(),
                kind,
            })
        })
        .collect()
}

fn publish_file_is_secret_scannable(relative: &str, path: &Path) -> bool {
    const MAX_SECRET_SCAN_BYTES: u64 = 1024 * 1024;
    let Ok(metadata) = fs::metadata(path) else {
        return false;
    };
    if metadata.len() > MAX_SECRET_SCAN_BYTES {
        return false;
    }
    let lower = relative.to_ascii_lowercase();
    lower == "index.html"
        || lower == "robots.txt"
        || [
            ".html",
            ".htm",
            ".css",
            ".js",
            ".mjs",
            ".cjs",
            ".json",
            ".xml",
            ".txt",
            ".md",
            ".svg",
            ".map",
            ".webmanifest",
            ".wasm.map",
        ]
        .iter()
        .any(|suffix| lower.ends_with(suffix))
}

fn publish_secret_kind(text: &str) -> Option<String> {
    let checks = [
        (
            "private key",
            r"-----BEGIN (?:RSA |DSA |EC |OPENSSH |PGP )?PRIVATE KEY-----",
        ),
        ("AWS access key", r"\b(?:AKIA|ASIA)[A-Z0-9]{16}\b"),
        (
            "AWS secret access key assignment",
            r#"\bAWS_SECRET_ACCESS_KEY\s*=\s*['"]?[A-Za-z0-9/+=]{32,}"#,
        ),
        (
            "GitHub token",
            r"\b(?:ghp|gho|ghu|ghs|ghr)_[A-Za-z0-9_]{36,}\b",
        ),
        ("npm token", r"\bnpm_[A-Za-z0-9]{30,}\b"),
        ("Slack token", r"\bxox[baprs]-[A-Za-z0-9-]{20,}\b"),
        (
            "OpenAI-style API key",
            r"\bsk-(?:proj-|svcacct-|admin-)?[A-Za-z0-9_-]{32,}\b",
        ),
    ];
    checks.iter().find_map(|(name, pattern)| {
        regex::Regex::new(pattern)
            .ok()
            .filter(|regex| regex.is_match(text))
            .map(|_| (*name).to_owned())
    })
}

fn publish_asset_warnings(
    publish_dir: &Path,
    files: &[String],
    square_listing: bool,
) -> Vec<String> {
    let mut warnings = Vec::new();
    if !files.iter().any(|file| file == "favicon.svg") {
        warnings.push(String::from(
            "Recommended top-level `favicon.svg` is missing.",
        ));
    }
    if !square_listing {
        return warnings;
    }
    if !files.iter().any(|file| file == "banner.jpg") {
        warnings.push(String::from(
            "Recommended top-level `banner.jpg` is missing; Square prefers 800x400.",
        ));
        return warnings;
    }
    match read_jpeg_dimensions(&publish_dir.join("banner.jpg")) {
        Some((800, 400)) => {}
        Some((width, height)) => warnings.push(format!(
            "`banner.jpg` is {width}x{height}; recommended size is exactly 800x400."
        )),
        None => warnings.push(String::from(
            "Could not verify `banner.jpg` dimensions; recommended size is exactly 800x400.",
        )),
    }
    warnings
}

fn read_jpeg_dimensions(path: &Path) -> Option<(u16, u16)> {
    let bytes = fs::read(path).ok()?;
    if bytes.len() < 4 || bytes[0] != 0xff || bytes[1] != 0xd8 {
        return None;
    }
    let mut i = 2usize;
    while i + 9 < bytes.len() {
        while i < bytes.len() && bytes[i] == 0xff {
            i += 1;
        }
        if i >= bytes.len() {
            return None;
        }
        let marker = bytes[i];
        i += 1;
        if marker == 0xd9 || marker == 0xda {
            return None;
        }
        if i + 2 > bytes.len() {
            return None;
        }
        let len = u16::from_be_bytes([bytes[i], bytes[i + 1]]) as usize;
        if len < 2 || i + len > bytes.len() {
            return None;
        }
        if matches!(
            marker,
            0xc0 | 0xc1
                | 0xc2
                | 0xc3
                | 0xc5
                | 0xc6
                | 0xc7
                | 0xc9
                | 0xca
                | 0xcb
                | 0xcd
                | 0xce
                | 0xcf
        ) {
            if i + 7 >= bytes.len() {
                return None;
            }
            let height = u16::from_be_bytes([bytes[i + 3], bytes[i + 4]]);
            let width = u16::from_be_bytes([bytes[i + 5], bytes[i + 6]]);
            return Some((width, height));
        }
        i += len;
    }
    None
}

fn render_secret_block_message(findings: &[PublishSecretFinding]) -> String {
    let mut lines = vec![String::from(
        "Publish blocked: possible secrets found in included files.",
    )];
    for finding in findings.iter().take(10) {
        lines.push(format!("  - {} ({})", finding.file, finding.kind));
    }
    if findings.len() > 10 {
        lines.push(format!("  ...and {} more", findings.len() - 10));
    }
    lines.push(String::from(
        "Add patterns to .clideignore or rerun with allow_secrets only when intentional.",
    ));
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

#[derive(Debug, Deserialize, Serialize)]
struct WorkspaceCreateResponse {
    workspace: WorkspaceInfo,
}

#[derive(Debug, Deserialize, Serialize)]
struct WorkspaceInfo {
    #[serde(rename = "hashKey")]
    hash_key: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct WorkspaceImportResult {
    #[serde(rename = "fileCount")]
    file_count: Option<u64>,
    #[serde(rename = "totalBytes")]
    total_bytes: Option<u64>,
}

#[derive(Debug, Deserialize, Serialize)]
struct ZipPresignResponse {
    #[serde(rename = "uploadUrl")]
    upload_url: String,
    #[serde(rename = "stagingKey")]
    staging_key: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct SquarePublishResult {
    #[serde(rename = "hashKey")]
    hash_key: String,
    #[serde(rename = "playUrl")]
    play_url: String,
    #[serde(rename = "publishedSiteHandle")]
    published_site_handle: Option<String>,
    status: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct PublishResult {
    handle: Option<String>,
    #[serde(rename = "workspaceHashKey")]
    workspace_hash_key: String,
    #[serde(rename = "squareAppHashKey")]
    square_app_hash_key: String,
    url: String,
    status: String,
    #[serde(rename = "fileCount")]
    file_count: u64,
    #[serde(rename = "totalBytes")]
    total_bytes: u64,
    title: String,
    category: String,
}

#[derive(Debug, Deserialize)]
struct HostingPublishResult {
    handle: String,
    url: String,
    #[serde(rename = "archiveUrl")]
    archive_url: Option<String>,
    #[serde(rename = "fileCount")]
    file_count: Option<u64>,
    #[serde(rename = "totalBytes")]
    total_bytes: Option<u64>,
    overwritten: Option<bool>,
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
    options: &PublishOptions,
) -> Result<String, String> {
    let auth = resolve_publish_auth()?;
    let zip_path = create_publish_zip(publish_dir, files)?;
    let result = (|| {
        if options.hosting_only {
            let hosted = publish_hosting_only(&auth, &zip_path, plan.effective_handle.as_deref())?;
            save_hosting_publish_state(context, &hosted, publish_dir, plan)?;
            return Ok(render_hosting_publish_success(
                &hosted,
                context,
                publish_dir,
                plan,
            ));
        }
        let state = load_publish_state(context);
        let workspace_hash_key = match state.and_then(|state| state.workspace_hash_key) {
            Some(hash_key) if !hash_key.trim().is_empty() => hash_key,
            _ => create_workspace(&auth, &plan.metadata)?,
        };
        let imported = import_zip(&auth, &workspace_hash_key, &zip_path)?;
        let square = publish_square(
            &auth,
            &workspace_hash_key,
            &plan.metadata,
            plan.effective_handle.as_deref(),
        )?;
        let response = PublishResult {
            handle: square.published_site_handle,
            workspace_hash_key,
            square_app_hash_key: square.hash_key,
            url: square.play_url,
            status: square.status.unwrap_or_else(|| String::from("listed")),
            file_count: imported.file_count.unwrap_or(plan.file_count as u64),
            total_bytes: imported.total_bytes.unwrap_or(plan.total_bytes),
            title: plan.metadata.title.clone(),
            category: plan.metadata.category.clone(),
        };
        save_publish_state(context, &response, publish_dir)?;
        Ok(render_publish_success(
            &response,
            context,
            publish_dir,
            plan,
        ))
    })();
    let _ = fs::remove_file(&zip_path);
    result
}

fn delete_publish(context: &ToolContext, handle: &str) -> Result<String, String> {
    let auth = resolve_publish_auth()?;
    let result = unpublish_handle(&auth, handle)?;
    mark_publish_deleted(context, handle)?;
    Ok(render_delete_success(&result, handle))
}

fn resolve_publish_auth() -> Result<PublishAuth, String> {
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
                "Paean login is missing or expired. Run `tide auth login` or set PAEAN_AUTH_TOKEN, then retry Publish.",
            )
        })?;
    let base_url = resolve_publish_api_base();
    Ok(PublishAuth {
        base_url: normalize_publish_base_url(&base_url),
        token,
    })
}

fn resolve_publish_api_base() -> String {
    for key in ["PAEAN_API_BASE", "PAEAN_API_BASE_URL", "CLIDE_API_BASE_URL"] {
        if let Ok(value) = std::env::var(key)
            && !value.trim().is_empty()
        {
            return value;
        }
    }
    for key in ["ZERO_API_BASE", "ZERO_CLI_BASE_URL"] {
        if let Ok(value) = std::env::var(key)
            && publish_base_is_paean(&value)
        {
            return value;
        }
    }
    String::from("https://api.paean.ai")
}

fn publish_base_is_paean(raw: &str) -> bool {
    reqwest::Url::parse(raw)
        .ok()
        .and_then(|url| url.host_str().map(ToOwned::to_owned))
        .is_some_and(|host| host == "paean.ai" || host.ends_with(".paean.ai"))
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

fn create_workspace(auth: &PublishAuth, metadata: &PublishMetadata) -> Result<String, String> {
    #[derive(Serialize)]
    struct WorkspaceRequest<'a> {
        title: &'a str,
        description: &'a str,
        goal: &'a str,
        tags: &'a [String],
    }
    let body = WorkspaceRequest {
        title: &metadata.title,
        description: &metadata.summary,
        goal: "Published from Deeptide.",
        tags: &metadata.tags,
    };
    let decoded: WorkspaceCreateResponse =
        post_json(auth, "/v2/workspace", &body, "Workspace API")?;
    if decoded.workspace.hash_key.trim().is_empty() {
        return Err(String::from(
            "Workspace API response missing workspace.hashKey",
        ));
    }
    Ok(decoded.workspace.hash_key)
}

fn import_zip(
    auth: &PublishAuth,
    workspace_hash_key: &str,
    zip_path: &Path,
) -> Result<WorkspaceImportResult, String> {
    const DEFAULT_DIRECT_UPLOAD_MAX_BYTES: u64 = 25 * 1024 * 1024;
    let max_direct = std::env::var("PAEAN_WORKSPACE_DIRECT_UPLOAD_MAX_BYTES")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(DEFAULT_DIRECT_UPLOAD_MAX_BYTES);
    let zip_bytes = fs::metadata(zip_path)
        .map_err(|error| format!("Failed to inspect publish archive: {error}"))?
        .len();
    if zip_bytes > max_direct {
        import_zip_via_presign(auth, workspace_hash_key, zip_path)
    } else {
        import_zip_direct(auth, workspace_hash_key, zip_path)
    }
}

fn import_zip_direct(
    auth: &PublishAuth,
    workspace_hash_key: &str,
    zip_path: &Path,
) -> Result<WorkspaceImportResult, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|error| format!("Failed to build publish HTTP client: {error}"))?;
    let file = fs::File::open(zip_path)
        .map_err(|error| format!("Failed to open publish archive: {error}"))?;
    let part = reqwest::blocking::multipart::Part::reader(file)
        .file_name("project.zip")
        .mime_str("application/zip")
        .map_err(|error| format!("Failed to prepare publish archive part: {error}"))?;
    let form = reqwest::blocking::multipart::Form::new().part("archive", part);
    let encoded = percent_encode_path_segment(workspace_hash_key);
    let response = client
        .post(format!(
            "{}/v2/workspace/{encoded}/files/zip",
            auth.base_url
        ))
        .bearer_auth(&auth.token)
        .multipart(form)
        .send()
        .map_err(|error| format!("Workspace zip import failed: {error}"))?;
    read_json_response(response, "Workspace zip import")
}

fn import_zip_via_presign(
    auth: &PublishAuth,
    workspace_hash_key: &str,
    zip_path: &Path,
) -> Result<WorkspaceImportResult, String> {
    #[derive(Serialize)]
    struct EmptyRequest {}
    #[derive(Serialize)]
    struct CommitRequest<'a> {
        #[serde(rename = "stagingKey")]
        staging_key: &'a str,
    }
    let encoded = percent_encode_path_segment(workspace_hash_key);
    let presign: ZipPresignResponse = post_json(
        auth,
        &format!("/v2/workspace/{encoded}/files/zip/presign"),
        &EmptyRequest {},
        "Workspace zip presign",
    )?;
    let bytes =
        fs::read(zip_path).map_err(|error| format!("Failed to read publish archive: {error}"))?;
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|error| format!("Failed to build publish HTTP client: {error}"))?;
    let response = client
        .put(&presign.upload_url)
        .header(reqwest::header::CONTENT_TYPE, "application/zip")
        .body(bytes)
        .send()
        .map_err(|error| format!("Signed zip upload failed: {error}"))?;
    if !response.status().is_success() {
        let status = response.status().as_u16();
        let body = response.text().unwrap_or_default();
        return Err(format!(
            "Signed zip upload {status}: {}",
            body.trim().chars().take(300).collect::<String>()
        ));
    }
    post_json(
        auth,
        &format!("/v2/workspace/{encoded}/files/zip/commit"),
        &CommitRequest {
            staging_key: &presign.staging_key,
        },
        "Workspace zip commit",
    )
}

fn publish_hosting_only(
    auth: &PublishAuth,
    zip_path: &Path,
    handle: Option<&str>,
) -> Result<HostingPublishResult, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|error| format!("Failed to build Clide hosting HTTP client: {error}"))?;
    let file = fs::File::open(zip_path)
        .map_err(|error| format!("Failed to open publish archive: {error}"))?;
    let part = reqwest::blocking::multipart::Part::reader(file)
        .file_name("site.zip")
        .mime_str("application/zip")
        .map_err(|error| format!("Failed to prepare publish archive part: {error}"))?;
    let mut form = reqwest::blocking::multipart::Form::new().part("archive", part);
    if let Some(handle) = handle {
        form = form.text("handle", handle.to_owned());
    }
    let response = client
        .post(format!("{}/publish/clide", auth.base_url))
        .bearer_auth(&auth.token)
        .multipart(form)
        .send()
        .map_err(|error| format!("Clide hosting upload failed: {error}"))?;
    read_json_response(response, "Clide hosting")
        .map_err(|error| explain_handle_failure(error, handle))
}

fn publish_square(
    auth: &PublishAuth,
    workspace_hash_key: &str,
    metadata: &PublishMetadata,
    handle: Option<&str>,
) -> Result<SquarePublishResult, String> {
    #[derive(Serialize)]
    struct SquareRequest<'a> {
        #[serde(rename = "workspaceHashKey")]
        workspace_hash_key: &'a str,
        title: &'a str,
        summary: &'a str,
        category: &'a str,
        tags: &'a [String],
        #[serde(rename = "pathPrefix")]
        path_prefix: &'a str,
        #[serde(rename = "indexFile")]
        index_file: &'a str,
        visibility: &'a str,
        #[serde(skip_serializing_if = "Option::is_none")]
        handle: Option<&'a str>,
        #[serde(rename = "remixOfHashKey")]
        #[serde(skip_serializing_if = "Option::is_none")]
        remix_of_hash_key: Option<&'a str>,
        #[serde(rename = "remixOfHashKeys")]
        #[serde(skip_serializing_if = "Vec::is_empty")]
        remix_of_hash_keys: Vec<&'a str>,
    }
    let remix_of_hash_keys = metadata
        .remix_of_hash_keys
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    post_json(
        auth,
        "/square/publish",
        &SquareRequest {
            workspace_hash_key,
            title: &metadata.title,
            summary: &metadata.summary,
            category: &metadata.category,
            tags: &metadata.tags,
            path_prefix: "",
            index_file: "index.html",
            visibility: "public",
            handle,
            remix_of_hash_key: metadata.remix_of_hash_key.as_deref(),
            remix_of_hash_keys,
        },
        "Square publish",
    )
    .map_err(|error| explain_handle_failure(error, handle))
}

fn explain_handle_failure(error: String, handle: Option<&str>) -> String {
    let Some(handle) = handle else {
        return error;
    };
    if error.contains(" 402:") {
        return format!(
            "Choosing the subdomain \"{handle}\" requires an active Paean subscription. Publish again without a handle for an assigned URL. Server said: {error}"
        );
    }
    if error.contains(" 409:") {
        return format!(
            "The subdomain \"{handle}\" is already taken. Choose another handle. Server said: {error}"
        );
    }
    if error.contains(" 400:") {
        return format!(
            "The subdomain \"{handle}\" was rejected. Choose another handle. Server said: {error}"
        );
    }
    error
}

fn post_json<T: for<'de> Deserialize<'de>, B: Serialize>(
    auth: &PublishAuth,
    path: &str,
    body: &B,
    label: &str,
) -> Result<T, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .map_err(|error| format!("Failed to build publish HTTP client: {error}"))?;
    let response = client
        .post(format!("{}{}", auth.base_url, path))
        .bearer_auth(&auth.token)
        .json(body)
        .send()
        .map_err(|error| format!("{label} failed: {error}"))?;
    read_json_response(response, label)
}

fn read_json_response<T: for<'de> Deserialize<'de>>(
    response: reqwest::blocking::Response,
    label: &str,
) -> Result<T, String> {
    if matches!(response.status().as_u16(), 401 | 403) {
        return Err(String::from(
            "Paean login is missing or expired. Refresh the publish token, then retry Publish.",
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
        return Err(format!(
            "{label} {status}: {}",
            parse_publish_error_message_from_value(&value).unwrap_or_else(|| body
                .trim()
                .chars()
                .take(300)
                .collect())
        ));
    }
    let payload = value
        .get("data")
        .filter(|data| !data.is_null())
        .cloned()
        .unwrap_or(value);
    serde_json::from_value(payload).map_err(|error| format!("Unexpected {label} response: {error}"))
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
    let state = PublishState {
        mode: Some(String::from("square")),
        handle: result.handle.clone(),
        url: Some(result.url.clone()),
        play_url: Some(result.url.clone()),
        archive_url: None,
        workspace_hash_key: Some(result.workspace_hash_key.clone()),
        square_app_hash_key: Some(result.square_app_hash_key.clone()),
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
        title: Some(result.title.clone()),
        category: Some(result.category.clone()),
    };
    write_publish_state(context, &state)
}

fn save_hosting_publish_state(
    context: &ToolContext,
    result: &HostingPublishResult,
    publish_dir: &Path,
    plan: &PublishPlan,
) -> Result<(), String> {
    let state = PublishState {
        mode: Some(String::from("hosting-only")),
        handle: Some(result.handle.clone()),
        url: Some(result.url.clone()),
        play_url: Some(result.url.clone()),
        archive_url: result.archive_url.clone(),
        workspace_hash_key: None,
        square_app_hash_key: None,
        publish_dir: Some({
            let rel = relative_path_string(&context.cwd, publish_dir);
            if rel.is_empty() {
                String::from(".")
            } else {
                rel
            }
        }),
        file_count: Some(result.file_count.unwrap_or(plan.file_count as u64)),
        total_bytes: Some(result.total_bytes.unwrap_or(plan.total_bytes)),
        published_at: Some(format_cron_datetime(std::time::SystemTime::now())),
        deleted_at: None,
        last_deleted_handle: None,
        title: Some(plan.metadata.title.clone()),
        category: Some(plan.metadata.category.clone()),
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
    plan: &PublishPlan,
) -> String {
    let rel_dir = {
        let rel = relative_path_string(&context.cwd, publish_dir);
        if rel.is_empty() {
            String::from(".")
        } else {
            rel
        }
    };
    let mut lines = vec![
        format!("Published: {}", result.url),
        format!("  Workspace:  {}", result.workspace_hash_key),
        format!("  Square app: {}", result.square_app_hash_key),
        format!("  Status:     {}", result.status),
        format!("  Title:      {}", result.title),
        format!("  Directory:  {rel_dir}"),
        format!("  Files:      {}", result.file_count),
        format!("  Bytes:      {}", result.total_bytes),
        String::from("  State:      .clide/publish.json"),
    ];
    if !plan.metadata.remix_of_hash_keys.is_empty() {
        lines.push(format!(
            "  Remix API:  remixOfHashKey={}, remixOfHashKeys={}",
            plan.metadata
                .remix_of_hash_key
                .as_deref()
                .unwrap_or_default(),
            plan.metadata.remix_of_hash_keys.join(", ")
        ));
    }
    if !plan.asset_warnings.is_empty() {
        lines.push(String::from("  Asset warnings:"));
        lines.extend(
            plan.asset_warnings
                .iter()
                .map(|warning| format!("    - {warning}")),
        );
    }
    lines.join("\n")
}

fn render_hosting_publish_success(
    result: &HostingPublishResult,
    context: &ToolContext,
    publish_dir: &Path,
    plan: &PublishPlan,
) -> String {
    let rel = relative_path_string(&context.cwd, publish_dir);
    let rel_dir = if rel.is_empty() { "." } else { rel.as_str() };
    let mut lines = vec![
        format!("Hosted: {}", result.url),
        String::from("  Mode:        hosting-only"),
        String::from("  Apps Square: not listed"),
        format!("  Handle:      {}", result.handle),
        format!(
            "  Status:      {}",
            if result.overwritten.unwrap_or(false) {
                "updated"
            } else {
                "hosted"
            }
        ),
        format!("  Title:       {}", plan.metadata.title),
        format!("  Directory:   {rel_dir}"),
        format!(
            "  Files:       {}",
            result.file_count.unwrap_or(plan.file_count as u64)
        ),
        format!(
            "  Bytes:       {}",
            result.total_bytes.unwrap_or(plan.total_bytes)
        ),
        String::from("  State:       .clide/publish.json"),
    ];
    if let Some(archive_url) = &result.archive_url {
        lines.push(format!("  Archive:     {archive_url}"));
    }
    if !plan.asset_warnings.is_empty() {
        lines.push(String::from("  Asset warnings:"));
        lines.extend(
            plan.asset_warnings
                .iter()
                .map(|warning| format!("    - {warning}")),
        );
    }
    lines.join("\n")
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

fn parse_publish_error_message(body: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|value| parse_publish_error_message_from_value(&value))
}

fn parse_publish_error_message_from_value(value: &serde_json::Value) -> Option<String> {
    value
        .get("error")
        .or_else(|| value.get("message"))
        .or_else(|| value.get("reason"))
        .and_then(serde_json::Value::as_str)
        .map(ToOwned::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_jpeg(width: u16, height: u16) -> Vec<u8> {
        vec![
            0xff,
            0xd8,
            0xff,
            0xc0,
            0x00,
            0x11,
            0x08,
            (height >> 8) as u8,
            height as u8,
            (width >> 8) as u8,
            width as u8,
            0x03,
            0x01,
            0x11,
            0x00,
            0x02,
            0x11,
            0x00,
            0x03,
            0x11,
            0x00,
            0xff,
            0xd9,
        ]
    }

    #[test]
    fn clide_json_metadata_and_remix_lineage_are_used() {
        let dir = tempfile::tempdir().expect("publish test");
        fs::write(
            dir.path().join("clide.json"),
            r#"{
              "title": "Remix Title",
              "summary": "From manifest",
              "category": "arcade",
              "tags": ["Fast", "fast", "neon"],
              "remix": {
                "parent": "parent1",
                "parents": [
                  {"hashKey": "parent1", "role": "gameplay"},
                  {"hashKey": "parent2", "role": "art"}
                ]
              }
            }"#,
        )
        .expect("publish test");

        let options = PublishOptions {
            dir: None,
            handle: None,
            title: None,
            summary: None,
            category: None,
            tags: vec![],
            allow_secrets: false,
            allow_static_only: false,
            hosting_only: false,
            delete: false,
            dry_run: true,
            status: false,
        };
        let metadata = build_publish_metadata(dir.path(), &options).expect("publish test");

        assert_eq!(metadata.title, "Remix Title");
        assert_eq!(metadata.summary, "From manifest");
        assert_eq!(metadata.category, "arcade");
        assert_eq!(metadata.tags, vec!["fast", "neon"]);
        assert_eq!(metadata.remix_of_hash_key.as_deref(), Some("parent1"));
        assert_eq!(metadata.remix_of_hash_keys, vec!["parent1", "parent2"]);
    }

    #[test]
    fn asset_warnings_are_non_blocking_and_check_banner_dimensions() {
        let dir = tempfile::tempdir().expect("publish test");
        let files = vec![String::from("index.html")];
        let warnings = publish_asset_warnings(dir.path(), &files, true);
        assert!(
            warnings
                .iter()
                .any(|warning| warning.contains("favicon.svg"))
        );
        assert!(
            warnings
                .iter()
                .any(|warning| warning.contains("banner.jpg"))
        );

        fs::write(dir.path().join("favicon.svg"), "<svg/>").expect("publish test");
        fs::write(dir.path().join("banner.jpg"), minimal_jpeg(1024, 512)).expect("publish test");
        let files = vec![
            String::from("index.html"),
            String::from("favicon.svg"),
            String::from("banner.jpg"),
        ];
        let warnings = publish_asset_warnings(dir.path(), &files, true);
        assert_eq!(
            read_jpeg_dimensions(&dir.path().join("banner.jpg")),
            Some((1024, 512))
        );
        assert!(warnings.iter().any(|warning| warning.contains("1024x512")));

        fs::write(dir.path().join("banner.jpg"), minimal_jpeg(800, 400)).expect("publish test");
        assert!(publish_asset_warnings(dir.path(), &files, true).is_empty());
    }

    #[test]
    fn hosting_only_accepts_custom_handle_and_runtime_acknowledgement() {
        let options = PublishOptions::from_input(&serde_json::json!({
            "hosting_only": true,
            "handle": "paeaninsight",
            "allow_static_only": true
        }))
        .expect("publish options");
        assert!(options.hosting_only);
        assert!(options.allow_static_only);
        assert_eq!(options.handle.as_deref(), Some("paeaninsight"));
    }

    #[test]
    fn wrangler_worker_bindings_block_static_publish_until_acknowledged() {
        let dir = tempfile::tempdir().expect("publish test");
        fs::write(
            dir.path().join("wrangler.jsonc"),
            r#"{"main":"worker/index.ts","d1_databases":[{"binding":"DB"}]}"#,
        )
        .expect("publish test");
        let blocked = detect_server_runtime(dir.path(), false);
        assert_eq!(blocked.status, "blocked");
        assert_eq!(blocked.config_file.as_deref(), Some("wrangler.jsonc"));
        assert_eq!(blocked.features, vec!["Worker entrypoint", "D1 binding"]);
        let acknowledged = detect_server_runtime(dir.path(), true);
        assert_eq!(acknowledged.status, "static-only-acknowledged");
    }

    #[test]
    fn hosting_only_does_not_require_square_banner() {
        let dir = tempfile::tempdir().expect("publish test");
        let files = vec![String::from("index.html")];
        let warnings = publish_asset_warnings(dir.path(), &files, false);
        assert!(
            warnings
                .iter()
                .any(|warning| warning.contains("favicon.svg"))
        );
        assert!(
            !warnings
                .iter()
                .any(|warning| warning.contains("banner.jpg"))
        );
    }

    #[test]
    fn generic_model_gateway_is_not_treated_as_paean_api() {
        assert!(publish_base_is_paean("https://api.paean.ai"));
        assert!(publish_base_is_paean("https://staging.api.paean.ai/zero"));
        assert!(!publish_base_is_paean("https://api.anthropic.com"));
        assert!(!publish_base_is_paean("not-a-url"));
    }
}
