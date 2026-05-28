use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;
use std::fs;
use std::io;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PermissionDecision {
    Allow,
    Deny { reason: String },
    Ask,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PermissionMode {
    Default,
    AcceptEdits,
    Plan,
    Bypass,
}

impl PermissionMode {
    pub fn description(self) -> &'static str {
        match self {
            Self::Default => {
                "Ask before running commands, writing files, or making network requests."
            }
            Self::AcceptEdits => "Auto-approve file edits. Ask for everything else.",
            Self::Plan => {
                "Allow reads and exploration (Read, Bash, …); block file writes and high-risk tools until you exit plan mode."
            }
            Self::Bypass => "YOLO mode: bypass all permission checks.",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::AcceptEdits => "accept-edits",
            Self::Plan => "plan",
            Self::Bypass => "yolo",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "default" => Some(Self::Default),
            "accept-edits" | "accept_edits" | "acceptedits" => Some(Self::AcceptEdits),
            "plan" => Some(Self::Plan),
            "bypass" | "yolo" => Some(Self::Bypass),
            _ => None,
        }
    }
}

impl fmt::Display for PermissionMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rule {
    pub pattern: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool: Option<String>,
}

impl Rule {
    pub fn new(pattern: impl Into<String>, tool: Option<impl Into<String>>) -> Self {
        Self {
            pattern: pattern.into(),
            tool: tool.map(Into::into),
        }
    }

    pub fn matches(&self, tool_name: &str, input: &ToolInput<'_>) -> bool {
        if let Some(tool) = &self.tool
            && tool != tool_name
        {
            return false;
        }

        if self.pattern == "*" {
            return true;
        }

        let candidate = input.match_candidate();
        if self.pattern.ends_with('*') {
            let prefix = &self.pattern[..self.pattern.len() - 1];
            return candidate.is_some_and(|value| value.starts_with(prefix));
        }

        candidate.is_some_and(|value| value == self.pattern)
    }
}

#[derive(Debug, Default, Clone)]
pub struct ToolInput<'a> {
    pub command: Option<&'a str>,
    pub text: Option<&'a str>,
    pub path: Option<&'a str>,
    pub operation: Option<&'a str>,
}

impl<'a> ToolInput<'a> {
    pub fn from_json(input: &'a serde_json::Map<String, Value>) -> Self {
        Self {
            command: value_at(input, &["command"]),
            text: value_at(input, &["text"]),
            path: value_at(input, &["path", "file_path"]),
            operation: value_at(input, &["operation"]),
        }
    }

    fn match_candidate(&self) -> Option<&'a str> {
        self.command.or(self.text).or(self.path)
    }
}

