# Arc plugin SDK reference

This reference is published with the SDK and starters for one Arc build. Its
`reference.json` records the Arc version, native ABI revision, JSON wire revision,
and SDK SHA-256. PluginDoc supplies an immutable commit URL and verifies the
reference before caching it. Keep that URL when sharing or revisiting a project.

Read [GUIDE.md](GUIDE.md) first. Then choose the relevant chapters:

| Chapter | Contract |
| --- | --- |
| [ABI.md](ABI.md) | Native entry point, table layout, calling convention, ownership, statuses, lifecycle, failures |
| [WIRE.md](WIRE.md) | JSON serialization, every request/response type, defaults, examples |
| [UI.md](UI.md) | Every view node, layout, validation, interaction, selection, diff menus |
| [SERVICES.md](SERVICES.md) | Journal snapshots and pagination, provider catalogs and authenticated reads, scheduling and prompts |
| [BACKGROUND.md](BACKGROUND.md) | GPU callbacks, shader bindings, multipass rendering, window lifecycle |

The exact Rust field definitions ship in [sdk/src/lib.rs](sdk/src/lib.rs),
[sdk/src/providers.rs](sdk/src/providers.rs), [sdk/src/abi.rs](sdk/src/abi.rs), and
[sdk/src/background.rs](sdk/src/background.rs). These same files are available in
the local kit even before downloading documentation. Do not import Arc's core or
GUI crates into a plugin.

Working examples:

- [starter/src/lib.rs](starter/src/lib.rs): a pane with a counter and a clickable button.
- [background-starter/src/lib.rs](background-starter/src/lib.rs): an instance-owned shader and per-frame uniforms.
- [background-starter/src/background.wgsl](background-starter/src/background.wgsl): a full-screen WGSL effect.
- GUIDE.md includes an asynchronous host-request pattern; UI.md includes a diff action pattern.

Native ABI and wire revisions must both match exactly. A semantic plugin version
does not establish ABI compatibility. Newer documentation may describe features
that an older host rejects. Use the reference supplied by the running PluginDoc,
including for development builds.

There is no SaveState, Complete, explicit Cancel input, hot library reload, or
live plugin enable/disable protocol. Rust plugins may own their own filesystem
or subprocess work, but the SDK provides no process-execution host service.
