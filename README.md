# Arc plugins

Official packages tested for Arc 0.8.0. Current ABI 2, wire 14. Older packages remain available in the catalog.

| Plugin | Version |
| --- | --- |
| [blackhole](blackhole/) | 0.1.13 |
| [git](git/) | 0.1.14 |
| [history](history/) | 0.1.12 |
| [matrix](matrix/) | 0.1.11 |
| [provider-status](provider-status/) | 0.1.12 |

macOS Apple Silicon packages passed native tests.
Windows x64 packages are unsigned cross-builds, not run or tested on Windows. Install the [Visual C++ v14 x64 runtime](https://aka.ms/vc14/vc_redist.x64.exe) if needed.

Open **Plugins → Browse** in Arc. To update, remove the installed plugin and install the current version. Restart Arc to load changed libraries. Session `/restart` does not reload plugins.

For manual installation, verify the ZIP checksum, extract the manifest and library, and run `arc plugin install <folder>`. See [CATALOG.md](CATALOG.md).

## Author plugins

Arc embeds the full guide, SDK and standalone starters. Run `arc plugin doc` for the full guide. Use `arc plugin new my-plugin` for a project with GUIDE.md, then `arc plugin build --install`. [Source guide](https://github.com/zeyangl/arc/blob/v0.8.0/crates/plugin/authoring/GUIDE.md).
