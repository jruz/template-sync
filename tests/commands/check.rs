use crate::utils::binary::{run, stderr, stdout};
use crate::utils::fixtures::{
    commit_changes, project, project_file, read, remotes, template_directory, template_revision,
    write_all,
};

#[test]
fn reports_drift() {
    let remotes = remotes();
    let origin = remotes.path();
    let project = project(&remotes);
    let root = project.path();
    let output = run(root, origin, &["check"]);
    assert!(!output.status.success());
    let report = stdout(&output);
    assert!(report.contains("CHANGED LICENSE\n"));
    assert!(report.contains("MISSING scripts/lib/log.ts\n"));
    assert!(
        report.contains(
            "CHANGED package.json#devDependencies.oxlint\n  required: \"2.0.0\"\n  actual:   \"1.0.0\"\n"
        )
    );
    assert!(!report.contains("REVIEW justfile"));
}

#[test]
fn reviews_template_changes_since_revision() {
    let remotes = remotes();
    let origin = remotes.path();
    let project = project(&remotes);
    let root = project.path();
    let template = template_directory(&remotes);
    write_all(&template, &[("justfile", "check:\n    just lint\n")]);
    commit_changes(&template);
    let output = run(root, origin, &["check"]);
    let report = stdout(&output);
    assert!(report.contains("REVIEW justfile\n--- base/justfile\n+++ template/justfile\n"));
    assert!(report.contains("+    just lint\n"));
    assert!(!report.contains("just test"));
}

#[test]
fn reviews_local_differences_after_pull_without_writing_or_failing() {
    let remotes = remotes();
    let origin = remotes.path();
    let project = project(&remotes);
    let root = project.path();
    let revision = template_revision(&remotes);
    let config = project_file(&revision, "\"LICENSE\"");
    write_all(root, &[("template-sync.json", &config)]);
    let pulled = run(root, origin, &["pull"]);
    assert!(pulled.status.success(), "{}", stderr(&pulled));
    let baseline = read(root, "template-sync.json");
    let checked = run(root, origin, &["check"]);
    assert!(checked.status.success(), "{}", stderr(&checked));
    assert_eq!(stdout(&checked), "in sync\n");
    let reviewed = run(root, origin, &["check", "--review"]);
    assert!(reviewed.status.success(), "{}", stderr(&reviewed));
    let report = stdout(&reviewed);
    assert!(report.contains("REVIEW LICENSE\n--- project/LICENSE\n+++ template/LICENSE\n"));
    assert!(report.contains("-old licence\n+licence\n"));
    assert!(report.contains("REVIEW justfile\n--- project/justfile\n+++ template/justfile\n"));
    assert!(report.contains("-    just test\n"));
    assert_eq!(read(root, "LICENSE"), "old licence\n");
    assert_eq!(read(root, "justfile"), "check:\n    just test\n");
    assert_eq!(read(root, "template-sync.json"), baseline);
}

#[test]
fn explicit_review_handles_matching_and_missing_files() {
    let remotes = remotes();
    let origin = remotes.path();
    let project = project(&remotes);
    let root = project.path();
    let pulled = run(root, origin, &["pull"]);
    assert!(pulled.status.success(), "{}", stderr(&pulled));
    write_all(root, &[("justfile", "check:\n")]);
    let matching = run(root, origin, &["check", "--review"]);
    assert!(matching.status.success(), "{}", stderr(&matching));
    assert_eq!(stdout(&matching), "in sync\n");
    let justfile = root.join("justfile");
    std::fs::remove_file(&justfile).expect("remove review file");
    let missing = run(root, origin, &["check", "--review"]);
    assert!(missing.status.success(), "{}", stderr(&missing));
    assert!(stdout(&missing).contains("REVIEW justfile\n"));
    assert!(!root.join("justfile").exists());
}

#[test]
fn rejects_object_merge_entries() {
    let remotes = remotes();
    let origin = remotes.path();
    let project = project(&remotes);
    let root = project.path();
    let template = template_directory(&remotes);
    let manifest = serde_json::json!({
        "merge": [{ "path": "github-settings.json", "keys": ["labels"] }],
        "replace": [],
        "review": []
    });
    write_all(
        &template,
        &[
            ("template.json", &manifest.to_string()),
            ("github-settings.json", "{}"),
        ],
    );
    commit_changes(&template);
    let output = run(root, origin, &["check"]);
    assert!(!output.status.success());
    assert!(stderr(&output).contains("invalid template.json"));
}

#[test]
fn refuses_to_copy_template_github_identity_into_missing_project_settings() {
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
    write_all(
        &template,
        &[
            ("template.json", &manifest.to_string()),
            ("github-settings.json", "{}"),
        ],
    );
    commit_changes(&template);
    let output = run(root, origin, &["check"]);
    assert!(!output.status.success());
    assert!(stderr(&output).contains("missing project github-settings.json"));
    assert!(!root.join("github-settings.json").exists());
}

#[test]
fn rejects_unsupported_structured_merge_files() {
    let remotes = remotes();
    let origin = remotes.path();
    let project = project(&remotes);
    let root = project.path();
    let template = template_directory(&remotes);
    let manifest = r#"{"merge":["settings.json"],"replace":[],"review":[]}"#;
    write_all(
        &template,
        &[("template.json", manifest), ("settings.json", "{}")],
    );
    commit_changes(&template);
    let output = run(root, origin, &["check"]);
    assert!(!output.status.success());
    assert!(stderr(&output).contains("unsupported merge file: settings.json"));
}

#[test]
fn refuses_to_copy_missing_project_identity() {
    for path in ["Cargo.toml", "package.json", "woodpecker-settings.yaml"] {
        let remotes = remotes();
        let origin = remotes.path();
        let project = project(&remotes);
        let root = project.path();
        let template = template_directory(&remotes);
        let manifest = serde_json::json!({ "merge": [path], "replace": [], "review": [] });
        let target = root.join(path);
        if target.exists() {
            std::fs::remove_file(target).expect("remove package");
            commit_changes(root);
        }
        write_all(
            &template,
            &[
                ("template.json", &manifest.to_string()),
                (path, "template identity"),
            ],
        );
        commit_changes(&template);
        for arguments in [&["check"][..], &["pull"][..], &["pull", "--dry-run"][..]] {
            let output = run(root, origin, arguments);
            assert!(!output.status.success());
            assert!(stderr(&output).contains(&format!("missing project {path}")));
            assert!(!root.join(path).exists());
        }
    }
}

#[test]
fn reports_missing_project_file() {
    let remotes = remotes();
    let origin = remotes.path();
    let project = project(&remotes);
    let root = project.path();
    let file = root.join("template-sync.json");
    std::fs::remove_file(file).expect("remove project file");
    let output = run(root, origin, &["check"]);
    assert!(!output.status.success());
    assert!(stderr(&output).contains("error: project not initialized, run template-sync init"));
}

#[test]
fn reports_unreachable_template() {
    let remotes = remotes();
    let origin = remotes.path();
    let project = project(&remotes);
    let root = project.path();
    let template = template_directory(&remotes);
    std::fs::remove_dir_all(&template).expect("remove template");
    let output = run(root, origin, &["check"]);
    assert!(!output.status.success());
    assert!(stderr(&output).contains("git clone"));
}
