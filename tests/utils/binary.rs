use std::path::Path;
use std::process::{Command, Output};

use crate::utils::env;

const FAKE_BIN: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/bin");

pub fn command(directory: &Path, origin: &Path) -> Command {
    let location = origin.to_str().expect("UTF-8 path");
    let rewrite = format!("url.file://{location}/.insteadOf");
    let path = env::path();
    let log = origin.join("nix-calls");
    let mut command = Command::new(env!("CARGO_BIN_EXE_template-sync"));
    command
        .current_dir(directory)
        .env("PATH", format!("{FAKE_BIN}:{path}"))
        .env("NIX_LOG", log)
        .env("GIT_CONFIG_COUNT", "1")
        .env("GIT_CONFIG_KEY_0", rewrite)
        .env(
            "GIT_CONFIG_VALUE_0",
            "https://x-access-token@github.com/jruz/",
        )
        .env("GIT_AUTHOR_NAME", "test")
        .env("GIT_AUTHOR_EMAIL", "test@example.com")
        .env("GIT_COMMITTER_NAME", "test")
        .env("GIT_COMMITTER_EMAIL", "test@example.com");
    command
}

pub fn run(directory: &Path, origin: &Path, arguments: &[&str]) -> Output {
    command(directory, origin)
        .args(arguments)
        .output()
        .expect("run CLI")
}

pub fn stdout(output: &Output) -> String {
    let bytes = output.stdout.clone();
    String::from_utf8(bytes).expect("UTF-8 output")
}

pub fn stderr(output: &Output) -> String {
    let bytes = output.stderr.clone();
    String::from_utf8(bytes).expect("UTF-8 output")
}
