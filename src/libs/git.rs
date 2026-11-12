use std::path::Path;

use crate::utils::process;

const CREDENTIAL_HELPER: &str = "credential.https://github.com.helper=!gh auth git-credential";

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Process(#[from] process::Error),
    #[error("git diff exited with {0}")]
    UnexpectedExit(i32),
}

fn git(directory: &Path, arguments: &[&str]) -> Result<String, Error> {
    let output = process::run("git", arguments, directory)?;
    Ok(output)
}

pub fn dirty(directory: &Path, paths: &[String]) -> Result<Vec<String>, Error> {
    if paths.is_empty() {
        let none = Vec::new();
        return Ok(none);
    }
    let options = ["status", "--porcelain", "--untracked-files=all", "--"];
    let targets = paths.iter().cloned();
    let arguments = options
        .into_iter()
        .map(str::to_owned)
        .chain(targets)
        .collect::<Vec<_>>();
    let output = process::run("git", &arguments, directory)?;
    let lines = output.lines().map(str::to_owned).collect();
    Ok(lines)
}

fn url(repository: &str) -> String {
    format!("https://x-access-token@github.com/{repository}.git")
}

pub fn clone(repository: &str, destination: &Path) -> Result<(), Error> {
    let url = url(repository);
    git(
        destination,
        &[
            "clone",
            "--quiet",
            "--depth",
            "1",
            "--config",
            CREDENTIAL_HELPER,
            &url,
            ".",
        ],
    )?;
    Ok(())
}

pub fn clone_at(repository: &str, revision: &str, destination: &Path) -> Result<(), Error> {
    let url = url(repository);
    git(destination, &["init", "--quiet"])?;
    git(
        destination,
        &[
            "-c",
            CREDENTIAL_HELPER,
            "fetch",
            "--quiet",
            "--depth",
            "1",
            &url,
            revision,
        ],
    )?;
    git(destination, &["checkout", "--quiet", "FETCH_HEAD"])?;
    Ok(())
}

pub fn head(directory: &Path) -> Result<String, Error> {
    git(directory, &["rev-parse", "HEAD"])
}

pub fn is_clean(directory: &Path) -> Result<bool, Error> {
    let output = git(directory, &["status", "--porcelain"])?;
    let clean = output.is_empty();
    Ok(clean)
}

pub fn revision(directory: &Path) -> Result<String, Error> {
    git(directory, &["rev-parse", "--short", "HEAD"])
}

pub fn branch(directory: &Path) -> Result<String, Error> {
    git(directory, &["rev-parse", "--abbrev-ref", "HEAD"])
}

pub fn checkout(directory: &Path, branch: &str) -> Result<(), Error> {
    git(directory, &["checkout", "-B", branch])?;
    Ok(())
}

pub fn commit_all(directory: &Path, message: &str) -> Result<(), Error> {
    git(directory, &["add", "--all"])?;
    git(directory, &["commit", "--message", message])?;
    Ok(())
}

pub fn matches_remote(directory: &Path, branch: &str) -> Result<bool, Error> {
    let heads = git(directory, &["ls-remote", "--heads", "origin", branch])?;
    if heads.is_empty() {
        return Ok(false);
    }
    git(directory, &["fetch", "--depth", "1", "origin", branch])?;
    let diff = ["diff", "--quiet", "FETCH_HEAD", "HEAD"];
    let code = process::exit_code("git", &diff, directory)?;
    match code {
        0 => Ok(true),
        1 => Ok(false),
        other => {
            let error = Error::UnexpectedExit(other);
            Err(error)
        }
    }
}

pub fn push(directory: &Path, branch: &str) -> Result<(), Error> {
    git(directory, &["push", "--force", "origin", branch])?;
    Ok(())
}
