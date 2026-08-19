# Memos (备忘录) 📝

一款基于 Rust 编写的高性能、轻量级终端备忘录工具。支持现代化的交互式 TUI 界面管理，也支持在命令行中极速查看最近备忘录。

---

## ✨ 核心特性

- 🚀 **极速启动与响应**：纯 Rust 打造，毫秒级冷启动，资源占用极低。
- 🖥️ **现代化 TUI 双栏交互**：
  - 左侧备忘录列表，支持上下浏览与实时搜索；
  - 右侧详情预览，支持多行正文排版与滚动查看。
- ⚡ **CLI 极速查看**：运行 `memos -l` 直接在终端输出最近 6 条备忘录，无需进入界面。
- 💾 **SQLite 本地持久化**：
  - 内嵌 SQLite 引擎（`rusqlite bundled`），零外部系统依赖；
  - 自动创建时间索引，海量数据下依然保持高性能检索。
- 🇨🇳 **原生中文输入法 (IME) 支持**：
  - 接入精确的硬件光标坐标同步与 Unicode 字符宽度计算；
  - 拼音预编辑与候选词窗口精准定焦在文字末尾，中文打字体验丝滑。

---

## 📦 安装方法

### 方式一：下载预编译可执行文件（开箱即用）

前往 [GitHub Releases](../../releases) 下载对应系统的单一可执行文件，无需安装任何依赖：

- **Windows 64位**：下载 `memos-windows-x86_64.exe`，直接在命令提示符或 PowerShell 中运行。
- **macOS (Apple Silicon M系列芯片)**：下载 `memos-macos-arm64`，赋予可执行权限后即可运行：
  ```bash
  chmod +x memos-macos-arm64
  # 可选：移动到系统 PATH 路径
  sudo mv memos-macos-arm64 /usr/local/bin/memos
  ```
- **Debian / Ubuntu (Linux 64位)**：下载 `memos-linux-x86_64`，赋予可执行权限后即可运行：
  ```bash
  chmod +x memos-linux-x86_64
  # 可选：移动到系统 PATH 路径
  sudo mv memos-linux-x86_64 /usr/local/bin/memos
  ```

### 方式二：从源码直接安装

