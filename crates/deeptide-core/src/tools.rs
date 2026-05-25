use std::collections::BTreeMap;
use std::fs;
use std::io::{self, BufRead};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

use reqwest::Url;
use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolResult {
    pub content: String,
    pub is_error: bool,
}

impl ToolResult {
    pub fn text(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            is_error: false,
        }
    }

    pub fn error(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            is_error: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolContext {
    pub cwd: PathBuf,
}

impl ToolContext {
    pub fn new(cwd: impl Into<PathBuf>) -> Self {
        Self { cwd: cwd.into() }
    }

    pub fn resolve_path(&self, path: &str) -> PathBuf {
        let expanded = expand_home(path);
        let path = PathBuf::from(expanded);
        if path.is_absolute() {
            normalize_path(path)
        } else {
            normalize_path(self.cwd.join(path))
        }
    }
}

pub trait Tool: Send + Sync {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn is_read_only(&self) -> bool;
    fn call(&self, input: serde_json::Value, context: &ToolContext) -> ToolResult;
}

#[derive(Default)]
pub struct ToolRegistry {
    tools: BTreeMap<&'static str, Box<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_builtin_tools() -> Self {
        let mut registry = Self::new();
        registry.register(Box::<ReadTool>::default());
        registry.register(Box::<FileMetadataTool>::default());
        registry.register(Box::<ReadFilesTool>::default());
        registry.register(Box::<GlobTool>::default());
        registry.register(Box::<GrepTool>::default());
        registry.register(Box::<WebFetchTool>::default());
        registry.register(Box::<WebSearchTool>::default());
        registry.register(Box::<WriteTool>::default());
        registry.register(Box::<EditTool>::default());
        registry.register(Box::<BashTool>::default());
        registry.register(Box::<TodoWriteTool>::default());
        registry.register(Box::<TaskCreateTool>::default());
        registry.register(Box::<TaskListTool>::default());
        registry.register(Box::<TaskGetTool>::default());
        registry.register(Box::<TaskUpdateTool>::default());
        registry.register(Box::<TaskStopTool>::default());
        registry.register(Box::<TaskOutputTool>::default());
        registry
    }

    pub fn register(&mut self, tool: Box<dyn Tool>) {
        self.tools.insert(tool.name(), tool);
    }

    pub fn get(&self, name: &str) -> Option<&dyn Tool> {
        self.tools.get(name).map(Box::as_ref)
    }

    pub fn call(&self, name: &str, input: serde_json::Value, context: &ToolContext) -> ToolResult {
        let Some(tool) = self.get(name) else {
            return ToolResult::error(format!("Unknown tool: {name}"));
        };
        tool.call(input, context)
    }

    pub fn names(&self) -> Vec<&'static str> {
        self.tools.keys().copied().collect()
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct ReadTool;

impl Tool for ReadTool {
    fn name(&self) -> &'static str {
        "Read"
    }

    fn description(&self) -> &'static str {
        "Read a text file with optional line offset and limit."
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn call(&self, input: serde_json::Value, context: &ToolContext) -> ToolResult {
        let Some(file_path) = input.get("file_path").and_then(serde_json::Value::as_str) else {
            return ToolResult::error("Missing file_path parameter");
        };
        if file_path.trim().is_empty() {
            return ToolResult::error("file_path is required");
        }

        let path = context.resolve_path(file_path);
        let offset = input
            .get("offset")
            .and_then(serde_json::Value::as_u64)
            .and_then(|value| usize::try_from(value).ok());
        let limit = input
            .get("limit")
            .and_then(serde_json::Value::as_u64)
            .and_then(|value| usize::try_from(value).ok());

        read_text_file(&path, offset, limit)
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct FileMetadataTool;

impl Tool for FileMetadataTool {
    fn name(&self) -> &'static str {
        "FileMetadata"
    }

    fn description(&self) -> &'static str {
        "Inspect file metadata without reading file contents."
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn call(&self, input: serde_json::Value, context: &ToolContext) -> ToolResult {
        let Some(file_path) = input.get("file_path").and_then(serde_json::Value::as_str) else {
            return ToolResult::error("file_path is required");
        };
        if file_path.trim().is_empty() {
            return ToolResult::error("file_path is required");
        }

        let path = context.resolve_path(file_path);
        ToolResult::text(render_file_metadata(&path))
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct ReadFilesTool;

impl Tool for ReadFilesTool {
    fn name(&self) -> &'static str {
        "ReadFiles"
    }

    fn description(&self) -> &'static str {
        "Read multiple text files in one ordered result."
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn call(&self, input: serde_json::Value, context: &ToolContext) -> ToolResult {
        let Some(paths) = input.get("paths").and_then(serde_json::Value::as_array) else {
            return ToolResult::error("paths must be an array");
        };
        if paths.is_empty() {
            return ToolResult::error("No paths provided");
        }
        if paths.len() > 50 {
            return ToolResult::error("paths array exceeds 50 entries; split into smaller batches");
        }

        let mut sections = Vec::new();
        let mut estimated_tokens = 0usize;
        for path_value in paths {
            let Some(file_path) = path_value.as_str() else {
                sections.push(String::from(
                    "===== <invalid> =====\n[Error: path must be a string]",
                ));
                continue;
            };
            let path = context.resolve_path(file_path);
            let section = match read_text_file_limited(&path, Some(2_000)) {
                Ok(content) => format!("===== {file_path} =====\n{content}"),
                Err(message) => format!("===== {file_path} =====\n[Error: {message}]"),
            };
            estimated_tokens += estimate_tokens(&section);
            if estimated_tokens > 60_000 {
                sections.push(format!(
                    "===== {file_path} =====\n[Skipped: total output cap reached (60000 tokens)]"
                ));
                break;
            }
            sections.push(section);
        }

        ToolResult::text(sections.join("\n\n"))
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct GlobTool;

impl Tool for GlobTool {
    fn name(&self) -> &'static str {
        "Glob"
    }

    fn description(&self) -> &'static str {
        "Find files by glob pattern."
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn call(&self, input: serde_json::Value, context: &ToolContext) -> ToolResult {
        let Some(pattern) = input.get("pattern").and_then(serde_json::Value::as_str) else {
            return ToolResult::error("Missing pattern parameter");
        };
        if pattern.trim().is_empty() {
            return ToolResult::error("pattern is required");
        }

        let base = input
            .get("path")
            .and_then(serde_json::Value::as_str)
            .map(|path| context.resolve_path(path))
            .unwrap_or_else(|| context.cwd.clone());
        if !base.is_dir() {
            return ToolResult::error(format!("Path is not a directory: {}", base.display()));
        }

        let matcher = GlobMatcher::new(pattern);
        let mut matches = Vec::new();
        collect_files(&base, &base, &mut |relative, _full_path| {
            if matcher.matches(relative) {
                matches.push(relative.to_string_lossy().replace('\\', "/"));
            }
            matches.len() < 100
        });

        matches.sort();
        let truncated = matches.len() >= 100;
        let output = matches.join("\n");
        let suffix = if truncated {
            "\n\n[Results truncated at 100]"
        } else {
            ""
        };
        ToolResult::text(format!(
            "Found {} file{}\n\n{}{}",
            matches.len(),
            if matches.len() == 1 { "" } else { "s" },
            output,
            suffix
        ))
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct GrepTool;

impl Tool for GrepTool {
    fn name(&self) -> &'static str {
        "Grep"
    }

    fn description(&self) -> &'static str {
        "Search text files using a regular expression."
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn call(&self, input: serde_json::Value, context: &ToolContext) -> ToolResult {
        let Some(pattern) = input.get("pattern").and_then(serde_json::Value::as_str) else {
            return ToolResult::error("Missing pattern parameter");
        };
        if pattern.trim().is_empty() {
            return ToolResult::error("pattern is required");
        }

        let pattern = if input.get("-i").and_then(serde_json::Value::as_bool) == Some(true) {
            format!("(?i){pattern}")
        } else {
            pattern.to_owned()
        };
        let regex = match regex::Regex::new(&pattern) {
            Ok(regex) => regex,
            Err(error) => return ToolResult::error(format!("Invalid regex pattern: {error}")),
        };
        let base = input
            .get("path")
            .and_then(serde_json::Value::as_str)
            .map(|path| context.resolve_path(path))
            .unwrap_or_else(|| context.cwd.clone());
        let output_mode = input
            .get("output_mode")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("files_with_matches");
        let head_limit = input
            .get("head_limit")
            .and_then(serde_json::Value::as_u64)
            .and_then(|value| usize::try_from(value).ok())
            .unwrap_or(250);
        let offset = input
            .get("offset")
            .and_then(serde_json::Value::as_u64)
            .and_then(|value| usize::try_from(value).ok())
            .unwrap_or(0);
        let glob = input
            .get("glob")
            .and_then(serde_json::Value::as_str)
            .map(GlobMatcher::new);

        grep_path(
            &base,
            &base,
            &regex,
            output_mode,
            glob.as_ref(),
            head_limit,
            offset,
        )
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct WebFetchTool;

impl Tool for WebFetchTool {
    fn name(&self) -> &'static str {
        "WebFetch"
    }

    fn description(&self) -> &'static str {
        "Fetch web content over HTTP or HTTPS and return readable text."
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn call(&self, input: serde_json::Value, _context: &ToolContext) -> ToolResult {
        let Some(url_value) = input.get("url").and_then(serde_json::Value::as_str) else {
            return ToolResult::error("url is required");
        };
        if url_value.trim().is_empty() {
            return ToolResult::error("url is required");
        }

        let requested_url = match Url::parse(url_value) {
            Ok(url) if matches!(url.scheme(), "http" | "https") => url,
            _ => return ToolResult::error(format!("Invalid URL: {url_value}")),
        };

        fetch_web_content(&requested_url)
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct WebSearchTool;

impl WebSearchTool {
    pub fn call_with_environment(
        &self,
        input: serde_json::Value,
        env: &BTreeMap<String, String>,
    ) -> ToolResult {
        web_search_with_environment(input, env)
    }
}

impl Tool for WebSearchTool {
    fn name(&self) -> &'static str {
        "WebSearch"
    }

    fn description(&self) -> &'static str {
        "Search the web using configured Brave Search or Serper credentials."
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn call(&self, input: serde_json::Value, _context: &ToolContext) -> ToolResult {
        let env = std::env::vars().collect::<BTreeMap<_, _>>();
        self.call_with_environment(input, &env)
    }
}

fn web_search_with_environment(
    input: serde_json::Value,
    env: &BTreeMap<String, String>,
) -> ToolResult {
    let Some(query) = input.get("query").and_then(serde_json::Value::as_str) else {
        return ToolResult::error("Missing query parameter");
    };
    if query.chars().count() < 2 {
        return ToolResult::error("query must be at least 2 characters");
    }
    if input.get("allowed_domains").is_some() && input.get("blocked_domains").is_some() {
        return ToolResult::error("Cannot specify both allowed_domains and blocked_domains");
    }

    let allowed_domains = extract_string_array(input.get("allowed_domains"));
    let blocked_domains = extract_string_array(input.get("blocked_domains"));

    if let Some(api_key) = env
        .get("BRAVE_SEARCH_API_KEY")
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        && let Ok(results) = search_brave(query, api_key, &allowed_domains, &blocked_domains)
    {
        return ToolResult::text(results);
    }

    if let Some(api_key) = env
        .get("SERPER_API_KEY")
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        && let Ok(results) = search_serper(query, api_key, &allowed_domains, &blocked_domains)
    {
        return ToolResult::text(results);
    }

    let encoded = encode_query_component(query);
    ToolResult::error(format!(
        "WebSearch requires an API key. Set one of:\n  export BRAVE_SEARCH_API_KEY=<key>   # https://search.brave.com (2000 free/month)\n  export SERPER_API_KEY=<key>          # https://serper.dev (Google results)\n\nAlternative: use WebFetch to retrieve search results directly:\n  - https://html.duckduckgo.com/html/?q={encoded}\n  - https://www.google.com/search?q={encoded}"
    ))
}

#[derive(Debug, Deserialize)]
struct BraveSearchResponse {
    web: Option<BraveWebResults>,
}

#[derive(Debug, Deserialize)]
struct BraveWebResults {
    results: Vec<BraveWebResult>,
}

#[derive(Debug, Clone, Deserialize)]
struct BraveWebResult {
    title: String,
    url: String,
    description: Option<String>,
    age: Option<String>,
}

fn search_brave(
    query: &str,
    api_key: &str,
    allowed_domains: &[String],
    blocked_domains: &[String],
) -> Result<String, String> {
    let mut url =
        Url::parse("https://api.search.brave.com/res/v1/web/search").map_err(|e| e.to_string())?;
    url.query_pairs_mut()
        .append_pair("q", query)
        .append_pair("count", "10")
        .append_pair("text_decorations", "false");

    let client = web_search_client()?;
    let response = client
        .get(url)
        .header("X-Subscription-Token", api_key)
        .header(reqwest::header::ACCEPT, "application/json")
        .send()
        .map_err(|error| error.to_string())?;
    if !response.status().is_success() {
        return Err(format!(
            "Brave Search failed with HTTP {}",
            response.status()
        ));
    }

    let parsed = response
        .json::<BraveSearchResponse>()
        .map_err(|error| error.to_string())?;
    let results = parsed.web.map(|web| web.results).unwrap_or_default();
    let results = apply_domain_filters(
        results
            .into_iter()
            .map(|result| (result.url.clone(), result)),
        allowed_domains,
        blocked_domains,
    );

    Ok(format_brave_results(query, &results))
}

fn format_brave_results(query: &str, results: &[BraveWebResult]) -> String {
    if results.is_empty() {
        return format!("No results found for: \"{query}\"");
    }

    let mut lines = vec![format!(
        "Web search results for: \"{query}\" (Brave Search)"
    )];
    for (index, result) in results.iter().take(8).enumerate() {
        lines.push(String::new());
        lines.push(format!("{}. {}", index + 1, result.title));
        lines.push(format!("   URL: {}", result.url));
        if let Some(description) = result
            .description
            .as_ref()
            .filter(|value| !value.is_empty())
        {
            lines.push(format!("   {description}"));
        }
        if let Some(age) = result.age.as_ref().filter(|value| !value.is_empty()) {
            lines.push(format!("   {age}"));
        }
    }
    lines.push(String::new());
    lines.push(String::from(
        "Use WebFetch on any URL above to read the full page content.",
    ));
    lines.join("\n")
}

#[derive(Debug, Deserialize)]
struct SerperResponse {
    organic: Option<Vec<SerperOrganic>>,
}

#[derive(Debug, Clone, Deserialize)]
struct SerperOrganic {
    title: String,
    link: String,
    snippet: Option<String>,
    date: Option<String>,
}

fn search_serper(
    query: &str,
    api_key: &str,
    allowed_domains: &[String],
    blocked_domains: &[String],
) -> Result<String, String> {
    let client = web_search_client()?;
    let response = client
        .post("https://google.serper.dev/search")
        .header("X-API-KEY", api_key)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .json(&serde_json::json!({"q": query, "num": 10}))
        .send()
        .map_err(|error| error.to_string())?;
    if !response.status().is_success() {
        return Err(format!(
            "Serper search failed with HTTP {}",
            response.status()
        ));
    }

    let parsed = response
        .json::<SerperResponse>()
        .map_err(|error| error.to_string())?;
    let results = apply_domain_filters(
        parsed
            .organic
            .unwrap_or_default()
            .into_iter()
            .map(|result| (result.link.clone(), result)),
        allowed_domains,
        blocked_domains,
    );

    Ok(format_serper_results(query, &results))
}

fn format_serper_results(query: &str, results: &[SerperOrganic]) -> String {
    if results.is_empty() {
        return format!("No results found for: \"{query}\"");
    }

    let mut lines = vec![format!(
        "Web search results for: \"{query}\" (Google via Serper)"
    )];
    for (index, result) in results.iter().take(8).enumerate() {
        lines.push(String::new());
        lines.push(format!("{}. {}", index + 1, result.title));
        lines.push(format!("   URL: {}", result.link));
        if let Some(snippet) = result.snippet.as_ref().filter(|value| !value.is_empty()) {
            lines.push(format!("   {snippet}"));
        }
        if let Some(date) = result.date.as_ref().filter(|value| !value.is_empty()) {
            lines.push(format!("   {date}"));
        }
    }
    lines.push(String::new());
    lines.push(String::from(
        "Use WebFetch on any URL above to read the full page content.",
    ));
    lines.join("\n")
}

fn web_search_client() -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|error| error.to_string())
}

fn apply_domain_filters<T>(
    items: impl IntoIterator<Item = (String, T)>,
    allowed_domains: &[String],
    blocked_domains: &[String],
) -> Vec<T> {
    items
        .into_iter()
        .filter(|(url, _)| domain_allowed(url, allowed_domains, blocked_domains))
        .take(8)
        .map(|(_, value)| value)
        .collect()
}

fn domain_allowed(url: &str, allowed_domains: &[String], blocked_domains: &[String]) -> bool {
    let Some(host) = Url::parse(url)
        .ok()
        .and_then(|url| url.host_str().map(ToOwned::to_owned))
    else {
        return true;
    };
    if !allowed_domains.is_empty() {
        return allowed_domains
            .iter()
            .any(|domain| host == *domain || host.ends_with(&format!(".{domain}")));
    }
    if !blocked_domains.is_empty() {
        return !blocked_domains
            .iter()
            .any(|domain| host == *domain || host.ends_with(&format!(".{domain}")));
    }
    true
}

fn extract_string_array(value: Option<&serde_json::Value>) -> Vec<String> {
    value
        .and_then(serde_json::Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(serde_json::Value::as_str)
                .map(ToOwned::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn encode_query_component(query: &str) -> String {
    let Ok(mut url) = Url::parse("https://example.invalid/") else {
        return query.to_owned();
    };
    url.query_pairs_mut().append_pair("q", query);
    url.query()
        .and_then(|query| query.strip_prefix("q="))
        .unwrap_or(query)
        .to_owned()
}

fn fetch_web_content(url: &Url) -> ToolResult {
    let client = match reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent(
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 \
             (KHTML, like Gecko) Deeptide/1.0 Safari/537.36",
        )
        .build()
    {
        Ok(client) => client,
        Err(error) => return ToolResult::error(format!("Failed to create HTTP client: {error}")),
    };

    let response = match client
        .get(url.clone())
        .header(
            reqwest::header::ACCEPT,
            "text/html,application/xhtml+xml,application/xml;q=0.9,text/plain;q=0.8,*/*;q=0.5",
        )
        .header(reqwest::header::ACCEPT_LANGUAGE, "en-US,en;q=0.9")
        .send()
    {
        Ok(response) => response,
        Err(error) => return ToolResult::error(format!("Failed to fetch URL: {error}")),
    };

    let status = response.status();
    let final_url = response.url().clone();
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("unknown")
        .to_owned();
    let selected_headers = selected_response_headers(&response);
    let bytes = match response.bytes() {
        Ok(bytes) => bytes,
        Err(error) => return ToolResult::error(format!("Failed to read response body: {error}")),
    };
    let size = bytes.len();

    let body = decode_response_body(&bytes);
    let is_http_error = !status.is_success();
    let mut output = response_header(
        status.as_u16(),
        size,
        url.as_str(),
        final_url.as_str(),
        &content_type,
        &selected_headers,
    );

    let Some(body) = body else {
        output.push_str("\n\nCould not decode response body (binary content).");
        return if is_http_error {
            ToolResult::error(output)
        } else {
            ToolResult::text(output)
        };
    };

    if is_http_error {
        output.push_str("\n\n--- HTTP diagnostics ---\n");
        output.push_str(&http_diagnostic(
            status.as_u16(),
            &selected_headers,
            final_url.as_str(),
            url.as_str(),
        ));
    }

    let lower_content_type = content_type.to_lowercase();
    let is_html_like = lower_content_type.contains("html") || body.to_lowercase().contains("<html");
    let mut text = if is_html_like {
        html_to_text(&body, Some(&final_url))
    } else {
        decode_entities(&body)
    };
    text = text.trim().to_owned();
    let total_chars = text.chars().count();
    if total_chars > 50_000 {
        text = format!(
            "{}\n\n[Content truncated: {total_chars} total chars; fetch a narrower URL or linked resource if needed]",
            truncate_chars(&text, 50_000)
        );
    }

    output.push_str("\n\n");
    output.push_str(&text);

    if is_http_error {
        ToolResult::error(output)
    } else {
        ToolResult::text(output)
    }
}

fn selected_response_headers(response: &reqwest::blocking::Response) -> Vec<(String, String)> {
    ["Location", "Retry-After", "WWW-Authenticate", "Server"]
        .into_iter()
        .filter_map(|name| {
            response
                .headers()
                .get(name)
                .and_then(|value| value.to_str().ok())
                .filter(|value| !value.is_empty())
                .map(|value| (name.to_owned(), value.to_owned()))
        })
        .collect()
}

fn decode_response_body(bytes: &[u8]) -> Option<String> {
    String::from_utf8(bytes.to_vec()).ok().or_else(|| {
        if bytes.contains(&0) {
            None
        } else {
            Some(bytes.iter().map(|byte| char::from(*byte)).collect())
        }
    })
}

fn response_header(
    code: u16,
    size: usize,
    requested_url: &str,
    final_url: &str,
    content_type: &str,
    selected_headers: &[(String, String)],
) -> String {
    let mut lines = vec![
        format!("HTTP {code} | {}", format_byte_count(size)),
        format!("URL: {requested_url}"),
    ];
    if final_url != requested_url {
        lines.push(format!("Final URL: {final_url}"));
    }
    lines.push(format!("Content-Type: {content_type}"));
    lines.extend(
        selected_headers
            .iter()
            .map(|(name, value)| format!("{name}: {value}")),
    );
    lines.join("\n")
}

fn http_diagnostic(
    code: u16,
    selected_headers: &[(String, String)],
    final_url: &str,
    requested_url: &str,
) -> String {
    let mut lines = Vec::new();
    if final_url != requested_url {
        lines.push(String::from(
            "Request followed redirects from the requested URL; verify the final URL when diagnosing.",
        ));
    }

    match code {
        400 => lines.push(String::from("400 Bad Request: inspect query encoding, required parameters, and whether the endpoint expects a different method or content type.")),
        401 => lines.push(String::from("401 Unauthorized: authentication is missing, expired, or not accepted by this endpoint.")),
        403 => lines.push(String::from("403 Forbidden: access may require login, a token, different permissions, or the site may be blocking automated clients.")),
        404 => lines.push(String::from("404 Not Found: verify the path, slug, version, and redirects; the host is reachable but this resource was not served.")),
        408 => lines.push(String::from("408 Request Timeout: retry later and check whether the server expects a smaller or more specific request.")),
        409 => lines.push(String::from("409 Conflict: the request reached the endpoint but conflicts with current resource state.")),
        410 => lines.push(String::from("410 Gone: the resource has been intentionally removed or archived.")),
        429 => {
            let retry = selected_headers
                .iter()
                .find(|(name, _)| name == "Retry-After")
                .map(|(_, value)| format!(" Retry-After: {value}."))
                .unwrap_or_default();
            lines.push(format!("429 Rate Limited: wait before retrying, reduce request frequency, or use an authenticated API.{retry}"));
        }
        500..=599 => lines.push(format!("{code} Server Error: the upstream service failed; capture the body, final URL, and relevant headers before retrying.")),
        _ => lines.push(format!("{code} HTTP status: use the response body plus headers above as the primary evidence.")),
    }

    lines.join("\n")
}

fn html_to_text(html: &str, base_url: Option<&Url>) -> String {
    let mut text = regex_replace(html, r"(?is)<!--.*?-->", "");
    text = regex_replace(&text, r"(?is)<script\b[^>]*>.*?</script>", "");
    text = regex_replace(&text, r"(?is)<style\b[^>]*>.*?</style>", "");
    text = replace_anchor_tags(&text, base_url);
    text = regex_replace(&text, r"(?i)<br\s*/?>", "\n");
    text = regex_replace(
        &text,
        r"(?i)</(p|div|section|article|header|footer|main|nav|tr|table|ul|ol|h[1-6])\s*>",
        "\n",
    );
    text = regex_replace(&text, r"(?i)<li\b[^>]*>", "\n- ");
    text = regex_replace(&text, r"(?i)</(td|th)\s*>", "\t");
    text = regex_replace(&text, r"<[^>]+>", "");

    let decoded = decode_entities(&text);
    let decoded = regex_replace(&decoded, r"[ \t\u{00A0}]+", " ");
    let decoded = regex_replace(&decoded, r" *\n *", "\n");
    regex_replace(&decoded, r"\n{3,}", "\n\n")
}

fn replace_anchor_tags(html: &str, base_url: Option<&Url>) -> String {
    let Ok(regex) = regex::Regex::new(r#"(?is)<a\b([^>]*)>(.*?)</a>"#) else {
        return html.to_owned();
    };
    regex
        .replace_all(html, |captures: &regex::Captures<'_>| {
            let attrs = captures.get(1).map(|value| value.as_str()).unwrap_or("");
            let inner = captures.get(2).map(|value| value.as_str()).unwrap_or("");
            let label = regex_replace(inner, r"<[^>]+>", "");
            if let Some(href) = extract_href(attrs).filter(|href| !href.is_empty()) {
                let resolved = base_url
                    .and_then(|base| base.join(&href).ok())
                    .map(|url| url.to_string())
                    .unwrap_or(href);
                format!("{label} [{resolved}]")
            } else {
                label
            }
        })
        .into_owned()
}

fn extract_href(attrs: &str) -> Option<String> {
    let regex = regex::Regex::new(r#"(?is)\bhref\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s>]+))"#).ok()?;
    let captures = regex.captures(attrs)?;
    (1..=3)
        .find_map(|group| captures.get(group).map(|value| value.as_str()))
        .map(decode_entities)
}

fn decode_entities(text: &str) -> String {
    let mut output = String::new();
    let mut chars = text.char_indices().peekable();
    while let Some((index, ch)) = chars.next() {
        if ch != '&' {
            output.push(ch);
            continue;
        }

        let Some((semicolon_index, _)) = text[index..]
            .char_indices()
            .take_while(|(_, candidate)| *candidate != '\n')
            .find(|(_, candidate)| *candidate == ';')
        else {
            output.push(ch);
            continue;
        };
        if semicolon_index > 12 {
            output.push(ch);
            continue;
        }

        let entity = &text[index + 1..index + semicolon_index];
        if let Some(replacement) = decode_entity(entity) {
            output.push(replacement);
            while chars
                .peek()
                .is_some_and(|(next_index, _)| *next_index <= index + semicolon_index)
            {
                chars.next();
            }
        } else {
            output.push(ch);
        }
    }
    output
}

fn decode_entity(entity: &str) -> Option<char> {
    match entity {
        "amp" => Some('&'),
        "lt" => Some('<'),
        "gt" => Some('>'),
        "quot" => Some('"'),
        "apos" | "#39" => Some('\''),
        "nbsp" => Some(' '),
        value if value.starts_with("#x") || value.starts_with("#X") => {
            u32::from_str_radix(&value[2..], 16)
                .ok()
                .and_then(char::from_u32)
        }
        value if value.starts_with('#') => value[1..].parse::<u32>().ok().and_then(char::from_u32),
        _ => None,
    }
}

fn regex_replace(input: &str, pattern: &str, replacement: &str) -> String {
    regex::Regex::new(pattern)
        .map(|regex| regex.replace_all(input, replacement).into_owned())
        .unwrap_or_else(|_| input.to_owned())
}

#[derive(Debug, Default, Clone, Copy)]
pub struct BashTool;

impl Tool for BashTool {
    fn name(&self) -> &'static str {
        "Bash"
    }

    fn description(&self) -> &'static str {
        "Execute a single-line shell command in the current workspace."
    }

    fn is_read_only(&self) -> bool {
        false
    }

    fn call(&self, input: serde_json::Value, context: &ToolContext) -> ToolResult {
        let Some(command) = input.get("command").and_then(serde_json::Value::as_str) else {
            return ToolResult::error("Missing command parameter");
        };
        if command.trim().is_empty() {
            return ToolResult::error("command is required and must be non-empty");
        }
        if command.contains('\n') || command.contains('\r') {
            return ToolResult::error("command must be a single line; rewrite it with && or ;");
        }

        let timeout_ms = input
            .get("timeout")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(120_000)
            .min(600_000);
        if input
            .get("timeout")
            .and_then(serde_json::Value::as_u64)
            .is_some_and(|timeout| timeout > 600_000)
        {
            return ToolResult::error("timeout must be <= 600000ms (10 minutes)");
        }

        if input
            .get("run_in_background")
            .and_then(serde_json::Value::as_bool)
            == Some(true)
        {
            return start_background_command(command, context);
        }

        execute_shell_command(command, context, Duration::from_millis(timeout_ms))
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct TodoWriteTool;

impl Tool for TodoWriteTool {
    fn name(&self) -> &'static str {
        "TodoWrite"
    }

    fn description(&self) -> &'static str {
        "Replace the in-memory todo list in one call."
    }

    fn is_read_only(&self) -> bool {
        false
    }

    fn call(&self, input: serde_json::Value, _context: &ToolContext) -> ToolResult {
        let Some(raw_items) = input.get("todos").and_then(serde_json::Value::as_array) else {
            return ToolResult::error("Missing or invalid todos array");
        };

        let parsed = raw_items
            .iter()
            .filter_map(parse_todo_item)
            .collect::<Vec<_>>();
        let all_done = !parsed.is_empty()
            && parsed
                .iter()
                .all(|item| item.status == TodoStatus::Completed);
        replace_todos(parsed.clone());

        if all_done {
            return ToolResult::text(
                "Todo list cleared (all tasks completed). Proceed with your summary.",
            );
        }

        ToolResult::text(format!(
            "Todo list updated ({} items). Ensure that you continue to use the todo list to track your progress. Please proceed with the current tasks if applicable.",
            parsed.len()
        ))
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct TaskCreateTool;

impl Tool for TaskCreateTool {
    fn name(&self) -> &'static str {
        "TaskCreate"
    }

    fn description(&self) -> &'static str {
        "Create one in-memory task with a subject and description."
    }

    fn is_read_only(&self) -> bool {
        false
    }

    fn call(&self, input: serde_json::Value, _context: &ToolContext) -> ToolResult {
        let Some(subject) = input.get("subject").and_then(serde_json::Value::as_str) else {
            return ToolResult::error("Missing subject or description");
        };
        let Some(description) = input.get("description").and_then(serde_json::Value::as_str) else {
            return ToolResult::error("Missing subject or description");
        };
        if subject.trim().is_empty() || description.trim().is_empty() {
            return ToolResult::error("Missing subject or description");
        }

        add_todo(TodoItem {
            content: subject.to_owned(),
            status: TodoStatus::Pending,
            active_form: Some(description.to_owned()),
        });

        ToolResult::text(format!("Task created: {subject}"))
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct TaskListTool;

impl Tool for TaskListTool {
    fn name(&self) -> &'static str {
        "TaskList"
    }

    fn description(&self) -> &'static str {
        "List the current in-memory todo tasks."
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn call(&self, _input: serde_json::Value, _context: &ToolContext) -> ToolResult {
        let todos = list_todos();
        if todos.is_empty() {
            return ToolResult::text("No tasks.");
        }

        let lines = todos
            .iter()
            .enumerate()
            .map(|(index, item)| format!("#{} {} {}", index + 1, item.status.icon(), item.content))
            .collect::<Vec<_>>()
            .join("\n");
        ToolResult::text(lines)
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct TaskGetTool;

impl Tool for TaskGetTool {
    fn name(&self) -> &'static str {
        "TaskGet"
    }

    fn description(&self) -> &'static str {
        "Get full details for one in-memory todo task by ID."
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn call(&self, input: serde_json::Value, _context: &ToolContext) -> ToolResult {
        let Some(task_id) = input.get("taskId").and_then(serde_json::Value::as_str) else {
            return ToolResult::error("Missing taskId parameter");
        };
        let Some(task) = get_todo(task_id) else {
            return ToolResult::error(format!("Task not found: {task_id}"));
        };

        let mut lines = vec![
            format!("Task: {}", task.content),
            format!("ID: {task_id}"),
            format!("Status: {}", task.status.as_str()),
        ];
        if let Some(active_form) = task.active_form.filter(|value| !value.is_empty()) {
            lines.push(format!("Description: {active_form}"));
        }
        ToolResult::text(lines.join("\n"))
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct TaskUpdateTool;

impl Tool for TaskUpdateTool {
    fn name(&self) -> &'static str {
        "TaskUpdate"
    }

    fn description(&self) -> &'static str {
        "Update an in-memory todo task by ID."
    }

    fn is_read_only(&self) -> bool {
        false
    }

    fn call(&self, input: serde_json::Value, _context: &ToolContext) -> ToolResult {
        let Some(task_id) = input.get("taskId").and_then(serde_json::Value::as_str) else {
            return ToolResult::error("Missing taskId");
        };

        if input.get("status").and_then(serde_json::Value::as_str) == Some("deleted") {
            return if delete_todo(task_id) {
                ToolResult::text(format!("Task #{task_id} deleted"))
            } else {
                ToolResult::error(format!("Task #{task_id} not found"))
            };
        }

        let status = input
            .get("status")
            .and_then(serde_json::Value::as_str)
            .map(TodoStatus::parse);
        let subject = input
            .get("subject")
            .and_then(serde_json::Value::as_str)
            .map(ToOwned::to_owned);
        let description = input
            .get("description")
            .and_then(serde_json::Value::as_str)
            .map(ToOwned::to_owned);

        let changes = update_todo(task_id, status, subject, description);
        if changes.is_empty() {
            return ToolResult::text(format!("Task #{task_id}: no changes (task may not exist)"));
        }

        ToolResult::text(format!("Task #{task_id} updated: {}", changes.join(", ")))
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct TaskStopTool;

impl Tool for TaskStopTool {
    fn name(&self) -> &'static str {
        "TaskStop"
    }

    fn description(&self) -> &'static str {
        "Stop a task by marking it completed."
    }

    fn is_read_only(&self) -> bool {
        false
    }

    fn call(&self, input: serde_json::Value, _context: &ToolContext) -> ToolResult {
        let Some(task_id) = input.get("taskId").and_then(serde_json::Value::as_str) else {
            return ToolResult::error("Missing taskId parameter");
        };
        let explanation = input
            .get("explanation")
            .and_then(serde_json::Value::as_str)
            .filter(|value| !value.is_empty());

        if complete_todo(task_id) {
            if let Some(explanation) = explanation {
                ToolResult::text(format!("Task stopped: {explanation}"))
            } else {
                ToolResult::text(format!("Task {task_id} stopped"))
            }
        } else {
            ToolResult::error(format!("Task not found or already completed: {task_id}"))
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct TaskOutputTool;

impl Tool for TaskOutputTool {
    fn name(&self) -> &'static str {
        "TaskOutput"
    }

    fn description(&self) -> &'static str {
        "Retrieve recorded metadata and output for one task."
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn call(&self, input: serde_json::Value, _context: &ToolContext) -> ToolResult {
        let Some(task_id) = input.get("task_id").and_then(serde_json::Value::as_str) else {
            return ToolResult::error("task_id is required");
        };
        if task_id.trim().is_empty() {
            return ToolResult::error("task_id is required");
        }

        let Some(task) = get_todo(task_id) else {
            return ToolResult::error(
                serde_json::json!({
                    "retrieval_status": "not_ready",
                    "task": null,
                    "note": format!("no task with id {task_id} (TaskStorage tracks the agent's todo list; background output capture is not yet implemented)")
                })
                .to_string(),
            );
        };

        ToolResult::text(
            serde_json::json!({
                "retrieval_status": "success",
                "task": {
                    "task_id": task_id,
                    "task_type": "todo",
                    "status": task.status.as_str(),
                    "description": task.active_form.unwrap_or_default(),
                    "output": task.content
                }
            })
            .to_string(),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TodoItem {
    content: String,
    status: TodoStatus,
    active_form: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TodoStatus {
    Pending,
    InProgress,
    Completed,
    Deleted,
}

fn parse_todo_item(value: &serde_json::Value) -> Option<TodoItem> {
    let object = value.as_object()?;
    let content = object.get("content")?.as_str()?.to_owned();
    let status = object
        .get("status")
        .and_then(serde_json::Value::as_str)
        .map(TodoStatus::parse)
        .unwrap_or(TodoStatus::Pending);
    let active_form = object
        .get("activeForm")
        .and_then(serde_json::Value::as_str)
        .map(ToOwned::to_owned);
    Some(TodoItem {
        content,
        status,
        active_form,
    })
}

impl TodoStatus {
    fn parse(value: &str) -> Self {
        match value {
            "in_progress" => Self::InProgress,
            "completed" => Self::Completed,
            "deleted" => Self::Deleted,
            _ => Self::Pending,
        }
    }

    fn icon(self) -> &'static str {
        match self {
            Self::Pending => "○",
            Self::InProgress => "◉",
            Self::Completed => "⌬",
            Self::Deleted => "✕",
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::InProgress => "in_progress",
            Self::Completed => "completed",
            Self::Deleted => "deleted",
        }
    }
}

fn todo_storage() -> &'static Mutex<Vec<TodoItem>> {
    static STORAGE: OnceLock<Mutex<Vec<TodoItem>>> = OnceLock::new();
    STORAGE.get_or_init(|| Mutex::new(Vec::new()))
}

fn replace_todos(items: Vec<TodoItem>) {
    if let Ok(mut todos) = todo_storage().lock() {
        *todos = items;
    }
}

fn add_todo(item: TodoItem) {
    if let Ok(mut todos) = todo_storage().lock() {
        todos.push(item);
    }
}

fn list_todos() -> Vec<TodoItem> {
    todo_storage()
        .lock()
        .map(|todos| todos.clone())
        .unwrap_or_default()
}

fn get_todo(task_id: &str) -> Option<TodoItem> {
    let index = task_id.parse::<usize>().ok()?.checked_sub(1)?;
    todo_storage()
        .lock()
        .ok()
        .and_then(|todos| todos.get(index).cloned())
}

fn complete_todo(task_id: &str) -> bool {
    let Some(index) = task_id
        .parse::<usize>()
        .ok()
        .and_then(|value| value.checked_sub(1))
    else {
        return false;
    };
    let Ok(mut todos) = todo_storage().lock() else {
        return false;
    };
    let Some(todo) = todos.get_mut(index) else {
        return false;
    };
    if todo.status == TodoStatus::Completed {
        return false;
    }
    todo.status = TodoStatus::Completed;
    true
}

fn update_todo(
    task_id: &str,
    status: Option<TodoStatus>,
    subject: Option<String>,
    description: Option<String>,
) -> Vec<String> {
    let Some(index) = task_id
        .parse::<usize>()
        .ok()
        .and_then(|value| value.checked_sub(1))
    else {
        return Vec::new();
    };

    let Ok(mut todos) = todo_storage().lock() else {
        return Vec::new();
    };
    let Some(todo) = todos.get_mut(index) else {
        return Vec::new();
    };

    let mut changes = Vec::new();
    if let Some(status) = status {
        todo.status = status;
        changes.push(format!("status -> {}", status.as_str()));
    }
    if let Some(subject) = subject {
        todo.content = subject;
        changes.push(String::from("subject updated"));
    }
    if let Some(description) = description {
        todo.active_form = Some(description);
        changes.push(String::from("description updated"));
    }
    changes
}

fn delete_todo(task_id: &str) -> bool {
    let Some(index) = task_id
        .parse::<usize>()
        .ok()
        .and_then(|value| value.checked_sub(1))
    else {
        return false;
    };
    let Ok(mut todos) = todo_storage().lock() else {
        return false;
    };
    if index >= todos.len() {
        return false;
    }
    todos.remove(index);
    true
}

#[derive(Debug, Default, Clone, Copy)]
pub struct WriteTool;

impl Tool for WriteTool {
    fn name(&self) -> &'static str {
        "Write"
    }

    fn description(&self) -> &'static str {
        "Write complete UTF-8 file contents, creating parent directories as needed."
    }

    fn is_read_only(&self) -> bool {
        false
    }

    fn call(&self, input: serde_json::Value, context: &ToolContext) -> ToolResult {
        let Some(file_path) = input.get("file_path").and_then(serde_json::Value::as_str) else {
            return ToolResult::error(
                "Write requires JSON with both `file_path` and `content`. Example: {\"file_path\":\"roguelike.html\",\"content\":\"<complete file contents>\"}. Do not call Write with `{}`. Retry with the exact requested file path and complete content.",
            );
        };
        if file_path.trim().is_empty() {
            return ToolResult::error(
                "Write requires JSON with both `file_path` and `content`. Example: {\"file_path\":\"roguelike.html\",\"content\":\"<complete file contents>\"}. Do not call Write with `{}`. Retry with the exact requested file path and complete content.",
            );
        }

        let Some(content) = input.get("content").and_then(serde_json::Value::as_str) else {
            return ToolResult::error(
                "Write requires a string `content` field containing the complete file contents. Retry with JSON keys exactly `file_path` and `content`.",
            );
        };

        let path = context.resolve_path(file_path);
        let existed = path.exists();
        if let Some(parent) = path.parent()
            && let Err(error) = fs::create_dir_all(parent)
        {
            return ToolResult::error(format!(
                "Failed to create parent directory {}: {error}",
                parent.display()
            ));
        }

        let normalized = normalize_line_endings(content);
        if let Err(error) = fs::write(&path, normalized.as_bytes()) {
            return ToolResult::error(format!("Failed to write {}: {error}", path.display()));
        }

        let action = if existed { "Updated" } else { "Created" };
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(file_path);
        ToolResult::text(format!(
            "{action} file: {name} ({})\nPath: {}",
            format_byte_count(normalized.len()),
            path.display()
        ))
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct EditTool;

impl Tool for EditTool {
    fn name(&self) -> &'static str {
        "Edit"
    }

    fn description(&self) -> &'static str {
        "Perform exact string replacement in an existing UTF-8 file."
    }

    fn is_read_only(&self) -> bool {
        false
    }

    fn call(&self, input: serde_json::Value, context: &ToolContext) -> ToolResult {
        let Some(file_path) = input.get("file_path").and_then(serde_json::Value::as_str) else {
            return ToolResult::error("file_path is required");
        };
        if file_path.trim().is_empty() {
            return ToolResult::error("file_path is required");
        }
        let Some(old_string) = input.get("old_string").and_then(serde_json::Value::as_str) else {
            return ToolResult::error("old_string is required");
        };
        let Some(new_string) = input.get("new_string").and_then(serde_json::Value::as_str) else {
            return ToolResult::error("new_string is required");
        };
        if old_string == new_string {
            return ToolResult::error("old_string and new_string must be different");
        }

        let path = context.resolve_path(file_path);
        let replace_all = input
            .get("replace_all")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);

        if !path.exists() && old_string.is_empty() {
            if let Some(parent) = path.parent()
                && let Err(error) = fs::create_dir_all(parent)
            {
                return ToolResult::error(format!(
                    "Failed to create parent directory {}: {error}",
                    parent.display()
                ));
            }
            let normalized_new = normalize_line_endings(new_string);
            if let Err(error) = fs::write(&path, normalized_new.as_bytes()) {
                return ToolResult::error(format!("Failed to write {}: {error}", path.display()));
            }
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or(file_path);
            return ToolResult::text(format!("Created new file: {name}"));
        }

        let original = match fs::read_to_string(&path) {
            Ok(content) => content,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return ToolResult::error(format!("File does not exist: {file_path}"));
            }
            Err(error) => {
                return ToolResult::error(format!("Failed to read {}: {error}", path.display()));
            }
        };
        if old_string.is_empty() {
            return ToolResult::error(
                "old_string not found in file. The file may have been modified. Please re-read the file.",
            );
        }

        let (edited, matches) =
            match apply_exact_edit(&original, old_string, new_string, replace_all) {
                Ok(result) => result,
                Err(message) => return ToolResult::error(message),
            };
        if let Err(error) = fs::write(&path, edited.as_bytes()) {
            return ToolResult::error(format!("Failed to write {}: {error}", path.display()));
        }

        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(file_path);
        let occurrence = if replace_all && matches > 1 {
            format!("all {matches} occurrences")
        } else {
            String::from("1 occurrence")
        };
        ToolResult::text(format!("File edited successfully: {name} ({occurrence})"))
    }
}

fn read_text_file(path: &Path, offset: Option<usize>, limit: Option<usize>) -> ToolResult {
    let content = match read_text_file_limited(path, None) {
        Ok(content) => content,
        Err(message) => return ToolResult::error(message),
    };

    let start = offset.unwrap_or(1).saturating_sub(1);
    let needed = limit.unwrap_or(usize::MAX);
    let output = content
        .lines()
        .skip(start)
        .take(needed)
        .collect::<Vec<_>>()
        .join("\n");

    ToolResult::text(output)
}

fn read_text_file_limited(path: &Path, line_limit: Option<usize>) -> Result<String, String> {
    let metadata = match fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            let filename = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or_default();
            return Err(format!(
                "File does not exist: {}\nHint: the file may be at a different location. Use Glob(pattern: \"**/{filename}\") to locate it.",
                path.display()
            ));
        }
        Err(error) => {
            return Err(format!("Failed to inspect {}: {error}", path.display()));
        }
    };

    if metadata.is_dir() {
        return Err(format!(
            "Path is a directory: {}\nUse Glob to list files or Grep to search within it.",
            path.display()
        ));
    }
    if let Some(kind) = special_file_kind(path) {
        return Err(unsupported_special_file_message(path, kind, metadata.len()));
    }

    let file = match fs::File::open(path) {
        Ok(file) => file,
        Err(error) => {
            return Err(format!("Failed to open {}: {error}", path.display()));
        }
    };

    let mut selected = Vec::new();
    let mut total_lines = 0usize;

    for line in io::BufReader::new(file).lines() {
        total_lines += 1;
        let line = match line {
            Ok(line) => line,
            Err(error) => {
                return Err(format!("Failed to read {}: {error}", path.display()));
            }
        };

        if line_limit.is_none_or(|limit| selected.len() < limit) {
            selected.push((total_lines, line));
        }
        if line_limit.is_some_and(|limit| selected.len() >= limit) {
            break;
        }
    }

    let mut output = selected
        .into_iter()
        .map(|(line_number, line)| format!("{line_number}\t{line}"))
        .collect::<Vec<_>>()
        .join("\n");
    if let Some(limit) = line_limit
        && total_lines >= limit
    {
        output.push_str(&format!("\n[File truncated at {limit} lines]"));
    }

    Ok(output)
}

fn render_file_metadata(path: &Path) -> String {
    let mut lines = vec![format!("[FileMetadata] {}", path.display())];
    let metadata = match fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            lines.push(String::from("exists: false"));
            return lines.join("\n");
        }
        Err(error) => {
            lines.push(String::from("exists: unknown"));
            lines.push(format!("error: {error}"));
            return lines.join("\n");
        }
    };

    lines.push(String::from("exists: true"));
    lines.push(format!(
        "kind: {}",
        if metadata.is_dir() {
            "directory"
        } else if metadata.is_file() {
            "file"
        } else {
            "other"
        }
    ));
    if metadata.is_file() {
        lines.push(format!(
            "size: {}",
            format_byte_count(metadata.len() as usize)
        ));
    }
    if let Ok(created) = metadata.created() {
        lines.push(format!("created: {}", format_system_time(created)));
    }
    if let Ok(modified) = metadata.modified() {
        lines.push(format!("modified: {}", format_system_time(modified)));
    }
    lines.push(format!("readonly: {}", metadata.permissions().readonly()));

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        lines.push(format!(
            "mode: {:o}",
            metadata.permissions().mode() & 0o7777
        ));
    }

    let ext = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    lines.push(format!(
        "extension: {}",
        if ext.is_empty() { "none" } else { ext.as_str() }
    ));
    let (type_name, mime) = file_type_hint(&ext, &metadata);
    lines.push(format!("type: {type_name}"));
    lines.push(format!("mime: {mime}"));

    #[cfg(target_os = "macos")]
    append_macos_metadata(path, &mut lines);
    #[cfg(not(target_os = "macos"))]
    {
        lines.push(String::from("xattrs: unsupported on this platform"));
        lines.push(String::from("spotlight: unavailable"));
    }

    lines.join("\n")
}

fn format_system_time(value: std::time::SystemTime) -> String {
    let datetime = time::OffsetDateTime::from(value);
    datetime
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| format!("{datetime}"))
}

fn file_type_hint(ext: &str, metadata: &fs::Metadata) -> (&'static str, &'static str) {
    if metadata.is_dir() {
        return ("directory", "inode/directory");
    }
    match ext {
        "txt" | "md" | "rs" | "swift" | "js" | "ts" | "tsx" | "jsx" | "py" | "go" | "java"
        | "c" | "h" | "cpp" | "hpp" | "json" | "toml" | "yaml" | "yml" | "csv" | "tsv" | "html"
        | "css" | "xml" | "sh" | "zsh" | "bash" => ("text", "text/plain"),
        "png" => ("image", "image/png"),
        "jpg" | "jpeg" => ("image", "image/jpeg"),
        "gif" => ("image", "image/gif"),
        "webp" => ("image", "image/webp"),
        "bmp" => ("image", "image/bmp"),
        "tif" | "tiff" => ("image", "image/tiff"),
        "pdf" => ("PDF document", "application/pdf"),
        "zip" => ("archive", "application/zip"),
        "gz" | "tgz" => ("archive", "application/gzip"),
        "tar" => ("archive", "application/x-tar"),
        "doc" | "docx" | "rtf" | "odt" => ("office or rich document", "application/octet-stream"),
        "ppt" | "pptx" | "xls" | "xlsx" => ("office document", "application/octet-stream"),
        "sqlite" | "sqlite3" | "db" => ("database", "application/octet-stream"),
        "" => ("unknown", "unknown"),
        _ => ("unknown", "application/octet-stream"),
    }
}

#[cfg(target_os = "macos")]
fn append_macos_metadata(path: &Path, lines: &mut Vec<String>) {
    let xattrs = Command::new("/usr/bin/xattr").arg("-l").arg(path).output();
    match xattrs {
        Ok(output) if output.status.success() && !output.stdout.is_empty() => {
            let text = String::from_utf8_lossy(&output.stdout);
            let names = text
                .lines()
                .filter_map(|line| line.split_once(": ").map(|(name, _)| name.to_owned()))
                .collect::<Vec<_>>();
            if names.is_empty() {
                lines.push(String::from("xattrs: present"));
            } else {
                lines.push(format!("xattrs: {}", names.join(", ")));
            }
            if let Some(quarantine) = text
                .lines()
                .find(|line| line.starts_with("com.apple.quarantine:"))
            {
                lines.push(format!(
                    "quarantine: {}",
                    quarantine
                        .strip_prefix("com.apple.quarantine:")
                        .unwrap_or("")
                        .trim()
                ));
            } else {
                lines.push(String::from("quarantine: none"));
            }
        }
        _ => lines.push(String::from("xattrs: none")),
    }

    let spotlight = Command::new("/usr/bin/mdls")
        .args([
            "-name",
            "kMDItemKind",
            "-name",
            "kMDItemContentType",
            "-name",
            "kMDItemFSName",
        ])
        .arg(path)
        .output();
    match spotlight {
        Ok(output) if output.status.success() && !output.stdout.is_empty() => {
            lines.push(String::from("spotlight:"));
            for line in String::from_utf8_lossy(&output.stdout).lines() {
                let trimmed = line.trim();
                if !trimmed.is_empty() && !trimmed.ends_with("= (null)") {
                    lines.push(format!("  {trimmed}"));
                }
            }
        }
        _ => lines.push(String::from("spotlight: unavailable")),
    }
}

fn special_file_kind(path: &Path) -> Option<&'static str> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    match ext.as_str() {
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" | "tiff" | "tif" | "heic" | "avif" => {
            Some("image")
        }
        "pdf" => Some("PDF document"),
        "doc" | "docx" | "rtf" | "rtfd" | "odt" | "ppt" | "pptx" | "xls" | "xlsx" | "ods"
        | "odp" | "webarchive" => Some("office or rich document"),
        "zip" | "tar" | "gz" | "tgz" | "bz2" | "xz" | "7z" | "rar" | "dmg" | "pkg" => {
            Some("archive or package")
        }
        "mp3" | "wav" | "m4a" | "flac" | "mp4" | "mov" | "avi" | "mkv" | "webm" => {
            Some("media file")
        }
        "sqlite" | "sqlite3" | "db" => Some("database file"),
        _ => None,
    }
}

