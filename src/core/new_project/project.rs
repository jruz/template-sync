#[derive(Clone, Copy, Debug, clap::ValueEnum)]
pub enum Template {
    #[value(alias = "rs")]
    Rust,
    #[value(alias = "ts")]
    Typescript,
}

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
pub enum Visibility {
    Private,
    Public,
}

#[derive(Debug)]
struct Name(String);

#[derive(Debug)]
struct Owner(String);

#[derive(Debug)]
pub struct Project {
    description: String,
    name: Name,
    owner: Owner,
    template: Template,
    topics: Vec<String>,
    visibility: Visibility,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("description must be a single line of at most 350 characters")]
    Description,
    #[error(
        "invalid project name {0:?}; use up to 64 ASCII letters, digits, hyphens or underscores, starting with a letter"
    )]
    Name(String),
    #[error("invalid GitHub owner {0:?}")]
    Owner(String),
    #[error(
        "provide up to 20 distinct topics, each containing up to 50 lowercase letters, digits or hyphens"
    )]
    Topics,
}

pub fn validate(
    name: String,
    owner: String,
    template: Template,
    visibility: Visibility,
    description: String,
    topics: &str,
) -> Result<Project, Error> {
    let valid_name = name.len() <= 64
        && name
            .bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_alphabetic())
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'));
    if !valid_name {
        let error = Error::Name(name);
        return Err(error);
    }
    let valid_owner = !owner.is_empty()
        && owner.len() <= 39
        && !owner.starts_with('-')
        && !owner.ends_with('-')
        && !owner.contains("--")
        && owner
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-');
    if !valid_owner {
        let error = Error::Owner(owner);
        return Err(error);
    }
    if description.chars().count() > 350 || description.chars().any(char::is_control) {
        return Err(Error::Description);
    }
    let topics = if topics.is_empty() {
        Vec::new()
    } else {
        topics
            .split(',')
            .map(str::trim)
            .map(str::to_owned)
            .collect::<Vec<_>>()
    };
    let valid_topics = topics.len() <= 20
        && topics.iter().enumerate().all(|(index, topic)| {
            !topic.is_empty()
                && topic.len() <= 50
                && topic
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
                && topics.iter().take(index).all(|previous| previous != topic)
        });
    if !valid_topics {
        return Err(Error::Topics);
    }
    Ok(Project {
        name: Name(name),
        owner: Owner(owner),
        template,
        visibility,
        description,
        topics,
    })
}

pub fn description(project: &Project) -> &str {
    &project.description
}

pub fn topics(project: &Project) -> &[String] {
    &project.topics
}

pub fn template(project: &Project) -> Template {
    project.template
}

pub fn name(project: &Project) -> &str {
    &project.name.0
}

pub fn repository(project: &Project) -> String {
    format!("{}/{}", project.owner.0, name(project))
}

pub fn template_repository(template: Template) -> &'static str {
    match template {
        Template::Rust => "jruz/template-rs",
        Template::Typescript => "jruz/template-ts",
    }
}

pub fn visibility(project: &Project) -> &'static str {
    match project.visibility {
        Visibility::Private => "private",
        Visibility::Public => "public",
    }
}
