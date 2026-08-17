use anyhow::Result;
use clap::Parser;

use memos::cli::{self, Cli};
use memos::db::Database;
use memos::tui;

fn main() -> Result<()> {
    let cli = Cli::parse();
    let db = Database::open_default()?;

    if cli.list {
        cli::handle_cli_list(&db)?;
    } else {
        tui::run(db)?;
    }

    Ok(())
}
