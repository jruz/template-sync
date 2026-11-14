---
name: template
description: Sync a repository with its template using template-sync. Use when initializing from template-rs or template-ts, checking template drift, pulling synced files, or resolving REVIEW diffs.
---

# Template Sync

## File modes

The template's `template.json` assigns synced paths to these modes:

| Mode      | How it works                                                                       |
| --------- | ---------------------------------------------------------------------------------- |
| `replace` | `pull` overwrites the file with the template version.                              |
| `merge`   | `pull` combines template settings with project settings using file-specific rules. |
| `review`  | You apply relevant template changes manually. `pull` leaves the file untouched.    |

## Justfile merging

- Root `justfile` merge entries use a parser-backed three-way merge.
- Independent template edits merge with project-specific recipe steps.
- Project-only recipes and declarations remain intact.
- Existing project item order is preserved.
- New template items are appended.
- Conflicting edits stop the pull before files or the revision change.
- Resolve the reported item against the template before retrying.

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
4. Run `template-sync pull` to apply automatic changes and advance the recorded revision.
5. Run `template-sync check` and confirm `in sync`.

| Command                        | Review-file comparison                                   |
| ------------------------------ | -------------------------------------------------------- |
| `template-sync check`          | Recorded template revision → current template.           |
| `template-sync check --review` | Local files → current template, including after pulling. |

- Advancing the revision acknowledges review changes, including changes you chose to skip.
- Review differences do not fail `check`.
- Automatic sync drift fails `check`.
- `pull` preserves review files when the template deletes them.

## Missing GitHub settings

- Run `github-settings pull` to create a missing `github-settings.json` from the repository's live settings.
- Commit `github-settings.json` before running `template-sync pull`.
- Run `template-sync pull` to merge the template's shared policy while preserving repository identity.

## New projects

| Language   | Repository                                              |
| ---------- | ------------------------------------------------------- |
| Rust       | [jruz/template-rs](https://github.com/jruz/template-rs) |
| TypeScript | [jruz/template-ts](https://github.com/jruz/template-ts) |

Rust:

```sh
gh repo create jruz/PROJECT --template jruz/template-rs --private --clone
```

TypeScript:

```sh
gh repo create jruz/PROJECT --template jruz/template-ts --private --clone
```

Enter the cloned project directory and run `template-sync init` to create `template-sync.json`.

- `Cargo.toml` selects `jruz/template-rs`.
- `package.json` selects `jruz/template-ts` when no `Cargo.toml` exists.
