//! Memory tools: search memory files (MemorySearch) and write memory entries (MemoryWrite).
//!
//! Shared infrastructure lives in the parent module (reached via `use super::*`).

use super::*;

#[derive(Debug, Default, Clone, Copy)]
pub struct MemorySearchTool;

impl Tool for MemorySearchTool {
    fn name(&self) -> &'static str {
        "MemorySearch"
    }

    fn description(&self) -> &'static str {
        "Search Deeptide project and global memory files."
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn call(&self, input: serde_json::Value, context: &ToolContext) -> ToolResult {
        let Some(query) = input.get("query").and_then(serde_json::Value::as_str) else {
            return ToolResult::error("query is required");
        };
        let query = query.trim();
        if query.is_empty() {
            return ToolResult::error("query is required");
        }

        let scope = input
            .get("scope")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("all");
        if !matches!(scope, "project" | "global" | "all") {
            return ToolResult::error("scope must be project, global, or all");
        }
        let max_results = input
            .get("max_results")
            .and_then(serde_json::Value::as_u64)
            .and_then(|value| usize::try_from(value).ok())
            .unwrap_or(10)
            .clamp(1, 50);

        search_memory(query, scope, max_results, &context.cwd)
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct MemoryWriteTool;

impl Tool for MemoryWriteTool {
    fn name(&self) -> &'static str {
        "MemoryWrite"
    }

    fn description(&self) -> &'static str {
        "Persist a concise Deeptide memory shard for future sessions."
    }

    fn is_read_only(&self) -> bool {
        false
    }

    fn call(&self, input: serde_json::Value, context: &ToolContext) -> ToolResult {
        let Some(title) = input.get("title").and_then(serde_json::Value::as_str) else {
            return ToolResult::error("title is required");
        };
        let title = safe_inline(title.trim());
        if title.is_empty() {
            return ToolResult::error("title is required");
        }
        if title.chars().count() > 80 {
            return ToolResult::error("title must be 80 characters or fewer");
        }

        let Some(body) = input.get("body").and_then(serde_json::Value::as_str) else {
            return ToolResult::error("body is required");
        };
        let body = body.trim();
        if body.is_empty() {
            return ToolResult::error("body is required");
        }
        if body.chars().count() > 2_000 {
            return ToolResult::error("body must be 2000 characters or fewer");
        }

        let Some(reason) = input.get("reason").and_then(serde_json::Value::as_str) else {
            return ToolResult::error("reason is required");
        };
        let reason = safe_inline(reason.trim());
        if reason.is_empty() {
            return ToolResult::error("reason is required");
        }
        if reason.chars().count() > 240 {
            return ToolResult::error("reason must be 240 characters or fewer");
        }

        let scope = input
            .get("scope")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("project");
        let Some(scope) = MemoryScope::parse(scope) else {
            return ToolResult::error("scope must be project or global");
        };
        let memory_type = match input.get("type").and_then(serde_json::Value::as_str) {
            Some(raw) => {
                let Some(memory_type) = parse_memory_type(raw) else {
                    return ToolResult::error("type must be user, feedback, project, or reference");
                };
                memory_type
            }
            None => match scope {
                MemoryScope::Project => MemoryType::Project,
                MemoryScope::Global => MemoryType::User,
            },
        };

        let file_name = unique_memory_file_name(&title, &context.cwd, scope);
        let content = create_memory_file(&title, &reason, memory_type, body);
        let path = match save_memory(&file_name, &content, &context.cwd, scope) {
            Ok(path) => path,
            Err(error) => return ToolResult::error(format!("Failed to save memory: {error}")),
        };
        if let Err(error) = add_to_memory_index(
            &format!("- [{title}]({file_name}) - {reason}"),
            &context.cwd,
            scope,
        ) {
            return ToolResult::error(format!("Failed to update memory index: {error}"));
        }

        let scope_label = match scope {
            MemoryScope::Project => "project",
            MemoryScope::Global => "global",
        };
        ToolResult::text(format!(
            "Saved {scope_label} memory: {title}\n{}",
            path.display()
        ))
    }
}

