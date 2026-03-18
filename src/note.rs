/// 笔记结构体、frontmatter 解析与生成
use anyhow::{Context, Result};
use chrono::{DateTime, Local};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::utils;

/// 笔记结构体
#[derive(Clone, Debug)]
pub struct Note {
    pub path: PathBuf,
    pub title: String,
    pub category: String,
    pub tags: Vec<String>,
    pub created: String,
    #[allow(dead_code)]
    pub updated: String,
    #[allow(dead_code)]
    pub merged_from: Vec<String>,
    pub body: String,
    pub mtime: SystemTime,
    pub is_deleted: bool,
}

impl Note {
    /// 从文件读取笔记，解析 frontmatter
    pub fn from_file(path: &Path) -> Result<Note> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("无法读取文件: {}", path.display()))?;

        let metadata = std::fs::metadata(path)?;
        let mtime = metadata.modified().unwrap_or(SystemTime::now());

        let title = path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let (category, tags, created, updated, merged_from) = parse_frontmatter(&content);
        let body = extract_body(&content).to_string();

        Ok(Note {
            path: path.to_path_buf(),
            title,
            category,
            tags,
            created,
            updated,
            merged_from,
            body,
            mtime,
            is_deleted: false,
        })
    }
}

/// 解析 frontmatter，返回 (category, tags, created, updated, merged_from)
fn parse_frontmatter(content: &str) -> (String, Vec<String>, String, String, Vec<String>) {
    let mut category = String::new();
    let mut tags: Vec<String> = Vec::new();
    let mut created = String::new();
    let mut updated = String::new();
    let mut merged_from: Vec<String> = Vec::new();

    let lines: Vec<&str> = content.lines().collect();
    if lines.is_empty() || lines[0].trim() != "---" {
        return (category, tags, created, updated, merged_from);
    }

    let mut in_merged_from = false;
    for line in &lines[1..] {
        let trimmed = line.trim();
        if trimmed == "---" {
            break;
        }

        // 处理 merged_from 列表项
        if in_merged_from {
            if let Some(item) = trimmed.strip_prefix("- ") {
                merged_from.push(item.trim().to_string());
                continue;
            } else {
                in_merged_from = false;
            }
        }

        if let Some(val) = trimmed.strip_prefix("category:") {
            category = val.trim().to_string();
        } else if let Some(val) = trimmed.strip_prefix("tags:") {
            tags = parse_tags_value(val.trim());
        } else if let Some(val) = trimmed.strip_prefix("created:") {
            created = val.trim().trim_matches('"').to_string();
        } else if let Some(val) = trimmed.strip_prefix("updated:") {
            updated = val.trim().trim_matches('"').to_string();
        } else if trimmed.starts_with("merged_from:") {
            in_merged_from = true;
        }
    }

    (category, tags, created, updated, merged_from)
}

/// 解析 tags 值，支持 [tag1, tag2] 和空 [] 格式
fn parse_tags_value(val: &str) -> Vec<String> {
    let trimmed = val.trim();
    if trimmed == "[]" || trimmed.is_empty() {
        return Vec::new();
    }

    // 去掉方括号
    let inner = trimmed
        .strip_prefix('[')
        .and_then(|s| s.strip_suffix(']'))
        .unwrap_or(trimmed);

    inner
        .split(',')
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .collect()
}

/// 从文件内容中提取正文（跳过 frontmatter）
pub fn extract_body(content: &str) -> &str {
    let bytes = content.as_bytes();
    if !content.starts_with("---") {
        return content;
    }

    // 找到第一行 "---" 之后的第二个 "---"
    let mut dash_count = 0;
    let mut pos = 0;
    for (i, line) in content.lines().enumerate() {
        if line.trim() == "---" {
            dash_count += 1;
            if dash_count == 2 {
                // 跳过第二个 "---" 行及其后的换行符
                pos = content
                    .lines()
                    .take(i + 1)
                    .map(|l| l.len() + 1)
                    .sum::<usize>();
                // 处理可能的 \r\n
                if pos <= bytes.len() {
                    // 跳过紧接的空行
                    let rest = &content[pos.min(content.len())..];
                    if rest.starts_with('\n') {
                        pos += 1;
                    } else if rest.starts_with("\r\n") {
                        pos += 2;
                    }
                }
                break;
            }
        }
    }

    if dash_count < 2 {
        return content;
    }

    &content[pos.min(content.len())..]
}

