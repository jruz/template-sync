use std::ffi::OsStr;
use std::path::Path;
use std::process::Command;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{program} {arguments}: {stderr}")]
    Failed {
        arguments: String,
        program: String,
        stderr: String,
    },
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("{program} {arguments}: terminated by signal")]
    Signal { arguments: String, program: String },
    #[error(transparent)]
    Utf8(#[from] std::string::FromUtf8Error),
}

fn join<S>(arguments: &[S]) -> String
where
    S: AsRef<OsStr>,
{
    arguments
        .iter()
        .map(|argument| argument.as_ref().to_string_lossy())
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn run<S>(program: &str, arguments: &[S], directory: &Path) -> Result<String, Error>
where
    S: AsRef<OsStr>,
{
    let output = Command::new(program)
        .args(arguments)
        .current_dir(directory)
        .output()?;
    if !output.status.success() {
        let stderr = String::from_utf8(output.stderr)?;
        return Err(Error::Failed {
            arguments: join(arguments),
            program: program.to_owned(),
            stderr: stderr.trim().to_owned(),
        });
    }
    let stdout = String::from_utf8(output.stdout)?;
    let trimmed = stdout.trim().to_owned();
    Ok(trimmed)
}

pub fn exit_code<S>(program: &str, arguments: &[S], directory: &Path) -> Result<i32, Error>
where
    S: AsRef<OsStr>,
{
    let status = Command::new(program)
        .args(arguments)
        .current_dir(directory)
        .output()?
        .status;
    let Some(code) = status.code() else {
        return Err(Error::Signal {
            arguments: join(arguments),
            program: program.to_owned(),
        });
    };
    Ok(code)
}
