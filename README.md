# template-sync

Keep repositories in sync with their template through automerged pull requests.

## How it works

- A template repository declares its synced files in `template.json`.
- A project opts in with `template-sync.json` naming its template and the template `revision` it last synced.
- `replace` files must match the template exactly, `update` overwrites them.
- `merge` is an array of paths using file-specific rules; unsupported structured files fail.
- `Cargo.toml` syncs build dependencies, lints and profiles while preserving project package metadata and runtime dependencies.
- `package.json` syncs shared scripts, dependency tables, private status, module type and engines while preserving project-only entries.
- Structured merges require existing project files to avoid copying template identity.
- `github-settings.json` uses GitHub-specific rules.
- GitHub sync requires an existing project file and preserves repository identity and topics.
- GitHub sync preserves `settings.description`, `settings.private` and `settings.is_template`.
- GitHub sync applies other template settings and replaces branch protection, Actions, security and labels exactly.
- `merge` line files such as `.gitignore` take every line above the marker from the template and keep the project lines below it.
- The marker is `# project entries below, everything above is managed by template-sync`.
- `merge` drops entries the template removed since `revision`.
- `review` files are never changed and only report template changes since `revision`.
- Synced files the template deleted since `revision` are deleted from the project.
- A project's `review` list in `template-sync.json` turns any synced file into a review file for that project.
- `update` records the template's current commit as `revision`.

## Usage

- `template-sync init` writes `template-sync.json` for the template matching the project files.
- `template-sync check` compares the current project with its template.
- `template-sync update` writes template files into the current project.
- `template-sync update --dry-run` lists the files `update` would overwrite, modify, and delete.
- `template-sync update-repositories --account jruz` opens automerged pull requests for every out-of-sync repository.
- `check` and `update` shallow-clone the template from GitHub into a temporary directory.
- Clones use HTTPS with `gh` as the repository credential helper, so `gh` must be authenticated.

## Development

- `just run`
- `just check`
- `just test`
- `just install`

## Licence

Licensed under the [EUPL 1.2](LICENSE).
