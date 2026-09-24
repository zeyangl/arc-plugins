# Arc plugin authoring

Call PluginDoc before creating or updating a plugin. It supplies the matching
local SDK/starters and a pinned, verified documentation snapshot. Start with
[INDEX.md](INDEX.md); the reference is complete without the Arc development repo.
Keep the snapshot supplied by your running build, including on development builds.

## Start a project

Use Rust/Cargo and a matching Arc CLI. Copy PluginDoc's kit before editing:

```sh
kit='/absolute/kit/path'
cp -R "$kit/starter" my-plugin
cp -R "$kit/sdk" my-plugin/sdk
cd my-plugin
arc plugin build
```

Commit `sdk/`, source, and Cargo.lock; no Arc checkout is needed.

Cargo.toml identity defaults to `package.name` for the plugin ID and
`package.description` for the display name. Override these with
`package.metadata.arc-plugin.id` and `.name` when the Cargo package name differs
from the public plugin identity. The ID is 1–64 bytes of lowercase letters,
digits, and nonempty components separated by single hyphens. The English display
name is nonempty and at most 128 bytes. `package.version` supplies the version. Keep `crate-type = ["cdylib"]`, the local SDK dependency,
and `[workspace]`. Optional `package.metadata.arc-plugin.{name,description}` holds a
string or `{ en = "Text", zh-Hans = "文本" }`, follows Arc's language with English
fallback, and is searched by both; `Plugin::NAME` must equal the English name.

## Minimal implementation

Implement `arc_plugin::Plugin`; `arc_plugin::export_plugin!(MyPlugin)` owns the C ABI
shim. Rust objects never cross FFI.

- `create(Context)` starts a session instance with `binding`, `session_id`, `workspace`.
- `update(Input)` handles events and changes local state.
- `view()` returns a host-rendered `View { nodes }`. Never block in these methods.
- Optional `take_request()` returns one `Request { id, query }`, once per request.
- Optional `take_prompt()` queues one user prompt per update, never a command; ≤64 KiB.
- Optional `badge()` returns the rail summary; `wake_after_ms()` schedules a wake.

Inputs: Click, Edit, DiffAction, Visibility, JournalChanged, ProvidersChanged, Wake,
Response; ignore unused ones. Click targets the current Button, ActionRow, IconButton, or
TimelineRow ID; Edit targets a TextInput ID.

## Views and limits

Nodes: Text, Button, ActionRow, IconButton, TextInput, Row, Column, Scroll, Markdown,
Code, Card, Meter, Grid, Timeline, StickySection, Diff. Text takes a semantic Tone:
Normal, Muted, Accent, Success, Warning, or Error. ActionRow is a clickable row of 1–16
TextSpan parts, each text plus Tone. StickySection needs at least one child: the first is
its header, pinned to the top of the enclosing scroll viewport while the rest scroll, with
header controls still interactive. Markdown fences tagged `diff` or `patch` use the host's
themed addition, deletion, and hunk colors. The host owns styling, layout, selection, and
widget state, so escape Markdown data and keep node IDs stable, unique across the view, and
≤64 bytes of ASCII letters, digits, `_`, or `-`. [UI.md](UI.md) specifies every node, its fields, validation, and layout behavior.
[WIRE.md](WIRE.md) gives serialization rules and message examples.

Diff renders unified diff text with the same colors and per-line context menus. Page at
MAX_DIFF_LINES (512); diff lines also count toward MAX_NODES. Declare up to 16
DiffActions, each with a local ID, label, and scope (Line, Block, or Selection).
`Input::DiffAction` returns the node and action IDs, a zero-based line index within the
displayed text, and, for Selection, the selected text with inclusive first/last line
indexes; only selections wholly inside that diff are sent. Set `in_hunk` when a page
starts inside a hunk from the previous page. Give changed snapshots new node IDs and
reject stale IDs; an unchanged snapshot keeps its ID.

Limits: 4096 nodes, depth 32, 4 MiB per encoded wire message, no per-field text limit, and
scroll portions 1–16. Page large collections, since scrolling does not virtualize nodes.
`View::validate()` and `Update::validate()` check structural bounds, while the ABI enforces
the encoded message budget, including JSON escaping and metadata. Plugins own timing and
paging semantics and need neither arc-core nor iced.

Rail badges allow three single-line parts of ≤64 bytes each. Wake delays are 16–60000 ms,
one-shot, and scheduled at their deadline: return the next delay from an event, or None to
disarm. Use short wakes only while awaiting background work and keep idle polling
infrequent; wakes also run while hidden. Use Visibility to avoid work.

## Host reads

Allow one outstanding request per instance: increase IDs, clear requests in
`take_request()`, and wait for the matching `Input::Response { id, result }` before issuing
another. Read errors are recoverable; returning Err from a plugin method retires the
worker until the application restarts. [SERVICES.md](SERVICES.md) specifies
request lifetimes, journal snapshots, provider restrictions, and scheduling.

- `ReadJournal { journal, cursor, refresh }` returns a JournalPage: start at
  `JournalRef::default()` with no cursor and page with the returned cursor. JournalChanged
  is a coalesced notice, so opening a pane should refresh even without one.
- `ReadRecord { record }` returns the JournalRecord at a returned RecordRef; a record's
  optional `child` JournalRef opens a recorded child. References are opaque, never
  filesystem paths, and belong to the session binding.