fn unsupported_special_file_message(path: &Path, kind: &str, bytes: u64) -> String {
    let file = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("<unknown>");
    format!(
        "reason: unsupported binary or package-like file\nfile: {file}\ntype: {kind} · size: {}\nlikely_cause: this path appears to be {kind}, and Read avoids dumping binary data into context.\nnext_action: use a dedicated tool for images, PDFs, Office files, archives, media, or app bundles; if this is actually source text, report the extension/filename so Read can classify it safely.",
        format_byte_count(bytes as usize)
    )
}

fn execute_shell_command(command: &str, context: &ToolContext, timeout: Duration) -> ToolResult {
    let mut child = match shell_command(command)
        .current_dir(&context.cwd)
        .env_remove("TIDE_API_KEY")
        .env_remove("DEEPSEEK_API_KEY")
        .env_remove("ZERO_CLI_API_KEY")
        .env_remove("ZERO_API_KEY")
        .env_remove("ANTHROPIC_API_KEY")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(error) => {
            return ToolResult::error(format!("Failed to execute command: {error}"));
        }
    };

    let started = Instant::now();
    let mut timed_out = false;
    loop {
        match child.try_wait() {
            Ok(Some(_status)) => break,
            Ok(None) if started.elapsed() >= timeout => {
                timed_out = true;
                let _ = child.kill();
                break;
            }
            Ok(None) => thread::sleep(Duration::from_millis(20)),
            Err(error) => return ToolResult::error(format!("Failed to wait for command: {error}")),
        }
    }

    let output = match child.wait_with_output() {
        Ok(output) => output,
        Err(error) => {
            return ToolResult::error(format!("Failed to collect command output: {error}"));
        }
    };

    render_command_output(
        command,
        output.status.code(),
        timed_out,
        &output.stdout,
        &output.stderr,
    )
}

