---
name: template
description: Create and sync repositories with template-sync. Use when creating Rust or TypeScript projects, checking template drift, pulling synced files, or resolving REVIEW diffs.
---

# Template Sync

## File modes

The template's `template.json` assigns synced paths to these modes:

| Mode      | How it works                                                                    |
| --------- | ------------------------------------------------------------------------------- |
| `replace` | `pull` overwrites the file with the template version.                           |
| `merge`   | `pull` combines template content with project content. See merged files.        |
| `review`  | You apply relevant template changes manually. `pull` leaves the file untouched. |

## Merged files

### package.json

- The template manages `scripts`, `dependencies`, `devDependencies`, `engines`, `private`, and `type`.
- Inside those sections, packages and scripts the template defines get the template's version.
- Packages and scripts you added yourself are kept.
- Packages and scripts the template stops defining are removed.
- Everything else, such as `name` and `version`, is yours.

### Cargo.toml

- The template manages `[build-dependencies]`, `[lints]`, and `[profile]`.
- Inside those sections, entries the template defines get the template's value.
- Entries you added yourself are kept.
- Entries the template stops defining are removed.
- `[package]` and `[dependencies]` are yours.

### justfile

- Template recipes get the template's changes.
- Recipes you added yourself are kept.
- Lines you added inside template recipes are kept.
- When your edit and the template's edit collide, `pull` stops without changing anything.
- Resolve the reported recipe against the template before retrying.

### github-settings.json

- `branch_protection`, `actions`, `security`, and `labels` come from the template.
- Private repositories keep `"branch_protection": null` when already configured.
- `settings` gets the template's values, except `description`, `private`, and `is_template`.
- Private repositories without branch protection keep their `allow_auto_merge` setting.

### woodpecker-settings.yaml

- Everything is replaced by the template, except `repo`.

### Other files

- Lines above `# project entries below, everything above is managed by template-sync` come from the template.
- Lines below it are yours.

## Project overrides

To manage a synced file manually, add its path to the project's `review` array in `template-sync.json`:

```json
{
  "review": ["Dockerfile.rust", "renovate.json"]
}
```

- Entries override the template's mode for those paths.
- File paths match exactly.
- A trailing `/` covers synced files beneath that directory.
- Entries must match files already synced by the template.
- Remove a path to restore the template's mode.

## Sync workflow

`template-sync.json` records the template revision used as the project's baseline.

1. Run `template-sync check` to see pending changes.
2. Inspect `REVIEW` diffs and apply relevant changes manually.
3. Run `template-sync pull --dry-run` to preview automatic changes.
4. Run `template-sync pull` to apply automatic changes, format written JSON and YAML with `nix run nixpkgs#oxfmt`, and advance the recorded revision.
5. Run `template-sync check` and confirm `in sync`.

| Command                        | Review-file comparison                                   |
| ------------------------------ | -------------------------------------------------------- |
| `template-sync check`          | Recorded template revision → current template.           |
| `template-sync check --review` | Local files → current template, including after pulling. |

- Advancing the revision acknowledges review changes, including changes you chose to skip.
- Review differences do not fail `check`.
- Automatic sync drift fails `check`.
- `pull` preserves review files when the template deletes them.

## Apply repository settings

- After pulling changes to `github-settings.json`, run `github-settings plan` and review the changes before running `github-settings push`.
- After pulling changes to `woodpecker-settings.yaml`, run `woodpecker-settings plan` and review the changes before running `woodpecker-settings apply`.
- Run the corresponding plan command again to confirm no drift.

## Missing GitHub settings

- Run `github-settings pull` to create a missing `github-settings.json` from the repository's live settings.
- Commit `github-settings.json` before running `template-sync pull`.
- Run `template-sync pull` to merge the template's shared policy while preserving repository identity.

## New projects

| Language   | Repository                                              |
| ---------- | ------------------------------------------------------- |
| Rust       | [jruz/template-rs](https://github.com/jruz/template-rs) |
| TypeScript | [jruz/template-ts](https://github.com/jruz/template-ts) |

Run `template-sync new PROJECT` from the parent directory. The wizard prompts for missing options and previews GitHub and Woodpecker settings before applying them.

For automation, pass every option and approve the changes:

```sh
template-sync new PROJECT --owner jruz --template rust --visibility private \
  --description 'Project description' --topics cli,rust --non-interactive --yes
```

- Authenticate `gh`, SSH, `github-settings`, `woodpecker-cli`, and `woodpecker-settings` first.
- Install `oxfmt` to format the generated files.
- Pass `--description ''` or `--topics ''` to leave those fields empty.
- Private repositories start without GitHub branch protection or auto-merge so they work on GitHub Free.
- Install `direnv` before creating a project.
- `just setup` enables Git hooks in the Rust checkout.
- `just setup` approves `.envrc` in both templates.
- `just setup` runs in the TypeScript checkout’s Nix shell.
- TypeScript setup needs Nix and Docker to install dependencies through sandboxed `pnpm`.
- Woodpecker is always activated before the first project push.
- The command commits and pushes the generated project to the default branch.
- Release secrets are not copied or provisioned.
- Changes to visibility or secrets make Woodpecker settings review-only during sync.
- If provisioning stops, inspect the remote and local checkout before finishing setup.
- The command does not delete partial repositories.

For an existing checkout, run `template-sync init` to create `template-sync.json`.

- `Cargo.toml` selects `jruz/template-rs`.
- `package.json` selects `jruz/template-ts` when no `Cargo.toml` exists.
