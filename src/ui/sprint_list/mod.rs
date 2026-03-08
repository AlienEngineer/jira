use crate::jira::sprint::{self, Pbi};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Cell, Clear, Paragraph, Row, Table, TableState, Wrap},
    Frame,
};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

// Messages sent from the background "load all" thread to the UI thread.
enum LoadMsg {
    /// Sprint refreshed successfully; carries the new PBI list and sprint end date.
    SprintRefreshed(Vec<Pbi>, String),
    /// Refresh failed.
    SprintError(String),
}

/// UI mode for the sprint viewer.
enum AppMode {
    /// Normal navigation mode.
    Normal,
    /// User is editing a branch name to create after starting work on a PBI.
    BranchInput,
}

pub struct SprintApp {
    pub sprint_name: String,
    pub sprint_goal: String,
    /// ISO-8601 date string for the sprint end (e.g. "2026-03-20"), empty when unknown.
    pub sprint_end_date: String,
    pub board_id: String,
    pub pbis: Vec<Pbi>,
    pub table_state: TableState,
    pub status_msg: String,
    /// Index currently being fetched (⟳ indicator).
    loading_idx: Option<usize>,
    /// Channel receiver for the async "load all" thread.
    load_rx: Option<mpsc::Receiver<LoadMsg>>,
    exit: bool,
    /// Current UI mode (normal navigation or branch name input).
    mode: AppMode,
    /// Branch name being edited when in BranchInput mode.
    branch_input: String,
}

impl SprintApp {
    pub fn new(
        sprint_name: String,
        sprint_goal: String,
        sprint_end_date: String,
        board_id: String,
        pbis: Vec<Pbi>,
    ) -> Self {
        let mut table_state = TableState::default();
        if !pbis.is_empty() {
            table_state.select(Some(0));
        }
        Self {
            sprint_name,
            sprint_goal,
            sprint_end_date,
            board_id,
            pbis,
            table_state,
            status_msg: String::new(),
            loading_idx: None,
            load_rx: None,
            exit: false,
            mode: AppMode::Normal,
            branch_input: String::new(),
        }
    }

    pub fn run(&mut self, terminal: &mut ratatui::DefaultTerminal) -> crate::prelude::Result<()> {
        while !self.exit {
            // Drain any messages from the background "load all" thread first.
            self.process_load_messages();
            terminal.draw(|frame| self.draw(frame))?;

            // Poll with a short timeout so background loads can keep the UI
            // refreshing without requiring a key press.
            if event::poll(Duration::from_millis(50))? {
                self.handle_events(terminal)?;
            }
        }
        Ok(())
    }

    // ── Cache ────────────────────────────────────────────────────────────────

    fn save_cache(&self) {
        sprint::save_sprint_cache(
            &self.board_id,
            &self.sprint_name,
            &self.sprint_goal,
            &self.sprint_end_date,
            &self.pbis,
        );
    }

    // ── Single-item load (synchronous, shows ⟳ before blocking) ─────────────

    fn load_selected(
        &mut self,
        terminal: &mut ratatui::DefaultTerminal,
    ) -> crate::prelude::Result<()> {
        let Some(i) = self.table_state.selected() else {
            return Ok(());
        };
        let key = self.pbis[i].key.clone();
        self.loading_idx = Some(i);
        self.status_msg = format!("Loading {}…", key);
        terminal.draw(|frame| self.draw(frame))?;

        match sprint::fetch_pbi_details(&mut self.pbis[i]) {
            Ok(()) => {
                sprint::sort_by_status(&mut self.pbis);
                self.status_msg = format!("Loaded {}", key);
                self.save_cache();
            }
            Err(e) => {
                self.status_msg = format!("Error loading {key}: {e}");
            }
        }
        self.loading_idx = None;
        Ok(())
    }

    // ── Bulk load (async thread → mpsc channel) ───────────────────────────────

    fn start_load_all(&mut self) {
        let board_id = self.board_id.clone();
        let (tx, rx) = mpsc::channel();
        self.load_rx = Some(rx);
        self.status_msg = "Refreshing sprint from Jira…".to_string();

        thread::spawn(move || {
            match sprint::fetch_active_sprint_issues(&board_id) {
                Ok((_name, _goal, end_date, pbis)) => {
                    let _ = tx.send(LoadMsg::SprintRefreshed(pbis, end_date));
                }
                Err(e) => {
                    let _ = tx.send(LoadMsg::SprintError(e.to_string()));
                }
            }
        });
    }

