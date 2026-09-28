# git

## 0.1.13

- Use the shared SDK Cargo metadata parser and unified package/archive generation.
- Adopt Cargo-generated plugin identity and SDK view constructors without changing the diff workflow.
- Compatible with Arc 0.7.0 and newer; ABI 2 and wire revision 14 are unchanged.
- macOS Apple Silicon: native package loading verified; background packages also passed GPU rendering checks.
- Windows x64: unsigned cross-build; architecture and native exports checked. Windows runtime verification was not performed.
- Source: `aa3f17dd736e5796f32a449438182aaeb86457df` in `zeyangl/arc`.

## 0.1.12

- Inline diff browsing: colored diffs with sticky file headers, paged file lists, compact metadata, and fixed footer controls.
- Ask the agent about a line, a diff block, or a selected range; the prompt carries the file, location, and surrounding diff.
- Requires Arc 0.7.0 or newer (plugin wire revision 14).
- macOS Apple Silicon: native tests (26) and packaged-library runtime validation passed.
- Windows x64: unsigned cross-build; no Windows runtime verification for this release.
- ABI 2; wire revision 14.
- Source: `05042ff74b87e5e2cb282204bf4ee72ecf90ebba` (`v0.7.0`) in `zeyangl/arc`.

## 0.1.3

- English and Simplified Chinese titles and descriptions; requires Arc 0.6.2 or newer.
- macOS Apple Silicon: native tests and runtime validation passed.
- Windows x64: unsigned cross-build; no Windows runtime verification for this release.
- ABI 2; wire revision 9.
- Source: `bc5f6a6273576ba41b58e3624bcfbd8d506a3e49` (`v0.6.2`) in `zeyangl/arc`.
