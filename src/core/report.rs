use crate::core::compare;
use crate::core::manifest;
use crate::libs::document::keys;
use crate::utils::style::{self, Style};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Heading {
    Changed,
    Missing,
    Removed,
    Review,
}

fn heading_name(heading: Heading) -> &'static str {
    match heading {
        Heading::Changed => "CHANGED",
        Heading::Missing => "MISSING",
        Heading::Removed => "REMOVED",
        Heading::Review => "REVIEW",
    }
}

fn heading_color(heading: Heading) -> &'static str {
    match heading {
        Heading::Changed => "\x1b[33m",
        Heading::Missing | Heading::Removed => "\x1b[31m",
        Heading::Review => "\x1b[36m",
    }
}

fn heading(style: Style, kind: Heading, target: &str) -> String {
    let name = heading_name(kind);
    let color = heading_color(kind);
    let text = format!("{name} {target}");
    let painted = style::paint(style, color, &text);
    format!("{painted}\n")
}

fn is_review(finding: &compare::Finding) -> bool {
    finding.strategy == manifest::Strategy::Review
}

fn file_heading(finding: &compare::Finding) -> Heading {
    if is_review(finding) {
        return Heading::Review;
    }
    match finding.status {
        compare::Status::Missing => Heading::Missing,
        compare::Status::Removed => Heading::Removed,
        compare::Status::Changed | compare::Status::Matching => Heading::Changed,
    }
}

fn key_heading(finding: &compare::Finding, kind: &keys::Kind) -> Heading {
    if is_review(finding) {
        return Heading::Review;
    }
    match kind {
        keys::Kind::Added { .. } => Heading::Missing,
        keys::Kind::Changed { .. } => Heading::Changed,
        keys::Kind::Removed { .. } => Heading::Removed,
    }
}

fn expected_label(finding: &compare::Finding) -> &'static str {
    if is_review(finding) {
        return "baseline";
    }
    "required"
}

fn indented(value: &str) -> String {
    value.replace('\n', "\n  ")
}

fn key_values(finding: &compare::Finding, kind: &keys::Kind) -> String {
    let label = expected_label(finding);
    match kind {
        keys::Kind::Added { after } => format!("  {label}: {}\n", indented(after)),
        keys::Kind::Changed { after, before } => format!(
            "  {label}: {}\n  actual:   {}\n",
            indented(after),
            indented(before)
        ),
        keys::Kind::Removed { before } => format!("  actual:   {}\n", indented(before)),
    }
}

fn key_block(style: Style, finding: &compare::Finding, change: &keys::Change) -> String {
    let target = format!("{}#{}", finding.path, change.path);
    let kind = key_heading(finding, &change.kind);
    let title = heading(style, kind, &target);
    let values = key_values(finding, &change.kind);
    format!("{title}{values}")
}

fn listed(name: &str, lines: &[String]) -> String {
    lines
        .iter()
        .map(|line| format!("  {name}: {line}\n"))
        .collect::<Vec<_>>()
        .concat()
}

fn lines_block(style: Style, finding: &compare::Finding, lines: &compare::Lines) -> String {
    let kind = file_heading(finding);
    let title = heading(style, kind, &finding.path);
    let missing = listed("missing", &lines.missing);
    let removed = listed("removed", &lines.removed);
    format!("{title}{missing}{removed}")
}

fn text_block(style: Style, finding: &compare::Finding) -> String {
    let kind = file_heading(finding);
    let title = heading(style, kind, &finding.path);
    let listed_only = matches!(
        finding.status,
        compare::Status::Missing | compare::Status::Removed
    );
    if listed_only && !is_review(finding) {
        return title;
    }
    format!("{title}{}", finding.diff)
}

fn blocks(style: Style, finding: &compare::Finding) -> Vec<String> {
    match &finding.detail {
        compare::Detail::Keys(changes) if !changes.is_empty() => changes
            .iter()
            .map(|change| key_block(style, finding, change))
            .collect(),
        compare::Detail::Lines(lines) => vec![lines_block(style, finding, lines)],
        compare::Detail::Keys(_) | compare::Detail::Text => vec![text_block(style, finding)],
    }
}

