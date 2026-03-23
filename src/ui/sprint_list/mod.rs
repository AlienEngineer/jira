mod progress_block;
mod sprint_goal;
mod sprint_table;

use progress_block::{ProgressBlock, SprintProgressData};
use sprint_goal::SprintGoalWidget;
use sprint_table::SprintTable;

use crate::config::keymaps::Scope;
use crate::jira::pbi::Pbi;
use crate::jira::sprint::{self, Sprint, SprintService};
use crate::lua::init::{take_command_receiver, JiraCommand};
use crate::prelude::Result;
use crate::ui::pbi_detail::PbiDetailView;
use crate::ui::plugin_list::PluginListView;
use crate::ui::shared::editor::{open_pbi_in_browser, open_raw_in_editor};
use crate::ui::shared::footer::Footer;
use crate::ui::shared::pbi_table::TableAction;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::{DefaultTerminal, Frame};
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc::Receiver;
use std::sync::Arc;
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
    selected_pbi: Option<Pbi>,
    pending_plugin_edit: Option<PathBuf>,
    command_rx: Option<Receiver<JiraCommand>>,
}

impl SprintApp {
    pub fn new(sprint: Sprint, sprint_service: Arc<dyn SprintService>) -> Self {
        Self {
            goal: SprintGoalWidget::new(sprint.name.clone(), sprint.goal.clone()),
            table: SprintTable::new(sprint, sprint_service),
            progress: ProgressBlock::new(),
            footer: Footer::new(vec![Scope::Sprint, Scope::Global, Scope::Pbi]),
            exit: false,
            active_view: ActiveView::Sprint,
            pending_plugin_edit: None,
            selected_pbi: None,
            command_rx: take_command_receiver(),
        }
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        while !self.exit {
            self.process_background_messages();
            self.process_lua_commands();

            if let Some(pbi) = self.selected_pbi.take() {
                ratatui::restore();
                open_raw_in_editor(&pbi);
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

    /// Process any commands received from Lua keybindings
    fn process_lua_commands(&mut self) {
        // Collect commands first to avoid borrow conflicts
        let commands: Vec<JiraCommand> = {
            let Some(rx) = &self.command_rx else { return };
            rx.try_iter().collect()
        };

        for cmd in commands {
            match &mut self.active_view {
                ActiveView::Sprint => self.process_sprint_command(cmd),
                ActiveView::PbiDetail(detail) => match cmd {
                    JiraCommand::GoLeft => {
                        self.active_view = ActiveView::Sprint;
                    }
                    JiraCommand::Quit => {
                        self.exit = true;
                    }
                    JiraCommand::GoUp => {
                        detail.scroll_up();
                    }
                    JiraCommand::GoDown => {
                        detail.scroll_down();
                    }
                    JiraCommand::OpenRawPbiJson => {
                        self.selected_pbi = Some(detail.pbi.clone());
                    }
                    JiraCommand::OpenInBrowser => {
                        match open_pbi_in_browser(&detail.pbi.key) {
                            Ok(msg) => self.footer.set_status(msg),
                            Err(msg) => self.footer.set_status(msg),
                        }
                    }
                    JiraCommand::Refresh => {
                        let api = self.table.jira_api();
                        if let Err(e) = crate::jira::pbi::fetch_pbi_details(api, &mut detail.pbi) {
                            self.footer.set_status(format!("Error: {e}"));
                        } else {
                            self.footer.set_status(format!("Loaded {}", detail.pbi.key));
                        }
                    }
                    _ => {}
                },
                ActiveView::PluginList(plugin_list) => match cmd {
                    JiraCommand::GoLeft | JiraCommand::Quit => {
                        self.active_view = ActiveView::Sprint;
                    }
                    JiraCommand::GoUp => {
                        plugin_list.navigate_up();
                    }
                    JiraCommand::GoDown => {
                        plugin_list.navigate_down();
                    }
                    JiraCommand::EditPluginSelected | JiraCommand::GoRight => {
                        if let Some(path) = plugin_list.get_selected_plugin() {
                            self.pending_plugin_edit = Some(path);
                        }
                    }
                    _ => {}
                },
            }
        }
    }

    fn process_sprint_command(&mut self, cmd: JiraCommand) {
        let actions = self.table.handle_command(&cmd);
        for action in actions {
            self.dispatch(action);
        }
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

    fn draw(&mut self, frame: &mut Frame) {
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

    fn draw_sprint(&mut self, frame: &mut Frame, area: Rect) {
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

    fn handle_events(&mut self) -> Result<()> {
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

    fn handle_detail_key(&mut self, key: KeyCode) {
        let ActiveView::PbiDetail(ref mut detail) = self.active_view else {
            return;
        };
        detail.handle_key(key);
    }

    fn handle_plugin_list_key(&mut self, key: KeyCode) {
        let ActiveView::PluginList(ref mut plugin_list) = self.active_view else {
            return;
        };
        plugin_list.handle_key(key);
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
            TableAction::OpenRaw(idx) => {
                if let Some(pbi) = self.table.pbis().get(idx) {
                    self.selected_pbi = Some(pbi.clone());
                }
            }
            TableAction::Refresh(idx) => {
                let actions = self.table.load_pbi(idx);
                for action in actions {
                    self.dispatch(action);
                }
            }
            _ => {} // Other actions not used in sprint view
        }
    }
}

// ── Free functions ────────────────────────────────────────────────────────────

fn open_file_in_editor(path: &Path) {
    let editor = env::var("VISUAL")
        .or_else(|_| env::var("EDITOR"))
        .unwrap_or_else(|_| "vi".to_string());

    let _ = Command::new(&editor)
        .arg(path)
        .status()
        .map_err(|e| eprintln!("Failed to open editor '{editor}': {e}"));
}