fn start_background_command(command: &str, context: &ToolContext) -> ToolResult {
    match shell_command(command)
        .current_dir(&context.cwd)
        .env_remove("TIDE_API_KEY")
        .env_remove("DEEPSEEK_API_KEY")
        .env_remove("ZERO_CLI_API_KEY")
        .env_remove("ZERO_API_KEY")
        .env_remove("ANTHROPIC_API_KEY")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(_child) => ToolResult::text(format!(
            "Command started in background: `{}`",
            truncate_chars(command, 100)
        )),
        Err(error) => ToolResult::error(format!("Failed to execute command: {error}")),
    }
}

fn shell_command(command: &str) -> Command {
    #[cfg(windows)]
    {
        let mut process = Command::new("cmd");
        process.args(["/C", command]);
        process
    }
    #[cfg(not(windows))]
    {
        let mut process = Command::new("sh");
        process.args(["-c", command]);
        process
    }
}

fn render_command_output(
    _command: &str,
    exit_code: Option<i32>,
    timed_out: bool,
    stdout: &[u8],
    stderr: &[u8],
) -> ToolResult {
    let stdout = String::from_utf8_lossy(stdout);
    let stderr = String::from_utf8_lossy(stderr);
    let mut output = String::new();
    if timed_out {
        output.push_str("[Timed out after configured timeout - process killed]\n");
    }
    if !stdout.is_empty() {
        output.push_str(&stdout);
    }
    if !stderr.is_empty() {
        if !output.is_empty() {
            output.push('\n');
        }
        output.push_str("[stderr]\n");
        output.push_str(&stderr);
    }

    let failed = timed_out || exit_code.unwrap_or(1) != 0;
    if output.is_empty() && failed {
        output = format!(
            "Command exited with code {} (no output)",
            exit_code.unwrap_or(-1)
        );
    }

    let output = truncate_command_output(&output, 30_000, 500);
    if failed {
        ToolResult::error(output)
    } else {
        ToolResult::text(output)
    }
}

