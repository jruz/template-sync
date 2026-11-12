#[must_use]
pub fn chain(error: &dyn std::error::Error) -> String {
    let first = Some(error);
    std::iter::successors(first, |current| current.source())
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(": ")
}
