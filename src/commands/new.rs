use std::io::{self, IsTerminal as _, Write as _};

use crate::core::new_project::{project, provision};
use crate::utils::output::Output;

#[derive(Debug, clap::Args)]
pub struct Arguments {
    #[arg(long, help = "GitHub description (pass an empty string for none)")]
    description: Option<String>,
    #[arg(value_name = "NAME")]
    name: String,
    #[arg(long, help = "Require all options and never prompt")]
    non_interactive: bool,
    #[arg(long)]
    owner: Option<String>,
    #[arg(long, value_enum)]
    template: Option<project::Template>,
    #[arg(
        long,
        help = "Comma-separated GitHub topics (pass an empty string for none)"
    )]
    topics: Option<String>,
    #[arg(long, value_enum)]
    visibility: Option<project::Visibility>,
    #[arg(
        long,
        help = "Approve creation and application of settings without prompting"
    )]
    yes: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("pass --yes to approve non-interactive creation")]
    ApprovalRequired,
    #[error("end of input while waiting for {0}")]
    EndOfInput(&'static str),
    #[error("invalid {0}: {1}")]
    InvalidChoice(&'static str, String),
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error("missing {0}; provide it as a flag")]
    MissingOption(&'static str),
    #[error(transparent)]
    Project(#[from] project::Error),
    #[error(transparent)]
    Provision(#[from] provision::Error),
}

fn prompt(label: &'static str) -> Result<String, Error> {
    let mut stderr = io::stderr();
    write!(stderr, "{label}: ")?;
    stderr.flush()?;
    let stdin = io::stdin();
    let mut value = String::new();
    let bytes = stdin.read_line(&mut value)?;
    if bytes == 0 {
        let error = Error::EndOfInput(label);
        return Err(error);
    }
    let value = value.trim().to_owned();
    Ok(value)
}

fn option(
    value: Option<String>,
    flag: &'static str,
    label: &'static str,
    interactive: bool,
) -> Result<String, Error> {
    match value {
        Some(value) => Ok(value),
        None if interactive => prompt(label),
        None => {
            let error = Error::MissingOption(flag);
            Err(error)
        }
    }
}

fn choice<T>(
    value: Option<T>,
    flag: &'static str,
    label: &'static str,
    interactive: bool,
) -> Result<T, Error>
where
    T: clap::ValueEnum,
{
    match value {
        Some(value) => Ok(value),
        None if interactive => {
            let entered = prompt(label)?;
            T::from_str(&entered, false).map_err(|reason| Error::InvalidChoice(flag, reason))
        }
        None => {
            let error = Error::MissingOption(flag);
            Err(error)
        }
    }
}

fn request(arguments: Arguments, interactive: bool) -> Result<project::Project, Error> {
    let owner = option(arguments.owner, "--owner", "GitHub owner", interactive)?;
    let template = choice(
        arguments.template,
        "--template",
        "Template (rust/typescript)",
        interactive,
    )?;
    let visibility = choice(
        arguments.visibility,
        "--visibility",
        "Visibility (private/public)",
        interactive,
    )?;
    let description = option(
        arguments.description,
        "--description",
        "Description (blank for none)",
        interactive,
    )?;
    let topics = option(
        arguments.topics,
        "--topics",
        "Topics (comma-separated, blank for none)",
        interactive,
    )?;
    let project = project::validate(
        arguments.name,
        owner,
        template,
        visibility,
        description,
        &topics,
    )?;
    Ok(project)
}

fn confirm(label: &'static str, yes: bool) -> Result<bool, Error> {
    if yes {
        return Ok(true);
    }
    let reply = prompt(label)?;
    Ok(matches!(reply.as_str(), "y" | "yes"))
}

fn stopped(text: String) -> Output {
    Output {
        success: true,
        text,
    }
}

pub fn run(arguments: Arguments) -> Result<Output, Error> {
    let interactive = !arguments.non_interactive && io::stdin().is_terminal();
    let yes = arguments.yes;
    let project = request(arguments, interactive)?;
    if !interactive && !yes {
        return Err(Error::ApprovalRequired);
    }
    let parent = std::env::current_dir()?;
    let name = project::name(&project);
    let destination = parent.join(name);
    provision::check_destination(&destination)?;
    let repository = project::repository(&project);
    let visibility = project::visibility(&project);
    let template = project::template(&project);
    let template = project::template_repository(template);
    let target = destination.display();
    let mut stderr = io::stderr();
    writeln!(
        stderr,
        "Create {visibility} {repository} from {template} at {target} with Woodpecker CI"
    )?;
    if !confirm("Create repository? [y/N]", yes)? {
        let text = "Cancelled; nothing created.\n".to_owned();
        let output = stopped(text);
        return Ok(output);
    }
    let setup = provision::prepare(&project, &parent, destination)?;
    let root = provision::destination(&setup);
    let plan = provision::settings_plans(root)?;
    writeln!(stderr, "{plan}")?;
    if !confirm(
        "Apply the settings shown above and push the project? [y/N]",
        yes,
    )? {
        let path = root.display();
        let text = format!(
            "Created {repository} at {path} and activated Woodpecker; settings and project commit not applied.\n"
        );
        let output = stopped(text);
        return Ok(output);
    }
    let commit = provision::finish(&setup)?;
    let verification = provision::settings_plans(root)?;
    writeln!(stderr, "{verification}")?;
    let path = root.display();
    let text = format!(
        "Created https://github.com/{repository}\nClone: {path}\nWoodpecker: enabled\nCommit: {commit}\n"
    );
    Ok(Output {
        success: true,
        text,
    })
}
