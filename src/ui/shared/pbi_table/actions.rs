use crate::jira::pbi::Pbi;

/// Actions that `PbiTable::handle_key` returns to the parent app.
/// The table itself only manages its own state; cross-cutting concerns are
/// delegated upward through these actions.
#[derive(Debug, Clone)]
pub enum TableAction {
    /// User pressed q/Q/Esc — signal the app to exit.
    Exit,
    /// Display this string in the footer.
    SetStatus(String),
    /// Clear the footer status.
    ClearStatus,
    /// PBI data changed; the caller should persist the cache.
    SaveCache,
    /// Open the detail view for the PBI at this index.
    OpenDetail(Box<Pbi>),
    /// Open the raw JSON in editor for the PBI at this index.
    OpenRaw(usize),
    /// Refresh a single PBI at this index.
    Refresh(usize),
    /// Refresh all PBIs.
    RefreshAll,
    /// Open the plugin list view.
    OpenPlugins,
    /// Start work on the selected PBI (run plugins).
    StartWork(usize),
    /// No action needed.
    Noop,
}
