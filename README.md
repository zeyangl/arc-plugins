# Arc plugins

Official prebuilt plugins for Arc. Packages are grouped by plugin, with version
and target in each filename. Source is maintained in `zeyangl/arc`.

| Plugin | Version | Description |
| --- | --- | --- |
| [Git](git/) | 0.1.1 | View your workspace's branch, changed files, and diffs. |
| [History](history/) | 0.1.1 | Browse session history, inspect tool calls, and follow child sessions. |
| [Quota](provider-status/) | 0.1.1 | Monitor quota, balances, and reset times for supported model providers. |
| [Matrix](matrix/) | 0.1.1 | Display animated code rain behind your workspace. |
| [Black Hole](blackhole/) | 0.1.1 | Display an animated black hole with a glowing disk and lensed stars. |

## Install

Open **Plugins** in a current Arc build, click **Refresh**, and choose **Install**.
Restart the application after installation or removal. Session `/restart` does
not reload native plugins. Select Matrix or Black Hole in Settings → background.

For manual installation, download a ZIP and its matching `.sha256` file, verify
the checksum, extract the ZIP, and run:

```sh
shasum -a 256 -c git-0.1.1-aarch64-apple-darwin.sha256
arc plugin install /path/to/extracted/package
```

The ZIP contains `plugin.toml` and one native library. Install plugins you trust;
they run with Arc's access to your computer. Removal affects local installation,
while removing a remote catalog entry leaves existing installations intact.

## Compatibility

This initial release supports **Apple Silicon macOS** (`aarch64-apple-darwin`),
plugin ABI **2**, and wire revision **9**. Windows and other targets
will be added after runtime verification.

## Repository layout

- `catalog.toml`: Arc's discovery index, read from `main`.
- `<id>/<id>-<version>-<target>.zip`: the plugin package.
- `<id>/<id>-<version>-<target>.sha256`: its SHA-256 checksum.
- `<id>/CHANGELOG.md`: release notes and source revisions.

Package URLs in the catalog point to full Git commit SHAs. Published package
files stay unchanged; new builds receive new versions. Older packages and
catalog entries remain available for compatible Arc builds.

See [CATALOG.md](CATALOG.md) for the schema and publication procedure.
