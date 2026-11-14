use super::parse;

pub const FILE: &str = "justfile";

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("conflicting justfile edits in {0}; resolve against the template before pulling")]
    Conflict(String),
    #[error("cannot merge justfile without its previous template version")]
    MissingBase,
    #[error("conflicting justfile edits in {0}; resolve against the template before pulling:\n{1}")]
    Overlap(String, String),
    #[error(transparent)]
    Parse(#[from] parse::Error),
}

fn lookup<'text>(items: &[parse::Item<'text>], key: &str) -> Option<&'text str> {
    items
        .iter()
        .find(|item| item.key == key)
        .map(|item| item.text)
}

fn merge_item(
    key: &str,
    base: Option<&str>,
    template: Option<&str>,
    project: Option<&str>,
) -> Result<Option<String>, Error> {
    if project == template || template == base {
        let kept = project.map(str::to_owned);
        return Ok(kept);
    }
    if project == base {
        let updated = template.map(str::to_owned);
        return Ok(updated);
    }
    let owned = key.to_owned();
    if let (Some(base), Some(template), Some(project)) = (base, template, project) {
        let original = format!("{base}\n");
        let ours = format!("{project}\n");
        let theirs = format!("{template}\n");
        return match diffy::merge(&original, &ours, &theirs) {
            Ok(merged) => {
                let text = merged.trim_end().to_owned();
                let present = Some(text);
                Ok(present)
            }
            Err(conflict) => {
                let error = Error::Overlap(owned, conflict);
                Err(error)
            }
        };
    }
    let error = Error::Conflict(owned);
    Err(error)
}

pub fn sync(template: &str, base: Option<&str>, project: &str) -> Result<String, Error> {
    let current = project;
    let template = parse::items(template)?;
    let project = parse::items(project)?;
    let baseline = base.ok_or(Error::MissingBase)?;
    let original = parse::items(baseline)?;
    let mut keys = Vec::new();
    for item in project.iter().chain(&template).chain(&original) {
        let key = item.key.as_str();
        if !keys.contains(&key) {
            keys.push(key);
        }
    }
    let merged = keys
        .into_iter()
        .map(|key| {
            let old_item = lookup(&original, key);
            let template_item = lookup(&template, key);
            let project_item = lookup(&project, key);
            merge_item(key, old_item, template_item, project_item)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let present = merged.into_iter().flatten().collect::<Vec<_>>();
    let previous = project.iter().map(|item| item.text);
    if present.iter().map(String::as_str).eq(previous) {
        let unchanged = current.to_owned();
        return Ok(unchanged);
    }
    let content = present.join("\n\n");
    let output = format!("{content}\n");
    parse::items(&output)?;
    Ok(output)
}
