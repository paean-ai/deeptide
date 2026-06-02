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

/// Snapshot of the current todo storage for cheap "is the agent
/// tracking tasks right now?" queries. Used by the pinned status bar
/// to surface a `todo N/M` segment so the user always knows whether
/// there's an open backlog. Returns zeros (not `None`) when the
/// storage is empty or poisoned so the caller can render
/// unconditionally.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct TodoSummary {
    /// Number of items whose status is `pending` (queued, not yet
    /// started).
    pub pending: usize,
    /// Number of items currently `in_progress` — usually 1 in
    /// well-behaved agent runs, but we count them honestly.
    pub in_progress: usize,
    /// Number of items already `completed`.
    pub completed: usize,
}

impl TodoSummary {
    /// Total number of items in the snapshot (pending + in_progress
    /// + completed).
    ///
    /// Deleted items are not counted because they're already removed
    /// from storage.
    pub fn total(self) -> usize {
        self.pending + self.in_progress + self.completed
    }

    /// `true` when the storage holds at least one item. The status
    /// bar uses this to decide whether to render the `todo` segment
    /// at all (no segment ⇒ less visual noise on greenfield
    /// sessions).
    pub fn is_active(self) -> bool {
        self.total() > 0
    }
}

/// The content of the next actionable todo: the first in-progress item, else
/// the first pending one. `None` when the plan is empty or fully done. Used by
/// the follow-up suggestion engine to propose "continue with <task>".
pub fn next_actionable_todo() -> Option<String> {
    let todos = todo_storage().lock().ok()?;
    todos
        .iter()
        .find(|t| t.status == TodoStatus::InProgress)
        .or_else(|| todos.iter().find(|t| t.status == TodoStatus::Pending))
        .map(|t| t.content.clone())
}

/// Read the current todo storage and reduce it to a [`TodoSummary`].
/// Public because the pinned status bar needs to surface this on
/// every repaint, but the underlying `TodoItem` type stays private
/// (the dashboard only needs counts, not contents).
pub fn todo_summary() -> TodoSummary {
    let Ok(todos) = todo_storage().lock() else {
        return TodoSummary::default();
    };
    let mut summary = TodoSummary::default();
    for item in todos.iter() {
        match item.status {
            TodoStatus::Pending => summary.pending += 1,
            TodoStatus::InProgress => summary.in_progress += 1,
            TodoStatus::Completed => summary.completed += 1,
            TodoStatus::Deleted => {}
        }
    }
    summary
}

#[cfg(test)]
mod todo_summary_tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    fn reset_storage() {
        replace_todos(Vec::new());
    }

    #[test]
    fn empty_storage_yields_a_zeroed_inactive_summary() {
        reset_storage();
        let s = todo_summary();
        assert_eq!(s, TodoSummary::default());
        assert!(!s.is_active(), "empty storage must read as inactive");
        assert_eq!(s.total(), 0);
    }

    #[test]
    fn summary_counts_each_status_independently() {
        reset_storage();
        replace_todos(vec![
            TodoItem {
                content: "a".into(),
                status: TodoStatus::Pending,
                active_form: None,
            },
            TodoItem {
                content: "b".into(),
                status: TodoStatus::Pending,
                active_form: None,
            },
            TodoItem {
                content: "c".into(),
                status: TodoStatus::InProgress,
                active_form: None,
            },
            TodoItem {
                content: "d".into(),
                status: TodoStatus::Completed,
                active_form: None,
            },
        ]);
        let s = todo_summary();
        assert_eq!(s.pending, 2);
        assert_eq!(s.in_progress, 1);
        assert_eq!(s.completed, 1);
        assert_eq!(s.total(), 4);
        assert!(s.is_active());
        reset_storage();
    }

    #[test]
    fn deleted_status_does_not_contribute_to_any_count() {
        // We never persist `Deleted` in storage (delete_todo removes
        // the item outright), but defend the match arm so a future
        // refactor can't silently bump counts.
        reset_storage();
        replace_todos(vec![TodoItem {
            content: "ghost".into(),
            status: TodoStatus::Deleted,
            active_form: None,
        }]);
        let s = todo_summary();
        assert_eq!(s.total(), 0);
        reset_storage();
    }
}
