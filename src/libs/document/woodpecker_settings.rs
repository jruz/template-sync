pub const FILE: &str = "woodpecker-settings.yaml";

type Value = serde_json::Value;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("project woodpecker-settings.yaml is missing repo")]
    MissingRepo,
    #[error("template woodpecker-settings.yaml is not an object")]
    NotObject,
}

pub fn sync(template: &Value, project: &Value) -> Result<Value, Error> {
    let Some(repo) = project.get("repo") else {
        return Err(Error::MissingRepo);
    };
    let Some(expected) = template.as_object() else {
        return Err(Error::NotObject);
    };
    let key = "repo".to_owned();
    let identity = repo.clone();
    let mut synced = expected.clone();
    synced.insert(key, identity);
    let value = Value::Object(synced);
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copies_every_template_field_except_repository_identity() {
        let template_timeout: u32 = 60;
        let project_timeout: u32 = 10;
        let template = serde_json::json!({
            "repo": { "name": "jruz/template-rs" },
            "settings": { "trusted": false, "timeout": template_timeout },
            "secrets": { "GITHUB_TOKEN": { "data": "encrypted-template", "events": ["tag"] } },
            "shared": ["template"]
        });
        let project = serde_json::json!({
            "repo": { "name": "jruz/project" },
            "settings": { "trusted": true, "timeout": project_timeout, "custom": true },
            "secrets": {
                "GITHUB_TOKEN": { "data": "encrypted-project", "events": ["push"] },
                "CUSTOM": { "data": "encrypted-custom" }
            },
            "custom": true
        });
        let expected = serde_json::json!({
            "repo": { "name": "jruz/project" },
            "settings": { "trusted": false, "timeout": template_timeout },
            "secrets": { "GITHUB_TOKEN": { "data": "encrypted-template", "events": ["tag"] } },
            "shared": ["template"]
        });
        let actual = sync(&template, &project).expect("sync");
        assert_eq!(actual, expected);
    }

    #[test]
    fn rejects_project_settings_without_repository_identity() {
        let template = serde_json::json!({"repo": {"name": "jruz/template-rs"}});
        let project = serde_json::json!({"settings": {}});
        let result = sync(&template, &project);
        assert!(matches!(result, Err(Error::MissingRepo)));
    }
}
