/// 配置管理模块：目录初始化、分类/标签配置读写
use anyhow::{Context, Result};
use std::path::PathBuf;

/// 默认分类列表
const DEFAULT_CATEGORIES: &str = "未分类\n工作\n生活\n技术\n其他\n";

/// 获取笔记目录路径 ~/.notes/
pub fn notes_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().context("无法获取用户主目录")?;
    Ok(home.join(".notes"))
}

/// 初始化目录和配置文件
pub fn ensure_init() -> Result<()> {
    let dir = notes_dir()?;

    // 创建目录
    std::fs::create_dir_all(&dir)?;
    std::fs::create_dir_all(dir.join(".config"))?;
    std::fs::create_dir_all(dir.join("merged"))?;
    std::fs::create_dir_all(dir.join("deleted"))?;

    // 创建默认 categories 文件
    let categories_path = dir.join(".config").join("categories");
    if !categories_path.exists() {
        std::fs::write(&categories_path, DEFAULT_CATEGORIES)?;
    }

    // 创建空 tags 文件
    let tags_path = dir.join(".config").join("tags");
    if !tags_path.exists() {
        std::fs::write(&tags_path, "")?;
    }

    Ok(())
}

/// 读取分类列表
pub fn load_categories() -> Result<Vec<String>> {
    let path = notes_dir()?.join(".config").join("categories");
    if !path.exists() {
        return Ok(vec!["未分类".to_string()]);
    }
    let content = std::fs::read_to_string(&path)?;
    Ok(content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.to_string())
        .collect())
}

/// 追加一个新分类到配置文件
pub fn save_category(name: &str) -> Result<()> {
    let path = notes_dir()?.join(".config").join("categories");
    let mut content = if path.exists() {
        std::fs::read_to_string(&path)?
    } else {
        String::new()
    };
    if !content.ends_with('\n') && !content.is_empty() {
        content.push('\n');
    }
    content.push_str(name);
    content.push('\n');
    std::fs::write(&path, content)?;
    Ok(())
}

/// 读取标签列表
pub fn load_tags() -> Result<Vec<String>> {
    let path = notes_dir()?.join(".config").join("tags");
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = std::fs::read_to_string(&path)?;
    Ok(content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.to_string())
        .collect())
}

/// 将新标签追加到配置文件（大小写不敏感去重）
pub fn save_tags(new_tags: &[String]) -> Result<()> {
    let path = notes_dir()?.join(".config").join("tags");
    let existing = load_tags()?;
    let existing_lower: Vec<String> = existing.iter().map(|t| t.to_lowercase()).collect();

    let mut content = if path.exists() {
        std::fs::read_to_string(&path)?
    } else {
        String::new()
    };

    for tag in new_tags {
        let lower = tag.to_lowercase();
        if !existing_lower.contains(&lower) {
            if !content.ends_with('\n') && !content.is_empty() {
                content.push('\n');
            }
            content.push_str(tag);
            content.push('\n');
        }
    }

    std::fs::write(&path, content)?;
    Ok(())
}

/// 确保分类存在于配置中，不存在则自动追加
pub fn ensure_category_exists(name: &str) -> Result<()> {
    let categories = load_categories()?;
    let lower = name.to_lowercase();
    if !categories.iter().any(|c| c.to_lowercase() == lower) {
        save_category(name)?;
    }
    Ok(())
}
