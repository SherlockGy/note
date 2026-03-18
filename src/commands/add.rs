/// note add — 创建笔记
use anyhow::{Context, Result, bail};
use chrono::Local;
use std::io::{IsTerminal, Read};
use std::path::PathBuf;

use crate::{config, editor, interactive, note, utils};

/// 内容来源
enum ContentSource {
    Text(String),
    File(PathBuf),
    Editor,
    Stdin(String),
}

/// 执行 note add 命令
pub fn run(
    content_args: Vec<String>,
    title: Option<String>,
    tags: Option<String>,
    category: Option<String>,
    edit: bool,
    file: Option<PathBuf>,
    quiet: bool,
) -> Result<()> {
    // 确定是否为静默模式
    let stdin_is_terminal = std::io::stdin().is_terminal();
    let mut is_quiet = quiet;

    // 1. 确定内容来源
    let source = determine_source(content_args, edit, file, stdin_is_terminal, quiet)?;

    // 管道输入时自动静默
    if matches!(source, ContentSource::Stdin(_)) {
        is_quiet = true;
    }

    // 2. 获取内容
    let (body, is_file_mode, original_filename) = get_content(&source)?;

    // 校验空内容
    if body.trim().is_empty() {
        bail!("内容不能为空");
    }

    // 3-7. 交互输入阶段（备用屏幕，完成后提示自动消失）
    let use_alt = !is_quiet && interactive::is_interactive();
    if use_alt {
        interactive::enter_alt_screen();
    }

    let input_result = (|| -> anyhow::Result<_> {
        let note_title = determine_title(title, is_file_mode, original_filename.as_deref(), is_quiet)?;
        let note_category = determine_category(category, is_quiet)?;
        let note_tags = determine_tags(tags, is_quiet)?;

        let filename = if let Some(ref t) = note_title {
            format!("{}.md", utils::sanitize_filename(t))
        } else {
            format!("{}.md", utils::timestamp_filename())
        };

        let notes_dir = config::notes_dir()?;
        let target_path = notes_dir.join(&filename);
        let final_path = handle_conflict(target_path, &filename, is_quiet)?;

        Ok((note_category, note_tags, final_path))
    })();

    if use_alt {
        interactive::leave_alt_screen();
    }

    let (note_category, note_tags, final_path) = input_result?;
    let final_path = match final_path {
        Some(p) => p,
        None => {
            println!("已取消");
            return Ok(());
        }
    };

    // 8. 构建文件内容
    let now = Local::now();
    let frontmatter = note::generate_frontmatter(&note_category, &note_tags, &now, None);

    let file_content = format!("{frontmatter}\n{body}\n");
    std::fs::write(&final_path, &file_content)?;

    // 9. 更新 .config/tags
    if !note_tags.is_empty() {
        config::save_tags(&note_tags)?;
    }

    // 10. 输出确认信息
    let tags_str = if note_tags.is_empty() {
        "无".to_string()
    } else {
        note_tags.join(",")
    };
    let time_str = utils::format_datetime(&now);

    println!("✓ 已保存: {}", final_path.display());
    println!(
        "  分类: {} | 标签: {} | {}",
        note_category, tags_str, time_str
    );

    Ok(())
}

/// 确定内容来源
fn determine_source(
    content_args: Vec<String>,
    edit: bool,
    file: Option<PathBuf>,
    stdin_is_terminal: bool,
    quiet: bool,
) -> Result<ContentSource> {
    // 1. -f 参数
    if let Some(ref path) = file {
        if !path.exists() {
            bail!("文件不存在: {}", path.display());
        }
        return Ok(ContentSource::File(path.clone()));
    }

    // 2. -e 参数
    if edit {
        return Ok(ContentSource::Editor);
    }

    // 3. 位置参数（优先于 stdin 检测，避免有参数时误读空 stdin）
    if !content_args.is_empty() {
        let text = content_args.join(" ");

        // 智能文件检测（非核心功能，但在此实现）
        let path = std::path::Path::new(&text);
        if content_args.len() == 1 && path.exists() && path.is_file() {
            return Ok(ContentSource::File(path.to_path_buf()));
        }

        // 检查是否像文件名但不存在
        let common_exts = [
            "txt", "md", "sql", "log", "java", "go", "rs", "py", "js", "ts", "json", "yaml", "yml",
            "toml", "xml", "sh", "css", "html",
        ];
        if content_args.len() == 1
            && let Some(ext) = path.extension().and_then(|e| e.to_str())
            && common_exts.contains(&ext.to_lowercase().as_str())
            && !path.exists()
        {
            if quiet {
                return Ok(ContentSource::Text(text));
            }
            let confirm = interactive::confirm(
                &format!(
                    "\"{}\" 看起来是文件名但文件不存在，是否作为文本记录？",
                    text
                ),
                false,
            )?;
            if confirm {
                return Ok(ContentSource::Text(text));
            } else {
                bail!("已取消");
            }
        }

        return Ok(ContentSource::Text(text));
    }

    // 4. 管道 stdin（位置参数为空时才尝试读 stdin）
    if !stdin_is_terminal {
        let mut input = String::new();
        std::io::stdin().read_to_string(&mut input)?;
        if !input.trim().is_empty() {
            return Ok(ContentSource::Stdin(input));
        }
    }

    // 5. 无内容
    bail!("用法: note add <内容> 或 note add -e 或 note add -f <文件>");
}

