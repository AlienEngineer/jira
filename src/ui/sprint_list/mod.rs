mod footer;
mod progress_block;
mod sprint_goal;
mod sprint_table;

use footer::Footer;
use progress_block::{ProgressBlock, SprintProgressData};
use sprint_goal::SprintGoalWidget;
use sprint_table::{SprintTable, TableAction};

use crate::jira::sprint::{self, Sprint};
use crate::ui::pbi_detail::{PbiDetailAction, PbiDetailView};
use crate::ui::plugin_list::{PluginListAction, PluginListView};
use crossterm::event::{self, Event, KeyEventKind};
use ratatui::layout::{Constraint, Layout};
use std::path::PathBuf;
use std::time::Duration;

// ── Active view ───────────────────────────────────────────────────────────────

enum ActiveView {
    Sprint,
    PbiDetail(Box<PbiDetailView>),
    PluginList(Box<PluginListView>),
}

/// Top-level coordinator that owns all sprint UI components and drives the
/// event loop.
///
/// `SprintApp` is responsible for:
/// - Composing the terminal layout
/// - Routing key events to `SprintTable` and dispatching the returned
///   [`TableAction`]s to the appropriate component
/// - Cross-cutting concerns: persisting the sprint cache (needs data from
///   multiple components)
pub struct SprintApp {
    goal: SprintGoalWidget,
    table: SprintTable,
    progress: ProgressBlock,
    footer: Footer,
    exit: bool,
    active_view: ActiveView,
    /// Key of a PBI whose raw JSON should be displayed on the next loop tick.
    pending_raw: Option<String>,
    /// Path to a plugin file to open in the editor on the next loop tick.
    pending_plugin_edit: Option<PathBuf>,
}

impl SprintApp {
    pub fn new(sprint: Sprint) -> Self {
        Self {
            goal: SprintGoalWidget::new(sprint.name.clone(), sprint.goal.clone()),
            table: SprintTable::new(sprint),
            progress: ProgressBlock::new(),
            footer: Footer::new(),
            exit: false,
            active_view: ActiveView::Sprint,
            pending_raw: None,
            pending_plugin_edit: None,
        }
    }

    pub fn run(&mut self, terminal: &mut ratatui::DefaultTerminal) -> crate::prelude::Result<()> {
        while !self.exit {
            self.process_background_messages();

            // Handle a pending raw-JSON edit: suspend the TUI, write JSON to a
            // temp file, open it in $VISUAL/$EDITOR (falling back to vi), wait
            // for the editor to close, delete the temp file, then reinitialise.
            if let Some(key) = self.pending_raw.take() {
                ratatui::restore();
                self.open_raw_in_editor(&key);
                *terminal = ratatui::init();
            }

            if let Some(path) = self.pending_plugin_edit.take() {
                ratatui::restore();
                open_file_in_editor(&path);
                *terminal = ratatui::init();
            }

            terminal.draw(|frame| self.draw(frame))?;

            if event::poll(Duration::from_millis(50))? {
                self.handle_events()?;
            }
        }
        Ok(())
    }

    // ── Background messages ───────────────────────────────────────────────────

    fn process_background_messages(&mut self) {
        if let Some(update) = self.table.process_messages() {
            self.footer.set_status(update.status);
            self.save_cache();
        }
    }

    // ── Cache persistence ─────────────────────────────────────────────────────

    fn save_cache(&self) {
        sprint::save_sprint_cache(&Sprint {
            name: self.goal.sprint_name.to_string(),
            goal: self.goal.sprint_goal.to_string(),
            end_date: self.table.sprint.end_date.clone(),
            pbis: self.table.pbis().to_vec(),
            board_id: self.table.sprint.board_id.clone(),
        });
    }

    // ── Layout & rendering ────────────────────────────────────────────────────

    fn draw(&mut self, frame: &mut ratatui::Frame) {
        let area = frame.area();
        match &mut self.active_view {
            ActiveView::PbiDetail(detail) => {
                detail.render(frame, area);
            }
            ActiveView::PluginList(plugin_list) => {
                plugin_list.render(frame, area);
            }
            ActiveView::Sprint => {
                self.draw_sprint(frame, area);
            }
        }
    }

