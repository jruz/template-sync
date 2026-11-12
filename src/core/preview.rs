use crate::core::compare;
use crate::core::manifest;
use crate::utils::style::{self, Style};

const MODIFIED: &str = "\x1b[33m";
const OVERWRITTEN: &str = "\x1b[31m";

#[derive(PartialEq, Eq)]
enum Action {
    Deleted,
    Modified,
    Overwritten,
}

fn action(finding: &compare::Finding) -> Action {
    if finding.status == compare::Status::Removed {
        return Action::Deleted;
    }
    if finding.strategy == manifest::Strategy::Replace {
        return Action::Overwritten;
    }
    Action::Modified
}

fn paths<'finding>(findings: &'finding [compare::Finding], wanted: &Action) -> Vec<&'finding str> {
    findings
        .iter()
        .filter(|finding| compare::needs_update(finding))
        .filter(|finding| action(finding) == *wanted)
        .map(|finding| finding.path.as_str())
        .collect()
}

fn section(style: Style, name: &str, color: &str, listed: &[&str]) -> Option<String> {
    if listed.is_empty() {
        return None;
    }
    let total = listed.len();
    let text = format!("{name} ({total})");
    let title = style::paint(style, color, &text);
    let lines = listed
        .iter()
        .map(|path| format!("  {path}\n"))
        .collect::<Vec<_>>()
        .concat();
    Some(format!("{title}\n{lines}"))
}

pub fn render(findings: &[compare::Finding], style: Style) -> String {
    let overwritten = paths(findings, &Action::Overwritten);
    let modified = paths(findings, &Action::Modified);
    let deleted = paths(findings, &Action::Deleted);
    let sections = [
        section(style, "OVERWRITTEN", OVERWRITTEN, &overwritten),
        section(style, "MODIFIED", MODIFIED, &modified),
        section(style, "DELETED", OVERWRITTEN, &deleted),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();
    if sections.is_empty() {
        return "nothing to update\n".to_owned();
    }
    sections.join("\n")
}
