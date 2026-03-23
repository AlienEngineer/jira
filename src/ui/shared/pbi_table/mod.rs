mod actions;
mod columns;

pub use actions::TableAction;
pub use columns::{ColumnConfig, PbiColumn};

use crate::jira::api::JiraApi;
use crate::jira::pbi::{fetch_pbi_details, pbi_elapsed_display, Pbi};
use crate::lua::init::{get_keymap_collection, JiraCommand};
use crate::ui::keycode_mapper::keycode_to_string;
use crate::ui::shared::editor::open_pbi_in_browser;
use crossterm::event::KeyCode;
use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Cell, Row, Table, TableState},
    Frame,
};

/// Reusable PBI table component with configurable columns.
///
/// This component handles:
/// - Rendering the PBI list with configurable columns
/// - Keyboard navigation (j/k/arrows)
/// - Lua keymap integration
///
/// Actions that affect other components are communicated back to the parent
/// via [`TableAction`] values returned from [`PbiTable::handle_key`].
pub struct PbiTable {
    pub table_state: TableState,
    column_config: ColumnConfig,
    loading_idx: Option<usize>,
}

impl PbiTable {
    /// Create a new PbiTable with the given column configuration.
    pub fn new(column_config: ColumnConfig) -> Self {
        Self {
            table_state: TableState::default(),
            column_config,
            loading_idx: None,
        }
    }

    /// Create a new PbiTable and select the first item if any exist.
    pub fn with_initial_selection(column_config: ColumnConfig, item_count: usize) -> Self {
        let mut table = Self::new(column_config);
        if item_count > 0 {
            table.table_state.select(Some(0));
        }
        table
    }

    /// Reset selection state, selecting first item if any exist.
    pub fn reset_selection(&mut self, item_count: usize) {
        if item_count > 0 {
            self.table_state.select(Some(0));
        } else {
            self.table_state.select(None);
        }
    }

    /// Get the currently selected index.
    pub fn selected_index(&self) -> Option<usize> {
        self.table_state.selected()
    }

