use std::path::Path;

use crate::utils::process;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Process(#[from] process::Error),
}

pub struct PullRequest {
    pub base: String,
    pub body: String,
    pub branch: String,
    pub repository: String,
    pub title: String,
}

fn gh(directory: &Path, arguments: &[&str]) -> Result<String, Error> {
    let output = process::run("gh", arguments, directory)?;
    Ok(output)
}

pub fn repositories(account: &str, directory: &Path) -> Result<Vec<String>, Error> {
    let output = gh(
        directory,
        &[
            "repo",
            "list",
            account,
            "--no-archived",
            "--source",
            "--limit",
            "1000",
            "--json",
            "nameWithOwner,isEmpty",
            "--jq",
            ".[] | select(.isEmpty | not) | .nameWithOwner",
        ],
    )?;
    let names = output.lines().map(str::to_owned).collect();
    Ok(names)
}

fn root_files(repository: &str, directory: &Path) -> Result<Vec<String>, Error> {
    let endpoint = format!("repos/{repository}/contents/");
    let output = gh(
        directory,
        &[
            "api",
            &endpoint,
            "--jq",
            ".[] | select(.type == \"file\") | .name",
        ],
    )?;
    let names = output.lines().map(str::to_owned).collect();
    Ok(names)
}

pub fn root_file(repository: &str, name: &str, directory: &Path) -> Result<Option<String>, Error> {
    let names = root_files(repository, directory)?;
    if !names.iter().any(|candidate| candidate == name) {
        return Ok(None);
    }
    let endpoint = format!("repos/{repository}/contents/{name}");
    let content = gh(
        directory,
        &[
            "api",
            &endpoint,
            "--header",
            "Accept: application/vnd.github.raw",
        ],
    )?;
    let found = Some(content);
    Ok(found)
}

fn open_pull_request(directory: &Path, branch: &str) -> Result<String, Error> {
    gh(
        directory,
        &[
            "pr",
            "list",
            "--head",
            branch,
            "--state",
            "open",
            "--json",
            "number",
            "--jq",
            ".[].number",
        ],
    )
}

fn update_body(directory: &Path, request: &PullRequest, number: &str) -> Result<(), Error> {
    let endpoint = format!("repos/{}/pulls/{number}", request.repository);
    let body = format!("body={}", request.body);
    gh(
        directory,
        &[
            "api",
            "--method",
            "PATCH",
            &endpoint,
            "--raw-field",
            &body,
            "--silent",
        ],
    )?;
    Ok(())
}

pub fn refresh_body(directory: &Path, request: &PullRequest) -> Result<(), Error> {
    let existing = open_pull_request(directory, &request.branch)?;
    if existing.is_empty() {
        Ok(())
    } else {
        update_body(directory, request, &existing)
    }
}

pub fn pull_request(directory: &Path, request: &PullRequest) -> Result<(), Error> {
    let existing = open_pull_request(directory, &request.branch)?;
    if existing.is_empty() {
        gh(
            directory,
            &[
                "pr",
                "create",
                "--base",
                &request.base,
                "--head",
                &request.branch,
                "--title",
                &request.title,
                "--body",
                &request.body,
            ],
        )?;
    } else {
        update_body(directory, request, &existing)?;
    }
    gh(
        directory,
        &["pr", "merge", &request.branch, "--auto", "--squash"],
    )?;
    Ok(())
}