/// 生成 frontmatter 字符串
pub fn generate_frontmatter(
    category: &str,
    tags: &[String],
    now: &DateTime<Local>,
    merged_from: Option<&[String]>,
) -> String {
    let time_str = utils::format_datetime(now);
    let tags_str = if tags.is_empty() {
        "[]".to_string()
    } else {
        format!("[{}]", tags.join(", "))
    };

    let mut fm = format!(
        "---\ncategory: {category}\ntags: {tags_str}\ncreated: \"{time_str}\"\nupdated: \"{time_str}\"\n"
    );

    if let Some(sources) = merged_from {
        fm.push_str("merged_from:\n");
        for src in sources {
            fm.push_str(&format!("  - {src}\n"));
        }
    }

    fm.push_str("---\n");
    fm
}

/// 替换 frontmatter 中指定字段的值
pub fn update_frontmatter_field(content: &str, key: &str, value: &str) -> String {
    let mut result = String::new();
    let mut in_frontmatter = false;
    let mut dash_count = 0;
    let mut replaced = false;

    for line in content.lines() {
        if line.trim() == "---" {
            dash_count += 1;
            if dash_count == 1 {
                in_frontmatter = true;
            } else if dash_count == 2 {
                in_frontmatter = false;
            }
            result.push_str(line);
            result.push('\n');
            continue;
        }

        if in_frontmatter && !replaced {
            let prefix = format!("{key}:");
            if line.starts_with(&prefix) || line.trim().starts_with(&prefix) {
                let indent = &line[..line.len() - line.trim_start().len()];
                result.push_str(&format!("{indent}{key}: \"{value}\"\n"));
                replaced = true;
                continue;
            }
        }

        result.push_str(line);
        result.push('\n');
    }

    // 移除末尾多余换行
    if result.ends_with('\n') && !content.ends_with('\n') {
        result.pop();
    }

    result
}

/// 加载指定目录下的所有笔记
pub fn load_notes_from_dir(dir: &Path) -> Result<Vec<Note>> {
    let mut notes = Vec::new();
    if !dir.exists() {
        return Ok(notes);
    }

    let entries =
        std::fs::read_dir(dir).with_context(|| format!("无法读取目录: {}", dir.display()))?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() && path.extension().is_some_and(|e| e == "md") {
            match Note::from_file(&path) {
                Ok(note) => notes.push(note),
                Err(e) => {
                    eprintln!("警告: 无法解析 {}: {}", path.display(), e);
                }
            }
        }
    }

    Ok(notes)
}

/// 加载所有笔记（可选包含 deleted/）
pub fn load_notes(include_deleted: bool) -> Result<Vec<Note>> {
    let notes_dir = crate::config::notes_dir()?;
    let mut notes = load_notes_from_dir(&notes_dir)?;

    if include_deleted {
        let deleted_dir = notes_dir.join("deleted");
        let deleted = load_notes_from_dir(&deleted_dir)?;
        for mut note in deleted {
            note.is_deleted = true;
            notes.push(note);
        }
    }

    // 按 mtime 倒序排序
    notes.sort_by(|a, b| b.mtime.cmp(&a.mtime));
    Ok(notes)
}

/// 通过序号、精确文件名或模糊匹配定位笔记
pub enum MatchResult {
    /// 精确匹配到一条
    Single(Note),
    /// 模糊匹配到多条
    Multiple(Vec<Note>),
    /// 没有匹配
    None,
}