确保已安装 [Rust 与 Cargo](https://rustup.rs/)：

```bash
# 克隆仓库
git clone https://github.com/your-username/memos.git
cd memos

# 安装到本地 Cargo Bin 目录 (~/.cargo/bin)
cargo install --path .
```

### 方式三：本地编译运行

```bash
cargo build --release
./target/release/memos
```

---

## 🎯 使用指南

### 1. 交互式 TUI 模式

在终端直接运行：

```bash
memos
```

进入双栏交互管理界面：

```text
 📝 MEMOS  极速终端备忘录
┌─ 备忘录列表 (3) ─────────────────┐┌─ 详情预览 ───────────────────────────────┐
│ ▶ [08-18 10:30] 准备周报选题     ││ 📌 标题: 准备周报选题                      │
│   [08-17 19:42] 视频脚本大纲     ││ 🕒 创建: 2026-08-18 10:30:15               │
│   [08-16 14:15] 购买麦克风支架   ││ ────────────────────────────────────────── │
│                                  ││ 1. 梳理各视频播放数据与完播率               │
│                                  ││ 2. 确定下期人物叙事选题核心切入点           │
└──────────────────────────────────┘└─────────────────────────────────────────────┘
[Tab]切换备忘/归档 [g]归档/恢复 [a]新建 [e]编辑 [d]删除 [/]搜索 [j/k/↑/↓/点击]选择 [q/Esc]退出
```

#### 🎹 快捷键与鼠标操作一览

| 快捷键 / 鼠标 | 功能 | 说明 |
| :--- | :--- | :--- |
| **鼠标滚轮** | **长文平滑翻页** | 鼠标滚轮向上/向下滚动右侧长文正文预览 |
| **鼠标左键点击** | **选择备忘录** | 点击列表任意一行直接选中查看详情 |
| `Tab` | **切换列表视图** | 在「活动备忘录」与「已归档备忘录」之间切换（在弹窗内用于切换输入框） |
| `g` | **归档 / 取消归档** | 归档选中的备忘录，或在归档列表中将备忘录恢复至活动列表 |
| `a` | **新建备忘录** | 弹出窗口，填写标题和多行正文 |
| `e` / `Enter` | **编辑备忘录** | 修改当前选中的备忘录 |
| `d` / `Delete` | **删除备忘录** | 弹出确认提示框（按 `y` 确认，`n/Esc` 取消） |
| `/` | **实时搜索** | 按关键词实时过滤备忘录（支持标题和正文搜索） |
| `j` / `k` 或 `↓` / `↑` | **上下导航** | 在备忘录列表中移动光标 |
| `Space` / `u` / `PageDown` / `PageUp` | **正文翻页** | 上下翻页查看长正文内容（支持 `Ctrl+D` / `Ctrl+U`、`[` / `]`） |
| `Ctrl + S` | **保存** | 保存新建或编辑内容并写入 SQLite |
| `q` / `Esc` | **退出** | 退出搜索、弹窗或退出程序 |

---

### 2. CLI 与脚本模式

CLI 提供完整的增删改查能力，不带子命令时仍然进入 TUI。旧版的 `memos -l` / `memos --list` 保留为“查看最近 6 条”的兼容入口。

```bash
# 查询
memos list
memos list --query "Rust" --limit 10
memos list --archived
memos list --all --format jsonl
memos get 42
memos get 42 --field content

# 创建与修改
memos create --title "会议记录" --content "正文"
printf '多行正文\n第二行\n' | memos create --title "stdin 示例" --content -
memos update 42 --title "更新后的标题"
memos update 42 --content - < revised.md

# 归档、恢复与删除
memos archive 42
memos restore 42
memos delete 42 --yes
```

全局 `--format` 支持四种输出格式：

| 格式 | 用途 |
| :--- | :--- |
| `table` | 默认的人类可读输出 |
| `json` | 单个 JSON 对象或 JSON 数组 |
| `jsonl` | 每行一个 JSON 对象，适合流式处理 |
| `plain` | 简洁的制表符分隔输出 |

机器模式只向 stdout 写入结果，错误写入 stderr。`get --field content` 会原样输出正文，不额外添加换行，方便管道处理：

```bash
memos list --format json | jq '.[].id'
memos get 42 --field content > memo.md
```

CLI 使用稳定退出码：`0` 成功、`1` 运行错误、`2` 参数或输入错误、`3` 记录不存在、`4` 删除缺少 `--yes`。

---

## 📂 数据存储位置

Memos 默认将 SQLite 数据库存储在操作系统标准数据目录下：

- **Linux / macOS**: `~/.local/share/memos/memos.db`
- **Windows**: `%LOCALAPPDATA%\memos\data\memos.db`

你可以随时备份该 `.db` 文件，迁移简单无负担。

脚本、测试或 Agent 可以使用 `--db` 隔离数据库，也可以设置 `MEMOS_DB_PATH`。命令行参数优先级更高：

```bash
memos --db /tmp/task-memos.db list --format json
MEMOS_DB_PATH=./project.db memos create --title "项目笔记"
```

---

## 🛠️ 技术栈

- **TUI 界面框架**：[ratatui](https://github.com/ratatui/ratatui) + [crossterm](https://github.com/crossterm-rs/crossterm)
- **数据库驱动**：[rusqlite](https://github.com/rusqlite/rusqlite) (bundled)
- **命令行解析**：[clap](https://github.com/clap-rs/clap)
- **时间与路径**：[chrono](https://github.com/chronotope/chrono) + [directories](https://github.com/dirs-dev/directories-rs)

---

## 📄 开源许可

本项目基于 [MIT License](LICENSE) 开源。