fn value_at<'a>(input: &'a serde_json::Map<String, Value>, keys: &[&str]) -> Option<&'a str> {
    keys.iter()
        .find_map(|key| input.get(*key).and_then(Value::as_str))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleResult {
    Allow,
    Deny,
    None,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct RulesFile {
    #[serde(default)]
    allow: Vec<Rule>,
    #[serde(default)]
    deny: Vec<Rule>,
}

#[derive(Debug)]
pub struct PermissionRules {
    allow_list: Vec<Rule>,
    deny_list: Vec<Rule>,
    session_allow_list: Vec<Rule>,
    session_deny_list: Vec<Rule>,
    storage_path: PathBuf,
}

impl PermissionRules {
    pub fn in_memory() -> Self {
        Self {
            allow_list: Vec::new(),
            deny_list: Vec::new(),
            session_allow_list: Vec::new(),
            session_deny_list: Vec::new(),
            storage_path: std::env::temp_dir().join("deeptide-permissions.json"),
        }
    }

    pub fn load(storage_path: Option<PathBuf>) -> io::Result<Self> {
        let storage_path = storage_path.unwrap_or_else(default_rules_path);
        let file = match fs::read(&storage_path) {
            Ok(bytes) => serde_json::from_slice::<RulesFile>(&bytes).unwrap_or_default(),
            Err(error) if error.kind() == io::ErrorKind::NotFound => RulesFile::default(),
            Err(error) => return Err(error),
        };

        Ok(Self {
            allow_list: file.allow,
            deny_list: file.deny,
            session_allow_list: Vec::new(),
            session_deny_list: Vec::new(),
            storage_path,
        })
    }

    pub fn allow_list(&self) -> &[Rule] {
        &self.allow_list
    }

    pub fn deny_list(&self) -> &[Rule] {
        &self.deny_list
    }

    pub fn session_allow_list(&self) -> &[Rule] {
        &self.session_allow_list
    }

    pub fn session_deny_list(&self) -> &[Rule] {
        &self.session_deny_list
    }

    pub fn check(&self, tool_name: &str, input: &ToolInput<'_>) -> RuleResult {
        if self
            .deny_list
            .iter()
            .any(|rule| rule.matches(tool_name, input))
        {
            return RuleResult::Deny;
        }
        if self
            .session_deny_list
            .iter()
            .any(|rule| rule.matches(tool_name, input))
        {
            return RuleResult::Deny;
        }
        if self
            .allow_list
            .iter()
            .any(|rule| rule.matches(tool_name, input))
        {
            return RuleResult::Allow;
        }
        if self
            .session_allow_list
            .iter()
            .any(|rule| rule.matches(tool_name, input))
        {
            return RuleResult::Allow;
        }

        RuleResult::None
    }

    pub fn add_allow(
        &mut self,
        pattern: impl Into<String>,
        tool: Option<impl Into<String>>,
    ) -> io::Result<()> {
        self.allow_list.push(Rule::new(pattern, tool));
        self.save()
    }

    pub fn add_deny(
        &mut self,
        pattern: impl Into<String>,
        tool: Option<impl Into<String>>,
    ) -> io::Result<()> {
        self.deny_list.push(Rule::new(pattern, tool));
        self.save()
    }

    pub fn add_session_allow(
        &mut self,
        pattern: impl Into<String>,
        tool: Option<impl Into<String>>,
    ) {
        self.session_allow_list.push(Rule::new(pattern, tool));
    }

    pub fn add_session_deny(
        &mut self,
        pattern: impl Into<String>,
        tool: Option<impl Into<String>>,
    ) {
        self.session_deny_list.push(Rule::new(pattern, tool));
    }

    pub fn remove_allow(&mut self, pattern: &str) -> io::Result<()> {
        self.allow_list.retain(|rule| rule.pattern != pattern);
        self.save()
    }

    pub fn remove_deny(&mut self, pattern: &str) -> io::Result<()> {
        self.deny_list.retain(|rule| rule.pattern != pattern);
        self.save()
    }

    pub fn clear_all(&mut self) -> io::Result<()> {
        self.allow_list.clear();
        self.deny_list.clear();
        self.save()
    }

    fn save(&self) -> io::Result<()> {
        let file = RulesFile {
            allow: self.allow_list.clone(),
            deny: self.deny_list.clone(),
        };
        let bytes = serde_json::to_vec_pretty(&file)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;

        if let Some(parent) = self.storage_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&self.storage_path, bytes)
    }
}

fn default_rules_path() -> PathBuf {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    home.join(".config").join("tide").join("permissions.json")
}

#[derive(Debug)]
pub struct PermissionManager {
    mode: PermissionMode,
    rules: PermissionRules,
}

impl PermissionManager {
    pub fn new(mode: PermissionMode, rules: PermissionRules) -> Self {
        Self { mode, rules }
    }

    pub fn mode(&self) -> PermissionMode {
        self.mode
    }

    pub fn set_mode(&mut self, mode: PermissionMode) {
        self.mode = mode;
    }

    pub fn rules(&self) -> &PermissionRules {
        &self.rules
    }

