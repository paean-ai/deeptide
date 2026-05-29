//! Context-management and plan-mode tools (Brief, CtxInspect, Snip,
//! EnterPlanMode, ExitPlanMode).
//!
//! These tool structs are advisory markers whose real work is performed by
//! the agent loop; shared infrastructure lives in the parent module and is
//! reached via `use super::*`.

use super::*;

#[derive(Debug, Default, Clone, Copy)]
pub struct BriefTool;

impl Tool for BriefTool {
    fn name(&self) -> &'static str {
        "Brief"
    }

    fn description(&self) -> &'static str {
        "Request a context compaction summary of the conversation so far."
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn call(&self, _input: serde_json::Value, _context: &ToolContext) -> ToolResult {
        ToolResult::text(
            "Context compaction triggered. The system will:\n\
             1. Summarize older messages into a compact form\n\
             2. Keep the most recent messages intact\n\
             3. Free context tokens for continued work\n\n\
             Key information (decisions, bugs, current task status) will be preserved in the summary. Continue your task after compaction completes.",
        )
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct CtxInspectTool;

impl Tool for CtxInspectTool {
    fn name(&self) -> &'static str {
        "CtxInspect"
    }

    fn description(&self) -> &'static str {
        "Inspect the current context window budget and cache expectations."
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn call(&self, input: serde_json::Value, _context: &ToolContext) -> ToolResult {
        let model = input
            .get("model")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unconfigured");
        let estimated_tokens = input
            .get("estimated_tokens")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0);
        let message_count = input
            .get("message_count")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0);
        let window = model_context_window(model);
        let used_pct = estimated_tokens
            .saturating_mul(100)
            .checked_div(window)
            .unwrap_or(0);
        let remaining = window.saturating_sub(estimated_tokens);

        let mut lines = vec![
            String::from("Context Window Report:"),
            String::new(),
            format!("Model: {model}"),
            format!("Context window: {} tokens", format_compact_number(window)),
            format!(
                "Estimated usage: {} tokens ({}%)",
                format_compact_number(estimated_tokens),
                used_pct
            ),
            format!("Remaining: {} tokens", format_compact_number(remaining)),
            format!("Active messages: {message_count}"),
        ];

        if used_pct > 90 {
            lines.push(String::new());
            lines.push(format!(
                "CRITICAL: Context at {used_pct}% - consider calling Brief or ending the task."
            ));
        } else if used_pct > 70 {
            lines.push(String::new());
            lines.push(format!(
                "WARNING: Context at {used_pct}% - monitor usage on upcoming tool calls."
            ));
        } else if used_pct < 30 {
            lines.push(String::new());
            lines.push(String::from(
                "Context usage is healthy - ample room for multi-step work.",
            ));
        }

        ToolResult::text(lines.join("\n"))
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct SnipTool;

impl Tool for SnipTool {
    fn name(&self) -> &'static str {
        "Snip"
    }

    fn description(&self) -> &'static str {
        "Request aggressive trimming of older conversation history."
    }

    fn is_read_only(&self) -> bool {
        false
    }

    fn call(&self, input: serde_json::Value, _context: &ToolContext) -> ToolResult {
        let keep_last = input
            .get("keepLast")
            .or_else(|| input.get("keep_last"))
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(10)
            .clamp(1, 100);
        let explanation = input
            .get("explanation")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty());

        let reason = explanation
            .map(|value| format!("\nReason: {value}\n"))
            .unwrap_or_else(|| String::from("\n"));
        ToolResult::text(format!(
            "History trim requested: keeping last {keep_last} messages.{reason}\nThe system will remove older messages and insert a boundary marker. The agent should continue the task with the remaining context."
        ))
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct EnterPlanModeTool;

impl Tool for EnterPlanModeTool {
    fn name(&self) -> &'static str {
        "EnterPlanMode"
    }

    fn description(&self) -> &'static str {
        "Enter plan mode before making significant code changes."
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn call(&self, _input: serde_json::Value, _context: &ToolContext) -> ToolResult {
        ToolResult::text(
            "Plan mode activated. You should now:\n\
             1. Explore the codebase to understand the existing architecture (Read, Grep, Glob, and read-only Bash are allowed)\n\
             2. Identify the files and components involved\n\
             3. Design an implementation approach\n\
             4. Present your plan to the user for approval using ExitPlanMode\n\
             File edits, writes, sub-agents, MCP calls, clipboard writes, and shell commands with side effects are blocked until you exit plan mode or the user changes permission mode. Do NOT modify project files until the user approves your plan.",
        )
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct ExitPlanModeTool;

impl Tool for ExitPlanModeTool {
    fn name(&self) -> &'static str {
        "ExitPlanMode"
    }

    fn description(&self) -> &'static str {
        "Exit plan mode and present the implementation plan for user approval."
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn call(&self, input: serde_json::Value, _context: &ToolContext) -> ToolResult {
        let prompts = extract_allowed_prompts(&input);
        let prompt_list = if prompts.is_empty() {
            String::from("(no additional permissions requested)")
        } else {
            prompts
                .into_iter()
                .map(|(tool, prompt)| format!("  - {tool}: {prompt}"))
                .collect::<Vec<_>>()
                .join("\n")
        };

        let plan = optional_trimmed_string(&input, "plan");
        let plan_file_path = optional_trimmed_string(&input, "planFilePath")
            .or_else(|| optional_trimmed_string(&input, "plan_file_path"));
        let plan_was_edited = input
            .get("planWasEdited")
            .or_else(|| input.get("plan_was_edited"))
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);

        let mut output =
            format!("Plan is ready for review. Implementation will require:\n{prompt_list}");

        if let Some(path) = plan_file_path {
            output.push_str("\n\nPlan file: ");
            output.push_str(&path);
        }
        if plan_was_edited {
            output.push_str("\nPlan was edited before approval.");
        }
        if let Some(plan) = plan {
            output.push_str("\n\nPlan:\n");
            output.push_str(&plan);
            output.push_str("\n\nPlease review and approve to begin implementation.");
        } else {
            output.push_str(
                "\n\nThe plan has been written to the plan file. Please review and approve to begin implementation.",
            );
        }

        ToolResult::text(output)
    }
}
