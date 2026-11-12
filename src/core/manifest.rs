use std::collections::BTreeSet;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

pub const MANIFEST_FILE: &str = "template.json";
pub const PROJECT_FILE: &str = "template-sync.json";

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("project already initialized")]
    AlreadyInitialized,
    #[error("invalid {0}")]
    Invalid(String, #[source] serde_json::Error),
    #[error("invalid template repository: {0}")]
    InvalidTemplate(String),
    #[error("missing template file: {0}")]
    MissingFile(String),
    #[error("non UTF-8 file name: {0:?}")]
    NonUtf8Name(OsString),
    #[error("project not initialized, run template-sync init")]
    NotInitialized,
    #[error("cannot read {0}")]
    Read(PathBuf, #[source] std::io::Error),
    #[error("cannot serialize {PROJECT_FILE}")]
    Serialize(#[source] serde_json::Error),
    #[error("{PROJECT_FILE} reviews {0}, which {MANIFEST_FILE} does not sync")]
    UnknownReview(String),
    #[error("cannot write {0}")]
    Write(PathBuf, #[source] std::io::Error),
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Merge {
    pub keys: Vec<String>,
    pub path: String,
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub copy: Vec<String>,
    pub merge: Vec<Merge>,
    pub review: Vec<String>,
    pub section: Vec<String>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct Project {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub review: Option<Vec<String>>,
    pub template: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Strategy {
    Copy,
    Merge(Vec<String>),
    Review,
    Section,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub path: String,
    pub strategy: Strategy,
}

fn read(path: &Path) -> Result<String, Error> {
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(text),
        Err(source) => {
            let owned = path.to_path_buf();
            let error = Error::Read(owned, source);
            Err(error)
        }
    }
}

fn parse<T>(text: &str, path: &str) -> Result<T, Error>
where
    T: serde::de::DeserializeOwned,
{
    match serde_json::from_str(text) {
        Ok(value) => Ok(value),
        Err(source) => {
            let owned = path.to_owned();
            let error = Error::Invalid(owned, source);
            Err(error)
        }
    }
}

pub fn load(template: &Path) -> Result<Manifest, Error> {
    let path = template.join(MANIFEST_FILE);
    let text = read(&path)?;
    parse(&text, MANIFEST_FILE)
}

pub fn parse_project(text: &str) -> Result<Project, Error> {
    parse(text, PROJECT_FILE)
}

pub fn project(directory: &Path) -> Result<Project, Error> {
    let path = directory.join(PROJECT_FILE);
    if !path.exists() {
        return Err(Error::NotInitialized);
    }
    let text = read(&path)?;
    parse_project(&text)
}

pub fn create(directory: &Path, template: &str) -> Result<(), Error> {
    let path = directory.join(PROJECT_FILE);
    if path.exists() {
        return Err(Error::AlreadyInitialized);
    }
    let config = Project {
        review: None,
        template: template.to_owned(),
    };
    let text = serde_json::to_string_pretty(&config).map_err(Error::Serialize)?;
    let content = format!("{text}\n");
    std::fs::write(&path, content).map_err(|source| Error::Write(path, source))
}

pub fn template_name(repository: &str) -> Result<&str, Error> {
    let Some((_, name)) = repository.split_once('/') else {
        let owned = repository.to_owned();
        let error = Error::InvalidTemplate(owned);
        return Err(error);
    };
    Ok(name)
}

fn file_name(entry: std::io::Result<std::fs::DirEntry>, directory: &Path) -> Result<String, Error> {
    let found = match entry {
        Ok(found) => found,
        Err(source) => {
            let owned = directory.to_path_buf();
            let error = Error::Read(owned, source);
            return Err(error);
        }
    };
    match found.file_name().into_string() {
        Ok(name) => Ok(name),
        Err(raw) => {
            let error = Error::NonUtf8Name(raw);
            Err(error)
        }
    }
}

fn walk(template: &Path, directory: &str) -> Result<Vec<String>, Error> {
    let absolute = template.join(directory);
    let listing = match std::fs::read_dir(&absolute) {
        Ok(listing) => listing,
        Err(source) => {
            let error = Error::Read(absolute, source);
            return Err(error);
        }
    };
    let names = listing
        .map(|entry| file_name(entry, &absolute))
        .collect::<Result<BTreeSet<_>, _>>()?;
    let nested = names
        .iter()
        .map(|name| {
            let relative = format!("{directory}/{name}");
            let path = template.join(&relative);
            if path.is_dir() {
                return walk(template, &relative);
            }
            Ok(vec![relative])
        })
        .collect::<Result<Vec<_>, _>>()?;
    let files = nested.concat();
    Ok(files)
}

fn expand(template: &Path, path: &str) -> Result<Vec<String>, Error> {
    if let Some(directory) = path.strip_suffix('/') {
        return walk(template, directory);
    }
    let owned = path.to_owned();
    let absolute = template.join(path);
    if !absolute.is_file() {
        let error = Error::MissingFile(owned);
        return Err(error);
    }
    Ok(vec![owned])
}

fn expand_all(template: &Path, paths: &[String], strategy: &Strategy) -> Result<Vec<Entry>, Error> {
    let expanded = paths
        .iter()
        .map(|path| expand(template, path))
        .collect::<Result<Vec<_>, _>>()?;
    let entries = expanded
        .concat()
        .into_iter()
        .map(|path| Entry {
            path,
            strategy: strategy.clone(),
        })
        .collect();
    Ok(entries)
}

fn merge_entry(template: &Path, merge: &Merge) -> Result<Entry, Error> {
    expand(template, &merge.path)?;
    let keys = merge.keys.clone();
    Ok(Entry {
        path: merge.path.clone(),
        strategy: Strategy::Merge(keys),
    })
}

fn covers(pattern: &str, path: &str) -> bool {
    match pattern.strip_suffix('/') {
        Some(directory) => path
            .strip_prefix(directory)
            .is_some_and(|rest| rest.starts_with('/')),
        None => path == pattern,
    }
}

fn reviewed(entry: Entry, patterns: &[String]) -> Entry {
    let matched = patterns.iter().any(|pattern| covers(pattern, &entry.path));
    if !matched {
        return entry;
    }
    Entry {
        strategy: Strategy::Review,
        ..entry
    }
}

fn overridden(all: Vec<Entry>, project: &Project) -> Result<Vec<Entry>, Error> {
    let Some(patterns) = &project.review else {
        return Ok(all);
    };
    let unknown = patterns
        .iter()
        .find(|pattern| !all.iter().any(|entry| covers(pattern, &entry.path)));
    if let Some(pattern) = unknown {
        let owned = pattern.clone();
        let error = Error::UnknownReview(owned);
        return Err(error);
    }
    let entries = all
        .into_iter()
        .map(|entry| reviewed(entry, patterns))
        .collect();
    Ok(entries)
}

pub fn entries(
    manifest: &Manifest,
    template: &Path,
    project: &Project,
) -> Result<Vec<Entry>, Error> {
    let copies = expand_all(template, &manifest.copy, &Strategy::Copy)?;
    let merges = manifest
        .merge
        .iter()
        .map(|merge| merge_entry(template, merge))
        .collect::<Result<Vec<_>, Error>>()?;
    let sections = expand_all(template, &manifest.section, &Strategy::Section)?;
    let reviews = expand_all(template, &manifest.review, &Strategy::Review)?;
    let all = [copies, merges, sections, reviews].concat();
    overridden(all, project)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_all(root: &Path, paths: &[&str]) {
        paths
            .iter()
            .try_for_each(|path| {
                let target = root.join(path);
                let parent = target.parent().expect("parent");
                std::fs::create_dir_all(parent)?;
                std::fs::write(target, path)
            })
            .expect("write files");
    }

    fn owned(paths: &[&str]) -> Vec<String> {
        paths.iter().map(|path| (*path).to_owned()).collect()
    }

    fn manifest(copy: &[&str], merge: &[&str], section: &[&str], review: &[&str]) -> Manifest {
        Manifest {
            copy: owned(copy),
            merge: merge
                .iter()
                .map(|path| Merge {
                    keys: owned(&["scripts"]),
                    path: (*path).to_owned(),
                })
                .collect(),
            review: owned(review),
            section: owned(section),
        }
    }

    fn project(review: &[&str]) -> Project {
        Project {
            review: owned(review).into(),
            template: "jruz/template-rs".to_owned(),
        }
    }

    #[test]
    fn expands_directories_recursively_in_sorted_order() {
        let template = tempfile::tempdir().expect("temporary directory");
        let root = template.path();
        write_all(
            root,
            &["lib/b.ts", "lib/a.ts", "lib/nested/c.ts", "LICENSE"],
        );
        let declared = manifest(&["LICENSE", "lib/"], &[], &[], &[]);
        let config = project(&[]);
        let listed = entries(&declared, root, &config).expect("entries");
        let paths = listed
            .iter()
            .map(|entry| entry.path.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            paths,
            ["LICENSE", "lib/a.ts", "lib/b.ts", "lib/nested/c.ts"]
        );
    }

    #[test]
    fn assigns_strategies() {
        let template = tempfile::tempdir().expect("temporary directory");
        let root = template.path();
        write_all(root, &["LICENSE", "package.json", ".gitignore", "justfile"]);
        let declared = manifest(
            &["LICENSE"],
            &["package.json"],
            &[".gitignore"],
            &["justfile"],
        );
        let config = project(&[]);
        let listed = entries(&declared, root, &config).expect("entries");
        let strategies = listed
            .into_iter()
            .map(|entry| entry.strategy)
            .collect::<Vec<_>>();
        let keys = owned(&["scripts"]);
        assert_eq!(
            strategies,
            [
                Strategy::Copy,
                Strategy::Merge(keys),
                Strategy::Section,
                Strategy::Review
            ]
        );
    }

    #[test]
    fn rejects_missing_files() {
        let template = tempfile::tempdir().expect("temporary directory");
        let root = template.path();
        let declared = manifest(&["LICENSE"], &[], &[], &[]);
        let config = project(&[]);
        let result = entries(&declared, root, &config);
        assert!(matches!(result, Err(Error::MissingFile(path)) if path == "LICENSE"));
    }

    #[test]
    fn project_reviews_override_template_strategies() {
        let template = tempfile::tempdir().expect("temporary directory");
        let root = template.path();
        write_all(root, &["LICENSE", ".envrc", "lib/a.ts"]);
        let declared = manifest(&["LICENSE", ".envrc", "lib/"], &[], &[], &[]);
        let config = project(&["LICENSE", "lib/"]);
        let listed = entries(&declared, root, &config).expect("entries");
        let strategies = listed
            .into_iter()
            .map(|entry| (entry.path, entry.strategy))
            .collect::<Vec<_>>();
        assert_eq!(
            strategies,
            [
                ("LICENSE".to_owned(), Strategy::Review),
                (".envrc".to_owned(), Strategy::Copy),
                ("lib/a.ts".to_owned(), Strategy::Review)
            ]
        );
    }

    #[test]
    fn rejects_reviews_of_unsynced_files() {
        let template = tempfile::tempdir().expect("temporary directory");
        let root = template.path();
        write_all(root, &["LICENSE"]);
        let declared = manifest(&["LICENSE"], &[], &[], &[]);
        let config = project(&["LICENCE"]);
        let result = entries(&declared, root, &config);
        assert!(matches!(result, Err(Error::UnknownReview(path)) if path == "LICENCE"));
    }

    #[test]
    fn accepts_projects_without_reviews() {
        let parsed = parse_project(r#"{"template": "jruz/template-ts"}"#).expect("project");
        assert_eq!(parsed.review, None);
    }

    #[test]
    fn rejects_unknown_fields() {
        let result =
            parse_project(r#"{"review": [], "template": "jruz/template-ts", "extra": true}"#);
        assert!(matches!(result, Err(Error::Invalid(..))));
    }
}
