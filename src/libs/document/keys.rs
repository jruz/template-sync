type Value = serde_json::Value;

#[derive(Debug, PartialEq, Eq)]
pub enum Kind {
    Added { after: String },
    Changed { after: String, before: String },
    Removed { before: String },
}

#[derive(Debug, PartialEq, Eq)]
pub struct Change {
    pub kind: Kind,
    pub path: String,
}

fn join(path: &str, key: &str) -> String {
    if path.is_empty() {
        return key.to_owned();
    }
    format!("{path}.{key}")
}

fn printed(value: &Value) -> String {
    format!("{value:#}")
}

fn has_objects(values: &[Value]) -> bool {
    values.iter().any(Value::is_object)
}

fn objects(
    path: &str,
    before: &serde_json::Map<String, Value>,
    after: &serde_json::Map<String, Value>,
) -> Vec<Change> {
    let removed = before.keys().filter(|key| !after.contains_key(*key));
    after
        .keys()
        .chain(removed)
        .flat_map(|key| {
            let nested = join(path, key);
            let old = before.get(key);
            let new = after.get(key);
            walk(&nested, old, new)
        })
        .collect()
}

fn arrays(path: &str, before: &[Value], after: &[Value]) -> Vec<Change> {
    let longest = after.len();
    let length = before.len().max(longest);
    (0..length)
        .flat_map(|index| {
            let position = index.to_string();
            let nested = join(path, &position);
            let old = before.get(index);
            let new = after.get(index);
            walk(&nested, old, new)
        })
        .collect()
}

fn leaf(path: &str, before: &Value, after: &Value) -> Vec<Change> {
    if before == after {
        return Vec::new();
    }
    let kind = Kind::Changed {
        after: printed(after),
        before: printed(before),
    };
    let path = path.to_owned();
    vec![Change { kind, path }]
}

fn single(path: &str, kind: Kind) -> Vec<Change> {
    let path = path.to_owned();
    vec![Change { kind, path }]
}

fn walk(path: &str, before: Option<&Value>, after: Option<&Value>) -> Vec<Change> {
    match (before, after) {
        (Some(Value::Object(old)), Some(Value::Object(new))) => objects(path, old, new),
        (Some(Value::Array(old)), Some(Value::Array(new)))
            if has_objects(old) || has_objects(new) =>
        {
            arrays(path, old, new)
        }
        (Some(old), Some(new)) => leaf(path, old, new),
        (None, Some(new)) => {
            let added = Kind::Added {
                after: printed(new),
            };
            single(path, added)
        }
        (Some(old), None) => {
            let removed = Kind::Removed {
                before: printed(old),
            };
            single(path, removed)
        }
        (None, None) => Vec::new(),
    }
}

#[must_use]
pub fn changes(before: &Value, after: &Value) -> Vec<Change> {
    let old = Some(before);
    let new = Some(after);
    walk("", old, new)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn change(path: &str, kind: Kind) -> Change {
        let path = path.to_owned();
        Change { kind, path }
    }

    #[test]
    fn reports_leaf_changes_with_dotted_paths() {
        let before = serde_json::json!({ "lints": { "a": "deny", "z": "allow" } });
        let after = serde_json::json!({ "lints": { "a": "allow", "b": "allow" } });
        let expected = vec![
            change(
                "lints.a",
                Kind::Changed {
                    after: "\"allow\"".to_owned(),
                    before: "\"deny\"".to_owned(),
                },
            ),
            change(
                "lints.b",
                Kind::Added {
                    after: "\"allow\"".to_owned(),
                },
            ),
            change(
                "lints.z",
                Kind::Removed {
                    before: "\"allow\"".to_owned(),
                },
            ),
        ];
        assert_eq!(changes(&before, &after), expected);
    }

    #[test]
    fn indexes_arrays_of_objects_and_keeps_scalar_arrays_whole() {
        let before = serde_json::json!({ "steps": [{ "run": "a" }], "tags": ["x"] });
        let after = serde_json::json!({ "steps": [{ "run": "b" }], "tags": ["x", "y"] });
        let paths = changes(&before, &after)
            .into_iter()
            .map(|found| found.path)
            .collect::<Vec<_>>();
        assert_eq!(paths, ["steps.0.run", "tags"]);
    }
}
