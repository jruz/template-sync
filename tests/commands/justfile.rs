use std::path::Path;
use std::process::Output;

use crate::utils::{binary, fixtures};

const MANIFEST: &str = r#"{"merge":["justfile"],"replace":[],"review":[]}"#;

fn setup(base: &str, current: &str, updated: &str) -> (tempfile::TempDir, tempfile::TempDir) {
    let remotes = fixtures::remotes();
    let template = fixtures::template_directory(&remotes);
    fixtures::write_all(
        &template,
        &[("template.json", MANIFEST), ("justfile", base)],
    );
    fixtures::commit_changes(&template);
    let project = fixtures::project(&remotes);
    let root = project.path();
    fixtures::write_all(root, &[("justfile", current)]);
    fixtures::commit_changes(root);
    fixtures::write_all(&template, &[("justfile", updated)]);
    fixtures::commit_changes(&template);
    (remotes, project)
}

fn pull(root: &Path, origin: &Path) -> Output {
    binary::run(root, origin, &["pull"])
}

#[test]
fn merges_independent_recipe_edits_and_preserves_custom_checks() {
    let base = "check:\n    just fmt\n    just lint\n    just test\n";
    let current = "check:\n    just fmt\n    just lint\n    just test\n    just custom-check\n\ncustom-check:\n    echo custom\n";
    let updated = "mod tools '.just/tools.justfile'\n\ncheck:\n    just format\n    just lint\n    just test\n";
    let (remotes, project) = setup(base, current, updated);
    let root = project.path();
    let origin = remotes.path();
    let before = fixtures::read(root, "justfile");
    let preview = binary::run(root, origin, &["pull", "--dry-run"]);
    assert!(preview.status.success(), "{}", binary::stderr(&preview));
    assert_eq!(fixtures::read(root, "justfile"), before);
    let pulled = pull(root, origin);
    assert!(pulled.status.success(), "{}", binary::stderr(&pulled));
    let actual = fixtures::read(root, "justfile");
    assert!(actual.contains("mod tools '.just/tools.justfile'"));
    assert!(
        actual.contains("    just format\n    just lint\n    just test\n    just custom-check")
    );
    assert!(actual.contains("custom-check:\n    echo custom"));
    let checked = binary::run(root, origin, &["check"]);
    assert!(checked.status.success(), "{}", binary::stderr(&checked));
    assert!(binary::stdout(&checked).contains("in sync"));
}

#[test]
fn migrates_a_replaced_justfile_to_merge_without_losing_project_recipes() {
    let remotes = fixtures::remotes();
    let origin = remotes.path();
    let template = fixtures::template_directory(&remotes);
    let replaced = r#"{"merge":[],"replace":["justfile"],"review":[]}"#;
    fixtures::write_all(&template, &[("template.json", replaced)]);
    fixtures::commit_changes(&template);
    let project = fixtures::project(&remotes);
    let root = project.path();
    let current = "check:\n\nskill-sync:\n    echo sync\n";
    fixtures::write_all(root, &[("justfile", current)]);
    fixtures::commit_changes(root);
    let updated = "mod tools '.just/tools.justfile'\n\ncheck:\n";
    fixtures::write_all(
        &template,
        &[("template.json", MANIFEST), ("justfile", updated)],
    );
    fixtures::commit_changes(&template);
    let pulled = pull(root, origin);
    assert!(pulled.status.success(), "{}", binary::stderr(&pulled));
    let actual = fixtures::read(root, "justfile");
    assert!(actual.contains("skill-sync:\n    echo sync"));
    assert!(actual.contains("mod tools '.just/tools.justfile'"));
    let checked = binary::run(root, origin, &["check"]);
    assert!(checked.status.success(), "{}", binary::stderr(&checked));
}

#[test]
fn conflicting_steps_fail_without_writing_files_or_revision() {
    let (remotes, project) = setup(
        "check:\n    just old\n",
        "check:\n    just project\n",
        "check:\n    just template\n",
    );
    let root = project.path();
    let origin = remotes.path();
    let before = fixtures::read(root, "justfile");
    let revision = fixtures::read(root, "template-sync.json");
    let pulled = pull(root, origin);
    assert!(!pulled.status.success());
    assert!(binary::stderr(&pulled).contains("conflicting justfile edits in recipe:check"));
    assert_eq!(fixtures::read(root, "justfile"), before);
    assert_eq!(fixtures::read(root, "template-sync.json"), revision);
}

