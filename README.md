# template-sync

Keep repositories in sync with their template through automerged pull requests.

## How it works

- A template repository declares its synced files in `template.json`.
- A project opts in with `template-sync.json` naming its template and the template `revision` it last synced.
- `replace` files must match the template exactly, `update` overwrites them.
- `merge` JSON, YAML, and TOML files take the listed top-level `keys` from the template and keep project-only entries.
- `merge` line files such as `.gitignore` take no `keys`, they take every line above the marker from the template and keep the project lines below it.
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
