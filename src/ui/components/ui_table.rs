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
        shared::pbi_table::{status_color, ColumnConfig, PbiColumn},
    },
};

pub struct UiTable {
    pub table_state: TableState,
    column_config: ColumnConfig,
    loading_idx: Option<usize>,
    pub pbis: Vec<Pbi>,
}

impl UiTable {
    pub fn new(column_config: ColumnConfig, pbis: Vec<Pbi>) -> Self {
        Self {
            table_state: TableState::default(),
            column_config,
            loading_idx: None,
            pbis,
        }
    }

    fn build_table_widget(&self) -> Table<'static> {
        let header = Row::new(self.column_config.headers().iter().map(|h| {
            Cell::from(*h).style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
        }))
        .style(Style::default().bg(Color::DarkGray))
        .height(1);

        let rows: Vec<Row> = self
            .pbis
            .iter()
            .enumerate()
            .map(|(idx, pbi)| self.build_row(idx, pbi))
            .collect();

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

    fn build_row(&self, idx: usize, pbi: &Pbi) -> Row<'static> {
        let cells: Vec<Cell> = self
            .column_config
            .columns
            .iter()
            .map(|col| self.build_cell(*col, idx, pbi))
            .collect();
        Row::new(cells)
    }

    fn build_cell(&self, column: PbiColumn, idx: usize, pbi: &Pbi) -> Cell<'static> {
        match column {
            PbiColumn::Indicator => {
                if self.loading_idx == Some(idx) {
                    Cell::from("⟳").style(
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    )
                } else if pbi.loaded {
                    Cell::from("✓").style(Style::default().fg(Color::Green))
                } else {
                    Cell::from(" ")
                }
            }
            PbiColumn::Type => {
                Cell::from(pbi.issue_type.clone()).style(Style::default().fg(Color::DarkGray))
            }
            PbiColumn::Key => Cell::from(pbi.key.clone()).style(Style::default().fg(Color::Cyan)),
            PbiColumn::Summary => Cell::from(pbi.summary.clone()),
            PbiColumn::Status => Cell::from(pbi.status.clone()).style(status_color(&pbi.status)),
            PbiColumn::Assignee => Cell::from(pbi.assignee.clone()),
            PbiColumn::Age => {
                Cell::from(pbi_elapsed_display(pbi)).style(Style::default().fg(Color::DarkGray))
            }
            PbiColumn::Priority => {
                let priority = pbi.priority.clone().unwrap_or_default();
                Cell::from(priority)
            }
        }
    }
}

impl UiWidget for UiTable {
    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let this = &self;
        let header = Row::new(this.column_config.headers().iter().map(|h| {
            Cell::from(*h).style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
        }))
        .style(Style::default().bg(Color::DarkGray))
        .height(1);

        let rows: Vec<Row> = this
            .pbis
            .iter()
            .enumerate()
            .map(|(idx, pbi)| this.build_row(idx, pbi))
            .collect();

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
        Constraint::Length(1)
    }

    fn skip(&self) -> bool {
        false
    }

    fn render_widget(&mut self, frame: &mut Frame, area: Rect) {
        let table_widget = self.build_table_widget();
        frame.render_stateful_widget(table_widget, area, &mut self.table_state);
    }
}
