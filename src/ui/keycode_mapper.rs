use crossterm::event::KeyCode;
use ratatui::{
    style::{Color, Style},
    text::Span,
};

use crate::{config::keymaps::Scope, lua::init::get_keymap_collection};

pub fn generate_keymaps(spans: &mut Vec<Span<'_>>, scopes: Vec<Scope>) {
    if let Some(collection) = get_keymap_collection() {
        let guard = collection.lock().expect("Failed to lock keymaps");
        let keymaps = guard.get_keymaps();
        let plugin_spans: Vec<Span> = keymaps
            .iter()
            .filter(|k| k.description.is_some())
            .filter(|k| k.scope.is_in(&scopes))
            .flat_map(|k| {
                [
                    Span::styled(k.key.clone(), Style::default().fg(Color::Yellow).bold()),
                    Span::raw(format!(" {}  ", k.description.as_ref().unwrap())),
                ]
            })
            .collect();

        spans.extend(plugin_spans);
    }
}

pub fn keycode_to_string(key: KeyCode) -> String {
    match key {
        KeyCode::Char(c) => c.to_string(),
        KeyCode::Enter => "<CR>".to_string(),
        KeyCode::Esc => "<ESC>".to_string(),
        KeyCode::Tab => "<TAB>".to_string(),
        KeyCode::Backspace => "<BACKSPACE>".to_string(),
        KeyCode::Left => "<LEFT>".to_string(),
        KeyCode::Right => "<RIGHT>".to_string(),
        KeyCode::Up => "<UP>".to_string(),
        KeyCode::Down => "<DOWN>".to_string(),
        KeyCode::Delete => "<DELETE>".to_string(),
        _ => format!("{:?}", key),
    }
}
