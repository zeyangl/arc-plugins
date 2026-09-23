# Arc plugins

Official prebuilt plugins for Arc. Source is maintained in `zeyangl/arc`.

No plugin packages are currently listed. The previous `0.1.1` releases have
been retired, and the catalog is empty until new packages are published.
Existing local installations are unaffected. Retired files remain in Git history.

## Install

When packages are available, open **Plugins → Browse** in Arc, refresh the
catalog, and choose **Install**. Restart Arc after installation or removal;
session `/restart` does not reload native plugins.

## Repository layout

- `catalog.toml`: Arc's discovery index, read from `main`.
- `<id>/<id>-<version>-<target>.zip`: the plugin package.
- `<id>/<id>-<version>-<target>.sha256`: its SHA-256 checksum.
- `<id>/CHANGELOG.md`: release notes and source revisions.

Package URLs pin a full Git commit SHA. New builds receive new versions.
See [CATALOG.md](CATALOG.md) for the schema and publication procedure.
