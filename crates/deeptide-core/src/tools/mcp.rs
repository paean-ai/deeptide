//! MCP client tools: invoke an MCP server method and list/read its
//! resources and prompts (McpTool + the List/Read/Get* family).
//!
//! The MCP wire/client helpers stay in the parent module and are reached
//! via `use super::*`.

use super::*;

#[derive(Debug, Default, Clone, Copy)]
pub struct McpTool;

impl Tool for McpTool {
    fn name(&self) -> &'static str {
        "MCP"
    }

    fn description(&self) -> &'static str {
        "Forward a JSON-RPC method to a configured MCP server."
    }

    fn is_read_only(&self) -> bool {
        false
    }

    fn call(&self, input: serde_json::Value, context: &ToolContext) -> ToolResult {
        let Some(server) = input.get("server").and_then(serde_json::Value::as_str) else {
            return ToolResult::error("Missing server or method");
        };
        let Some(method) = input.get("method").and_then(serde_json::Value::as_str) else {
            return ToolResult::error("Missing server or method");
        };
        if server.trim().is_empty() || method.trim().is_empty() {
            return ToolResult::error("Missing server or method");
        }

        let params = input
            .get("params")
            .cloned()
            .unwrap_or_else(|| serde_json::json!({}));
        match call_configured_mcp_server(&context.cwd, server, method, params) {
            Ok(result) => ToolResult::text(format_json_value(&result)),
            Err(error) => ToolResult::error(error),
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct ListMcpResourcesTool;

impl Tool for ListMcpResourcesTool {
    fn name(&self) -> &'static str {
        "ListMcpResources"
    }

    fn description(&self) -> &'static str {
        "List resources exposed by configured MCP servers."
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn call(&self, input: serde_json::Value, context: &ToolContext) -> ToolResult {
        let target = input
            .get("server")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|server| !server.is_empty());
        render_mcp_list("resources", target, &context.cwd)
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct ReadMcpResourceTool;

impl Tool for ReadMcpResourceTool {
    fn name(&self) -> &'static str {
        "ReadMcpResource"
    }

    fn description(&self) -> &'static str {
        "Read a resource from a configured MCP server by URI."
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn call(&self, input: serde_json::Value, context: &ToolContext) -> ToolResult {
        let Some(server) = input.get("server").and_then(serde_json::Value::as_str) else {
            return ToolResult::error("Missing server or uri");
        };
        let Some(uri) = input.get("uri").and_then(serde_json::Value::as_str) else {
            return ToolResult::error("Missing server or uri");
        };
        if server.trim().is_empty() || uri.trim().is_empty() {
            return ToolResult::error("Missing server or uri");
        }
        match call_configured_mcp_server(
            &context.cwd,
            server,
            "resources/read",
            serde_json::json!({"uri": uri}),
        ) {
            Ok(result) => ToolResult::text(format_json_value(&result)),
            Err(error) => ToolResult::error(error),
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct ListMcpPromptsTool;

impl Tool for ListMcpPromptsTool {
    fn name(&self) -> &'static str {
        "ListMcpPrompts"
    }

    fn description(&self) -> &'static str {
        "List prompt templates exposed by configured MCP servers."
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn call(&self, input: serde_json::Value, context: &ToolContext) -> ToolResult {
        let target = input
            .get("server")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|server| !server.is_empty());
        render_mcp_list("prompts", target, &context.cwd)
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct GetMcpPromptTool;

impl Tool for GetMcpPromptTool {
    fn name(&self) -> &'static str {
        "GetMcpPrompt"
    }

    fn description(&self) -> &'static str {
        "Fetch a prompt template from a configured MCP server by name."
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn call(&self, input: serde_json::Value, context: &ToolContext) -> ToolResult {
        let Some(server) = input.get("server").and_then(serde_json::Value::as_str) else {
            return ToolResult::error("Missing server or name");
        };
        let Some(name) = input.get("name").and_then(serde_json::Value::as_str) else {
            return ToolResult::error("Missing server or name");
        };
        if server.trim().is_empty() || name.trim().is_empty() {
            return ToolResult::error("Missing server or name");
        }
        let mut params = serde_json::json!({"name": name});
        if let Some(arguments) = input.get("arguments")
            && arguments.is_object()
            && let Some(object) = params.as_object_mut()
        {
            object.insert("arguments".to_owned(), arguments.clone());
        }
        match call_configured_mcp_server(&context.cwd, server, "prompts/get", params) {
            Ok(result) => ToolResult::text(format_json_value(&result)),
            Err(error) => ToolResult::error(error),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct McpServerConfig {
    name: String,
    source: PathBuf,
    command: Option<String>,
    args: Vec<String>,
    env: BTreeMap<String, String>,
    framing: McpFraming,
    url: Option<String>,
    /// Extra HTTP headers sent with every request to an HTTP/SSE MCP server
    /// (e.g. `Authorization`). Ignored for stdio servers.
    headers: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum McpFraming {
    ContentLength,
    NewlineJson,
}

fn render_mcp_list(kind: &str, target: Option<&str>, cwd: &Path) -> ToolResult {
    let servers = match discover_mcp_servers(cwd) {
        Ok(servers) => servers,
        Err(error) => return ToolResult::error(error),
    };
    let selected = servers
        .iter()
        .filter(|server| target.is_none_or(|target| server.name == target))
        .collect::<Vec<_>>();

    if selected.is_empty() {
        return if let Some(target) = target {
            ToolResult::error(format!("MCP server not configured: {target}"))
        } else {
            ToolResult::error(
                "No MCP servers configured. Add servers under mcp_servers in .deeptide/settings.json or mcpServers in .mcp.json.",
            )
        };
    }

    let mut lines = Vec::new();
    for server in selected {
        lines.push(format!("[{}]", server.name));
        lines.push(format!("  source: {}", server.source.display()));
        if let Some(command) = &server.command {
            lines.push(format!("  command: {command}"));
        } else if let Some(url) = &server.url {
            lines.push(format!("  url: {url}"));
        } else {
            lines.push(String::from("  transport: configured"));
            lines.push(format!(
                "  {kind}: unavailable because no command or url is configured"
            ));
            continue;
        }
        let method = match kind {
            "resources" => "resources/list",
            "prompts" => "prompts/list",
            _ => unreachable!("MCP list kind should be known"),
        };
        match call_mcp_server(server, method, serde_json::json!({})) {
            Ok(result) => lines.extend(render_mcp_collection(kind, &result)),
            Err(error) => lines.push(format!("  error: {error}")),
        }
    }

    ToolResult::text(lines.join("\n"))
}

fn call_configured_mcp_server(
    cwd: &Path,
    server: &str,
    method: &str,
    params: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let servers = discover_mcp_servers(cwd)?;
    let Some(config) = servers.iter().find(|configured| configured.name == server) else {
        return Err(format!("MCP server not configured: {server}"));
    };
    call_mcp_server(config, method, params)
}

pub(crate) fn parse_dynamic_mcp_tool_name(name: &str) -> Option<(&str, &str)> {
    let rest = name.strip_prefix("mcp__")?;
    let (server, tool) = rest.split_once("__")?;
    (!server.is_empty() && !tool.is_empty()).then_some((server, tool))
}

pub(crate) fn call_dynamic_mcp_tool(
    server: &str,
    tool_name: &str,
    input: serde_json::Value,
    context: &ToolContext,
) -> ToolResult {
    let arguments = input
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| input.clone());
    let params = serde_json::json!({
        "name": tool_name,
        "arguments": arguments
    });
    match call_configured_mcp_server(&context.cwd, server, "tools/call", params) {
        Ok(result) => ToolResult::text(format_json_value(&result)),
        Err(error) => ToolResult::error(error),
    }
}

fn discover_mcp_servers(cwd: &Path) -> Result<Vec<McpServerConfig>, String> {
    let mut servers = Vec::new();
    for path in mcp_config_paths(cwd) {
        if !path.exists() {
            continue;
        }
        let raw = fs::read_to_string(&path)
            .map_err(|error| format!("Failed to read MCP settings {}: {error}", path.display()))?;
        let json: serde_json::Value = serde_json::from_str(&raw)
            .map_err(|error| format!("Failed to parse MCP settings {}: {error}", path.display()))?;
        extract_mcp_servers_from_json(&json, &path, &mut servers);
    }
    servers.sort_by(|left, right| left.name.cmp(&right.name));
    servers.dedup_by(|left, right| left.name == right.name);
    Ok(servers)
}

fn mcp_config_paths(cwd: &Path) -> Vec<PathBuf> {
    let mut paths = vec![
        cwd.join(".mcp.json"),
        cwd.join(".deeptide").join("mcp.json"),
        cwd.join(".deeptide").join("settings.json"),
        cwd.join(".deeptide").join("config.json"),
    ];
    if let Some(config_dir) = tide_config_dir() {
        let project = project_settings_slug(cwd);
        paths.push(config_dir.join("settings.json"));
        paths.push(
            config_dir
                .join("projects")
                .join(&project)
                .join("settings.json"),
        );
        paths.push(
            config_dir
                .join("projects")
                .join(project)
                .join("settings.local.json"),
        );
    }
    if let Some(home) = home_dir() {
        paths.push(home.join(".deeptide").join("settings.json"));
        paths.push(home.join(".deeptide").join("config.json"));
    }
    paths
}

fn extract_mcp_servers_from_json(
    json: &serde_json::Value,
    source: &Path,
    servers: &mut Vec<McpServerConfig>,
) {
    for key in ["mcp_servers", "mcpServers", "servers"] {
        if let Some(object) = json.get(key).and_then(serde_json::Value::as_object) {
            collect_mcp_servers(object, source, servers);
        }
    }
    if let Some(settings) = json.get("settings") {
        for key in ["mcp_servers", "mcpServers"] {
            if let Some(object) = settings.get(key).and_then(serde_json::Value::as_object) {
                collect_mcp_servers(object, source, servers);
            }
        }
    }
}

fn collect_mcp_servers(
    object: &serde_json::Map<String, serde_json::Value>,
    source: &Path,
    servers: &mut Vec<McpServerConfig>,
) {
    for (name, value) in object {
        let Some(config) = value.as_object() else {
            continue;
        };
        if config.get("disabled").and_then(serde_json::Value::as_bool) == Some(true) {
            continue;
        }
        let command = config
            .get("command")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned);
        let url = config
            .get("url")
            .or_else(|| config.get("endpoint"))
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned);
        let args = config
            .get("args")
            .and_then(serde_json::Value::as_array)
            .map(|values| {
                values
                    .iter()
                    .filter_map(serde_json::Value::as_str)
                    .map(ToOwned::to_owned)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let env = config
            .get("env")
            .and_then(serde_json::Value::as_object)
            .map(|values| {
                values
                    .iter()
                    .filter_map(|(key, value)| {
                        value
                            .as_str()
                            .map(|value| (key.to_owned(), value.to_owned()))
                    })
                    .collect::<BTreeMap<_, _>>()
            })
            .unwrap_or_default();
        let framing = config
            .get("framing")
            .or_else(|| config.get("message_framing"))
            .and_then(serde_json::Value::as_str)
            .map(parse_mcp_framing)
            .unwrap_or(McpFraming::ContentLength);
        let headers = config
            .get("headers")
            .and_then(serde_json::Value::as_object)
            .map(|values| {
                values
                    .iter()
                    .filter_map(|(key, value)| {
                        value
                            .as_str()
                            .map(|value| (key.to_owned(), value.to_owned()))
                    })
                    .collect::<BTreeMap<_, _>>()
            })
            .unwrap_or_default();
        servers.push(McpServerConfig {
            name: name.to_owned(),
            source: source.to_path_buf(),
            command,
            args,
            env,
            framing,
            url,
            headers,
        });
    }
}

fn call_mcp_server(
    config: &McpServerConfig,
    method: &str,
    params: serde_json::Value,
) -> Result<serde_json::Value, String> {
    if config.command.is_some() {
        call_mcp_stdio_server(config, method, params)
    } else if config.url.is_some() {
        call_mcp_http_server(config, method, params)
    } else {
        Err(format!(
            "MCP server {} has no command or url configured.",
            config.name
        ))
    }
}

fn call_mcp_stdio_server(
    config: &McpServerConfig,
    method: &str,
    params: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let command = config
        .command
        .as_ref()
        .expect("stdio MCP server must have a command");

    let mut child = Command::new(command)
        .args(&config.args)
        .envs(&config.env)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("Failed to start MCP server {}: {error}", config.name))?;

    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| format!("Failed to open stdin for MCP server {}", config.name))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| format!("Failed to open stdout for MCP server {}", config.name))?;
    let stderr = child.stderr.take();
    let (tx, rx) = mpsc::channel();
    let framing = config.framing;
    thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        loop {
            match read_mcp_frame(&mut reader, framing) {
                Ok(message) => {
                    if tx.send(Ok(message)).is_err() {
                        break;
                    }
                }
                Err(error) => {
                    let _ = tx.send(Err(error.to_string()));
                    break;
                }
            }
        }
    });

    let initialize = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {"name": "deeptide-rs", "version": env!("CARGO_PKG_VERSION")}
        }
    });
    let initialized = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "notifications/initialized"
    });
    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": method,
        "params": params
    });
    write_mcp_frame(&mut stdin, &initialize, config.framing)
        .and_then(|_| write_mcp_frame(&mut stdin, &initialized, config.framing))
        .and_then(|_| write_mcp_frame(&mut stdin, &request, config.framing))
        .map_err(|error| format!("Failed to write MCP request to {}: {error}", config.name))?;
    drop(stdin);

    let deadline = Instant::now() + Duration::from_secs(10);
    let mut response = None;
    while Instant::now() < deadline {
        let timeout = deadline.saturating_duration_since(Instant::now());
        match rx.recv_timeout(timeout.min(Duration::from_millis(250))) {
            Ok(Ok(message)) => {
                if message.get("id").and_then(serde_json::Value::as_i64) == Some(2) {
                    response = Some(message);
                    break;
                }
            }
            Ok(Err(error)) => {
                return Err(format!("MCP server {} closed stdout: {error}", config.name));
            }
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Err(format!(
                    "MCP server {} stopped before replying",
                    config.name
                ));
            }
        }
    }

    let _ = child.kill();
    let _ = child.wait();

    let Some(response) = response else {
        let stderr_text = read_child_stderr(stderr);
        return Err(format!(
            "MCP server {} timed out waiting for {method}.{}",
            config.name, stderr_text
        ));
    };
    if let Some(error) = response.get("error") {
        return Err(format!(
            "MCP call failed on {}: {}",
            config.name,
            format_json_value(error)
        ));
    }
    Ok(response
        .get("result")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({})))
}

