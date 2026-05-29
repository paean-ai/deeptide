//! Task and todo tracking tools (TodoWrite + Task* family).
//!
//! Split out of the monolithic tools module for maintainability. All shared
//! infrastructure (the `Tool` trait, `ToolResult`, `ToolContext`, helper
//! functions) lives in the parent module and is reached via `use super::*`.

use super::*;

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
                    "note": format!("no todo task with id {task_id}. TaskStorage tracks the agent's todo list. For background shell output use BashOutput with the shell_id from Bash run_in_background=true."),
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
