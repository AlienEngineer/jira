use crate::jira::pbi::{pbi_elapsed_display, Pbi};
use crossterm::event::KeyCode;
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{
        Block, Cell, Paragraph, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, Table, Wrap,
    },
    Frame,
};

// ── Actions ───────────────────────────────────────────────────────────────────

pub enum PbiDetailAction {
    Back,
    ShowRaw,
}

// ── View ──────────────────────────────────────────────────────────────────────

pub struct PbiDetailView {
    pub pbi: Pbi,
    desc_scroll: u16,
}

impl PbiDetailView {
    pub fn new(pbi: Pbi) -> Self {
        Self {
            pbi,
            desc_scroll: 0,
        }
    }

    // ── Key handling ──────────────────────────────────────────────────────────

    pub fn handle_key(&mut self, key: KeyCode) -> Option<PbiDetailAction> {
        match key {
            KeyCode::Left | KeyCode::Char('h') | KeyCode::Esc => Some(PbiDetailAction::Back),
            KeyCode::Char('r') | KeyCode::Char('R') => Some(PbiDetailAction::ShowRaw),
            KeyCode::Down | KeyCode::Char('j') => {
                self.desc_scroll = self.desc_scroll.saturating_add(1);
                None
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.desc_scroll = self.desc_scroll.saturating_sub(1);
                None
            }
            _ => None,
        }
    }

    // ── Rendering ─────────────────────────────────────────────────────────────

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        // ┌──────────────────────────────────────────────────────┐
        // │  header bar (title + status + age)          1 line   │
        // │  metadata block (key info, labels, …)       7 lines  │
        // │  description block                          rest     │
        // │  footer / key hints                         1 line   │
        // └──────────────────────────────────────────────────────┘
        let layout = Layout::vertical([
            Constraint::Length(1), // header bar
            Constraint::Length(9), // metadata block
            Constraint::Min(3),    // description
            Constraint::Length(1), // footer
        ])
        .split(area);