fn truncate_command_output(output: &str, max_chars: usize, max_lines: usize) -> String {
    let mut truncated = output
        .lines()
        .take(max_lines)
        .collect::<Vec<_>>()
        .join("\n");
    if output.ends_with('\n') && output.lines().count() <= max_lines {
        truncated.push('\n');
    }
    if truncated.chars().count() > max_chars {
        truncated = truncate_chars(&truncated, max_chars);
        truncated.push_str("\n[Output truncated]");
    } else if output.lines().count() > max_lines {
        truncated.push_str("\n[Output truncated]");
    }
    truncated
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_owned();
    }
    value.chars().take(max_chars).collect()
}

fn estimate_tokens(value: &str) -> usize {
    value.len().div_ceil(4)
}

fn normalize_line_endings(content: &str) -> String {
    content.replace("\r\n", "\n").replace('\r', "\n")
}

fn format_byte_count(bytes: usize) -> String {
    if bytes == 1 {
        String::from("1 byte")
    } else {
        format!("{bytes} bytes")
    }
}

fn apply_exact_edit(
    content: &str,
    old_string: &str,
    new_string: &str,
    replace_all: bool,
) -> Result<(String, usize), String> {
    let matches = content.matches(old_string).count();
    if matches == 0 {
        let normalized_content = normalize_quotes(content);
        let normalized_old = normalize_quotes(old_string);
        let normalized_matches = normalized_content.matches(&normalized_old).count();
        if normalized_matches == 0 {
            return Err(String::from(
                "old_string not found in file. The file may have been modified. Please re-read the file.",
            ));
        }
        if normalized_matches > 1 && !replace_all {
            return Err(format!(
                "old_string matches {normalized_matches} locations. Use replace_all=true or provide a more specific string."
            ));
        }

        let normalized_new = normalize_quotes(new_string);
        let edited = if replace_all {
            normalized_content.replace(&normalized_old, &normalized_new)
        } else {
            normalized_content.replacen(&normalized_old, &normalized_new, 1)
        };
        return Ok((edited, normalized_matches));
    }

    if matches > 1 && !replace_all {
        return Err(format!(
            "old_string matches {matches} locations. Use replace_all=true or provide a more specific string."
        ));
    }

    let edited = if replace_all {
        content.replace(old_string, new_string)
    } else {
        content.replacen(old_string, new_string, 1)
    };
    Ok((edited, matches))
}

