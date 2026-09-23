# Public plugin catalog

The public `zeyangl/arc-plugins` repository groups ZIP packages by plugin.
Versions and targets live in filenames; plugin source stays in the main Arc repo.

```text
arc-plugins/
├── README.md
├── catalog.toml
├── git/
│   ├── CHANGELOG.md
│   ├── git-0.1.0-aarch64-apple-darwin.zip
│   ├── git-0.1.0-aarch64-apple-darwin.sha256
│   ├── git-0.1.1-aarch64-apple-darwin.zip
│   ├── git-0.1.1-aarch64-apple-darwin.sha256
│   ├── git-0.1.1-x86_64-pc-windows-msvc.zip
│   └── git-0.1.1-x86_64-pc-windows-msvc.sha256
├── history/
├── provider-status/
├── matrix/
└── blackhole/
```

Commit the ZIPs and checksum files directly in these directories. Only include
verified targets, keep older versions, and never overwrite a published package.
Commit the packages first, then use that commit's full SHA in the catalog URLs.
Commit the catalog update and push both commits together. GitHub Release assets
are not needed for this layout.

Arc reads `main/catalog.toml` when the Plugins tab opens or Refresh is pressed.
An empty catalog is `schema = 1`. Each `[[plugins]]` entry describes one plugin
version for one Rust target. Arc selects the newest stable release matching its
target, ABI revision, and wire revision. Pre-release versions are omitted.

```toml
schema = 1

[[plugins]]
id = "git"
name = "Git"
description = "View your workspace's branch, changed files, and diffs."
version = "0.1.1"
target = "aarch64-apple-darwin"
abi_revision = 2
wire_revision = 9
url = "https://raw.githubusercontent.com/zeyangl/arc-plugins/<40-character-package-commit>/git/git-0.1.1-aarch64-apple-darwin.zip"
sha256 = "REPLACE_WITH_THE_ARCHIVE_SHA256"
```

Replace the URL placeholder with the full commit SHA containing the package.
Arc requires the path `<id>/<id>-<version>-<target>.zip` at that
commit; branch-based download URLs are rejected. The top-level catalog follows
`main`, while each package URL pins immutable bytes.

Use the actual version, target, and SDK revisions from the build. `id`, `name`,
and `version` must match the package manifest; the English name must match the
compiled descriptor. `sha256` is the ZIP's 64-character hexadecimal SHA-256 digest.
Copy `description` from
the built `plugin.toml`; its source is `package.metadata.arc-plugin.description`
in the plugin's Cargo.toml.

Names and descriptions can be plain strings or localized tables:

```toml
name = { en = "History", zh-Hans = "历史" }
description = { en = "View workspace changes.", zh-Hans = "查看工作区更改。" }
```

The same format works in Cargo metadata, `plugin.toml`, and the catalog. Copy both
fields from the package into its catalog entry. Arc's language setting selects
the text immediately in Installed, Browse, pane titles, the rail, and the
background picker; search matches either language. `en` is required; omitted
`zh-Hans` falls back to English.
Localized values must be nonempty, with at most 128 UTF-8 bytes per name and
4096 per description in each language. IDs and compiled English names stay stable.
Plain strings remain supported. Localized tables require an updated Arc host;
older hosts only accept strings, including in catalogs.

Archives contain exactly two regular files at the root: `plugin.toml` and the
library named in it. Use ZIP's stored or deflate compression. Arc checks the
checksum, manifest identity, and compatibility before installing. Catalogs are
limited to 1 MiB, ZIP downloads to 64 MiB, and expanded libraries to 256 MiB.

Installation preserves existing enablement and workspace scope. Removal drops
the registration immediately; managed package files are reclaimed on a later
application launch, when they are no longer loaded. Development directories
are never deleted. A running session's `/restart` does not reload libraries.

Verify the published catalog with Arc's downloader, installer, and native loader:

```sh
cargo test --release -p arc-gui --lib plugins::registry::tests::published_packages_install_and_load -- --ignored --exact
```

This downloads compatible packages into a temporary installation and checks their
native descriptors without changing the user's installed plugins.
