use crate::core::manifest;
use crate::libs::document::{format, keys, lines, text};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("cannot merge lines of {0}")]
    Lines(String, #[source] lines::Error),
    #[error("cannot merge {0}")]
    Merge(String, #[source] text::Error),
    #[error("missing project {0}; create it with this repository's identity before syncing")]
    MissingProject(String),
    #[error("cannot parse {0}")]
    Parse(String, #[source] text::Error),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Changed,
    Matching,
    Missing,
    Removed,
}

pub struct Contents<'text> {
    pub base: Option<&'text str>,
    pub project: Option<&'text str>,
    pub template: &'text str,
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

fn labelled_diff(origin: &str, path: &str, before: &str, after: &str) -> String {
    let from = format!("{origin}/{path}");
    let template = format!("template/{path}");
    similar::TextDiff::from_lines(before, after)
        .unified_diff()
        .header(&from, &template)
        .to_string()
}

fn unified_diff(path: &str, before: &str, after: &str) -> String {
    labelled_diff("project", path, before, after)
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

fn review(
    entry: &manifest::Entry,
    contents: &Contents<'_>,
    project: &str,
) -> Result<Finding, Error> {
    let Some(base) = contents.base else {
        return text(entry, contents.template, project);
    };
    let template = contents.template;
    let found = detail(&entry.path, base, template)?;
    let unchanged = base == template || project == template;
    Ok(Finding {
        content: template.to_owned(),
        detail: found,
        diff: labelled_diff("base", &entry.path, base, template),
        path: entry.path.clone(),
        status: status(!unchanged),
        strategy: entry.strategy.clone(),
    })
}

fn merged(path: &str, contents: &Contents<'_>, project: &str) -> Result<text::Merged, Error> {
    let sources = text::Sources {
        base: contents.base,
        project,
        template: contents.template,
    };
    match text::merge(path, &sources) {
        Ok(merged) => Ok(merged),
        Err(source) => {
            let owned = path.to_owned();
            let error = Error::Merge(owned, source);
            Err(error)
        }
    }
}

fn data(entry: &manifest::Entry, contents: &Contents<'_>, project: &str) -> Result<Finding, Error> {
    let text::Merged { after, before } = merged(&entry.path, contents, project)?;
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

fn merged_lines(path: &str, contents: &Contents<'_>, project: &str) -> Result<String, Error> {
    match lines::merge(contents.template, contents.base, project) {
        Ok(merged) => Ok(merged),
        Err(source) => {
            let owned = path.to_owned();
            let error = Error::Lines(owned, source);
            Err(error)
        }
    }
}

fn line_merge(
    entry: &manifest::Entry,
    contents: &Contents<'_>,
    project: &str,
) -> Result<Finding, Error> {
    let content = merged_lines(&entry.path, contents, project)?;
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

#[must_use]
pub fn removal(entry: &manifest::Entry, project: &str) -> Finding {
    Finding {
        content: String::new(),
        detail: Detail::Text,
        diff: unified_diff(&entry.path, project, ""),
        path: entry.path.clone(),
        status: Status::Removed,
        strategy: entry.strategy.clone(),
    }
}

pub fn finding(entry: &manifest::Entry, contents: &Contents<'_>) -> Result<Finding, Error> {
    let template = contents.template;
    let Some(current) = contents.project else {
        if matches!(
            entry.strategy,
            manifest::Strategy::Cargo
                | manifest::Strategy::GithubSettings
                | manifest::Strategy::Package
        ) {
            let path = entry.path.clone();
            let error = Error::MissingProject(path);
            return Err(error);
        }
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
        manifest::Strategy::Cargo
        | manifest::Strategy::GithubSettings
        | manifest::Strategy::Package => data(entry, contents, current),
        manifest::Strategy::Lines => line_merge(entry, contents, current),
        manifest::Strategy::Replace => text(entry, template, current),
        manifest::Strategy::Review => review(entry, contents, current),
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
        with_base(path, strategy, None, template, project)
    }

    fn with_base(
        path: &str,
        strategy: manifest::Strategy,
        base: Option<&str>,
        template: &str,
        project: Option<&str>,
    ) -> Finding {
        let entry = manifest::Entry {
            path: path.to_owned(),
            strategy,
        };
        let contents = Contents {
            base,
            project,
            template,
        };
        finding(&entry, &contents).expect("finding")
    }

    #[test]
    fn reviews_match_when_the_template_did_not_change_since_base() {
        let base = Some("new\n");
        let project = Some("project\n");
        let result = with_base(
            "justfile",
            manifest::Strategy::Review,
            base,
            "new\n",
            project,
        );
        assert_eq!(result.status, Status::Matching);
    }

    #[test]
    fn reviews_show_template_changes_since_base() {
        let base = Some("old\n");
        let project = Some("project\n");
        let result = with_base(
            "justfile",
            manifest::Strategy::Review,
            base,
            "new\n",
            project,
        );
        assert_eq!(result.status, Status::Changed);
        assert!(result.diff.contains("--- base/justfile"));
        assert!(result.diff.contains("-old"));
        assert!(!result.diff.contains("project"));
    }

    #[test]
    fn removals_need_update() {
        let entry = manifest::Entry {
            path: "scripts/old.nu".to_owned(),
            strategy: manifest::Strategy::Replace,
        };
        let result = removal(&entry, "old\n");
        assert_eq!(result.status, Status::Removed);
        assert!(needs_update(&result));
    }

    fn engines(template: &str, project: &str) -> Finding {
        let strategy = manifest::Strategy::Package;
        let current = Some(project);
        compare("package.json", strategy, template, current)
    }

    fn text_change(path: &str, strategy: manifest::Strategy) -> Finding {
        let current = Some("old\n");
        compare(path, strategy, "new\n", current)
    }

    #[test]
    fn reports_missing_files_with_template_content() {
        let result = compare("LICENSE", manifest::Strategy::Replace, "text\n", None);
        assert_eq!(result.status, Status::Missing);
        assert_eq!(result.content, "text\n");
        assert!(result.diff.contains("+text"));
        assert!(needs_update(&result));
    }

    #[test]
    fn reports_changed_replacements_with_diff() {
        let result = text_change("LICENSE", manifest::Strategy::Replace);
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