        self.render_header(frame, layout[0]);
        self.render_metadata(frame, layout[1]);
        self.render_description(frame, layout[2]);
        self.render_footer(frame, layout[3]);
    }

    // ── Header bar ────────────────────────────────────────────────────────────

    fn render_header(&self, frame: &mut Frame, area: Rect) {
        let status_color = status_color(&self.pbi.status);
        let age = pbi_elapsed_display(&self.pbi);
        let age_span = if age.is_empty() {
            Span::raw("")
        } else {
            Span::styled(format!("  ⏱ {age}"), Style::default().fg(Color::Cyan))
        };

        frame.render_widget(
            Line::from(vec![
                Span::styled(
                    format!(" {} ", self.pbi.key),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("│ "),
                Span::styled(
                    self.pbi.issue_type.clone(),
                    Style::default().fg(Color::White),
                ),
                Span::raw("  │  "),
                Span::styled(
                    self.pbi.status.clone(),
                    Style::default()
                        .fg(status_color)
                        .add_modifier(Modifier::BOLD),
                ),
                age_span,
            ]),
            area,
        );
    }

    // ── Metadata block ────────────────────────────────────────────────────────

    fn render_metadata(&self, frame: &mut Frame, area: Rect) {
        let cols = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        self.render_left_meta(frame, cols[0]);
        self.render_right_meta(frame, cols[1]);
    }

    fn render_left_meta(&self, frame: &mut Frame, area: Rect) {
        let summary = self.pbi.summary.clone();
        let assignee = self.pbi.assignee.clone();
        let priority = self.pbi.priority.clone().unwrap_or_else(|| "—".to_string());
        let story_pts = self
            .pbi
            .story_points
            .map(|sp| format!("{sp}"))
            .unwrap_or_else(|| "—".to_string());

        let rows = vec![
            Row::new(vec![
                Cell::from("Summary").style(Style::default().fg(Color::Yellow)),
                Cell::from(summary),
            ]),
            Row::new(vec![
                Cell::from("Assignee").style(Style::default().fg(Color::Yellow)),
                Cell::from(assignee),
            ]),
            Row::new(vec![
                Cell::from("Priority").style(Style::default().fg(Color::Yellow)),
                Cell::from(priority).style(
                    Style::default().fg(priority_color(self.pbi.priority.as_deref().unwrap_or(""))),
                ),
            ]),
            Row::new(vec![
                Cell::from("Story Pts").style(Style::default().fg(Color::Yellow)),
                Cell::from(story_pts),
            ]),
        ];

        frame.render_widget(
            Table::new(rows, [Constraint::Length(10), Constraint::Min(0)])
                .block(
                    Block::bordered()
                        .title(Span::styled(
                            " Details ",
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::BOLD),
                        ))
                        .border_style(Style::default().fg(Color::DarkGray)),
                )
                .column_spacing(1),
            area,
        );
    }

    fn render_right_meta(&self, frame: &mut Frame, area: Rect) {
        let labels = if self.pbi.labels.is_empty() {
            "—".to_string()
        } else {
            self.pbi.labels.join(", ")
        };

        let in_progress = self
            .pbi
            .in_progress_at
            .as_deref()
            .map(format_timestamp)
            .unwrap_or_else(|| "—".to_string());

        let resolved = self
            .pbi
            .resolved_at
            .as_deref()
            .map(format_timestamp)
            .unwrap_or_else(|| "—".to_string());

        let rows = vec![
            Row::new(vec![
                Cell::from("Labels").style(Style::default().fg(Color::Yellow)),
                Cell::from(labels),
            ]),
            Row::new(vec![
                Cell::from("In Progress").style(Style::default().fg(Color::Yellow)),
                Cell::from(in_progress).style(Style::default().fg(Color::Blue)),
            ]),
            Row::new(vec![
                Cell::from("Resolved").style(Style::default().fg(Color::Yellow)),
                Cell::from(resolved).style(Style::default().fg(Color::Green)),
            ]),
        ];

        frame.render_widget(
            Table::new(rows, [Constraint::Length(11), Constraint::Min(0)])
                .block(
                    Block::bordered()
                        .title(Span::styled(
                            " Timestamps ",
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::BOLD),
                        ))
                        .border_style(Style::default().fg(Color::DarkGray)),
                )
                .column_spacing(1),
            area,
        );
    }

    // ── Description block ─────────────────────────────────────────────────────

    fn render_description(&self, frame: &mut Frame, area: Rect) {
        let body = self
            .pbi
            .description
            .as_deref()
            .unwrap_or("No description loaded. Press  f  to fetch details.");

        // Reserve space for the scrollbar on the right edge.
        let inner_layout =
            Layout::horizontal([Constraint::Min(0), Constraint::Length(1)]).split(area);

        let para = Paragraph::new(body)
            .style(Style::default().fg(Color::White))
            .block(
                Block::bordered()
                    .title(Span::styled(
                        " Description ",
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ))
                    .border_style(Style::default().fg(Color::DarkGray)),
            )
            .wrap(Wrap { trim: false })
            .scroll((self.desc_scroll, 0));

        frame.render_widget(para, inner_layout[0]);

        // Approximate content line count for the scrollbar.
        let content_lines = body.lines().count() as u16;
        let visible_height = inner_layout[0].height.saturating_sub(2);
        let max_scroll = content_lines.saturating_sub(visible_height);

        let mut scrollbar_state =
            ScrollbarState::new(max_scroll as usize).position(self.desc_scroll as usize);

        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight),
            inner_layout[1],
            &mut scrollbar_state,
        );
    }

    // ── Footer ────────────────────────────────────────────────────────────────

    fn render_footer(&self, frame: &mut Frame, area: Rect) {
        frame.render_widget(
            Line::from(vec![
                Span::raw(" "),
                Span::styled("h", Style::default().fg(Color::Yellow).bold()),
                Span::raw(" Back  "),
                Span::styled("j/k", Style::default().fg(Color::Yellow).bold()),
                Span::raw(" Scroll  "),
                Span::styled("r", Style::default().fg(Color::Yellow).bold()),
                Span::raw(" Raw in editor  "),
            ]),
            area,
        );
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn status_color(status: &str) -> Color {
    match status.to_lowercase().as_str() {
        s if s.contains("done") || s.contains("closed") => Color::Green,
        s if s.contains("progress") => Color::Blue,
        s if s.contains("review") => Color::Magenta,
        s if s.contains("blocked") => Color::Red,
        s if s.contains("resolved") => Color::Green,
        _ => Color::White,
    }
}

fn priority_color(priority: &str) -> Color {
    match priority.to_lowercase().as_str() {
        "highest" | "critical" => Color::Red,
        "high" => Color::LightRed,
        "medium" => Color::Yellow,
        "low" => Color::LightBlue,
        "lowest" => Color::DarkGray,
        _ => Color::White,
    }
}

/// Trim an ISO-8601 Jira timestamp to a readable `YYYY-MM-DD HH:MM` form.
fn format_timestamp(ts: &str) -> String {
    // Jira format: "2026-03-08T09:15:00.000+0000"
    // Take the first 16 characters → "2026-03-08T09:15" then replace 'T' with ' '.
    let trimmed = &ts[..ts.len().min(16)];
    trimmed.replace('T', " ")
}
