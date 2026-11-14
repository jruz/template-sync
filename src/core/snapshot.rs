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

#[derive(Clone, Copy)]
pub enum Review {
    Project,
    Upstream,
}

pub struct Roots<'path> {
    pub base: &'path Path,
    pub project: &'path Path,
    pub template: &'path Path,
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

fn read_in(root: &Path, path: &str) -> Result<Option<String>, Error> {
    let target = root.join(path);
    read_optional(&target)
}

fn entry_finding(
    roots: &Roots<'_>,
    entry: &manifest::Entry,
    review: Review,
) -> Result<compare::Finding, Error> {
    let template_path = roots.template.join(&entry.path);
    let template = read(&template_path)?;
    let base = match (&entry.strategy, review) {
        (manifest::Strategy::Review, Review::Project) => None,
        _ => read_in(roots.base, &entry.path)?,
    };
    let project = read_in(roots.project, &entry.path)?;
    let contents = compare::Contents {
        base: base.as_deref(),
        project: project.as_deref(),
        template: &template,
    };
    let compared = compare::finding(entry, &contents)?;
    Ok(compared)
}

fn removal(roots: &Roots<'_>, entry: &manifest::Entry) -> Result<Option<compare::Finding>, Error> {
    let project = read_in(roots.project, &entry.path)?;
    let found = project.map(|text| compare::removal(entry, &text));
    Ok(found)
}

fn removals(roots: &Roots<'_>, config: &manifest::Project) -> Result<Vec<compare::Finding>, Error> {
    let base_manifest = manifest::load(roots.base)?;
    let gone = manifest::removed(&base_manifest, roots.base, roots.template, config)?;
    let found = gone
        .iter()
        .map(|entry| removal(roots, entry))
        .collect::<Result<Vec<_>, _>>()?;
    let present = found.into_iter().flatten().collect();
    Ok(present)
}

pub fn findings(
    roots: &Roots<'_>,
    config: &manifest::Project,
    review: Review,
) -> Result<Vec<compare::Finding>, Error> {
    let template_manifest = manifest::load(roots.template)?;
    let current = manifest::entries(&template_manifest, roots.template, config)?
        .iter()
        .map(|entry| entry_finding(roots, entry, review))
        .collect::<Result<Vec<_>, _>>()?;
    let gone = removals(roots, config)?;
    let all = current.into_iter().chain(gone).collect();
    Ok(all)
}
