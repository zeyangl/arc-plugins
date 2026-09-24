# Arc plugins

Official prebuilt plugins for Arc. The current versions require **Arc 0.7.0 or newer**;
the 0.1.3 packages stay published for Arc 0.6.2 and earlier. Source is maintained in
`zeyangl/arc`.

| Plugin | 中文 | Version |
| --- | --- | --- |
| [Git](git/) | Git | 0.1.12 |
| [History](history/) | 历史 | 0.1.9 |
| [Quota](provider-status/) | 配额 | 0.1.9 |
| [Matrix](matrix/) | 代码雨 | 0.1.8 |
| [Black Hole](blackhole/) | 黑洞 | 0.1.8 |

## Platforms

- macOS Apple Silicon (`aarch64-apple-darwin`): native tests and runtime validation passed.
- Windows x64 (`x86_64-pc-windows-msvc`): unsigned cross-builds, not run or tested on Windows.
  Install the [Microsoft Visual C++ v14 Redistributable (x64)](https://aka.ms/vc14/vc_redist.x64.exe) if needed.

Current packages use plugin ABI 2 and wire revision 14, so they load only in Arc 0.7.0
or newer. The 0.1.3 packages use wire revision 9 and remain available for Arc 0.6.2 and
earlier. Windows runtime verification was explicitly omitted for this release. Archive
integrity and SHA-256 checks were performed for both platforms.

## Install

Upgrade Arc first. Open **Plugins → Browse**, refresh the catalog, and install.
To update an already installed plugin, remove it and install its current version.
Restart Arc to load changed libraries; session `/restart` does not reload plugins.
Select Matrix or Black Hole in Settings → Background.

For manual installation, download the ZIP and matching `.sha256` file for your
platform, verify the checksum, extract the two files, then run:

```sh
arc plugin install /path/to/extracted/package
```

The ZIP contains `plugin.toml` and one native library. The catalog pins immutable
Git commits. Earlier retired packages remain in Git history; installed copies
are unaffected by catalog changes.

See [CATALOG.md](CATALOG.md) for the format and publication procedure.


## Author plugins

Arc agents should call `PluginDoc` (or users can run `/plugindoc`) in Arc.
It supplies core instructions, a bundled SDK and standalone starters matching the
running binary, and the full guide's commit-pinned URL and verified local cache.

Read the [complete authoring guide](authoring/GUIDE.md) for the native ABI, JSON
wire format, UI nodes, host services, background rendering, and examples. Create
a project with `arc plugin new my-plugin`, then use `arc plugin build --install`.
No Arc development checkout is needed to author plugins.

The guide is maintained in `zeyangl/arc/crates/plugin/authoring/GUIDE.md` and
published with the native `arc plugin docs prepare`, `pin`, and `check` commands.
Each Arc build pins a publication commit and verifies the guide's SHA-256. SDK
sources remain bundled in Arc; the guide cache contains only Markdown. Use the
reference returned by your Arc build when targeting that version.
