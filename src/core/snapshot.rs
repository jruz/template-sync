use std::path::{Path, PathBuf};

use crate::core::compare;
use crate::core::manifest;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Compare(#[from] compare::Error),
    #[error(transparent)]
    Manifest(#[from] manifest::Error),
    #[error("cannot read {0}")]
    Read(PathBuf, #[source] std::io::Error),
}

fn read_error(path: &Path, source: std::io::Error) -> Error {
    let owned = path.to_path_buf();
    Error::Read(owned, source)
}

fn read(path: &Path) -> Result<String, Error> {
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(text),
        Err(source) => {
            let error = read_error(path, source);
            Err(error)
        }
    }
}

fn exists(path: &Path) -> Result<bool, Error> {
    match path.try_exists() {
        Ok(found) => Ok(found),
        Err(source) => {
            let error = read_error(path, source);
            Err(error)
        }
    }
}

fn read_optional(path: &Path) -> Result<Option<String>, Error> {
    if !exists(path)? {
        return Ok(None);
    }
    let text = read(path)?;
    let found = Some(text);
    Ok(found)
}

fn entry_finding(
    project: &Path,
    template: &Path,
    entry: &manifest::Entry,
) -> Result<compare::Finding, Error> {
    let template_path = template.join(&entry.path);
    let project_path = project.join(&entry.path);
    let expected = read(&template_path)?;
    let actual = read_optional(&project_path)?;
    let actual_text = actual.as_deref();
    let compared = compare::finding(entry, &expected, actual_text)?;
    Ok(compared)
}

pub fn findings(
    project: &Path,
    template: &Path,
    manifest: &manifest::Manifest,
    config: &manifest::Project,
) -> Result<Vec<compare::Finding>, Error> {
    manifest::entries(manifest, template, config)?
        .iter()
        .map(|entry| entry_finding(project, template, entry))
        .collect()
}
