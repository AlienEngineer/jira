use crate::jira::pbi::Pbi;
use crossterm::event::KeyCode;
use ratatui::{
    layout::{Alignment, Constraint, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Cell, Row, Table, TableState},
    Frame,
};

// ── Actions returned to PbiListApp ────────────────────────────────────────────

pub enum IssueTableAction {
    OpenDetail(Box<Pbi>),
    Quit,
}

// ── IssueTable ────────────────────────────────────────────────────────────────

pub struct IssueTable {
    pub table_state: TableState,
}

impl IssueTable {
    pub fn new(issue_count: usize) -> Self {
        let mut table_state = TableState::default();
        if issue_count > 0 {
            table_state.select(Some(0));
        }
        Self { table_state }
    }

    pub fn reset_selection(&mut self, issue_count: usize) {
        if issue_count > 0 {
            self.table_state.select(Some(0));
        } else {
            self.table_state.select(None);
        }
    }

    // ── Navigation ────────────────────────────────────────────────────────────

    fn navigate_down(&mut self, count: usize) {
        let next = self.table_state.selected().map_or(0, |i| {
            if i >= count.saturating_sub(1) {
                0
            } else {
                i + 1
            }
        });
        self.table_state.select(Some(next));
    }

    fn navigate_up(&mut self, count: usize) {
        let prev = self.table_state.selected().map_or(0, |i| {
            if i == 0 {
                count.saturating_sub(1)
            } else {
                i - 1
            }
        });
        self.table_state.select(Some(prev));
    }

    // ── Key handling ──────────────────────────────────────────────────────────

    pub fn handle_key(&mut self, key: KeyCode, issues: &[Pbi]) -> Option<IssueTableAction> {
        match key {
            KeyCode::Down | KeyCode::Char('j') => {
                self.navigate_down(issues.len());
                None
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.navigate_up(issues.len());
                None
            }
            KeyCode::Right | KeyCode::Char('l') | KeyCode::Enter => {
                if let Some(i) = self.table_state.selected() {
                    if let Some(pbi) = issues.get(i) {
                        return Some(IssueTableAction::OpenDetail(Box::new(pbi.clone())));
                    }
                }
                None
            }
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => Some(IssueTableAction::Quit),
            _ => None,
        }
    }

    // ── Rendering ─────────────────────────────────────────────────────────────

    pub fn render(&mut self, frame: &mut Frame, area: Rect, issues: &[Pbi]) {
        let header = Row::new(
            ["Type", "Key", "Summary", "Status", "Assignee"]
                .iter()
                .map(|h| {
                    Cell::from(*h).style(
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    )
                }),
        )
        .style(Style::default().bg(Color::DarkGray))
        .height(1);

        let rows: Vec<Row> = issues
            .iter()
            .map(|pbi| {
                let status_style = status_color(&pbi.status);
                Row::new(vec![
                    Cell::from(pbi.issue_type.clone()).style(Style::default().fg(Color::DarkGray)),
                    Cell::from(pbi.key.clone()).style(Style::default().fg(Color::Cyan)),
                    Cell::from(pbi.summary.clone()),
                    Cell::from(pbi.status.clone()).style(status_style),
                    Cell::from(pbi.assignee.clone()),
                ])
            })
            .collect();

        let table = Table::new(
            rows,
            [
                Constraint::Length(12),
                Constraint::Length(12),
                Constraint::Min(40),
                Constraint::Length(18),
                Constraint::Length(20),
            ],
        )
        .header(header)
        .block(
            Block::bordered()
                .title(format!(" {} issues ", issues.len()))
                .title_alignment(Alignment::Right),
        )
        .row_highlight_style(
            Style::default()
                .bg(Color::Blue)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

        frame.render_stateful_widget(table, area, &mut self.table_state);
    }
}

fn status_color(status: &str) -> Style {
    match status.to_lowercase().as_str() {
        s if s.contains("done") || s.contains("closed") => Style::default().fg(Color::Green),
        s if s.contains("progress") => Style::default().fg(Color::Blue),
        s if s.contains("review") => Style::default().fg(Color::Magenta),
        s if s.contains("blocked") => Style::default().fg(Color::Red),
        s if s.contains("resolved") => Style::default().fg(Color::Green),
        _ => Style::default().fg(Color::White),
    }
}
