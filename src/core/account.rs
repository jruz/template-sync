use std::path::Path;

use crate::core::apply;
use crate::core::compare;
use crate::core::manifest;
use crate::core::report;
use crate::core::snapshot;
use crate::libs::git;
use crate::libs::github;
use crate::utils::error;
use crate::utils::output::Output;
use crate::utils::style::Style;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Apply(#[from] apply::Error),
    #[error(transparent)]
    Git(#[from] git::Error),
    #[error(transparent)]
    GitHub(#[from] github::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Manifest(#[from] manifest::Error),
    #[error(transparent)]
    Snapshot(#[from] snapshot::Error),
}

#[derive(Debug, PartialEq, Eq)]
enum Outcome {
    InSync,
    Pending(String),
    Skipped,
    Unchanged,
    Updated,
}

pub struct Repository {
    name: String,
    result: Result<Outcome, Error>,
}

struct Checkout<'run> {
    config: &'run manifest::Project,
    findings: &'run [compare::Finding],
    name: &'run str,
    project: &'run Path,
    template: &'run Path,
}

fn publish(checkout: &Checkout<'_>) -> Result<Outcome, Error> {
    let template_repository = checkout.config.template.as_str();
    let name = manifest::template_name(template_repository)?;
    let branch = format!("template-sync/{name}");
    let base = git::branch(checkout.project)?;
    let revision = git::revision(checkout.template)?;
    let head = git::head(checkout.template)?;
    git::checkout(checkout.project, &branch)?;
    apply::write(checkout.project, checkout.template, checkout.findings)?;
    let config = checkout.config.clone();
    manifest::record(checkout.project, config, &head)?;
    if git::is_clean(checkout.project)? {
        return Ok(Outcome::InSync);
    }
    let title = format!("[tool] Template: Sync with {template_repository}");
    let message = format!("{title}@{revision}");
    git::commit_all(checkout.project, &message)?;
    if git::matches_remote(checkout.project, &branch)? {
        return Ok(Outcome::Unchanged);
    }
    git::push(checkout.project, &branch)?;
    let body = report::markdown(template_repository, &revision, checkout.findings);
    let request = github::PullRequest {
        base,
        body,
        branch,
        repository: checkout.name.to_owned(),
        title,
    };
    github::pull_request(checkout.project, &request)?;
    Ok(Outcome::Updated)
}

fn repository(name: &str, workspace: &Path, dry_run: bool, style: Style) -> Result<Outcome, Error> {
    let Some(config) = github::root_file(name, manifest::PROJECT_FILE, workspace)? else {
        return Ok(Outcome::Skipped);
    };
    let project_config = manifest::parse_project(&config)?;
    let project = tempfile::tempdir_in(workspace)?;
    let project_path = project.path();
    git::clone(name, project_path)?;
    let template = tempfile::tempdir_in(workspace)?;
    let template_path = template.path();
    git::clone(&project_config.template, template_path)?;
    let base = tempfile::tempdir_in(workspace)?;
    let base_path = base.path();
    git::clone_at(
        &project_config.template,
        &project_config.revision,
        base_path,
    )?;
    let roots = snapshot::Roots {
        base: base_path,
        project: project_path,
        template: template_path,
    };
    let findings = snapshot::findings(&roots, &project_config, snapshot::Review::Upstream)?;
    if !report::failed(&findings) {
        return Ok(Outcome::InSync);
    }
    if dry_run {
        let rendered = report::render(&findings, style);
        let pending = Outcome::Pending(rendered);
        return Ok(pending);
    }
    publish(&Checkout {
        config: &project_config,
        findings: &findings,
        name,
        project: project_path,
        template: template_path,
    })
}

fn describe(repository: &Repository) -> String {
    let status = match &repository.result {
        Ok(Outcome::InSync) => "in sync\n".to_owned(),
        Ok(Outcome::Pending(rendered)) => format!("out of sync\n{rendered}"),
        Ok(Outcome::Skipped) => "skipped\n".to_owned(),
        Ok(Outcome::Unchanged) => "pull request up to date\n".to_owned(),
        Ok(Outcome::Updated) => "pull request updated\n".to_owned(),
        Err(error) => {
            let chain = error::chain(error);
            format!("error: {chain}\n")
        }
    };
    format!("{}: {status}", repository.name)
}

pub fn sync(account: &str, dry_run: bool, style: Style) -> Result<Vec<Repository>, Error> {
    let workspace = tempfile::tempdir()?;
    let workspace_path = workspace.path();
    let names = github::repositories(account, workspace_path)?;
    let repositories = names
        .into_iter()
        .map(|name| {
            let result = repository(&name, workspace_path, dry_run, style);
            Repository { name, result }
        })
        .collect();
    Ok(repositories)
}

#[must_use]
pub fn output(repositories: &[Repository]) -> Output {
    let text = repositories
        .iter()
        .map(describe)
        .collect::<Vec<_>>()
        .concat();
    let failures = repositories
        .iter()
        .filter_map(|repository| repository.result.as_ref().err())
        .count();
    Output {
        success: failures == 0,
        text,
    }
}