    /// Get the currently selected PBI from the provided slice.
    pub fn selected<'a>(&self, pbis: &'a [Pbi]) -> Option<&'a Pbi> {
        self.table_state.selected().and_then(|i| pbis.get(i))
    }

    /// Get a clone of the currently selected PBI.
    pub fn selected_cloned(&self, pbis: &[Pbi]) -> Option<Pbi> {
        self.selected(pbis).cloned()
    }

    /// Set the loading indicator for a specific index.
    pub fn set_loading(&mut self, idx: Option<usize>) {
        self.loading_idx = idx;
    }

    // ── PBI Loading ───────────────────────────────────────────────────────────

    /// Load details for a single PBI at the given index.
    /// Returns actions to be dispatched by the parent.
    pub fn load_pbi(
        &mut self,
        idx: usize,
        pbis: &mut [Pbi],
        api: &dyn JiraApi,
    ) -> Vec<TableAction> {
        if idx >= pbis.len() {
            return vec![];
        }
        let key = pbis[idx].key.clone();
        self.set_loading(Some(idx));

        let actions = match fetch_pbi_details(api, &mut pbis[idx]) {
            Ok(()) => {
                vec![TableAction::SetStatus(format!("Loaded {key}"))]
            }
            Err(e) => {
                vec![TableAction::SetStatus(format!("Error loading {key}: {e}"))]
            }
        };

        self.set_loading(None);
        actions
    }

    // ── Navigation ────────────────────────────────────────────────────────────

    pub fn navigate_down(&mut self, count: usize) {
        let next = self.table_state.selected().map_or(0, |i| {
            if i >= count.saturating_sub(1) {
                0
            } else {
                i + 1
            }
        });
        self.table_state.select(Some(next));
    }

    pub fn navigate_up(&mut self, count: usize) {
        let prev = self.table_state.selected().map_or(0, |i| {
            if i == 0 {
                count.saturating_sub(1)
            } else {
                i - 1
            }
        });
        self.table_state.select(Some(prev));
    }

    // ── Command handling ──────────────────────────────────────────────────────

    /// Handle a JiraCommand and return any actions for the parent app.
    /// Navigation commands are handled internally; other commands return actions.
    pub fn handle_command(&mut self, cmd: &JiraCommand, pbis: &[Pbi]) -> Vec<TableAction> {
        match cmd {
            JiraCommand::GoUp => {
                self.navigate_up(pbis.len());
                vec![TableAction::ClearStatus]
            }
            JiraCommand::GoDown => {
                self.navigate_down(pbis.len());
                vec![TableAction::ClearStatus]
            }
            JiraCommand::GoRight | JiraCommand::OpenPbiDetails => {
                if let Some(pbi) = self.selected_cloned(pbis) {
                    vec![TableAction::OpenDetail(Box::new(pbi))]
                } else {
                    vec![]
                }
            }
            JiraCommand::OpenRawPbiJson => {
                if let Some(idx) = self.selected_index() {
                    vec![TableAction::OpenRaw(idx)]
                } else {
                    vec![]
                }
            }
            JiraCommand::Refresh => {
                if let Some(idx) = self.selected_index() {
                    vec![TableAction::Refresh(idx)]
                } else {
                    vec![]
                }
            }
            JiraCommand::OpenInBrowser => self.open_in_browser(pbis),
            JiraCommand::Quit => vec![TableAction::Exit],
            _ => vec![], // Other commands handled by parent
        }
    }

    /// Open the selected PBI in the browser.
    fn open_in_browser(&self, pbis: &[Pbi]) -> Vec<TableAction> {
        let Some(pbi) = self.selected(pbis) else {
            return vec![];
        };
        match open_pbi_in_browser(&pbi.key) {
            Ok(msg) => vec![TableAction::SetStatus(msg)],
            Err(msg) => vec![TableAction::SetStatus(msg)],
        }
    }

    // ── Key handling ──────────────────────────────────────────────────────────

    /// Process a key press for Lua keymaps only.
    /// Navigation and action keys should be handled by the parent app.
    pub fn handle_lua_keymap(&self, key: KeyCode, scopes: &[crate::config::keymaps::Scope]) -> Vec<TableAction> {
        let keycode = keycode_to_string(key);
        if let Some(collection) = get_keymap_collection() {
            let guard = collection.lock().expect("Failed to lock keymaps");
            if let Some(keymap) = guard.get_keymap(&keycode, scopes) {
                match keymap.execute() {
                    Ok(result) => {
                        if !result.is_empty() {
                            return vec![TableAction::SetStatus(result)];
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to execute keymap '{}': {}", keycode, e);
                    }
                }
            }
        }
        vec![]
    }

    // ── Rendering ─────────────────────────────────────────────────────────────

    /// Render the table with the provided PBI data.
    pub fn render(&mut self, frame: &mut Frame, area: Rect, pbis: &[Pbi]) {
        let header = Row::new(self.column_config.headers().iter().map(|h| {
            Cell::from(*h).style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
        }))
        .style(Style::default().bg(Color::DarkGray))
        .height(1);

        let rows: Vec<Row> = pbis
            .iter()
            .enumerate()
            .map(|(idx, pbi)| self.build_row(idx, pbi))
            .collect();

        let table_widget = Table::new(rows, self.column_config.constraints())
            .header(header)
            .block(
                Block::bordered()
                    .title(format!(" {} items ", pbis.len()))
                    .title_alignment(Alignment::Right),
            )
            .row_highlight_style(
                Style::default()
                    .bg(Color::Blue)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(">> ");

        frame.render_stateful_widget(table_widget, area, &mut self.table_state);
    }

    fn build_row(&self, idx: usize, pbi: &Pbi) -> Row<'static> {
        let cells: Vec<Cell> = self
            .column_config
            .columns
            .iter()
            .map(|col| self.build_cell(*col, idx, pbi))
            .collect();
        Row::new(cells)
    }

    fn build_cell(&self, column: PbiColumn, idx: usize, pbi: &Pbi) -> Cell<'static> {
        match column {
            PbiColumn::Indicator => {
                if self.loading_idx == Some(idx) {
                    Cell::from("⟳").style(
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    )
                } else if pbi.loaded {
                    Cell::from("✓").style(Style::default().fg(Color::Green))
                } else {
                    Cell::from(" ")
                }
            }
            PbiColumn::Type => {
                Cell::from(pbi.issue_type.clone()).style(Style::default().fg(Color::DarkGray))
            }
            PbiColumn::Key => {
                Cell::from(pbi.key.clone()).style(Style::default().fg(Color::Cyan))
            }
            PbiColumn::Summary => Cell::from(pbi.summary.clone()),
            PbiColumn::Status => Cell::from(pbi.status.clone()).style(status_color(&pbi.status)),
            PbiColumn::Assignee => Cell::from(pbi.assignee.clone()),
            PbiColumn::Age => {
                Cell::from(pbi_elapsed_display(pbi)).style(Style::default().fg(Color::DarkGray))
            }
            PbiColumn::Priority => {
                let priority = pbi.priority.clone().unwrap_or_default();
                Cell::from(priority)
            }
        }
    }
}

/// Get the appropriate color style for a status string.
pub fn status_color(status: &str) -> Style {
    match status.to_lowercase().as_str() {
        s if s.contains("done") || s.contains("closed") => Style::default().fg(Color::Green),
        s if s.contains("progress") => Style::default().fg(Color::Blue),
        s if s.contains("review") => Style::default().fg(Color::Magenta),
        s if s.contains("blocked") => Style::default().fg(Color::Red),
        s if s.contains("resolved") => Style::default().fg(Color::Green),
        _ => Style::default().fg(Color::White),
    }
}
