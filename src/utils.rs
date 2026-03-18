/// 工具函数模块：文件名安全处理、时间格式化、标签解析等
use anyhow::{Result, bail};
use chrono::{Datelike, Local, Weekday};
use std::path::Path;

/// 预览行数常量
pub const PREVIEW_LINES: usize = 30;

/// 周几中文映射
pub fn weekday_cn(w: Weekday) -> &'static str {
    match w {
        Weekday::Mon => "周一",
        Weekday::Tue => "周二",
        Weekday::Wed => "周三",
        Weekday::Thu => "周四",
        Weekday::Fri => "周五",
        Weekday::Sat => "周六",
        Weekday::Sun => "周日",
    }
}

/// 格式化时间为 "YYYY-MM-DD 周X HH:mm"
pub fn format_datetime(dt: &chrono::DateTime<Local>) -> String {
    format!(
        "{} {} {}",
        dt.format("%Y-%m-%d"),
        weekday_cn(dt.weekday()),
        dt.format("%H:%M")
    )
}

/// 生成时间戳文件名 "yyyy-MM-dd_HHmmss"
pub fn timestamp_filename() -> String {
    Local::now().format("%Y-%m-%d_%H%M%S").to_string()
}

/// 检查是否为 Windows 保留文件名
fn is_windows_reserved(name: &str) -> bool {
    let upper = name.to_uppercase();
    let stem = upper.split('.').next().unwrap_or("");
    matches!(
        stem,
        "CON"
            | "PRN"
            | "AUX"
            | "NUL"
            | "COM1"
            | "COM2"
            | "COM3"
            | "COM4"
            | "COM5"
            | "COM6"
            | "COM7"
            | "COM8"
            | "COM9"
            | "LPT1"
            | "LPT2"
            | "LPT3"
            | "LPT4"
            | "LPT5"
            | "LPT6"
            | "LPT7"
            | "LPT8"
            | "LPT9"
    )
}

/// 文件名安全处理：去除非法字符
pub fn sanitize_filename(name: &str) -> String {
    let mut sanitized: String = name
        .chars()
        .filter(|c| !"/\\:*?\"<>|".contains(*c))
        .collect();
    sanitized = sanitized.trim().to_string();

    if cfg!(windows) && is_windows_reserved(&sanitized) {
        sanitized = format!("_{sanitized}");
    }

    if sanitized.is_empty() {
        sanitized = timestamp_filename();
    }

    sanitized
}

/// 检查文件是否为文本文件（读取前 8192 字节，含 NULL 字节则判定为二进制）
pub fn is_text_file(path: &Path) -> Result<bool> {
    use std::io::Read;
    let mut file = std::fs::File::open(path)?;
    let mut buffer = [0u8; 8192];
    let bytes_read = file.read(&mut buffer)?;
    Ok(!buffer[..bytes_read].contains(&0))
}

/// 解析标签输入（支持中英文逗号、trim、空格校验）
/// quiet 模式下自动将空格替换为 -
pub fn parse_tags_input(input: &str, quiet: bool) -> Result<Vec<String>> {
    let tags: Vec<String> = input
        .replace('，', ",")
        .split(',')
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .collect();

    let mut result = Vec::new();
    for tag in tags {
        if tag.contains(' ') {
            if quiet {
                result.push(tag.replace(' ', "-"));
            } else {
                bail!(
                    "标签不能含空格: \"{}\"\n  请用 - 或 _ 代替（如 {}）",
                    tag,
                    tag.replace(' ', "-")
                );
            }
        } else {
            result.push(tag);
        }
    }

    // 大小写不敏感去重，保留首次出现的大小写
    let mut seen: Vec<String> = Vec::new();
    let mut deduped = Vec::new();
    for tag in result {
        let lower = tag.to_lowercase();
        if !seen.contains(&lower) {
            seen.push(lower);
            deduped.push(tag);
        }
    }

    Ok(deduped)
}

