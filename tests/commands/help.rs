use crate::utils::binary::{run, stdout};

#[test]
fn lists_commands() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let root = directory.path();
    let output = run(root, root, &["--help"]);
    assert!(output.status.success());
    let help = stdout(&output);
    assert!(
        ["check", "update", "update-repositories", "version"]
            .iter()
            .all(|command| help.contains(command))
    );
}
