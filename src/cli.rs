use anyhow::Result;
use clap::Parser;

use crate::db::Database;

#[derive(Parser, Debug)]
#[command(
    name = "memos",
    author = "steelway",
    version,
    about = "极速终端备忘录工具 (TUI & CLI)",
    long_about = "输入 memos 进入交互式 TUI 备忘录管理；输入 memos -l 快速在终端中查看最近 6 条备忘录。"
)]
pub struct Cli {
    /// 在终端中快速查看最近 6 条备忘录的标题
    #[arg(short = 'l', long = "list")]
    pub list: bool,
}

pub fn handle_cli_list(db: &Database) -> Result<()> {
    let recent = db.get_recent(6)?;
    if recent.is_empty() {
        println!("📭 暂无备忘录。输入 `memos` 进入 TUI 创建你的第一条备忘录吧！");
        return Ok(());
    }

    println!("📝 最近备忘录 (最新 {} 条):", recent.len());
    println!("──────────────────────────────────────────────────");
    for (idx, memo) in recent.iter().enumerate() {
        let time_str = memo.created_at.format("%m-%d %H:%M").to_string();
        println!("  {}. [{}] {}", idx + 1, time_str, memo.title);
    }
    println!("──────────────────────────────────────────────────");
    println!("💡 提示: 运行 `memos` 进入 TUI 查看完整详情或进行编辑");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_list() -> Result<()> {
        let db = Database::open_in_memory()?;
        // 空列表测试
        assert!(handle_cli_list(&db).is_ok());

        // 插入多条备忘录
        for i in 1..=10 {
            db.insert(&format!("备忘录项目 {}", i), &format!("内容 {}", i))?;
        }

        let recent = db.get_recent(6)?;
        assert_eq!(recent.len(), 6);
        assert_eq!(recent[0].title, "备忘录项目 10");
        assert_eq!(recent[5].title, "备忘录项目 5");

        assert!(handle_cli_list(&db).is_ok());
        Ok(())
    }
}

