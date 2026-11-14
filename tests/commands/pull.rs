use crate::utils::binary::{run, stderr, stdout};
use crate::utils::fixtures::{
    commit_changes, git, project, project_file, read, remotes, template_directory,
    template_revision, write_all,
};

#[test]
fn writes_project_and_keeps_review_files() {
    let remotes = remotes();
    let origin = remotes.path();
    let project = project(&remotes);
    let root = project.path();
    let updated = run(root, origin, &["pull"]);
    assert!(updated.status.success(), "{}", stderr(&updated));
    assert_eq!(read(root, "LICENSE"), "licence\n");
    assert_eq!(read(root, "scripts/lib/log.ts"), "export {};\n");
    assert_eq!(
        read(root, "package.json"),
        "{\n  \"name\": \"project\",\n  \"devDependencies\": {\n    \"oxlint\": \"2.0.0\"\n  }\n}\n"
    );
    assert_eq!(read(root, "justfile"), "check:\n    just test\n");
    let checked = run(root, origin, &["check"]);
    assert!(checked.status.success());
}

#[test]
fn syncs_github_policy_and_preserves_repository_settings() {
    let remotes = remotes();
    let origin = remotes.path();
    let project = project(&remotes);
    let root = project.path();
    let template = template_directory(&remotes);
    let manifest = serde_json::json!({
        "merge": ["github-settings.json"],
        "replace": [],
        "review": []
    });
    let policy = serde_json::json!({
        "repository": "jruz/template-rs",
        "settings": {
            "description": "template", "private": false, "is_template": true,
            "allow_auto_merge": true, "allow_merge_commit": false,
            "delete_branch_on_merge": true, "default_branch": "main"
        },
        "topics": { "names": ["rust"] },
        "branch_protection": {
            "enforce_admins": false,
            "required_status_checks": { "strict": true, "contexts": ["ci/woodpecker/pr/check"] }
        },
        "actions": { "enabled": false },
        "security": { "vulnerability-alerts": true },
        "labels": [{ "name": "bugs", "color": "d73a4a" }]
    });
    let current = serde_json::json!({
        "repository": "jruz/project",
        "settings": {
            "description": "project", "private": true, "is_template": false,
            "allow_auto_merge": false, "allow_merge_commit": true,
            "delete_branch_on_merge": false, "default_branch": "develop"
        },
        "topics": { "names": ["project"] },
        "branch_protection": { "enforce_admins": true },
        "actions": { "enabled": true },
        "security": { "vulnerability-alerts": false },
        "labels": [{ "name": "custom", "color": "ffffff" }]
    });
    write_all(root, &[("github-settings.json", &current.to_string())]);
    commit_changes(root);
    write_all(
        &template,
        &[
            ("template.json", &manifest.to_string()),
            ("github-settings.json", &policy.to_string()),
        ],
    );
    commit_changes(&template);
    let updated = run(root, origin, &["pull"]);
    assert!(updated.status.success(), "{}", stderr(&updated));
    let text = read(root, "github-settings.json");
    let actual: serde_json::Value = serde_json::from_str(&text).expect("GitHub settings");
    let mut expected = current;
    *expected.get_mut("settings").expect("project settings") = serde_json::json!({
        "description": "project", "private": true, "is_template": false,
        "allow_auto_merge": true, "allow_merge_commit": false,
        "delete_branch_on_merge": true, "default_branch": "main"
    });
    for key in ["branch_protection", "actions", "security", "labels"] {
        *expected.get_mut(key).expect("project policy key") =
            policy.get(key).expect("template policy key").clone();
    }
    assert_eq!(actual, expected);
    let checked = run(root, origin, &["check"]);
    assert!(checked.status.success(), "{}", stderr(&checked));
}

#[test]
fn syncs_cargo_tables_and_preserves_project_entries() {
    let remotes = remotes();
    let origin = remotes.path();
    let project = project(&remotes);
    let root = project.path();
    let template = template_directory(&remotes);
    let manifest = r#"{"merge":["Cargo.toml"],"replace":[],"review":[]}"#;
    let policy = "[package]\nname = \"template\"\n[dependencies]\nclap = \"=2.0.0\"\n[build-dependencies]\nthiserror = \"=2.0.0\"\n[lints.clippy]\nshared = \"deny\"\n[profile.release]\nlto = true\n";
    let current = "[package]\nname = \"project\"\n[dependencies]\nclap = \"=1.0.0\"\n[lints.clippy]\ncustom = \"allow\"\n";
    write_all(root, &[("Cargo.toml", current)]);
    commit_changes(root);
    write_all(
        &template,
        &[("template.json", manifest), ("Cargo.toml", policy)],
    );
    commit_changes(&template);
    let preview = run(root, origin, &["pull", "--dry-run"]);
    assert!(preview.status.success(), "{}", stderr(&preview));
    assert!(stdout(&preview).contains("MODIFIED (1)\n  Cargo.toml\n"));
    assert_eq!(read(root, "Cargo.toml"), current);
    let drift = run(root, origin, &["check"]);
    assert!(!drift.status.success());
    assert!(stdout(&drift).contains("Cargo.toml#profile"));
    let updated = run(root, origin, &["pull"]);
    assert!(updated.status.success(), "{}", stderr(&updated));
    let actual = read(root, "Cargo.toml");
    assert!(actual.contains("name = \"project\""));
    assert!(actual.contains("clap = \"=1.0.0\""));
    assert!(actual.contains("thiserror = \"=2.0.0\""));
    assert!(actual.contains("custom = \"allow\""));
    assert!(actual.contains("shared = \"deny\""));
    assert!(actual.contains("lto = true"));
    let checked = run(root, origin, &["check"]);
    assert!(checked.status.success(), "{}", stderr(&checked));
    assert_eq!(stdout(&checked), "in sync\n");
}

