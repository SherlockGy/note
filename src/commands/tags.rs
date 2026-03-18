/// note tags — 标签统计
use anyhow::Result;
use console::Style;
use std::collections::HashMap;

use crate::note;

/// 执行 note tags 命令
pub fn run() -> Result<()> {
    let notes = note::load_notes(false)?;

    // 统计每个标签出现次数（大小写不敏感合并，保留首次出现的大小写）
    let mut counts: HashMap<String, usize> = HashMap::new();
    let mut display: HashMap<String, String> = HashMap::new();

    for n in &notes {
        for tag in &n.tags {
            let lower = tag.to_lowercase();
            *counts.entry(lower.clone()).or_insert(0) += 1;
            display.entry(lower).or_insert_with(|| tag.clone());
        }
    }

    if counts.is_empty() {
        println!("没有标签");
        return Ok(());
    }

    // 按次数降序排列
    let mut sorted: Vec<(String, usize)> = counts.into_iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(&a.1));

    let cyan = Style::new().cyan();
    let dim = Style::new().dim();

    for (lower, count) in &sorted {
        let name = display.get(lower).unwrap_or(lower);
        println!("{} {}", cyan.apply_to(name), dim.apply_to(format!("({count})")));
    }

    Ok(())
}