/// Call an MCP server over the Streamable HTTP transport.
///
/// Mirrors the stdio handshake: POST `initialize`, then the
/// `notifications/initialized` notification, then the requested method. The
/// server may answer each POST with either a single `application/json`
/// JSON-RPC object or a `text/event-stream` (SSE) sequence; both are handled.
/// A `Mcp-Session-Id` header returned by `initialize` is echoed on subsequent
/// requests, as required by the MCP spec.
fn call_mcp_http_server(
    config: &McpServerConfig,
    method: &str,
    params: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let url = config
        .url
        .as_ref()
        .expect("HTTP MCP server must have a url");

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|error| {
            format!(
                "Failed to build HTTP client for MCP server {}: {error}",
                config.name
            )
        })?;

    let initialize = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {"name": "deeptide-rs", "version": env!("CARGO_PKG_VERSION")}
        }
    });
    let init_response = post_mcp_http(&client, url, &config.headers, None, &initialize, config)?;
    let session_id = init_response
        .headers()
        .get("mcp-session-id")
        .and_then(|value| value.to_str().ok())
        .map(ToOwned::to_owned);
    // Surface initialization errors early; the result body is otherwise unused.
    parse_mcp_http_message(init_response, 1, config)?;

    let initialized = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "notifications/initialized"
    });
    post_mcp_http(
        &client,
        url,
        &config.headers,
        session_id.as_deref(),
        &initialized,
        config,
    )?;

    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": method,
        "params": params
    });
    let response = post_mcp_http(
        &client,
        url,
        &config.headers,
        session_id.as_deref(),
        &request,
        config,
    )?;
    let message = parse_mcp_http_message(response, 2, config)?;

    if let Some(error) = message.get("error") {
        return Err(format!(
            "MCP call failed on {}: {}",
            config.name,
            format_json_value(error)
        ));
    }
    Ok(message
        .get("result")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({})))
}

