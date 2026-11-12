use crate::utils::binary::{run, stderr, stdout};
use crate::utils::fixtures::{project, read, remotes, write_all};

#[test]
fn writes_project_and_keeps_review_files() {
    let remotes = remotes();
    let origin = remotes.path();
    let project = project();
    let root = project.path();
    let updated = run(root, origin, &["update"]);
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
fn dry_run_lists_files_without_writing() {
    let remotes = remotes();
    let origin = remotes.path();
    let project = project();
    let root = project.path();
    let output = run(root, origin, &["update", "--dry-run"]);
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
    let project = project();
    let root = project.path();
    let reviewing = "{ \"review\": [\"LICENSE\"], \"template\": \"jruz/template-ts\" }\n";
    write_all(root, &[("template-sync.json", reviewing)]);
    let updated = run(root, origin, &["update"]);
    assert!(updated.status.success(), "{}", stderr(&updated));
    assert_eq!(read(root, "LICENSE"), "old licence\n");
    assert!(stdout(&updated).contains("REVIEW LICENSE\n"));
}

#[test]
fn refuses_uncommitted_targets() {
    let remotes = remotes();
    let origin = remotes.path();
    let project = project();
    let root = project.path();
    write_all(root, &[("LICENSE", "edited\n")]);
    let updated = run(root, origin, &["update"]);
    assert!(!updated.status.success());
    assert!(stderr(&updated).contains("uncommitted changes in template targets"));
    assert_eq!(read(root, "LICENSE"), "edited\n");
}
