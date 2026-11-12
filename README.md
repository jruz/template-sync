# template-sync

Keep repositories in sync with their template through automerged pull requests.

## How it works

- A template repository declares its synced files in `template.json`.
- A project opts in with `template-sync.json` naming its template.
- `copy` files must match the template exactly.
- `merge` JSON, YAML, and TOML files take the listed top-level keys from the template and keep project-only entries.
- `section` files take every line above `# this project` from the template and keep the project lines below it.
- `review` files are reported but never changed.
- A project's `review` list in `template-sync.json` turns any synced file into a review file for that project.

## Usage

- `template-sync init` writes `template-sync.json` for the template matching the project files.
- `template-sync check` compares the current project with its template.
- `template-sync update` writes template files into the current project.
- `template-sync update --dry-run` lists the files `update` would overwrite and modify.
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
