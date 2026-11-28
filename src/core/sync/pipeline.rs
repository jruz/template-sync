use std::path::Path;

use super::apply;
use crate::core::compare;
use crate::core::formatter;
use crate::core::manifest;
use crate::core::source;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Apply(#[from] apply::Error),
    #[error(transparent)]
    Formatter(#[from] formatter::Error),
    #[error(transparent)]
    Manifest(#[from] manifest::Error),
    #[error(transparent)]
    Source(#[from] source::Error),
}

pub fn apply(
    project: &Path,
    template: &tempfile::TempDir,
    config: manifest::Project,
    findings: &[compare::Finding],
) -> Result<String, Error> {
    let template_root = template.path();
    apply::write(project, template_root, findings)?;
    formatter::findings(project, findings)?;
    let revision = source::head(template)?;
    manifest::record(project, config, &revision)?;
    formatter::manifest(project)?;
    Ok(revision)
}
