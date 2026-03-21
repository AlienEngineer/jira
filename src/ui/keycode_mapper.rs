use crossterm::event::KeyCode;

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
