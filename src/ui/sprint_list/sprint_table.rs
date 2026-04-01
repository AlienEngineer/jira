use crate::config::keymaps::Scope;
use crate::jira::assign::AssignService;
use crate::jira::pbi::Pbi;
use crate::jira::sprint::{sort_by_status, Sprint, SprintService};
use crate::jira::transitions::TransitionService;
use crate::lua::init::{create_context, inject_context, JiraCommand};
use crate::ui::components::ui_table::UiTable;
use crate::ui::components::ui_widget::UiWidget;
use crate::ui::shared::pbi_table::{ColumnConfig, PbiTable, TableAction};
use crossterm::event::KeyCode;
use ratatui::{layout::Rect, Frame};
use std::sync::{mpsc, Arc};
use std::thread;

fn service<T>() -> Arc<T>
where
    T: ?Sized + crate::ioc::interface::Interface + 'static,
{
    crate::ioc::global()
        .get::<T>()
        .expect("service not registered in IoC container")
}

enum LoadMsg {
    SprintRefreshed(Sprint),
    SprintError(String),
}

pub struct LoadUpdate {
    pub status: String,
    pub sprint_goal: Option<String>,
    pub sprint: Option<Sprint>,
}

pub struct SprintTable {
    pub sprint: Sprint,
    sprint_service: Arc<dyn SprintService>,
    table: PbiTable,
    load_rx: Option<mpsc::Receiver<LoadMsg>>,
}

impl SprintTable {
    pub fn new(sprint: Sprint, sprint_service: Arc<dyn SprintService>) -> Self {
        let table = PbiTable::new(ColumnConfig::sprint_view());
        Self {
            sprint,
            sprint_service,
            table,
            load_rx: None,
        }
    }

    pub fn pbis(&self) -> &[Pbi] {
        &self.sprint.pbis
    }

    pub fn selected(&self) -> Option<&Pbi> {
        self.table.selected(&self.sprint.pbis)
    }

    pub fn jira_api(&self) -> &dyn crate::jira::api::JiraApi {
        self.sprint_service.jira_api()
    }

    pub fn handle_command(&mut self, cmd: &JiraCommand) -> Vec<TableAction> {
        let mut actions = self.table.handle_command(cmd, &self.sprint.pbis);

        match cmd {
            JiraCommand::RefreshAll => {
                self.start_load_all_public();
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

        actions
    }

    pub fn load_pbi(&mut self, idx: usize) -> Vec<TableAction> {
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

        actions
    }

    fn start_load_all(&mut self) {
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

    pub fn start_load_all_public(&mut self) {
        if self.load_rx.is_none() {
            self.start_load_all();
        }
    }

    pub fn process_messages(&mut self) -> Option<LoadUpdate> {
        let msg = {
            let rx = self.load_rx.as_ref()?;
            match rx.try_recv() {
                Ok(msg) => msg,
                Err(_) => return None,
            }
        };

        self.load_rx = None;

        Some(match msg {
            LoadMsg::SprintRefreshed(sprint) => {
                let count = sprint.pbis.len();
                let goal = sprint.goal.clone();
                self.sprint = sprint;
                LoadUpdate {
                    status: format!("Refreshed — {count} issues loaded"),
                    sprint_goal: Some(goal),
                    sprint: Some(self.sprint.clone()),
                }
            }
            LoadMsg::SprintError(e) => LoadUpdate {
                status: format!("Error refreshing sprint: {e}"),
                sprint_goal: None,
                sprint: None,
            },
        })
    }

    pub fn handle_key(&mut self, key: KeyCode) -> Vec<TableAction> {
        inject_context(&create_context(
            Some(self.sprint.clone()),
            self.selected().cloned(),
        ))
        .expect("Failed to inject context");
        self.table
            .handle_lua_keymap(key, &[Scope::Sprint, Scope::Global, Scope::Pbi])
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        self.table.render(frame, area, &self.sprint.pbis);
    }

    pub fn get_widget(&mut self) -> &mut UiTable {
        self.table.get_widget()
    }
}
