use crate::utils::binary::{run, stdout};

#[test]
fn includes_build_metadata() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let root = directory.path();
    let output = run(root, root, &["version"]);
    assert!(output.status.success());
    let version = stdout(&output);
    assert!(version.starts_with(concat!(
        "template-sync \nversion: ",
        env!("CARGO_PKG_VERSION")
    )));
    assert!(version.contains(concat!("revision: ", env!("REVISION"))));
}

#[test]
fn flag_matches_command() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let root = directory.path();
    let command = run(root, root, &["version"]);
    let flag = run(root, root, &["--version"]);
    assert!(flag.status.success());
    assert_eq!(stdout(&flag), stdout(&command));
}
