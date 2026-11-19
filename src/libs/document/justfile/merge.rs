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

fn update_existing(
    source: &str,
    project: &[parse::Item<'_>],
    original: &[parse::Item<'_>],
    template: &[parse::Item<'_>],
) -> Result<String, Error> {
    let mut output = source.to_owned();
    for item in project.iter().rev() {
        let base = lookup(original, &item.key);
        let upstream = lookup(template, &item.key);
        if base == upstream {
            continue;
        }
        let local = Some(item.text);
        let merged = merge_item(&item.key, base, upstream, local)?;
        let span = item.span.clone();
        match merged {
            Some(text) => output.replace_range(span, &text),
            None => output.replace_range(span, ""),
        }
    }
    Ok(output)
}

fn append(output: &mut String, item: &str) {
    if !output.is_empty() && !output.ends_with("\n\n") {
        if !output.ends_with('\n') {
            output.push('\n');
        }
        output.push('\n');
    }
    output.push_str(item);
    output.push('\n');
}

pub fn sync(template: &str, base: Option<&str>, project: &str) -> Result<String, Error> {
    let baseline = base.ok_or(Error::MissingBase)?;
    let original = parse::items(baseline)?;
    let template = parse::items(template)?;
    let items = parse::items(project)?;
    let mut output = update_existing(project, &items, &original, &template)?;
    for item in &template {
        if lookup(&items, &item.key).is_some() {
            continue;
        }
        let previous = lookup(&original, &item.key);
        let upstream = Some(item.text);
        if let Some(text) = merge_item(&item.key, previous, upstream, None)? {
            append(&mut output, &text);
        }
    }
    parse::items(&output)?;
    Ok(output)
}
