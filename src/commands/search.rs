/// note search — 搜索笔记
use anyhow::{bail, Result};

use crate::interactive;
use crate::note::{self, Note};
use crate::utils;

/// 搜索匹配层级
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
enum MatchLevel {
    Title,
    Tag,
    Body,
}

/// 执行 note search 命令
pub fn run(
    keywords: Vec<String>,
    tags: Option<String>,
    category: Option<String>,
    all: bool,
) -> Result<()> {
    if keywords.is_empty() {
        bail!("用法: note search <关键词...>");
    }

    let notes = note::load_notes(all)?;

    let mut title_matches: Vec<Note> = Vec::new();
    let mut tag_matches: Vec<Note> = Vec::new();
    let mut body_matches: Vec<Note> = Vec::new();

    for n in &notes {
        // 附加过滤：-t 标签精确匹配
        if let Some(ref tag_filter) = tags {
            let lower_filter = tag_filter.to_lowercase();
            if !n.tags.iter().any(|t| t.to_lowercase() == lower_filter) {
                continue;
            }
        }

        // 附加过滤：-c 分类精确匹配
        if let Some(ref cat_filter) = category
            && n.category.to_lowercase() != cat_filter.to_lowercase()
        {
            continue;
        }

        // 检查每个关键词是否在标题、标签、正文中至少一处命中
        let mut all_match = true;
        let mut highest_level = MatchLevel::Body;

        for kw in &keywords {
            let lower_kw = kw.to_lowercase();
            let in_title = n.title.to_lowercase().contains(&lower_kw);
            let in_tags = n.tags.iter().any(|t| t.to_lowercase().contains(&lower_kw));
            let in_body = n.body.to_lowercase().contains(&lower_kw);

            if !in_title && !in_tags && !in_body {
                all_match = false;
                break;
            }

            if in_title && highest_level > MatchLevel::Title {
                highest_level = MatchLevel::Title;
            } else if in_tags && highest_level > MatchLevel::Tag {
                highest_level = MatchLevel::Tag;
            }
        }

        if !all_match {
            continue;
        }

        match highest_level {
            MatchLevel::Title => title_matches.push(n.clone()),
            MatchLevel::Tag => tag_matches.push(n.clone()),
            MatchLevel::Body => body_matches.push(n.clone()),
        }
    }

    let total = title_matches.len() + tag_matches.len() + body_matches.len();
    if total == 0 {
        println!("未找到匹配的笔记");
        return Ok(());
    }

    // 构建带分层标记的结果列表
    let mut all_results: Vec<Note> = Vec::new();
    let mut group_labels: Vec<(usize, &str)> = Vec::new(); // (起始索引, 标签)

    if !title_matches.is_empty() {
        group_labels.push((all_results.len(), "标题匹配"));
        all_results.extend(title_matches);
    }
    if !tag_matches.is_empty() {
        group_labels.push((all_results.len(), "标签匹配"));
        all_results.extend(tag_matches);
    }
    if !body_matches.is_empty() {
        group_labels.push((all_results.len(), "正文匹配"));
        all_results.extend(body_matches);
    }

    if interactive::is_interactive() {
        // 交互模式：带分组标题的循环选择
        interactive::select_note_grouped_loop(&all_results, &group_labels, |idx| {
            let selected = &all_results[idx];
            if selected.is_deleted {
                let restore = interactive::confirm(
                    "该笔记已在 deleted/ 中，是否恢复到主目录？",
                    false,
                )?;
                if restore {
                    let notes_dir = crate::config::notes_dir()?;
                    let new_path =
                        notes_dir.join(selected.path.file_name().unwrap_or_default());
                    std::fs::rename(&selected.path, &new_path)?;
                    println!("✓ 已恢复到 {}", new_path.display());
                    return utils::show_note_content(&new_path, utils::PREVIEW_LINES);
                }
            }
            utils::show_note_content(&selected.path, utils::PREVIEW_LINES)
        })?;
    } else {
        // 非交互模式：打印分层结果
        for &(start, label) in &group_labels {
            let end = group_labels
                .iter()
                .find(|&&(s, _)| s > start)
                .map_or(all_results.len(), |&(s, _)| s);
            println!("── {label} ──────────────────────────────────");
            for n in &all_results[start..end] {
                println!("  {}", interactive::format_note_line(n));
            }
            println!();
        }
    }

    Ok(())
}
