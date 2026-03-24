use crate::config::keymaps::Scope;
use crate::config::JiraConfig;
use crate::jira::assign::AssignService;
use crate::jira::pbi::Pbi;
use crate::jira::sprint::{sort_by_status, Sprint, SprintService};
use crate::jira::transitions::TransitionService;
use crate::lua::init::{create_context, inject_context, JiraCommand};
use crate::plugins::lua_plugin::{execute_plugins, JiraContext};
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
// ── Internal channel message ─────────────────────────────────────────────────

enum LoadMsg {
    SprintRefreshed(Sprint),
    SprintError(String),
}

// ── Public types returned to SprintApp ───────────────────────────────────────

/// Carries the result of a completed background refresh for `SprintApp` to act on.
pub struct LoadUpdate {
    pub status: String,
    /// Updated sprint name (if refresh succeeded).
    pub sprint_name: Option<String>,
    /// Updated sprint goal (if refresh succeeded).
    pub sprint_goal: Option<String>,
}

// ── SprintTable ──────────────────────────────────────────────────────────────

/// Sprint-specific table component wrapping the shared PbiTable.
///
/// Responsibilities:
/// - Rendering the PBI list with sprint-specific columns (indicator, age)
/// - Loading PBI details from Jira (f = single, F = all async)
/// - Starting work on a ticket (Enter): run Lua plugins with the selected PBI as context
///
/// Results that affect other components are communicated back to `SprintApp`
/// via [`TableAction`] values returned from [`SprintTable::handle_key`].
pub struct SprintTable {
    pub sprint: Sprint,
    sprint_service: Arc<dyn SprintService>,
    table: PbiTable,
    load_rx: Option<mpsc::Receiver<LoadMsg>>,
}

impl SprintTable {
    pub fn new(sprint: Sprint, sprint_service: Arc<dyn SprintService>) -> Self {
        let table =
            PbiTable::with_initial_selection(ColumnConfig::sprint_view(), sprint.pbis.len());
        Self {
            sprint,
            sprint_service,
            table,
            load_rx: None,
        }
    }

    /// Borrow the current PBI slice (used by `ProgressBlock` at render time).
    pub fn pbis(&self) -> &[Pbi] {
        &self.sprint.pbis
    }

    /// Returns the currently selected PBI, if any.
    pub fn selected(&self) -> Option<&Pbi> {
        self.table.selected(&self.sprint.pbis)
    }

    /// Get access to the JiraApi for fetching PBI details.
    pub fn jira_api(&self) -> &dyn crate::jira::api::JiraApi {
        self.sprint_service.jira_api()
    }

    // ── Command handling ──────────────────────────────────────────────────────

    /// Handle a JiraCommand and return any actions for the parent app.
    pub fn handle_command(&mut self, cmd: &JiraCommand) -> Vec<TableAction> {
        // Let the shared PbiTable handle common commands
        let mut actions = self.table.handle_command(cmd, &self.sprint.pbis);

        // Handle sprint-specific commands
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
            JiraCommand::StartWork => {
                actions.extend(self.start_work_on_selected());
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

    /// Load details for a single PBI at the given index.
    pub fn load_pbi(&mut self, idx: usize) -> Vec<TableAction> {
        let api = self.sprint_service.jira_api();
        let mut actions = self.table.load_pbi(idx, &mut self.sprint.pbis, api);

        // Sprint-specific: sort by status and save cache on success
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

    // ── Background refresh ────────────────────────────────────────────────────

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

    /// Start a background refresh of all sprint issues (public wrapper).
    pub fn start_load_all_public(&mut self) {
        if self.load_rx.is_none() {
            self.start_load_all();
        }
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
            LoadMsg::SprintRefreshed(sprint) => {
                let count = sprint.pbis.len();
                let name = sprint.name.clone();
                let goal = sprint.goal.clone();
                self.sprint = sprint;
                LoadUpdate {
                    status: format!("Refreshed — {count} issues loaded"),
                    sprint_name: Some(name),
                    sprint_goal: Some(goal),
                }
            }
            LoadMsg::SprintError(e) => LoadUpdate {
                status: format!("Error refreshing sprint: {e}"),
                sprint_name: None,
                sprint_goal: None,
            },
        })
    }

    // ── Start work (Enter) ────────────────────────────────────────────────────

    pub fn start_work_on_selected(&mut self) -> Vec<TableAction> {
        let Some(pbi) = self.table.selected(&self.sprint.pbis) else {
            return vec![];
        };
        let mut actions: Vec<TableAction> = Vec::new();
        let ctx = JiraContext {
            config: JiraConfig::load().unwrap_or_default(),
            sprint: self.sprint.clone(),
            selected_pbi: pbi.clone(),
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
        inject_context(&create_context(
            Some(self.sprint.clone()),
            self.selected().cloned(),
        ))
        .expect("Failed to inject context");
        self.table
            .handle_lua_keymap(key, &[Scope::Sprint, Scope::Global, Scope::Pbi])
    }

    // ── Rendering ─────────────────────────────────────────────────────────────

    /// Render the table.
    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        self.table.render(frame, area, &self.sprint.pbis);
    }
}