    pub fn add_rule(
        &mut self,
        allowed: bool,
        pattern: impl Into<String>,
        tool: Option<String>,
    ) -> io::Result<()> {
        if allowed {
            self.rules.add_allow(pattern, tool)
        } else {
            self.rules.add_deny(pattern, tool)
        }
    }

    pub fn remove_rule(&mut self, pattern: &str) -> io::Result<()> {
        self.rules.remove_allow(pattern)?;
        self.rules.remove_deny(pattern)
    }

    pub fn check_json(
        &self,
        tool_name: &str,
        input: &serde_json::Map<String, Value>,
    ) -> PermissionDecision {
        self.check(tool_name, &ToolInput::from_json(input))
    }

    pub fn check(&self, tool_name: &str, input: &ToolInput<'_>) -> PermissionDecision {
        if self.mode == PermissionMode::Bypass {
            return PermissionDecision::Allow;
        }

        if self.mode == PermissionMode::Plan {
            if matches!(tool_name, "Bash" | "Monitor") {
                return if is_plan_safe_shell_command(input.command) {
                    PermissionDecision::Allow
                } else {
                    PermissionDecision::Deny {
                        reason: "Plan mode — Bash/Monitor are limited to read-only exploration commands. Use Read/Grep/Glob for inspection, or call ExitPlanMode before running commands with side effects.".into(),
                    }
                };
            }

            if tool_name == "Clipboard" {
                return if matches!(
                    input.operation,
                    Some("inspect" | "read" | "files" | "finder_selection")
                ) {
                    PermissionDecision::Allow
                } else {
                    PermissionDecision::Deny {
                        reason: "Plan mode — Clipboard write is blocked until you exit plan mode."
                            .into(),
                    }
                };
            }

            if is_denied_in_plan_mode(tool_name) {
                return PermissionDecision::Deny {
                    reason: "Plan mode — file writes, sub-agents, and similar actions are blocked. Use Read/Grep/Glob/Bash to explore; call ExitPlanMode when ready to implement.".into(),
                };
            }

            return PermissionDecision::Allow;
        }

        if is_read_only(tool_name) {
            return PermissionDecision::Allow;
        }

        if self.mode == PermissionMode::AcceptEdits && matches!(tool_name, "Edit" | "Write") {
            return PermissionDecision::Allow;
        }

        match self.rules.check(tool_name, input) {
            RuleResult::Allow => return PermissionDecision::Allow,
            RuleResult::Deny => {
                return PermissionDecision::Deny {
                    reason: "Blocked by permission rule.".into(),
                };
            }
            RuleResult::None => {}
        }

        if is_destructive(tool_name) {
            return PermissionDecision::Ask;
        }

        PermissionDecision::Allow
    }

    pub fn add_session_rule(&mut self, allowed: bool, tool_name: &str, pattern: &str) {
        if allowed {
            self.rules
                .add_session_allow(pattern.to_owned(), Some(tool_name.to_owned()));
        } else {
            self.rules
                .add_session_deny(pattern.to_owned(), Some(tool_name.to_owned()));
        }
    }
}

fn is_read_only(tool_name: &str) -> bool {
    matches!(
        tool_name,
        "Read"
            | "ReadFiles"
            | "Glob"
            | "Grep"
            | "TaskList"
            | "TaskGet"
            | "TaskOutput"
            | "VerifyPlanExecution"
            | "Sleep"
            | "CtxInspect"
            | "Snip"
            | "Brief"
            | "AudioTranscribe"
            | "VideoTranscribe"
            | "MemorySearch"
            | "ListMcpResources"
            | "ReadMcpResource"
            | "ListMcpPrompts"
            | "GetMcpPrompt"
            | "LSP"
            | "SpotlightSearch"
            | "ScreenCapture"
            | "ImagePreprocess"
            | "Vision"
            | "CrashLog"
            | "MacLog"
            | "MacDiagnose"
            | "ReviewArtifact"
    )
}

