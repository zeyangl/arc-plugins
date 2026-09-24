# Native ABI

This chapter specifies the native boundary. Rust authors should use the supplied
export macro. Consult reference.json for this snapshot's ABI and wire revisions;
the exact values also appear in sdk/src/abi.rs.

## Bootstrap and layout

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

## Buffers and instances

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

## Status and payloads

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

## Calling order and execution

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
Inputs; see BACKGROUND.md.

Successfully loaded libraries remain loaded until process exit, including a
library rejected after native initializers ran. Restart the application to load
new code. A session restart only replaces session instances.

## GPU callback ownership

The GPU table and its context are borrowed for a single render call. Do not
retain them or call them from another thread. Shader handles are nonzero u64
values owned by one background instance. `create_shader` consumes borrowed UTF-8
WGSL and writes a handle on success; `draw` consumes borrowed uniform bytes.
The host copies the data before either callback returns. Both return OK or ERROR;
an error fails the frame even if the plugin ignores it. No GPU callback returns
an allocated message to release. Shader resources are owned by the host; there
is no explicit shader-release callback. See BACKGROUND.md for limits and bindings.