/// 输出笔记内容（从文件第一行开始，含 frontmatter）
/// 短内容直接打印，长内容调用系统 pager（less/more），找不到 pager 时截断显示
pub fn show_note_content(path: &Path, max_lines: usize) -> Result<()> {
    let content = std::fs::read_to_string(path)?;
    let lines: Vec<&str> = content.lines().collect();
    let total = lines.len();

    if total <= max_lines {
        println!("{content}");
        return Ok(());
    }

    // 长内容：尝试使用 pager
    if std::io::IsTerminal::is_terminal(&std::io::stdout())
        && let Some(pager) = find_pager() {
            use std::io::Write;
            let mut child = std::process::Command::new(&pager)
                .stdin(std::process::Stdio::piped())
                .spawn()
                .ok();
            if let Some(ref mut proc) = child {
                if let Some(ref mut stdin) = proc.stdin {
                    let _ = stdin.write_all(content.as_bytes());
                }
                let _ = proc.wait();
                return Ok(());
            }
        }

    // 回退：截断显示
    for line in &lines[..max_lines] {
        println!("{line}");
    }
    let filename = path.file_name().unwrap_or_default().to_string_lossy();
    println!(
        "\n... 共 {} 行，已显示前 {} 行。使用 note edit {} 查看完整内容。",
        total, max_lines, filename
    );
    Ok(())
}

/// 查找系统 pager
fn find_pager() -> Option<String> {
    if let Ok(p) = std::env::var("PAGER") {
        return Some(p);
    }
    for cmd in ["less", "more"] {
        if which::which(cmd).is_ok() {
            return Some(cmd.to_string());
        }
    }
    None
}

