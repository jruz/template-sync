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

fn kept(template: &Object, base: &Object, key: &str) -> bool {
    template.contains_key(key) || !base.contains_key(key)
}

fn pruned_child(template: &Object, base: &Object, key: &str, current: &Value) -> Value {
    match (template.get(key), base.get(key)) {
        (Some(expected), Some(previous)) => prune(expected, previous, current),
        _ => current.clone(),
    }
}

fn pruned_objects(template: &Object, base: &Object, project: &Object) -> Object {
    project
        .iter()
        .filter(|&(key, _)| kept(template, base, key))
        .map(|(key, current)| (key.clone(), pruned_child(template, base, key, current)))
        .collect()
}

fn pruned_arrays(template: &[Value], base: &[Value], project: &[Value]) -> Vec<Value> {
    project
        .iter()
        .filter(|item| template.contains(item) || !base.contains(item))
        .cloned()
        .collect()
}

#[must_use]
pub fn prune(template: &Value, base: &Value, project: &Value) -> Value {
    match (template, base, project) {
        (Value::Object(expected), Value::Object(previous), Value::Object(current)) => {
            let pruned = pruned_objects(expected, previous, current);
            Value::Object(pruned)
        }
        (Value::Array(expected), Value::Array(previous), Value::Array(current)) => {
            let pruned = pruned_arrays(expected, previous, current);
            Value::Array(pruned)
        }
        _ => project.clone(),
    }
}

fn current_value(
    template_value: &Value,
    base: Option<&Object>,
    current: &Object,
    key: &str,
) -> Option<Value> {
    let project_value = current.get(key)?;
    let previous = base.and_then(|object| object.get(key));
    let pruned = match previous {
        Some(base_value) => prune(template_value, base_value, project_value),
        None => project_value.clone(),
    };
    Some(pruned)
}

fn merge_key(
    expected: &Object,
    base: Option<&Object>,
    current: &Object,
    key: &str,
) -> Result<Value, Error> {
    let Some(template_value) = expected.get(key) else {
        let owned = key.to_owned();
        let error = Error::MissingKey(owned);
        return Err(error);
    };
    let merged = match current_value(template_value, base, current, key) {
        Some(project_value) => value(template_value, &project_value),
        None => template_value.clone(),
    };
    Ok(merged)
}

fn base_object(base: Option<&Value>) -> Result<Option<&Object>, Error> {
    match base {
        Some(Value::Object(object)) => {
            let found = Some(object);
            Ok(found)
        }
        Some(_) => Err(Error::NotObject),
        None => Ok(None),
    }
}

pub fn keys(
    template: &Value,
    base: Option<&Value>,
    project: &Value,
    keys: &[String],
) -> Result<Value, Error> {
    let (Value::Object(expected), Value::Object(current)) = (template, project) else {
        return Err(Error::NotObject);
    };
    let previous = base_object(base)?;
    let initial = current.clone();
    let merged = keys.iter().try_fold(initial, |merged, key| {
        let merged_value = merge_key(expected, previous, current, key)?;
        let inserted = insert(merged, key, merged_value);
        Ok(inserted)
    })?;
    let object = Value::Object(merged);
    Ok(object)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn owned(names: &[&str]) -> Vec<String> {
        names.iter().map(|name| (*name).to_owned()).collect()
    }

    fn merge(template: &Value, project: &Value, names: &[&str]) -> Result<Value, Error> {
        let listed = owned(names);
        keys(template, None, project, &listed)
    }

    #[test]
    fn drops_entries_the_template_removed_since_base() {
        let base = serde_json::json!({
            "devDependencies": { "oxlint": "1.0.0", "eslint": "9.0.0" },
            "ignorePatterns": ["node_modules/", "coverage/"]
        });
        let template = serde_json::json!({
            "devDependencies": { "oxlint": "2.0.0" },
            "ignorePatterns": ["node_modules/"]
        });
        let project = serde_json::json!({
            "devDependencies": { "oxlint": "1.0.0", "eslint": "9.0.0", "vitest": "4.0.0" },
            "ignorePatterns": ["node_modules/", "coverage/", "dist/"]
        });
        let names = owned(&["devDependencies", "ignorePatterns"]);
        let previous = Some(&base);
        let merged = keys(&template, previous, &project, &names).expect("merge");
        let expected = serde_json::json!({
            "devDependencies": { "oxlint": "2.0.0", "vitest": "4.0.0" },
            "ignorePatterns": ["node_modules/", "dist/"]
        });
        assert_eq!(merged, expected);
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
