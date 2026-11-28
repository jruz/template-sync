use std::path::{Path, PathBuf};
use std::time::Duration;

use super::project::{self, Project};
use super::scaffold;
use crate::core::formatter;
use crate::core::source;
use crate::libs::git;
use crate::utils::process;

#[derive(Debug)]
pub struct Prepared {
    destination: PathBuf,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("destination already exists: {0}")]
    Destination(PathBuf),
    #[error(transparent)]
    Formatter(#[from] formatter::Error),
    #[error(transparent)]
    Git(#[from] git::Error),
    #[error("invalid Woodpecker repository information for {0}")]
    InvalidRepo(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("setup stopped; inspect {0} and {1} before retrying")]
    Partial(String, PathBuf, #[source] Box<Self>),
    #[error(transparent)]
    Process(#[from] process::Error),
    #[error(transparent)]
    Scaffold(#[from] scaffold::Error),
    #[error(transparent)]
    Source(#[from] source::Error),
    #[error("GitHub generated a different tree than the template snapshot")]
    TemplateChanged,
    #[error("Woodpecker did not discover {0}")]
    Undiscovered(String),
}

pub fn check_destination(path: &Path) -> Result<(), Error> {
    let path = path.to_path_buf();
    match path.symlink_metadata() {
        Ok(_) => {
            let error = Error::Destination(path);
            Err(error)
        }
        Err(error) => {
            if error.kind() == std::io::ErrorKind::NotFound {
                Ok(())
            } else {
                let error = Error::Io(error);
                Err(error)
            }
        }
    }
}

pub fn destination(setup: &Prepared) -> &Path {
    &setup.destination
}

fn activate(root: &Path, repository: &str) -> Result<(), Error> {
    let format = "{{ .FullName }} {{ .ForgeRemoteID }} {{ .IsActive }}";
    let repos = process::run(
        "woodpecker-cli",
        &["repo", "sync", "--format", format],
        root,
    )?;
    let entry = repos.lines().find_map(|line| {
        let (name, fields) = line.split_once(' ')?;
        name.eq_ignore_ascii_case(repository).then_some(fields)
    });
    let repository = repository.to_owned();
    let Some(fields) = entry else {
        let error = Error::Undiscovered(repository);
        return Err(error);
    };
    let Some((id, active)) = fields.split_once(' ') else {
        let error = Error::InvalidRepo(repository);
        return Err(error);
    };
    match active {
        "true" => {}
        "false" if !id.is_empty() && id.bytes().all(|byte| byte.is_ascii_digit()) => {
            process::run("woodpecker-cli", &["repo", "add", id], root)?;
        }
        _ => {
            let error = Error::InvalidRepo(repository);
            return Err(error);
        }
    }
    Ok(())
}

pub fn settings_plans(root: &Path) -> Result<String, Error> {
    let github = process::run("github-settings", &["plan"], root)?;
    let woodpecker = process::run("woodpecker-settings", &["plan"], root)?;
    let plans = format!("GitHub settings:\n{github}\nWoodpecker settings:\n{woodpecker}");
    Ok(plans)
}

fn wait_for_branch(parent: &Path, remote: &str) -> Result<(), Error> {
    let arguments = ["ls-remote", "--exit-code", remote, "HEAD"];
    let mut attempts = 10_u8;
    let mut status = process::exit_code("git", &arguments, parent)?;
    while status == 2_i32 && attempts > 0 {
        let delay = Duration::from_secs(1);
        std::thread::sleep(delay);
        attempts -= 1;
        status = process::exit_code("git", &arguments, parent)?;
    }
    if status != 0_i32 {
        process::run("git", &arguments, parent)?;
    }
    Ok(())
}

fn setup_project(template: project::Template, root: &Path) -> Result<(), Error> {
    match template {
        project::Template::Rust => {
            process::run("just", &["setup"], root)?;
        }
        project::Template::Typescript => {
            process::run(
                "nix",
                &[
                    "develop",
                    "--no-update-lock-file",
                    "--command",
                    "just",
                    "setup",
                ],
                root,
            )?;
        }
    }
    Ok(())
}

fn clone_and_activate(
    project: &Project,
    parent: &Path,
    path: &Path,
    repository: &str,
    revision: &str,
    tree: &str,
) -> Result<(), Error> {
    let remote = format!("git@github.com:{repository}.git");
    wait_for_branch(parent, &remote)?;
    let name = project::name(project);
    process::run("git", &["clone", "--", &remote, name], parent)?;
    let actual = process::run("git", &["show", "-s", "--format=%T", "HEAD"], path)?;
    if actual != tree {
        return Err(Error::TemplateChanged);
    }
    scaffold::write(path, project, revision)?;
    let mut files = vec![
        "github-settings.json",
        "woodpecker-settings.yaml",
        "template-sync.json",
        "README.md",
    ];
    let template = project::template(project);
    if matches!(template, project::Template::Typescript) {
        files.push("package.json");
    }
    formatter::files(path, &files)?;
    setup_project(template, path)?;
    activate(path, repository)?;
    Ok(())
}

pub fn prepare(project: &Project, parent: &Path, destination: PathBuf) -> Result<Prepared, Error> {
    for helper in [
        "github-settings",
        "nix",
        "woodpecker-cli",
        "woodpecker-settings",
    ] {
        process::run(helper, &["--version"], parent)?;
    }
    let template = project::template(project);
    let setup_tool = match template {
        project::Template::Rust => "just",
        project::Template::Typescript => "nix",
    };
    process::run(setup_tool, &["--version"], parent)?;
    let template = project::template_repository(template);
    let checkout = source::clone(template)?;
    let revision = source::head(&checkout)?;
    let snapshot = checkout.path();
    let tree = process::run("git", &["show", "-s", "--format=%T", "HEAD"], snapshot)?;
    let repository = project::repository(project);
    let visibility = project::visibility(project);
    let visibility = format!("--{visibility}");
    process::run(
        "gh",
        &[
            "repo",
            "create",
            &repository,
            "--template",
            template,
            &visibility,
        ],
        parent,
    )?;
    if let Err(source) =
        clone_and_activate(project, parent, &destination, &repository, &revision, &tree)
    {
        let source = Box::new(source);
        let error = Error::Partial(repository, destination, source);
        return Err(error);
    }
    Ok(Prepared { destination })
}

pub fn finish(setup: &Prepared) -> Result<String, Error> {
    let root = destination(setup);
    process::run("woodpecker-settings", &["apply"], root)?;
    git::commit_all(root, "[feat] Project: Initialize from template")?;
    process::run("git", &["push", "origin", "HEAD"], root)?;
    process::run("github-settings", &["push"], root)?;
    let commit = git::head(root)?;
    Ok(commit)
}
