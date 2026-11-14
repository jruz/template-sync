use crate::libs::document::{
    cargo, format, github_settings, keys, package, toml_value, woodpecker_settings,
};

type Value = serde_json::Value;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Cargo(#[from] cargo::Error),
    #[error(transparent)]
    GithubSettings(#[from] github_settings::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Package(#[from] package::Error),
    #[error(transparent)]
    TomlValue(#[from] toml_value::Error),
    #[error("unsupported data format: {0}")]
    Unsupported(String),
    #[error(transparent)]
    WoodpeckerSettings(#[from] woodpecker_settings::Error),
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

pub fn merge(path: &str, sources: &Sources<'_>) -> Result<Merged, Error> {
    if path == cargo::FILE {
        let after = cargo::sync(sources.template, sources.base, sources.project)?;
        return Ok(Merged {
            before: sources.project.to_owned(),
            after,
        });
    }
    if path == package::FILE {
        let template = parse_json(sources.template)?;
        let base = sources.base.map(parse_json).transpose()?;
        let project = parse_json(sources.project)?;
        let previous = base.as_ref();
        let synced = package::sync(&template, previous, &project)?;
        return Ok(Merged {
            before: print_json(&project)?,
            after: print_json(&synced)?,
        });
    }
    if path == github_settings::FILE {
        let template = parse_json(sources.template)?;
        let project = parse_json(sources.project)?;
        let synced = github_settings::sync(&template, &project)?;
        return Ok(Merged {
            before: print_json(&project)?,
            after: print_json(&synced)?,
        });
    }
    if path == woodpecker_settings::FILE {
        let template = parse_yaml(sources.template)?;
        let project = parse_yaml(sources.project)?;
        let synced = woodpecker_settings::sync(&template, &project)?;
        return Ok(Merged {
            before: serde_norway::to_string(&project)?,
            after: serde_norway::to_string(&synced)?,
        });
    }
    let owned = path.to_owned();
    let error = Error::Unsupported(owned);
    Err(error)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_json_preserving_key_order() {
        let text = "{\n  \"b\": 1,\n  \"a\": [\n    \"x\"\n  ]\n}\n";
        let sources = Sources {
            base: None,
            project: text,
            template: text,
        };
        let merged = merge("package.json", &sources).expect("merge");
        assert_eq!(merged.after, text);
        assert_eq!(merged.before, text);
    }
}
