use ratatui::{
    buffer::Buffer,
    layout::{Layout, Rect},
};

use crate::ui::components::ui_widget::UiWidget;

pub struct UiLayout {
    widgets: Vec<Box<dyn UiWidget>>,
}

impl UiLayout {
    pub fn new() -> Self {
        Self {
            widgets: Vec::new(),
        }
    }

    pub fn add_widget(&mut self, widget: Box<dyn UiWidget>) {
        self.widgets.push(widget);
    }

    fn visible_widgets(&self) -> impl Iterator<Item = &dyn UiWidget> {
        self.widgets
            .iter()
            .map(|w| w.as_ref())
            .filter(|w| !w.skip())
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        let layout =
            Layout::vertical(self.visible_widgets().map(|w| w.get_constraint())).split(area);

        for (i, widget) in self.visible_widgets().enumerate() {
            widget.render(layout[i], buf);
        }
    }
}

impl Default for UiLayout {
    fn default() -> Self {
        Self::new()
    }
}
