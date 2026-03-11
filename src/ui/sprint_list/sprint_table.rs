use crate::jira::sprint::{self, Pbi, Sprint};
use crate::plugins::lua_plugin::{execute_plugins, JiraContext};
use crossterm::event::KeyCode;
use ratatui::{
    layout::{Alignment, Constraint, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Cell, Row, Table, TableState},
    Frame,
};
use std::sync::mpsc;
use std::thread;

// ── Internal channel message ─────────────────────────────────────────────────

enum LoadMsg {
    SprintRefreshed(Vec<Pbi>), // pbis, sprint_end_date
    SprintError(String),
}

// ── Public types returned to SprintApp ───────────────────────────────────────

/// Carries the result of a completed background refresh for `SprintApp` to act on.
pub struct LoadUpdate {
    pub status: String,
}

/// Actions that `SprintTable::handle_key` returns to the coordinator (`SprintApp`).
/// The table itself only manages its own state; cross-cutting concerns are
/// delegated upward through these actions.
#[allow(dead_code)]
pub enum TableAction {
    /// User pressed q/Q/Esc — signal the app to exit.
    Exit,
    /// Display this string in the footer.
    SetStatus(String),
    /// Clear the footer status.
    ClearStatus,
    /// PBI data changed; the caller should persist the cache.
    SaveCache,
}

// ── Internal mode ────────────────────────────────────────────────────────────

enum TableMode {
    /// Normal navigation.
    Normal,
}

// ── SprintTable ──────────────────────────────────────────────────────────────

/// Interactive PBI table component.
///
/// Responsibilities:
/// - Rendering the PBI list
/// - Keyboard navigation (j / k / arrows)
/// - Loading PBI details from Jira (f = single, F = all async)
/// - Starting work on a ticket (Enter): run Lua plugins with the selected PBI as context
///
/// Results that affect other components are communicated back to `SprintApp`
/// via [`TableAction`] values returned from [`SprintTable::handle_key`].
pub struct SprintTable {
    pub sprint: Sprint,
    pub table_state: TableState,
    loading_idx: Option<usize>,
    load_rx: Option<mpsc::Receiver<LoadMsg>>,
    mode: TableMode,
}

impl SprintTable {
    pub fn new(sprint: Sprint) -> Self {
        let mut table_state = TableState::default();
        if !sprint.pbis.is_empty() {
            table_state.select(Some(0));
        }
        Self {
            sprint,
            table_state,
            loading_idx: None,
            load_rx: None,
            mode: TableMode::Normal,
        }
    }

    /// Borrow the current PBI slice (used by `ProgressBlock` at render time).
    pub fn pbis(&self) -> &[Pbi] {
        &self.sprint.pbis
    }

    // ── Background refresh ────────────────────────────────────────────────────

    fn start_load_all(&mut self) {
        let board_id = self.sprint.board_id.clone();
        let (tx, rx) = mpsc::channel();
        self.load_rx = Some(rx);

        thread::spawn(
            move || match sprint::fetch_active_sprint_issues(&board_id) {
                Ok(s) => {
                    let _ = tx.send(LoadMsg::SprintRefreshed(s.pbis));
                }
                Err(e) => {
                    let _ = tx.send(LoadMsg::SprintError(e.to_string()));
                }
            },
        );
    }

    /// Drain one pending message from the background refresh thread.
    ///
    /// Returns `Some(LoadUpdate)` when a result arrived; `None` when the
    /// channel is still empty or no refresh is running.
    pub fn process_messages(&mut self) -> Option<LoadUpdate> {
        // Borrow the receiver inside a tight scope so we can freely mutate
        // `self.sprint.pbis` and `self.load_rx` afterwards.
        let msg = {
            let rx = self.load_rx.as_ref()?;
            match rx.try_recv() {
                Ok(msg) => msg,
                Err(_) => return None,
            }
        };

        self.load_rx = None; // channel done

        Some(match msg {
            LoadMsg::SprintRefreshed(pbis) => {
                let count = pbis.len();
                self.sprint.pbis = pbis;
                LoadUpdate {
                    status: format!("Refreshed — {count} issues loaded"),
                }
            }
            LoadMsg::SprintError(e) => LoadUpdate {
                status: format!("Error refreshing sprint: {e}"),
            },
        })
    }

    // ── Navigation ────────────────────────────────────────────────────────────

    fn navigate_down(&mut self) {
        let next = self.table_state.selected().map_or(0, |i| {
            if i >= self.sprint.pbis.len().saturating_sub(1) {
                0
            } else {
                i + 1
            }
        });
        self.table_state.select(Some(next));
    }

    fn navigate_up(&mut self) {
        let prev = self.table_state.selected().map_or(0, |i| {
            if i == 0 {
                self.sprint.pbis.len().saturating_sub(1)
            } else {
                i - 1
            }
        });
        self.table_state.select(Some(prev));
    }

