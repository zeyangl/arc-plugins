//! Private plugin SDK. Only the C table in `abi` crosses the library boundary.

pub mod abi;
pub mod background;
pub use background::{BackgroundFrame, BackgroundGpu, Shader};
mod providers;
pub use providers::*;
#[cfg(test)]
mod tests;

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub const MAX_MESSAGE_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_PROMPT_BYTES: usize = 64 * 1024;
pub const MAX_NODES: usize = 4096;
pub const MAX_DEPTH: usize = 32;
pub const MAX_DIFF_LINES: usize = 512;
pub const MIN_WAKE_MS: u64 = 16;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct PluginId(String);

impl PluginId {
    pub fn new(id: impl Into<String>) -> Result<Self, String> {
        let id = id.into();
        if id.len() > 64
            || !id.split('-').all(|part| {
                !part.is_empty()
                    && part
                        .bytes()
                        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
            })
        {
            return Err("plugin id must be lowercase letters/digits separated by single hyphens (max 64 bytes)".into());
        }
        Ok(Self(id))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for PluginId {
    type Error = String;
    fn try_from(id: String) -> Result<Self, String> {
        Self::new(id)
    }
}

impl From<PluginId> for String {
    fn from(id: PluginId) -> Self {
        id.0
    }
}

impl std::fmt::Display for PluginId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Descriptor {
    pub pane: bool,
    pub background_fps: Option<u32>,
    pub id: PluginId,
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Context {
    pub binding: u64,
    pub session_id: String,
    pub workspace: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Input {
    Click {
        id: String,
    },
    DiffAction {
        id: String,
        action: String,
        line: usize,
        selection: Option<DiffSelection>,
    },
    Edit {
        id: String,
        value: String,
    },
    Visibility {
        visible: bool,
    },
    JournalChanged,
    ProvidersChanged,
    Wake,
    Response {
        id: u64,
        result: Result<Response, String>,
    },
}

/// An address issued by a journal read, relative to the attached session bundle.
/// The empty address denotes its root journal. Plugins never resolve this as a path.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct JournalRef(pub String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cursor {
    pub binding: u64,
    pub journal: JournalRef,
    pub offset: u64,
    pub end: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordRef {
    pub binding: u64,
    pub journal: JournalRef,
    pub start: u64,
    pub end: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalRecord {
    pub reference: RecordRef,
    pub data: serde_json::Value,
    pub child: Option<JournalRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalPage {
    pub records: Vec<JournalRecord>,
    pub cursor: Cursor,
    pub more: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Query {
    ReadJournal {
        journal: JournalRef,
        cursor: Option<Cursor>,
        refresh: bool,
    },
    ReadRecord {
        record: RecordRef,
    },
    ReadProviders,
    ProviderGet {
        provider: ProviderRef,
        /// Absolute path and optional query on this provider's configured origin.
        path: String,
        auth: ProviderAuth,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    pub id: u64,
    pub query: Query,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Response {
    JournalPage(JournalPage),
    Record(JournalRecord),
    Providers(Providers),
    ProviderJson {
        provider: ProviderRef,
        data: serde_json::Value,
    },
}

/// A view and its independently admitted effects. At most one request may be outstanding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Update {
    pub view: View,
    pub request: Option<Request>,
    pub badge: Vec<Badge>,
    /// Replace the one-shot wake deadline; None disarms it. Wakes also run while hidden.
    pub wake_after_ms: Option<u64>,
    /// A prompt queued into the session as user text — never command-parsed.
    pub prompt: Option<String>,
}

impl Update {
    pub fn validate(&self) -> Result<(), String> {
        self.view.validate()?;
        if self
            .prompt
            .as_ref()
            .is_some_and(|prompt| prompt.is_empty() || prompt.len() > MAX_PROMPT_BYTES)
        {
            return Err("prompt must be 1 byte to 64 KiB".into());
        }
        if self.badge.len() > 3
            || self
                .badge
                .iter()
                .any(|part| part.text.len() > 64 || part.text.contains(['\n', '\r']))
        {
            return Err("rail badge exceeds three single-line parts of 64 bytes".into());
        }
        if self
            .wake_after_ms
            .is_some_and(|ms| !(MIN_WAKE_MS..=60_000).contains(&ms))
        {
            return Err("wake delay must be between 16 and 60000 milliseconds".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Badge {
    pub text: String,
    pub tone: Tone,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tone {
    #[default]
    Normal,
    Muted,
    Accent,
    Success,
    Warning,
    Error,
}

/// A vertical pane of keyed controls; layout and widget state belong to the host.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct View {
    pub nodes: Vec<Node>,
}

/// One clickable event; gaps are elapsed time, durations are measured work.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimelineRow {
    pub id: String,
    pub stamp: String,
    pub label: String,
    pub summary: String,
    pub tone: Tone,
    pub duration_secs: Option<u64>,
    pub gap_secs: Option<u64>,
    pub selected: bool,
}

/// One independently colored part of a clickable row label.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TextSpan {
    pub text: String,
    #[serde(default)]
    pub tone: Tone,
}

/// Context-menu actions on a displayed unified diff. Line indexes are zero-based
/// within this node's text; use a new node ID whenever that snapshot changes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiffAction {
    pub id: String,
    pub label: String,
    pub scope: DiffScope,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiffScope {
    Line,
    Block,
    Selection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiffSelection {
    pub first_line: usize,
    pub last_line: usize,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Node {
    /// A scrollable, contiguous event timeline with a measured-work lane.
    Timeline {
        id: String,
        rows: Vec<TimelineRow>,
        follow: bool,
        portion: u16,
    },
    /// A bounded surface with a title, optional status, and supporting text.
    Card {
        id: String,
        title: String,
        subtitle: String,
        badge: Option<Badge>,
        children: Vec<Node>,
    },
    /// A labeled value with supporting detail and an optional 0–100 progress bar.
    Meter {
        id: String,
        label: String,
        value: String,
        detail: String,
        progress: Option<f32>,
        tone: Tone,
    },
    Text {
        id: String,
        text: String,
        #[serde(default)]
        tone: Tone,
    },
    Button {
        id: String,
        label: String,
        #[serde(default)]
        selected: bool,
    },
    /// A full-width clickable row with independently colored text parts.
    ActionRow {
        id: String,
        spans: Vec<TextSpan>,
        #[serde(default)]
        selected: bool,
    },
    /// A compact glyph button with a tooltip hint (pager arrows, refresh).
    IconButton {
        id: String,
        icon: String,
        hint: String,
        #[serde(default)]
        selected: bool,
    },
    TextInput {
        id: String,
        label: String,
        value: String,
    },
    Row {
        id: String,
        children: Vec<Node>,
    },
    Column {
        id: String,
        children: Vec<Node>,
    },
    /// The first child stays at the viewport top until the section's end.
    StickySection {
        id: String,
        children: Vec<Node>,
    },
    /// Equal-width columns that wrap to fit the available pane width.
    Grid {
        id: String,
        children: Vec<Node>,
    },
    Scroll {
        id: String,
        children: Vec<Node>,
        portion: u16,
    },
    Diff {
        id: String,
        text: String,
        /// Whether this page starts inside a hunk from the previous page.
        in_hunk: bool,
        actions: Vec<DiffAction>,
    },
    Markdown {
        id: String,
        text: String,
    },
    Code {
        id: String,
        text: String,
    },
}

impl Node {
    pub fn id(&self) -> &str {
        match self {
            Self::Timeline { id, .. }
            | Self::Card { id, .. }
            | Self::Meter { id, .. }
            | Self::Text { id, .. }
            | Self::Button { id, .. }
            | Self::ActionRow { id, .. }
            | Self::IconButton { id, .. }
            | Self::TextInput { id, .. }
            | Self::Row { id, .. }
            | Self::Column { id, .. }
            | Self::StickySection { id, .. }
            | Self::Grid { id, .. }
            | Self::Scroll { id, .. }
            | Self::Diff { id, .. }
            | Self::Markdown { id, .. }
            | Self::Code { id, .. } => id,
        }
    }

    pub fn kind(&self) -> &'static str {
        match self {
            Self::Timeline { .. } => "timeline",
            Self::Card { .. } => "card",
            Self::Meter { .. } => "meter",
            Self::Text { .. } => "text",
            Self::Button { .. } => "button",
            Self::ActionRow { .. } => "action_row",
            Self::IconButton { .. } => "icon_button",
            Self::TextInput { .. } => "text_input",
            Self::Row { .. } => "row",
            Self::Column { .. } => "column",
            Self::StickySection { .. } => "sticky_section",
            Self::Grid { .. } => "grid",
            Self::Scroll { .. } => "scroll",
            Self::Diff { .. } => "diff",
            Self::Markdown { .. } => "markdown",
            Self::Code { .. } => "code",
        }
    }

    pub fn children(&self) -> &[Node] {
        match self {
            Self::Card { children, .. }
            | Self::Row { children, .. }
            | Self::Column { children, .. }
            | Self::StickySection { children, .. }
            | Self::Grid { children, .. }
            | Self::Scroll { children, .. } => children,
            _ => &[],
        }
    }
}

impl View {
    pub fn validate(&self) -> Result<(), String> {
        let mut ids = HashSet::new();
        let mut diff_lines = 0;
        let mut pending: Vec<_> = self.nodes.iter().map(|node| (node, 1)).collect();
        while let Some((node, depth)) = pending.pop() {
            if depth > MAX_DEPTH {
                return Err("view exceeds its nesting limit".into());
            }
            let id = node.id();
            validate_id(id, &mut ids)?;
            let valid = match node {
                Node::Timeline { rows, portion, .. } => {
                    for row in rows {
                        validate_id(&row.id, &mut ids)?;
                    }
                    (1..=16).contains(portion)
                }
                Node::Card { badge, .. } => badge.as_ref().is_none_or(|badge| {
                    badge.text.len() <= 64 && !badge.text.contains(['\n', '\r'])
                }),
                Node::Meter { progress, .. } => {
                    progress.is_none_or(|value| value.is_finite() && (0.0..=100.0).contains(&value))
                }
                Node::Diff { text, actions, .. } => {
                    let mut action_ids = HashSet::new();
                    for action in actions {
                        validate_id(&action.id, &mut action_ids)?;
                    }
                    let lines = text.split_inclusive('\n').count();
                    diff_lines += lines;
                    lines <= MAX_DIFF_LINES
                        && actions.len() <= 16
                        && actions.iter().all(|a| !a.label.is_empty())
                }
                Node::IconButton { icon, .. } => !icon.is_empty(),
                Node::ActionRow { spans, .. } => (1..=16).contains(&spans.len()),
                Node::StickySection { children, .. } => !children.is_empty(),
                Node::Scroll { portion, .. } if !(1..=16).contains(portion) => {
                    return Err(format!("scroll {id:?} requires a portion between 1 and 16"));
                }
                Node::Text { .. }
                | Node::Markdown { .. }
                | Node::Code { .. }
                | Node::Button { .. }
                | Node::TextInput { .. }
                | Node::Row { .. }
                | Node::Column { .. }
                | Node::Grid { .. }
                | Node::Scroll { .. } => true,
            };
            if !valid || ids.len() + diff_lines > MAX_NODES {
                return Err(format!("node {id:?} contains invalid or oversized content"));
            }
            pending.extend(node.children().iter().map(|node| (node, depth + 1)));
        }
        Ok(())
    }

    pub fn accepts(&self, input: &Input) -> bool {
        self.nodes().any(|node| match (node, input) {
            (Node::Timeline { rows, .. }, Input::Click { id }) => {
                rows.iter().any(|row| row.id == *id)
            }
            (
                Node::Button { id, .. } | Node::IconButton { id, .. } | Node::ActionRow { id, .. },
                Input::Click { id: target },
            ) => id == target,
            (
                Node::Diff {
                    id, text, actions, ..
                },
                Input::DiffAction {
                    id: target,
                    action,
                    line,
                    selection,
                },
            ) => {
                id == target
                    && *line < text.lines().count()
                    && actions.iter().any(|item| {
                        item.id == *action
                            && match (&item.scope, selection) {
                                (DiffScope::Selection, Some(selection)) => {
                                    selection.first_line <= *line
                                        && *line <= selection.last_line
                                        && selection.last_line < text.lines().count()
                                        && !selection.text.is_empty()
                                }
                                (DiffScope::Line | DiffScope::Block, None) => true,
                                _ => false,
                            }
                    })
            }
            (Node::TextInput { id, .. }, Input::Edit { id: target, .. }) => id == target,
            _ => false,
        })
    }

    pub fn nodes(&self) -> impl Iterator<Item = &Node> {
        let mut pending: Vec<_> = self.nodes.iter().rev().collect();
        std::iter::from_fn(move || {
            let node = pending.pop()?;
            pending.extend(node.children().iter().rev());
            Some(node)
        })
    }
}

fn validate_id<'a>(id: &'a str, ids: &mut HashSet<&'a str>) -> Result<(), String> {
    if ids.len() >= MAX_NODES
        || id.is_empty()
        || id.len() > 64
        || !id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
        || !ids.insert(id)
    {
        return Err(format!("invalid, duplicate, or excess node id {id:?}"));
    }
    Ok(())
}

/// This trait is local to the plugin. It is never passed to arc as a Rust value.
pub trait Plugin: Sized + 'static {
    const ID: &'static str;
    const NAME: &'static str;
    const VERSION: &'static str;
    const PANE: bool = true;
    const BACKGROUND_FPS: Option<u32> = None;

    fn create(context: Context) -> Result<Self, String>;
    fn view(&self) -> View {
        View { nodes: Vec::new() }
    }
    fn update(&mut self, _input: Input) -> Result<(), String> {
        Ok(())
    }
    fn render_background(
        &mut self,
        _frame: BackgroundFrame,
        _gpu: &mut BackgroundGpu<'_>,
    ) -> Result<(), String> {
        Err("plugin has no background renderer".into())
    }
    fn take_request(&mut self) -> Option<Request> {
        None
    }
    fn take_prompt(&mut self) -> Option<String> {
        None
    }
    fn badge(&self) -> Vec<Badge> {
        Vec::new()
    }
    fn wake_after_ms(&self) -> Option<u64> {
        None
    }
}

#[macro_export]
macro_rules! export_plugin {
    ($plugin:ty) => {
        #[no_mangle]
        pub extern "C" fn arc_plugin_entry() -> *const $crate::abi::Header {
            static API: $crate::abi::Api = $crate::abi::api::<$plugin>();
            &API.header
        }
    };
}
