# provider-status

## 0.1.10

- Generate the native descriptor and package identity from Cargo metadata with the shared SDK build helper.
- Compatible with Arc 0.7.0 and newer; ABI 2, wire revision 14 is unchanged.
- macOS Apple Silicon: plugin tests and native host verification passed.
- Windows x64: unsigned cross-build; Windows runtime verification was not performed.
- Source: `4112888cef4af87e3e2b4c96eb19bbf6bef46865` (`v0.7.1`) in `zeyangl/arc`.

## 0.1.9

- Plan names, meter labels, and meter rows render in full now that per-field text caps are gone; the meter count follows the node budget.
- Requires Arc 0.7.0 or newer (plugin wire revision 14).
- macOS Apple Silicon: native tests (9) and packaged-library runtime validation passed.
- Windows x64: unsigned cross-build; no Windows runtime verification for this release.
- ABI 2; wire revision 14.
- Source: `05042ff74b87e5e2cb282204bf4ee72ecf90ebba` (`v0.7.0`) in `zeyangl/arc`.

## 0.1.3

- English and Simplified Chinese titles and descriptions; requires Arc 0.6.2 or newer.
- macOS Apple Silicon: native tests and runtime validation passed.
- Windows x64: unsigned cross-build; no Windows runtime verification for this release.
- ABI 2; wire revision 9.
- Source: `bc5f6a6273576ba41b58e3624bcfbd8d506a3e49` (`v0.6.2`) in `zeyangl/arc`.