#[test]
fn removes_unchanged_template_recipes_and_keeps_local_edits() {
    let base = "old:\n    echo old\n\ncheck:\n    echo check\n";
    let current = "old:\n    echo old\n\ncheck:\n    echo custom\n";
    let updated = "check:\n    echo check\n\nnew:\n    echo new\n";
    let (remotes, project) = setup(base, current, updated);
    let root = project.path();
    let origin = remotes.path();
    let pulled = pull(root, origin);
    assert!(pulled.status.success(), "{}", binary::stderr(&pulled));
    let actual = fixtures::read(root, "justfile");
    assert!(!actual.contains("old:"));
    assert!(actual.contains("check:\n    echo custom"));
    assert!(actual.contains("new:\n    echo new"));
}

#[test]
fn rejects_invalid_syntax_without_writing() {
    let (remotes, project) = setup("check:\n", "check:\n", "check [\n");
    let root = project.path();
    let origin = remotes.path();
    let revision = fixtures::read(root, "template-sync.json");
    let pulled = pull(root, origin);
    assert!(!pulled.status.success());
    assert!(binary::stderr(&pulled).contains("invalid justfile syntax"));
    assert_eq!(fixtures::read(root, "justfile"), "check:\n");
    assert_eq!(fixtures::read(root, "template-sync.json"), revision);
}

#[test]
fn preserves_default_recipe_and_formatted_source_when_template_is_unchanged() {
    let base = "value := 'base'\n\ncheck:\n    echo {{ value }}\n";
    let current =
        "value := 'project'\n\nlocal-default:\n    echo default\n\ncheck:\n    echo {{ value }}\n";
    let updated = "value := 'base'\n\ncheck:\n    echo {{ value }}\n\nadded:\n    echo added\n";
    let (remotes, project) = setup(base, current, updated);
    let root = project.path();
    let origin = remotes.path();
    let pulled = pull(root, origin);
    assert!(pulled.status.success(), "{}", binary::stderr(&pulled));
    let expected = format!("{current}\nadded:\n    echo added\n");
    assert_eq!(fixtures::read(root, "justfile"), expected);
    let formatted = expected.replace("\n\n", "\n\n\n");
    fixtures::write_all(root, &[("justfile", &formatted)]);
    let checked = binary::run(root, origin, &["check"]);
    assert!(checked.status.success(), "{}", binary::stderr(&checked));
    assert_eq!(fixtures::read(root, "justfile"), formatted);
}

#[test]
fn deletion_conflicts_with_project_recipe_changes() {
    let (remotes, project) = setup(
        "old:\n    echo old\n\ncheck:\n",
        "old:\n    echo custom\n\ncheck:\n",
        "check:\n",
    );
    let root = project.path();
    let origin = remotes.path();
    let pulled = pull(root, origin);
    assert!(!pulled.status.success());
    assert!(binary::stderr(&pulled).contains("conflicting justfile edits in recipe:old"));
}

#[test]
fn merges_variables_attributes_dependencies_and_multiline_bodies() {
    let base = "set shell := ['sh', '-cu']\n\nvalue := 'old'\n\n[private]\ncheck: dep\n    echo first\n    echo second\n    echo third\n\ndep:\n";
    let current = "set shell := ['sh', '-cu']\n\nvalue := 'old'\n\n[private]\ncheck: dep custom\n    echo first\n    echo second\n    echo third\n\ndep:\n\ncustom:\n";
    let updated = "set shell := ['sh', '-cu']\n\nvalue := 'new'\n\n[private]\ncheck: dep\n    echo first\n    echo second\n    echo updated\n\ndep:\n";
    let (remotes, project) = setup(base, current, updated);
    let root = project.path();
    let origin = remotes.path();
    let pulled = pull(root, origin);
    assert!(pulled.status.success(), "{}", binary::stderr(&pulled));
    let actual = fixtures::read(root, "justfile");
    assert!(actual.contains("value := 'new'"));
    assert!(actual.contains("[private]\ncheck: dep custom"));
    assert!(actual.contains("echo updated"));
    assert!(actual.contains("custom:"));
}
