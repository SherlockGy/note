/// CLI 命令定义（clap derive 模式）
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "note",
    about = "轻量级命令行备忘录工具",
    version,
    arg_required_else_help = false
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// 记录笔记（文本/文件）
    Add {
        /// 标题 = 文件名
        #[arg(short = 'T', long)]
        title: Option<String>,

        /// 标签，逗号分隔
        #[arg(short = 't', long)]
        tags: Option<String>,

        /// 分类
        #[arg(short, long)]
        category: Option<String>,

        /// 打开终端编辑器写内容
        #[arg(short = 'e', long = "edit")]
        edit: bool,

        /// 从文件读取内容
        #[arg(short, long = "file")]
        file: Option<PathBuf>,

        /// 静默模式，跳过所有交互
        #[arg(short, long)]
        quiet: bool,

        /// 笔记内容（以 - 开头时请用 -- 分隔，如: note add -- -内容）
        content: Vec<String>,
    },

    /// 列出笔记 + 交互选择
    List {
        /// 显示条数
        #[arg(short = 'n', default_value = "10")]
        count: usize,

        /// 按标签筛选
        #[arg(short = 't', long)]
        tags: Option<String>,

        /// 按分类筛选
        #[arg(short, long)]
        category: Option<String>,

        /// 包含已删除笔记
        #[arg(short, long)]
        all: bool,
    },

    /// 搜索笔记
    #[command(alias = "s")]
    Search {
        /// 搜索关键词
        #[arg(required = true)]
        keywords: Vec<String>,

        /// 额外标签过滤
        #[arg(short = 't', long)]
        tags: Option<String>,

        /// 额外分类过滤
        #[arg(short, long)]
        category: Option<String>,

        /// 包含已删除笔记
        #[arg(short, long)]
        all: bool,
    },

    /// 查看笔记内容
    Show {
        /// 序号或文件名/模糊词
        target: Option<String>,
    },

    /// 编辑已有笔记
    Edit {
        /// 序号或文件名/模糊词
        target: Option<String>,
    },

    /// 合并多条笔记
    Merge {
        /// 序号或文件名列表
        targets: Vec<String>,

        /// 按标签批量合并
        #[arg(short = 't', long)]
        tags: Option<String>,
    },

    /// 清理笔记到 deleted/
    Rm {
        /// 文件名/模糊词列表
        targets: Vec<String>,
    },

    /// 列出标签及使用次数
    Tags,
}
