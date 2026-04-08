use crate::lua::init::TableColumn as LuaTableColumn;
use ratatui::layout::Constraint;

/// Available columns for the PBI table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PbiColumn {
    /// Loading state indicator (✓/⟳)
    Indicator,
    /// Issue type (Story, Bug, etc.)
    Type,
    /// Issue key (PROJ-123)
    Key,
    /// Issue summary/title
    Summary,
    /// Issue status
    Status,
    /// Assignee name
    Assignee,
    /// Time since work started
    Age,
    /// Priority level
    Priority,
}

impl PbiColumn {
    /// Get the header label for this column.
    pub fn header(&self) -> &'static str {
        match self {
            PbiColumn::Indicator => "",
            PbiColumn::Type => "Type",
            PbiColumn::Key => "Key",
            PbiColumn::Summary => "Summary",
            PbiColumn::Status => "Status",
            PbiColumn::Assignee => "Assignee",
            PbiColumn::Age => "Age",
            PbiColumn::Priority => "Priority",
        }
    }

    /// Get the layout constraint for this column.
    pub fn constraint(&self) -> Constraint {
        match self {
            PbiColumn::Indicator => Constraint::Length(2),
            PbiColumn::Type => Constraint::Length(12),
            PbiColumn::Key => Constraint::Length(12),
            PbiColumn::Summary => Constraint::Min(40),
            PbiColumn::Status => Constraint::Length(18),
            PbiColumn::Assignee => Constraint::Length(20),
            PbiColumn::Age => Constraint::Length(5),
            PbiColumn::Priority => Constraint::Length(10),
        }
    }
}

#[derive(Debug, Clone)]
pub enum TableColumn {
    BuiltIn(PbiColumn),
    Custom(LuaTableColumn),
}

impl TableColumn {
    pub fn header(&self) -> String {
        match self {
            TableColumn::BuiltIn(column) => column.header().to_string(),
            TableColumn::Custom(column) => column.name().to_string(),
        }
    }

    pub fn constraint(&self) -> Constraint {
        match self {
            TableColumn::BuiltIn(column) => column.constraint(),
            TableColumn::Custom(column) => {
                let width = (column.name().len() + 2).clamp(10, 20) as u16;
                Constraint::Length(width)
            }
        }
    }
}

/// Configuration for which columns to display in the table.
#[derive(Debug, Clone)]
pub struct ColumnConfig {
    pub columns: Vec<TableColumn>,
}

impl ColumnConfig {
    pub fn sprint_view() -> Self {
        Self {
            columns: vec![
                TableColumn::BuiltIn(PbiColumn::Indicator),
                TableColumn::BuiltIn(PbiColumn::Type),
                TableColumn::BuiltIn(PbiColumn::Key),
                TableColumn::BuiltIn(PbiColumn::Summary),
                TableColumn::BuiltIn(PbiColumn::Status),
                TableColumn::BuiltIn(PbiColumn::Assignee),
                TableColumn::BuiltIn(PbiColumn::Age),
            ],
        }
    }

    pub fn list_view() -> Self {
        Self::sprint_view()
    }

    pub fn headers(&self) -> Vec<String> {
        self.columns.iter().map(|c| c.header()).collect()
    }

    pub fn constraints(&self) -> Vec<Constraint> {
        self.columns.iter().map(|c| c.constraint()).collect()
    }

    pub fn has_column(&self, column: PbiColumn) -> bool {
        self.columns
            .iter()
            .any(|existing| matches!(existing, TableColumn::BuiltIn(current) if *current == column))
    }

    pub fn new() -> Self {
        Self { columns: vec![] }
    }

    pub fn add_custom(&mut self, column: LuaTableColumn) {
        self.columns.push(TableColumn::Custom(column));
    }
}

impl Default for ColumnConfig {
    fn default() -> Self {
        Self::list_view()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mlua::{Function, Lua};

    fn custom_column(name: &str) -> (Lua, LuaTableColumn) {
        let lua = Lua::new();
        let func: Function = lua
            .create_function(|_, ()| Ok(String::from("value")))
            .expect("callback should be created");
        let registry_key = lua
            .create_registry_value(func)
            .expect("callback should be stored");
        (lua, LuaTableColumn::new(name.to_string(), registry_key))
    }

    // ── column configuration ──────────────────────────────────────────────────

    #[test]
    fn add_custom_appends_header_and_constraint() {
        let mut config = ColumnConfig::new();
        let (_lua, column) = custom_column("Cycle");
        config.add_custom(column);

        assert_eq!(config.headers(), vec![String::from("Cycle")]);
        assert_eq!(config.constraints(), vec![Constraint::Length(10)]);
    }
}
