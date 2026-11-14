mod commands;
mod core;
mod libs;
mod utils;

use std::io::IsTerminal as _;
use std::process::ExitCode;

use clap::Parser as _;
use commands::{check, init, pull, update_repositories, version};
use utils::error;
use utils::output::Output;
use utils::style::Style;

#[derive(Debug, clap::Subcommand)]
enum Command {
    #[command(about = "Compare the project with its template")]
    Check {
        #[arg(long, help = "Compare local review files with the current template")]
        review: bool,
    },
    #[command(about = "Make the project follow the template that matches its files")]
    Init,
    #[command(about = "Pull template changes into the project")]
    Pull {
        #[arg(long, help = "List the files that would change without writing them")]
        dry_run: bool,
    },
    #[command(about = "Open pull requests for every repository of an account that is out of sync")]
    UpdateRepositories {
        #[arg(long, help = "Report differences without pushing")]
        dry_run: bool,
        #[arg(long, help = "GitHub user or organisation to scan")]
        account: String,
    },
    #[command(about = "Print version and build metadata")]
    Version,
}

#[derive(Debug, clap::Parser)]
#[command(
    version = version::text(),
    about = "Keep repositories in sync with their template"
)]
struct Arguments {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error(transparent)]
    Check(#[from] check::Error),
    #[error(transparent)]
    Init(#[from] init::Error),
    #[error(transparent)]
    Pull(#[from] pull::Error),
    #[error(transparent)]
    UpdateRepositories(#[from] update_repositories::Error),
}

fn run(command: Command, style: Style) -> Result<Output, Error> {
    let output = match command {
        Command::Check { review } => check::run(review, style)?,
        Command::Init => init::run()?,
        Command::Pull { dry_run } => pull::run(dry_run, style)?,
        Command::UpdateRepositories { dry_run, account } => {
            update_repositories::run(&account, dry_run, style)?
        }
        Command::Version => version::run(),
    };
    Ok(output)
}

fn style() -> Style {
    if std::io::stdout().is_terminal() {
        return Style::Color;
    }
    Style::Plain
}

fn main() -> ExitCode {
    let arguments = Arguments::parse();
    let chosen = style();
    let success = match run(arguments.command, chosen) {
        Ok(output) => {
            print!("{}", output.text);
            output.success
        }
        Err(error) => {
            let chain = error::chain(&error);
            eprintln!("error: {chain}");
            false
        }
    };
    if success {
        return ExitCode::SUCCESS;
    }
    ExitCode::FAILURE
}