fn label(finding: &compare::Finding) -> &'static str {
    match (&finding.strategy, finding.status) {
        (_, compare::Status::Matching) => "matching",
        (manifest::Strategy::Review, compare::Status::Changed | compare::Status::Missing) => {
            "review"
        }
        (_, compare::Status::Removed) => "removed",
        (_, compare::Status::Missing) => "missing",
        (
            manifest::Strategy::Cargo
            | manifest::Strategy::GithubSettings
            | manifest::Strategy::Lines
            | manifest::Strategy::Package
            | manifest::Strategy::Replace
            | manifest::Strategy::WoodpeckerSettings,
            compare::Status::Changed,
        ) => "changed",
    }
}

fn is_matching(finding: &compare::Finding) -> bool {
    finding.status == compare::Status::Matching
}

fn group(style: Style, pending: &[&compare::Finding], kind: Heading) -> Option<String> {
    let paths = pending
        .iter()
        .filter(|finding| file_heading(finding) == kind)
        .map(|finding| format!("  {}\n", finding.path))
        .collect::<Vec<_>>();
    if paths.is_empty() {
        return None;
    }
    let total = format!("({})", paths.len());
    let title = heading(style, kind, &total);
    let listed = paths.concat();
    Some(format!("{title}{listed}"))
}

fn summary(style: Style, pending: &[&compare::Finding]) -> String {
    [
        Heading::Missing,
        Heading::Changed,
        Heading::Removed,
        Heading::Review,
    ]
    .into_iter()
    .filter_map(|kind| group(style, pending, kind))
    .collect::<Vec<_>>()
    .join("\n")
}

#[must_use]
pub fn render(findings: &[compare::Finding], style: Style) -> String {
    let pending = findings
        .iter()
        .filter(|finding| !is_matching(finding))
        .collect::<Vec<_>>();
    if pending.is_empty() {
        return "in sync\n".to_owned();
    }
    let details = pending
        .iter()
        .flat_map(|finding| blocks(style, finding))
        .collect::<Vec<_>>()
        .join("\n");
    let grouped = summary(style, &pending);
    format!("{details}\n{grouped}")
}

#[must_use]
pub fn failed(findings: &[compare::Finding]) -> bool {
    findings.iter().any(compare::needs_update)
}

fn review_block(finding: &compare::Finding) -> String {
    format!(
        "<details><summary><code>{}</code></summary>\n\n```diff\n{}```\n\n</details>\n",
        finding.path, finding.diff
    )
}

