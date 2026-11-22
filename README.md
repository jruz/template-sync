# template-sync

Keep repositories in sync with their template through automerged pull requests.

- Templates declare synced files in `template.json`.
- Projects track their template revision in `template-sync.json`.
- Files are replaced, merged with file-specific rules, or flagged for manual review.

## Usage

Authenticate `gh` before syncing.

```sh
template-sync init
template-sync check
template-sync check --review
template-sync pull --dry-run
template-sync pull
```

See the [template skill](skills/template/SKILL.md) for sync rules and the full workflow.

## Template demos

- `just new-demo rust` creates a private demo at `tmp/template-sync-rust-demo/`.
- `just new-demo typescript` creates a private demo at `tmp/template-sync-ts-demo/`.
- `just new-delete jruz/template-sync-rust-demo` permanently removes the Rust demo.
- `just new-delete jruz/template-sync-ts-demo` permanently removes the TypeScript demo.

Deletion removes the Woodpecker record, GitHub repository, and local checkout.

## Licence

Licensed under the [EUPL 1.2](LICENSE).
