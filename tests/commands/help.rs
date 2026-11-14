use crate::utils::binary::{run, stderr, stdout};

#[test]
fn lists_commands() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let root = directory.path();
    let output = run(root, root, &["--help"]);
    assert!(output.status.success());
    let help = stdout(&output);
    assert!(
        ["check", "pull", "update-repositories", "version"]
            .iter()
            .all(|command| help.contains(command))
    );
    assert!(
        !help
            .lines()
            .any(|line| line.trim_start().starts_with("update "))
    );
}

#[test]
fn rejects_the_removed_update_command() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let root = directory.path();
    let output = run(root, root, &["update"]);
    assert!(!output.status.success());
    assert!(stderr(&output).contains("unrecognized subcommand 'update'"));
}

#[test]
fn describes_explicit_review() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let root = directory.path();
    let output = run(root, root, &["check", "--help"]);
    assert!(output.status.success());
    assert!(stdout(&output).contains("--review"));
}
