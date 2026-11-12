const RESET: &str = "\x1b[0m";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Style {
    Color,
    Plain,
}

#[must_use]
pub fn paint(style: Style, color: &str, text: &str) -> String {
    match style {
        Style::Color => format!("{color}{text}{RESET}"),
        Style::Plain => text.to_owned(),
    }
}
