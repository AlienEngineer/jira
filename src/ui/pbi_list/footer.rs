use ratatui::{
    layout::Rect,
    style::{Color, Style, Stylize},
    text::{Line, Span},
    Frame,
};

pub struct Footer {
    pub status_msg: String,
}

impl Footer {
    pub fn new() -> Self {
        Self {
            status_msg: String::new(),
        }
    }

    pub fn set_status(&mut self, msg: impl Into<String>) {
        self.status_msg = msg.into();
    }

    #[allow(dead_code)]
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
                Span::styled("j/k", Style::default().fg(Color::Yellow).bold()),
                Span::raw(" Navigate  "),
                Span::styled("l", Style::default().fg(Color::Yellow).bold()),
                Span::raw(" Detail  "),
                Span::styled("f", Style::default().fg(Color::Yellow).bold()),
                Span::raw(" Filter  "),
                Span::styled("F", Style::default().fg(Color::Yellow).bold()),
                Span::raw(" Refresh  "),
                Span::styled("q", Style::default().fg(Color::Yellow).bold()),
                Span::raw(" Quit"),
                status_span,
            ]),
            area,
        );
    }
}
