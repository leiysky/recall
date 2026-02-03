# CHANGE-2026-02-03-global-config-store-discovery

Milestone: M6
Summary:
- Removed per-directory `recall.toml`; config is now global in the OS config dir.
- Store discovery walks up for `recall.db` (or configured `store_path`).
- Lock files moved to the OS temp directory with best-effort cleanup.

User impact:
- Existing local `recall.toml` files are no longer supported; move settings to the global config.
- `recall init` now creates only the store file, not a local config.

Migration:
- Move settings to `<config_dir>/recall/recall.toml`.
- Remove local `recall.toml` files from repos or store roots.

References:
- (none)