#[test]
fn syncs_all_woodpecker_settings_except_repo() {
    let remotes = remotes();
    let origin = remotes.path();
    let project = project(&remotes);
    let root = project.path();
    let template = template_directory(&remotes);
    let manifest = r#"{"merge":["woodpecker-settings.yaml"],"replace":[],"review":[]}"#;
    let policy = "repo:\n  name: jruz/template-rs\nsettings:\n  timeout: 60\n  trusted: false\nsecrets:\n  GITHUB_TOKEN:\n    data: encrypted-template\n    events: [tag]\n";
    let current = "repo:\n  name: jruz/project\nsettings:\n  timeout: 10\n  trusted: true\n  custom: true\nsecrets:\n  GITHUB_TOKEN:\n    data: encrypted-project\n    events: [push]\n  CUSTOM:\n    data: encrypted-custom\ncustom: true\n";
    write_all(root, &[("woodpecker-settings.yaml", current)]);
    commit_changes(root);
    write_all(
        &template,
        &[
            ("template.json", manifest),
            ("woodpecker-settings.yaml", policy),
        ],
    );
    commit_changes(&template);
    let preview = run(root, origin, &["pull", "--dry-run"]);
    assert!(preview.status.success(), "{}", stderr(&preview));
    assert!(stdout(&preview).contains("MODIFIED (1)\n  woodpecker-settings.yaml\n"));
    assert_eq!(read(root, "woodpecker-settings.yaml"), current);
    let updated = run(root, origin, &["pull"]);
    assert!(updated.status.success(), "{}", stderr(&updated));
    let actual = read(root, "woodpecker-settings.yaml");
    let actual: serde_json::Value = serde_norway::from_str(&actual).expect("CI settings");
    let timeout: u32 = 60;
    let expected = serde_json::json!({
        "repo": { "name": "jruz/project" },
        "settings": { "timeout": timeout, "trusted": false },
        "secrets": { "GITHUB_TOKEN": { "data": "encrypted-template", "events": ["tag"] } }
    });
    assert_eq!(actual, expected);
    let checked = run(root, origin, &["check"]);
    assert!(checked.status.success(), "{}", stderr(&checked));
    assert_eq!(stdout(&checked), "in sync\n");
}

#[test]
fn records_template_revision() {
    let remotes = remotes();
    let origin = remotes.path();
    let project = project(&remotes);
    let root = project.path();
    let template = template_directory(&remotes);
    write_all(&template, &[("README.md", "template\n")]);
    commit_changes(&template);
    let revision = template_revision(&remotes);
    let updated = run(root, origin, &["pull"]);
    assert!(updated.status.success(), "{}", stderr(&updated));
    assert!(read(root, "template-sync.json").contains(&revision));
}

#[test]
fn deletes_files_the_template_removed_since_revision() {
    let remotes = remotes();
    let origin = remotes.path();
    let project = project(&remotes);
    let root = project.path();
    write_all(root, &[("scripts/lib/log.ts", "export {};\n")]);
    commit_changes(root);
    let template = template_directory(&remotes);
    git(&template, &["rm", "--quiet", "scripts/lib/log.ts"]);
    write_all(&template, &[("scripts/lib/run.ts", "export {};\n")]);
    commit_changes(&template);
    let previewed = run(root, origin, &["pull", "--dry-run"]);
    assert!(stdout(&previewed).contains("DELETED (1)\n  scripts/lib/log.ts\n"));
    let updated = run(root, origin, &["pull"]);
    assert!(updated.status.success(), "{}", stderr(&updated));
    assert!(!root.join("scripts/lib/log.ts").exists());
    assert_eq!(read(root, "scripts/lib/run.ts"), "export {};\n");
}

#[test]
fn dry_run_lists_files_without_writing() {
    let remotes = remotes();
    let origin = remotes.path();
    let project = project(&remotes);
    let root = project.path();
    let output = run(root, origin, &["pull", "--dry-run"]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(
        stdout(&output),
        "OVERWRITTEN (2)\n  LICENSE\n  scripts/lib/log.ts\n\nMODIFIED (1)\n  package.json\n"
    );
    assert_eq!(read(root, "LICENSE"), "old licence\n");
}

#[test]
fn keeps_files_the_project_reviews() {
    let remotes = remotes();
    let origin = remotes.path();
    let project = project(&remotes);
    let root = project.path();
    let revision = template_revision(&remotes);
    let reviewing = project_file(&revision, "\"LICENSE\"");
    write_all(root, &[("template-sync.json", &reviewing)]);
    let updated = run(root, origin, &["pull"]);
    assert!(updated.status.success(), "{}", stderr(&updated));
    assert_eq!(read(root, "LICENSE"), "old licence\n");
}

#[test]
fn refuses_uncommitted_targets() {
    let remotes = remotes();
    let origin = remotes.path();
    let project = project(&remotes);
    let root = project.path();
    write_all(root, &[("LICENSE", "edited\n")]);
    let updated = run(root, origin, &["pull"]);
    assert!(!updated.status.success());
    assert!(stderr(&updated).contains("uncommitted changes in template targets"));
    assert_eq!(read(root, "LICENSE"), "edited\n");
}