fn normalize_quotes(content: &str) -> String {
    content
        .replace(['\u{201c}', '\u{201d}'], "\"")
        .replace(['\u{2018}', '\u{2019}'], "'")
}

fn grep_path(
    base: &Path,
    path: &Path,
    regex: &regex::Regex,
    output_mode: &str,
    glob: Option<&GlobMatcher>,
    head_limit: usize,
    offset: usize,
) -> ToolResult {
    if path.is_file() {
        return grep_file(
            path.parent().unwrap_or(base),
            path,
            regex,
            output_mode,
            head_limit,
            offset,
        );
    }
    if !path.is_dir() {
        return ToolResult::error(format!("Path does not exist: {}", path.display()));
    }

    let collect_limit = collect_limit(head_limit, offset);
    let mut matching_files = BTreeMap::<String, usize>::new();
    let mut content_matches = Vec::new();
    collect_files(path, path, &mut |relative, full_path| {
        if let Some(glob) = glob
            && !glob.matches(relative)
        {
            return true;
        }
        match grep_file_matches(path, full_path, regex) {
            Ok(matches) if !matches.is_empty() => {
                let relative = relative.to_string_lossy().replace('\\', "/");
                matching_files.insert(relative.clone(), matches.len());
                for (line_number, line) in matches {
                    content_matches.push(format!("{relative}:{line_number}:{line}"));
                }
            }
            Ok(_) => {}
            Err(_) => {}
        }
        matching_files.len().max(content_matches.len()) < collect_limit
    });

    render_grep_output(
        output_mode,
        matching_files,
        content_matches,
        head_limit,
        offset,
        base,
    )
}

