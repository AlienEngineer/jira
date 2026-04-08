use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Cell, Row, StatefulWidget, Table, TableState},
    Frame,
};

use crate::{
    jira::pbi::{pbi_elapsed_display, Pbi},
    ui::{
        components::ui_widget::UiWidget,
        shared::pbi_table::{ColumnConfig, PbiColumn, TableColumn},
    },
};

#[derive(Clone)]
pub struct UiTable {
    table_state: TableState,
    column_config: ColumnConfig,
    pbis: Vec<Pbi>,
}

impl UiTable {
    pub fn new(pbis: Vec<Pbi>) -> Self {
        Self {
            table_state: TableState::default().with_selected(Some(0)),
            column_config: ColumnConfig::new(),
            pbis,
        }
    }

    pub fn set_columns(&mut self, column_config: ColumnConfig) {
        self.column_config = column_config;
    }

    pub fn add_column(&mut self, column: crate::lua::init::TableColumn) {
        self.column_config.add_custom(column);
    }

    pub fn selected_index(&self) -> Option<usize> {
        self.table_state.selected()
    }

    pub fn selected<'a>(&self, pbis: &'a [Pbi]) -> Option<&'a Pbi> {
        self.table_state.selected().and_then(|i| pbis.get(i))
    }

    pub fn reset_selection(&mut self) {
        if !self.pbis.is_empty() {
            self.table_state.select(Some(0));
        } else {
            self.table_state.select(None);
        }
    }

    pub(crate) fn load(&mut self, pbis: Vec<Pbi>) {
        self.pbis = pbis;
        if self.table_state.selected().is_none() {
            self.reset_selection();
        }
    }

    pub fn navigate_down(&mut self) {
        let next = self.table_state.selected().map_or(0, |i| {
            if i >= self.pbis.len().saturating_sub(1) {
                0
            } else {
                i + 1
            }
        });
        self.table_state.select(Some(next));
    }

    pub fn navigate_up(&mut self) {
        let prev = self.table_state.selected().map_or(0, |i| {
            if i == 0 {
                self.pbis.len().saturating_sub(1)
            } else {
                i - 1
            }
        });
        self.table_state.select(Some(prev));
    }

    fn build_table_widget(&self) -> Table<'static> {
        let header = Row::new(self.column_config.headers().iter().map(|h| {
            Cell::from(h.clone()).style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
        }))
        .style(Style::default().bg(Color::DarkGray))
        .height(1);

        let rows: Vec<Row> = self.pbis.iter().map(|pbi| self.build_row(pbi)).collect();

        Table::new(rows, self.column_config.constraints())
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
            .highlight_symbol(">> ")
    }

    fn build_row(&self, pbi: &Pbi) -> Row<'static> {
        let cells: Vec<Cell> = self
            .column_config
            .columns
            .iter()
            .map(|col| self.build_cell(col, pbi))
            .collect();
        Row::new(cells)
    }

    fn build_cell(&self, column: &TableColumn, pbi: &Pbi) -> Cell<'static> {
        match column {
            TableColumn::BuiltIn(PbiColumn::Indicator) => Cell::from(" "),
            TableColumn::BuiltIn(PbiColumn::Type) => {
                Cell::from(pbi.issue_type.clone()).style(Style::default().fg(Color::DarkGray))
            }
            TableColumn::BuiltIn(PbiColumn::Key) => {
                Cell::from(pbi.key.clone()).style(Style::default().fg(Color::Cyan))
            }
            TableColumn::BuiltIn(PbiColumn::Summary) => Cell::from(pbi.summary.clone()),
            TableColumn::BuiltIn(PbiColumn::Status) => {
                Cell::from(pbi.status.clone()).style(self.status_color(&pbi.status))
            }
            TableColumn::BuiltIn(PbiColumn::Assignee) => Cell::from(pbi.assignee.clone()),
            TableColumn::BuiltIn(PbiColumn::Age) => {
                Cell::from(pbi_elapsed_display(pbi)).style(Style::default().fg(Color::DarkGray))
            }
            TableColumn::BuiltIn(PbiColumn::Priority) => {
                let priority = pbi.priority.clone().unwrap_or_default();
                Cell::from(priority)
            }
            TableColumn::Custom(column) => match column.value_for(pbi, self.pbis.clone()) {
                Ok(value) => Cell::from(value),
                Err(error) => {
                    println!("Error evaluating column {}", error);

                    Cell::from(format!("error: {error}"))
                }
            },
        }
    }

    fn status_color(&self, status: &str) -> Style {
        match status.to_lowercase().as_str() {
            s if s.contains("done") || s.contains("closed") => Style::default().fg(Color::Green),
            s if s.contains("progress") => Style::default().fg(Color::Blue),
            s if s.contains("review") => Style::default().fg(Color::Magenta),
            s if s.contains("blocked") => Style::default().fg(Color::Red),
            s if s.contains("resolved") => Style::default().fg(Color::Green),
            _ => Style::default().fg(Color::White),
        }
    }
}

impl UiWidget for UiTable {
    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let this = &self;
        let header = Row::new(this.column_config.headers().iter().map(|h| {
            Cell::from(h.clone()).style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
        }))
        .style(Style::default().bg(Color::DarkGray))
        .height(1);

        let rows: Vec<Row> = this.pbis.iter().map(|pbi| this.build_row(pbi)).collect();

        Table::new(rows, this.column_config.constraints())
            .header(header)
            .block(
                Block::bordered()
                    .title(format!(" {} items ", this.pbis.len()))
                    .title_alignment(Alignment::Right),
            )
            .row_highlight_style(
                Style::default()
                    .bg(Color::Blue)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(">> ")
            .render(area, buf, &mut self.table_state);
    }

    fn get_constraint(&self) -> Constraint {
        Constraint::Min(0)
    }

    fn skip(&self) -> bool {
        false
    }

    fn render_widget(&mut self, frame: &mut Frame, area: Rect) {
        frame.render_stateful_widget(self.build_table_widget(), area, &mut self.table_state);
    }
}
