use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub const MEMORY_INDEX_FILE: &str = "MEMORY.md";
/// Caps on the cached memory **core** injected into the system prompt.
///
/// NOTE: `truncate_memory` runs *after* `load_memory_index` has expanded each
/// `MEMORY.md` index entry into the referenced file's full body, so these bound
/// the concatenated **bodies** (in lines / bytes) — they are not a count of
/// memory entries. In practice the resident core is only the first handful of
/// files; everything past the cap is reached on demand via `MemorySearch`.
pub const MEMORY_MAX_LINES: usize = 200;
pub const MEMORY_MAX_BYTES: usize = 25 * 1024;

pub struct MemorySystem;

impl MemorySystem {
    pub fn tide_config_dir() -> PathBuf {
        if let Some(raw) = env::var_os("TIDE_CONFIG_DIR") {
            let path = expand_tilde(&PathBuf::from(raw));
            if !path.as_os_str().is_empty() {
                return path;
            }
        }

        home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".config")
            .join("tide")
    }

    pub fn project_slug(cwd: impl AsRef<Path>) -> String {
        let normalized = cwd.as_ref().to_string_lossy().replace('\\', "/");
        let mut slug = normalized.replace('/', "-");
        while slug.starts_with('-') {
            slug.remove(0);
        }

        if slug.chars().count() > 64 {
            let hash = fnv1a_hex_prefix(&normalized, 8);
            slug = format!("{}-{hash}", slug.chars().take(55).collect::<String>());
        }

        slug
    }

    pub fn project_memory_dir(cwd: impl AsRef<Path>) -> PathBuf {
        Self::tide_config_dir()
            .join("projects")
            .join(Self::project_slug(cwd))
            .join("memory")
    }

    pub fn legacy_project_memory_dir(cwd: impl AsRef<Path>) -> PathBuf {
        cwd.as_ref().join(".deeptide").join("memory")
    }

    pub fn global_memory_dir() -> PathBuf {
        Self::tide_config_dir().join("memory")
    }

    pub fn legacy_global_memory_dir() -> PathBuf {
        home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".deeptide")
            .join("memory")
    }

    pub fn project_memory_index(cwd: impl AsRef<Path>) -> PathBuf {
        let canonical = Self::project_memory_dir(cwd.as_ref()).join(MEMORY_INDEX_FILE);
        if canonical.exists() {
            return canonical;
        }

        let legacy = Self::legacy_project_memory_dir(cwd.as_ref()).join(MEMORY_INDEX_FILE);
        if legacy.exists() {
            return legacy;
        }

        canonical
    }

    pub fn global_memory_index() -> PathBuf {
        let canonical = Self::global_memory_dir().join(MEMORY_INDEX_FILE);
        if canonical.exists() {
            return canonical;
        }

        let legacy = Self::legacy_global_memory_dir().join(MEMORY_INDEX_FILE);
        if legacy.exists() {
            return legacy;
        }

        canonical
    }

    pub fn load_memory_prompt(cwd: impl AsRef<Path>) -> String {
        let mut parts = Vec::new();

        if let Some(content) = load_memory_index(&Self::global_memory_index()) {
            parts.push(format!("# auto memory\n{content}"));
        }

        if let Some(content) = load_memory_index(&Self::project_memory_index(cwd)) {
            parts.push(format!("# project memory\n{content}"));
        }

        truncate_memory(&parts.join("\n\n"))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryListBlock {
    pub label: &'static str,
    pub index_path: PathBuf,
    pub exists: bool,
    pub files: Vec<MemoryFileSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryFileSummary {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryEntry {
    pub scope: &'static str,
    pub file_path: PathBuf,
    pub index_path: PathBuf,
    pub frontmatter: Vec<(String, String)>,
}

impl MemoryEntry {
    pub fn frontmatter_value(&self, key: &str) -> Option<&str> {
        self.frontmatter
            .iter()
            .find(|(stored, _)| stored == key)
            .map(|(_, value)| value.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemoryResolveResult {
    Found(MemoryEntry),
    Missing(String),
    Ambiguous(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryScope {
    Project,
    Global,
}

impl MemoryScope {
    pub fn parse(raw: &str) -> Option<Self> {
        match raw.to_ascii_lowercase().as_str() {
            "project" => Some(Self::Project),
            "global" => Some(Self::Global),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryType {
    User,
    Feedback,
    Project,
    Reference,
}

impl MemoryType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Feedback => "feedback",
            Self::Project => "project",
            Self::Reference => "reference",
        }
    }
}

pub fn create_memory_file(
    name: &str,
    description: &str,
    memory_type: MemoryType,
    body: &str,
) -> String {
    format!(
        "---\nname: {name}\ndescription: {description}\ntype: {}\n---\n\n{body}",
        memory_type.as_str()
    )
}

pub fn save_memory(
    file_name: &str,
    content: &str,
    cwd: impl AsRef<Path>,
    scope: MemoryScope,
) -> io::Result<PathBuf> {
    let dir = match scope {
        MemoryScope::Project => MemorySystem::project_memory_dir(cwd.as_ref()),
        MemoryScope::Global => MemorySystem::global_memory_dir(),
    };
    fs::create_dir_all(&dir)?;
    let file_path = dir.join(file_name);
    fs::write(&file_path, content)?;
    Ok(file_path)
}

pub fn add_to_memory_index(
    entry: &str,
    cwd: impl AsRef<Path>,
    scope: MemoryScope,
) -> io::Result<PathBuf> {
    let index_path = match scope {
        MemoryScope::Project => MemorySystem::project_memory_index(cwd.as_ref()),
        MemoryScope::Global => MemorySystem::global_memory_index(),
    };
    if let Some(dir) = index_path.parent() {
        fs::create_dir_all(dir)?;
    }
    append_line(&index_path, entry)?;
    Ok(index_path)
}

pub fn remember_project_note(
    body: &str,
    cwd: impl AsRef<Path>,
    timestamp: &str,
) -> io::Result<PathBuf> {
    let body = body.trim();
    let index_path = MemorySystem::project_memory_dir(cwd.as_ref()).join(MEMORY_INDEX_FILE);
    if let Some(dir) = index_path.parent() {
        fs::create_dir_all(dir)?;
    }

    let line = format!("- [{timestamp}] {body}");
    if index_path.exists() {
        append_line(&index_path, &line)?;
    } else {
        fs::write(
            &index_path,
            format!("# Deeptide project memory\n\n{line}\n"),
        )?;
    }
    Ok(index_path)
}

pub fn list_memory_blocks(cwd: impl AsRef<Path>) -> Vec<MemoryListBlock> {
    let project_index = MemorySystem::project_memory_index(cwd.as_ref());
    let global_index = MemorySystem::global_memory_index();
    vec![
        MemoryListBlock {
            label: "project",
            exists: project_index.exists(),
            files: list_memory_files(project_index.parent().unwrap_or_else(|| Path::new("."))),
            index_path: project_index,
        },
        MemoryListBlock {
            label: "global",
            exists: global_index.exists(),
            files: list_memory_files(global_index.parent().unwrap_or_else(|| Path::new("."))),
            index_path: global_index,
        },
    ]
}

pub fn resolve_memory(
    target: &str,
    explicit_scope: Option<MemoryScope>,
    cwd: impl AsRef<Path>,
) -> MemoryResolveResult {
    let scopes: Vec<(&'static str, PathBuf, PathBuf)> = match explicit_scope {
        Some(MemoryScope::Project) => vec![(
            "project",
            MemorySystem::project_memory_dir(cwd.as_ref()),
            MemorySystem::project_memory_index(cwd.as_ref()),
        )],
        Some(MemoryScope::Global) => vec![(
            "global",
            MemorySystem::global_memory_dir(),
            MemorySystem::global_memory_index(),
        )],
        None => vec![
            (
                "project",
                MemorySystem::project_memory_dir(cwd.as_ref()),
                MemorySystem::project_memory_index(cwd.as_ref()),
            ),
            (
                "global",
                MemorySystem::global_memory_dir(),
                MemorySystem::global_memory_index(),
            ),
        ],
    };

    let needle = target.to_ascii_lowercase();
    let mut matches = Vec::new();

    for (scope, dir, index) in scopes {
        let Ok(entries) = fs::read_dir(dir) else {
            continue;
        };

        for entry in entries.flatten() {
            let file_path = entry.path();
            if !is_memory_markdown_file(&file_path) {
                continue;
            }

            let frontmatter = extract_frontmatter(&file_path);
            let filename = file_path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or_default()
                .to_ascii_lowercase();
            let stem = file_path
                .file_stem()
                .and_then(|name| name.to_str())
                .unwrap_or_default()
                .to_ascii_lowercase();
            let title = frontmatter
                .iter()
                .find(|(key, _)| key == "name")
                .map(|(_, value)| value.to_ascii_lowercase());

            if filename == needle
                || stem == needle
                || format!("{stem}.md") == needle
                || title.as_deref() == Some(needle.as_str())
            {
                matches.push(MemoryEntry {
                    scope,
                    file_path,
                    index_path: index.clone(),
                    frontmatter,
                });
            }
        }
    }

    match matches.len() {
        1 => MemoryResolveResult::Found(matches.remove(0)),
        count if count > 1 => {
            let choices = matches
                .iter()
                .map(|entry| {
                    format!(
                        "{}: {}",
                        entry.scope,
                        entry
                            .file_path
                            .file_name()
                            .and_then(|name| name.to_str())
                            .unwrap_or_default()
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");
            MemoryResolveResult::Ambiguous(format!(
                "Memory name is ambiguous: {choices}. Add project or global."
            ))
        }
        _ => MemoryResolveResult::Missing(format!("Memory not found: {target}")),
    }
}

pub fn render_memory(entry: &MemoryEntry) -> String {
    let raw = fs::read_to_string(&entry.file_path).unwrap_or_default();
    let title = entry
        .frontmatter_value("name")
        .map(ToOwned::to_owned)
        .or_else(|| {
            entry
                .file_path
                .file_name()
                .and_then(|name| name.to_str())
                .map(ToOwned::to_owned)
        })
        .unwrap_or_default();

    let mut lines = vec![
        title,
        format!("  scope: {}", entry.scope),
        format!("  path: {}", tildify(&entry.file_path)),
    ];

    if let Some(kind) = entry.frontmatter_value("type") {
        lines.push(format!("  type: {kind}"));
    }
    if let Some(description) = entry.frontmatter_value("description") {
        lines.push(format!("  description: {description}"));
    }
    lines.push(String::new());
    lines.push(truncate_for_display(&strip_frontmatter(&raw)));

    lines.join("\n")
}

pub fn delete_memory(entry: &MemoryEntry) -> io::Result<()> {
    fs::remove_file(&entry.file_path)?;

    if !entry.index_path.exists() {
        return Ok(());
    }

    let index = fs::read_to_string(&entry.index_path)?;
    let file_name = entry
        .file_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    let kept = index
        .split('\n')
        .filter(|line| !line.contains(&format!("]({file_name})")))
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(&entry.index_path, kept)
}

fn append_line(path: &Path, line: &str) -> io::Result<()> {
    use std::io::Write;

    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    writeln!(file, "{line}")
}

pub fn extract_frontmatter(path: &Path) -> Vec<(String, String)> {
    let Ok(content) = fs::read_to_string(path) else {
        return Vec::new();
    };

    let mut in_frontmatter = false;
    let mut fields = Vec::new();
    for line in content.lines().take(30) {
        if line == "---" {
            if in_frontmatter {
                break;
            }
            in_frontmatter = true;
            continue;
        }

        if in_frontmatter && let Some((key, value)) = line.split_once(':') {
            let key = key.trim().to_ascii_lowercase();
            let value = value.trim().to_owned();
            if !key.is_empty() && !value.is_empty() {
                fields.push((key, value));
            }
        }
    }

    fields
}

pub fn strip_frontmatter(raw: &str) -> String {
    let mut lines = raw.split('\n').collect::<Vec<_>>();
    if lines.first().copied() != Some("---") {
        return raw.trim().to_owned();
    }

    lines.remove(0);
    while !lines.is_empty() {
        let line = lines.remove(0);
        if line == "---" {
            break;
        }
    }

    lines.join("\n").trim().to_owned()
}

/// Cap the memory block injected into the system prompt to a deterministic,
/// **line-aligned**, byte-stable prefix.
///
/// Cutting only on line boundaries (never mid-line) is what makes the kept
/// block byte-identical as long as the leading lines are unchanged — and a
/// byte-identical prefix is what lets DeepSeek's automatic prefix cache hit on
/// it every turn. `benchmarks/cache_memory_bench.py` measures this against the
/// live API: a stable prefix sustains ~95% cache-hit (≈10× cheaper input)
/// across turns, versus 0% the moment the prefix shifts. So appending new
/// memory entries (which land at the end of the index) must never perturb the
/// retained head — this function guarantees that.
///
/// Overflow is **relocated, not lost**: the notice always points the agent at
/// `MemorySearch`, which retrieves the full memory files on demand (two-tier
/// memory — a cached core plus an on-demand long tail), and reports how many
/// lines were dropped from the core. The count is in *lines* rather than
/// entries because the only production caller ([`load_memory_index`]) expands
/// each index entry into the referenced file's body before this runs, so by
/// the time we truncate there are no entry boundaries left to count — the cut
/// lands inside concatenated bodies.
pub fn truncate_memory(text: &str) -> String {
    let lines = text.split('\n').collect::<Vec<_>>();

    let mut kept: Vec<&str> = Vec::new();
    let mut bytes = 0usize;
    let mut cut_at: Option<usize> = None;
    for (i, line) in lines.iter().enumerate() {
        // +1 for the '\n' that rejoins this line to the previous one.
        let added = if kept.is_empty() {
            line.len()
        } else {
            line.len() + 1
        };
        // Always retain at least the first line, even if it alone exceeds the
        // byte cap — cutting on the line boundary keeps the prefix cache-stable,
        // and dropping the whole block to leave only the notice would lose more
        // than it saves. (The old mid-line byte cut produced a non-line-aligned,
        // cache-busting prefix; we accept a slightly over-cap single line over
        // that.)
        if !kept.is_empty() && (kept.len() >= MEMORY_MAX_LINES || bytes + added > MEMORY_MAX_BYTES)
        {
            cut_at = Some(i);
            break;
        }
        bytes += added;
        kept.push(line);
    }

    let mut out = kept.join("\n");

    if let Some(i) = cut_at {
        // How many non-empty lines were cut from the cached core. They aren't
        // lost — they still live in the memory files MemorySearch retrieves.
        let dropped = lines[i..].iter().filter(|l| !l.trim().is_empty()).count();
        let by_lines = kept.len() >= MEMORY_MAX_LINES;
        let why = if by_lines {
            format!("{MEMORY_MAX_LINES} lines")
        } else {
            format!("{} KB", MEMORY_MAX_BYTES / 1024)
        };
        // Always point at MemorySearch; include the dropped-line count only when
        // real content was cut (a cut through trailing blank lines drops 0 and
        // "0 more lines" would read oddly).
        let tail = if dropped > 0 {
            format!(
                "{dropped} more line{} available via MemorySearch",
                if dropped == 1 { "" } else { "s" }
            )
        } else {
            String::from("more available via MemorySearch")
        };
        out.push_str(&format!("\n\n...(memory core capped at {why}; {tail})..."));
    }

    out
}

fn list_memory_files(dir: &Path) -> Vec<MemoryFileSummary> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };

    let mut files = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| is_memory_markdown_file(path))
        .map(|path| {
            let description = extract_frontmatter(&path)
                .into_iter()
                .find(|(key, _)| key == "description")
                .map(|(_, value)| value);
            MemoryFileSummary {
                name: path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or_default()
                    .to_owned(),
                description,
            }
        })
        .collect::<Vec<_>>();
    files.sort_by(|left, right| left.name.cmp(&right.name));
    files
}

fn load_memory_index(path: &Path) -> Option<String> {
    let content = fs::read_to_string(path).ok()?;
    if content.trim().is_empty() {
        return None;
    }

    let dir = path.parent()?;
    let mut full_content = String::new();
    for raw in content.split('\n') {
        let trimmed = raw.trim();
        if trimmed.starts_with("- [")
            && trimmed.contains("](")
            && trimmed.contains(".md")
            && let Some(start) = trimmed.find("](")
            && let Some(end) = trimmed[start + 2..].find(')')
        {
            let filename = &trimmed[start + 2..start + 2 + end];
            if let Ok(body) = fs::read_to_string(dir.join(filename)) {
                full_content.push_str(&body);
                full_content.push('\n');
            }
        } else if !trimmed.is_empty() {
            full_content.push_str(trimmed);
            full_content.push('\n');
        }
    }

    if full_content.is_empty() {
        None
    } else {
        Some(full_content)
    }
}

fn truncate_for_display(text: &str) -> String {
    let lines = text.split('\n').collect::<Vec<_>>();
    if lines.len() <= 120 && text.len() <= 16 * 1024 {
        return text.to_owned();
    }

    format!(
        "{}\n\n...truncated for display; open the path for full content.",
        lines[..120].join("\n")
    )
}

fn is_memory_markdown_file(path: &Path) -> bool {
    path.extension().and_then(|ext| ext.to_str()) == Some("md")
        && path.file_name().and_then(|name| name.to_str()) != Some(MEMORY_INDEX_FILE)
}

fn expand_tilde(path: &Path) -> PathBuf {
    let raw = path.to_string_lossy();
    if raw == "~" {
        return home_dir().unwrap_or_else(|| path.to_path_buf());
    }
    if let Some(rest) = raw.strip_prefix("~/") {
        return home_dir()
            .map(|home| home.join(rest))
            .unwrap_or_else(|| path.to_path_buf());
    }
    path.to_path_buf()
}

fn home_dir() -> Option<PathBuf> {
    env::var_os("HOME")
        .or_else(|| env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

fn tildify(path: &Path) -> String {
    let raw = path.to_string_lossy();
    let Some(home) = home_dir() else {
        return raw.into_owned();
    };
    let home = home.to_string_lossy();
    if raw == home {
        "~".to_owned()
    } else if let Some(rest) = raw.strip_prefix(&format!("{home}/")) {
        format!("~/{rest}")
    } else {
        raw.into_owned()
    }
}

fn fnv1a_hex_prefix(value: &str, length: usize) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:x}").chars().take(length).collect()
}

#[cfg(test)]
mod cache_alignment_tests {
    use super::*;

    /// Build a synthetic MEMORY.md index with `n` entries.
    fn index_with(n: usize) -> String {
        let mut s = String::from("# memory\n");
        for i in 0..n {
            s.push_str(&format!("- [Entry {i}](file-{i}.md) — hook number {i}\n"));
        }
        s
    }

    #[test]
    fn short_memory_is_unchanged() {
        let text = "- [A](a.md) — x\n- [B](b.md) — y";
        assert_eq!(truncate_memory(text), text);
    }

    #[test]
    fn truncation_cuts_on_line_boundaries_never_mid_line() {
        // Far more than the byte cap so a cut is forced.
        let text = index_with(5000);
        let out = truncate_memory(&text);
        let body = out.split("\n\n...").next().expect("body before notice");
        // Every retained line must be an exact, complete original line.
        let originals: std::collections::HashSet<&str> = text.lines().collect();
        for line in body.lines() {
            assert!(
                originals.contains(line),
                "partial / corrupted line retained: {line:?}"
            );
        }
    }

    #[test]
    fn truncation_is_deterministic() {
        let text = index_with(5000);
        assert_eq!(truncate_memory(&text), truncate_memory(&text));
    }

    /// The cache property: appending new entries at the END of the index must
    /// not change the retained core, so DeepSeek's prefix cache keeps hitting.
    #[test]
    fn retained_core_is_prefix_stable_when_entries_appended() {
        let small = index_with(5000);
        let larger = index_with(7000); // same leading lines, more appended
        let core_small = truncate_memory(&small);
        let core_larger = truncate_memory(&larger);
        let body_small = core_small
            .split("\n\n...")
            .next()
            .expect("body before notice");
        let body_larger = core_larger
            .split("\n\n...")
            .next()
            .expect("body before notice");
        assert_eq!(
            body_small, body_larger,
            "retained core shifted when entries were appended — would bust the prefix cache"
        );
    }

    #[test]
    fn overflow_points_at_memory_search_with_a_count() {
        let out = truncate_memory(&index_with(5000));
        assert!(
            out.contains("via MemorySearch"),
            "missing retrieval pointer: {out:?}"
        );
        assert!(
            out.contains("more line"),
            "missing dropped-line count: {out:?}"
        );
    }

    /// Regression guard for the real production path: `load_memory_index`
    /// expands each `- [..](x.md)` index line into the referenced file's *body*
    /// before `truncate_memory` runs, so the cut lands inside concatenated
    /// bodies — not on index lines. This drives that exact path into overflow
    /// and asserts the MemorySearch relocation pointer still fires (it used to
    /// be keyed on index-line shape that never reaches `truncate_memory`, so it
    /// silently never fired in production).
    #[test]
    fn real_index_expansion_path_relocates_overflow() {
        let dir = tempfile::tempdir().expect("tempdir");
        let index = dir.path().join(MEMORY_INDEX_FILE);
        // Each referenced file has a body large enough that the concatenation
        // overflows the byte cap.
        let big_body = "lorem ipsum dolor sit amet consectetur. ".repeat(800);
        let mut idx = String::from("# memory\n");
        for i in 0..3 {
            let fname = format!("mem-{i}.md");
            std::fs::write(dir.path().join(&fname), &big_body).expect("write body");
            idx.push_str(&format!("- [Entry {i}]({fname}) — hook {i}\n"));
        }
        std::fs::write(&index, idx).expect("write index");

        let expanded = load_memory_index(&index).expect("index loads + expands");
        let out = truncate_memory(&expanded);
        assert!(
            out.contains("via MemorySearch"),
            "real expansion path must relocate overflow to MemorySearch: {}",
            &out[out.len().saturating_sub(200)..]
        );
    }

    #[test]
    fn line_cap_is_respected() {
        let out = truncate_memory(&index_with(5000));
        let body = out.split("\n\n...").next().expect("body before notice");
        assert!(body.lines().count() <= MEMORY_MAX_LINES);
    }

    /// A single line longer than the byte cap is kept whole (line-aligned),
    /// not dropped to leave only the notice — otherwise an over-long leading
    /// line would erase the entire block.
    #[test]
    fn over_long_single_line_is_kept_whole() {
        let huge = "x".repeat(MEMORY_MAX_BYTES * 2);
        let out = truncate_memory(&huge);
        let body = out.split("\n\n...").next().expect("body before notice");
        assert_eq!(body, huge, "over-long single line must be retained whole");
    }
}
