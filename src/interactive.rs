/// 交互组件：分类选择器、标签输入、列表选择等
use anyhow::{Result, bail};
use console::{Key, Style, Term};
use dialoguer::theme::Theme;
use dialoguer::{Confirm, MultiSelect, Select};
use std::fmt;
use std::io::IsTerminal;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::note::Note;

/// 自定义 Theme：prompt 不追加 ":"，退出后不留残留文本
struct NoteSelectTheme;

impl Theme for NoteSelectTheme {
    fn format_select_prompt(&self, f: &mut dyn fmt::Write, prompt: &str) -> fmt::Result {
        write!(f, "{}", prompt)
    }

    fn format_select_prompt_selection(
        &self,
        _f: &mut dyn fmt::Write,
        _prompt: &str,
        _sel: &str,
    ) -> fmt::Result {
        // 退出/选择后不显示任何残留文本
        Ok(())
    }
}

/// 生成列表 prompt：列名 + 控制提示（全部 dim 风格）
fn list_prompt() -> String {
    let dim = Style::new().dim();
    let header = format!(
        "  {}  {}  {}  标题",
        pad_display("日期", 16),
        pad_display("分类", 6),
        pad_display("标签", 16),
    );
    format!(
        "{}\n\n{}\n",
        dim.apply_to("  ↑↓选择  ↵查看  q退出"),
        dim.apply_to(&header),
    )
}

/// 检查当前环境是否支持交互
pub fn is_interactive() -> bool {
    std::io::stdout().is_terminal() && std::io::stdin().is_terminal()
}

/// 列表可见行数（超出滚动）
const LIST_PAGE_SIZE: usize = 15;

/// 单选笔记列表，返回选中笔记的索引
pub fn select_note(notes: &[Note]) -> Result<Option<usize>> {
    if notes.is_empty() {
        return Ok(None);
    }

    let items: Vec<String> = notes.iter().map(format_note_line).collect();

    let dim = Style::new().dim();
    let prompt = format!("{}", dim.apply_to("  ↑↓选择  ↵确认  Esc取消"));

    let selection = Select::with_theme(&NoteSelectTheme)
        .with_prompt(&prompt)
        .items(&items)
        .default(0)
        .max_length(LIST_PAGE_SIZE)
        .interact_opt()?;

    Ok(selection)
}

/// 进入备用屏幕缓冲区（和 less/vim 同原理，退出后自动恢复主屏幕）
pub fn enter_alt_screen() {
    use std::io::Write;
    print!("\x1B[?1049h");
    let _ = std::io::stdout().flush();
}

/// 离开备用屏幕缓冲区，恢复主屏幕
pub fn leave_alt_screen() {
    use std::io::Write;
    print!("\x1B[?1049l");
    let _ = std::io::stdout().flush();
}

/// 等待按键返回
fn wait_for_key() {
    let dim = Style::new().dim();
    println!("\n{}", dim.apply_to("  按任意键返回列表..."));
    let _ = console::Term::stdout().read_key();
}

/// 单选笔记列表 + 循环查看（选中查看后回到列表，q 退出）
pub fn select_note_loop(notes: &[Note], viewer: impl Fn(usize) -> Result<()>) -> Result<()> {
    if notes.is_empty() {
        return Ok(());
    }

    let items: Vec<String> = notes.iter().map(format_note_line).collect();
    let prompt = list_prompt();
    let mut last_selected: usize = 0;

    loop {
        let selection = Select::with_theme(&NoteSelectTheme)
            .with_prompt(&prompt)
            .items(&items)
            .default(last_selected)
            .max_length(LIST_PAGE_SIZE)
            .interact_opt()?;

        match selection {
            Some(idx) => {
                last_selected = idx;
                enter_alt_screen();
                println!();
                let result = viewer(idx);
                if result.is_ok() {
                    wait_for_key();
                }
                leave_alt_screen();
                result?;
            }
            None => break,
        }
    }

    Ok(())
}

