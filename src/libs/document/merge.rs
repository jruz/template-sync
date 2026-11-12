type Value = serde_json::Value;
type Object = serde_json::Map<String, Value>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("missing key in template: {0}")]
    MissingKey(String),
    #[error("top level is not an object")]
    NotObject,
}

fn insert(mut map: Object, key: &str, value: Value) -> Object {
    let owned = key.to_owned();
    map.insert(owned, value);
    map
}

fn entry(template: &Object, key: &str, current: &Value) -> (String, Value) {
    let owned = key.to_owned();
    let merged = match template.get(key) {
        Some(expected) => value(expected, current),
        None => current.clone(),
    };
    (owned, merged)
}

fn objects(template: &Object, project: &Object) -> Object {
    let updated = project
        .iter()
        .map(|(key, current)| entry(template, key, current));
    let added = template
        .iter()
        .filter(|&(key, _)| !project.contains_key(key))
        .map(|(key, expected)| (key.clone(), expected.clone()));
    updated.chain(added).collect()
}

fn arrays(template: &[Value], project: &[Value]) -> Vec<Value> {
    let extra = project.iter().filter(|item| !template.contains(item));
    template.iter().chain(extra).cloned().collect()
}

#[must_use]
pub fn value(template: &Value, project: &Value) -> Value {
    if let (Value::Object(expected), Value::Object(current)) = (template, project) {
        let merged = objects(expected, current);
        return Value::Object(merged);
    }
    if let (Value::Array(expected), Value::Array(current)) = (template, project) {
        let merged = arrays(expected, current);
        return Value::Array(merged);
    }
    template.clone()
}

fn merge_key(expected: &Object, current: &Object, key: &str) -> Result<Value, Error> {
    let Some(template_value) = expected.get(key) else {
        let owned = key.to_owned();
        let error = Error::MissingKey(owned);
        return Err(error);
    };
    let merged = match current.get(key) {
        Some(project_value) => value(template_value, project_value),
        None => template_value.clone(),
    };
    Ok(merged)
}

pub fn keys(template: &Value, project: &Value, keys: &[String]) -> Result<Value, Error> {
    let (Value::Object(expected), Value::Object(current)) = (template, project) else {
        return Err(Error::NotObject);
    };
    let initial = current.clone();
    let merged = keys.iter().try_fold(initial, |merged, key| {
        let merged_value = merge_key(expected, current, key)?;
        let inserted = insert(merged, key, merged_value);
        Ok(inserted)
    })?;
    let object = Value::Object(merged);
    Ok(object)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn merge(template: &Value, project: &Value, names: &[&str]) -> Result<Value, Error> {
        let owned = names
            .iter()
            .map(|name| (*name).to_owned())
            .collect::<Vec<_>>();
        keys(template, project, &owned)
    }

    #[test]
    fn overrides_template_keys_and_keeps_project_keys() {
        let template = serde_json::json!({
            "devDependencies": { "oxlint": "2.0.0", "knip": "6.0.0" },
            "name": "template"
        });
        let project = serde_json::json!({
            "name": "project",
            "devDependencies": { "vitest": "4.0.0", "oxlint": "1.0.0" }
        });
        let merged = merge(&template, &project, &["devDependencies"]).expect("merge");
        let expected = serde_json::json!({
            "name": "project",
            "devDependencies": { "vitest": "4.0.0", "oxlint": "2.0.0", "knip": "6.0.0" }
        });
        assert_eq!(merged, expected);
    }

    #[test]
    fn unions_arrays_with_template_items_first() {
        let template = serde_json::json!({ "ignorePatterns": ["node_modules/", "target/"] });
        let project = serde_json::json!({ "ignorePatterns": ["dist/", "node_modules/"] });
        let merged = merge(&template, &project, &["ignorePatterns"]).expect("merge");
        let expected =
            serde_json::json!({ "ignorePatterns": ["node_modules/", "target/", "dist/"] });
        assert_eq!(merged, expected);
    }

    #[test]
    fn adds_missing_keys() {
        let template = serde_json::json!({ "engines": { "node": "24.0.0" } });
        let project = serde_json::json!({ "private": true });
        let merged = merge(&template, &project, &["engines"]).expect("merge");
        let expected = serde_json::json!({ "private": true, "engines": { "node": "24.0.0" } });
        assert_eq!(merged, expected);
    }

    #[test]
    fn rejects_keys_missing_from_template() {
        let template = serde_json::json!({});
        let project = serde_json::json!({});
        let result = merge(&template, &project, &["engines"]);
        assert!(matches!(result, Err(Error::MissingKey(key)) if key == "engines"));
    }
}
