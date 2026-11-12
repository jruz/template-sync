pub const MARKER: &str = "# project entries below, everything above is managed by template-sync";

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("template has no `{MARKER}` line")]
    MissingMarker,
}

fn split(text: &str) -> Option<(Vec<&str>, Vec<&str>)> {
    let lines = text.lines().collect::<Vec<_>>();
    let position = lines.iter().position(|line| line.trim_end() == MARKER)?;
    let (head, rest) = lines.split_at(position);
    let tail = rest.iter().skip(1).copied().collect();
    Some((head.to_vec(), tail))
}

fn template_lines(base: Option<&str>) -> Vec<&str> {
    let Some(text) = base else {
        return Vec::new();
    };
    match split(text) {
        Some((head, _)) => head,
        None => text.lines().collect(),
    }
}

fn project_parts(project: &str) -> (Vec<&str>, Vec<&str>) {
    if let Some(parts) = split(project) {
        return parts;
    }
    let above = project.lines().collect();
    (above, Vec::new())
}

pub fn merge(template: &str, base: Option<&str>, project: &str) -> Result<String, Error> {
    let Some((head, _)) = split(template) else {
        return Err(Error::MissingMarker);
    };
    let previous = template_lines(base);
    let (above, below) = project_parts(project);
    let moved = above
        .into_iter()
        .filter(|line| !line.trim().is_empty())
        .filter(|line| !head.contains(line))
        .filter(|line| !previous.contains(line))
        .filter(|line| !below.contains(line))
        .collect::<Vec<_>>();
    let marker = [MARKER];
    let lines = [
        head.as_slice(),
        marker.as_slice(),
        moved.as_slice(),
        below.as_slice(),
    ]
    .concat();
    let joined = lines.join("\n");
    Ok(format!("{joined}\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn merged(base: Option<&str>, project: &str) -> String {
        let template = format!("/target/\n.direnv/\n\n{MARKER}\n");
        merge(&template, base, project).expect("merge")
    }

    fn expected(tail: &str) -> String {
        format!("/target/\n.direnv/\n\n{MARKER}\n{tail}")
    }

    #[test]
    fn replaces_template_lines_and_keeps_project_lines() {
        let project = format!("/target/\n\n{MARKER}\n/fixtures/out/\n");
        let result = merged(None, &project);
        assert_eq!(result, expected("/fixtures/out/\n"));
    }

    #[test]
    fn moves_project_lines_above_marker_below_it() {
        let project = format!("/target/\nPLAN.md\n{MARKER}\n/fixtures/out/\n");
        let result = merged(None, &project);
        assert_eq!(result, expected("PLAN.md\n/fixtures/out/\n"));
    }

    #[test]
    fn adopts_project_lines_without_marker() {
        let result = merged(None, "target/\n/target/\n\nPLAN.md\n");
        assert_eq!(result, expected("target/\nPLAN.md\n"));
    }

    #[test]
    fn drops_lines_the_template_removed_since_base() {
        let base = format!("/target/\nresult\n\n{MARKER}\n");
        let project = format!("/target/\nresult\n\n{MARKER}\n/fixtures/out/\n");
        let text = base.as_str();
        let previous = Some(text);
        let result = merged(previous, &project);
        assert_eq!(result, expected("/fixtures/out/\n"));
    }

    #[test]
    fn replaces_bases_without_marker() {
        let previous = Some("/target/\n# this project\n");
        let result = merged(previous, "/target/\n# this project\nPLAN.md\n");
        assert_eq!(result, expected("PLAN.md\n"));
    }

    #[test]
    fn rejects_templates_without_marker() {
        let result = merge("/target/\n", None, "");
        assert!(matches!(result, Err(Error::MissingMarker)));
    }
}
