/// note show — 查看笔记详情
use anyhow::{bail, Result};

use crate::interactive;
use crate::note::{self, MatchResult};
use crate::utils;

/// 执行 note show 命令
pub fn run(target: Option<String>) -> Result<()> {
    let notes = note::load_notes(false)?;

    if notes.is_empty() {
        println!("没有笔记");
        return Ok(());
    }

    match target {
        None => {
            // 无参数：循环选择查看
            if !interactive::is_interactive() {
                bail!("非交互环境下需要指定笔记序号或文件名");
            }
            interactive::select_note_loop(&notes, |idx| {
                utils::show_note_content(&notes[idx].path, utils::PREVIEW_LINES)
            })?;
        }
        Some(arg) => {
            // 尝试解析为数字（序号）
            if let Ok(n) = arg.parse::<usize>() {
                if n == 0 || n > notes.len() {
                    bail!("序号超出范围，当前共 {} 条笔记", notes.len());
                }
                utils::show_note_content(&notes[n - 1].path, utils::PREVIEW_LINES)?;
                return Ok(());
            }

            match note::resolve_note(&arg, &notes) {
                MatchResult::Single(n) => {
                    utils::show_note_content(&n.path, utils::PREVIEW_LINES)?;
                }
                MatchResult::Multiple(matched) => {
                    if !interactive::is_interactive() {
                        println!("匹配到多条笔记:");
                        for m in &matched {
                            println!("  {}", m.title);
                        }
                        return Ok(());
                    }
                    interactive::select_note_loop(&matched, |idx| {
                        utils::show_note_content(&matched[idx].path, utils::PREVIEW_LINES)
                    })?;
                }
                MatchResult::None => {
                    bail!("未找到匹配的笔记: {arg}");
                }
            }
        }
    }

    Ok(())
}
