use crate::commands::SlashCommand;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandCompletionSource {
    pub name: String,
    pub aliases: Vec<String>,
    pub description: String,
    pub usage: String,
}

impl CommandCompletionSource {
    pub fn new(
        name: impl Into<String>,
        aliases: impl IntoIterator<Item = impl Into<String>>,
        description: impl Into<String>,
        usage: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            aliases: aliases.into_iter().map(Into::into).collect(),
            description: description.into(),
            usage: usage.into(),
        }
    }

    pub fn from_command(command: &impl SlashCommand) -> Self {
        Self {
            name: command.name().to_owned(),
            aliases: command.aliases().iter().map(ToString::to_string).collect(),
            description: command.description().to_owned(),
            usage: command.usage().to_owned(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandCompletionCandidate {
    pub name: String,
    pub aliases: Vec<String>,
    pub description: String,
    pub usage: String,
    pub matched_text: String,
    pub score: u8,
}

impl CommandCompletionCandidate {
    pub fn replacement(&self) -> String {
        format!("/{}", self.name)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandCompletionResult {
    pub signature: String,
    pub token_start: usize,
    pub token_end: usize,
    pub typed: String,
    pub candidates: Vec<CommandCompletionCandidate>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Replacement {
    pub text: String,
    pub cursor: usize,
}

pub struct CompletionEngine;

impl CompletionEngine {
    pub fn command_completions(
        input: &str,
        cursor: usize,
        commands: &[CommandCompletionSource],
        limit: usize,
    ) -> Option<CommandCompletionResult> {
        let chars: Vec<char> = input.chars().collect();
        if cursor < 1 || cursor > chars.len() {
            return None;
        }

        let mut start = cursor;
        while start > 0 && !chars[start - 1].is_whitespace() {
            start -= 1;
        }
        if start != 0 {
            return None;
        }

        let mut end = cursor;
        while end < chars.len() && !chars[end].is_whitespace() {
            end += 1;
        }

        let token: String = chars[start..cursor].iter().collect();
        let typed = token.strip_prefix('/')?.to_lowercase();

        let mut candidates: Vec<_> = commands
            .iter()
            .filter_map(|command| candidate_for(command, &typed))
            .collect();

        candidates.sort_by(|left, right| {
            left.score
                .cmp(&right.score)
                .then_with(|| left.name.chars().count().cmp(&right.name.chars().count()))
                .then_with(|| left.name.cmp(&right.name))
        });

        if candidates.is_empty() {
            return None;
        }

        candidates.truncate(limit);

        Some(CommandCompletionResult {
            signature: chars[start..end].iter().collect(),
            token_start: start,
            token_end: end,
            typed,
            candidates,
        })
    }

    pub fn replacing_token(
        input: &str,
        result: &CommandCompletionResult,
        candidate: &CommandCompletionCandidate,
    ) -> Replacement {
        let chars: Vec<char> = input.chars().collect();
        let prefix: String = chars[..result.token_start].iter().collect();
        let suffix: String = chars[result.token_end..].iter().collect();
        let replacement = candidate.replacement();
        let spacer = if suffix.is_empty() { " " } else { "" };
        let text = format!("{prefix}{replacement}{spacer}{suffix}");
        let cursor = (prefix + &replacement + spacer).chars().count();

        Replacement { text, cursor }
    }
}

fn candidate_for(
    command: &CommandCompletionSource,
    typed: &str,
) -> Option<CommandCompletionCandidate> {
    let name = command.name.to_lowercase();
    let aliases: Vec<String> = command
        .aliases
        .iter()
        .map(|alias| alias.to_lowercase())
        .collect();

    let (score, matched_text) = if typed.is_empty() || name.starts_with(typed) {
        (0, command.name.clone())
    } else if let Some((index, _)) = aliases
        .iter()
        .enumerate()
        .find(|(_, alias)| alias.starts_with(typed))
    {
        (1, command.aliases[index].clone())
    } else if name.contains(typed) {
        (2, command.name.clone())
    } else if let Some((index, _)) = aliases
        .iter()
        .enumerate()
        .find(|(_, alias)| alias.contains(typed))
    {
        (3, command.aliases[index].clone())
    } else {
        return None;
    };

    Some(CommandCompletionCandidate {
        name: command.name.clone(),
        aliases: command.aliases.clone(),
        description: command.description.clone(),
        usage: command.usage.clone(),
        matched_text,
        score,
    })
}
