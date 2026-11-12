use crate::core::detect;
use crate::core::manifest;
use crate::core::source;
use crate::utils::output::Output;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Detect(#[from] detect::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Manifest(#[from] manifest::Error),
    #[error(transparent)]
    Source(#[from] source::Error),
}

pub fn run() -> Result<Output, Error> {
    let project = std::env::current_dir()?;
    let template = detect::template(&project)?;
    let checkout = source::clone(template)?;
    let revision = source::head(&checkout)?;
    manifest::create(&project, template, &revision)?;
    Ok(Output {
        success: true,
        text: format!(
            "created {} tracking {template}@{revision}\nnext: template-sync check\n",
            manifest::PROJECT_FILE
        ),
    })
}
