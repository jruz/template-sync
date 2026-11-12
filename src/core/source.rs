use crate::libs::git;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Git(#[from] git::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

pub fn clone(repository: &str) -> Result<tempfile::TempDir, Error> {
    let directory = tempfile::tempdir()?;
    let destination = directory.path();
    git::clone(repository, destination)?;
    Ok(directory)
}
