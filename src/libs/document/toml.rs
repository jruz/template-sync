use toml_edit::{Array, DocumentMut, Item, RawString, Table, TableLike, Value};

use crate::libs::document::section;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("missing key in template: {0}")]
    MissingKey(String),
    #[error(transparent)]
    Parse(#[from] Box<toml_edit::TomlError>),
}

fn bare(value: &Value) -> Value {
    let mut stripped = value.clone();
    stripped.decor_mut().clear();
    stripped
}

fn normalized(value: &Value) -> String {
    bare(value).to_string()
}

fn contains(array: &Array, wanted: &Value) -> bool {
    let target = normalized(wanted);
    array.iter().any(|value| normalized(value) == target)
}

fn arrays(template: &Array, project: &Array) -> Array {
    let extra = project.iter().filter(|value| !contains(template, value));
    let mut merged = project.clone();
    merged.clear();
    template
        .iter()
        .chain(extra)
        .map(bare)
        .for_each(|value| merged.push(value));
    merged
}

fn added(template: &Item) -> Item {
    let mut copy = template.clone();
    if let Item::Table(table) = &mut copy {
        table.decor_mut().set_prefix("\n");
    }
    copy
}

fn scalar(template: &Item, project: &Item) -> Item {
    let (Item::Value(expected), Item::Value(current)) = (template, project) else {
        return template.clone();
    };
    let mut replaced = expected.clone();
    *replaced.decor_mut() = current.decor().clone();
    Item::Value(replaced)
}

fn entry(table: &mut dyn TableLike, key: &str, template: &Item) {
    let Some(slot) = table.get_mut(key) else {
        let fresh = added(template);
        table.insert(key, fresh);
        return;
    };
    let current = slot.clone();
    *slot = item(template, current);
}

fn is_marker(table: &Table, name: &str) -> bool {
    table
        .key(name)
        .and_then(|key| key.leaf_decor().prefix())
        .and_then(RawString::as_str)
        .is_some_and(|prefix| prefix.contains(section::MARKER))
}

fn before_marker(table: &mut Table, added: usize) {
    let names = table
        .iter()
        .map(|(name, _)| name.to_owned())
        .collect::<Vec<_>>();
    let Some(marker) = names.iter().position(|name| is_marker(table, name)) else {
        return;
    };
    let moved = names.len() - added - marker;
    names.iter().skip(marker).take(moved).for_each(|name| {
        if let Some((key, value)) = table.remove_entry(name) {
            table.insert_formatted(&key, value);
        }
    });
}

fn tables(template: &dyn TableLike, mut project: Item) -> Item {
    let Some(table) = project.as_table_like_mut() else {
        return project;
    };
    let added = template
        .iter()
        .filter(|&(key, _)| !table.contains_key(key))
        .count();
    template
        .iter()
        .for_each(|(key, expected)| entry(table, key, expected));
    if let Item::Table(concrete) = &mut project {
        before_marker(concrete, added);
    }
    project
}

fn item(template: &Item, project: Item) -> Item {
    if let (Some(expected), true) = (template.as_table_like(), project.is_table_like()) {
        return tables(expected, project);
    }
    if let (Some(expected), Some(current)) = (template.as_array(), project.as_array()) {
        let merged = arrays(expected, current);
        let mut value = Value::Array(merged);
        *value.decor_mut() = current.decor().clone();
        return Item::Value(value);
    }
    scalar(template, &project)
}

fn merge_key(
    template: &DocumentMut,
    mut project: DocumentMut,
    key: &str,
) -> Result<DocumentMut, Error> {
    let Some(expected) = template.get(key) else {
        let owned = key.to_owned();
        let error = Error::MissingKey(owned);
        return Err(error);
    };
    let table = project.as_table_mut();
    entry(table, key, expected);
    Ok(project)
}

pub fn keys(template: &str, project: &str, keys: &[String]) -> Result<String, Error> {
    let expected = template.parse::<DocumentMut>().map_err(Box::new)?;
    let current = project.parse::<DocumentMut>().map_err(Box::new)?;
    let merged = keys
        .iter()
        .try_fold(current, |merged, key| merge_key(&expected, merged, key))?;
    let text = merged.to_string();
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn merge(template: &str, project: &str, names: &[&str]) -> Result<String, Error> {
        let owned = names
            .iter()
            .map(|name| (*name).to_owned())
            .collect::<Vec<_>>();
        keys(template, project, &owned)
    }

    #[test]
    fn overrides_template_keys_and_keeps_project_formatting() {
        let template = "[dependencies]\nclap = { version = \"=4.7.0\", features = [\"derive\"] }\n";
        let project = "[package]\nname = \"project\"\n\n# pinned\n[dependencies]\n# cli\nclap = { version = \"=4.6.7\", features = [\"derive\"] }\nignore = \"=0.4.33\"\n";
        let merged = merge(template, project, &["dependencies"]).expect("merge");
        let expected = "[package]\nname = \"project\"\n\n# pinned\n[dependencies]\n# cli\nclap = { version = \"=4.7.0\", features = [\"derive\"] }\nignore = \"=0.4.33\"\n";
        assert_eq!(merged, expected);
    }

    #[test]
    fn adds_missing_tables() {
        let template = "[build-dependencies]\nthiserror = \"=2.0.21\"\n";
        let project = "[package]\nname = \"project\"\n";
        let merged = merge(template, project, &["build-dependencies"]).expect("merge");
        let expected =
            "[package]\nname = \"project\"\n\n[build-dependencies]\nthiserror = \"=2.0.21\"\n";
        assert_eq!(merged, expected);
    }

    #[test]
    fn unions_arrays_with_template_items_first() {
        let template = "[dependencies]\nclap = { version = \"=4.6.7\", features = [\"derive\"] }\n";
        let project =
            "[dependencies]\nclap = { version = \"=4.6.7\", features = [\"env\", \"derive\"] }\n";
        let merged = merge(template, project, &["dependencies"]).expect("merge");
        let expected =
            "[dependencies]\nclap = { version = \"=4.6.7\", features = [\"derive\", \"env\"] }\n";
        assert_eq!(merged, expected);
    }

    #[test]
    fn inserts_new_template_keys_before_project_section() {
        let template =
            "[lints.clippy]\n# all projects\na = \"allow\"\nb = \"allow\"\n\n# this project\n";
        let project =
            "[lints.clippy]\n# all projects\na = \"allow\"\n\n# this project\nz = \"allow\"\n";
        let merged = merge(template, project, &["lints"]).expect("merge");
        let expected = "[lints.clippy]\n# all projects\na = \"allow\"\nb = \"allow\"\n\n# this project\nz = \"allow\"\n";
        assert_eq!(merged, expected);
    }

    #[test]
    fn rejects_keys_missing_from_template() {
        let result = merge("", "", &["lints"]);
        assert!(matches!(result, Err(Error::MissingKey(key)) if key == "lints"));
    }
}