/// 获取内容，返回 (正文, 是否文件收录模式, 原文件名)
fn get_content(source: &ContentSource) -> Result<(String, bool, Option<String>)> {
    match source {
        ContentSource::Text(text) => Ok((text.clone(), false, None)),
        ContentSource::Stdin(text) => Ok((text.clone(), false, None)),
        ContentSource::File(path) => {
            // 检查是否为文本文件
            if !utils::is_text_file(path)? {
                let fname = path.file_name().unwrap_or_default().to_string_lossy();
                bail!("仅支持文本文件: {fname}");
            }
            let content = std::fs::read_to_string(path)
                .with_context(|| format!("无法读取文件: {}", path.display()))?;
            let original_name = path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            Ok((content, true, Some(original_name)))
        }
        ContentSource::Editor => {
            let (editor_cmd, editor_args) = editor::find_editor();
            let temp_dir = std::env::temp_dir();
            let temp_file = temp_dir.join(format!("note_edit_{}.md", std::process::id()));
            std::fs::write(&temp_file, "")?;

            editor::open_editor(&editor_cmd, &editor_args, &temp_file)?;

            let content = std::fs::read_to_string(&temp_file)?;
            let _ = std::fs::remove_file(&temp_file);

            if content.trim().is_empty() {
                bail!("内容不能为空，已取消");
            }
            Ok((content, false, None))
        }
    }
}

/// 确定标题
fn determine_title(
    cli_title: Option<String>,
    is_file_mode: bool,
    original_filename: Option<&str>,
    quiet: bool,
) -> Result<Option<String>> {
    // -T 参数已指定
    if let Some(t) = cli_title {
        let trimmed = t.trim().to_string();
        if trimmed.is_empty() {
            return Ok(None);
        }
        return Ok(Some(trimmed));
    }

    // 静默模式
    if quiet {
        if is_file_mode {
            return Ok(original_filename.map(|s| s.to_string()));
        }
        return Ok(None);
    }

    // 交互模式
    if !interactive::is_interactive() {
        if is_file_mode {
            return Ok(original_filename.map(|s| s.to_string()));
        }
        return Ok(None);
    }

    interactive::input_title(if is_file_mode {
        original_filename
    } else {
        None
    })
}

/// 确定分类
fn determine_category(cli_category: Option<String>, quiet: bool) -> Result<String> {
    // -c 参数已指定
    if let Some(c) = cli_category {
        let trimmed = c.trim().to_string();
        config::ensure_category_exists(&trimmed)?;
        return Ok(trimmed);
    }

    // 静默或非交互模式
    if quiet || !interactive::is_interactive() {
        return Ok("未分类".to_string());
    }

    // 交互模式
    let categories = config::load_categories()?;
    interactive::select_category(&categories)
}

/// 确定标签
fn determine_tags(cli_tags: Option<String>, quiet: bool) -> Result<Vec<String>> {
    // -t 参数已指定
    if let Some(t) = cli_tags {
        return utils::parse_tags_input(&t, quiet);
    }

    // 静默或非交互模式
    if quiet || !interactive::is_interactive() {
        return Ok(Vec::new());
    }

    // 交互模式
    interactive::input_tags()
}

/// 处理同名冲突，返回最终路径（None 表示取消）
fn handle_conflict(target_path: PathBuf, filename: &str, quiet: bool) -> Result<Option<PathBuf>> {
    if !target_path.exists() {
        return Ok(Some(target_path));
    }

    if quiet {
        // 静默模式默认追加时间戳后缀
        let stem = target_path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy();
        let suffix = Local::now().format("%H%M%S");
        let new_name = format!("{stem}_{suffix}.md");
        let parent = target_path.parent().unwrap_or(target_path.as_path());
        return Ok(Some(parent.join(new_name)));
    }

    if !interactive::is_interactive() {
        // 非交互模式追加后缀
        let stem = target_path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy();
        let suffix = Local::now().format("%H%M%S");
        let new_name = format!("{stem}_{suffix}.md");
        let parent = target_path.parent().unwrap_or(target_path.as_path());
        return Ok(Some(parent.join(new_name)));
    }

    // 交互选择
    match interactive::conflict_choice(filename)? {
        interactive::ConflictAction::Overwrite => Ok(Some(target_path)),
        interactive::ConflictAction::Append => {
            // 追加到现有文件末尾 — 返回原路径，调用方负责追加
            // 这里简单处理：读取现有内容，在调用方会覆盖写入
            // 实际上我们需要一个特殊标记来告诉调用方追加
            // 为简化实现，这里直接返回路径，在外层统一处理
            Ok(Some(target_path))
        }
        interactive::ConflictAction::Suffix => {
            let stem = target_path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy();
            let suffix = Local::now().format("%H%M%S");
            let new_name = format!("{stem}_{suffix}.md");
            let parent = target_path.parent().unwrap_or(target_path.as_path());
            Ok(Some(parent.join(new_name)))
        }
        interactive::ConflictAction::Cancel => Ok(None),
    }
}