    /// Drain all pending messages from the background thread without blocking.
    fn process_load_messages(&mut self) {
        let done = {
            let Some(ref rx) = self.load_rx else { return };
            let mut done = false;
            loop {
                match rx.try_recv() {
                    Ok(LoadMsg::SprintRefreshed(pbis, end_date)) => {
                        let count = pbis.len();
                        self.pbis = pbis;
                        if !end_date.is_empty() {
                            self.sprint_end_date = end_date;
                        }
                        self.status_msg = format!("Refreshed — {count} issues loaded");
                        done = true;
                        break;
                    }
                    Ok(LoadMsg::SprintError(e)) => {
                        self.status_msg = format!("Error refreshing sprint: {e}");
                        done = true;
                        break;
                    }
                    Err(_) => break, // empty or disconnected
                }
            }
            done
        };
        if done {
            self.load_rx = None;
            self.save_cache();
        }
    }

    // ── Rendering ────────────────────────────────────────────────────────────

    fn draw(&mut self, frame: &mut Frame) {
        let area = frame.area();

        let goal_height = if self.sprint_goal.is_empty() { 0u16 } else { 3 };

        let layout = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(goal_height),
            Constraint::Min(0),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .split(area);

        // Title bar
        frame.render_widget(
            Line::from(vec![
                Span::raw(" Sprint: "),
                Span::styled(
                    self.sprint_name.clone(),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            layout[0],
        );

        // Sprint goal
        if !self.sprint_goal.is_empty() {
            frame.render_widget(
                Paragraph::new(self.sprint_goal.as_str())
                    .style(Style::default().fg(Color::White).italic())
                    .block(
                        Block::bordered()
                            .title(Span::styled(
                                " Goal ",
                                Style::default()
                                    .fg(Color::Yellow)
                                    .add_modifier(Modifier::BOLD),
                            ))
                            .border_style(Style::default().fg(Color::Yellow)),
                    )
                    .wrap(Wrap { trim: true }),
                layout[1],
            );
        }

        // Table
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

        let table = Table::new(
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
                .title_alignment(ratatui::layout::Alignment::Right),
        )
        .row_highlight_style(
            Style::default()
                .bg(Color::Blue)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

        frame.render_stateful_widget(table, layout[2], &mut self.table_state);

        // Progress block
        frame.render_widget(self.progress_widget(), layout[3]);

        // Footer
        let status_span = if self.status_msg.is_empty() {
            Span::raw("")
        } else {
            Span::styled(
                format!("  [{}]", self.status_msg),
                Style::default().fg(Color::Green),
            )
        };
        frame.render_widget(
            Line::from(vec![
                Span::raw(" "),
                Span::styled("↵", Style::default().fg(Color::Yellow).bold()),
                Span::raw(" Start  "),
                Span::styled("k/j", Style::default().fg(Color::Yellow).bold()),
                Span::raw(" Navigate  "),
                Span::styled("f", Style::default().fg(Color::Yellow).bold()),
                Span::raw(" Load line  "),
                Span::styled("F", Style::default().fg(Color::Yellow).bold()),
                Span::raw(" Load all  "),
                Span::styled("q", Style::default().fg(Color::Yellow).bold()),
                Span::raw(" Quit"),
                status_span,
            ]),
            layout[4],
        );

        // Branch input popup overlay (rendered last so it appears on top).
        if matches!(self.mode, AppMode::BranchInput) {
            self.draw_branch_popup(frame);
        }
    }

    // ── Progress block ────────────────────────────────────────────────────────

    /// Build the progress `Paragraph` widget shown between the table and footer.
    fn progress_widget(&self) -> Paragraph<'_> {
        let total = self.pbis.len();
        let resolved = self.pbis.iter().filter(|p| {
            let s = p.status.to_lowercase();
            s.contains("done") || s.contains("closed") || s.contains("resolved")
        }).count();

        let pct = if total > 0 { resolved * 100 / total } else { 0 };

        // Build the bar (28 characters wide).
        const BAR_WIDTH: usize = 28;
        let filled = if total > 0 { resolved * BAR_WIDTH / total } else { 0 };
        let bar: String = format!(
            "[{}{}]",
            "█".repeat(filled),
            "░".repeat(BAR_WIDTH - filled),
        );

        // Working days label.
        let days_label = match Self::working_days_remaining(&self.sprint_end_date) {
            Some(0) => " ⏱ Sprint ends today!".to_string(),
            Some(d) => format!(" ⏱ {} working day{} left", d, if d == 1 { "" } else { "s" }),
            None => String::new(),
        };

        let bar_color = if pct >= 80 {
            Color::Green
        } else if pct >= 40 {
            Color::Yellow
        } else {
            Color::Red
        };

        Paragraph::new(Line::from(vec![
            Span::styled(bar, Style::default().fg(bar_color)),
            Span::styled(
                format!(" {}% ({}/{} resolved)", pct, resolved, total),
                Style::default().fg(Color::White),
            ),
            Span::styled(days_label, Style::default().fg(Color::Cyan)),
        ]))
        .block(
            Block::bordered()
                .title(Span::styled(
                    " Sprint Progress ",
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                ))
                .border_style(Style::default().fg(Color::DarkGray)),
        )
    }

    /// Count weekdays (Mon–Fri) from today through `end_date_str` (inclusive).
    /// Returns `None` when the date string is absent or unparseable.
    fn working_days_remaining(end_date_str: &str) -> Option<i64> {
        use chrono::{Datelike, Local, NaiveDate, Weekday};

        if end_date_str.is_empty() {
            return None;
        }
        let end = NaiveDate::parse_from_str(&end_date_str[..10.min(end_date_str.len())], "%Y-%m-%d").ok()?;
        let today = Local::now().date().naive_local();
        if today > end {
            return Some(0);
        }
        let mut count = 0i64;
        let mut d = today;
        while d <= end {
            match d.weekday() {
                Weekday::Sat | Weekday::Sun => {}
                _ => count += 1,
            }
            d = d.succ_opt()?;
        }
        Some(count)
    }

    // ── Branch input popup ────────────────────────────────────────────────────

    fn draw_branch_popup(&self, frame: &mut Frame) {
        let area = frame.area();
        let popup_width = 66u16.min(area.width.saturating_sub(4));
        let popup_height = 7u16;
        let popup_x = area.width.saturating_sub(popup_width) / 2;
        let popup_y = area.height.saturating_sub(popup_height) / 2;
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

    // ── Start work on selected PBI ────────────────────────────────────────────

    fn start_work_on_selected(
        &mut self,
        terminal: &mut ratatui::DefaultTerminal,
    ) -> crate::prelude::Result<()> {
        let Some(i) = self.table_state.selected() else {
            return Ok(());
        };
        let key = self.pbis[i].key.clone();
        let summary = self.pbis[i].summary.clone();

        self.status_msg = format!("Starting work on {}…", key);
        terminal.draw(|frame| self.draw(frame))?;

        // 1. Assign the ticket to the current user via account_id from config.
        let account_id = crate::config::get_config("account_id".to_string());
        if account_id.is_empty() {
            self.status_msg =
                "Cannot assign: account_id not set. Re-run initial setup.".to_string();
            return Ok(());
        }
        let payload = json::object! { "accountId": account_id.as_str() };
        if let Err(e) =
            crate::jira::api::put_call(format!("issue/{key}/assignee"), payload, 3)
        {
            self.status_msg = format!("Error assigning {key}: {e}");
            return Ok(());
        }

        // Update the local PBI assignee with the configured email prefix.
        let email = crate::config::get_config("email".to_string());
        self.pbis[i].assignee = email
            .split('@')
            .next()
            .unwrap_or("You")
            .to_string();

        // 2. Transition the ticket to "In Progress".
        match crate::jira::transitions::get_transition_code(
            key.clone(),
            "in progress".to_string(),
        ) {
            Some(code) => {
                let json_object = json::object! { "transition": { "id": code } };
                if let Err(e) = crate::jira::api::post_call(
                    format!("issue/{key}/transitions"),
                    json_object,
                    3,
                ) {
                    self.status_msg = format!("Error transitioning {key}: {e}");
                    return Ok(());
                }
                self.pbis[i].status = "In Progress".to_string();
            }
            None => {
                self.status_msg =
                    format!("No 'In Progress' transition found for {key}");
                return Ok(());
            }
        }

        // Re-sort and persist; keep the cursor on the same PBI.
        let key_ref = key.clone();
        sprint::sort_by_status(&mut self.pbis);
        if let Some(new_i) = self.pbis.iter().position(|p| p.key == key_ref) {
            self.table_state.select(Some(new_i));
        }
        self.save_cache();

        self.status_msg = format!("{key} assigned to you and moved to In Progress");
        terminal.draw(|frame| self.draw(frame))?;

        // 3. Suggest a branch name and switch to branch-input mode.
        self.branch_input = self.suggest_branch_name(&key, &summary);
        self.mode = AppMode::BranchInput;
        Ok(())
    }

    /// Run `copilot -p <prompt>` and return the first non-empty output line.
    /// Falls back to a deterministic name derived from the ticket key and summary.
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
                let suggestion = raw
                    .lines()
                    .map(str::trim)
                    .find(|l| !l.is_empty())
                    .unwrap_or("")
                    .to_string();
                if !suggestion.is_empty() {
                    return suggestion;
                }
            }
        }
        Self::fallback_branch_name(key, summary)
    }

    /// Generate a branch name from the ticket key and summary without external tools.
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

    /// Run `git checkout -b <branch>` and update the status message.
    fn create_branch(&mut self) {
        let branch_name = self.branch_input.trim().to_string();
        self.mode = AppMode::Normal;

        if branch_name.is_empty() {
            self.status_msg = "Branch name cannot be empty — cancelled.".to_string();
            self.branch_input.clear();
            return;
        }

        match std::process::Command::new("git")
            .args(["checkout", "-b", &branch_name])
            .output()
        {
            Ok(output) if output.status.success() => {
                self.status_msg =
                    format!("✓ Created and switched to branch '{branch_name}'");
            }
            Ok(output) => {
                let err = String::from_utf8_lossy(&output.stderr);
                self.status_msg = format!("git error: {}", err.trim());
            }
            Err(e) => {
                self.status_msg = format!("Failed to run git: {e}");
            }
        }
        self.branch_input.clear();
    }

    // ── Input ────────────────────────────────────────────────────────────────

    fn handle_events(
        &mut self,
        terminal: &mut ratatui::DefaultTerminal,
    ) -> crate::prelude::Result<()> {
        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                match self.mode {
                    AppMode::BranchInput => match key.code {
                        KeyCode::Esc => {
                            self.mode = AppMode::Normal;
                            self.branch_input.clear();
                            self.status_msg = "Branch creation cancelled.".to_string();
                        }
                        KeyCode::Enter => {
                            self.create_branch();
                        }
                        KeyCode::Backspace => {
                            self.branch_input.pop();
                        }
                        KeyCode::Char(c) => {
                            self.branch_input.push(c);
                        }
                        _ => {}
                    },
                    AppMode::Normal => match key.code {
                        KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                            self.exit = true;
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            let next = self.table_state.selected().map_or(0, |i| {
                                if i >= self.pbis.len().saturating_sub(1) {
                                    0
                                } else {
                                    i + 1
                                }
                            });
                            self.table_state.select(Some(next));
                            self.status_msg.clear();
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            let prev = self.table_state.selected().map_or(0, |i| {
                                if i == 0 {
                                    self.pbis.len().saturating_sub(1)
                                } else {
                                    i - 1
                                }
                            });
                            self.table_state.select(Some(prev));
                            self.status_msg.clear();
                        }
                        KeyCode::Char('f') => {
                            self.load_selected(terminal)?;
                        }
                        KeyCode::Char('F') => {
                            if self.load_rx.is_none() {
                                self.start_load_all();
                            }
                        }
                        KeyCode::Enter => {
                            self.start_work_on_selected(terminal)?;
                        }
                        _ => {}
                    },
                }
            }
        }
        Ok(())
    }
}
