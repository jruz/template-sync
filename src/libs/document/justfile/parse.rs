#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("duplicate justfile item: {0}")]
    Duplicate(String),
    #[error("justfile parsing was interrupted")]
    Interrupted,
    #[error(transparent)]
    Language(#[from] tree_sitter::LanguageError),
    #[error("missing justfile syntax field in {0}")]
    MissingField(String),
    #[error("invalid justfile syntax")]
    Syntax,
    #[error(transparent)]
    Utf8(#[from] std::str::Utf8Error),
}

pub struct Item<'text> {
    pub key: String,
    pub span: std::ops::Range<usize>,
    pub text: &'text str,
}

fn name(node: tree_sitter::Node<'_>) -> Result<tree_sitter::Node<'_>, Error> {
    let kind = node.kind();
    let field = match kind {
        "recipe" => {
            let mut cursor = node.walk();
            let header = node
                .named_children(&mut cursor)
                .find(|child| child.kind() == "recipe_header")
                .ok_or_else(|| {
                    let owned = kind.to_owned();
                    Error::MissingField(owned)
                })?;
            header.child_by_field_name("name")
        }
        "assignment" | "alias" => node.child_by_field_name("left"),
        "module" => node.child_by_field_name("name"),
        "export" => {
            let assignment = node.named_child(0).ok_or_else(|| {
                let owned = kind.to_owned();
                Error::MissingField(owned)
            })?;
            return name(assignment);
        }
        "setting" => node.child(1),
        "import" => node.named_child(0),
        _ => Some(node),
    };
    field.ok_or_else(|| {
        let owned = kind.to_owned();
        Error::MissingField(owned)
    })
}

fn key(node: tree_sitter::Node<'_>, source: &str) -> Result<String, Error> {
    if node.kind() == "shebang" {
        let key = "shebang".to_owned();
        return Ok(key);
    }
    let named = name(node)?;
    let bytes = source.as_bytes();
    let value = named.utf8_text(bytes)?;
    let kind = match node.kind() {
        "export" => "assignment",
        other => other,
    };
    let key = format!("{kind}:{value}");
    Ok(key)
}

fn has_structure_error(node: tree_sitter::Node<'_>) -> bool {
    if node.is_error() || node.is_missing() {
        return true;
    }
    if !node.has_error() || node.kind() == "recipe_body" {
        return false;
    }
    let mut cursor = node.walk();
    node.children(&mut cursor).any(has_structure_error)
}

pub fn items(source: &str) -> Result<Vec<Item<'_>>, Error> {
    let mut parser = tree_sitter::Parser::new();
    let language = tree_sitter_just::LANGUAGE.into();
    parser.set_language(&language)?;
    let tree = parser.parse(source, None).ok_or(Error::Interrupted)?;
    let root = tree.root_node();
    if has_structure_error(root) {
        return Err(Error::Syntax);
    }
    let mut cursor = root.walk();
    let mut found = Vec::new();
    for node in root
        .named_children(&mut cursor)
        .filter(|node| node.kind() != "comment")
    {
        let key = key(node, source)?;
        if found.iter().any(|item: &Item<'_>| item.key == key) {
            let error = Error::Duplicate(key);
            return Err(error);
        }
        let source_span = node.byte_range();
        let start = source_span.start;
        let raw = source.get(source_span).ok_or(Error::Syntax)?;
        let text = raw.trim_end();
        let span = start..start + text.len();
        found.push(Item { key, span, text });
    }
    Ok(found)
}
