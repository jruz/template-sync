mod commands;
mod utils;

use std::process::ExitCode;

use clap::Parser as _;
use commands::version;
use utils::output::Output;

#[derive(Debug, clap::Subcommand)]
enum Command {
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

fn run(command: &Command) -> Output {
    match command {
        Command::Version => version::run(),
    }
}

fn main() -> ExitCode {
    let arguments = Arguments::parse();
    let output = run(&arguments.command);
    print!("{}", output.text);
    if output.success {
        return ExitCode::SUCCESS;
    }
    ExitCode::FAILURE
}