fn post_mcp_http(
    client: &reqwest::blocking::Client,
    url: &str,
    headers: &BTreeMap<String, String>,
    session_id: Option<&str>,
    body: &serde_json::Value,
    config: &McpServerConfig,
) -> Result<reqwest::blocking::Response, String> {
    let mut request = client
        .post(url)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .header(
            reqwest::header::ACCEPT,
            "application/json, text/event-stream",
        )
        .json(body);
    if let Some(session_id) = session_id {
        request = request.header("Mcp-Session-Id", session_id);
    }
    for (key, value) in headers {
        request = request.header(key, value);
    }
    request
        .send()
        .map_err(|error| format!("MCP HTTP request to {} failed: {error}", config.name))
}

/// Read a JSON-RPC message from an HTTP response, accepting both a single JSON
/// object and an SSE stream of `data:` events.
fn parse_mcp_http_message(
    response: reqwest::blocking::Response,
    expected_id: i64,
    config: &McpServerConfig,
) -> Result<serde_json::Value, String> {
    let status = response.status();
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .to_ascii_lowercase();
    let body = response
        .text()
        .map_err(|error| format!("Failed to read MCP response from {}: {error}", config.name))?;

    if content_type.contains("text/event-stream") {
        return extract_jsonrpc_from_sse(&body, expected_id).ok_or_else(|| {
            format!(
                "MCP server {} returned no JSON-RPC message for id {expected_id}",
                config.name
            )
        });
    }

    if body.trim().is_empty() {
        return Err(format!(
            "MCP server {} returned an empty response (HTTP {})",
            config.name,
            status.as_u16()
        ));
    }

    let value: serde_json::Value = serde_json::from_str(&body)
        .map_err(|error| format!("MCP server {} returned invalid JSON: {error}", config.name))?;
    Ok(select_jsonrpc_message(value, expected_id))
}

