use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    Frame,
};

use crate::lua::init::get_keymap_collection;

/// Renders the key-binding hint bar and a status message.
///
/// Owns the current status string; updated by `SprintApp` after it processes
/// `TableAction`s returned by `SprintTable`.
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

        let keymaps = Vec::from([
            ("↵", "Start"),
            ("f", "Load line"),
            ("F", "Load all"),
            ("o", "Browser"),
            ("q", "Quit"),
        ]);

        let len = keymaps.len();
        let mut spans: Vec<Span> = std::iter::once(Span::raw(" "))
            .chain(keymaps.iter().enumerate().flat_map(|(i, (key, desc))| {
                let suffix = if i == len - 1 { " " } else { "  " };
                [
                    Span::styled(*key, Style::default().fg(Color::Yellow).bold()),
                    Span::raw(
                        format!(" {} {}", desc, suffix).trim_end().to_string()
                            + if i == len - 1 { "" } else { "  " },
                    ),
                ]
            }))
            .collect();

        append_key_maps(&mut spans);

        spans.push(status_span);
        frame.render_widget(Line::from(spans), area);
    }
}

// TODO: duplicated code with pbi_list/footer.rs
fn append_key_maps(spans: &mut Vec<Span<'_>>) {
    if let Some(collection) = get_keymap_collection() {
        let guard = collection.lock().expect("Failed to lock keymaps");
        let keymaps = guard.get_keymaps();
        let plugin_spans: Vec<Span> = keymaps
            .iter()
            .flat_map(|k| {
                [
                    Span::styled(k.key.clone(), Style::default().fg(Color::Cyan).bold()),
                    Span::raw(format!(" {}  ", k.description)),
                ]
            })
            .collect();

        spans.push(Span::raw("  ")); // Add spacing before plugin keymaps
        spans.extend(plugin_spans);
    }
}
