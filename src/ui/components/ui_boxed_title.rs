use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, Paragraph, Widget, Wrap},
};

use crate::ui::components::ui_widget::UiWidget;

pub struct UiBoxedTitle {
    pub title: String,
    pub content: String,
}

impl UiBoxedTitle {
    pub fn new(title: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            content: content.into(),
        }
    }
}

impl UiWidget for UiBoxedTitle {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        Paragraph::new(self.content.as_str())
            .style(Style::default().fg(Color::White).italic())
            .block(
                Block::bordered()
                    .title(Span::styled(
                        self.title.as_str(),
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ))
                    .border_style(Style::default().fg(Color::Yellow)),
            )
            .wrap(Wrap { trim: true })
            .render(area, buf);
    }

    fn get_constraint(&self) -> Constraint {
        Constraint::Length(3)
    }

    fn skip(&self) -> bool {
        self.content.is_empty()
    }
}

#[cfg(test)]
mod test {

    use ratatui::{
        buffer::Buffer,
        layout::Rect,
        style::{Color, Modifier, Style},
    };

    use crate::ui::components::{ui_boxed_title::UiBoxedTitle, ui_widget::UiWidget};

    #[derive(Debug, Default)]
    pub struct App {}

    impl App {
        fn render(&self, area: Rect, buf: &mut Buffer) {
            UiBoxedTitle::new("My Boxed Title", "This is the content of the boxed title.")
                .render(area, buf);
        }
    }

    #[test]
    fn rendering_title_renders_label_and_description() {
        let app = App::default();
        let mut buf = Buffer::empty(Rect::new(0, 0, 50, 3));

        app.render(buf.area, &mut buf);

        let mut expected = Buffer::with_lines(vec![
            "┌My Boxed Title──────────────────────────────────┐",
            "│This is the content of the boxed title.         │",
            "└────────────────────────────────────────────────┘",
        ]);

        // Row 0: border (full row), then title overlay
        expected.set_style(Rect::new(0, 0, 50, 1), get_border_style());
        expected.set_style(Rect::new(1, 0, 14, 1), get_title_style());

        // Row 1: left border, content, right border
        expected.set_style(Rect::new(0, 1, 1, 1), get_border_style());
        expected.set_style(Rect::new(1, 1, 48, 1), get_content_style());
        expected.set_style(Rect::new(49, 1, 1, 1), get_border_style());

        // Row 2: border (full row)
        expected.set_style(Rect::new(0, 2, 50, 1), get_border_style());

        assert_eq!(buf, expected);
    }

    fn get_content_style() -> Style {
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::ITALIC)
    }

    fn get_title_style() -> Style {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD | Modifier::ITALIC)
    }

    fn get_border_style() -> Style {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::ITALIC)
    }
}