/// 解析参数，定位笔记（支持序号、精确文件名、模糊匹配）
pub fn resolve_note(arg: &str, notes: &[Note]) -> MatchResult {
    // 1. 尝试解析为序号
    if let Ok(n) = arg.parse::<usize>() {
        if n == 0 || n > notes.len() {
            return MatchResult::None;
        }
        return MatchResult::Single(notes[n - 1].clone());
    }

    // 2. 尝试精确匹配文件名
    let with_md = if arg.ends_with(".md") {
        arg.to_string()
    } else {
        format!("{arg}.md")
    };

    for note in notes {
        let fname = note.path.file_name().unwrap_or_default().to_string_lossy();
        if fname == with_md {
            return MatchResult::Single(note.clone());
        }
    }

    // 3. 模糊匹配（子串匹配标题，大小写不敏感）
    let lower_arg = arg.to_lowercase();
    let matched: Vec<Note> = notes
        .iter()
        .filter(|n| n.title.to_lowercase().contains(&lower_arg))
        .cloned()
        .collect();

    match matched.len() {
        0 => MatchResult::None,
        1 => {
            // SAFETY: len() == 1 保证 next() 一定有值
            if let Some(n) = matched.into_iter().next() {
                MatchResult::Single(n)
            } else {
                MatchResult::None
            }
        }
        _ => MatchResult::Multiple(matched),
    }
}

