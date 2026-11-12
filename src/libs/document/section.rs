pub const MARKER: &str = "# this project";

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

fn adopted<'text>(head: &[&str], project: &'text str) -> Vec<&'text str> {
    project
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter(|line| !head.contains(line))
        .collect()
}

pub fn merge(template: &str, project: &str) -> Result<String, Error> {
    let Some((head, _)) = split(template) else {
        return Err(Error::MissingMarker);
    };
    let tail = match split(project) {
        Some((_, owned)) => owned,
        None => adopted(&head, project),
    };
    let marker = [MARKER];
    let lines = [head.as_slice(), marker.as_slice(), tail.as_slice()].concat();
    let joined = lines.join("\n");
    Ok(format!("{joined}\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEMPLATE: &str = "/target/\n.direnv/\n\n# this project\n";

    #[test]
    fn replaces_template_lines_and_keeps_project_section() {
        let project = "/target/\nresult\n\n# this project\n/fixtures/out/\n";
        let merged = merge(TEMPLATE, project).expect("merge");
        assert_eq!(
            merged,
            "/target/\n.direnv/\n\n# this project\n/fixtures/out/\n"
        );
    }

    #[test]
    fn adopts_project_only_lines_below_marker() {
        let project = "target/\n/target/\n\nPLAN.md\n";
        let merged = merge(TEMPLATE, project).expect("merge");
        assert_eq!(
            merged,
            "/target/\n.direnv/\n\n# this project\ntarget/\nPLAN.md\n"
        );
    }

    #[test]
    fn rejects_templates_without_marker() {
        let result = merge("/target/\n", "");
        assert!(matches!(result, Err(Error::MissingMarker)));
    }
}
