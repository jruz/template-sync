use std::path::{Path, PathBuf};
use std::process::Command;

use crate::utils::binary::{stderr, stdout};

const MANIFEST: &str = r#"{
  "merge": ["package.json"],
  "replace": ["LICENSE", "scripts/"],
  "review": ["justfile"]
}
"#;

const PROJECT_FILES: [(&str, &str); 3] = [
    ("LICENSE", "old licence\n"),
    (
        "package.json",
        "{\n  \"name\": \"project\",\n  \"devDependencies\": {\n    \"oxlint\": \"1.0.0\"\n  }\n}\n",
    ),
    ("justfile", "check:\n    just test\n"),
];

pub fn project_file(revision: &str, review: &str) -> String {
    format!(
        "{{ \"review\": [{review}], \"revision\": \"{revision}\", \"template\": \"jruz/template-ts\" }}\n"
    )
}

pub fn write_all(root: &Path, files: &[(&str, &str)]) {
    files
        .iter()
        .try_for_each(|(path, content)| {
            let target = root.join(path);
            let parent = target.parent().expect("parent");
            std::fs::create_dir_all(parent)?;
            std::fs::write(target, content)
        })
        .expect("write files");
}

pub fn git(directory: &Path, arguments: &[&str]) -> String {
    let output = Command::new("git")
        .args(arguments)
        .current_dir(directory)
        .output()
        .expect("run git");
    assert!(output.status.success(), "{}", stderr(&output));
    stdout(&output)
}

fn commit_all(root: &Path) {
    git(root, &["init", "--quiet"]);
    commit_changes(root);
}

pub fn commit_changes(root: &Path) {
    git(root, &["add", "--all"]);
    git(
        root,
        &[
            "-c",
            "user.name=test",
            "-c",
            "user.email=test@example.com",
            "commit",
            "--quiet",
            "--message",
            "init",
        ],
    );
}

pub fn remotes() -> tempfile::TempDir {
    let directory = tempfile::tempdir().expect("temporary directory");
    let root = directory.path().join("template-ts.git");
    write_all(
        &root,
        &[
            ("template.json", MANIFEST),
            ("LICENSE", "licence\n"),
            ("scripts/lib/log.ts", "export {};\n"),
            (
                "package.json",
                "{\n  \"devDependencies\": {\n    \"oxlint\": \"2.0.0\"\n  }\n}\n",
            ),
            ("justfile", "check:\n"),
        ],
    );
    commit_all(&root);
    directory
}

pub fn template_directory(remotes: &tempfile::TempDir) -> PathBuf {
    remotes.path().join("template-ts.git")
}

pub fn template_revision(remotes: &tempfile::TempDir) -> String {
    let template = template_directory(remotes);
    git(&template, &["rev-parse", "HEAD"]).trim().to_owned()
}

fn project_files(root: &Path, remotes: &tempfile::TempDir) {
    let revision = template_revision(remotes);
    let config = project_file(&revision, "");
    write_all(root, &PROJECT_FILES);
    write_all(root, &[("template-sync.json", &config)]);
    commit_all(root);
}

pub fn remote_project(remotes: &tempfile::TempDir, name: &str) -> PathBuf {
    let root = remotes.path().join(format!("{name}.git"));
    project_files(&root, remotes);
    root
}

pub fn project(remotes: &tempfile::TempDir) -> tempfile::TempDir {
    let directory = tempfile::tempdir().expect("temporary directory");
    let root = directory.path();
    project_files(root, remotes);
    directory
}

pub fn read(root: &Path, file: &str) -> String {
    let path = root.join(file);
    std::fs::read_to_string(path).expect("read")
}
