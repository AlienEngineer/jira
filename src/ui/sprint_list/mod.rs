use crate::config::keymaps::Scope;
use crate::jira::assign::AssignService;
use crate::jira::pbi::Pbi;
use crate::jira::sprint::{self, sort_by_status, Sprint, SprintService};
use crate::jira::transitions::TransitionService;
use crate::lua::init::{create_context, inject_context, take_command_receiver, JiraCommand};
use crate::prelude::Result;
use crate::ui::components::ui_boxed_title::UiBoxedTitle;
use crate::ui::components::ui_footer::UiFooter;
use crate::ui::components::ui_layout::UiLayout;
use crate::ui::components::ui_sprint_progress::UiSprintProgress;
use crate::ui::components::ui_title::UiTitle;
use crate::ui::components::ui_widget::UiWidget;
use crate::ui::pbi_detail::PbiDetailView;
use crate::ui::plugin_list::PluginListView;
use crate::ui::shared::editor::{open_pbi_in_browser, open_raw_in_editor};
use crate::ui::shared::pbi_table::{ColumnConfig, PbiTable, TableAction};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::layout::Rect;
use ratatui::{DefaultTerminal, Frame};
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc::Receiver;
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::Duration;

fn service<T>() -> Arc<T>
where
    T: ?Sized + crate::ioc::interface::Interface + 'static,
{
    crate::ioc::global()
        .get::<T>()
        .expect("service not registered in IoC container")
}

// ── Background loading ────────────────────────────────────────────────────────

enum LoadMsg {
    SprintRefreshed(Sprint),
    SprintError(String),
}

// ── Active view ───────────────────────────────────────────────────────────────

enum ActiveView {
    Sprint,
    PbiDetail(Box<PbiDetailView>),
    PluginList(Box<PluginListView>),
}

pub struct SprintApp {
    table: PbiTable,
    sprint_service: Arc<dyn SprintService>,
    load_rx: Option<mpsc::Receiver<LoadMsg>>,
    footer: UiFooter,
    exit: bool,
    active_view: ActiveView,
    selected_pbi: Option<Pbi>,
    pending_plugin_edit: Option<PathBuf>,
    command_rx: Option<Receiver<JiraCommand>>,
    sprint: Sprint,
}