/// 带分组标题的循环选择（用于 search 分层结果）
pub fn select_note_grouped_loop(
    notes: &[Note],
    groups: &[(usize, &str)],
    viewer: impl Fn(usize) -> Result<()>,
) -> Result<()> {
    if notes.is_empty() {
        return Ok(());
    }

    // 构建带分组标题的显示列表
    let dim = Style::new().dim();
    let mut items: Vec<String> = Vec::new();
    let mut item_to_note: Vec<Option<usize>> = Vec::new(); // 选项索引 → 笔记索引（None = 分组标题）

    for (gi, &(start, label)) in groups.iter().enumerate() {
        let end = groups
            .get(gi + 1)
            .map_or(notes.len(), |&(s, _)| s);

        // 分组标题行
        items.push(format!("{}", dim.apply_to(format!("── {label} ──"))));
        item_to_note.push(None);

        for (i, note) in notes.iter().enumerate().take(end).skip(start) {
            items.push(format!("  {}", format_note_line(note)));
            item_to_note.push(Some(i));
        }
    }

    let prompt = list_prompt();
    let mut last_selected: usize = 1; // 跳过第一个分组标题

    loop {
        let selection = Select::with_theme(&NoteSelectTheme)
            .with_prompt(&prompt)
            .items(&items)
            .default(last_selected)
            .max_length(LIST_PAGE_SIZE)
            .interact_opt()?;

        match selection {
            Some(idx) => {
                if let Some(note_idx) = item_to_note[idx] {
                    last_selected = idx;
                    enter_alt_screen();
                    println!();
                    let result = viewer(note_idx);
                    if result.is_ok() {
                        wait_for_key();
                    }
                    leave_alt_screen();
                    result?;
                }
                // 选中分组标题行则忽略，继续选择
            }
            None => break,
        }
    }

    Ok(())
}

/// 多选笔记列表，返回选中笔记的索引列表
pub fn multi_select_notes(notes: &[Note], prompt: &str) -> Result<Vec<usize>> {
    if notes.is_empty() {
        return Ok(Vec::new());
    }

    let items: Vec<String> = notes.iter().map(format_note_line).collect();

    let selections = MultiSelect::new()
        .with_prompt(prompt)
        .items(&items)
        .interact()?;

    Ok(selections)
}

/// 支持宽字符（中文）的文本输入，替代 dialoguer::Input::interact_text()
///
/// dialoguer 在左右方向键移动光标时固定移动 1 列，不考虑中文等宽字符占 2 列的情况，
/// 导致光标位置错乱。此函数使用 unicode-width 按实际显示宽度移动光标。
pub(crate) fn read_input(prompt: &str, allow_empty: bool, default: Option<&str>) -> Result<String> {
    let term = Term::stderr();

    loop {
        // 构造 prompt（匹配 dialoguer SimpleTheme 格式）
        let prompt_str = match default {
            Some(def) => format!("{} [{}]: ", prompt, def),
            None => format!("{}: ", prompt),
        };

        term.write_str(&prompt_str)?;
        term.flush()?;

        let mut chars: Vec<char> = Vec::new();
        let mut pos: usize = 0; // 字符索引

        loop {
            match term.read_key()? {
                Key::Enter => break,
                Key::Backspace if pos > 0 => {
                    pos -= 1;
                    chars.remove(pos);
                    redraw_input(&term, &prompt_str, &chars, pos)?;
                }
                Key::Del if pos < chars.len() => {
                    chars.remove(pos);
                    redraw_input(&term, &prompt_str, &chars, pos)?;
                }
                Key::Char(ch) if !ch.is_ascii_control() => {
                    chars.insert(pos, ch);
                    pos += 1;
                    redraw_input(&term, &prompt_str, &chars, pos)?;
                }
                Key::ArrowLeft if pos > 0 => {
                    pos -= 1;
                    let w = UnicodeWidthChar::width(chars[pos]).unwrap_or(1);
                    term.move_cursor_left(w)?;
                    term.flush()?;
                }
                Key::ArrowRight if pos < chars.len() => {
                    let w = UnicodeWidthChar::width(chars[pos]).unwrap_or(1);
                    term.move_cursor_right(w)?;
                    pos += 1;
                    term.flush()?;
                }
                Key::Home => {
                    let cols: usize = chars[..pos]
                        .iter()
                        .map(|c| UnicodeWidthChar::width(*c).unwrap_or(1))
                        .sum();
                    if cols > 0 {
                        term.move_cursor_left(cols)?;
                    }
                    pos = 0;
                    term.flush()?;
                }
                Key::End => {
                    let cols: usize = chars[pos..]
                        .iter()
                        .map(|c| UnicodeWidthChar::width(*c).unwrap_or(1))
                        .sum();
                    if cols > 0 {
                        term.move_cursor_right(cols)?;
                    }
                    pos = chars.len();
                    term.flush()?;
                }
                _ => {}
            }
        }

        let input: String = chars.iter().collect();
        let trimmed = input.trim().to_string();

        // 清除输入行，显示最终结果
        term.clear_line()?;

        if trimmed.is_empty() {
            if let Some(def) = default {
                term.write_str(&format!("{}: {}\n", prompt, def))?;
                term.flush()?;
                return Ok(def.to_string());
            }
            if allow_empty {
                term.write_str(&format!("{}: \n", prompt))?;
                term.flush()?;
                return Ok(String::new());
            }
            // 不允许空输入，重新提示
            continue;
        }

        term.write_str(&format!("{}: {}\n", prompt, trimmed))?;
        term.flush()?;
        return Ok(trimmed);
    }
}

