use std::io::{self, Read, Write};
use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use serde_json::json;

use crate::db::Database;
use crate::model::Memo;

#[derive(Parser, Debug)]
#[command(
    name = "memos",
    author = "steelway",
    version,
    about = "极速终端备忘录工具 (TUI & CLI)",
    long_about = "不带子命令时进入交互式 TUI；也可使用 list、get、create、update 等子命令供脚本和 Agent 调用。"
)]
pub struct Cli {
    /// 兼容旧版：查看最近 6 条未归档备忘录
    #[arg(short = 'l', long = "list")]
    pub legacy_list: bool,

    /// 指定 SQLite 数据库路径，优先级高于 MEMOS_DB_PATH
    #[arg(long, global = true, value_name = "PATH")]
    pub db: Option<PathBuf>,

    /// 设置 CLI 输出格式
    #[arg(long, global = true, value_enum, default_value = "table")]
    pub format: OutputFormat,

    #[command(subcommand)]
    pub command: Option<Command>,
}

impl Cli {
    pub fn opens_tui(&self) -> bool {
        !self.legacy_list && self.command.is_none()
    }
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// 列出备忘录
    List {
        /// 同时显示活动和已归档备忘录
        #[arg(long, conflicts_with = "archived")]
        all: bool,

        /// 仅显示已归档备忘录
        #[arg(long)]
        archived: bool,

        /// 在标题和正文中搜索关键词
        #[arg(long, short = 'q', value_name = "TEXT")]
        query: Option<String>,

        /// 最多返回多少条记录
        #[arg(long, default_value_t = 20)]
        limit: usize,
    },

    /// 按 ID 查看单条备忘录
    Get {
        id: i64,

        /// 只输出指定字段
        #[arg(long, value_enum)]
        field: Option<MemoField>,
    },

    /// 创建备忘录；--content - 表示从 stdin 读取正文
    Create {
        #[arg(long, short = 't')]
        title: String,

        #[arg(long, short = 'c', default_value = "", allow_hyphen_values = true)]
        content: String,
    },

    /// 局部更新备忘录；--content - 表示从 stdin 读取正文
    Update {
        id: i64,

        #[arg(long, short = 't', required_unless_present = "content")]
        title: Option<String>,

        #[arg(
            long,
            short = 'c',
            required_unless_present = "title",
            allow_hyphen_values = true
        )]
        content: Option<String>,
    },

    /// 归档备忘录
    Archive { id: i64 },

    /// 恢复已归档备忘录
    Restore { id: i64 },

    /// 永久删除备忘录
    Delete {
        id: i64,

        /// 确认执行不可恢复的删除
        #[arg(long)]
        yes: bool,
    },
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Table,
    Json,
    Jsonl,
    Plain,
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoField {
    Id,
    Title,
    Content,
    Archived,
    CreatedAt,
    UpdatedAt,
}

#[derive(Debug)]
pub enum CliError {
    NotFound(i64),
    ConfirmationRequired(i64),
    InvalidInput(String),
    Other(anyhow::Error),
}

impl CliError {
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::InvalidInput(_) => 2,
            Self::NotFound(_) => 3,
            Self::ConfirmationRequired(_) => 4,
            Self::Other(_) => 1,
        }
    }

    fn kind(&self) -> &'static str {
        match self {
            Self::InvalidInput(_) => "invalid_input",
            Self::NotFound(_) => "not_found",
            Self::ConfirmationRequired(_) => "confirmation_required",
            Self::Other(_) => "runtime_error",
        }
    }

    fn message(&self) -> String {
        match self {
            Self::NotFound(id) => format!("未找到 ID 为 {id} 的备忘录"),
            Self::ConfirmationRequired(id) => {
                format!("删除备忘录 {id} 需要显式传入 --yes")
            }
            Self::InvalidInput(message) => message.clone(),
            Self::Other(error) => format!("{error:#}"),
        }
    }
}

impl From<anyhow::Error> for CliError {
    fn from(error: anyhow::Error) -> Self {
        Self::Other(error)
    }
}

