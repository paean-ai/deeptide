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
