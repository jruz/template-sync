use toml_edit::{DocumentMut, Item, TableLike, Value};

type Json = serde_json::Value;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Parse(#[from] Box<toml_edit::TomlError>),
}

fn table(entries: &dyn TableLike) -> Json {
    let object = entries
        .iter()
        .map(|(key, nested)| (key.to_owned(), item(nested)))
        .collect();
    Json::Object(object)
}

fn value(found: &Value) -> Json {
    match found {
        Value::String(text) => text.value().as_str().into(),
        Value::Integer(number) => (*number.value()).into(),
        Value::Float(number) => (*number.value()).into(),
        Value::Boolean(flag) => (*flag.value()).into(),
        Value::Datetime(moment) => moment.value().to_string().into(),
        Value::Array(items) => items.iter().map(value).collect(),
        Value::InlineTable(entries) => table(entries),
    }
}

fn item(found: &Item) -> Json {
    match found {
        Item::None => Json::Null,
        Item::Value(scalar) => value(scalar),
        Item::Table(entries) => table(entries),
        Item::ArrayOfTables(tables) => tables.iter().map(|entries| table(entries)).collect(),
    }
}

pub fn parse(text: &str) -> Result<Json, Error> {
    let document = text.parse::<DocumentMut>().map_err(Box::new)?;
    let root = document.as_table();
    let converted = table(root);
    Ok(converted)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_tables_and_inline_values() {
        let text = "[dependencies]\nclap = { version = \"=4.6.7\", features = [\"derive\"] }\n\n[[bin]]\nname = \"cli\"\n";
        let converted = parse(text).expect("parse");
        let expected = serde_json::json!({
            "dependencies": { "clap": { "version": "=4.6.7", "features": ["derive"] } },
            "bin": [{ "name": "cli" }]
        });
        assert_eq!(converted, expected);
    }
}