/// 清除当前行并重绘 prompt + 输入内容，将光标定位到 pos 对应的显示位置
fn redraw_input(term: &Term, prompt: &str, chars: &[char], pos: usize) -> Result<()> {
    term.clear_line()?;
    let text: String = chars.iter().collect();
    term.write_str(prompt)?;
    term.write_str(&text)?;

    // 将光标从末尾回退到 pos 的位置
    let tail_width: usize = chars[pos..]
        .iter()
        .map(|c| UnicodeWidthChar::width(*c).unwrap_or(1))
        .sum();
    if tail_width > 0 {
        term.move_cursor_left(tail_width)?;
    }
    term.flush()?;
    Ok(())
}

/// 标题输入
pub fn input_title(default: Option<&str>) -> Result<Option<String>> {
    let prompt = if default.is_some() {
        "标题 (回车使用默认值)"
    } else {
        "标题 (回车跳过，将以时间戳命名)"
    };

    let input = read_input(prompt, true, default)?;

    if input.is_empty() {
        Ok(None)
    } else {
        Ok(Some(input))
    }
}

/// 分类选择器（窄终端自动降级为数字输入模式）
pub fn select_category(categories: &[String]) -> Result<String> {
    let mut items: Vec<String> = categories.to_vec();
    items.push("+ 新建分类...".to_string());

    let term = console::Term::stdout();
    let (_, cols) = term.size();

    let selection = if cols < 40 {
        // 窄终端降级：数字输入模式
        for (i, cat) in items.iter().enumerate() {
            println!("  {}) {cat}", i + 1);
        }
        let input = read_input(&format!("输入序号 [1-{}]", items.len()), false, None)?;
        let n: usize = input
            .trim()
            .parse()
            .map_err(|_| anyhow::anyhow!("请输入有效序号"))?;
        if n == 0 || n > items.len() {
            bail!("序号超出范围");
        }
        n - 1
    } else {
        // 正常模式：dialoguer::Select
        Select::new()
            .with_prompt("分类")
            .items(&items)
            .default(0)
            .interact()?
    };

    if selection == items.len() - 1 {
        // 新建分类
        let trimmed = read_input("新分类名", false, None)?;
        if trimmed.is_empty() {
            bail!("分类名不能为空");
        }
        crate::config::save_category(&trimmed)?;
        Ok(trimmed)
    } else {
        Ok(items[selection].clone())
    }
}

