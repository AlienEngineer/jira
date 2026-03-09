use crate::jira::{
    api,
    sprint::{self, Pbi},
    transitions,
};
use crossterm::event::KeyCode;
use ratatui::{
    layout::{Alignment, Constraint, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Cell, Clear, Paragraph, Row, Table, TableState},
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
    /// User is editing the branch name after starting work on a PBI.
    BranchInput,
}

// ── SprintTable ──────────────────────────────────────────────────────────────

/// Interactive PBI table component.
///
/// Responsibilities:
/// - Rendering the PBI list
/// - Keyboard navigation (j / k / arrows)
/// - Loading PBI details from Jira (f = single, F = all async)
/// - Starting work on a ticket (Enter): assign to self, transition to
///   "In Progress", then prompt for a git branch name
/// - Creating the git branch (Enter in branch-input mode)
///
/// Results that affect other components are communicated back to `SprintApp`
/// via [`TableAction`] values returned from [`SprintTable::handle_key`].
pub struct SprintTable {
    pub pbis: Vec<Pbi>,
    pub table_state: TableState,
    loading_idx: Option<usize>,
    load_rx: Option<mpsc::Receiver<LoadMsg>>,
    board_id: String,
    mode: TableMode,
    branch_input: String,
}

impl SprintTable {
    pub fn new(board_id: String, pbis: Vec<Pbi>) -> Self {
        let mut table_state = TableState::default();
        if !pbis.is_empty() {
            table_state.select(Some(0));
        }
        Self {
            pbis,
            table_state,
            loading_idx: None,
            load_rx: None,
            board_id,
            mode: TableMode::Normal,
            branch_input: String::new(),
        }
    }

    /// Borrow the current PBI slice (used by `ProgressBlock` at render time).
    pub fn pbis(&self) -> &[Pbi] {
        &self.pbis
    }

    // ── Background refresh ────────────────────────────────────────────────────

