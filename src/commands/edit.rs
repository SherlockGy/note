/// note edit — 编辑已有笔记
use anyhow::{Result, bail};
use chrono::Local;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use crate::note::MatchResult;
use crate::{editor, interactive, note, utils};

/// 执行 note edit 命令
pub fn run(target: Option<String>) -> Result<()> {
    let notes = note::load_notes(false)?;

    if notes.is_empty() {
        println!("没有笔记");
        return Ok(());
    }

    let selected = match target {
        None => {
            // 无参数：弹出交互列表
            if !interactive::is_interactive() {
                bail!("非交互环境下需要指定笔记序号或文件名");
            }
            match interactive::select_note(&notes)? {
                Some(idx) => notes[idx].clone(),
                None => return Ok(()),
            }
        }
        Some(arg) => {
            // 序号处理
            if let Ok(n) = arg.parse::<usize>() {
                if n == 0 || n > notes.len() {
                    bail!("序号超出范围，当前共 {} 条笔记", notes.len());
                }
                notes[n - 1].clone()
            } else {
                match note::resolve_note(&arg, &notes) {
                    MatchResult::Single(n) => n,
                    MatchResult::Multiple(matched) => {
                        if !interactive::is_interactive() {
                            println!("匹配到多条笔记:");
                            for m in &matched {
                                println!("  {}", m.title);
                            }
                            bail!("请指定更精确的文件名");
                        }
                        match interactive::select_note(&matched)? {
                            Some(idx) => matched[idx].clone(),
                            None => return Ok(()),
                        }
                    }
                    MatchResult::None => {
                        bail!("未找到匹配的笔记: {arg}");
                    }
                }
            }
        }
    };

    // 记录编辑前的内容哈希
    let content_before = std::fs::read_to_string(&selected.path)?;
    let hash_before = hash_string(&content_before);

    // 打开编辑器
    let (editor_cmd, args) = editor::find_editor();
    editor::open_editor(&editor_cmd, &args, &selected.path)?;

    // 检查是否有变化
    let content_after = std::fs::read_to_string(&selected.path)?;
    let hash_after = hash_string(&content_after);

    if hash_before != hash_after {
        // 更新 updated 字段
        let now = Local::now();
        let time_str = utils::format_datetime(&now);
        let updated = note::update_frontmatter_field(&content_after, "updated", &time_str);
        std::fs::write(&selected.path, updated)?;
        println!("✓ 已更新: {}", selected.path.display());
    }

    Ok(())
}

/// 计算字符串哈希
fn hash_string(s: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish()
}
