use crate::core::manifest;
use crate::core::report;
use crate::core::snapshot;
use crate::core::source;
use crate::utils::output::Output;
use crate::utils::style::Style;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Manifest(#[from] manifest::Error),
    #[error(transparent)]
    Snapshot(#[from] snapshot::Error),
    #[error(transparent)]
    Source(#[from] source::Error),
}

pub fn run(style: Style) -> Result<Output, Error> {
    let project = std::env::current_dir()?;
    let config = manifest::project(&project)?;
    let template = source::clone(&config.template)?;
    let template_root = template.path();
    let template_manifest = manifest::load(template_root)?;
    let findings = snapshot::findings(&project, template_root, &template_manifest, &config)?;
    Ok(Output {
        success: !report::failed(&findings),
        text: report::render(&findings, style),
    })
}