fn is_denied_in_plan_mode(tool_name: &str) -> bool {
    matches!(
        tool_name,
        "Write"
            | "Edit"
            | "NotebookEdit"
            | "MemoryWrite"
            | "WebFetch"
            | "TodoWrite"
            | "TaskCreate"
            | "TaskUpdate"
            | "TaskStop"
            | "CronCreate"
            | "CronDelete"
            | "EnterWorktree"
            | "ExitWorktree"
            | "Agent"
            | "Skill"
            | "MCP"
            | "Publish"
            | "RemoteTrigger"
            | "PushNotification"
    )
}

fn is_plan_safe_shell_command(command: Option<&str>) -> bool {
    let Some(command) = command else {
        return false;
    };
    let trimmed = command.trim();
    if trimmed.is_empty() {
        return false;
    }

    const BLOCKED_FRAGMENTS: [&str; 8] = [";", "&&", "||", ">", "<", "&", "`", "$("];
    if BLOCKED_FRAGMENTS
        .iter()
        .any(|fragment| trimmed.contains(fragment))
    {
        return false;
    }

    for raw_segment in trimmed.split('|') {
        let segment = raw_segment.trim();
        if segment.contains(" -exec ")
            || segment.contains(" --exec")
            || segment.contains(" -delete")
            || segment.contains(" -i ")
            || segment.ends_with(" -i")
        {
            return false;
        }

        let mut parts = segment.split_whitespace();
        let Some(executable) = parts.next() else {
            return false;
        };
        if !is_plan_safe_executable(executable, segment) {
            return false;
        }
    }

    true
}

fn is_plan_safe_executable(executable: &str, segment: &str) -> bool {
    match executable {
        "pwd" | "ls" | "find" | "fd" | "rg" | "grep" | "cat" | "sed" | "head" | "tail" | "wc"
        | "sort" | "uniq" | "jq" | "tree" | "file" | "stat" | "du" | "df" | "which" | "type" => {
            true
        }
        "git" => {
            let parts: Vec<_> = segment.split_whitespace().collect();
            matches!(
                parts.get(1).copied(),
                Some(
                    "status"
                        | "diff"
                        | "log"
                        | "show"
                        | "branch"
                        | "rev-parse"
                        | "grep"
                        | "ls-files"
                        | "remote"
                        | "blame"
                        | "describe"
                )
            )
        }
        _ => false,
    }
}