impl SprintApp {
    pub fn new(sprint: Sprint, sprint_service: Arc<dyn SprintService>) -> Self {
        let table = PbiTable::new(sprint.pbis.clone());
        Self {
            table,
            sprint_service,
            load_rx: None,
            sprint,
            footer: UiFooter::new(vec![Scope::Sprint, Scope::Global, Scope::Pbi]),
            exit: false,
            active_view: ActiveView::Sprint,
            pending_plugin_edit: None,
            selected_pbi: None,
            command_rx: take_command_receiver(),
        }
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        self.table.set_columns(ColumnConfig::sprint_view());
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

    fn process_lua_commands(&mut self) {
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
                    JiraCommand::OpenInBrowser => match open_pbi_in_browser(&detail.pbi.key) {
                        Ok(msg) => self.footer.set_status(msg),
                        Err(msg) => self.footer.set_status(msg),
                    },
                    JiraCommand::Refresh => {
                        let api = self.sprint_service.jira_api();
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
        let mut actions = self.table.handle_command(&cmd, &self.sprint.pbis);

        match &cmd {
            JiraCommand::RefreshAll => {
                self.start_load_all();
                actions.push(TableAction::SetStatus(
                    "Refreshing sprint from Jira…".into(),
                ));
            }
            JiraCommand::OpenPluginList => {
                actions.push(TableAction::OpenPlugins);
            }
            JiraCommand::Print(msg) => {
                actions.push(TableAction::SetStatus(msg.clone()));
            }
            JiraCommand::AssignPbi(pbi_id, account_id) => {
                let result = service::<dyn AssignService>()
                    .assign_ticket_to_account(pbi_id.clone(), account_id.clone());

                if result.is_err() {
                    actions.push(TableAction::SetStatus(format!(
                        "Error assigning {pbi_id} to {account_id}"
                    )));
                }
            }
            JiraCommand::ChangePbiStatus(pbi_id, status) => {
                let result = service::<dyn TransitionService>()
                    .change_pbi_status(pbi_id.clone(), status.clone());

                if result.is_err() {
                    actions.push(TableAction::SetStatus(format!(
                        "Error changing {pbi_id} status to {status}"
                    )));
                }
            }
            _ => {}
        }

        for action in actions {
            self.dispatch(action);
        }
    }

    // ── Background messages ───────────────────────────────────────────────────

    fn process_background_messages(&mut self) {
        let msg = {
            let Some(rx) = self.load_rx.as_ref() else {
                return;
            };
            match rx.try_recv() {
                Ok(msg) => msg,
                Err(_) => return,
            }
        };

        self.load_rx = None;

        match msg {
            LoadMsg::SprintRefreshed(sprint) => {
                let count = sprint.pbis.len();
                self.sprint = sprint;
                self.footer
                    .set_status(format!("Refreshed — {count} issues loaded"));
                self.table.load(self.sprint.pbis.clone());
                self.save_cache();
            }
            LoadMsg::SprintError(e) => {
                self.footer
                    .set_status(format!("Error refreshing sprint: {e}"));
            }
        }
    }

    // ── Cache persistence ─────────────────────────────────────────────────────

    fn save_cache(&self) {
        sprint::save_sprint_cache(&self.sprint);
    }

    // ── Background sprint loading ─────────────────────────────────────────────

    fn start_load_all(&mut self) {
        if self.load_rx.is_some() {
            return;
        }

        let board_id = self.sprint.board_id.clone();
        let sprint_service = Arc::clone(&self.sprint_service);
        let (tx, rx) = mpsc::channel();
        self.load_rx = Some(rx);

        thread::spawn(
            move || match sprint_service.fetch_active_sprint_issues(&board_id) {
                Ok(s) => {
                    let _ = tx.send(LoadMsg::SprintRefreshed(s));
                }
                Err(e) => {
                    let _ = tx.send(LoadMsg::SprintError(e.to_string()));
                }
            },
        );
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
        let mut layout = UiLayout::new();
        let goal_widget = UiBoxedTitle::new(" Goal ", self.sprint.goal.as_str());
        let title_widget = UiTitle::new(" Sprint: ", self.sprint.name.as_str());
        let ui_sprint_progress = UiSprintProgress::new(
            " Sprint Progress ",
            self.sprint.pbis.len(),
            self.sprint
                .pbis
                .iter()
                .filter(|p| {
                    let s = p.status.to_lowercase();
                    s.contains("closed") || s.contains("resolved")
                })
                .count(),
            &self.sprint.end_date,
        );

        let table_widget = self.table.table.clone();
        layout.add_widget(Box::new(title_widget));
        layout.add_widget(Box::new(goal_widget));
        layout.add_widget(Box::new(table_widget));
        layout.add_widget(Box::new(ui_sprint_progress));
        layout.add_widget(Box::new(self.footer.clone()));

        layout.render_widget(frame, area);
    }

    fn handle_events(&mut self) -> Result<()> {
        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                match &mut self.active_view {
                    ActiveView::PbiDetail(_) => self.handle_detail_key(key.code),
                    ActiveView::PluginList(_) => self.handle_plugin_list_key(key.code),
                    ActiveView::Sprint => {
                        inject_context(&create_context(
                            Some(self.sprint.clone()),
                            self.table.selected(&self.sprint.pbis).cloned(),
                        ))
                        .expect("Failed to inject context");
                        let actions = self.table.handle_lua_keymap(
                            key.code,
                            &[Scope::Sprint, Scope::Global, Scope::Pbi],
                        );
                        for action in actions {
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
                if let Some(pbi) = self.sprint.pbis.get(idx) {
                    self.selected_pbi = Some(pbi.clone());
                }
            }
            TableAction::Refresh(idx) => {
                let api = self.sprint_service.jira_api();
                let mut actions = self.table.load_pbi(idx, &mut self.sprint.pbis, api);

                if !actions.is_empty() {
                    if let Some(TableAction::SetStatus(msg)) = actions.first() {
                        if msg.starts_with("Loaded") {
                            sort_by_status(&mut self.sprint.pbis);
                            actions.push(TableAction::SaveCache);
                        }
                    }
                }

                for action in actions {
                    self.dispatch(action);
                }
            }
            _ => {}
        }
    }

    pub fn handle_key_event(&mut self, key: KeyCode) {
        match &mut self.active_view {
            ActiveView::PbiDetail(_) => self.handle_detail_key(key),
            ActiveView::PluginList(_) => self.handle_plugin_list_key(key),
            ActiveView::Sprint => {
                inject_context(&create_context(
                    Some(self.sprint.clone()),
                    self.table.selected(&self.sprint.pbis).cloned(),
                ))
                .expect("Failed to inject context");
                let actions = self
                    .table
                    .handle_lua_keymap(key, &[Scope::Sprint, Scope::Global, Scope::Pbi]);
                for action in actions {
                    self.dispatch(action);
                }
            }
        }
        self.process_lua_commands();
    }

    /// Returns the currently selected PBI, if any.
    pub fn selected_pbi(&self) -> Option<&Pbi> {
        self.table.selected(&self.sprint.pbis)
    }

    /// Returns true if the app should exit.
    pub fn is_exit(&self) -> bool {
        self.exit
    }

    /// Returns true if in detail view.
    pub fn is_detail_view(&self) -> bool {
        matches!(self.active_view, ActiveView::PbiDetail(_))
    }

    /// Returns the list of PBIs.
    pub fn pbis(&self) -> &[Pbi] {
        &self.sprint.pbis
    }

    /// Returns the sprint end date.
    pub fn sprint_end_date(&self) -> &str {
        &self.sprint.end_date
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