/// Extract the JSON-RPC message matching `expected_id` from an SSE body.
///
/// Falls back to the first event carrying a `result` or `error` when no id
/// matches, which tolerates servers that omit the id on their final event.
fn extract_jsonrpc_from_sse(body: &str, expected_id: i64) -> Option<serde_json::Value> {
    let mut data = String::new();
    let mut fallback = None;
    let consider = |data: &str, fallback: &mut Option<serde_json::Value>| {
        if data.is_empty() {
            return None;
        }
        let value = serde_json::from_str::<serde_json::Value>(data).ok()?;
        if value.get("id").and_then(serde_json::Value::as_i64) == Some(expected_id) {
            return Some(value);
        }
        if fallback.is_none() && (value.get("result").is_some() || value.get("error").is_some()) {
            *fallback = Some(value);
        }
        None
    };

    for line in body.lines() {
        if let Some(rest) = line.strip_prefix("data:") {
            let rest = rest.strip_prefix(' ').unwrap_or(rest);
            if !data.is_empty() {
                data.push('\n');
            }
            data.push_str(rest);
        } else if line.trim().is_empty() {
            if let Some(found) = consider(&data, &mut fallback) {
                return Some(found);
            }
            data.clear();
        }
    }
    if let Some(found) = consider(&data, &mut fallback) {
        return Some(found);
    }
    fallback
}

