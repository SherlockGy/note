/// note merge — 合并笔记
use anyhow::{Result, bail};
use chrono::Local;
use std::collections::HashMap;

use crate::note::{MatchResult, Note};
use crate::{config, interactive, note, utils};

/// 执行 note merge 命令
pub fn run(targets: Vec<String>, tags_filter: Option<String>) -> Result<()> {
    let notes = note::load_notes(false)?;

    if notes.is_empty() {
        println!("没有笔记");
        return Ok(());
    }

    // 确定待合并的笔记列表
    let to_merge = if let Some(ref tag) = tags_filter {
        // 按标签批量合并
        let lower_tag = tag.to_lowercase();
        let matched: Vec<Note> = notes
            .iter()
            .filter(|n| n.tags.iter().any(|t| t.to_lowercase() == lower_tag))
            .cloned()
            .collect();

        if matched.len() < 2 {
            bail!("标签 \"{}\" 匹配到的笔记不足 2 条，无法合并", tag);
        }
        matched
    } else if !targets.is_empty() {
        // 按参数指定
        let mut selected = Vec::new();
        for arg in &targets {
            match note::resolve_note(arg, &notes) {
                MatchResult::Single(n) => selected.push(n),
                MatchResult::Multiple(matched) => {
                    if !interactive::is_interactive() {
                        bail!("\"{}\" 匹配到多条笔记，请指定更精确的文件名", arg);
                    }
                    if let Some(idx) = interactive::select_note(&matched)? {
                        selected.push(matched[idx].clone())
                    }
                }
                MatchResult::None => {
                    eprintln!("未找到匹配的笔记: {arg}");
                }
            }
        }

        if selected.len() < 2 {
            bail!("至少需要 2 条笔记才能合并，当前选中 {} 条", selected.len());
        }
        selected
    } else {
        // 无参数：弹出多选列表
        if !interactive::is_interactive() {
            bail!("非交互环境下需要指定要合并的笔记");
        }

        let selections = interactive::multi_select_notes(
            &notes,
            "📎 合并笔记 — 空格勾选，回车确认（至少选 2 条）",
        )?;

        if selections.len() < 2 {
            bail!("至少选择 2 条笔记");
        }

        selections.iter().map(|&i| notes[i].clone()).collect()
    };

    // 显示待合并列表
    println!("\n📎 合并笔记\n");
    println!("待合并 ({} 条):", to_merge.len());
    for (i, n) in to_merge.iter().enumerate() {
        let tags_str = if n.tags.is_empty() {
            String::new()
        } else {
            n.tags.join(",")
        };
        println!("  {}. {}  [{}] {}", i + 1, n.title, n.category, tags_str);
    }
    println!();

    // 输入新标题（必填）
    let new_title = if interactive::is_interactive() {
        let trimmed = interactive::read_input("新标题", false, None)?;
        if trimmed.is_empty() {
            bail!("标题不能为空");
        }
        trimmed
    } else {
        bail!("非交互环境下无法输入合并标题");
    };

    // 确认合并
    if !interactive::confirm("确认合并？旧笔记将移入 merged/", false)? {
        println!("已取消");
        return Ok(());
    }

    // 执行合并
    execute_merge(&to_merge, &new_title)?;

    Ok(())
}

/// 执行合并操作
fn execute_merge(to_merge: &[Note], new_title: &str) -> Result<()> {
    let notes_dir = config::notes_dir()?;
    let now = Local::now();

    // 1. 计算合并后的标签（并集，大小写不敏感去重）
    let merged_tags = merge_tags(to_merge);

    // 2. 计算合并后的分类（投票取最常见）
    let merged_category = vote_category(to_merge);

    // 3. 收集来源文件名
    let merged_from: Vec<String> = to_merge
        .iter()
        .map(|n| {
            n.path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string()
        })
        .collect();

    // 4. 生成 frontmatter
    let frontmatter =
        note::generate_frontmatter(&merged_category, &merged_tags, &now, Some(&merged_from));

    // 5. 拼接正文
    let mut body = String::new();
    for n in to_merge {
        let note_body = n.body.trim();
        if !note_body.is_empty() {
            body.push_str(note_body);
            body.push('\n');
            let fname = n.path.file_name().unwrap_or_default().to_string_lossy();
            body.push_str(&format!("<!-- from: {fname} -->\n\n"));
        }
    }

    // 6. 写入新文件
    let filename = format!("{}.md", utils::sanitize_filename(new_title));
    let target_path = notes_dir.join(&filename);
    let file_content = format!("{frontmatter}\n{body}");
    std::fs::write(&target_path, &file_content)?;

    // 7. 移动旧笔记到 merged/
    let merged_dir = notes_dir.join("merged");
    for n in to_merge {
        let dest = merged_dir.join(n.path.file_name().unwrap_or_default());
        std::fs::rename(&n.path, &dest)?;
    }

    // 8. 更新 .config/tags
    config::save_tags(&merged_tags)?;

    // 9. 输出确认
    let tags_str = if merged_tags.is_empty() {
        "无".to_string()
    } else {
        merged_tags.join(",")
    };
    let time_str = utils::format_datetime(&now);

    println!("\n✓ 合并完成");
    println!("  新笔记: {filename}");
    println!(
        "  分类: {} | 标签: {} | {}",
        merged_category, tags_str, time_str
    );
    println!("  已移入 merged/: {} 条", to_merge.len());

    Ok(())
}

/// 合并标签（并集，大小写不敏感去重，保留首次出现的大小写）
fn merge_tags(notes: &[Note]) -> Vec<String> {
    let mut seen: Vec<String> = Vec::new();
    let mut result = Vec::new();

    for n in notes {
        for tag in &n.tags {
            let lower = tag.to_lowercase();
            if !seen.contains(&lower) {
                seen.push(lower);
                result.push(tag.clone());
            }
        }
    }

    result
}

/// 投票选分类（出现最多的；平票取第一条的）
fn vote_category(notes: &[Note]) -> String {
    let mut counts: HashMap<String, usize> = HashMap::new();
    for n in notes {
        *counts.entry(n.category.clone()).or_insert(0) += 1;
    }

    let max_count = counts.values().max().copied().unwrap_or(0);
    // 在最高票中，取在原始笔记列表中最早出现的
    for n in notes {
        if counts.get(&n.category).copied().unwrap_or(0) == max_count {
            return n.category.clone();
        }
    }

    notes
        .first()
        .map(|n| n.category.clone())
        .unwrap_or_else(|| "未分类".to_string())
}
