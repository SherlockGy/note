/// note list — 列出笔记 + 交互选择
use anyhow::Result;

use crate::interactive;
use crate::note;
use crate::utils;

/// 执行 note list 命令
pub fn run(count: usize, tags: Option<String>, category: Option<String>, all: bool) -> Result<()> {
    let mut notes = note::load_notes(all)?;

    // 标签过滤（精确匹配，大小写不敏感）
    if let Some(ref tag_filter) = tags {
        let lower_filter = tag_filter.to_lowercase();
        notes.retain(|n| n.tags.iter().any(|t| t.to_lowercase() == lower_filter));
    }

    // 分类过滤（精确匹配，大小写不敏感）
    if let Some(ref cat_filter) = category {
        let lower_filter = cat_filter.to_lowercase();
        notes.retain(|n| n.category.to_lowercase() == lower_filter);
    }

    // 限制条数
    notes.truncate(count);

    if notes.is_empty() {
        println!("没有找到笔记");
        return Ok(());
    }

    if interactive::is_interactive() {
        // 交互模式：循环选择查看，q 退出
        interactive::select_note_loop(&notes, |idx| {
            utils::show_note_content(&notes[idx].path, utils::PREVIEW_LINES)
        })?;
    } else {
        // 非交互模式：打印静态表格
        print_table(&notes);
    }

    Ok(())
}

/// 打印静态表格（非交互模式）
fn print_table(notes: &[note::Note]) {
    use crate::interactive::{pad_display, truncate_display};

    println!(
        "  {:<4} {}  {}  {}  标题",
        "#",
        pad_display("日期", 18),
        pad_display("分类", 6),
        pad_display("标签", 16),
    );
    for (i, n) in notes.iter().enumerate() {
        let date = format_date_column(&n.created);
        let tags_str = if n.tags.is_empty() {
            String::new()
        } else {
            n.tags.join(",")
        };
        let deleted_mark = if n.is_deleted { "[已删除] " } else { "" };
        println!(
            "  {:<4} {}  {}  {}  {}{}",
            i + 1,
            pad_display(&date, 18),
            pad_display(&n.category, 6),
            pad_display(&truncate_display(&tags_str, 16), 16),
            deleted_mark,
            n.title
        );
    }
}

/// 从 created 字段提取 "MM-DD 周X HH:mm"
fn format_date_column(created: &str) -> String {
    // created 格式: "2026-03-12 周四 14:30"
    if created.len() >= 10 {
        let rest = if created.len() > 10 {
            &created[5..]
        } else {
            created
        };
        rest.to_string()
    } else {
        created.to_string()
    }
}
