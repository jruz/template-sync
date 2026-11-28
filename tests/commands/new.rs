use std::os::unix::fs::PermissionsExt as _;
use std::path::Path;
use std::process::Output;

use crate::utils::binary::{command, run, stderr, stdout};
use crate::utils::env;
use crate::utils::fixtures::{
    commit_changes, git, read, remotes, template_directory, template_revision, write_all,
};

const OPTIONS: [&str; 11] = [
    "--owner",
    "jruz",
    "--template",
    "typescript",
    "--visibility",
    "private",
    "--description",
    "Fixture project",
    "--topics",
    "",
    "--non-interactive",
];

fn fake_workflow(root: &Path) {
    let script = r#"#!/bin/sh
set -eu
printf '%s %s\n' "${0##*/}" "$*" >> "$NEW_FIXTURE/calls"
case "${0##*/}:$1" in
    gh:repo) git clone --bare "$NEW_FIXTURE/template-ts.git" "$NEW_FIXTURE/project.git" ;;
    nix:develop) printf '%s\n' "$PWD" > "$NEW_FIXTURE/setup-directory" ;;
    woodpecker-cli:repo)
        if [ "$2" = sync ]; then printf '%s\n' 'jruz/project 123 false'; fi ;;
esac
"#;
    for tool in [
        "gh",
        "github-settings",
        "nix",
        "woodpecker-cli",
        "woodpecker-settings",
    ] {
        let path = root.join(tool);
        std::fs::write(&path, script).expect("write fake helper");
        let permissions = std::fs::Permissions::from_mode(0o755);
        std::fs::set_permissions(path, permissions).expect("make helper executable");
    }
}

fn create(root: &Path) -> Output {
    let path = env::path();
    let location = root.display();
    command(root, root)
        .env("PATH", format!("{location}:{path}"))
        .env("NEW_FIXTURE", root)
        .env("GIT_CONFIG_COUNT", "2")
        .env(
            "GIT_CONFIG_KEY_1",
            format!("url.file://{location}/.insteadOf"),
        )
        .env("GIT_CONFIG_VALUE_1", "git@github.com:jruz/")
        .args(["new", "project", "--yes"])
        .args(OPTIONS)
        .output()
        .expect("run CLI")
}

#[test]
fn creates_tracks_and_pushes_project_with_woodpecker() {
    let remotes = remotes();
    let root = remotes.path();
    let template = template_directory(&remotes);
    write_all(
        &template,
        &[
            (
                "github-settings.json",
                r#"{"settings":{"allow_auto_merge":true},"branch_protection":{},"security":{"vulnerability-alerts":true}}"#,
            ),
            (
                "woodpecker-settings.yaml",
                "repo:\n  name: jruz/template-ts\nsettings:\n  visibility: private\nsecrets:\n  TEST: encrypted\n",
            ),
        ],
    );
    commit_changes(&template);
    fake_workflow(root);
    let created = create(root);
    assert!(created.status.success(), "{}", stderr(&created));
    assert!(stdout(&created).contains("Woodpecker: enabled"));
    let project = root.join("project");
    let manifest = read(&project, "template-sync.json");
    assert!(manifest.contains(&template_revision(&remotes)));
    assert!(manifest.contains("woodpecker-settings.yaml"));
    let github = read(&project, "github-settings.json");
    assert!(github.contains("\"branch_protection\": null"));
    assert!(github.contains("\"allow_auto_merge\": false"));
    assert!(github.contains("\"vulnerability_alerts\": true"));
    let woodpecker = read(&project, "woodpecker-settings.yaml");
    assert!(woodpecker.contains("name: jruz/project"));
    assert!(!woodpecker.contains("encrypted"));
    let remote = root.join("project.git");
    assert_eq!(git(&remote, &["show", "HEAD:template-sync.json"]), manifest);
    let calls = read(root, "calls");
    assert_eq!(
        read(root, "setup-directory").trim(),
        project.display().to_string()
    );
    let setup = calls
        .find("nix develop --no-update-lock-file --command just setup\n")
        .expect("setup call");
    let activate = calls
        .find("woodpecker-cli repo add 123\n")
        .expect("Woodpecker activation");
    assert!(setup < activate);
    assert!(calls.contains(
        "nix run nixpkgs#oxfmt -- --write github-settings.json woodpecker-settings.yaml \
         template-sync.json README.md package.json\n"
    ));
    assert!(calls.contains("woodpecker-settings apply\n"));
    assert!(calls.contains("github-settings push\n"));
    let duplicate = create(root);
    assert!(!duplicate.status.success());
    assert!(stderr(&duplicate).contains("destination already exists"));
    assert_eq!(read(root, "calls"), calls);
}

#[test]
fn requires_flags_and_approval_without_a_terminal() {
    let workspace = tempfile::tempdir().expect("temporary directory");
    let root = workspace.path();
    let missing = run(
        root,
        root,
        &["new", "project", "--non-interactive", "--yes"],
    );
    assert!(!missing.status.success());
    assert!(stderr(&missing).contains("missing --owner"));
    let not_approved = command(root, root)
        .args(["new", "project"])
        .args(OPTIONS)
        .output()
        .expect("run CLI");
    assert!(!not_approved.status.success());
    assert!(stderr(&not_approved).contains("pass --yes"));
    assert!(!root.join("project").exists());
}