#[must_use]
pub fn markdown(template: &str, revision: &str, findings: &[compare::Finding]) -> String {
    let updated = findings
        .iter()
        .filter(|finding| compare::needs_update(finding))
        .map(|finding| format!("- `{}` ({})\n", finding.path, label(finding)))
        .collect::<Vec<_>>()
        .concat();
    let reviews = findings
        .iter()
        .filter(|finding| compare::needs_review(finding))
        .map(review_block)
        .collect::<String>();
    let review_section = if reviews.is_empty() {
        String::new()
    } else {
        format!("\n## Needs manual review\n\nNot changed by this PR.\n\n{reviews}")
    };
    format!("Sync with `{template}@{revision}`.\n\n## Updated\n\n{updated}{review_section}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn finding(
        path: &str,
        status: compare::Status,
        strategy: manifest::Strategy,
    ) -> compare::Finding {
        compare::Finding {
            content: String::new(),
            detail: compare::Detail::Text,
            diff: "-old\n+new\n".to_owned(),
            path: path.to_owned(),
            status,
            strategy,
        }
    }

    fn with_detail(mut base: compare::Finding, detail: compare::Detail) -> compare::Finding {
        base.detail = detail;
        base
    }

    fn findings() -> Vec<compare::Finding> {
        vec![
            finding(
                "LICENSE",
                compare::Status::Matching,
                manifest::Strategy::Replace,
            ),
            finding(
                ".oxlintrc.json",
                compare::Status::Changed,
                manifest::Strategy::Replace,
            ),
            finding(
                "justfile",
                compare::Status::Changed,
                manifest::Strategy::Review,
            ),
        ]
    }

    #[test]
    fn renders_differences_and_summary() {
        let all = findings();
        let output = render(&all, Style::Plain);
        assert!(output.contains("CHANGED .oxlintrc.json\n-old\n+new\n"));
        assert!(output.contains("REVIEW justfile\n"));
        assert!(!output.contains("LICENSE"));
        assert!(output.ends_with("CHANGED (1)\n  .oxlintrc.json\n\nREVIEW (1)\n  justfile\n"));
        assert!(!output.contains("MISSING"));
    }

    #[test]
    fn reports_in_sync_without_groups() {
        let matching = finding(
            "LICENSE",
            compare::Status::Matching,
            manifest::Strategy::Replace,
        );
        let output = render(&[matching], Style::Plain);
        assert_eq!(output, "in sync\n");
    }

    #[test]
    fn labels_missing_review_files_as_review() {
        let missing = finding(
            "nu-lint.toml",
            compare::Status::Missing,
            manifest::Strategy::Review,
        );
        let output = render(&[missing], Style::Plain);
        assert!(output.starts_with("REVIEW nu-lint.toml\n-old\n+new\n"));
        assert!(output.ends_with("REVIEW (1)\n  nu-lint.toml\n"));
    }

    #[test]
    fn lists_missing_replacements_without_content() {
        let missing = finding(
            "LICENSE",
            compare::Status::Missing,
            manifest::Strategy::Replace,
        );
        let output = render(&[missing], Style::Plain);
        assert_eq!(output, "MISSING LICENSE\n\nMISSING (1)\n  LICENSE\n");
    }

    #[test]
    fn renders_one_block_per_key() {
        let changes = vec![
            keys::Change {
                kind: keys::Kind::Added {
                    after: "\"=2.0.21\"".to_owned(),
                },
                path: "build-dependencies.thiserror".to_owned(),
            },
            keys::Change {
                kind: keys::Kind::Changed {
                    after: "\"allow\"".to_owned(),
                    before: "\"deny\"".to_owned(),
                },
                path: "lints.clippy.a".to_owned(),
            },
        ];
        let strategy = manifest::Strategy::Cargo;
        let base = finding("Cargo.toml", compare::Status::Changed, strategy);
        let detail = compare::Detail::Keys(changes);
        let merged = with_detail(base, detail);
        let output = render(&[merged], Style::Plain);
        let expected = "MISSING Cargo.toml#build-dependencies.thiserror\n  required: \"=2.0.21\"\n\nCHANGED Cargo.toml#lints.clippy.a\n  required: \"allow\"\n  actual:   \"deny\"\n\nCHANGED (1)\n  Cargo.toml\n";
        assert_eq!(output, expected);
    }

    #[test]
    fn lists_merged_lines() {
        let lines = compare::Lines {
            missing: vec!["/tmp/*".to_owned()],
            removed: vec!["old".to_owned()],
        };
        let base = finding(
            ".gitignore",
            compare::Status::Changed,
            manifest::Strategy::Lines,
        );
        let detail = compare::Detail::Lines(lines);
        let merged = with_detail(base, detail);
        let output = render(&[merged], Style::Plain);
        assert!(output.starts_with("CHANGED .gitignore\n  missing: /tmp/*\n  removed: old\n"));
    }

    #[test]
    fn fails_only_on_updates() {
        let all = findings();
        assert!(failed(&all));
        let review = finding(
            "justfile",
            compare::Status::Changed,
            manifest::Strategy::Review,
        );
        assert!(!failed(&[review]));
    }

    #[test]
    fn markdown_separates_updates_and_reviews() {
        let all = findings();
        let body = markdown("jruz/template-ts", "abc1234", &all);
        assert!(body.starts_with("Sync with `jruz/template-ts@abc1234`."));
        assert!(body.contains("- `.oxlintrc.json` (changed)\n"));
        assert!(body.contains("<summary><code>justfile</code></summary>"));
        assert!(!body.contains("LICENSE"));
    }
}