/// 输出笔记正文（跳过 frontmatter，仅正文部分）
/// 用于 note rm 删除前预览
pub fn preview_note_body(path: &Path, max_lines: usize) -> Result<()> {
    let content = std::fs::read_to_string(path)?;
    let body = crate::note::extract_body(&content);
    let lines: Vec<&str> = body.lines().collect();
    let total = lines.len();

    let display_lines = max_lines.min(total);
    for line in &lines[..display_lines] {
        println!("  {line}");
    }

    if total <= max_lines {
        println!("  （共 {total} 行）");
    } else {
        println!("  （共 {total} 行，已显示前 {max_lines} 行）");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Weekday};

    // ────────── weekday_cn ──────────

    #[test]
    fn weekday_cn_all_days() {
        assert_eq!(weekday_cn(Weekday::Mon), "周一");
        assert_eq!(weekday_cn(Weekday::Tue), "周二");
        assert_eq!(weekday_cn(Weekday::Wed), "周三");
        assert_eq!(weekday_cn(Weekday::Thu), "周四");
        assert_eq!(weekday_cn(Weekday::Fri), "周五");
        assert_eq!(weekday_cn(Weekday::Sat), "周六");
        assert_eq!(weekday_cn(Weekday::Sun), "周日");
    }

    // ────────── format_datetime ──────────

    #[test]
    fn format_datetime_format() {
        // 构造一个已知时间
        let dt = Local.with_ymd_and_hms(2026, 3, 13, 15, 30, 0).unwrap();
        let result = format_datetime(&dt);
        assert!(result.starts_with("2026-03-13"));
        assert!(result.contains("周"));
        assert!(result.ends_with("15:30"));
    }

    #[test]
    fn format_datetime_midnight() {
        let dt = Local.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        let result = format_datetime(&dt);
        assert!(result.contains("2026-01-01"));
        assert!(result.ends_with("00:00"));
    }

    #[test]
    fn format_datetime_weekday_correct() {
        // 2026-03-13 是周五
        let dt = Local.with_ymd_and_hms(2026, 3, 13, 12, 0, 0).unwrap();
        let result = format_datetime(&dt);
        assert!(result.contains("周五"));
    }

    // ────────── is_windows_reserved ──────────

    #[test]
    fn is_windows_reserved_basic() {
        assert!(is_windows_reserved("CON"));
        assert!(is_windows_reserved("PRN"));
        assert!(is_windows_reserved("AUX"));
        assert!(is_windows_reserved("NUL"));
        assert!(is_windows_reserved("COM1"));
        assert!(is_windows_reserved("LPT1"));
    }

    #[test]
    fn is_windows_reserved_case_insensitive() {
        assert!(is_windows_reserved("con"));
        assert!(is_windows_reserved("Con"));
        assert!(is_windows_reserved("nul"));
    }

    #[test]
    fn is_windows_reserved_with_extension() {
        assert!(is_windows_reserved("CON.txt"));
        assert!(is_windows_reserved("nul.md"));
    }

    #[test]
    fn is_windows_reserved_normal_names() {
        assert!(!is_windows_reserved("hello"));
        assert!(!is_windows_reserved("CONNECT"));
        assert!(!is_windows_reserved("日记"));
        assert!(!is_windows_reserved("COM10"));
    }

    // ────────── sanitize_filename ──────────

    #[test]
    fn sanitize_filename_normal() {
        assert_eq!(sanitize_filename("hello"), "hello");
    }

    #[test]
    fn sanitize_filename_chinese() {
        assert_eq!(sanitize_filename("我的笔记"), "我的笔记");
    }

    #[test]
    fn sanitize_filename_mixed_chinese_english() {
        assert_eq!(sanitize_filename("Rust学习笔记2026"), "Rust学习笔记2026");
    }

    #[test]
    fn sanitize_filename_removes_illegal_chars() {
        assert_eq!(sanitize_filename("a/b\\c:d*e?f\"g<h>i|j"), "abcdefghij");
    }

    #[test]
    fn sanitize_filename_trims_whitespace() {
        assert_eq!(sanitize_filename("  hello world  "), "hello world");
    }

    #[test]
    fn sanitize_filename_empty_after_sanitize() {
        // 全部是非法字符，清理后为空，应回退到时间戳
        let result = sanitize_filename("/:*?\"<>|");
        // 应该是时间戳格式 YYYY-MM-DD_HHmmss
        assert!(result.len() >= 15);
        assert!(result.contains('-'));
        assert!(result.contains('_'));
    }

    #[test]
    fn sanitize_filename_windows_reserved() {
        if cfg!(windows) {
            assert_eq!(sanitize_filename("CON"), "_CON");
            assert_eq!(sanitize_filename("nul"), "_nul");
        }
    }

    #[test]
    fn sanitize_filename_preserves_dots() {
        assert_eq!(sanitize_filename("v1.2.3-beta"), "v1.2.3-beta");
    }

    // ────────── parse_tags_input ──────────

    #[test]
    fn parse_tags_input_basic() {
        let tags = parse_tags_input("rust,cli,web", false).unwrap();
        assert_eq!(tags, vec!["rust", "cli", "web"]);
    }

    #[test]
    fn parse_tags_input_chinese_commas() {
        let tags = parse_tags_input("学习，算法，数据结构", false).unwrap();
        assert_eq!(tags, vec!["学习", "算法", "数据结构"]);
    }

    #[test]
    fn parse_tags_input_mixed_commas() {
        let tags = parse_tags_input("rust，cli,算法", false).unwrap();
        assert_eq!(tags, vec!["rust", "cli", "算法"]);
    }

    #[test]
    fn parse_tags_input_trims_spaces() {
        let tags = parse_tags_input("  rust ,  cli  , web ", false).unwrap();
        assert_eq!(tags, vec!["rust", "cli", "web"]);
    }

    #[test]
    fn parse_tags_input_filters_empty() {
        let tags = parse_tags_input("rust,,cli,,,", false).unwrap();
        assert_eq!(tags, vec!["rust", "cli"]);
    }

    #[test]
    fn parse_tags_input_space_in_tag_error() {
        let result = parse_tags_input("hello world", false);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("标签不能含空格"));
    }

    #[test]
    fn parse_tags_input_space_in_tag_quiet_mode() {
        let tags = parse_tags_input("hello world, foo bar", true).unwrap();
        assert_eq!(tags, vec!["hello-world", "foo-bar"]);
    }

    #[test]
    fn parse_tags_input_dedup_case_insensitive() {
        let tags = parse_tags_input("Rust,rust,RUST", false).unwrap();
        assert_eq!(tags, vec!["Rust"]); // 保留首次出现的大小写
    }

    #[test]
    fn parse_tags_input_dedup_preserves_order() {
        let tags = parse_tags_input("b,a,B,A,c", false).unwrap();
        assert_eq!(tags, vec!["b", "a", "c"]);
    }

    #[test]
    fn parse_tags_input_single_tag() {
        let tags = parse_tags_input("rust", false).unwrap();
        assert_eq!(tags, vec!["rust"]);
    }

    #[test]
    fn parse_tags_input_chinese_tag_with_space_quiet() {
        let tags = parse_tags_input("学习 笔记", true).unwrap();
        assert_eq!(tags, vec!["学习-笔记"]);
    }

    #[test]
    fn parse_tags_input_only_commas() {
        let tags = parse_tags_input(",,,", false).unwrap();
        assert!(tags.is_empty());
    }

    #[test]
    fn parse_tags_input_only_spaces() {
        let tags = parse_tags_input("   ", false).unwrap();
        assert!(tags.is_empty());
    }
}
