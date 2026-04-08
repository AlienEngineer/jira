mod actions;
mod columns;

pub use actions::TableAction;
pub use columns::{ColumnConfig, PbiColumn, TableColumn};

use crate::jira::api::JiraApi;
use crate::jira::pbi::{fetch_pbi_details, Pbi};
use crate::lua::init::{get_keymap_collection, JiraCommand};
use crate::ui::components::ui_table::UiTable;
use crate::ui::keycode_mapper::keycode_to_string;
use crate::ui::shared::editor::open_pbi_in_browser;
use crossterm::event::KeyCode;

#[derive(Clone)]
pub struct PbiTable {
    pub table: UiTable,
}

impl PbiTable {
    pub fn new(pbis: Vec<Pbi>) -> Self {
        Self {
            table: UiTable::new(pbis),
        }
    }

    pub fn selected<'a>(&self, pbis: &'a [Pbi]) -> Option<&'a Pbi> {
        self.table.selected(pbis)
    }

    pub fn selected_cloned(&self, pbis: &[Pbi]) -> Option<Pbi> {
        self.selected(pbis).cloned()
    }

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
        let actions = match fetch_pbi_details(api, &mut pbis[idx]) {
            Ok(()) => {
                vec![TableAction::SetStatus(format!("Loaded {key}"))]
            }
            Err(e) => {
                vec![TableAction::SetStatus(format!("Error loading {key}: {e}"))]
            }
        };

        actions
    }

    pub fn handle_command(&mut self, cmd: &JiraCommand, pbis: &[Pbi]) -> Vec<TableAction> {
        match cmd {
            JiraCommand::GoUp => {
                self.table.navigate_up();
                vec![TableAction::ClearStatus]
            }
            JiraCommand::GoDown => {
                self.table.navigate_down();
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
                if let Some(idx) = { self.table.selected_index() } {
                    vec![TableAction::OpenRaw(idx)]
                } else {
                    vec![]
                }
            }
            JiraCommand::Refresh => {
                if let Some(idx) = self.table.selected_index() {
                    vec![TableAction::Refresh(idx)]
                } else {
                    vec![]
                }
            }
            JiraCommand::OpenInBrowser => self.open_in_browser(pbis),
            JiraCommand::AddColumn(column) => {
                self.table.add_column(column.clone());
                vec![]
            }
            JiraCommand::Quit => vec![TableAction::Exit],
            _ => vec![],
        }
    }

    fn open_in_browser(&self, pbis: &[Pbi]) -> Vec<TableAction> {
        let Some(pbi) = self.selected(pbis) else {
            return vec![];
        };
        match open_pbi_in_browser(&pbi.key) {
            Ok(msg) => vec![TableAction::SetStatus(msg)],
            Err(msg) => vec![TableAction::SetStatus(msg)],
        }
    }

    pub fn handle_lua_keymap(
        &self,
        key: KeyCode,
        scopes: &[crate::config::keymaps::Scope],
    ) -> Vec<TableAction> {
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

    pub fn load(&mut self, pbis: Vec<Pbi>) {
        self.table.load(pbis);
    }

    pub(crate) fn set_columns(&mut self, columns: ColumnConfig) {
        self.table.set_columns(columns);
    }
}
