# Host services and scheduling

## Request lifecycle

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

## Journal reads

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

## Provider catalog

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

## Authenticated provider GET

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

## Wakes, visibility, and prompts

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
