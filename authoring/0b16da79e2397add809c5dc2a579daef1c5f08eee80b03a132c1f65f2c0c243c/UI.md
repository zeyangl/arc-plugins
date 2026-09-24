# Views and interaction

Return a complete `View { nodes }` after each update. Arc renders these declarative
nodes using its fonts, theme, selection system, layout, and widget state. Plugins
do not link iced or submit arbitrary GUI widgets. Identity is the node's kind and
ID inside the plugin/session binding; retain IDs for the same logical control.
Changing an ID intentionally resets that control's widget state.

## Node reference

Every variant has `id: String`; the table lists the remaining fields. Wire kinds
are snake_case. `children` is always `Vec<Node>`; empty containers are allowed
except StickySection. See WIRE.md for defaults.

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

## Limits

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

## Diff actions

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
