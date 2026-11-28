use std::path::{Path, PathBuf};

use crate::core::compare;
use crate::core::manifest;
use crate::libs::git;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("uncommitted changes in template targets:\n{0}")]
    DirtyTargets(String),
    #[error(transparent)]
    Git(#[from] git::Error),
    #[error("cannot write {0}")]
    Write(PathBuf, #[source] std::io::Error),
}

fn written(path: &Path, result: std::io::Result<()>) -> Result<(), Error> {
    match result {
        Ok(()) => Ok(()),
        Err(source) => {
            let owned = path.to_path_buf();
            let error = Error::Write(owned, source);
            Err(error)
        }
    }
}

fn replace(source: &Path, target: &Path) -> std::io::Result<()> {
    std::fs::copy(source, target)?;
    Ok(())
}

fn write_file(project: &Path, template: &Path, finding: &compare::Finding) -> Result<(), Error> {
    let target = project.join(&finding.path);
    if finding.status == compare::Status::Removed {
        let removed = std::fs::remove_file(&target);
        return written(&target, removed);
    }
    if let Some(parent) = target.parent() {
        let created = std::fs::create_dir_all(parent);
        written(parent, created)?;
    }
    let result = match finding.strategy {
        manifest::Strategy::Replace => {
            let source = template.join(&finding.path);
            replace(&source, &target)
        }
        manifest::Strategy::Cargo
        | manifest::Strategy::GithubSettings
        | manifest::Strategy::Justfile
        | manifest::Strategy::Lines
        | manifest::Strategy::Package
        | manifest::Strategy::Review
        | manifest::Strategy::WoodpeckerSettings => std::fs::write(&target, &finding.content),
    };
    written(&target, result)
}

pub fn write(project: &Path, template: &Path, findings: &[compare::Finding]) -> Result<(), Error> {
    let targets = findings
        .iter()
        .filter(|finding| compare::needs_update(finding))
        .collect::<Vec<_>>();
    let paths = targets
        .iter()
        .map(|finding| finding.path.clone())
        .collect::<Vec<_>>();
    let dirty = git::dirty(project, &paths)?;
    if !dirty.is_empty() {
        let listed = dirty.join("\n");
        let error = Error::DirtyTargets(listed);
        return Err(error);
    }
    targets
        .iter()
        .try_for_each(|finding| write_file(project, template, finding))
}