/// Pick the JSON-RPC message matching `expected_id` from a value that may be a
/// single object or a batch array.
fn select_jsonrpc_message(value: serde_json::Value, expected_id: i64) -> serde_json::Value {
    let serde_json::Value::Array(items) = value else {
        return value;
    };
    let mut fallback = None;
    for item in items {
        if item.get("id").and_then(serde_json::Value::as_i64) == Some(expected_id) {
            return item;
        }
        if fallback.is_none() && (item.get("result").is_some() || item.get("error").is_some()) {
            fallback = Some(item);
        }
    }
    fallback.unwrap_or_else(|| serde_json::json!({}))
}

fn read_child_stderr(stderr: Option<std::process::ChildStderr>) -> String {
    let Some(mut stderr) = stderr else {
        return String::new();
    };
    let mut text = String::new();
    let _ = stderr.read_to_string(&mut text);
    let text = text.trim();
    if text.is_empty() {
        String::new()
    } else {
        format!(" stderr: {text}")
    }
}

fn parse_mcp_framing(raw: &str) -> McpFraming {
    match raw.trim().to_ascii_lowercase().as_str() {
        "newline" | "json-lines" | "jsonl" | "ndjson" => McpFraming::NewlineJson,
        _ => McpFraming::ContentLength,
    }
}

fn write_mcp_frame(
    writer: &mut impl IoWrite,
    message: &serde_json::Value,
    framing: McpFraming,
) -> io::Result<()> {
    let body = serde_json::to_vec(message).map_err(io::Error::other)?;
    match framing {
        McpFraming::ContentLength => {
            write!(writer, "Content-Length: {}\r\n\r\n", body.len())?;
            writer.write_all(&body)?;
        }
        McpFraming::NewlineJson => {
            writer.write_all(&body)?;
            writer.write_all(b"\n")?;
        }
    }
    writer.flush()
}