    // ── Single-item load (f) ──────────────────────────────────────────────────

    fn load_selected(&mut self) -> Vec<TableAction> {
        let Some(i) = self.table_state.selected() else {
            return vec![];
        };
        let key = self.sprint.pbis[i].key.clone();
        self.loading_idx = Some(i);

        let actions = match sprint::fetch_pbi_details(&mut self.sprint.pbis[i]) {
            Ok(()) => {
                sprint::sort_by_status(&mut self.sprint.pbis);
                vec![
                    TableAction::SetStatus(format!("Loaded {key}")),
                    TableAction::SaveCache,
                ]
            }
            Err(e) => {
                vec![TableAction::SetStatus(format!("Error loading {key}: {e}"))]
            }
        };

        self.loading_idx = None;
        actions
    }

    // ── Start work (Enter) ────────────────────────────────────────────────────

    fn start_work_on_selected(&mut self) -> Vec<TableAction> {
        let Some(i) = self.table_state.selected() else {
            return vec![];
        };

        // Run Lua plugins with the selected PBI as context, pushing a
        // SetStatus action for each plugin result as it arrives.
        let mut actions: Vec<TableAction> = Vec::new();
        let ctx = JiraContext {
            config: crate::config::JiraConfig::load().unwrap_or_default(),
            sprint: self.sprint.clone(),
            selected_pbi: Some(self.sprint.pbis[i].clone()),
        };
        if let Err(e) = execute_plugins(&ctx, |result| match result {
            Ok(msg) => actions.push(TableAction::SetStatus(msg)),
            Err(e) => actions.push(TableAction::SetStatus(format!("plugin error: {e}"))),
        }) {
            actions.push(TableAction::SetStatus(format!("plugin load error: {e}")));
        }

        actions.push(TableAction::SaveCache);
        actions
    }

    // ── Key handling ──────────────────────────────────────────────────────────

    /// Process a key press and return any [`TableAction`]s for `SprintApp`.
    pub fn handle_key(&mut self, key: KeyCode) -> Vec<TableAction> {
        match self.mode {
            TableMode::Normal => match key {
                KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                    vec![TableAction::Exit]
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.navigate_down();
                    vec![TableAction::ClearStatus]
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    self.navigate_up();
                    vec![TableAction::ClearStatus]
                }
                KeyCode::Char('f') => self.load_selected(),
                KeyCode::Char('F') => {
                    if self.load_rx.is_none() {
                        self.start_load_all();
                        vec![TableAction::SetStatus(
                            "Refreshing sprint from Jira…".into(),
                        )]
                    } else {
                        vec![]
                    }
                }
                KeyCode::Enter => self.start_work_on_selected(),
                _ => vec![],
            },
        }
    }

    // ── Rendering ─────────────────────────────────────────────────────────────

    /// Render the table (and the branch-input popup when active).
    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        let header = Row::new(
            ["", "Type", "Key", "Summary", "Status", "Assignee"]
                .iter()
                .map(|h| {
                    Cell::from(*h).style(
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    )
                }),
        )
        .style(Style::default().bg(Color::DarkGray))
        .height(1);

        let rows: Vec<Row> = self
            .sprint
            .pbis
            .iter()
            .enumerate()
            .map(|(idx, pbi)| {
                let status_style = match pbi.status.to_lowercase().as_str() {
                    s if s.contains("done") || s.contains("closed") => {
                        Style::default().fg(Color::Green)
                    }
                    s if s.contains("progress") => Style::default().fg(Color::Blue),
                    s if s.contains("review") => Style::default().fg(Color::Magenta),
                    s if s.contains("blocked") => Style::default().fg(Color::Red),
                    s if s.contains("resolved") => Style::default().fg(Color::Green),
                    _ => Style::default().fg(Color::White),
                };

                let indicator = if self.loading_idx == Some(idx) {
                    Cell::from("⟳").style(
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    )
                } else if pbi.loaded {
                    Cell::from("✓").style(Style::default().fg(Color::Green))
                } else {
                    Cell::from(" ")
                };

                Row::new(vec![
                    indicator,
                    Cell::from(pbi.issue_type.clone()),
                    Cell::from(pbi.key.clone()).style(Style::default().fg(Color::Cyan)),
                    Cell::from(pbi.summary.clone()),
                    Cell::from(pbi.status.clone()).style(status_style),
                    Cell::from(pbi.assignee.clone()),
                ])
            })
            .collect();

        let table_widget = Table::new(
            rows,
            [
                Constraint::Length(2),
                Constraint::Length(12),
                Constraint::Length(12),
                Constraint::Min(40),
                Constraint::Length(18),
                Constraint::Length(20),
            ],
        )
        .header(header)
        .block(
            Block::bordered()
                .title(format!(" {} items ", self.sprint.pbis.len()))
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
}
