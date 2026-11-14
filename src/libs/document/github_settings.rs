pub const FILE: &str = "github-settings.json";

type Value = serde_json::Value;
type Object = serde_json::Map<String, Value>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{location} is missing {key}")]
    MissingKey { location: String, key: String },
    #[error("{0} is not an object")]
    NotObject(String),
}

fn object<'value>(value: &'value Value, location: &str) -> Result<&'value Object, Error> {
    let Some(object) = value.as_object() else {
        let location = location.to_owned();
        let error = Error::NotObject(location);
        return Err(error);
    };
    Ok(object)
}

fn required<'value>(
    object: &'value Object,
    key: &str,
    location: &str,
) -> Result<&'value Value, Error> {
    let Some(value) = object.get(key) else {
        return Err(Error::MissingKey {
            location: location.to_owned(),
            key: key.to_owned(),
        });
    };
    Ok(value)
}

fn settings(template: &Object, project: &Object) -> Result<Value, Error> {
    let expected = required(template, "settings", "template")?;
    let current = required(project, "settings", "project")?;
    let expected = object(expected, "template.settings")?;
    let current = object(current, "project.settings")?;
    let owned = ["description", "private", "is_template"];
    let shared = expected.iter().filter(|(key, _)| {
        let key = key.as_str();
        !owned.contains(&key)
    });
    let initial = current.clone();
    let merged = shared.fold(initial, |settings, (key, value)| {
        let value = value.clone();
        insert(settings, key, value)
    });
    let value = Value::Object(merged);
    Ok(value)
}

fn insert(mut object: Object, key: &str, value: Value) -> Object {
    let key = key.to_owned();
    object.insert(key, value);
    object
}

pub fn sync(template: &Value, project: &Value) -> Result<Value, Error> {
    let expected = object(template, "template")?;
    let current = object(project, "project")?;
    let settings = settings(expected, current)?;
    let initial = current.clone();
    let initial = insert(initial, "settings", settings);
    let shared = ["branch_protection", "actions", "security", "labels"];
    let merged = shared.into_iter().try_fold(initial, |merged, key| {
        let value = required(expected, key, "template")?.clone();
        let inserted = insert(merged, key, value);
        Ok::<_, Error>(inserted)
    })?;
    let value = Value::Object(merged);
    Ok(value)
}
