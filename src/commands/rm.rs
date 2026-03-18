/// note rm — 清理笔记
use anyhow::Result;
use chrono::Local;

use crate::note::MatchResult;
use crate::{config, interactive, note, utils};

/// 执行 note rm 命令
pub fn run(targets: Vec<String>) -> Result<()> {
    let notes = note::load_notes(false)?;

    if notes.is_empty() {
        println!("没有笔记");
        return Ok(());
    }

    let to_delete = if targets.is_empty() {
        // 无参数：弹出多选列表
        if !interactive::is_interactive() {
            anyhow::bail!("非交互环境下需要指定要清理的笔记");
        }

        let selections =
            interactive::multi_select_notes(&notes, "🗑️ 清理笔记 — 空格勾选，回车确认")?;

        if selections.is_empty() {
            println!("未选择任何笔记");
            return Ok(());
        }

        selections
            .iter()
            .map(|&i| notes[i].clone())
            .collect::<Vec<_>>()
    } else {
        // 有参数：逐个解析（不支持序号）
        let mut selected = Vec::new();
        let mut warnings = Vec::new();

        for arg in &targets {
            match note::resolve_note_no_index(arg, &notes) {
                MatchResult::Single(n) => selected.push(n),
                MatchResult::Multiple(matched) => {
                    if !interactive::is_interactive() {
                        eprintln!("\"{}\" 匹配到多条笔记，请指定更精确的文件名", arg);
                        continue;
                    }
                    if let Some(idx) = interactive::select_note(&matched)? {
                        selected.push(matched[idx].clone())
                    }
                }
                MatchResult::None => {
                    warnings.push(format!("未找到匹配的笔记: {arg}"));
                }
            }
        }

        // 输出所有警告
        for w in &warnings {
            eprintln!("{w}");
        }

        if selected.is_empty() {
            return Ok(());
        }

        selected
    };

    // 确认流程：展示预览
    println!("确定清理以下笔记？（将移入 deleted/）\n");

    for n in &to_delete {
        let fname = n.path.file_name().unwrap_or_default().to_string_lossy();
        println!("── {fname} ──");
        utils::preview_note_body(&n.path, utils::PREVIEW_LINES)?;
        println!();
    }

    if interactive::is_interactive() && !interactive::confirm("确认清理？", false)? {
        println!("已取消");
        return Ok(());
    }

    // 执行清理
    let notes_dir = config::notes_dir()?;
    let deleted_dir = notes_dir.join("deleted");
    let mut count = 0;

    for n in &to_delete {
        let fname = n.path.file_name().unwrap_or_default();
        let mut dest = deleted_dir.join(fname);

        // 处理同名冲突：追加时间戳后缀
        if dest.exists() {
            let stem = n.path.file_stem().unwrap_or_default().to_string_lossy();
            let suffix = Local::now().format("%H%M%S");
            dest = deleted_dir.join(format!("{stem}_{suffix}.md"));
        }

        std::fs::rename(&n.path, &dest)?;
        count += 1;
    }

    println!(
        "\n🗑️ 已清理 {} 条笔记，可在 ~/.notes/deleted/ 中恢复。",
        count
    );

    Ok(())
}
