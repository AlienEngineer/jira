use crate::config::keymaps::Scope;
use crate::plugins::lua_plugin::get_plugins_path;
use crate::ui::keycode_mapper::keycode_to_string;
use crate::{lua::init::get_keymap_collection, ui::shared::footer::Footer};
use crossterm::event::KeyCode;
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, HighlightSpacing, List, ListItem, ListState},
    Frame,
};
use std::fs;
use std::path::PathBuf;

// ── Actions ───────────────────────────────────────────────────────────────────

pub enum PluginListAction {
    Back,
    OpenEditor(PathBuf),
}

// ── View ──────────────────────────────────────────────────────────────────────

pub struct PluginListView {
    plugins: Vec<PathBuf>,
    list_state: ListState,
    footer: Footer,
}

impl PluginListView {
    pub fn new() -> Self {
        let plugins = load_start_plugins();
        let mut list_state = ListState::default();
        if !plugins.is_empty() {
            list_state.select(Some(0));
        }
        Self {
            plugins,
            list_state,
            footer: Footer::new(vec![Scope::Global, Scope::Plugin]),
        }
    }

    // ── Key handling ──────────────────────────────────────────────────────────

    pub fn handle_key(&mut self, key: KeyCode) -> Option<PluginListAction> {
        self.handle_lua_keymaps(key);
        None
    }

    fn handle_lua_keymaps(&mut self, key: KeyCode) {
        let keycode = keycode_to_string(key);
        if let Some(collection) = get_keymap_collection() {
            let guard = collection.lock().expect("Failed to lock keymaps");
            if let Some(keymap) = guard.get_keymap(&keycode) {
                if let Err(e) = keymap.execute() {
                    eprintln!("Failed to execute keymap '{}': {}", keycode, e);
                }
            }
        }
    }

    pub fn navigate_down(&mut self) {
        if self.plugins.is_empty() {
            return;
        }
        let next = match self.list_state.selected() {
            Some(i) => (i + 1) % self.plugins.len(),
            None => 0,
        };
        self.list_state.select(Some(next));
    }

    pub fn navigate_up(&mut self) {
        if self.plugins.is_empty() {
            return;
        }
        let prev = match self.list_state.selected() {
            Some(0) | None => self.plugins.len().saturating_sub(1),
            Some(i) => i - 1,
        };
        self.list_state.select(Some(prev));
    }

    pub fn get_selected_plugin(&self) -> Option<PathBuf> {
        self.list_state
            .selected()
            .and_then(|idx| self.plugins.get(idx).cloned())
    }

    // ── Rendering ─────────────────────────────────────────────────────────────

    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        let layout = Layout::vertical([
            Constraint::Min(0),    // list
            Constraint::Length(1), // footer
        ])
        .split(area);

        self.render_list(frame, layout[0]);
        self.footer.render(frame, layout[1]);
    }

    fn render_list(&mut self, frame: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = if self.plugins.is_empty() {
            vec![ListItem::new(Span::styled(
                " No plugins found in ~/plugins/ (files must start with \"start_\")",
                Style::default().fg(Color::DarkGray),
            ))]
        } else {
            self.plugins
                .iter()
                .map(|p| {
                    let name = p
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("<unknown>");
                    ListItem::new(Span::raw(format!(" {name}")))
                })
                .collect()
        };

        let list = List::new(items)
            .block(
                Block::bordered()
                    .title(Span::styled(
                        " Plugins ",
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ))
                    .border_style(Style::default().fg(Color::DarkGray)),
            )
            .highlight_style(
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ ")
            .highlight_spacing(HighlightSpacing::Always);

        frame.render_stateful_widget(list, area, &mut self.list_state);
    }
}

impl Default for PluginListView {
    fn default() -> Self {
        Self::new()
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn load_start_plugins() -> Vec<PathBuf> {
    let plugins_path = get_plugins_path();
    let Ok(entries) = fs::read_dir(&plugins_path) else {
        return Vec::new();
    };

    let mut plugins: Vec<PathBuf> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.extension().and_then(|s| s.to_str()) == Some("lua")
                && p.file_name()
                    .and_then(|s| s.to_str())
                    .map(|n| n.starts_with("start_"))
                    .unwrap_or(false)
        })
        .collect();

    plugins.sort();
    plugins
}
