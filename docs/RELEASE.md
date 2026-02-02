# Recall Release Checklist & Versioning Policy

## Versioning Policy
- Use semantic versioning (MAJOR.MINOR.PATCH).
- MAJOR: breaking CLI/RQL changes, JSON schema version bumps, or on-disk format changes.
- MINOR: backward-compatible features, new flags, or new JSON fields.
- PATCH: bug fixes and internal improvements with no interface changes.
- On-disk schema changes must include:
  - Migration logic.
  - A migration test.
  - A note in `docs/history/changes/`.
- JSON schema version changes must be documented and migrated in tooling.

## Release Checklist
- [ ] Update `Cargo.toml` version.
- [ ] Run `./x fmt` and `./x clippy -- -D warnings`.
- [ ] Run `./x test` and review snapshot updates.
- [ ] Run core flows:
  - `recall init` → `recall add` → `recall search` → `recall context`.
- [ ] Verify migrations on an older store (schema version bump).
- [ ] Update `docs/history/changes/` for user-visible changes.
- [ ] Update `ROADMAP.md` milestones and issue status.
- [ ] Tag the release and publish notes.
