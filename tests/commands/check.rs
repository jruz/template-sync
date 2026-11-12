use crate::utils::binary::{run, stderr, stdout};
use crate::utils::fixtures::{project, remotes, template_directory};

#[test]
fn reports_drift() {
    let remotes = remotes();
    let origin = remotes.path();
    let project = project();
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
    assert!(report.contains("REVIEW justfile\n"));
}

#[test]
fn reports_missing_project_file() {
    let remotes = remotes();
    let origin = remotes.path();
    let project = project();
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
    let project = project();
    let root = project.path();
    let template = template_directory(&remotes);
    std::fs::remove_dir_all(&template).expect("remove template");
    let output = run(root, origin, &["check"]);
    assert!(!output.status.success());
    assert!(stderr(&output).contains("git clone"));
}