fn grep_file(
    base: &Path,
    path: &Path,
    regex: &regex::Regex,
    output_mode: &str,
    head_limit: usize,
    offset: usize,
) -> ToolResult {
    match grep_file_matches(base, path, regex) {
        Ok(matches) => {
            let relative = path
                .strip_prefix(base)
                .unwrap_or(path)
                .to_string_lossy()
                .replace('\\', "/");
            let mut matching_files = BTreeMap::new();
            let mut content_matches = Vec::new();
            if !matches.is_empty() {
                matching_files.insert(relative.clone(), matches.len());
                for (line_number, line) in matches {
                    content_matches.push(format!("{relative}:{line_number}:{line}"));
                }
            }
            render_grep_output(
                output_mode,
                matching_files,
                content_matches,
                head_limit,
                offset,
                base,
            )
        }
        Err(error) => ToolResult::error(format!("Failed to search {}: {error}", path.display())),
    }
}

fn grep_file_matches(
    base: &Path,
    path: &Path,
    regex: &regex::Regex,
) -> io::Result<Vec<(usize, String)>> {
    let file = fs::File::open(path)?;
    let mut matches = Vec::new();
    for (index, line) in io::BufReader::new(file).lines().enumerate() {
        let line = line?;
        if regex.is_match(&line) {
            matches.push((index + 1, line));
        }
    }
    let _ = base;
    Ok(matches)
}

