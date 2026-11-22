use std::path::Path;

use super::project::{self, Project, Template};
use crate::core::manifest;
use crate::libs::document::{github_settings, woodpecker_settings};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid {0} in template: expected an object")]
    Invalid(&'static str),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Manifest(#[from] manifest::Error),
    #[error(transparent)]
    Yaml(#[from] serde_norway::Error),
}

fn extend(config: &mut serde_json::Map<String, serde_json::Value>, update: serde_json::Value) {
    if let serde_json::Value::Object(entries) = update {
        config.extend(entries);
    }
}

fn github(root: &Path, project: &Project, repository: &str) -> Result<(), Error> {
    let path = root.join(github_settings::FILE);
    let text = std::fs::read_to_string(&path)?;
    let mut document = serde_json::from_str::<serde_json::Value>(&text)?;
    let Some(config) = document.as_object_mut() else {
        let error = Error::Invalid(github_settings::FILE);
        return Err(error);
    };
    let Some(settings) = config
        .get_mut("settings")
        .and_then(serde_json::Value::as_object_mut)
    else {
        let error = Error::Invalid("GitHub settings");
        return Err(error);
    };
    let private = project::visibility(project) == "private";
    let description = project::description(project);
    let topics = project::topics(project);
    let identity =
        serde_json::json!({"description": description, "private": private, "is_template": false});
    extend(settings, identity);
    if private {
        let merge_policy = serde_json::json!({"allow_auto_merge": false});
        extend(settings, merge_policy);
        let protection = serde_json::json!({"branch_protection": null});
        extend(config, protection);
    }
    let metadata = serde_json::json!({"repository": repository, "topics": {"names": topics}});
    extend(config, metadata);
    github_settings::normalize_security(config);
    save_json(&path, &document)
}

fn woodpecker(root: &Path, project: &Project, repository: &str) -> Result<bool, Error> {
    let path = root.join(woodpecker_settings::FILE);
    let text = std::fs::read_to_string(&path)?;
    let mut document = serde_norway::from_str::<serde_json::Value>(&text)?;
    let Some(config) = document.as_object_mut() else {
        let error = Error::Invalid(woodpecker_settings::FILE);
        return Err(error);
    };
    let Some(settings) = config
        .get_mut("settings")
        .and_then(serde_json::Value::as_object_mut)
    else {
        let error = Error::Invalid("Woodpecker settings");
        return Err(error);
    };
    let visibility = settings
        .get("visibility")
        .and_then(serde_json::Value::as_str);
    let desired = project::visibility(project);
    let review_visibility = visibility != Some(desired);
    let settings_update = serde_json::json!({"visibility": desired});
    extend(settings, settings_update);
    let review_secrets = config
        .get("secrets")
        .and_then(serde_json::Value::as_object)
        .is_some_and(|entries| !entries.is_empty());
    let identity = serde_json::json!({"repo": {"name": repository}, "secrets": {}});
    extend(config, identity);
    let updated = serde_norway::to_string(&document)?;
    std::fs::write(path, updated)?;
    Ok(review_secrets || review_visibility)
}

fn rust(root: &Path, name: &str) -> Result<(), Error> {
    let paths = [
        "Cargo.toml",
        "Cargo.lock",
        "tests/utils/binary.rs",
        "tests/commands/help.rs",
        "tests/commands/version.rs",
    ];
    for relative in paths {
        let path = root.join(relative);
        let text = std::fs::read_to_string(&path)?;
        let text = text.replace(
            r#""template-rs \nversion: ""#,
            "env!(\"CARGO_PKG_NAME\"),\n        \" \\nversion: \"",
        );
        let updated = text.replace("template-rs", name);
        std::fs::write(path, updated)?;
    }
    Ok(())
}

fn typescript(root: &Path, name: &str) -> Result<(), Error> {
    let path = root.join("package.json");
    let text = std::fs::read_to_string(&path)?;
    let mut document = serde_json::from_str::<serde_json::Value>(&text)?;
    let Some(package) = document.as_object_mut() else {
        let error = Error::Invalid("package.json");
        return Err(error);
    };
    let name = name.to_ascii_lowercase();
    let identity = serde_json::json!({"name": name});
    extend(package, identity);
    save_json(&path, &document)
}

fn save_json(path: &Path, document: &serde_json::Value) -> Result<(), Error> {
    let text = serde_json::to_string_pretty(document)?;
    let content = format!("{text}\n");
    std::fs::write(path, content)?;
    Ok(())
}

pub fn write(root: &Path, project: &Project, revision: &str) -> Result<(), Error> {
    let repository = project::repository(project);
    github(root, project, &repository)?;
    let review = woodpecker(root, project, &repository)?;
    let name = project::name(project);
    match project::template(project) {
        Template::Rust => rust(root, name)?,
        Template::Typescript => typescript(root, name)?,
    }
    let description = project::description(project);
    let readme = format!(
        "# {name}\n\n{description}\n\n## Licence\n\nLicensed under the [EUPL 1.2](LICENSE).\n"
    );
    let readme_path = root.join("README.md");
    std::fs::write(readme_path, readme)?;
    let template = project::template(project);
    let template = project::template_repository(template);
    manifest::create(root, template, revision)?;
    if review {
        let mut config = manifest::project(root)?;
        config.review = Some(vec![woodpecker_settings::FILE.to_owned()]);
        manifest::record(root, config, revision)?;
    }
    Ok(())
}
