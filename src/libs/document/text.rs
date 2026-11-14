use crate::libs::document::{format, github_settings, keys, merge, toml, toml_value};

const JSON: Codec = Codec {
    parse: parse_json,
    print: print_json,
};

const YAML: Codec = Codec {
    parse: parse_yaml,
    print: print_yaml,
};

type Value = serde_json::Value;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    GithubSettings(#[from] github_settings::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Merge(#[from] merge::Error),
    #[error(transparent)]
    Toml(#[from] toml::Error),
    #[error(transparent)]
    TomlValue(#[from] toml_value::Error),
    #[error("unsupported data format: {0}")]
    Unsupported(String),
    #[error(transparent)]
    Yaml(#[from] serde_norway::Error),
}

pub struct Merged {
    pub after: String,
    pub before: String,
}

pub struct Sources<'text> {
    pub base: Option<&'text str>,
    pub project: &'text str,
    pub template: &'text str,
}

struct Codec {
    parse: fn(&str) -> Result<Value, Error>,
    print: fn(&Value) -> Result<String, Error>,
}

fn parse_json(text: &str) -> Result<Value, Error> {
    let value = serde_json::from_str(text)?;
    Ok(value)
}

fn print_json(value: &Value) -> Result<String, Error> {
    let json = serde_json::to_string_pretty(value)?;
    Ok(format!("{json}\n"))
}

fn parse_yaml(text: &str) -> Result<Value, Error> {
    let value = serde_norway::from_str(text)?;
    Ok(value)
}

fn print_yaml(value: &Value) -> Result<String, Error> {
    let yaml = serde_norway::to_string(value)?;
    Ok(yaml)
}

fn values(codec: &Codec, sources: &Sources<'_>, keys: &[String]) -> Result<Merged, Error> {
    let expected = (codec.parse)(sources.template)?;
    let previous = sources.base.map(codec.parse).transpose()?;
    let current = (codec.parse)(sources.project)?;
    let base = previous.as_ref();
    let merged = merge::keys(&expected, base, &current, keys)?;
    let before = (codec.print)(&current)?;
    let after = (codec.print)(&merged)?;
    Ok(Merged { after, before })
}

fn document(sources: &Sources<'_>, keys: &[String]) -> Result<Merged, Error> {
    let after = toml::keys(sources.template, sources.base, sources.project, keys)?;
    let before = sources.project.to_owned();
    Ok(Merged { after, before })
}

fn parse(kind: format::Format, text: &str) -> Result<Value, Error> {
    match kind {
        format::Format::Json => parse_json(text),
        format::Format::Toml => {
            let value = toml_value::parse(text)?;
            Ok(value)
        }
        format::Format::Yaml => parse_yaml(text),
    }
}

pub fn changes(
    kind: format::Format,
    before: &str,
    after: &str,
) -> Result<Vec<keys::Change>, Error> {
    let old = parse(kind, before)?;
    let new = parse(kind, after)?;
    let found = keys::changes(&old, &new);
    Ok(found)
}

pub fn merge(path: &str, sources: &Sources<'_>, keys: &[String]) -> Result<Merged, Error> {
    if path == github_settings::FILE {
        let template = parse_json(sources.template)?;
        let project = parse_json(sources.project)?;
        let synced = github_settings::sync(&template, &project)?;
        return Ok(Merged {
            before: print_json(&project)?,
            after: print_json(&synced)?,
        });
    }
    let Some(kind) = format::detect(path) else {
        let owned = path.to_owned();
        let error = Error::Unsupported(owned);
        return Err(error);
    };
    match kind {
        format::Format::Json => values(&JSON, sources, keys),
        format::Format::Toml => document(sources, keys),
        format::Format::Yaml => values(&YAML, sources, keys),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_json_preserving_key_order() {
        let text = "{\n  \"b\": 1,\n  \"a\": [\n    \"x\"\n  ]\n}\n";
        let keys = vec!["a".to_owned()];
        let sources = Sources {
            base: None,
            project: text,
            template: text,
        };
        let merged = merge("package.json", &sources, &keys).expect("merge");
        assert_eq!(merged.after, text);
        assert_eq!(merged.before, text);
    }
}
