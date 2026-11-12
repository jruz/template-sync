# Agent instructions

`template-sync` keeps repositories in sync with their template repository.

## Architecture

- `src/main.rs`: entrypoint, argument parsing, and command dispatch.
- `src/commands/`: one thin module per command.
- `src/core/`: business logic.
- `src/libs/`: self-contained units that could be extracted into their own crate.
- `src/utils/`: small generic helpers.
- `tests/main.rs`: integration test entrypoint.
- `tests/commands/`: tests that run the binary, one file per command.
- `tests/utils/`: shared test helpers.
- `tests/data/bin/gh`: fake `gh` used by the `update-repositories` tests.

## Layers

- `main.rs` calls `commands` and `utils`.
- `commands` calls `core` and `utils`.
- `core` calls `libs` and `utils`.
- `libs` calls `utils`.
- `utils` calls nothing.
- `just rust::architecture-check` enforces these rules with `scripts/architecture-check.nu`.

## Development

- Use `justfile` and `.just/` recipes for development tasks.
- Run Cargo commands through `sandbox`.
- Run `just check`, `just test`, and `slop-detector` after changes.
