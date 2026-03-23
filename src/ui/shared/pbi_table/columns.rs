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

/// Configuration for which columns to display in the table.
#[derive(Debug, Clone)]
pub struct ColumnConfig {
    pub columns: Vec<PbiColumn>,
}

impl ColumnConfig {
    /// Sprint view configuration: 7 columns including indicator and age.
    pub fn sprint_view() -> Self {
        Self {
            columns: vec![
                PbiColumn::Indicator,
                PbiColumn::Type,
                PbiColumn::Key,
                PbiColumn::Summary,
                PbiColumn::Status,
                PbiColumn::Assignee,
                PbiColumn::Age,
            ],
        }
    }

    /// List view configuration: same columns as sprint view.
    pub fn list_view() -> Self {
        Self::sprint_view()
    }

    /// Get header labels for all columns.
    pub fn headers(&self) -> Vec<&'static str> {
        self.columns.iter().map(|c| c.header()).collect()
    }

    /// Get layout constraints for all columns.
    pub fn constraints(&self) -> Vec<Constraint> {
        self.columns.iter().map(|c| c.constraint()).collect()
    }

    /// Check if this config includes a specific column.
    pub fn has_column(&self, column: PbiColumn) -> bool {
        self.columns.contains(&column)
    }
}

impl Default for ColumnConfig {
    fn default() -> Self {
        Self::list_view()
    }
}