pub fn print_error(error: &CliError, format: OutputFormat) {
    match format {
        OutputFormat::Json | OutputFormat::Jsonl => {
            eprintln!(
                "{}",
                json!({
                    "error": {
                        "code": error.kind(),
                        "message": error.message(),
                    }
                })
            );
        }
        OutputFormat::Table | OutputFormat::Plain => {
            eprintln!("错误: {}", error.message());
        }
    }
}

pub fn execute(db: &Database, cli: &Cli) -> std::result::Result<(), CliError> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    execute_with_io(db, cli, &mut stdin.lock(), &mut stdout.lock())
}

fn execute_with_io<R: Read, W: Write>(
    db: &Database,
    cli: &Cli,
    input: &mut R,
    output: &mut W,
) -> std::result::Result<(), CliError> {
    if cli.legacy_list && cli.command.is_some() {
        return Err(CliError::InvalidInput(
            "--list 兼容参数不能与子命令同时使用".to_string(),
        ));
    }

    if cli.legacy_list {
        let memos = db.get_recent(6)?;
        return write_memos(output, &memos, cli.format).map_err(Into::into);
    }

    let command = cli.command.as_ref().ok_or_else(|| {
        CliError::InvalidInput("缺少 CLI 子命令；直接运行 memos 可进入 TUI".to_string())
    })?;

    match command {
        Command::List {
            all,
            archived,
            query,
            limit,
        } => {
            let mut memos = db.get_all()?;
            memos.retain(|memo| *all || memo.archived == *archived);

            if let Some(query) = query.as_deref().map(str::trim).filter(|q| !q.is_empty()) {
                let query = query.to_lowercase();
                memos.retain(|memo| {
                    memo.title.to_lowercase().contains(&query)
                        || memo.content.to_lowercase().contains(&query)
                });
            }

            memos.truncate(*limit);
            write_memos(output, &memos, cli.format)?;
        }
        Command::Get { id, field } => {
            let memo = require_memo(db, *id)?;
            if let Some(field) = field {
                write_field(output, &memo, *field)?;
            } else {
                write_memo(output, &memo, cli.format)?;
            }
        }
        Command::Create { title, content } => {
            let title = validate_title(title)?;
            let content = resolve_content(content, input)?;
            let memo = db.insert(title, &content)?;
            write_memo(output, &memo, cli.format)?;
        }
        Command::Update { id, title, content } => {
            let current = require_memo(db, *id)?;
            let title = match title {
                Some(title) => validate_title(title)?.to_string(),
                None => current.title,
            };
            let content = match content {
                Some(content) => resolve_content(content, input)?,
                None => current.content,
            };

            db.update(*id, &title, &content)?;
            let memo = require_memo(db, *id)?;
            write_memo(output, &memo, cli.format)?;
        }
        Command::Archive { id } => {
            require_memo(db, *id)?;
            db.set_archived(*id, true)?;
            let memo = require_memo(db, *id)?;
            write_memo(output, &memo, cli.format)?;
        }
        Command::Restore { id } => {
            require_memo(db, *id)?;
            db.set_archived(*id, false)?;
            let memo = require_memo(db, *id)?;
            write_memo(output, &memo, cli.format)?;
        }
        Command::Delete { id, yes } => {
            if !yes {
                return Err(CliError::ConfirmationRequired(*id));
            }
            require_memo(db, *id)?;
            db.delete(*id)?;
            write_delete_result(output, *id, cli.format)?;
        }
    }

    Ok(())
}

fn require_memo(db: &Database, id: i64) -> std::result::Result<Memo, CliError> {
    db.get_by_id(id)?.ok_or(CliError::NotFound(id))
}

fn validate_title(title: &str) -> std::result::Result<&str, CliError> {
    let title = title.trim();
    if title.is_empty() {
        Err(CliError::InvalidInput("标题不能为空".to_string()))
    } else {
        Ok(title)
    }
}

