# note

轻量级命令行备忘录工具，笔记以 Markdown 文件存储在 `~/.notes/`，支持标签、分类、全文搜索和交互选择。

## 安装

```bash
cargo build --release
# 将 target/release/note 复制到 PATH 中
```

## 快速上手

```bash
note add 今天学了 Rust 的生命周期        # 直接记录文本
note add -e                              # 打开编辑器写内容
note list                                # 列出最近 10 条笔记（交互模式）
note search rust                         # 搜索关键词
```

## 命令

### `note add` — 记录笔记

```bash
note add <内容>                          # 直接输入文本
note add -e                             # 打开终端编辑器
note add -f report.md                   # 导入文本文件
echo "内容" | note add                  # 管道输入（自动静默）
```

**可选参数：**

| 参数 | 说明 |
|------|------|
| `-T <标题>` | 指定标题（同时作为文件名） |
| `-t <标签>` | 逗号分隔的标签，如 `rust,cli` |
| `-c <分类>` | 分类名，未指定时可交互选择 |
| `-e` | 打开终端编辑器 |
| `-f <文件>` | 从文件读取内容 |
| `-q` | 静默模式，跳过所有交互提示 |

内容开头含 `-` 时，用 `--` 分隔：`note add -- -这是内容`

**内容来源优先级：** `-f 文件` > `-e 编辑器` > 位置参数 > 管道 stdin

---

### `note list` — 列出笔记

```bash
note list                   # 最近 10 条（交互模式可上下选择回车查看）
note list -n 20             # 显示 20 条
note list -t rust           # 只看带 rust 标签的
note list -c 技术           # 只看"技术"分类
note list -a                # 包含已删除的笔记
```

终端交互模式下，上下键浏览，回车预览，`q` 退出。

---

### `note search` / `note s` — 搜索笔记

```bash
note search rust cli        # 同时包含 rust 和 cli（所有关键词均须命中）
note s lifetime             # s 是 search 的别名
note search rust -t入门     # 关键词 + 标签过滤
note search bug -c 工作     # 关键词 + 分类过滤
note search 关键词 -a       # 包含已删除的笔记
```

搜索结果按优先级分组：**标题匹配 → 标签匹配 → 正文匹配**。

---

### `note show` — 查看笔记内容

```bash
note show                   # 不带参数，从列表交互选择
note show 3                 # 按序号（同 list 输出的编号）
note show rust笔记          # 按文件名模糊匹配
```

---

### `note edit` — 编辑笔记

```bash
note edit                   # 交互选择
note edit 2                 # 按序号
note edit "每日总结"        # 模糊匹配标题
```

用系统 `$EDITOR` 打开，保存退出后自动更新 `updated` 时间。

---

### `note merge` — 合并笔记

```bash
note merge                  # 交互多选（空格勾选，回车确认）
note merge 1 3 5            # 按序号指定
note merge -t rust          # 将带 rust 标签的笔记全部合并
```

合并后自动：标签取并集、分类投票选最常见、旧笔记移入 `~/.notes/merged/`。

---

### `note rm` — 删除笔记

```bash
note rm rust笔记            # 模糊匹配标题，移入 deleted/
note rm a.md b.md           # 精确文件名，支持多个
```

删除是软删除，文件移入 `~/.notes/deleted/`，可用 `note list -a` 或 `note search -a` 找回。

---

### `note tags` — 查看标签

```bash
note tags                   # 列出所有标签及使用次数
```

---

## AI Agent 集成（note-skill）

`note-skill/SKILL.md` 是一个 Claude Code Skill，让任何 AI Agent 都能直接读写你的笔记，无需手动调用 CLI。

### 安装

将 `note-skill/` 目录复制到 Claude Code 的 skills 路径，或在 `settings.json` 中注册：

```json
{
  "skills": ["./note-skill"]
}
```

### Agent 能做什么

安装后，在任何对话中自然表达意图即可触发：

| 你说的话 | Agent 的动作 |
|----------|-------------|
| "查一下我关于 Docker 的笔记" | 扫描 `~/.notes/`，按相关度返回摘要 |
| "我之前记过一个 MySQL 的解决方案" | 语义搜索，扩展同义词 `sql/db/数据库` |
| "帮我把这几条 React 笔记合并" | 理解内容、消除重复、整合结构，移旧建新 |
| "这条没用了，清理掉" | 确认后移入 `deleted/`（软删除） |

### CLI vs Agent 的区别

CLI 做精确匹配，Agent 理解语义——搜索"部署问题"时会自动扩展到 `docker/k8s/ci/cd` 等关键词；合并时会重新组织段落结构而非简单拼接。

---

## 存储结构

```
~/.notes/
├── 笔记文件.md         # 普通笔记
├── merged/             # 合并后的旧笔记
├── deleted/            # 软删除的笔记
└── .config/
    ├── categories      # 分类列表（每行一个）
    └── tags            # 标签记录
```

每条笔记是一个带 YAML frontmatter 的 Markdown 文件：

```markdown
---
category: 技术
tags: [rust, cli]
created: "2026-03-18 周三 14:30"
updated: "2026-03-18 周三 14:30"
---
笔记正文内容
```

默认分类：`未分类`、`工作`、`生活`、`技术`、`其他`（可在 `~/.notes/.config/categories` 中编辑）。
