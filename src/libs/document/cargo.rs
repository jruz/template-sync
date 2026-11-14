use crate::libs::document::toml;

pub const FILE: &str = "Cargo.toml";

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Toml(#[from] toml::Error),
}

pub fn sync(template: &str, base: Option<&str>, project: &str) -> Result<String, Error> {
    let keys = ["build-dependencies", "lints", "profile"].map(str::to_owned);
    let synced = toml::keys(template, base, project, &keys)?;
    Ok(synced)
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASE: &str = "[build-dependencies]\nthiserror = \"=1.0.0\"\nold = \"=1.0.0\"\n[lints.clippy]\nold = \"allow\"\n[profile.release]\nlto = false\n[profile.obsolete]\ninherits = \"release\"\n";
    const TEMPLATE: &str = "[package]\nname = \"template\"\n[dependencies]\nclap = \"=9.0.0\"\n[build-dependencies]\nthiserror = \"=2.0.0\"\n[lints.clippy]\nnew = \"deny\"\n[profile.release]\nlto = true\n";
    const PROJECT: &str = "[package]\nname = \"project\"\nversion = \"3.0.0\"\n[dependencies]\nclap = \"=1.0.0\"\n[build-dependencies]\nthiserror = \"=1.0.0\"\nold = \"=1.0.0\"\ncustom = \"=1.0.0\"\n[lints.clippy]\nold = \"allow\"\ncustom = \"allow\"\n[profile.release]\nlto = false\nstrip = true\n[profile.obsolete]\ninherits = \"release\"\n[profile.custom]\ninherits = \"dev\"\n";

    #[test]
    fn syncs_shared_tables_without_changing_project_identity_or_runtime_dependencies() {
        let base = Some(BASE);
        let merged = sync(TEMPLATE, base, PROJECT).expect("sync");
        let actual = crate::libs::document::toml_value::parse(&merged).expect("Cargo document");
        let expected = "[package]\nname = \"project\"\nversion = \"3.0.0\"\n[dependencies]\nclap = \"=1.0.0\"\n[build-dependencies]\nthiserror = \"=2.0.0\"\ncustom = \"=1.0.0\"\n[lints.clippy]\ncustom = \"allow\"\nnew = \"deny\"\n[profile.release]\nlto = true\nstrip = true\n[profile.custom]\ninherits = \"dev\"\n";
        let expected =
            crate::libs::document::toml_value::parse(expected).expect("expected document");
        assert_eq!(actual, expected);
    }

    #[test]
    fn adds_missing_shared_tables() {
        let project = "[package]\nname = \"project\"\n";
        let merged = sync(TEMPLATE, None, project).expect("sync");
        assert!(merged.starts_with(project));
        assert!(merged.contains("[build-dependencies]"));
        assert!(merged.contains("[lints.clippy]"));
        assert!(merged.contains("[profile.release]"));
        assert!(!merged.contains("[dependencies]"));
    }
}