/// 标签输入（显示已有标签作为补全提示）
pub fn input_tags() -> Result<Vec<String>> {
    // 加载已有标签列表作为提示
    let known_tags = crate::config::load_tags().unwrap_or_default();
    if !known_tags.is_empty() {
        let display: Vec<&str> = known_tags.iter().take(20).map(|s| s.as_str()).collect();
        println!("  已有标签: {}", display.join(", "));
    }

    loop {
        let input = read_input("标签 (逗号分隔, 回车跳过)", true, None)?;

        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Ok(Vec::new());
        }

        match crate::utils::parse_tags_input(trimmed, false) {
            Ok(tags) => return Ok(tags),
            Err(e) => {
                eprintln!("✗ {e}");
                // 循环重新输入
            }
        }
    }
}

/// 确认对话框
pub fn confirm(prompt: &str, default: bool) -> Result<bool> {
    let result = Confirm::new()
        .with_prompt(prompt)
        .default(default)
        .interact()?;
    Ok(result)
}

/// 同名冲突交互选择
pub fn conflict_choice(filename: &str) -> Result<ConflictAction> {
    let items = [
        "覆盖现有笔记",
        "追加在现有文件末尾",
        "追加时间戳后缀保存",
        "取消",
    ];

    println!("文件已存在: {filename}");
    let selection = Select::new()
        .with_prompt("请选择")
        .items(&items)
        .default(0)
        .interact()?;

    match selection {
        0 => Ok(ConflictAction::Overwrite),
        1 => Ok(ConflictAction::Append),
        2 => Ok(ConflictAction::Suffix),
        _ => Ok(ConflictAction::Cancel),
    }
}

/// 同名冲突处理方式
pub enum ConflictAction {
    Overwrite,
    Append,
    Suffix,
    Cancel,
}

/// 按显示宽度左对齐填充到 width 列
pub fn pad_display(s: &str, width: usize) -> String {
    let display_w = UnicodeWidthStr::width(s);
    if display_w >= width {
        s.to_string()
    } else {
        format!("{}{}", s, " ".repeat(width - display_w))
    }
}

/// 按显示宽度截断，超出部分用 "…" 替代
pub fn truncate_display(s: &str, width: usize) -> String {
    let display_w = UnicodeWidthStr::width(s);
    if display_w <= width {
        return s.to_string();
    }
    let mut current_w = 0;
    let mut result = String::new();
    for ch in s.chars() {
        let ch_w = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
        if current_w + ch_w > width - 1 {
            break;
        }
        result.push(ch);
        current_w += ch_w;
    }
    result.push('…');
    result
}

/// 格式化笔记行用于列表显示（带颜色）
pub fn format_note_line(note: &Note) -> String {
    let date = format_note_date(note);
    let tags_str = if note.tags.is_empty() {
        String::new()
    } else {
        note.tags.join(",")
    };

    // 先对纯文本做宽度填充/截断，再包裹颜色
    let date_padded = pad_display(&date, 16);
    let cat_padded = pad_display(&note.category, 6);
    let tags_padded = pad_display(&truncate_display(&tags_str, 16), 16);

    let dim = Style::new().dim();
    let cyan = Style::new().cyan();
    let green = Style::new().green();

    if note.is_deleted {
        let red = Style::new().red();
        format!(
            "{}  {}  {}  {} {}",
            dim.apply_to(&date_padded),
            cat_padded,
            cyan.apply_to(&tags_padded),
            red.apply_to("[已删除]"),
            note.title,
        )
    } else {
        format!(
            "{}  {}  {}  {}",
            dim.apply_to(&date_padded),
            cat_padded,
            cyan.apply_to(&tags_padded),
            green.apply_to(&note.title),
        )
    }
}

/// 从笔记的 created 字段提取 "MM-DD 周X HH:mm" 格式
fn format_note_date(note: &Note) -> String {
    // created 格式: "2026-03-12 周四 14:30"
    let parts: Vec<&str> = note.created.splitn(2, ' ').collect();
    if parts.len() < 2 {
        return note.created.clone();
    }
    // 取月-日
    let date_part = parts[0]; // "2026-03-12"
    let rest = parts[1]; // "周四 14:30"
    if date_part.len() >= 10 {
        format!("{} {}", &date_part[5..], rest)
    } else {
        note.created.clone()
    }
}
