use ratatui::{
    layout::Rect,
    style::{Color, Style, Stylize},
    text::{Line, Span},
    Frame,
};

/// Renders the key-binding hint bar and a status message.
///
/// Owns the current status string; updated by `SprintApp` after it processes
/// `TableAction`s returned by `SprintTable`.
pub struct Footer {
    pub status_msg: String,
}

impl Footer {
    pub fn new() -> Self {
        Self { status_msg: String::new() }
    }

    pub fn set_status(&mut self, msg: impl Into<String>) {
        self.status_msg = msg.into();
    }

    pub fn clear_status(&mut self) {
        self.status_msg.clear();
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let status_span = if self.status_msg.is_empty() {
            Span::raw("")
        } else {
            Span::styled(
                format!("  [{}]", self.status_msg),
                Style::default().fg(Color::Green),
            )
        };

        frame.render_widget(
            Line::from(vec![
                Span::raw(" "),
                Span::styled("↵", Style::default().fg(Color::Yellow).bold()),
                Span::raw(" Start  "),
                Span::styled("k/j", Style::default().fg(Color::Yellow).bold()),
                Span::raw(" Navigate  "),
                Span::styled("f", Style::default().fg(Color::Yellow).bold()),
                Span::raw(" Load line  "),
                Span::styled("F", Style::default().fg(Color::Yellow).bold()),
                Span::raw(" Load all  "),
                Span::styled("q", Style::default().fg(Color::Yellow).bold()),
                Span::raw(" Quit"),
                status_span,
            ]),
            area,
        );
    }
}
