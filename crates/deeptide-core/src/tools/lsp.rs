//! LSP tool: goto-definition, references, hover, and document symbols via a language server.
//!
//! Shared infrastructure lives in the parent module (reached via `use super::*`).

use super::*;

#[derive(Debug, Default, Clone, Copy)]
pub struct LspTool;

#[derive(Debug, Clone, PartialEq, Eq)]
struct LspLocation {
    uri: String,
    line: usize,
    character: usize,
}

impl Tool for LspTool {
    fn name(&self) -> &'static str {
        "LSP"
    }

    fn description(&self) -> &'static str {
        "Run code intelligence requests through a local Language Server Protocol server."
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn call(&self, input: serde_json::Value, context: &ToolContext) -> ToolResult {
        let valid_ops = [
            "goToDefinition",
            "findReferences",
            "hover",
            "documentSymbol",
        ];
        let Some(operation) = input.get("operation").and_then(serde_json::Value::as_str) else {
            return ToolResult::error(format!(
                "operation must be one of: {}",
                valid_ops.join(", ")
            ));
        };
        if !valid_ops.contains(&operation) {
            return ToolResult::error(format!(
                "operation must be one of: {}",
                valid_ops.join(", ")
            ));
        }
        let Some(file_path) = input.get("file_path").and_then(serde_json::Value::as_str) else {
            return ToolResult::error("file_path is required");
        };
        if file_path.trim().is_empty() {
            return ToolResult::error("file_path is required");
        }
        let Some(line) = json_usize(&input, "line") else {
            return ToolResult::error("line is required");
        };
        let character = json_usize(&input, "character").unwrap_or(1);

        let abs_path = context.resolve_path(file_path);
        if !abs_path.exists() {
            return ToolResult::error(format!("File not found: {}", abs_path.display()));
        }
        let Some(server_bin) = find_lsp_server(&abs_path) else {
            return ToolResult::error(
                "No LSP server found. For Swift files, install Xcode or Swift toolchain. Ensure the language server binary is on PATH.",
            );
        };
        let workspace = find_lsp_workspace_root(&abs_path).unwrap_or_else(|| context.cwd.clone());
        let source = match fs::read_to_string(&abs_path) {
            Ok(source) => source,
            Err(error) => {
                return ToolResult::error(format!(
                    "Failed to read {}: {error}",
                    abs_path.display()
                ));
            }
        };

        match run_lsp_request(
            operation,
            &server_bin,
            &workspace,
            &abs_path,
            line,
            character,
            source,
        ) {
            Ok(content) => ToolResult::text(content),
            Err(message) => ToolResult::error(format!("LSP error: {message}")),
        }
    }
}

fn run_lsp_request(
    operation: &str,
    server_bin: &Path,
    workspace: &Path,
    file_path: &Path,
    line: usize,
    character: usize,
    source: String,
) -> Result<String, String> {
    let mut session = LspSession::new(server_bin, workspace)?;
    let file_uri = lsp_file_uri(file_path);
    session.initialize(workspace)?;
    session.did_open(&file_uri, language_id_for_path(file_path), source)?;
    thread::sleep(Duration::from_millis(300));

    let lsp_line = line.saturating_sub(1);
    let lsp_character = character.saturating_sub(1);
    let result = match operation {
        "goToDefinition" => {
            let value = session.request(
                "textDocument/definition",
                serde_json::json!({
                    "textDocument": {"uri": file_uri},
                    "position": {"line": lsp_line, "character": lsp_character}
                }),
                Duration::from_secs(8),
            )?;
            format_lsp_locations(parse_lsp_locations(value.as_ref()), "Definition", workspace)
        }
        "findReferences" => {
            let value = session.request(
                "textDocument/references",
                serde_json::json!({
                    "textDocument": {"uri": file_uri},
                    "position": {"line": lsp_line, "character": lsp_character},
                    "context": {"includeDeclaration": true}
                }),
                Duration::from_secs(10),
            )?;
            format_lsp_locations(parse_lsp_locations(value.as_ref()), "References", workspace)
        }
        "hover" => {
            let value = session.request(
                "textDocument/hover",
                serde_json::json!({
                    "textDocument": {"uri": file_uri},
                    "position": {"line": lsp_line, "character": lsp_character}
                }),
                Duration::from_secs(8),
            )?;
            format_lsp_hover(value.as_ref()).unwrap_or_else(|| {
                format!(
                    "No hover information available at {}:{line}:{character}",
                    file_path.display()
                )
            })
        }
        "documentSymbol" => {
            let value = session.request(
                "textDocument/documentSymbol",
                serde_json::json!({"textDocument": {"uri": file_uri}}),
                Duration::from_secs(8),
            )?;
            let output = format_lsp_document_symbols(value.as_ref());
            if output.is_empty() {
                format!(
                    "No symbols found in {}",
                    file_path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or("file")
                )
            } else {
                output
            }
        }
        _ => return Err(format!("Unknown operation: {operation}")),
    };
    session.shutdown();
    Ok(result)
}

