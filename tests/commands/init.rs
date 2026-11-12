use crate::utils::binary::{run, stderr, stdout};
use crate::utils::fixtures::{project, read, remotes, template_revision};

#[test]
fn creates_project_file_for_detected_template() {
    let remotes = remotes();
    let origin = remotes.path();
    let project = project(&remotes);
    let root = project.path();
    let file = root.join("template-sync.json");
    std::fs::remove_file(file).expect("remove project file");
    let revision = template_revision(&remotes);
    let output = run(root, origin, &["init"]);
    assert!(output.status.success(), "{}", stderr(&output));
    let tracking = format!("created template-sync.json tracking jruz/template-ts@{revision}\n");
    assert!(stdout(&output).contains(&tracking));
    let created = read(root, "template-sync.json");
    let expected =
        format!("{{\n  \"revision\": \"{revision}\",\n  \"template\": \"jruz/template-ts\"\n}}\n");
    assert_eq!(created, expected);
}

#[test]
fn refuses_initialized_projects() {
    let remotes = remotes();
    let origin = remotes.path();
    let project = project(&remotes);
    let root = project.path();
    let output = run(root, origin, &["init"]);
    assert!(!output.status.success());
    assert!(stderr(&output).contains("error: project already initialized"));
}

#[test]
fn refuses_projects_without_markers() {
    let remotes = remotes();
    let origin = remotes.path();
    let directory = tempfile::tempdir().expect("temporary directory");
    let root = directory.path();
    let output = run(root, origin, &["init"]);
    assert!(!output.status.success());
    assert!(stderr(&output).contains("cannot pick a template"));
}
