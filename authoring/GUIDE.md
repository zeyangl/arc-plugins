# Arc plugin authoring reference

Call PluginDoc before creating or updating a plugin. It supplies the matching
local SDK and starters, core instructions, and this complete guide. No Arc
development checkout is needed. Use the pinned reference supplied by your running
build, including for development builds; do not substitute the newest guide.

PluginDoc downloads this Markdown file from a commit-pinned URL, verifies its
SHA-256, and caches it for offline use. SDK sources and starters remain bundled
in Arc even before the first fetch. The tool output identifies the ABI/wire revisions.

- [Start a project](#start-a-project)
- [Plugin implementation](#minimal-implementation)
- [Build and install](#build-install-and-load)
- [Host-request example](#host-request-example)
- [Views and interaction](#views-and-interaction)
- [Host services and scheduling](#host-services-and-scheduling)
- [Background renderers](#background-renderers)
- [Native ABI](#native-abi)
- [JSON wire format](#json-wire-format)
- [Debugging and compatibility](#debugging-and-compatibility)

The exact Rust definitions are in `sdk/src/lib.rs`,
`sdk/src/providers.rs`, `sdk/src/abi.rs`, and
`sdk/src/background.rs`. Working examples are
`starter/src/lib.rs` and
`background-starter/src/lib.rs`, with its
`background-starter/src/background.wgsl`. These same resources are
available in the local kit. Do not import Arc's core or GUI crates into a plugin.

## Start a project

Use Rust/Cargo and a matching Arc CLI:

```sh
arc plugin new my-plugin
cd my-plugin
arc plugin build
```

The command copies the bundled SDK and starter into the new directory. Use
`arc plugin new my-background --background` for a renderer. Existing directories
are never overwritten. Commit `sdk/`, `build.rs`, source, and Cargo.lock; no Arc
checkout is needed.

Cargo.toml identity defaults to `package.name` for the plugin ID and
`package.description` for the display name. Override these with
`package.metadata.arc-plugin.id` and `.name` when the Cargo package name differs
from the public plugin identity. The ID is 1–64 bytes of lowercase letters,
digits, and nonempty components separated by single hyphens. The English display
name is nonempty and at most 128 bytes. `package.version` supplies the version.
Keep `crate-type = ["cdylib"]`, the local SDK dependency,
and `[workspace]`. Optional `package.metadata.arc-plugin.{name,description}` holds a
string or `{ en = "Text", zh-Hans = "文本" }`, follows Arc's language with English
fallback, and is searched by both. The starter's `build.rs` calls
`arc_plugin::build::configure()` using an SDK build-dependency with feature
`build`. Inside `impl Plugin`, `arc_plugin::plugin_metadata!()` supplies ID, English
name, version, pane capability, and background FPS from Cargo metadata. Change
these in Cargo.toml; do not repeat them in Rust. The host still checks the compiled
descriptor against the generated manifest.

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

## Build, install, and load

`arc plugin build [project]` writes `dist/plugin.toml` and the native library from Cargo's
output; `--target <triple>` sets the target and `--locked` requires Cargo.lock.
`--install` also installs the package locally; cross-target installation is rejected.
The generated `dist/catalog-entry.toml` supplies catalog identity and compatibility
fields. ABI/wire revisions come from the SDK build helper used by that plugin.
Older SDKs without this helper leave those fields for the release author to fill. Add the committed archive URL and SHA-256 when publishing; keep this
metadata file outside the two-file release ZIP. Manifest
ID, English name, version, pane capability, and background FPS must match the
compiled descriptor. The library must name one native file inside the package.
Arc replaces files atomically, so never overwrite a loaded library's inode.

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
    arc_plugin::plugin_metadata!();

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

## Views and interaction

The SDK offers `Node::text(id, text, tone)`, `Node::button(id, label)`,
`Node::row(id, children)`, `Node::column(id, children)`, and
`DiffAction::new(id, label, scope)` for common controls. These produce the same
wire nodes as enum literals; use literals when setting other fields such as
button selection. IDs remain explicit and stable.


Return a complete `View { nodes }` after each update. Arc renders these declarative
nodes using its fonts, theme, selection system, layout, and widget state. Plugins
do not link iced or submit arbitrary GUI widgets. Identity is the node's kind and
ID inside the plugin/session binding; retain IDs for the same logical control.
Changing an ID intentionally resets that control's widget state.

### Node reference

Every variant has `id: String`; the table lists the remaining fields. Wire kinds
are snake_case. `children` is always `Vec<Node>`; empty containers are allowed
except StickySection. See [JSON wire format](#json-wire-format) for defaults.

| Node | Fields | Behavior |
| --- | --- | --- |
| Text | `text: String, tone: Tone` | Selectable body text |
| Button | `label: String, selected: bool` | Text button; emits Click |
| ActionRow | `spans: Vec<TextSpan>, selected: bool` | Full-width clickable row, each span independently colored; emits Click |
| IconButton | `icon: String, hint: String, selected: bool` | Compact glyph button with tooltip; emits Click |
| TextInput | `label: String, value: String` | Editable field; emits Edit with the entire value |
| Row | `children` | Horizontal layout |
| Column | `children` | Vertical layout |
| Grid | `children` | Equal-width columns wrapping to available width |
| Scroll | `children, portion: u16` | Vertical scrolling; proportional allocation among fill regions |
| StickySection | `children` | First child stays at viewport top until the section bottom pushes it out |
| Card | `title: String, subtitle: String, badge: Option<Badge>, children` | Group surface; an empty heading and no badge gives a compact faint surface |
| Meter | `label: String, value: String, detail: String, progress: Option<f32>, tone: Tone` | Labeled value with optional 0–100 progress bar |
| Timeline | `rows: Vec<TimelineRow>, follow: bool, portion: u16` | Scrollable event timeline; follow snaps to newest events after updates |
| Markdown | `text: String` | Selectable Markdown; diff/patch fences get semantic diff colors |
| Code | `text: String` | Selectable monospaced text |
| Diff | `text: String, in_hunk: bool, actions: Vec<DiffAction>` | Selectable unified diff with per-line context actions |

TextSpan has text and tone. Badge has text and tone. Tone maps to the user's theme,
so retain semantic roles rather than depending on fixed RGB values.

TimelineRow has `id, stamp, label, summary: String`, `tone: Tone`,
`duration_secs, gap_secs: Option<u64>`, and `selected: bool`. Duration represents
measured work; gaps represent elapsed time between rows. Gaps of at least ten
seconds have visible extent. Clicking a row emits Click for its row ID. The
plugin owns timing semantics, pagination, and selection state.

If a pane contains an explicit Scroll or Timeline, the host respects that layout.
Otherwise it supplies an outer scrollable pane. Put footer controls outside an
explicit Scroll to keep them fixed. Scrolling does not virtualize arbitrary
nodes: page large data sets. Row spacing, margins, scrollbar clearance, theme
colors, and font sizes are host-owned and can evolve without a wire change.

Selectable text shares a selection region within the pane. Copy and Select All
are host operations. Button labels and ActionRow spans are click targets. A
TextInput's value in the next view should reflect the edit; the host preserves
newer in-flight drafts over older returned views. There is no generic right-click
event or arbitrary per-node menu API; Diff is the scoped context-menu surface.

### Limits

- All node and timeline-row IDs share one namespace and resource budget. IDs are
  1–64 bytes of ASCII letters, digits, `_`, or `-`. They must be unique in a view.
- MAX_NODES is 4,096; each diff line counts in addition to its enclosing node.
  Timeline rows also count. The root nodes have depth 1; maximum depth is 32.
- A serialized Update must fit in 4 MiB, including JSON escaping and effects.
  There is no separate text-field cap. Views may need smaller pages for long paths.
- Scroll and Timeline portions are 1–16. StickySection requires at least a header.
- ActionRow has 1–16 spans. IconButton.icon must not be empty.
- Card badge text is at most 64 bytes without CR/LF. Meter progress is finite
  and between 0 and 100 inclusive when present.
- Each Diff has at most 512 displayed lines and at most 16 actions. Action IDs
  use the node-ID grammar and must be unique within that Diff's menu; they do not
  share the view's ID namespace. Action labels must not be empty.

### Diff actions

Diff text is raw unified diff, without a Markdown fence. Addition/deletion/hunk
prefixes control coloring. `in_hunk` says that this page starts inside a hunk
whose header was on a previous page; it does not identify the original source
line numbers or provide missing context. The plugin must retain its own snapshot.

Declare actions using IDs meaningful to the plugin:

```rust
use arc_plugin::{DiffAction, DiffScope, Node};
let node = Node::Diff {
    id: "diff_file7_revision3_page0".into(),
    text: "@@ -1 +1 @@\n-old\n+new\n".into(),
    in_hunk: false,
    actions: vec![
        DiffAction { id: "line".into(), label: "Explain line".into(), scope: DiffScope::Line },
        DiffAction { id: "block".into(), label: "Explain diff block".into(), scope: DiffScope::Block },
        DiffAction { id: "selection".into(), label: "Explain selection".into(), scope: DiffScope::Selection },
    ],
};
```

The host offers Line actions on added, removed, and context lines inside a hunk;
Block actions on a hunk header and subsequent lines; Selection actions when a
nonempty selection lies entirely in this Diff and includes the clicked line.
Normal Copy/Select All are also available. Soft wrapping retains the original
logical line target. The host sends
`Input::DiffAction { id, action, line, selection }`. `line` and the selection's
inclusive first/last indexes are relative to the displayed text. Only Selection
actions carry DiffSelection; its text may begin/end partway through a line.

A Block action supplies the clicked line, not a parsed hunk object. Resolve the
hunk against the snapshot you rendered, including context on other pages. Give
each changed snapshot/page a new node ID, retain IDs for unchanged refreshes,
and reject stale actions. Never resolve an old click against freshly read file
contents. For explanations, queue a prompt containing the path, original line
numbers, +/- markers, and the relevant surrounding hunk.

When a very long source line must span pages, preserve its diff marker on each
continuation page and keep a mapping back to the original logical line. Source
text can contain Markdown fences: raw Diff avoids that escaping problem. For
Markdown nodes, choose fences that cannot be closed by the source text.

## Host services and scheduling

### Request lifecycle

`take_request()` yields at most one new Request after an update, including the
initial create. Consume it with Option::take or equivalent; returning it again
resubmits work. Use increasing request IDs and wait for the matching
`Input::Response { id, result }` before issuing another request. Issuing a second
request before the first response is processed fails the instance. A response
handler may immediately submit its next request. Service errors arrive as Err
inside Response and are recoverable; update your UI and retry deliberately.

The host runs service work asynchronously and returns serial Inputs. User edits,
visibility changes, notifications, and wakes may arrive while a service is
outstanding. IDs and current plugin state must determine whether results are
still relevant. Session replacement cancels reads and rejects old replies.
There is no explicit cancel Query; discard superseded results in plugin state.

### Journal reads

The initial root reference is `JournalRef::default()` (the empty string).
`ReadJournal { journal, cursor: None, refresh: false }` starts from byte zero and
captures the current journal end. Read with the returned cursor while `more` is
true, keeping `refresh: false` to traverse that captured snapshot.

`refresh: true` retains the cursor's offset and captures a new end. Use it after
reaching the previous end to read appended records. Cursor binding and journal
must match the request. If the journal shrank, changed beneath the cursor, or
the cursor is not at a record boundary, the host returns an error; start a new
read. A partial final line is not committed and does not advance the cursor.
Retry it with a refreshed end after more data arrives.

Each page has at most 32 records and a 3 MiB budget for encoded records. A raw
record over 2 MiB is represented by an explicit `{"type":"oversized", ...}`
entry, not silently dropped. An encoded record/reference exceeding the page
budget is also replaced by an oversized entry. The 4 MiB wire budget still
applies to the complete response. Invalid JSON or a record without a string
`type` returns an error. Unknown record types are preserved as JSON: consumers
must tolerate new journal events.

`ReadRecord { record }` reads exactly one returned RecordRef. Its range must
remain complete and belong to this binding. Use `JournalRecord.child`, when
present, to traverse a recorded child journal. References are opaque and scoped
to the actual attached session bundle, which may live outside the default state
directory. Do not guess paths from session IDs or interpret a reference as a
workspace-relative filesystem path. Symlink/path traversal outside the attached
bundle is rejected.

JournalChanged is coalesced and delivered for visible panes. On Visibility(true),
refresh even if no notification was received. A plugin may pause paging when
hidden and resume from its retained cursor when visible.

### Provider catalog

ReadProviders returns a credential-free catalog, its revision, and an optional
selected provider/model. Each entry has a ProviderRef, a name, a sanitized
endpoint URL, and `configured` (whether a credential is present). Credentials
are never included in the catalog. URL userinfo, query, and fragment are removed.

ProviderRef contains a catalog revision and index. Endpoint or credential changes
revoke old references and cancel reads using them. Selection-only changes retain
references but still notify plugins through ProvidersChanged. Notifications are
coalesced and may arrive while hidden. Re-read the catalog after a notification
or stale-reference error. Never infer a key or a provider identity from an index
in a different revision. The encoded catalog is bounded to 128 KiB.

### Authenticated provider GET

ProviderGet takes a current ProviderRef, `path`, and `auth`:

- Path is at most 2,048 bytes, starts with `/`, must not start with `//`, and
  contains no fragment (`#`). A query string is allowed.
- The path is resolved at the configured HTTP(S) origin, not relative to a
  `/v1` suffix. It cannot change origin or introduce URL credentials.
- Bearer uses `Authorization: Bearer <key>`; Raw sends the key as the header
  value without a prefix. The key is captured when the read is admitted.
- The request uses GET and `Accept: application/json`. No custom headers,
  request body, credential-return API, or redirect following are provided.
- Connection timeout is five seconds; the complete request timeout is ten
  seconds. These service deadlines are distinct from native callback deadlines.
- Only 2xx responses succeed. Error bodies are not forwarded. Successful bodies
  must be JSON and at most 128 KiB. The encoded ProviderJson response must also
  fit 128 KiB because some JSON values grow when re-encoded.

Plugins own endpoint paths and response parsing. Unsupported providers should
show an explicit unavailable state. Cache useful previous data during transient
failures while marking it stale; do not turn a failed read into zero usage.

### Wakes, visibility, and prompts

Every returned Update replaces the one-shot wake deadline. `Some(ms)` accepts
16–60,000 milliseconds inclusive, measured from admission of that Update; None
disarms it. The host schedules the earliest pending deadline. A wake waits for
an in-flight native reply and does not accumulate another Wake while that update
runs. Wakes run even when hidden. They are scheduling requests, not real-time
guarantees. Use short delays only while collecting your own background work,
then return to infrequent idle polling. Visibility describes UI presence, not
instance destruction.

`take_prompt()` consumes an optional nonempty UTF-8 prompt of at most 64 KiB.
It enters the attached Session as ordinary user text and is never parsed as an
Arc slash/shell command. Include enough context for the Session to understand
the requested operation; do not rely on the model seeing the plugin UI.
The initial create can also return a prompt. Background instances cannot issue
requests or prompts. A pane's rail badge is independent of whether it is visible:
return at most three text/tone parts, each at most 64 bytes and without CR/LF.

Use your own bounded worker for filesystem, network, or subprocess work not
covered by these services. The host does not execute a process on a plugin's
behalf and cannot cancel plugin-owned work. Arrange cancellation and teardown
without waiting indefinitely for a filesystem or child process.

## Background renderers

A native plugin may provide a pane, a background, or both. Start a background by
copying `background-starter/` instead of `starter/`, then copy `sdk/` alongside
its Cargo.toml. The same `arc plugin build` and `arc plugin install dist` commands
package and register it.

Set `Plugin::PANE = false` and `Plugin::BACKGROUND_FPS = Some(15)` for a
background-only plugin. Match these with `pane = false` and `background_fps = 15`
in `[package.metadata.arc-plugin]`; the builder includes them in plugin.toml.
The host accepts 1–60 FPS. Pane defaults to true; background defaults to absent.
The ID `default` is reserved for the flat background.

Implement `render_background(frame, gpu)`. Create each shader once with
`gpu.create_shader(wgsl)` and retain its opaque Shader handle. Each call records
one to eight ordered `gpu.draw(shader, uniform_bytes)` calls. The last draw is
the visible background. Shader handles belong to that plugin instance and
cannot be transferred between instances.

The GPU interface supports a full-screen triangle with WGSL `vertex` and
`fragment` entry points, no vertex buffers, and one uniform struct at group 0,
binding 0. The struct and each upload must be multiples of 16 bytes, at most
1 KiB. Shader source is limited to 64 KiB and an instance may create eight shaders.
The last draw writes opaque sRGB-encoded colors; Arc handles output color conversion. The frame
contains logical dimensions, texture dimensions in physical pixels, and active
animation time in seconds. Asset textures and feedback between frames are not supported.

Earlier draws in the same frame are available as `texture_2d<f32>` at group 1,
bindings 0–7 (binding 0 is the first draw). Declare only the inputs you need;
every input must refer to an earlier draw. A linear, trilinear, clamp-to-edge
sampler is available at group 1, binding 8. Intermediate targets use RGBA16Float
and a full mip chain, rebuilt after each draw. They carry linear HDR color and
an unrestricted alpha channel for depth or other data. Texture coordinates have
their origin at the top left. All passes use the frame's physical dimensions.
The host reuses targets and bindings until the dimensions or shader sequence change.
The native ABI remains compatible with single-pass plugins; multipass plugins
require a host with this extension (older hosts reject the additional bindings).

For expensive effects, set `background_max_pixels` in `[package.metadata.arc-plugin]`.
It is a positive pixel budget for the entire background texture, independent of
window scale. Arc reduces physical dimensions proportionally and stretches the
cached result to the window. Logical dimensions remain unchanged. Omitting it
keeps native resolution. This manifest hint requires `background_fps`; it has no
corresponding `Plugin` constant.

Background instances live for the window, independently of session replacement.
Their create Context has binding 1 and an empty session_id. They cannot request
session services or submit prompts. If a plugin also provides a pane, that pane
gets its own session-scoped instance. Native callbacks run on a worker; GPU
resources and the cached output texture stay in the host. Newer frame requests
replace queued ones. Calls have the same five-second deadline as pane plugins.
The background pauses while unfocused; software rendering shows a flat fallback.

Install the plugin, restart Arc, and select it in Settings → background. Existing
`gui-background` files containing `matrix` continue to use the installed Matrix plugin.
A missing or failed renderer falls back to the last frame or flat background;
Settings shows its error. Restart to reload a modified native library.

## Native ABI

This section specifies the native boundary. Rust authors should use the supplied
export macro. Consult PluginDoc's output for this build's ABI and wire revisions;
the exact values also appear in sdk/src/abi.rs.

### Bootstrap and layout

Export `arc_plugin_entry` with C calling convention and no arguments. It returns
a non-null pointer to the header at the beginning of a statically allocated API
table. The table and function pointers must remain valid for the process lifetime.
The pointer must be aligned for the complete table. The host reads only the
bootstrap header before checking compatibility.

The following C declarations describe the target-native layout. Use normal C
alignment and padding, not packed structures. Pointers and padding depend on the
target architecture; integer widths below are fixed. A package must be built for
the host OS and architecture.

```c
#include <stdint.h>
typedef struct { uint32_t abi_revision, wire_revision, table_size; } ArcHeader;
typedef struct { const uint8_t *ptr; uint64_t len; } ArcBytes;
typedef void *ArcInstance;
typedef struct {
    float width, height;
    uint32_t pixel_width, pixel_height;
    float seconds;
} ArcBackgroundFrame;
typedef struct {
    void *context;
    uint32_t (*create_shader)(void *, ArcBytes, uint64_t *);
    uint32_t (*draw)(void *, uint64_t, ArcBytes);
} ArcGpuApi;
typedef struct {
    ArcHeader header;
    uint32_t (*descriptor)(ArcBytes *);
    uint32_t (*create)(ArcBytes, ArcInstance *, ArcBytes *);
    uint32_t (*update)(ArcInstance, ArcBytes, ArcBytes *);
    uint32_t (*destroy)(ArcInstance);
    uint32_t (*release)(ArcBytes);
    uint32_t (*render_background)(ArcInstance, ArcBackgroundFrame,
                                  const ArcGpuApi *, ArcBytes *);
} ArcApi;
const ArcHeader *arc_plugin_entry(void);
```

The loader requires exact equality of `abi_revision`, `wire_revision`, and
`table_size == sizeof(ArcApi)` on this target. Larger tables are not accepted as
forward-compatible extensions. All callbacks except `render_background` must be
non-null. The render callback is present exactly when the descriptor advertises
`background_fps`. A Rust trait object, Rust String/Vec, future, or Rust Result
must never cross this boundary.

### Buffers and instances

`ArcBytes` is a length-delimited byte span, not a NUL-terminated string. Native
JSON inputs are borrowed for the duration of the call. The caller supplies a
readable non-null span no larger than 4 MiB. Plugins must copy any input they retain.

Every non-null output buffer belongs to the plugin module that allocated it.
The host copies/deserializes it, then calls that module's `release` exactly once,
including for an ERROR response or invalid JSON. Do not use the host's allocator
to free plugin memory. Do not mutate or reuse a returned buffer before release.
Keep `ptr` and `len` together unchanged when releasing it. The SDK allocates each
output as a boxed byte slice and reconstructs that allocation in `release`.

The host initializes output slots to `{NULL, 0}` and the create instance slot to
NULL. Plugins must initialize their outputs on every call. Successful JSON
responses require a non-null buffer with 1 byte to 4 MiB inclusive. The literal
JSON `null` is four bytes, not an empty span. On successful create, the instance
handle must be non-null. An instance is opaque; it may only be used with the API
table that created it and may not be shared between unrelated instances.

`destroy` consumes the instance, and `release` consumes its buffer. Neither
returns JSON. The SDK accepts null for these two no-op cases, but a host must
never double-destroy or double-release a live allocation.

### Status and payloads

| Status | Meaning and output |
| --- | --- |
| 0 / OK | Valid JSON of the callback's success type |
| 1 / ERROR | A JSON string containing an error, for callbacks with a buffer output |
| 2 / PANICKED | Native state is unusable; the host does not inspect/release the output or destroy the affected instance |

Unknown status codes fail the plugin. `destroy` and `release` return OK on
success; any other status is failure. No exception or unwind may cross a C call.
The SDK catches Rust unwinding panics, including serialization and cleanup paths.
An abort, segmentation fault, or memory violation is not isolated by this shim:
plugins execute inside Arc's process. On PANICKED the host deliberately abandons
affected state rather than calling into it again. Cleanup is therefore not
guaranteed after a failure. SDK ERROR strings are shortened to 2,048 Unicode
characters before JSON encoding.

| Callback | Input | Successful output |
| --- | --- | --- |
| descriptor | none | Descriptor |
| create | Context JSON | Initial Update JSON plus instance handle |
| update | instance and Input JSON | Complete replacement Update JSON |
| render_background | instance, native frame, borrowed GPU API | JSON null; draws are recorded through GPU callbacks |
| destroy | instance | status only |
| release | original output ArcBytes | status only |

The compiled Descriptor must match plugin.toml: ID, English name, version,
pane capability, and background FPS. The loader also checks render callback
presence against that capability. `background_max_pixels` is a manifest-only
host hint and is not part of Descriptor.

### Calling order and execution

Each pane instance is driven serially by its host worker. There is no concurrent
`update` on one instance. Distinct pane/background instances may run concurrently;
do not use unsynchronized mutable globals. Descriptor lookup can also occur for
separate loads of the same library.

The Rust shim invokes `create(Context)` once, then collects an initial Update in
this order: `view`, `take_request`, `badge`, `wake_after_ms`, `take_prompt`. For each
Input it invokes `update(Input)` and collects the same five outputs in that order.
The first view does not wait for an input. All outputs must describe a consistent
post-update state. View is replaced in full; it is not a patch stream.

Pane creation and native work are subject to a five-second host response
deadline. Keep callbacks, descriptor generation, serialization, and teardown
short. A timeout retires the worker from use; it does not forcibly kill native
code or a plugin-owned thread. Late results cannot restore a retired pane.
Invalid output, a returned error, or a panic also fails the worker until Arc
restarts. A recoverable operation error belongs in plugin UI state or in a host
service Response, rather than being returned from `Plugin::update`.

On session replacement the host retires the old pane instance, creates a new
one with the new Context, cancels old host reads, and discards obsolete input and
replies. Plugin-owned jobs must bound their own work and cancel without blocking
Drop. A background instance instead lives for its window and receives no session
Inputs; see [Background renderers](#background-renderers).

Successfully loaded libraries remain loaded until process exit, including a
library rejected after native initializers ran. Restart the application to load
new code. A session restart only replaces session instances.

### GPU callback ownership

The GPU table and its context are borrowed for a single render call. Do not
retain them or call them from another thread. Shader handles are nonzero u64
values owned by one background instance. `create_shader` consumes borrowed UTF-8
WGSL and writes a handle on success; `draw` consumes borrowed uniform bytes.
The host copies the data before either callback returns. Both return OK or ERROR;
an error fails the frame even if the plugin ignores it. No GPU callback returns
an allocated message to release. Shader resources are owned by the host; there
is no explicit shader-release callback. See [Background renderers](#background-renderers) for limits and bindings.

## JSON wire format

ABI buffer payloads are UTF-8 JSON without a terminator or framing prefix. Each
input/output is one complete JSON value, at most 4 MiB including escaping and
metadata. Native numeric fields use target C layout; JSON numbers below are
integers in the stated range. Objects are unordered. Emit all fields shown below
when implementing a shim outside Rust; nullable fields use JSON null.

The SDK's serde declarations are the exact field definitions. String-backed
PluginId and JournalRef serialize as strings, not one-field objects. Rust
`Option<T>` serializes as T or null. Missing optional fields deserialize as None.
`Result<T, String>` serializes as `{"Ok": <T>}` or `{"Err": "message"}`; this
capitalization is significant. Arrays represent Vec. Booleans are JSON booleans.

Input, Query, Response, and Node are internally tagged objects using `kind` and
snake_case variant names. Unit variants have only `kind`. Tone, DiffScope, and
ProviderAuth are snake_case strings. Struct field names are snake_case.

Descriptor, Context, View, Badge, TimelineRow, TextSpan, DiffAction, DiffSelection,
Input, and Node reject unknown fields. Query, Response, Update, Request, journal
types, and provider types currently ignore unknown fields. This does not provide
version negotiation: the native header still requires an exact wire revision.

### Creation and results

| Type | Fields |
| --- | --- |
| Descriptor | `id: PluginId`, `name: string`, `version: string`, `pane: bool`, `background_fps: u32 or null` |
| Context | `binding: u64`, `session_id: string`, `workspace: string` |
| Update | `view: View`, `request: Request or null`, `badge: Badge[]`, `wake_after_ms: u64 or null`, `prompt: string or null` |
| View | `nodes: Node[]` |
| Badge | `text: string`, `tone: Tone` |
| Request | `id: u64`, `query: Query` |

The host assigns a nonzero session binding. Workspace is the attached workspace
path. A window background uses binding 1 and an empty session_id. IDs are opaque
to the plugin except where it assigns its own node/action/request IDs.

```json
{"binding":1,"session_id":"session-id","workspace":"/work/project"}
```

```json
{"view":{"nodes":[{"kind":"text","id":"status","text":"Ready","tone":"normal"}]},"request":null,"badge":[],"wake_after_ms":null,"prompt":null}
```

### Inputs

| kind | Additional fields |
| --- | --- |
| click | `id: string` |
| edit | `id: string`, `value: string` |
| diff_action | `id: string`, `action: string`, `line: usize`, `selection: DiffSelection or null` |
| visibility | `visible: bool` |
| journal_changed | none |
| providers_changed | none |
| wake | none |
| response | `id: u64`, `result: Result<Response, string>` |

`usize` is an unsigned integer fitting the target pointer width. Diff line
indexes are zero-based within the node's displayed text, independent of soft
wrapping. DiffSelection is `{first_line: usize, last_line: usize, text: string}`
with inclusive ordered ends and nonempty text. See [Views and interaction](#views-and-interaction) for menu scope rules.

```json
{"kind":"response","id":7,"result":{"Err":"provider settings changed; read the catalog again"}}
```

### Queries and responses

| Query kind | Additional fields | Response kind |
| --- | --- | --- |
| read_journal | `journal: JournalRef`, `cursor: Cursor or null`, `refresh: bool` | journal_page |
| read_record | `record: RecordRef` | record |
| read_providers | none | providers |
| provider_get | `provider: ProviderRef`, `path: string`, `auth: ProviderAuth` | provider_json |

Response variant fields are flattened into the tagged object. For example,
JournalPage's fields are beside `kind`, not inside a `data` or `journal_page` field.

| Type | Fields |
| --- | --- |
| Cursor | `binding: u64`, `journal: JournalRef`, `offset: u64`, `end: u64` |
| RecordRef | `binding: u64`, `journal: JournalRef`, `start: u64`, `end: u64` |
| JournalRecord | `reference: RecordRef`, `data: JSON value`, `child: JournalRef or null` |
| JournalPage | `records: JournalRecord[]`, `cursor: Cursor`, `more: bool` |
| ProviderRef | `revision: u64`, `index: u32` |
| ProviderInfo | `reference: ProviderRef`, `name: string`, `base_url: string`, `configured: bool` |
| SelectedModel | `provider: ProviderRef`, `model: string` |
| Providers | `revision: u64`, `entries: ProviderInfo[]`, `selected: SelectedModel or null` |

`journal_page` flattens JournalPage; `record` flattens JournalRecord; `providers`
flattens Providers. `provider_json` has `provider: ProviderRef` and `data: JSON value`.
ProviderAuth is `bearer` or `raw`. References and cursor byte offsets are host-issued;
do not construct filesystem paths from them. [Host services and scheduling](#host-services-and-scheduling) specifies their lifetime.

```json
{"kind":"response","id":7,"result":{"Ok":{"kind":"journal_page","records":[],"cursor":{"binding":1,"journal":"","offset":0,"end":0},"more":false}}}
```

```json
{"id":8,"query":{"kind":"provider_get","provider":{"revision":1,"index":0},"path":"/account/usage?window=day","auth":"bearer"}}
```

### Node fields and defaults

[Views and interaction](#views-and-interaction) lists all Node variants and their complete field sets. Every node has
`kind` and a nonempty `id`. Tone is one of `normal`, `muted`, `accent`, `success`,
`warning`, `error`. Only Text.tone and TextSpan.tone default to `normal` when
omitted. Only Button.selected, ActionRow.selected, and IconButton.selected default
to false. Other non-optional fields are required, including arrays and booleans.

TextSpan is `{text: string, tone: Tone}`. DiffAction is
`{id: string, label: string, scope: DiffScope}`, where DiffScope is `line`, `block`,
or `selection`. TimelineRow fields are listed in [Views and interaction](#views-and-interaction). All strings use UTF-8;
limits expressed in bytes measure UTF-8 bytes, not characters or displayed width.

### Errors and validation

An ABI ERROR payload is a JSON string, such as `"cannot initialize repository"`.
It is different from a successful Input::Response carrying `{"Err": ...}`.
The latter reports a recoverable service error to an otherwise healthy instance.

The host validates the complete Update before admitting its view and effects.
Malformed JSON, structural/size violations, a null instance, or a bad native
status fails the worker. Oversized user input or a full host input queue is
rejected with a host notice; text drafts are retained. Input admission checks the
currently displayed view and binding. Plugins must still reject obsolete IDs
against their own state because their state can advance before a click arrives.

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

Use [Native ABI](#native-abi) for ownership and failure semantics when debugging native
integration. A native plugin is not sandboxed, and an abort or memory violation
can terminate Arc. Rust's export shim contains unwinding panics but does not
provide process isolation. Never retain host-borrowed pointers or GPU callbacks.

## Not implemented

SaveState, Complete, explicit Cancel, hot loading/unloading, and live enable/disable are
unavailable. Use the SDK's actual variants.
