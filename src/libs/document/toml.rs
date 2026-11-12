use toml_edit::{Array, DocumentMut, Item, RawString, Table, TableLike, Value};

use crate::libs::document::lines;

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
        .is_some_and(|prefix| prefix.contains(lines::MARKER))
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

fn pruned_entry(
    table: &mut dyn TableLike,
    name: &str,
    template: &dyn TableLike,
    base: &dyn TableLike,
) {
    match (template.get(name), base.get(name)) {
        (None, Some(_)) => {
            table.remove(name);
        }
        (Some(expected), Some(previous)) => {
            if let Some(slot) = table.get_mut(name) {
                let current = slot.clone();
                *slot = prune(expected, previous, current);
            }
        }
        _ => {}
    }
}

fn pruned_tables(template: &dyn TableLike, base: &dyn TableLike, mut project: Item) -> Item {
    let Some(table) = project.as_table_like_mut() else {
        return project;
    };
    let names = table
        .iter()
        .map(|(name, _)| name.to_owned())
        .collect::<Vec<_>>();
    for name in &names {
        pruned_entry(table, name, template, base);
    }
    project
}

fn pruned_array(template: &Array, base: &Array, project: &Array) -> Item {
    let mut kept = project.clone();
    kept.retain(|value| contains(template, value) || !contains(base, value));
    let value = Value::Array(kept);
    Item::Value(value)
}

fn prune(template: &Item, base: &Item, project: Item) -> Item {
    if let (Some(expected), Some(previous), true) = (
        template.as_table_like(),
        base.as_table_like(),
        project.is_table_like(),
    ) {
        return pruned_tables(expected, previous, project);
    }
    if let (Some(expected), Some(previous), Some(current)) =
        (template.as_array(), base.as_array(), project.as_array())
    {
        return pruned_array(expected, previous, current);
    }
    project
}

fn pruned_key(table: &mut Table, key: &str, template: &Item, base: Option<&DocumentMut>) {
    let Some(previous) = base.and_then(|document| document.get(key)) else {
        return;
    };
    if let Some(slot) = table.get_mut(key) {
        let current = slot.clone();
        *slot = prune(template, previous, current);
    }
}

fn merge_key(
    template: &DocumentMut,
    base: Option<&DocumentMut>,
    mut project: DocumentMut,
    key: &str,
) -> Result<DocumentMut, Error> {
    let Some(expected) = template.get(key) else {
        let owned = key.to_owned();
        let error = Error::MissingKey(owned);
        return Err(error);
    };
    let table = project.as_table_mut();
    pruned_key(table, key, expected, base);
    entry(table, key, expected);
    Ok(project)
}

fn document(text: &str) -> Result<DocumentMut, Error> {
    let parsed = text.parse::<DocumentMut>().map_err(Box::new)?;
    Ok(parsed)
}

pub fn keys(
    template: &str,
    base: Option<&str>,
    project: &str,
    keys: &[String],
) -> Result<String, Error> {
    let expected = document(template)?;
    let previous = base.map(document).transpose()?;
    let current = document(project)?;
    let base_document = previous.as_ref();
    let merged = keys.iter().try_fold(current, |merged, key| {
        merge_key(&expected, base_document, merged, key)
    })?;
    let text = merged.to_string();
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn owned(names: &[&str]) -> Vec<String> {
        names.iter().map(|name| (*name).to_owned()).collect()
    }

    fn merge(template: &str, project: &str, names: &[&str]) -> Result<String, Error> {
        let listed = owned(names);
        keys(template, None, project, &listed)
    }

    #[test]
    fn drops_entries_the_template_removed_since_base() {
        let base =
            "[lints.clippy]\n# all projects\na = \"allow\"\nb = \"allow\"\n\n# this project\n";
        let template = "[lints.clippy]\n# all projects\na = \"allow\"\n\n# this project\n";
        let project = "[lints.clippy]\n# all projects\na = \"allow\"\nb = \"allow\"\n\n# this project\nz = \"allow\"\n";
        let names = owned(&["lints"]);
        let previous = Some(base);
        let merged = keys(template, previous, project, &names).expect("merge");
        let expected =
            "[lints.clippy]\n# all projects\na = \"allow\"\n\n# this project\nz = \"allow\"\n";
        assert_eq!(merged, expected);
    }

    #[test]
    fn drops_array_items_the_template_removed_since_base() {
        let base =
            "[dependencies]\nclap = { version = \"=4.6.7\", features = [\"derive\", \"env\"] }\n";
        let template = "[dependencies]\nclap = { version = \"=4.6.7\", features = [\"derive\"] }\n";
        let project = "[dependencies]\nclap = { version = \"=4.6.7\", features = [\"derive\", \"env\", \"cargo\"] }\n";
        let names = owned(&["dependencies"]);
        let previous = Some(base);
        let merged = keys(template, previous, project, &names).expect("merge");
        let expected =
            "[dependencies]\nclap = { version = \"=4.6.7\", features = [\"derive\", \"cargo\"] }\n";
        assert_eq!(merged, expected);
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
    fn inserts_new_template_keys_before_marker() {
        let marker = lines::MARKER;
        let template = format!("[lints.clippy]\na = \"allow\"\nb = \"allow\"\n\n{marker}\n");
        let project = format!("[lints.clippy]\na = \"allow\"\n\n{marker}\nz = \"allow\"\n");
        let merged = merge(&template, &project, &["lints"]).expect("merge");
        let expected =
            format!("[lints.clippy]\na = \"allow\"\nb = \"allow\"\n\n{marker}\nz = \"allow\"\n");
        assert_eq!(merged, expected);
    }

    #[test]
    fn rejects_keys_missing_from_template() {
        let result = merge("", "", &["lints"]);
        assert!(matches!(result, Err(Error::MissingKey(key)) if key == "lints"));
    }
}
