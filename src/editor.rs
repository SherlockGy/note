/// 编辑器检测与打开逻辑
use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;

/// 查找编辑器，返回 (命令, 额外参数)
/// 优先级：code --wait → $VISUAL → $EDITOR → vi（Windows: notepad）
pub fn find_editor() -> (String, Vec<String>) {
    if find_vscode() {
        return ("code".into(), vec!["--wait".into()]);
    }
    if let Ok(v) = std::env::var("VISUAL") {
        return (v, vec![]);
    }
    if let Ok(v) = std::env::var("EDITOR") {
        return (v, vec![]);
    }
    if cfg!(windows) {
        ("notepad".into(), vec![])
    } else {
        ("vi".into(), vec![])
    }
}

/// 检测 VS Code 是否可用
fn find_vscode() -> bool {
    which::which("code").is_ok()
}

/// 执行编辑器并阻塞等待
/// Windows 上 Command::new("code") 无法直接执行 .cmd 文件，
/// 必须通过 cmd /C 启动
pub fn open_editor(editor: &str, args: &[String], file: &Path) -> Result<()> {
    let status = if cfg!(windows) && editor == "code" {
        // Windows 上 Command::new("code") 找不到 code.cmd，
        // 需要通过 cmd /C 启动，逐个传参让 Rust 自动处理转义
        Command::new("cmd")
            .arg("/C")
            .arg("code")
            .args(args)
            .arg(file)
            .status()
            .with_context(|| "无法通过 cmd /C 启动 VS Code")?
    } else {
        Command::new(editor)
            .args(args)
            .arg(file)
            .status()
            .with_context(|| format!("无法启动编辑器: {editor}"))?
    };

    if !status.success() {
        anyhow::bail!("编辑器退出异常 (exit code: {:?})", status.code());
    }
    Ok(())
}