fn read_mcp_frame(reader: &mut impl BufRead, framing: McpFraming) -> io::Result<serde_json::Value> {
    match framing {
        McpFraming::ContentLength => read_mcp_content_length_frame(reader),
        McpFraming::NewlineJson => read_mcp_newline_json_frame(reader),
    }
}

fn read_mcp_newline_json_frame(reader: &mut impl BufRead) -> io::Result<serde_json::Value> {
    loop {
        let mut line = String::new();
        let read = reader.read_line(&mut line)?;
        if read == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "unexpected EOF while reading MCP JSON line",
            ));
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        return serde_json::from_str(trimmed).map_err(io::Error::other);
    }
}

fn read_mcp_content_length_frame(reader: &mut impl BufRead) -> io::Result<serde_json::Value> {
    let mut content_length = None;
    loop {
        let mut line = String::new();
        let read = reader.read_line(&mut line)?;
        if read == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "unexpected EOF while reading MCP headers",
            ));
        }
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            break;
        }
        if let Some(value) = trimmed.strip_prefix("Content-Length:") {
            content_length = Some(value.trim().parse::<usize>().map_err(|error| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("invalid MCP content length: {error}"),
                )
            })?);
        }
    }

    let Some(length) = content_length else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "missing MCP Content-Length header",
        ));
    };
    let mut body = vec![0u8; length];
    reader.read_exact(&mut body)?;
    serde_json::from_slice(&body).map_err(io::Error::other)
}

fn render_mcp_collection(kind: &str, result: &serde_json::Value) -> Vec<String> {
    let key = match kind {
        "resources" => "resources",
        "prompts" => "prompts",
        _ => return vec![format!("  {kind}: {}", format_json_value(result))],
    };
    let Some(items) = result.get(key).and_then(serde_json::Value::as_array) else {
        return vec![format!("  {kind}: (none)")];
    };
    if items.is_empty() {
        return vec![format!("  {kind}: (none)")];
    }

    let mut lines = vec![format!("  {kind}:")];
    for item in items {
        if let Some(object) = item.as_object() {
            let primary = if kind == "prompts" {
                object.get("name")
            } else {
                object.get("uri").or_else(|| object.get("name"))
            }
            .and_then(serde_json::Value::as_str)
            .unwrap_or("?");
            let secondary = if kind == "prompts" {
                object.get("description")
            } else {
                object.get("name").or_else(|| object.get("description"))
            }
            .and_then(serde_json::Value::as_str)
            .unwrap_or("");
            if secondary.is_empty() || secondary == primary {
                lines.push(format!("    - {primary}"));
            } else {
                lines.push(format!("    - {primary} - {secondary}"));
            }
        }
    }
    lines
}

pub(crate) fn render_dynamic_mcp_tool_search_entries(cwd: &Path) -> Vec<String> {
    let Ok(servers) = discover_mcp_servers(cwd) else {
        return Vec::new();
    };
    let mut lines = Vec::new();
    for server in servers {
        let Some(command) = &server.command else {
            continue;
        };
        match call_mcp_server(&server, "tools/list", serde_json::json!({})) {
            Ok(result) => {
                let Some(tools) = result.get("tools").and_then(serde_json::Value::as_array) else {
                    continue;
                };
                if !tools.is_empty() && !lines.iter().any(|line| line == "[MCP tools]") {
                    lines.push(String::from("[MCP tools]"));
                }
                for tool in tools {
                    let Some(object) = tool.as_object() else {
                        continue;
                    };
                    let Some(name) = object.get("name").and_then(serde_json::Value::as_str) else {
                        continue;
                    };
                    let description = object
                        .get("description")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("MCP server tool");
                    lines.push(format!(
                        "- mcp__{}__{} [external] - {}",
                        server.name,
                        name,
                        truncate_chars(description, 160)
                    ));
                }
            }
            Err(error) => {
                lines.push(format!(
                    "- MCP server {} ({command}) - tools/list unavailable: {}",
                    server.name,
                    truncate_chars(&error, 120)
                ));
            }
        }
    }
    lines
}

fn format_json_value(value: &serde_json::Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
}
