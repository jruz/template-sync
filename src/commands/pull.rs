use crate::core::apply;
use crate::core::manifest;
use crate::core::preview;
use crate::core::report;
use crate::core::snapshot;
use crate::core::source;
use crate::utils::output::Output;
use crate::utils::style::Style;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Apply(#[from] apply::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Manifest(#[from] manifest::Error),
    #[error(transparent)]
    Snapshot(#[from] snapshot::Error),
    #[error(transparent)]
    Source(#[from] source::Error),
}

pub fn run(dry_run: bool, style: Style) -> Result<Output, Error> {
    let project = std::env::current_dir()?;
    let config = manifest::project(&project)?;
    let template = source::clone(&config.template)?;
    let base = source::clone_at(&config.template, &config.revision)?;
    let roots = snapshot::Roots {
        base: base.path(),
        project: &project,
        template: template.path(),
    };
    let findings = snapshot::findings(&roots, &config, snapshot::Review::Upstream)?;
    if dry_run {
        return Ok(Output {
            success: true,
            text: preview::render(&findings, style),
        });
    }
    let written = apply::write(&project, roots.template, &findings)?;
    let revision = source::head(&template)?;
    manifest::record(&project, config, &revision)?;
    let updates = written
        .iter()
        .map(|path| format!("updated {path}\n"))
        .collect::<Vec<_>>()
        .concat();
    let rendered = report::render(&findings, style);
    Ok(Output {
        success: true,
        text: format!("{updates}revision {revision}\n{rendered}"),
    })
}