    fn start_load_all(&mut self) {
        let board_id = self.board_id.clone();
        let (tx, rx) = mpsc::channel();
        self.load_rx = Some(rx);

        thread::spawn(
            move || match sprint::fetch_active_sprint_issues(&board_id) {
                Ok(sprint) => {
                    let _ = tx.send(LoadMsg::SprintRefreshed(sprint.pbis));
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
        // `self.pbis` and `self.load_rx` afterwards.
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
                self.pbis = pbis;
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
            if i >= self.pbis.len().saturating_sub(1) {
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
                self.pbis.len().saturating_sub(1)
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
        let key = self.pbis[i].key.clone();
        self.loading_idx = Some(i);

        let actions = match sprint::fetch_pbi_details(&mut self.pbis[i]) {
            Ok(()) => {
                sprint::sort_by_status(&mut self.pbis);
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
        let key = self.pbis[i].key.clone();
        let summary = self.pbis[i].summary.clone();

        // 1. Assign to the current user.
        let account_id = crate::config::get_config("account_id".to_string());
        if account_id.is_empty() {
            return vec![TableAction::SetStatus(
                "Cannot assign: account_id not set. Re-run initial setup.".into(),
            )];
        }

        let payload = json::object! { "accountId": account_id.as_str() };
        if let Err(e) = api::put_call(format!("issue/{key}/assignee"), payload, 3) {
            return vec![TableAction::SetStatus(format!(
                "Error assigning {key}: {e}"
            ))];
        }

        let email = crate::config::get_config("email".to_string());
        self.pbis[i].assignee = email.split('@').next().unwrap_or("You").to_string();

        // 2. Transition to "In Progress".
        match transitions::get_transition_code(key.clone(), "in progress".to_string()) {
            Some(code) => {
                let json_object = json::object! { "transition": { "id": code } };
                if let Err(e) = api::post_call(format!("issue/{key}/transitions"), json_object, 3) {
                    return vec![TableAction::SetStatus(format!(
                        "Error transitioning {key}: {e}"
                    ))];
                }
                self.pbis[i].status = "In Progress".to_string();
            }
            None => {
                return vec![TableAction::SetStatus(format!(
                    "No 'In Progress' transition found for {key}"
                ))];
            }
        }

        // Re-sort and keep cursor on the same PBI.
        let key_ref = key.clone();
        sprint::sort_by_status(&mut self.pbis);
        if let Some(new_i) = self.pbis.iter().position(|p| p.key == key_ref) {
            self.table_state.select(Some(new_i));
        }

        // 3. Suggest a branch name and enter branch-input mode.
        self.branch_input = self.suggest_branch_name(&key, &summary);
        self.mode = TableMode::BranchInput;

        vec![
            TableAction::SetStatus(format!("{key} assigned to you and moved to In Progress")),
            TableAction::SaveCache,
        ]
    }

    // ── Branch helpers ────────────────────────────────────────────────────────

    fn suggest_branch_name(&self, key: &str, summary: &str) -> String {
        let prompt = format!(
            "Suggest a git branch name for Jira ticket {} with summary: {}. \
             Return only the branch name in kebab-case, no explanation.",
            key, summary
        );
        if let Ok(output) = std::process::Command::new("copilot")
            .arg("-p")
            .arg(&prompt)
            .output()
        {
            if output.status.success() {
                let raw = String::from_utf8_lossy(&output.stdout);
                if let Some(line) = raw.lines().map(str::trim).find(|l| !l.is_empty()) {
                    if !line.is_empty() {
                        return line.to_string();
                    }
                }
            }
        }
        Self::fallback_branch_name(key, summary)
    }

    fn fallback_branch_name(key: &str, summary: &str) -> String {
        let slug: String = summary
            .to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '-' })
            .collect::<String>()
            .split('-')
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("-");
        let truncated: String = slug.chars().take(40).collect();
        let trimmed = truncated.trim_end_matches('-');
        format!("feature/{}-{}", key.to_lowercase(), trimmed)
    }

    fn create_branch(&mut self) -> Vec<TableAction> {
        let branch_name = self.branch_input.trim().to_string();
        self.mode = TableMode::Normal;
        self.branch_input.clear();

        if branch_name.is_empty() {
            return vec![TableAction::SetStatus(
                "Branch name cannot be empty — cancelled.".into(),
            )];
        }

        let msg = match std::process::Command::new("git")
            .args(["checkout", "-b", &branch_name])
            .output()
        {
            Ok(output) if output.status.success() => {
                format!("✓ Created and switched to branch '{branch_name}'")
            }
            Ok(output) => {
                let err = String::from_utf8_lossy(&output.stderr);
                format!("git error: {}", err.trim())
            }
            Err(e) => format!("Failed to run git: {e}"),
        };

        vec![TableAction::SetStatus(msg)]
    }

    // ── Key handling ──────────────────────────────────────────────────────────

    /// Process a key press and return any [`TableAction`]s for `SprintApp`.
    pub fn handle_key(&mut self, key: KeyCode) -> Vec<TableAction> {
        match self.mode {
            TableMode::BranchInput => match key {
                KeyCode::Esc => {
                    self.mode = TableMode::Normal;
                    self.branch_input.clear();
                    vec![TableAction::SetStatus("Branch creation cancelled.".into())]
                }
                KeyCode::Enter => self.create_branch(),
                KeyCode::Backspace => {
                    self.branch_input.pop();
                    vec![]
                }
                KeyCode::Char(c) => {
                    self.branch_input.push(c);
                    vec![]
                }
                _ => vec![],
            },
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
                .title(format!(" {} items ", self.pbis.len()))
                .title_alignment(Alignment::Right),
        )
        .row_highlight_style(
            Style::default()
                .bg(Color::Blue)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

        frame.render_stateful_widget(table_widget, area, &mut self.table_state);

        if matches!(self.mode, TableMode::BranchInput) {
            self.render_branch_popup(frame, area);
        }
    }

    fn render_branch_popup(&self, frame: &mut Frame, parent_area: Rect) {
        let popup_width = 66u16.min(parent_area.width.saturating_sub(4));
        let popup_height = 7u16;
        let popup_x = parent_area.x + parent_area.width.saturating_sub(popup_width) / 2;
        let popup_y = parent_area.y + parent_area.height.saturating_sub(popup_height) / 2;
        let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

        frame.render_widget(Clear, popup_area);

        let block = Block::bordered()
            .title(Span::styled(
                " Create Git Branch ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ))
            .border_style(Style::default().fg(Color::Cyan));

        let inner = block.inner(popup_area);
        frame.render_widget(block, popup_area);

        let input_display = format!("{}█", self.branch_input);
        frame.render_widget(
            Paragraph::new(vec![
                Line::from(""),
                Line::from(Span::styled(
                    input_display,
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(vec![
                    Span::styled(" Enter", Style::default().fg(Color::Yellow).bold()),
                    Span::raw(" Create branch   "),
                    Span::styled("Esc", Style::default().fg(Color::Yellow).bold()),
                    Span::raw(" Cancel"),
                ]),
            ]),
            inner,
        );
    }
}