fn is_destructive(tool_name: &str) -> bool {
    matches!(
        tool_name,
        "Bash"
            | "Monitor"
            | "Write"
            | "Edit"
            | "MemoryWrite"
            | "WebFetch"
            | "WebSearch"
            | "TaskCreate"
            | "TaskUpdate"
            | "TaskStop"
            | "NotebookEdit"
            | "CronCreate"
            | "CronDelete"
            | "EnterWorktree"
            | "ExitWorktree"
            | "Agent"
            | "Skill"
            | "EnterPlanMode"
            | "ExitPlanMode"
            | "AskUserQuestion"
            | "MCP"
            | "Publish"
            | "RemoteTrigger"
            | "PushNotification"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::TempDir;

    const ASK_TOOLS: &[&str] = &[
        "Agent",
        "AskUserQuestion",
        "Bash",
        "Monitor",
        "CronCreate",
        "CronDelete",
        "Edit",
        "Write",
        "NotebookEdit",
        "EnterPlanMode",
        "ExitPlanMode",
        "EnterWorktree",
        "ExitWorktree",
        "MCP",
        "MemoryWrite",
        "Publish",
        "PushNotification",
        "RemoteTrigger",
        "Skill",
        "TaskCreate",
        "TaskStop",
        "TaskUpdate",
        "WebFetch",
        "WebSearch",
    ];

    const ALLOW_TOOLS: &[&str] = &[
        "AudioTranscribe",
        "Brief",
        "Clipboard",
        "CrashLog",
        "CronList",
        "CtxInspect",
        "FileMetadata",
        "GetMcpPrompt",
        "Glob",
        "Grep",
        "ImagePreprocess",
        "LSP",
        "ListMcpPrompts",
        "ListMcpResources",
        "MacDiagnose",
        "MacLog",
        "MemorySearch",
        "Read",
        "ReadFiles",
        "ReadMcpResource",
        "ReviewArtifact",
        "ScreenCapture",
        "Sleep",
        "Snip",
        "SpotlightSearch",
        "TaskGet",
        "TaskList",
        "TaskOutput",
        "TodoWrite",
        "ToolSearch",
        "VerifyPlanExecution",
        "VideoTranscribe",
        "Vision",
    ];

    const PLAN_DENY_TOOLS: &[&str] = &[
        "Agent",
        "CronCreate",
        "CronDelete",
        "Edit",
        "Write",
        "NotebookEdit",
        "EnterWorktree",
        "ExitWorktree",
        "MCP",
        "MemoryWrite",
        "Publish",
        "PushNotification",
        "RemoteTrigger",
        "Skill",
        "TaskCreate",
        "TaskStop",
        "TaskUpdate",
        "TodoWrite",
        "WebFetch",
    ];

    #[test]
    fn permission_mode_parse_and_labels_match_swift() {
        assert_eq!(
            PermissionMode::parse("default"),
            Some(PermissionMode::Default)
        );
        assert_eq!(
            PermissionMode::parse("accept_edits"),
            Some(PermissionMode::AcceptEdits)
        );
        assert_eq!(
            PermissionMode::parse("acceptedits"),
            Some(PermissionMode::AcceptEdits)
        );
        assert_eq!(PermissionMode::parse("plan"), Some(PermissionMode::Plan));
        assert_eq!(PermissionMode::parse("yolo"), Some(PermissionMode::Bypass));
        assert_eq!(PermissionMode::Bypass.label(), "yolo");
    }

    #[test]
    fn default_mode_tool_classification_matches_swift_reference() {
        let manager = PermissionManager::new(PermissionMode::Default, temp_rules());
        for tool in ASK_TOOLS {
            assert_eq!(
                manager.check(tool, &sample_input(tool)),
                PermissionDecision::Ask
            );
        }
        for tool in ALLOW_TOOLS {
            assert_eq!(
                manager.check(tool, &sample_input(tool)),
                PermissionDecision::Allow
            );
        }
    }

    #[test]
    fn plan_mode_behavior_matches_swift_reference() {
        let manager = PermissionManager::new(PermissionMode::Plan, temp_rules());
        for tool in PLAN_DENY_TOOLS {
            assert_denied(manager.check(tool, &sample_input(tool)));
        }

        for tool in ASK_TOOLS
            .iter()
            .chain(ALLOW_TOOLS.iter())
            .copied()
            .filter(|tool| !PLAN_DENY_TOOLS.contains(tool))
        {
            assert_eq!(
                manager.check(tool, &sample_input(tool)),
                PermissionDecision::Allow
            );
        }
    }

    #[test]
    fn plan_mode_allows_read_only_exploration_but_blocks_risky_shell() {
        let manager = PermissionManager::new(PermissionMode::Plan, temp_rules());
        assert_eq!(
            manager.check(
                "Bash",
                &ToolInput {
                    command: Some("git status"),
                    ..ToolInput::default()
                }
            ),
            PermissionDecision::Allow
        );
        assert_denied(manager.check(
            "Bash",
            &ToolInput {
                command: Some("rm -rf build"),
                ..ToolInput::default()
            },
        ));
        assert_denied(manager.check(
            "Clipboard",
            &ToolInput {
                operation: Some("write"),
                ..ToolInput::default()
            },
        ));
    }

    #[test]
    fn session_rules_override_default_prompting_but_not_deny_rules() {
        let mut manager = PermissionManager::new(PermissionMode::Default, temp_rules());
        assert_eq!(
            manager.check("MemoryWrite", &ToolInput::default()),
            PermissionDecision::Ask
        );

        manager.add_session_rule(true, "MemoryWrite", "*");
        assert_eq!(
            manager.check("MemoryWrite", &ToolInput::default()),
            PermissionDecision::Allow
        );

        manager.add_session_rule(false, "MemoryWrite", "*");
        assert_denied(manager.check("MemoryWrite", &ToolInput::default()));
    }

    #[test]
    fn accept_edits_mode_allows_file_write_tools() {
        let manager = PermissionManager::new(PermissionMode::AcceptEdits, temp_rules());

        assert_eq!(
            manager.check("Write", &sample_input("Write")),
            PermissionDecision::Allow
        );
        assert_eq!(
            manager.check("Edit", &sample_input("Edit")),
            PermissionDecision::Allow
        );
        assert_eq!(
            manager.check("Bash", &sample_input("Bash")),
            PermissionDecision::Ask
        );
    }

    #[test]
    fn persistent_rules_round_trip() {
        let temp = TempDir::new()
            .unwrap_or_else(|error| panic!("temporary directory should be created: {error}"));
        let path = temp.path().join("permissions.json");

        let mut rules = PermissionRules::load(Some(path.clone()))
            .unwrap_or_else(|error| panic!("permission rules should load: {error}"));
        rules
            .add_allow("git status", Some("Bash"))
            .unwrap_or_else(|error| panic!("allow rule should be persisted: {error}"));
        rules
            .add_deny("rm -rf *", Some("Bash"))
            .unwrap_or_else(|error| panic!("deny rule should be persisted: {error}"));

        let loaded = PermissionRules::load(Some(path))
            .unwrap_or_else(|error| panic!("permission rules should reload: {error}"));
        assert_eq!(loaded.allow_list().len(), 1);
        assert_eq!(loaded.deny_list().len(), 1);
    }

    #[test]
    fn json_input_adapter_matches_swift_fields() {
        let input = json!({
            "command": "git status",
            "file_path": "README.md",
            "operation": "read"
        });
        let object = input
            .as_object()
            .unwrap_or_else(|| panic!("test input should be a JSON object"));
        let manager = PermissionManager::new(PermissionMode::Plan, temp_rules());
        assert_eq!(
            manager.check_json("Bash", object),
            PermissionDecision::Allow
        );
    }

    fn temp_rules() -> PermissionRules {
        let temp = TempDir::new()
            .unwrap_or_else(|error| panic!("temporary directory should be created: {error}"));
        PermissionRules::load(Some(temp.path().join("permissions.json")))
            .unwrap_or_else(|error| panic!("permission rules should load: {error}"))
    }

    fn sample_input(tool_name: &str) -> ToolInput<'static> {
        match tool_name {
            "Bash" | "Monitor" => ToolInput {
                command: Some("git status"),
                ..ToolInput::default()
            },
            "Clipboard" => ToolInput {
                operation: Some("read"),
                ..ToolInput::default()
            },
            "Read" | "AudioTranscribe" | "FileMetadata" | "ImagePreprocess" | "VideoTranscribe"
            | "Vision" => ToolInput {
                path: Some("README.md"),
                ..ToolInput::default()
            },
            "MemorySearch" | "WebSearch" | "Grep" | "SpotlightSearch" => ToolInput {
                text: Some("needle"),
                ..ToolInput::default()
            },
            "Glob" => ToolInput {
                text: Some("*.swift"),
                ..ToolInput::default()
            },
            "Write" | "Edit" | "NotebookEdit" => ToolInput {
                path: Some("file.txt"),
                ..ToolInput::default()
            },
            "PushNotification" => ToolInput {
                text: Some("done"),
                ..ToolInput::default()
            },
            _ => ToolInput::default(),
        }
    }

    fn assert_denied(decision: PermissionDecision) {
        assert!(matches!(decision, PermissionDecision::Deny { .. }));
    }
}
