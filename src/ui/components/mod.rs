pub mod ui_boxed_title;
pub mod ui_layout;
pub mod ui_sprint_progress;
pub mod ui_title;
pub mod ui_widget;
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph, Wrap},
};

pub trait Component {
    fn blocked_title<'a>(title: &'a str, description: &'a str) -> Paragraph<'a>;
    fn title(name: &str) -> Span<'_>;
    fn labeled_text<'a>(label: &'a str, text: &'a str) -> Line<'a>;
}

pub struct UiComponent {}

impl Component for UiComponent {
    fn title(name: &str) -> Span<'_> {
        Span::styled(
            name,
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
    }

    fn labeled_text<'a>(label: &'a str, text: &'a str) -> Line<'a> {
        Line::from(vec![
            Span::raw(label),
            Span::styled(
                text,
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        ])
    }
    fn blocked_title<'a>(title: &'a str, description: &'a str) -> Paragraph<'a> {
        Paragraph::new(description)
            .style(Style::default().fg(Color::White).italic())
            .block(
                Block::bordered()
                    .title(Span::styled(
                        title,
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ))
                    .border_style(Style::default().fg(Color::Yellow)),
            )
            .wrap(Wrap { trim: true })
    }
}
