use std::path::Path;
use std::process::Output;

use crate::utils::binary::{command, stderr, stdout};
use crate::utils::fixtures::{git, read, remote_project, remotes, write_all};

fn update_repositories(origin: &Path, log: &Path, arguments: &[&str]) -> Output {
    command(origin, origin)
        .arg("update-repositories")
        .args(["--account", "jruz"])
        .args(arguments)
        .env("GH_LOG", log)
        .env("GH_REMOTES", origin)
        .env("GH_REPOSITORIES", "jruz/project jruz/other")
        .output()
        .expect("run CLI")
}

fn other(remotes: &tempfile::TempDir) {
    let root = remotes.path().join("other.git");
    write_all(&root, &[("README.md", "other\n")]);
}

#[test]
fn dry_run_reports_without_pushing() {
    let remotes = remotes();
    let origin = remotes.path();
    let project = remote_project(&remotes, "project");
    other(&remotes);
    let log = origin.join("gh.log");
    let output = update_repositories(origin, &log, &["--dry-run"]);
    assert!(output.status.success(), "{}", stderr(&output));
    let report = stdout(&output);
    assert!(report.contains("jruz/project: out of sync\n"));
    assert!(report.contains("CHANGED LICENSE\n"));
    assert!(report.contains("jruz/other: skipped\n"));
    assert!(!read(origin, "gh.log").contains("pr create"));
    assert_eq!(git(&project, &["branch", "--list", "template-sync/*"]), "");
}

#[test]
fn opens_pull_request_once() {
    let remotes = remotes();
    let origin = remotes.path();
    let project = remote_project(&remotes, "project");
    other(&remotes);
    let log = origin.join("gh.log");
    let first = update_repositories(origin, &log, &[]);
    assert!(first.status.success(), "{}", stderr(&first));
    assert!(stdout(&first).contains("jruz/project: pull request updated\n"));
    assert!(read(origin, "gh.log").contains("pr create --base"));
    let pushed = git(&project, &["show", "template-sync/template-ts:LICENSE"]);
    assert_eq!(pushed, "licence\n");
    let second = update_repositories(origin, &log, &[]);
    assert!(stdout(&second).contains("jruz/project: pull request up to date\n"));
}