    fn draw_sprint(&mut self, frame: &mut ratatui::Frame, area: ratatui::layout::Rect) {
        let goal_height = self.goal.goal_height();

        let layout = Layout::vertical([
            Constraint::Length(1),           // title bar
            Constraint::Length(goal_height), // sprint goal (collapses when empty)
            Constraint::Min(0),              // PBI table
            Constraint::Length(3),           // progress block
            Constraint::Length(1),           // footer / key hints
        ])
        .split(area);

        self.goal.render_title(frame, layout[0]);
        self.goal.render_goal(frame, layout[1]);
        self.table.render(frame, layout[2]);

        let progress_data =
            SprintProgressData::from_sprint(self.table.pbis(), &self.table.sprint.end_date);
        self.progress.render(frame, layout[3], &progress_data);

        self.footer.render(frame, layout[4]);
    }

    // ── Event handling ────────────────────────────────────────────────────────

    fn handle_events(&mut self) -> crate::prelude::Result<()> {
        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                match &mut self.active_view {
                    ActiveView::PbiDetail(_) => self.handle_detail_key(key.code),
                    ActiveView::PluginList(_) => self.handle_plugin_list_key(key.code),
                    ActiveView::Sprint => {
                        for action in self.table.handle_key(key.code) {
                            self.dispatch(action);
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn handle_detail_key(&mut self, key: crossterm::event::KeyCode) {
        // Temporarily take ownership so we can mutate the view and then
        // potentially replace active_view without a borrow conflict.
        let ActiveView::PbiDetail(ref mut detail) = self.active_view else {
            return;
        };
        match detail.handle_key(key) {
            Some(PbiDetailAction::Back) => {
                self.active_view = ActiveView::Sprint;
            }
            Some(PbiDetailAction::ShowRaw) => {
                self.pending_raw = Some(detail.pbi.key.clone());
            }
            None => {}
        }
    }

    fn handle_plugin_list_key(&mut self, key: crossterm::event::KeyCode) {
        let ActiveView::PluginList(ref mut plugin_list) = self.active_view else {
            return;
        };
        match plugin_list.handle_key(key) {
            Some(PluginListAction::Back) => {
                self.active_view = ActiveView::Sprint;
            }
            Some(PluginListAction::OpenEditor(path)) => {
                self.pending_plugin_edit = Some(path);
            }
            None => {}
        }
    }

    fn open_raw_in_editor(&self, key: &str) {
        let json = match crate::jira::api::get_call_v2(format!(
            "issue/{key}?expand=changelog,renderedFields"
        )) {
            Ok(v) => json::stringify_pretty(v, 2),
            Err(e) => {
                eprintln!("Error fetching {key}: {e}");
                return;
            }
        };

        let tmp_path = std::env::temp_dir().join(format!("jira_raw_{key}.json"));
        if let Err(e) = std::fs::write(&tmp_path, &json) {
            eprintln!("Failed to write temp file: {e}");
            return;
        }

        let editor = std::env::var("VISUAL")
            .or_else(|_| std::env::var("EDITOR"))
            .unwrap_or_else(|_| "vi".to_string());

        let _ = std::process::Command::new(&editor)
            .arg(&tmp_path)
            .status()
            .map_err(|e| eprintln!("Failed to open editor '{editor}': {e}"));

        let _ = std::fs::remove_file(&tmp_path);
    }

    fn dispatch(&mut self, action: TableAction) {
        match action {
            TableAction::Exit => self.exit = true,
            TableAction::SetStatus(msg) => self.footer.set_status(msg),
            TableAction::ClearStatus => self.footer.clear_status(),
            TableAction::SaveCache => self.save_cache(),
            TableAction::OpenDetail(selected_pbi) => {
                self.active_view =
                    ActiveView::PbiDetail(Box::new(PbiDetailView::new(*selected_pbi)));
            }
            TableAction::OpenPlugins => {
                self.active_view = ActiveView::PluginList(Box::default());
            }
        }
    }
}

// ── Free functions ────────────────────────────────────────────────────────────

fn open_file_in_editor(path: &std::path::Path) {
    let editor = std::env::var("VISUAL")
        .or_else(|_| std::env::var("EDITOR"))
        .unwrap_or_else(|_| "vi".to_string());

    let _ = std::process::Command::new(&editor)
        .arg(path)
        .status()
        .map_err(|e| eprintln!("Failed to open editor '{editor}': {e}"));
}
