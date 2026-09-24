use serde::{Deserialize, Serialize};

/// An entry in one revision of the host's provider catalog.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProviderRef {
    pub revision: u64,
    pub index: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderInfo {
    pub reference: ProviderRef,
    pub name: String,
    /// Endpoint identity with userinfo, query, and fragment removed.
    pub base_url: String,
    pub configured: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectedModel {
    pub provider: ProviderRef,
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Providers {
    pub revision: u64,
    pub entries: Vec<ProviderInfo>,
    pub selected: Option<SelectedModel>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderAuth {
    Bearer,
    /// Some provider account APIs expect the configured key without a prefix.
    Raw,
}
