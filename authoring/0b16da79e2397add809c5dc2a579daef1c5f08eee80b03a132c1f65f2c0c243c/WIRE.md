# JSON wire format

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

## Creation and results

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

## Inputs

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
with inclusive ordered ends and nonempty text. See UI.md for menu scope rules.

```json
{"kind":"response","id":7,"result":{"Err":"provider settings changed; read the catalog again"}}
```

## Queries and responses

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
do not construct filesystem paths from them. SERVICES.md specifies their lifetime.

```json
{"kind":"response","id":7,"result":{"Ok":{"kind":"journal_page","records":[],"cursor":{"binding":1,"journal":"","offset":0,"end":0},"more":false}}}
```

```json
{"id":8,"query":{"kind":"provider_get","provider":{"revision":1,"index":0},"path":"/account/usage?window=day","auth":"bearer"}}
```

## Node fields and defaults

UI.md lists all Node variants and their complete field sets. Every node has
`kind` and a nonempty `id`. Tone is one of `normal`, `muted`, `accent`, `success`,
`warning`, `error`. Only Text.tone and TextSpan.tone default to `normal` when
omitted. Only Button.selected, ActionRow.selected, and IconButton.selected default
to false. Other non-optional fields are required, including arrays and booleans.

TextSpan is `{text: string, tone: Tone}`. DiffAction is
`{id: string, label: string, scope: DiffScope}`, where DiffScope is `line`, `block`,
or `selection`. TimelineRow fields are listed in UI.md. All strings use UTF-8;
limits expressed in bytes measure UTF-8 bytes, not characters or displayed width.

## Errors and validation

An ABI ERROR payload is a JSON string, such as `"cannot initialize repository"`.
It is different from a successful Input::Response carrying `{"Err": ...}`.
The latter reports a recoverable service error to an otherwise healthy instance.

The host validates the complete Update before admitting its view and effects.
Malformed JSON, structural/size violations, a null instance, or a bad native
status fails the worker. Oversized user input or a full host input queue is
rejected with a host notice; text drafts are retained. Input admission checks the
currently displayed view and binding. Plugins must still reject obsolete IDs
against their own state because their state can advance before a click arrives.
