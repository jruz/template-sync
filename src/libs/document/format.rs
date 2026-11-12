#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Json,
    Toml,
    Yaml,
}

#[must_use]
pub fn detect(path: &str) -> Option<Format> {
    let extension = std::path::Path::new(path)
        .extension()
        .and_then(std::ffi::OsStr::to_str)?;
    match extension {
        "json" => Some(Format::Json),
        "toml" => Some(Format::Toml),
        "yaml" | "yml" => Some(Format::Yaml),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_formats_by_extension() {
        assert_eq!(detect("package.json"), Some(Format::Json));
        assert_eq!(detect("Cargo.toml"), Some(Format::Toml));
        assert_eq!(detect("pnpm-workspace.yaml"), Some(Format::Yaml));
        assert_eq!(detect(".woodpecker/check.yml"), Some(Format::Yaml));
        assert_eq!(detect("justfile"), None);
    }
}