fn resolve_content<R: Read>(value: &str, input: &mut R) -> std::result::Result<String, CliError> {
    if value != "-" {
        return Ok(value.to_string());
    }

    let mut content = String::new();
    input.read_to_string(&mut content).map_err(|error| {
        CliError::Other(anyhow::Error::new(error).context("无法从 stdin 读取正文"))
    })?;
    Ok(content)
}

fn write_memos<W: Write>(output: &mut W, memos: &[Memo], format: OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Table => {
            writeln!(output, "ID\t状态\t更新时间\t标题")?;
            for memo in memos {
                writeln!(
                    output,
                    "{}\t{}\t{}\t{}",
                    memo.id,
                    if memo.archived { "已归档" } else { "活动" },
                    memo.updated_at.format("%Y-%m-%d %H:%M:%S"),
                    sanitize_line(&memo.title)
                )?;
            }
        }
        OutputFormat::Json => {
            writeln!(output, "{}", serde_json::to_string(memos)?)?;
        }
        OutputFormat::Jsonl => {
            for memo in memos {
                writeln!(output, "{}", serde_json::to_string(memo)?)?;
            }
        }
        OutputFormat::Plain => {
            for memo in memos {
                writeln!(output, "{}\t{}", memo.id, sanitize_line(&memo.title))?;
            }
        }
    }
    Ok(())
}

fn write_memo<W: Write>(output: &mut W, memo: &Memo, format: OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Table => {
            writeln!(output, "ID: {}", memo.id)?;
            writeln!(output, "标题: {}", memo.title)?;
            writeln!(
                output,
                "状态: {}",
                if memo.archived { "已归档" } else { "活动" }
            )?;
            writeln!(output, "创建时间: {}", memo.created_at.to_rfc3339())?;
            writeln!(output, "更新时间: {}", memo.updated_at.to_rfc3339())?;
            writeln!(output, "正文:")?;
            write!(output, "{}", memo.content)?;
            if !memo.content.ends_with('\n') {
                writeln!(output)?;
            }
        }
        OutputFormat::Json | OutputFormat::Jsonl => {
            writeln!(output, "{}", serde_json::to_string(memo)?)?;
        }
        OutputFormat::Plain => {
            writeln!(
                output,
                "{}\t{}\t{}",
                memo.id,
                if memo.archived { "archived" } else { "active" },
                sanitize_line(&memo.title)
            )?;
        }
    }
    Ok(())
}

fn write_field<W: Write>(output: &mut W, memo: &Memo, field: MemoField) -> Result<()> {
    match field {
        MemoField::Id => writeln!(output, "{}", memo.id)?,
        MemoField::Title => writeln!(output, "{}", memo.title)?,
        MemoField::Content => write!(output, "{}", memo.content)?,
        MemoField::Archived => writeln!(output, "{}", memo.archived)?,
        MemoField::CreatedAt => writeln!(output, "{}", memo.created_at.to_rfc3339())?,
        MemoField::UpdatedAt => writeln!(output, "{}", memo.updated_at.to_rfc3339())?,
    }
    Ok(())
}

fn write_delete_result<W: Write>(output: &mut W, id: i64, format: OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Table => writeln!(output, "已删除备忘录 {id}")?,
        OutputFormat::Json | OutputFormat::Jsonl => {
            writeln!(output, "{}", json!({ "deleted": true, "id": id }))?
        }
        OutputFormat::Plain => writeln!(output, "{id}")?,
    }
    Ok(())
}

