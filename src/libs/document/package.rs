use crate::libs::document::merge;

pub const FILE: &str = "package.json";

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Merge(#[from] merge::Error),
    #[error("template package.json is not an object")]
    NotObject,
}

pub fn sync(
    template: &serde_json::Value,
    base: Option<&serde_json::Value>,
    project: &serde_json::Value,
) -> Result<serde_json::Value, Error> {
    let Some(object) = template.as_object() else {
        return Err(Error::NotObject);
    };
    let shared = [
        "scripts",
        "devDependencies",
        "private",
        "type",
        "engines",
        "dependencies",
    ];
    let keys = shared
        .into_iter()
        .filter(|key| object.contains_key(*key))
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let synced = merge::keys(template, base, project, &keys)?;
    Ok(synced)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_existing_shared_key_merge_behavior() {
        let base = serde_json::json!({
            "scripts": { "check": "old", "removed": "old" },
            "devDependencies": { "lint": "1", "removed": "1" }
        });
        let template = serde_json::json!({
            "name": "template", "version": "0.1.0",
            "scripts": { "check": "new" },
            "devDependencies": { "lint": "2" },
            "dependencies": { "shared": "2" },
            "private": true, "type": "module", "engines": { "node": "24" }
        });
        let project = serde_json::json!({
            "name": "project", "version": "3.0.0",
            "scripts": { "check": "old", "removed": "old", "custom": "test" },
            "devDependencies": { "lint": "1", "removed": "1", "test": "3" },
            "dependencies": { "shared": "1", "custom": "3" },
            "private": false, "type": "commonjs", "engines": { "node": "22", "bun": "1" }
        });
        let previous = Some(&base);
        let actual = sync(&template, previous, &project).expect("sync");
        let expected = serde_json::json!({
            "name": "project", "version": "3.0.0",
            "scripts": { "check": "new", "custom": "test" },
            "devDependencies": { "lint": "2", "test": "3" },
            "dependencies": { "shared": "2", "custom": "3" },
            "private": true, "type": "module", "engines": { "node": "24", "bun": "1" }
        });
        assert_eq!(actual, expected);
    }
}