fn search_memory(query: &str, scope: &str, max_results: usize, cwd: &Path) -> ToolResult {
    let dirs = memory_search_dirs(scope, cwd)
        .into_iter()
        .filter(|(dir_scope, dir)| {
            let _ = dir_scope;
            dir.exists()
        })
        .collect::<Vec<_>>();
    if dirs.is_empty() {
        return ToolResult::text(format!(
            "No Deeptide memory directories found for scope: {scope}"
        ));
    }

    let needle = query.to_ascii_lowercase();
    let mut hits = Vec::new();
    for (scope_label, dir) in dirs {
        collect_memory_hits(scope_label, &dir, &needle, &mut hits);
    }
    // De-dup the same logical memory that exists in both the canonical and the
    // legacy dir of a scope (during migration): key on (scope, file name) and
    // keep the first seen. `memory_search_dirs` lists the canonical dir first,
    // so canonical wins and the legacy copy is dropped — mirroring the loader's
    // canonical-over-legacy resolution. `retain` preserves that insertion order.
    let mut seen = std::collections::HashSet::new();
    hits.retain(|hit| {
        let name = hit
            .path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_owned();
        seen.insert((hit.scope, name))
    });
    // Deterministic order into the ranker (its index tie-break is then stable).
    hits.sort_by(|left, right| {
        left.scope
            .cmp(right.scope)
            .then_with(|| left.path.cmp(&right.path))
    });

    if hits.is_empty() {
        return ToolResult::text("No matching Deeptide memory files found.");
    }

    // Rank by BM25 + a mild recency nudge. Newest file → recency 1.0, oldest →
    // 0.0; single-file corpora get 1.0. The ranker drops zero-overlap docs, so
    // irrelevant files never surface — same guarantee as the old substring
    // filter, but now ordered by relevance instead of path.
    let (min_mtime, max_mtime) = hits.iter().fold((f64::MAX, f64::MIN), |(lo, hi), h| {
        (lo.min(h.mtime_secs), hi.max(h.mtime_secs))
    });
    let span = (max_mtime - min_mtime).max(1.0);
    let docs: Vec<crate::memory_rank::RankDoc> = hits
        .iter()
        .map(|h| crate::memory_rank::RankDoc {
            id: h.path.display().to_string(),
            text: h.text.clone(),
            recency: if max_mtime <= min_mtime {
                1.0
            } else {
                (h.mtime_secs - min_mtime) / span
            },
        })
        .collect();

    let ranked = crate::memory_rank::rank(query, &docs, max_results);
    if ranked.is_empty() {
        return ToolResult::text("No matching Deeptide memory files found.");
    }

    ToolResult::text(
        ranked
            .into_iter()
            .map(|(idx, _score)| render_memory_hit(hits[idx].clone()))
            .collect::<Vec<_>>()
            .join("\n\n"),
    )
}

#[derive(Debug, Clone, PartialEq)]
struct MemorySearchHit {
    scope: &'static str,
    path: PathBuf,
    title: String,
    description: Option<String>,
    matching_line: Option<String>,
    /// Full searchable text (title + description + body), for BM25 ranking.
    text: String,
    /// Modification time in seconds since the epoch, for the recency nudge.
    mtime_secs: f64,
}

fn memory_search_dirs(scope: &str, cwd: &Path) -> Vec<(&'static str, PathBuf)> {
    match scope {
        "project" => vec![
            ("project", MemorySystem::project_memory_dir(cwd)),
            ("project", MemorySystem::legacy_project_memory_dir(cwd)),
        ],
        "global" => vec![
            ("global", MemorySystem::global_memory_dir()),
            ("global", MemorySystem::legacy_global_memory_dir()),
        ],
        _ => vec![
            ("project", MemorySystem::project_memory_dir(cwd)),
            ("project", MemorySystem::legacy_project_memory_dir(cwd)),
            ("global", MemorySystem::global_memory_dir()),
            ("global", MemorySystem::legacy_global_memory_dir()),
        ],
    }
}

fn collect_memory_hits(
    scope: &'static str,
    dir: &Path,
    needle: &str,
    hits: &mut Vec<MemorySearchHit>,
) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.file_name().and_then(|name| name.to_str()) == Some("MEMORY.md") {
            continue;
        }
        if path.extension().and_then(|extension| extension.to_str()) != Some("md") {
            continue;
        }
        let Ok(content) = fs::read_to_string(&path) else {
            continue;
        };
        // No substring pre-filter: collect every candidate and let BM25 ranking
        // (in search_memory) decide relevance and order. Zero-overlap docs are
        // dropped by the ranker, so irrelevant files still never surface.
        let mtime_secs = entry
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0);
        let frontmatter = extract_frontmatter(&path);
        let title = frontmatter
            .iter()
            .find(|(key, _)| key == "name")
            .map(|(_, value)| value.to_owned())
            .or_else(|| first_markdown_heading(&content))
            .or_else(|| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .map(ToOwned::to_owned)
            })
            .unwrap_or_default();
        let description = frontmatter
            .iter()
            .find(|(key, _)| key == "description")
            .map(|(_, value)| value.to_owned());
        let matching_line = first_matching_memory_line(needle, &content);
        let text = format!(
            "{title}\n{}\n{content}",
            description.as_deref().unwrap_or("")
        );
        hits.push(MemorySearchHit {
            scope,
            path,
            title,
            description,
            matching_line,
            text,
            mtime_secs,
        });
    }
}

