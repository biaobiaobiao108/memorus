use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;

use memos::cli::{self, Cli, CliError};
use memos::db::Database;
use memos::tui;

fn main() -> ExitCode {
    let cli = Cli::parse();
    let format = cli.format;

    match run(&cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            cli::print_error(&error, format);
            ExitCode::from(error.exit_code())
        }
    }
}

fn run(cli: &Cli) -> Result<(), CliError> {
    let db = open_database(cli)?;
    if cli.opens_tui() {
        tui::run(db)?;
    } else {
        cli::execute(&db, cli)?;
    }
    Ok(())
}

fn open_database(cli: &Cli) -> Result<Database, CliError> {
    let path = cli.db.clone().or_else(|| {
        std::env::var_os("MEMOS_DB_PATH")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
    });

    match path {
        Some(path) => Ok(Database::open(&path)?),
        None => Ok(Database::open_default()?),
    }
}