fn render_grep_output(
    output_mode: &str,
    matching_files: BTreeMap<String, usize>,
    content_matches: Vec<String>,
    head_limit: usize,
    offset: usize,
    _base: &Path,
) -> ToolResult {
    match output_mode {
        "content" => {
            let (lines, truncated) = page_lines(content_matches, head_limit, offset);
            if lines.is_empty() {
                ToolResult::text("No matches found")
            } else if truncated {
                ToolResult::text(format!(
                    "{}\n\n[Results truncated: use offset to paginate]",
                    lines.join("\n")
                ))
            } else {
                ToolResult::text(lines.join("\n"))
            }
        }
        "count" => {
            let lines = matching_files
                .into_iter()
                .map(|(path, count)| format!("{path}:{count}"))
                .collect::<Vec<_>>();
            let (lines, truncated) = page_lines(lines, head_limit, offset);
            if truncated {
                ToolResult::text(format!(
                    "{}\n\n[Results truncated: use offset to paginate]",
                    lines.join("\n")
                ))
            } else {
                ToolResult::text(lines.join("\n"))
            }
        }
        _ => {
            let lines = matching_files.into_keys().collect::<Vec<_>>();
            let (paged, truncated) = page_lines(lines, head_limit, offset);
            let suffix = if truncated {
                "\n\n[Results truncated: use offset to paginate]"
            } else {
                ""
            };
            ToolResult::text(
                format!(
                    "Found {} file{}\n\n{}",
                    paged.len(),
                    if paged.len() == 1 { "" } else { "s" },
                    paged.join("\n")
                ) + suffix,
            )
        }
    }
}

