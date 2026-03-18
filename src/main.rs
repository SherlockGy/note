/// note-cli — 轻量级命令行备忘录工具
mod cli;
mod commands;
mod config;
mod editor;
mod interactive;
mod note;
mod utils;

use anyhow::Result;
use clap::Parser;

fn main() -> Result<()> {
    // 初始化目录和配置
    config::ensure_init()?;

    let cli = cli::Cli::parse();

    match cli.command {
        // 无子命令时等同 note list（默认 10 条）
        None => commands::list::run(10, None, None, false),

        Some(cmd) => match cmd {
            cli::Commands::Add {
                content,
                title,
                tags,
                category,
                edit,
                file,
                quiet,
            } => commands::add::run(content, title, tags, category, edit, file, quiet),

            cli::Commands::List {
                count,
                tags,
                category,
                all,
            } => commands::list::run(count, tags, category, all),

            cli::Commands::Search {
                keywords,
                tags,
                category,
                all,
            } => commands::search::run(keywords, tags, category, all),

            cli::Commands::Show { target } => commands::show::run(target),

            cli::Commands::Edit { target } => commands::edit::run(target),

            cli::Commands::Merge { targets, tags } => commands::merge::run(targets, tags),

            cli::Commands::Rm { targets } => commands::rm::run(targets),

            cli::Commands::Tags => commands::tags::run(),
        },
    }
}