struct LspSession {
    child: Child,
    stdin: ChildStdin,
    responses: mpsc::Receiver<Result<serde_json::Value, String>>,
    next_id: u64,
}

impl LspSession {
    fn new(server_bin: &Path, workspace: &Path) -> Result<Self, String> {
        let mut child = Command::new(server_bin)
            .current_dir(workspace)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| format!("Failed to start {}: {error}", server_bin.display()))?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| String::from("LSP server stdin was unavailable"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| String::from("LSP server stdout was unavailable"))?;
        let (sender, responses) = mpsc::channel();
        thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            loop {
                match read_lsp_message(&mut reader) {
                    Ok(Some(value)) => {
                        if sender.send(Ok(value)).is_err() {
                            break;
                        }
                    }
                    Ok(None) => break,
                    Err(error) => {
                        let _ = sender.send(Err(error));
                        break;
                    }
                }
            }
        });
        Ok(Self {
            child,
            stdin,
            responses,
            next_id: 1,
        })
    }

    fn initialize(&mut self, workspace: &Path) -> Result<(), String> {
        self.request(
            "initialize",
            serde_json::json!({
                "processId": std::process::id(),
                "rootUri": lsp_file_uri(workspace),
                "capabilities": {
                    "textDocument": {
                        "definition": {"dynamicRegistration": false},
                        "references": {"dynamicRegistration": false},
                        "hover": {"dynamicRegistration": false},
                        "documentSymbol": {
                            "dynamicRegistration": false,
                            "hierarchicalDocumentSymbolSupport": false
                        }
                    }
                },
                "trace": "off"
            }),
            Duration::from_secs(10),
        )?;
        self.notification("initialized", serde_json::json!({}))
    }

    fn did_open(&mut self, uri: &str, language_id: &str, text: String) -> Result<(), String> {
        self.notification(
            "textDocument/didOpen",
            serde_json::json!({
                "textDocument": {
                    "uri": uri,
                    "languageId": language_id,
                    "version": 1,
                    "text": text
                }
            }),
        )
    }

    fn request(
        &mut self,
        method: &str,
        params: serde_json::Value,
        timeout: Duration,
    ) -> Result<Option<serde_json::Value>, String> {
        let id = self.next_id;
        self.next_id += 1;
        self.send(serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params
        }))?;

        let deadline = Instant::now() + timeout;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(String::from("LSP server timed out"));
            }
            let message = self
                .responses
                .recv_timeout(remaining)
                .map_err(|_| String::from("LSP server timed out"))??;
            if message.get("id").and_then(serde_json::Value::as_u64) != Some(id) {
                continue;
            }
            if let Some(error) = message.get("error") {
                return Err(format!("LSP server error: {error}"));
            }
            return Ok(message.get("result").cloned());
        }
    }

    fn notification(&mut self, method: &str, params: serde_json::Value) -> Result<(), String> {
        self.send(serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params
        }))
    }

    fn send(&mut self, value: serde_json::Value) -> Result<(), String> {
        let body = serde_json::to_vec(&value)
            .map_err(|error| format!("Failed to encode LSP message: {error}"))?;
        write!(self.stdin, "Content-Length: {}\r\n\r\n", body.len())
            .map_err(|error| format!("Failed to write LSP header: {error}"))?;
        self.stdin
            .write_all(&body)
            .map_err(|error| format!("Failed to write LSP body: {error}"))?;
        self.stdin
            .flush()
            .map_err(|error| format!("Failed to flush LSP body: {error}"))
    }

    fn shutdown(&mut self) {
        let _ = self.request("shutdown", serde_json::json!(null), Duration::from_secs(2));
        let _ = self.notification("exit", serde_json::json!({}));
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Drop for LspSession {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn read_lsp_message(
    reader: &mut BufReader<impl Read>,
) -> Result<Option<serde_json::Value>, String> {
    let mut content_length = None;
    loop {
        let mut line = String::new();
        let bytes = reader
            .read_line(&mut line)
            .map_err(|error| format!("Failed to read LSP header: {error}"))?;
        if bytes == 0 {
            return Ok(None);
        }
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            break;
        }
        if let Some(value) = trimmed
            .strip_prefix("Content-Length:")
            .or_else(|| trimmed.strip_prefix("content-length:"))
        {
            content_length = Some(
                value
                    .trim()
                    .parse::<usize>()
                    .map_err(|error| format!("Invalid LSP Content-Length: {error}"))?,
            );
        }
    }
    let Some(length) = content_length else {
        return Err(String::from("LSP message did not include Content-Length"));
    };
    let mut body = vec![0u8; length];
    reader
        .read_exact(&mut body)
        .map_err(|error| format!("Failed to read LSP body: {error}"))?;
    serde_json::from_slice(&body).map(Some).map_err(|error| {
        format!(
            "Failed to parse LSP JSON: {error}: {}",
            String::from_utf8_lossy(&body)
        )
    })
}

fn find_lsp_server(file_path: &Path) -> Option<PathBuf> {
    let ext = file_path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let candidates: &[&str] = match ext.as_str() {
        "swift" | "m" | "mm" | "h" | "c" | "cpp" | "cc" | "cxx" => &[
            "/Applications/Xcode.app/Contents/Developer/Toolchains/XcodeDefault.xctoolchain/usr/bin/sourcekit-lsp",
            "/usr/bin/sourcekit-lsp",
            "sourcekit-lsp",
        ],
        "py" => &["pyright-langserver", "pylsp", "python-lsp-server"],
        "js" | "ts" | "tsx" | "jsx" => &["typescript-language-server", "tsserver"],
        "rs" => &["rust-analyzer"],
        "go" => &["gopls"],
        _ => &[],
    };

    candidates.iter().find_map(|candidate| {
        let path = PathBuf::from(candidate);
        if path.is_absolute() {
            path.is_file().then_some(path)
        } else {
            which_binary(candidate)
        }
    })
}

fn find_lsp_workspace_root(file_path: &Path) -> Option<PathBuf> {
    let mut dir = file_path.parent()?;
    for _ in 0..12 {
        for marker in [
            "Package.swift",
            ".xcodeproj",
            ".xcworkspace",
            ".git",
            "Cargo.toml",
            "pyproject.toml",
            "package.json",
            "go.mod",
        ] {
            if dir.join(marker).exists() {
                return Some(dir.to_path_buf());
            }
        }
        dir = dir.parent()?;
    }
    None
}

fn lsp_file_uri(path: &Path) -> String {
    let raw = path.to_string_lossy().replace('\\', "/");
    #[cfg(windows)]
    let raw = if raw.starts_with('/') {
        raw
    } else {
        format!("/{raw}")
    };
    format!("file://{}", percent_encode_path(&raw))
}

fn path_from_lsp_uri(uri: &str) -> String {
    percent_decode_path(uri.strip_prefix("file://").unwrap_or(uri))
}

fn percent_encode_path(path: &str) -> String {
    let mut encoded = String::new();
    for byte in path.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'/' | b':' => {
                encoded.push(char::from(*byte))
            }
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}

fn percent_decode_path(path: &str) -> String {
    let bytes = path.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%'
            && index + 2 < bytes.len()
            && let (Some(high), Some(low)) =
                (hex_value(bytes[index + 1]), hex_value(bytes[index + 2]))
        {
            decoded.push((high << 4) | low);
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn language_id_for_path(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "swift" => "swift",
        "m" | "mm" => "objective-c",
        "h" | "c" => "c",
        "cpp" | "cc" | "cxx" => "cpp",
        "py" => "python",
        "ts" | "tsx" => "typescript",
        "js" | "jsx" => "javascript",
        "rs" => "rust",
        "go" => "go",
        _ => "plaintext",
    }
}

fn parse_lsp_locations(value: Option<&serde_json::Value>) -> Vec<LspLocation> {
    fn parse_one(value: &serde_json::Value) -> Option<LspLocation> {
        let (uri, range) = if let Some(target_uri) = value.get("targetUri") {
            (target_uri.as_str()?, value.get("targetSelectionRange")?)
        } else {
            (value.get("uri")?.as_str()?, value.get("range")?)
        };
        let start = range.get("start")?;
        Some(LspLocation {
            uri: uri.to_owned(),
            line: start.get("line")?.as_u64()?.try_into().ok()?,
            character: start.get("character")?.as_u64()?.try_into().ok()?,
        })
    }

    match value {
        Some(serde_json::Value::Array(items)) => items.iter().filter_map(parse_one).collect(),
        Some(serde_json::Value::Object(_)) => value.and_then(parse_one).into_iter().collect(),
        _ => Vec::new(),
    }
}

fn format_lsp_locations(locations: Vec<LspLocation>, label: &str, workspace: &Path) -> String {
    if locations.is_empty() {
        return format!("No {} found.", label.to_ascii_lowercase());
    }
    let mut lines = vec![format!("{label}s ({}):", locations.len())];
    for location in locations {
        let path = PathBuf::from(path_from_lsp_uri(&location.uri));
        let display_path = path
            .strip_prefix(workspace)
            .unwrap_or(&path)
            .display()
            .to_string();
        lines.push(format!(
            "  {}:{}:{}",
            display_path,
            location.line + 1,
            location.character + 1
        ));
    }
    lines.join("\n")
}

fn format_lsp_hover(value: Option<&serde_json::Value>) -> Option<String> {
    let contents = value?.get("contents")?;
    if let Some(text) = contents.as_str() {
        return Some(text.to_owned());
    }
    if let Some(value) = contents.get("value").and_then(serde_json::Value::as_str) {
        return Some(value.to_owned());
    }
    if let Some(items) = contents.as_array() {
        let output = items
            .iter()
            .filter_map(|item| {
                item.as_str()
                    .or_else(|| item.get("value").and_then(serde_json::Value::as_str))
            })
            .collect::<Vec<_>>()
            .join("\n");
        return (!output.is_empty()).then_some(output);
    }
    None
}

fn format_lsp_document_symbols(value: Option<&serde_json::Value>) -> String {
    let Some(serde_json::Value::Array(symbols)) = value else {
        return String::new();
    };
    symbols
        .iter()
        .filter_map(|symbol| {
            let name = symbol.get("name").and_then(serde_json::Value::as_str)?;
            let kind = symbol_kind_name(symbol.get("kind").and_then(serde_json::Value::as_u64));
            if let Some(line) = symbol
                .pointer("/location/range/start/line")
                .or_else(|| symbol.pointer("/range/start/line"))
                .and_then(serde_json::Value::as_u64)
            {
                Some(format!("{kind} {name} (line {})", line + 1))
            } else {
                Some(format!("{kind} {name}"))
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn symbol_kind_name(kind: Option<u64>) -> &'static str {
    match kind {
        Some(1) => "File",
        Some(2) => "Module",
        Some(3) => "Namespace",
        Some(4) => "Package",
        Some(5) => "Class",
        Some(6) => "Method",
        Some(7) => "Property",
        Some(8) => "Field",
        Some(9) => "Constructor",
        Some(10) => "Enum",
        Some(11) => "Interface",
        Some(12) => "Function",
        Some(13) => "Variable",
        Some(14) => "Constant",
        Some(15) => "String",
        Some(16) => "Number",
        Some(17) => "Boolean",
        Some(18) => "Array",
        Some(23) => "Struct",
        Some(25) => "EnumMember",
        _ => "Symbol",
    }
}

#[cfg(test)]
mod lsp_tests {
    use super::{
        LspLocation, format_lsp_document_symbols, format_lsp_hover, format_lsp_locations,
        lsp_file_uri, parse_lsp_locations, path_from_lsp_uri,
    };
    use std::path::Path;

    #[test]
    fn lsp_file_uris_escape_spaces_and_decode_for_display() {
        let uri = lsp_file_uri(Path::new("/repo/with space/main.rs"));

        assert_eq!(uri, "file:///repo/with%20space/main.rs");
        assert_eq!(path_from_lsp_uri(&uri), "/repo/with space/main.rs");
    }

    #[test]
    fn lsp_locations_parse_location_and_location_link_shapes() {
        let value = serde_json::json!([
            {
                "uri": "file:///repo/src/lib.rs",
                "range": {"start": {"line": 2, "character": 4}}
            },
            {
                "targetUri": "file:///repo/src/main.rs",
                "targetSelectionRange": {"start": {"line": 9, "character": 1}}
            }
        ]);

        let locations = parse_lsp_locations(Some(&value));

        assert_eq!(
            locations,
            vec![
                LspLocation {
                    uri: String::from("file:///repo/src/lib.rs"),
                    line: 2,
                    character: 4,
                },
                LspLocation {
                    uri: String::from("file:///repo/src/main.rs"),
                    line: 9,
                    character: 1,
                },
            ]
        );
    }

    #[test]
    fn lsp_locations_render_relative_editor_positions() {
        let rendered = format_lsp_locations(
            vec![LspLocation {
                uri: String::from("file:///repo/src/lib.rs"),
                line: 2,
                character: 4,
            }],
            "Definition",
            Path::new("/repo"),
        );

        assert_eq!(rendered, "Definitions (1):\n  src/lib.rs:3:5");
    }

    #[test]
    fn lsp_hover_extracts_plain_marked_string_and_arrays() {
        assert_eq!(
            format_lsp_hover(Some(&serde_json::json!({"contents": "let value: String"}))),
            Some(String::from("let value: String"))
        );
        assert_eq!(
            format_lsp_hover(Some(&serde_json::json!({
                "contents": [{"language": "rust", "value": "fn main()"}, "plain"]
            }))),
            Some(String::from("fn main()\nplain"))
        );
    }

    #[test]
    fn lsp_document_symbols_render_flat_symbol_lists() {
        let rendered = format_lsp_document_symbols(Some(&serde_json::json!([
            {
                "name": "run",
                "kind": 12,
                "location": {"range": {"start": {"line": 7}}}
            },
            {
                "name": "State",
                "kind": 23,
                "range": {"start": {"line": 1}}
            }
        ])));

        assert_eq!(rendered, "Function run (line 8)\nStruct State (line 2)");
    }
}
