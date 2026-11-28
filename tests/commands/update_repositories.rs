use std::path::Path;
use std::process::Output;

use crate::utils::binary::{command, stderr, stdout};
use crate::utils::fixtures::{git, read, remote_project, remotes, write_all};

fn update_repositories_with(
    origin: &Path,
    log: &Path,
    arguments: &[&str],
    open_pull_requests: &str,
) -> Output {
    command(origin, origin)
        .arg("update-repositories")
        .args(["--account", "jruz"])
        .args(arguments)
        .env("GH_LOG", log)
        .env("GH_OPEN_PULL_REQUESTS", open_pull_requests)
        .env("GH_REMOTES", origin)
        .env("GH_REPOSITORIES", "jruz/project jruz/other")
        .output()
        .expect("run CLI")
}

fn update_repositories(origin: &Path, log: &Path, arguments: &[&str]) -> Output {
    update_repositories_with(origin, log, arguments, "")
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

#[test]
fn refreshes_open_pull_request_body_when_branch_is_up_to_date() {
    let remotes = remotes();
    let origin = remotes.path();
    remote_project(&remotes, "project");
    other(&remotes);
    let log = origin.join("gh.log");
    let first = update_repositories(origin, &log, &[]);
    assert!(first.status.success(), "{}", stderr(&first));
    assert!(!read(origin, "gh.log").contains("api --method PATCH"));
    let second = update_repositories_with(origin, &log, &[], "7");
    assert!(second.status.success(), "{}", stderr(&second));
    assert!(stdout(&second).contains("jruz/project: pull request up to date\n"));
    assert!(read(origin, "gh.log").contains("api --method PATCH repos/jruz/project/pulls/7"));
}

#[test]
fn formats_written_files_before_committing() {
    let remotes = remotes();
    let origin = remotes.path();
    remote_project(&remotes, "project");
    other(&remotes);
    let log = origin.join("gh.log");
    let output = update_repositories(origin, &log, &[]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(
        read(origin, "nix-calls"),
        "run nixpkgs#oxfmt -- --write package.json\n\
         run nixpkgs#oxfmt -- --write template-sync.json\n"
    );
}

#[test]
fn updates_existing_pull_request_body_through_the_api() {
    let remotes = remotes();
    let origin = remotes.path();
    remote_project(&remotes, "project");
    other(&remotes);
    let log = origin.join("gh.log");
    let output = update_repositories_with(origin, &log, &[], "7");
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(stdout(&output).contains("jruz/project: pull request updated\n"));
    let calls = read(origin, "gh.log");
    assert!(
        calls.contains("api --method PATCH repos/jruz/project/pulls/7 --raw-field body=Sync with")
    );
    assert!(!calls.contains("pr create"));
    assert!(!calls.contains("pr edit"));
}
