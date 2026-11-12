use crate::core::detect;
use crate::core::manifest;
use crate::utils::output::Output;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Detect(#[from] detect::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Manifest(#[from] manifest::Error),
}

pub fn run() -> Result<Output, Error> {
    let project = std::env::current_dir()?;
    let template = detect::template(&project)?;
    manifest::create(&project, template)?;
    Ok(Output {
        success: true,
        text: format!(
            "created {} tracking {template}\nnext: template-sync check\n",
            manifest::PROJECT_FILE
        ),
    })
}
