use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Rect},
};

pub trait UiWidget {
    fn render(&self, area: Rect, buf: &mut Buffer);
    fn get_constraint(&self) -> Constraint;
    fn skip(&self) -> bool;
}
