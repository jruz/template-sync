use std::path::Path;

use crate::core::compare;
use crate::core::manifest;
use crate::utils::process;

const PROGRAM: &str = "nix";
const COMMAND: [&str; 4] = ["run", "nixpkgs#oxfmt", "--", "--write"];
const EXTENSIONS: [&str; 3] = ["json", "yaml", "yml"];

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Process(#[from] process::Error),
}

fn written_structured(finding: &compare::Finding) -> bool {
    let extension = Path::new(&finding.path).extension();
    compare::needs_update(finding)
        && finding.status != compare::Status::Removed
        && extension.is_some_and(|extension| EXTENSIONS.iter().any(|known| extension == *known))
}

pub fn files(project: &Path, files: &[&str]) -> Result<(), Error> {
    let files = files.iter().copied();
    let arguments = COMMAND.into_iter().chain(files).collect::<Vec<_>>();
    process::run(PROGRAM, &arguments, project)?;
    Ok(())
}

pub fn findings(project: &Path, findings: &[compare::Finding]) -> Result<(), Error> {
    let written = findings
        .iter()
        .filter(|finding| written_structured(finding))
        .map(|finding| finding.path.as_str())
        .collect::<Vec<_>>();
    if written.is_empty() {
        Ok(())
    } else {
        files(project, &written)
    }
}

pub fn manifest(project: &Path) -> Result<(), Error> {
    files(project, &[manifest::PROJECT_FILE])
}
