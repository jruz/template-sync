use crate::core::manifest;
use crate::libs::document::{format, keys, section, text};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("cannot merge {0}")]
    Merge(String, #[source] text::Error),
    #[error("cannot parse {0}")]
    Parse(String, #[source] text::Error),
    #[error("cannot merge sections of {0}")]
    Section(String, #[source] section::Error),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Changed,
    Matching,
    Missing,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Lines {
    pub missing: Vec<String>,
    pub removed: Vec<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Detail {
    Keys(Vec<keys::Change>),
    Lines(Lines),
    Text,
}

#[derive(Debug)]
pub struct Finding {
    pub content: String,
    pub detail: Detail,
    pub diff: String,
    pub path: String,
    pub status: Status,
    pub strategy: manifest::Strategy,
}

#[must_use]
pub fn needs_update(finding: &Finding) -> bool {
    finding.status != Status::Matching && finding.strategy != manifest::Strategy::Review
}

#[must_use]
pub fn needs_review(finding: &Finding) -> bool {
    finding.status != Status::Matching && finding.strategy == manifest::Strategy::Review
}

fn unified_diff(path: &str, before: &str, after: &str) -> String {
    let project = format!("project/{path}");
    let template = format!("template/{path}");
    similar::TextDiff::from_lines(before, after)
        .unified_diff()
        .header(&project, &template)
        .to_string()
}

fn status(changed: bool) -> Status {
    if changed {
        return Status::Changed;
    }
    Status::Matching
}

fn key_changes(
    path: &str,
    kind: format::Format,
    before: &str,
    after: &str,
) -> Result<Vec<keys::Change>, Error> {
    match text::changes(kind, before, after) {
        Ok(found) => Ok(found),
        Err(source) => {
            let owned = path.to_owned();
            let error = Error::Parse(owned, source);
            Err(error)
        }
    }
}

fn detail(path: &str, before: &str, after: &str) -> Result<Detail, Error> {
    let Some(kind) = format::detect(path) else {
        return Ok(Detail::Text);
    };
    let found = key_changes(path, kind, before, after)?;
    let keyed = Detail::Keys(found);
    Ok(keyed)
}

fn absent(lines: &str, others: &str) -> Vec<String> {
    let present = others.lines().collect::<Vec<_>>();
    lines
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter(|line| !present.contains(line))
        .map(str::to_owned)
        .collect()
}

fn line_changes(before: &str, after: &str) -> Detail {
    let missing = absent(after, before);
    let removed = absent(before, after);
    Detail::Lines(Lines { missing, removed })
}

fn text(entry: &manifest::Entry, template: &str, project: &str) -> Result<Finding, Error> {
    let found = detail(&entry.path, project, template)?;
    Ok(Finding {
        content: template.to_owned(),
        detail: found,
        diff: unified_diff(&entry.path, project, template),
        path: entry.path.clone(),
        status: status(template != project),
        strategy: entry.strategy.clone(),
    })
}

fn merged(
    path: &str,
    template: &str,
    project: &str,
    keys: &[String],
) -> Result<text::Merged, Error> {
    match text::merge(path, template, project, keys) {
        Ok(merged) => Ok(merged),
        Err(source) => {
            let owned = path.to_owned();
            let error = Error::Merge(owned, source);
            Err(error)
        }
    }
}

fn data(
    entry: &manifest::Entry,
    keys: &[String],
    template: &str,
    project: &str,
) -> Result<Finding, Error> {
    let text::Merged { after, before } = merged(&entry.path, template, project, keys)?;
    let diff = unified_diff(&entry.path, &before, &after);
    let changed = after != before;
    let found = detail(&entry.path, &before, &after)?;
    Ok(Finding {
        content: after,
        detail: found,
        diff,
        path: entry.path.clone(),
        status: status(changed),
        strategy: entry.strategy.clone(),
    })
}

fn sectioned(path: &str, template: &str, project: &str) -> Result<String, Error> {
    match section::merge(template, project) {
        Ok(merged) => Ok(merged),
        Err(source) => {
            let owned = path.to_owned();
            let error = Error::Section(owned, source);
            Err(error)
        }
    }
}

fn sections(entry: &manifest::Entry, template: &str, project: &str) -> Result<Finding, Error> {
    let content = sectioned(&entry.path, template, project)?;
    let diff = unified_diff(&entry.path, project, &content);
    let changed = content != project;
    let found = line_changes(project, &content);
    Ok(Finding {
        content,
        detail: found,
        diff,
        path: entry.path.clone(),
        status: status(changed),
        strategy: entry.strategy.clone(),
    })
}

pub fn finding(
    entry: &manifest::Entry,
    template: &str,
    project: Option<&str>,
) -> Result<Finding, Error> {
    let Some(current) = project else {
        return Ok(Finding {
            content: template.to_owned(),
            detail: Detail::Text,
            diff: unified_diff(&entry.path, "", template),
            path: entry.path.clone(),
            status: Status::Missing,
            strategy: entry.strategy.clone(),
        });
    };
    match &entry.strategy {
        manifest::Strategy::Copy | manifest::Strategy::Review => text(entry, template, current),
        manifest::Strategy::Merge(keys) => data(entry, keys, template, current),
        manifest::Strategy::Section => sections(entry, template, current),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn compare(
        path: &str,
        strategy: manifest::Strategy,
        template: &str,
        project: Option<&str>,
    ) -> Finding {
        let entry = manifest::Entry {
            path: path.to_owned(),
            strategy,
        };
        finding(&entry, template, project).expect("finding")
    }

    fn engines(template: &str, project: &str) -> Finding {
        let keys = vec!["engines".to_owned()];
        let strategy = manifest::Strategy::Merge(keys);
        let current = Some(project);
        compare("package.json", strategy, template, current)
    }

    fn text_change(path: &str, strategy: manifest::Strategy) -> Finding {
        let current = Some("old\n");
        compare(path, strategy, "new\n", current)
    }

    #[test]
    fn reports_missing_files_with_template_content() {
        let result = compare("LICENSE", manifest::Strategy::Copy, "text\n", None);
        assert_eq!(result.status, Status::Missing);
        assert_eq!(result.content, "text\n");
        assert!(result.diff.contains("+text"));
        assert!(needs_update(&result));
    }

    #[test]
    fn reports_changed_copies_with_diff() {
        let result = text_change("LICENSE", manifest::Strategy::Copy);
        assert_eq!(result.status, Status::Changed);
        assert!(result.diff.contains("-old"));
        assert!(result.diff.contains("+new"));
    }

    #[test]
    fn review_changes_need_review_not_update() {
        let result = text_change("justfile", manifest::Strategy::Review);
        assert!(needs_review(&result));
        assert!(!needs_update(&result));
    }

    #[test]
    fn merges_ignore_project_only_keys() {
        let result = engines(
            r#"{"engines": {"node": "24.0.0"}, "name": "template"}"#,
            r#"{"name": "project", "engines": {"node": "24.0.0"}}"#,
        );
        assert_eq!(result.status, Status::Matching);
    }

    #[test]
    fn merges_template_values_into_project() {
        let result = engines(
            r#"{"engines": {"node": "24.1.0"}}"#,
            r#"{"name": "project", "engines": {"node": "24.0.0"}}"#,
        );
        assert_eq!(result.status, Status::Changed);
        assert_eq!(
            result.content,
            "{\n  \"name\": \"project\",\n  \"engines\": {\n    \"node\": \"24.1.0\"\n  }\n}\n"
        );
    }
}