fn collect_limit(head_limit: usize, offset: usize) -> usize {
    if head_limit == 0 {
        usize::MAX
    } else {
        offset.saturating_add(head_limit)
    }
}

fn page_lines(lines: Vec<String>, head_limit: usize, offset: usize) -> (Vec<String>, bool) {
    let start = offset.min(lines.len());
    let tail_len = lines.len().saturating_sub(start);
    if head_limit == 0 {
        (lines.into_iter().skip(start).collect(), false)
    } else {
        (
            lines.into_iter().skip(start).take(head_limit).collect(),
            tail_len > head_limit,
        )
    }
}

fn collect_files(base: &Path, path: &Path, visit: &mut impl FnMut(&Path, &Path) -> bool) -> bool {
    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(_) => return true,
    };

    for entry in entries.flatten() {
        let full_path = entry.path();
        let relative = full_path.strip_prefix(base).unwrap_or(&full_path);
        if is_vcs_path(relative) {
            continue;
        }
        if full_path.is_dir() {
            if !collect_files(base, &full_path, visit) {
                return false;
            }
        } else if full_path.is_file() && !visit(relative, &full_path) {
            return false;
        }
    }
    true
}

fn is_vcs_path(path: &Path) -> bool {
    path.components().any(|component| {
        matches!(
            component.as_os_str().to_str(),
            Some(".git" | ".svn" | ".hg" | ".bzr" | ".jj" | ".sl")
        )
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GlobMatcher {
    components: Vec<String>,
}

impl GlobMatcher {
    fn new(pattern: &str) -> Self {
        Self {
            components: pattern.split('/').map(ToOwned::to_owned).collect(),
        }
    }

    fn matches(&self, path: &Path) -> bool {
        let parts = path
            .components()
            .filter_map(|component| component.as_os_str().to_str())
            .collect::<Vec<_>>();
        match_glob_components(&parts, &self.components)
    }
}

fn match_glob_components(path: &[&str], pattern: &[String]) -> bool {
    if pattern.is_empty() {
        return path.is_empty();
    }
    if pattern[0] == "**" {
        return match_glob_components(path, &pattern[1..])
            || (!path.is_empty() && match_glob_components(&path[1..], pattern));
    }
    if path.is_empty() {
        return false;
    }
    matches_glob_segment(path[0], &pattern[0]) && match_glob_components(&path[1..], &pattern[1..])
}

fn matches_glob_segment(name: &str, pattern: &str) -> bool {
    let name = name.as_bytes();
    let pattern = pattern.as_bytes();
    let (mut ni, mut pi) = (0usize, 0usize);
    let (mut star, mut match_i) = (None, 0usize);
    while ni < name.len() {
        if pi < pattern.len() && (pattern[pi] == b'?' || pattern[pi] == name[ni]) {
            ni += 1;
            pi += 1;
        } else if pi < pattern.len() && pattern[pi] == b'*' {
            star = Some(pi);
            match_i = ni;
            pi += 1;
        } else if let Some(star_i) = star {
            pi = star_i + 1;
            match_i += 1;
            ni = match_i;
        } else {
            return false;
        }
    }
    while pi < pattern.len() && pattern[pi] == b'*' {
        pi += 1;
    }
    pi == pattern.len()
}

fn expand_home(path: &str) -> String {
    if path == "~" {
        return home_dir()
            .map(|home| home.display().to_string())
            .unwrap_or_else(|| path.to_owned());
    }
    if let Some(rest) = path.strip_prefix("~/") {
        return home_dir()
            .map(|home| home.join(rest).display().to_string())
            .unwrap_or_else(|| path.to_owned());
    }
    path.to_owned()
}

fn home_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        std::env::var_os("USERPROFILE").map(PathBuf::from)
    }
    #[cfg(not(windows))]
    {
        std::env::var_os("HOME").map(PathBuf::from)
    }
}

fn normalize_path(path: PathBuf) -> PathBuf {
    path.components().collect()
}