- `ReadProviders` returns a Providers catalog and the selected model. ProviderInfo has a
  label, a sanitized endpoint, and a credential-presence flag, never API keys.
- `ProviderGet { provider, path, auth }` returns ProviderJson from a ProviderRef and an
  absolute resource path with ProviderAuth Bearer or Raw; plugins own endpoint paths and
  parsing. Configuration changes revoke references and cancel their reads.

Session replacement creates a fresh instance and cancels old reads, so discard old
references and results. Calls have a 5-second deadline: put slow work in a worker, poll
results on Wake, and cancel it on drop without blocking teardown.

## Build, install, and load

`arc plugin build [project]` writes `dist/plugin.toml` and the native library from Cargo's
output; `--target <triple>` sets the target and `--locked` requires Cargo.lock. Manifest
ID, English name, version, pane capability, and background FPS must match the
compiled descriptor. The library must name one native file inside the package. Arc replaces files atomically, so never overwrite a loaded library's inode.

Increment the plugin package version before distributing a changed build and
update Cargo.lock. Regenerate the package; never pair a new manifest version with
an old library. Build native packages separately for each target. The target must
match the running Arc architecture as well as its ABI/wire revisions.

When the user requests installation, run in the plugin project:

```sh
arc plugin install dist
```

That installs to `<arc-state>/plugins/<id>` and registers it in `<arc-state>/plugins.toml`,
preserving enablement and scope. State lives in Documents/arc on Windows, including
redirected folders, or `~/.config/arc` on Unix and macOS. Remove a development registration
for the same ID first. Restart the application after updates; session `/restart` does not
reload libraries, and Windows needs Arc quit before updating a loaded plugin.

A development registration points at an absolute `dist` path, with an optional absolute
`workspace` path restricting enablement. Arc never auto-enables plugins found in a
repository, because these trusted native libraries run with Arc's privileges.

## Background renderers

Use `background-starter/`; **BACKGROUND.md** covers its GPU interface and lifecycle.
Background plugins share the SDK, loader, and installer.

## Not implemented

SaveState, Complete, explicit Cancel, hot loading/unloading, and live enable/disable are
unavailable. Use the SDK's actual variants.


## Host-request example

The pane below reads the provider catalog once on creation and again after a
notification. It coalesces notifications while a read is outstanding and keeps
service errors in UI state. This code needs only the local arc-plugin SDK.

```rust
use arc_plugin::{Context, Input, Node, Plugin, Query, Request, Response, Tone, View};

struct CatalogPane {
    next_id: u64,
    pending: Option<u64>,
    dirty: bool,
    status: String,
}

impl Plugin for CatalogPane {
    const ID: &'static str = env!("CARGO_PKG_NAME");
    const NAME: &'static str = env!("CARGO_PKG_DESCRIPTION");
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    fn create(_: Context) -> Result<Self, String> {
        Ok(Self { next_id: 0, pending: None, dirty: true, status: "Loading…".into() })
    }
    fn update(&mut self, input: Input) -> Result<(), String> {
        match input {
            Input::ProvidersChanged => self.dirty = true,
            Input::Response { id, result } if self.pending == Some(id) => {
                self.pending = None;
                self.status = match result {
                    Ok(Response::Providers(catalog)) => format!("{} providers", catalog.entries.len()),
                    Ok(_) => "Unexpected response".into(),
                    Err(error) => error,
                };
            }
            _ => {}
        }
        Ok(())
    }
    fn take_request(&mut self) -> Option<Request> {
        if self.pending.is_some() || !self.dirty { return None; }
        self.dirty = false;
        self.next_id += 1;
        self.pending = Some(self.next_id);
        Some(Request { id: self.next_id, query: Query::ReadProviders })
    }
    fn view(&self) -> View {
        View { nodes: vec![Node::Text {
            id: "catalog".into(), text: self.status.clone(), tone: Tone::Normal,
        }] }
    }
}
arc_plugin::export_plugin!(CatalogPane);
```

## Debugging and compatibility

- Build failure: use the complete SDK directory produced by PluginDoc, retain
  its own Cargo.toml, and keep `[workspace]` in the independent plugin project.
- Incompatible plugin: compare target, ABI revision, wire revision, and native
  table size. Copy the current SDK and rebuild. Editing version strings cannot
  repair binary incompatibility.
- Descriptor mismatch: align Plugin constants, Cargo metadata, and the generated
  manifest. Localized English name is the compiled name. Rebuild the package.
- Invalid view/output: call View::validate and Update::validate in plugin tests.
  Account for JSON escaping and host-generated metadata in the wire byte budget.
- Worker failure/timeout: keep native calls short, report recoverable errors in
  UI state, bound background work, and restart Arc after correcting the plugin.
- Missing refresh: visibility, service notifications, and wakes are separate.
  An idle None wake does not periodically call your update method.
- Old code after installation: restart the application. Native libraries are
  retained for the process lifetime; replacing files does not reload them.

Use [ABI.md](ABI.md) for ownership and failure semantics when debugging native
integration. A native plugin is not sandboxed, and an abort or memory violation
can terminate Arc. Rust's export shim contains unwinding panics but does not
provide process isolation. Never retain host-borrowed pointers or GPU callbacks.
