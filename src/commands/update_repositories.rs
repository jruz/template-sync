use crate::core::account;
use crate::utils::output::Output;
use crate::utils::style::Style;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Account(#[from] account::Error),
}

pub fn run(account: &str, dry_run: bool, style: Style) -> Result<Output, Error> {
    let repositories = account::sync(account, dry_run, style)?;
    let output = account::output(&repositories);
    Ok(output)
}
