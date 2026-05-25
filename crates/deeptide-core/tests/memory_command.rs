use std::fs;
use std::sync::{Mutex, OnceLock};

use deeptide_core::{
    CommandContext, CommandResult, MemoryCommand, RememberCommand, SlashCommand,
    memory::{
        MemoryScope, MemorySystem, MemoryType, add_to_memory_index, create_memory_file, save_memory,
    },
};
use tempfile::TempDir;

static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

#[test]
fn memory_show_and_delete_project_shard() {
    let _guard = ENV_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let temp = TestEnv::new();
    temp.install();

    let workspace = temp.root.path().join("workspace");
    fs::create_dir_all(&workspace).expect("workspace should be created");

    let memory_dir = MemorySystem::project_memory_dir(&workspace);
    fs::create_dir_all(&memory_dir).expect("memory dir should be created");
    let shard = memory_dir.join("coding-style.md");
    fs::write(
        &shard,
        "---\nname: Coding Style\ndescription: Prefer focused tests\ntype: project\n---\n\nKeep tests narrow and deterministic.\n",
    )
    .expect("memory shard should be written");
    fs::write(
        memory_dir.join("MEMORY.md"),
        "- [Coding Style](coding-style.md) - Prefer focused tests\n",
    )
    .expect("memory index should be written");

    let context = CommandContext::builder()
        .cwd({
            let workspace = workspace.clone();
            move || workspace.clone()
        })
        .build();
    let command = MemoryCommand;

    let list = text(command.execute("", &context));
    assert!(list.contains("coding-style.md"));
    assert!(list.contains("Prefer focused tests"));

    let shown = text(command.execute("show Coding Style", &context));
    assert!(shown.contains("Coding Style"));
    assert!(shown.contains("scope:"));
    assert!(shown.contains("Keep tests narrow and deterministic."));

    let deleted = text(command.execute("delete coding-style", &context));
    assert!(deleted.contains("Deleted project memory: coding-style.md"));
    assert!(!shard.exists());
    let index =
        fs::read_to_string(memory_dir.join("MEMORY.md")).expect("index should still be readable");
    assert!(!index.contains("coding-style.md"));
}

#[test]
fn memory_ambiguous_name_requires_scope() {
    let _guard = ENV_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let temp = TestEnv::new();
    temp.install();

    let workspace = temp.root.path().join("ambiguous-workspace");
    fs::create_dir_all(&workspace).expect("workspace should be created");
    write_memory("Shared Fact", "shared.md", Scope::Project, &workspace);
    write_memory("Shared Fact", "shared.md", Scope::Global, &workspace);

    let context = CommandContext::builder()
        .cwd({
            let workspace = workspace.clone();
            move || workspace.clone()
        })
        .build();

    let output = text(MemoryCommand.execute("show Shared Fact", &context));
    assert!(output.contains("Memory name is ambiguous"));
    assert!(output.contains("project: shared.md"));
    assert!(output.contains("global: shared.md"));
}

#[test]
fn remember_command_appends_timestamped_project_note() {
    let _guard = ENV_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let temp = TestEnv::new();
    temp.install();

    let workspace = temp.root.path().join("remember-workspace");
    fs::create_dir_all(&workspace).expect("workspace should be created");
    let context = CommandContext::builder()
        .cwd({
            let workspace = workspace.clone();
            move || workspace.clone()
        })
        .now_rfc3339(|| String::from("2026-05-25T01:23:45Z"))
        .build();

    let output = text(RememberCommand.execute("prefer cargo nextest when available", &context));

    let index = MemorySystem::project_memory_dir(&workspace).join("MEMORY.md");
    assert_eq!(output, format!("Saved to {}", index.display()));
    let content = fs::read_to_string(index).expect("memory index should be readable");
    assert_eq!(
        content,
        "# Deeptide project memory\n\n- [2026-05-25T01:23:45Z] prefer cargo nextest when available\n"
    );
}

#[test]
fn memory_shard_helpers_create_file_and_index_entry() {
    let _guard = ENV_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let temp = TestEnv::new();
    temp.install();

    let workspace = temp.root.path().join("write-tool-workspace");
    fs::create_dir_all(&workspace).expect("workspace should be created");
    let content = create_memory_file(
        "Provider Policy",
        "Keep API selection explicit",
        MemoryType::Project,
        "Use configured provider profiles instead of hard-coded endpoints.",
    );

    let file_path = save_memory(
        "provider-policy.md",
        &content,
        &workspace,
        MemoryScope::Project,
    )
    .expect("memory file should be saved");
    let index_path = add_to_memory_index(
        "- [Provider Policy](provider-policy.md) - Keep API selection explicit",
        &workspace,
        MemoryScope::Project,
    )
    .expect("memory index should be updated");

    assert_eq!(
        fs::read_to_string(file_path).expect("memory shard should be readable"),
        "---\nname: Provider Policy\ndescription: Keep API selection explicit\ntype: project\n---\n\nUse configured provider profiles instead of hard-coded endpoints."
    );
    assert_eq!(
        fs::read_to_string(index_path).expect("memory index should be readable"),
        "- [Provider Policy](provider-policy.md) - Keep API selection explicit\n"
    );
}

enum Scope {
    Project,
    Global,
}

fn write_memory(name: &str, file: &str, scope: Scope, cwd: &std::path::Path) {
    let dir = match scope {
        Scope::Project => MemorySystem::project_memory_dir(cwd),
        Scope::Global => MemorySystem::global_memory_dir(),
    };
    fs::create_dir_all(&dir).expect("memory dir should be created");
    fs::write(
        dir.join(file),
        format!("---\nname: {name}\ndescription: Test memory\ntype: project\n---\n\nBody.\n"),
    )
    .expect("memory file should be written");
    fs::write(
        dir.join("MEMORY.md"),
        format!("- [{name}]({file}) - Test memory\n"),
    )
    .expect("memory index should be written");
}

fn text(result: CommandResult) -> String {
    match result {
        CommandResult::Text(value) => value,
        other => panic!("expected text command result, got {other:?}"),
    }
}

struct TestEnv {
    root: TempDir,
    original_home: Option<String>,
    original_tide_config_dir: Option<String>,
}

impl TestEnv {
    fn new() -> Self {
        Self {
            root: TempDir::new().expect("tempdir should be created"),
            original_home: std::env::var("HOME").ok(),
            original_tide_config_dir: std::env::var("TIDE_CONFIG_DIR").ok(),
        }
    }

    fn install(&self) {
        unsafe {
            std::env::set_var("HOME", self.root.path().join("home"));
            std::env::set_var("TIDE_CONFIG_DIR", self.root.path().join("tide-config"));
        }
    }
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        unsafe {
            if let Some(value) = &self.original_home {
                std::env::set_var("HOME", value);
            } else {
                std::env::remove_var("HOME");
            }

            if let Some(value) = &self.original_tide_config_dir {
                std::env::set_var("TIDE_CONFIG_DIR", value);
            } else {
                std::env::remove_var("TIDE_CONFIG_DIR");
            }
        }
    }
}
