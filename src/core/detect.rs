use std::path::Path;

const MARKERS: [(&str, &str); 2] = [
    ("Cargo.toml", "jruz/template-rs"),
    ("package.json", "jruz/template-ts"),
];

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("cannot pick a template, found {}", .0.join(" and "))]
    Ambiguous(Vec<&'static str>),
    #[error("cannot pick a template, expected one of {}", markers().join(", "))]
    Unknown,
}

fn markers() -> Vec<&'static str> {
    MARKERS.iter().map(|(marker, _)| *marker).collect()
}

pub fn template(directory: &Path) -> Result<&'static str, Error> {
    let found = MARKERS
        .iter()
        .filter(|(marker, _)| directory.join(marker).is_file())
        .collect::<Vec<_>>();
    match found.as_slice() {
        [] => Err(Error::Unknown),
        [(_, repository)] => Ok(repository),
        many => {
            let names = many.iter().map(|(marker, _)| *marker).collect();
            let error = Error::Ambiguous(names);
            Err(error)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn directory(files: &[&str]) -> tempfile::TempDir {
        let directory = tempfile::tempdir().expect("temporary directory");
        files
            .iter()
            .try_for_each(|file| {
                let path = directory.path().join(file);
                std::fs::write(path, "")
            })
            .expect("write files");
        directory
    }

    #[test]
    fn picks_template_from_marker() {
        let project = directory(&["Cargo.toml"]);
        let root = project.path();
        let picked = template(root).expect("template");
        assert_eq!(picked, "jruz/template-rs");
    }

    #[test]
    fn rejects_projects_without_markers() {
        let project = directory(&[]);
        let root = project.path();
        let result = template(root);
        assert!(matches!(result, Err(Error::Unknown)));
    }

    #[test]
    fn rejects_projects_with_several_markers() {
        let project = directory(&["Cargo.toml", "package.json"]);
        let root = project.path();
        let result = template(root);
        assert!(
            matches!(result, Err(Error::Ambiguous(names)) if names == ["Cargo.toml", "package.json"])
        );
    }
}
