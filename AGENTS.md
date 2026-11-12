# Agent instructions

`template-sync` keeps repositories in sync with their template repository.

## Architecture

- `src/main.rs`: entrypoint.
- `src/cli.rs`: argument parsing.
- `src/version.rs`: version metadata from the release skill.
- `build.rs`: Git revision and build timestamp with typed errors using `thiserror`.
- `tests/cli.rs`: CLI integration tests.

## Development

- Use `justfile` and `.just/` recipes for development tasks.
- Run Cargo commands through `sandbox`.
- Run `just check`, `just test`, and `slop-detector` after changes.