/// 解析参数，定位笔记（仅支持精确文件名和模糊匹配，不支持序号）
/// 用于 note rm
pub fn resolve_note_no_index(arg: &str, notes: &[Note]) -> MatchResult {
    // 1. 尝试精确匹配文件名
    let with_md = if arg.ends_with(".md") {
        arg.to_string()
    } else {
        format!("{arg}.md")
    };

    for note in notes {
        let fname = note.path.file_name().unwrap_or_default().to_string_lossy();
        if fname == with_md {
            return MatchResult::Single(note.clone());
        }
    }

    // 2. 模糊匹配
    let lower_arg = arg.to_lowercase();
    let matched: Vec<Note> = notes
        .iter()
        .filter(|n| n.title.to_lowercase().contains(&lower_arg))
        .cloned()
        .collect();

    match matched.len() {
        0 => MatchResult::None,
        1 => {
            // SAFETY: len() == 1 保证 next() 一定有值
            if let Some(n) = matched.into_iter().next() {
                MatchResult::Single(n)
            } else {
                MatchResult::None
            }
        }
        _ => MatchResult::Multiple(matched),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::time::SystemTime;

    /// 创建测试用 Note
    fn make_note(filename: &str, title: &str, category: &str, tags: &[&str]) -> Note {
        Note {
            path: PathBuf::from(format!("/tmp/{filename}")),
            title: title.to_string(),
            category: category.to_string(),
            tags: tags.iter().map(|t| t.to_string()).collect(),
            created: "2026-03-13 周五 15:00".to_string(),
            updated: String::new(),
            merged_from: Vec::new(),
            body: String::new(),
            mtime: SystemTime::now(),
            is_deleted: false,
        }
    }

    // ────────── parse_frontmatter ──────────

    #[test]
    fn parse_frontmatter_basic() {
        let content = "---\ncategory: 技术\ntags: [rust, cli]\ncreated: \"2026-03-13 周五 15:00\"\nupdated: \"2026-03-13 周五 16:00\"\n---\n正文内容";
        let (cat, tags, created, updated, merged) = parse_frontmatter(content);
        assert_eq!(cat, "技术");
        assert_eq!(tags, vec!["rust", "cli"]);
        assert_eq!(created, "2026-03-13 周五 15:00");
        assert_eq!(updated, "2026-03-13 周五 16:00");
        assert!(merged.is_empty());
    }

    #[test]
    fn parse_frontmatter_empty_content() {
        let (cat, tags, created, updated, merged) = parse_frontmatter("");
        assert!(cat.is_empty());
        assert!(tags.is_empty());
        assert!(created.is_empty());
        assert!(updated.is_empty());
        assert!(merged.is_empty());
    }

    #[test]
    fn parse_frontmatter_no_delimiter() {
        let content = "这不是 frontmatter\n只是普通文本";
        let (cat, tags, _, _, _) = parse_frontmatter(content);
        assert!(cat.is_empty());
        assert!(tags.is_empty());
    }

    #[test]
    fn parse_frontmatter_missing_fields() {
        let content = "---\ncategory: 日记\n---\n";
        let (cat, tags, created, _, _) = parse_frontmatter(content);
        assert_eq!(cat, "日记");
        assert!(tags.is_empty());
        assert!(created.is_empty());
    }

    #[test]
    fn parse_frontmatter_empty_tags() {
        let content = "---\ntags: []\n---\n";
        let (_, tags, _, _, _) = parse_frontmatter(content);
        assert!(tags.is_empty());
    }

    #[test]
    fn parse_frontmatter_chinese_tags() {
        let content = "---\ntags: [学习笔记, 算法, Rust入门]\n---\n";
        let (_, tags, _, _, _) = parse_frontmatter(content);
        assert_eq!(tags, vec!["学习笔记", "算法", "Rust入门"]);
    }

    #[test]
    fn parse_frontmatter_with_merged_from() {
        let content = "---\ncategory: 技术\ntags: []\nmerged_from:\n  - old_note1.md\n  - old_note2.md\n---\n";
        let (cat, _, _, _, merged) = parse_frontmatter(content);
        assert_eq!(cat, "技术");
        assert_eq!(merged, vec!["old_note1.md", "old_note2.md"]);
    }

    #[test]
    fn parse_frontmatter_only_opening_delimiter() {
        let content = "---\ncategory: 技术\n没有结束分隔符";
        let (cat, _, _, _, _) = parse_frontmatter(content);
        assert_eq!(cat, "技术");
    }

    #[test]
    fn parse_frontmatter_category_with_spaces() {
        let content = "---\ncategory:   工作笔记  \n---\n";
        let (cat, _, _, _, _) = parse_frontmatter(content);
        assert_eq!(cat, "工作笔记");
    }

    // ────────── parse_tags_value ──────────

    #[test]
    fn parse_tags_value_empty_brackets() {
        assert!(parse_tags_value("[]").is_empty());
    }

    #[test]
    fn parse_tags_value_empty_string() {
        assert!(parse_tags_value("").is_empty());
    }

    #[test]
    fn parse_tags_value_single_tag() {
        assert_eq!(parse_tags_value("[rust]"), vec!["rust"]);
    }

    #[test]
    fn parse_tags_value_multiple_tags() {
        assert_eq!(parse_tags_value("[rust, cli, 中文]"), vec!["rust", "cli", "中文"]);
    }

    #[test]
    fn parse_tags_value_extra_spaces() {
        assert_eq!(parse_tags_value("[  a ,  b  , c ]"), vec!["a", "b", "c"]);
    }

    #[test]
    fn parse_tags_value_no_brackets() {
        // 没有方括号时按逗号分隔
        assert_eq!(parse_tags_value("rust, cli"), vec!["rust", "cli"]);
    }

    // ────────── extract_body ──────────

    #[test]
    fn extract_body_with_frontmatter() {
        let content = "---\ncategory: 技术\n---\n这是正文\n第二行";
        assert_eq!(extract_body(content), "这是正文\n第二行");
    }

    #[test]
    fn extract_body_no_frontmatter() {
        let content = "没有 frontmatter 的纯文本";
        assert_eq!(extract_body(content), content);
    }

    #[test]
    fn extract_body_empty() {
        assert_eq!(extract_body(""), "");
    }

    #[test]
    fn extract_body_only_frontmatter() {
        let content = "---\ncategory: 技术\n---\n";
        assert_eq!(extract_body(content), "");
    }

    #[test]
    fn extract_body_frontmatter_with_blank_line_after() {
        let content = "---\ncategory: 技术\n---\n\n正文开始";
        assert_eq!(extract_body(content), "正文开始");
    }

    #[test]
    fn extract_body_incomplete_frontmatter() {
        let content = "---\ncategory: 技术\n没有结束";
        // 缺少第二个 "---"，返回完整内容
        assert_eq!(extract_body(content), content);
    }

    #[test]
    fn extract_body_content_with_triple_dash_in_body() {
        // 正文中包含 "---" 不应被误判
        let content = "---\ncategory: 技术\n---\n正文\n---\n这不是分隔符";
        let body = extract_body(content);
        assert!(body.contains("正文"));
        assert!(body.contains("---"));
        assert!(body.contains("这不是分隔符"));
    }

    // ────────── generate_frontmatter ──────────

    #[test]
    fn generate_frontmatter_basic() {
        let now = Local::now();
        let fm = generate_frontmatter("技术", &["rust".to_string(), "cli".to_string()], &now, None);
        assert!(fm.starts_with("---\n"));
        assert!(fm.ends_with("---\n"));
        assert!(fm.contains("category: 技术"));
        assert!(fm.contains("tags: [rust, cli]"));
        assert!(fm.contains("created:"));
        assert!(fm.contains("updated:"));
    }

    #[test]
    fn generate_frontmatter_empty_tags() {
        let now = Local::now();
        let fm = generate_frontmatter("未分类", &[], &now, None);
        assert!(fm.contains("tags: []"));
    }

    #[test]
    fn generate_frontmatter_with_merged_from() {
        let now = Local::now();
        let sources = vec!["a.md".to_string(), "b.md".to_string()];
        let fm = generate_frontmatter("技术", &[], &now, Some(&sources));
        assert!(fm.contains("merged_from:"));
        assert!(fm.contains("  - a.md"));
        assert!(fm.contains("  - b.md"));
    }

    #[test]
    fn generate_frontmatter_chinese_category_and_tags() {
        let now = Local::now();
        let tags = vec!["学习笔记".to_string(), "算法".to_string()];
        let fm = generate_frontmatter("工作", &tags, &now, None);
        assert!(fm.contains("category: 工作"));
        assert!(fm.contains("tags: [学习笔记, 算法]"));
    }

    // ────────── update_frontmatter_field ──────────

    #[test]
    fn update_frontmatter_field_basic() {
        let content = "---\ncategory: 技术\nupdated: \"old\"\n---\n正文";
        let result = update_frontmatter_field(content, "updated", "new value");
        assert!(result.contains("updated: \"new value\""));
        assert!(result.contains("category: 技术"));
        assert!(result.contains("正文"));
    }

    #[test]
    fn update_frontmatter_field_not_found() {
        let content = "---\ncategory: 技术\n---\n正文";
        let result = update_frontmatter_field(content, "updated", "2026-03-13");
        // 字段不存在时，内容不变
        assert!(!result.contains("updated:"));
        assert!(result.contains("category: 技术"));
    }

    #[test]
    fn update_frontmatter_field_chinese_value() {
        let content = "---\ncategory: 旧分类\n---\n";
        let result = update_frontmatter_field(content, "category", "新分类");
        assert!(result.contains("category: \"新分类\""));
    }

    #[test]
    fn update_frontmatter_field_preserves_body() {
        let content = "---\nupdated: \"old\"\n---\n# 标题\n\n正文内容\n包含多行";
        let result = update_frontmatter_field(content, "updated", "new");
        assert!(result.contains("# 标题"));
        assert!(result.contains("正文内容"));
        assert!(result.contains("包含多行"));
    }

    #[test]
    fn update_frontmatter_field_only_replaces_first() {
        // body 中也有类似字段名不应被替换
        let content = "---\ncategory: 技术\n---\ncategory: 这是正文中的";
        let result = update_frontmatter_field(content, "category", "工作");
        assert!(result.contains("category: \"工作\""));
        assert!(result.contains("category: 这是正文中的"));
    }

    // ────────── resolve_note ──────────

    #[test]
    fn resolve_note_by_index() {
        let notes = vec![
            make_note("a.md", "第一条", "技术", &[]),
            make_note("b.md", "第二条", "工作", &[]),
        ];
        match resolve_note("1", &notes) {
            MatchResult::Single(n) => assert_eq!(n.title, "第一条"),
            _ => panic!("应该匹配到第一条"),
        }
        match resolve_note("2", &notes) {
            MatchResult::Single(n) => assert_eq!(n.title, "第二条"),
            _ => panic!("应该匹配到第二条"),
        }
    }

    #[test]
    fn resolve_note_index_zero() {
        let notes = vec![make_note("a.md", "A", "技术", &[])];
        assert!(matches!(resolve_note("0", &notes), MatchResult::None));
    }

    #[test]
    fn resolve_note_index_out_of_range() {
        let notes = vec![make_note("a.md", "A", "技术", &[])];
        assert!(matches!(resolve_note("99", &notes), MatchResult::None));
    }

    #[test]
    fn resolve_note_by_exact_filename() {
        let notes = vec![
            make_note("我的笔记.md", "我的笔记", "日记", &[]),
            make_note("other.md", "other", "技术", &[]),
        ];
        match resolve_note("我的笔记.md", &notes) {
            MatchResult::Single(n) => assert_eq!(n.title, "我的笔记"),
            _ => panic!("应该精确匹配文件名"),
        }
    }

    #[test]
    fn resolve_note_by_filename_without_extension() {
        let notes = vec![make_note("test.md", "test", "技术", &[])];
        match resolve_note("test", &notes) {
            MatchResult::Single(n) => assert_eq!(n.title, "test"),
            _ => panic!("应该匹配不带扩展名的文件名"),
        }
    }

    #[test]
    fn resolve_note_fuzzy_match_chinese() {
        let notes = vec![
            make_note("a.md", "Rust学习笔记", "技术", &[]),
            make_note("b.md", "Go入门教程", "技术", &[]),
        ];
        match resolve_note("学习", &notes) {
            MatchResult::Single(n) => assert_eq!(n.title, "Rust学习笔记"),
            _ => panic!("应该模糊匹配到 Rust学习笔记"),
        }
    }

    #[test]
    fn resolve_note_fuzzy_case_insensitive() {
        let notes = vec![make_note("a.md", "MyRustNote", "技术", &[])];
        match resolve_note("myrust", &notes) {
            MatchResult::Single(n) => assert_eq!(n.title, "MyRustNote"),
            _ => panic!("应该大小写不敏感匹配"),
        }
    }

    #[test]
    fn resolve_note_fuzzy_multiple_matches() {
        let notes = vec![
            make_note("a.md", "Rust学习", "技术", &[]),
            make_note("b.md", "Rust进阶", "技术", &[]),
        ];
        match resolve_note("Rust", &notes) {
            MatchResult::Multiple(matched) => assert_eq!(matched.len(), 2),
            _ => panic!("应该返回多条匹配"),
        }
    }

    #[test]
    fn resolve_note_no_match() {
        let notes = vec![make_note("a.md", "Rust笔记", "技术", &[])];
        assert!(matches!(resolve_note("Python", &notes), MatchResult::None));
    }

    #[test]
    fn resolve_note_empty_list() {
        assert!(matches!(resolve_note("1", &[]), MatchResult::None));
    }

    // ────────── resolve_note_no_index ──────────

    #[test]
    fn resolve_note_no_index_ignores_number() {
        let notes = vec![
            make_note("1.md", "1", "技术", &[]),
            make_note("hello.md", "hello", "技术", &[]),
        ];
        // "1" 应该匹配文件名 "1.md"，而非作为序号
        match resolve_note_no_index("1", &notes) {
            MatchResult::Single(n) => assert_eq!(n.title, "1"),
            _ => panic!("应该精确匹配文件名 1.md"),
        }
    }

    #[test]
    fn resolve_note_no_index_fuzzy_chinese() {
        let notes = vec![
            make_note("a.md", "每日总结", "日记", &[]),
            make_note("b.md", "周报", "工作", &[]),
        ];
        match resolve_note_no_index("总结", &notes) {
            MatchResult::Single(n) => assert_eq!(n.title, "每日总结"),
            _ => panic!("应该模糊匹配到每日总结"),
        }
    }

    #[test]
    fn resolve_note_no_index_no_match() {
        let notes = vec![make_note("a.md", "笔记", "技术", &[])];
        assert!(matches!(resolve_note_no_index("不存在", &notes), MatchResult::None));
    }
}