fn render_memory_hit(hit: MemorySearchHit) -> String {
    let mut lines = vec![
        format!("- {}", hit.path.display()),
        format!("  scope: {}", hit.scope),
        format!("  title: {}", hit.title),
    ];
    if let Some(description) = hit.description {
        lines.push(format!("  description: {description}"));
    }
    if let Some(matching_line) = hit.matching_line {
        lines.push(format!("  match: {}", truncate_chars(&matching_line, 180)));
    }
    lines.join("\n")
}

fn first_markdown_heading(content: &str) -> Option<String> {
    strip_frontmatter(content)
        .lines()
        .find_map(|line| line.trim().strip_prefix("# ").map(str::trim))
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
}

/// Pick the snippet line to show under a memory hit. BM25 ranks on individual
/// query terms, so requiring the *whole* query as a contiguous substring (the
/// old behaviour) left multi-word queries with no `match:` line even when the
/// file ranked first. Instead, return the line that contains the most query
/// terms (ties → the first such line), or `None` if no line shares any term —
/// keeping the snippet consistent with why the file ranked.
fn first_matching_memory_line(query: &str, content: &str) -> Option<String> {
    let terms = crate::memory_rank::tokenize(query);
    if terms.is_empty() {
        return None;
    }
    let mut best: Option<(usize, String)> = None;
    for line in strip_frontmatter(content).lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed == "---" {
            continue;
        }
        let lower = trimmed.to_ascii_lowercase();
        let hits = terms
            .iter()
            .filter(|term| lower.contains(term.as_str()))
            .count();
        if hits > 0 && best.as_ref().is_none_or(|(best_hits, _)| hits > *best_hits) {
            best = Some((hits, trimmed.to_owned()));
        }
    }
    best.map(|(_, line)| line)
}

fn parse_memory_type(raw: &str) -> Option<MemoryType> {
    match raw {
        "user" => Some(MemoryType::User),
        "feedback" => Some(MemoryType::Feedback),
        "project" => Some(MemoryType::Project),
        "reference" => Some(MemoryType::Reference),
        _ => None,
    }
}

fn unique_memory_file_name(title: &str, cwd: &Path, scope: MemoryScope) -> String {
    let dir = match scope {
        MemoryScope::Project => MemorySystem::project_memory_dir(cwd),
        MemoryScope::Global => MemorySystem::global_memory_dir(),
    };
    let base = memory_slug(title);
    let mut candidate = format!("{base}.md");
    let mut suffix = 2usize;
    while dir.join(&candidate).exists() {
        candidate = format!("{base}-{suffix}.md");
        suffix += 1;
    }
    candidate
}

fn memory_slug(title: &str) -> String {
    let words = title
        .to_ascii_lowercase()
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|part| !part.is_empty())
        .take(8)
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    if words.is_empty() {
        String::from("memory")
    } else {
        words.join("-")
    }
}

fn safe_inline(value: &str) -> String {
    value
        .replace(['\n', '\r'], " ")
        .replace('[', "(")
        .replace(']', ")")
        .replace(':', " -")
        .trim()
        .to_owned()
}

pub(crate) fn model_context_window(model: &str) -> u64 {
    let lower = model.to_ascii_lowercase();
    if lower.contains("deepseek-v4-flash-q4") {
        1_000_000
    } else if lower.contains("v4-flash-mlx-q4q8") || lower.contains("v4-flash-q4q8") {
        1_048_576
    } else if lower.contains("deepseek-v4-flash") {
        512_000
    } else if lower.contains("deepseek-v4") {
        1_000_000
    } else if lower.contains("qwen3-coder-next")
        || lower.contains("qwen3.6-35b-a3b")
        || lower.contains("qwen3.6")
    {
        262_144
    } else if lower.contains("glm-4.7-flash") {
        131_072
    } else if lower.contains("deepseek-v3") || lower.contains("deepseek-chat") {
        128_000
    } else if lower.contains("claude-3-opus")
        || lower.contains("claude-3-5")
        || lower.contains("claude-3.5")
    {
        200_000
    } else if lower.contains("gemini") {
        1_000_000
    } else {
        128_000
    }
}