fn sanitize_line(value: &str) -> String {
    value.replace(['\r', '\n', '\t'], " ")
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    fn run(db: &Database, args: &[&str], input: &str) -> (Cli, String) {
        let cli = Cli::try_parse_from(args).unwrap();
        let mut input = Cursor::new(input.as_bytes());
        let mut output = Vec::new();
        execute_with_io(db, &cli, &mut input, &mut output).unwrap();
        let output = String::from_utf8(output).unwrap();
        (cli, output)
    }

    #[test]
    fn test_crud_commands_and_stdin() -> Result<()> {
        let db = Database::open_in_memory()?;

        let (_, output) = run(
            &db,
            &[
                "memos",
                "create",
                "--title",
                "脚本记录",
                "--content",
                "-",
                "--format",
                "json",
            ],
            "来自 stdin\n",
        );
        let created: Memo = serde_json::from_str(output.trim())?;
        assert_eq!(created.title, "脚本记录");
        assert_eq!(created.content, "来自 stdin\n");

        let id = created.id.to_string();
        let (_, content) = run(&db, &["memos", "get", &id, "--field", "content"], "");
        assert_eq!(content, "来自 stdin\n");

        run(&db, &["memos", "update", &id, "--title", "更新标题"], "");
        assert_eq!(db.get_by_id(created.id)?.unwrap().content, "来自 stdin\n");

        run(&db, &["memos", "archive", &id], "");
        assert!(db.get_by_id(created.id)?.unwrap().archived);

        run(&db, &["memos", "restore", &id], "");
        assert!(!db.get_by_id(created.id)?.unwrap().archived);

        run(&db, &["memos", "delete", &id, "--yes"], "");
        assert!(db.get_by_id(created.id)?.is_none());
        Ok(())
    }

    #[test]
    fn test_list_filters_and_machine_formats() -> Result<()> {
        let db = Database::open_in_memory()?;
        let rust = db.insert("Rust", "ratatui")?;
        let archived = db.insert("旧记录", "archive")?;
        db.set_archived(archived.id, true)?;

        let (_, json_output) = run(
            &db,
            &["memos", "list", "--query", "rust", "--format", "json"],
            "",
        );
        let memos: Vec<Memo> = serde_json::from_str(json_output.trim())?;
        assert_eq!(memos.len(), 1);
        assert_eq!(memos[0].id, rust.id);

        let (_, jsonl_output) = run(&db, &["memos", "list", "--all", "--format", "jsonl"], "");
        assert_eq!(jsonl_output.lines().count(), 2);

        let (_, archived_output) = run(
            &db,
            &["memos", "list", "--archived", "--format", "plain"],
            "",
        );
        assert!(archived_output.contains("旧记录"));
        assert!(!archived_output.contains("Rust"));
        Ok(())
    }

    #[test]
    fn test_errors_have_stable_exit_codes() -> Result<()> {
        let db = Database::open_in_memory()?;
        let memo = db.insert("待删除", "")?;
        let id = memo.id.to_string();

        let cli = Cli::try_parse_from(["memos", "delete", &id]).unwrap();
        let error = execute_with_io(&db, &cli, &mut io::empty(), &mut io::sink()).unwrap_err();
        assert_eq!(error.exit_code(), 4);

        let cli = Cli::try_parse_from(["memos", "get", "999"]).unwrap();
        let error = execute_with_io(&db, &cli, &mut io::empty(), &mut io::sink()).unwrap_err();
        assert_eq!(error.exit_code(), 3);

        let cli = Cli::try_parse_from(["memos", "--list", "list"]).unwrap();
        let error = execute_with_io(&db, &cli, &mut io::empty(), &mut io::sink()).unwrap_err();
        assert_eq!(error.exit_code(), 2);
        Ok(())
    }

    #[test]
    fn test_global_options_work_with_subcommands() {
        let cli = Cli::try_parse_from([
            "memos",
            "list",
            "--db",
            "/tmp/agent.db",
            "--format",
            "jsonl",
        ])
        .unwrap();

        assert_eq!(cli.db, Some(PathBuf::from("/tmp/agent.db")));
        assert_eq!(cli.format, OutputFormat::Jsonl);
        assert!(matches!(cli.command, Some(Command::List { .. })));
    }

    #[test]
    fn test_legacy_list_flag() -> Result<()> {
        let db = Database::open_in_memory()?;
        for index in 0..10 {
            db.insert(&format!("备忘录 {index}"), "")?;
        }

        let (_, output) = run(&db, &["memos", "-l", "--format", "json"], "");
        let memos: Vec<Memo> = serde_json::from_str(output.trim())?;
        assert_eq!(memos.len(), 6);
        Ok(())
    }
}
