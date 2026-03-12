use crate::plugins::lua_plugin::get_plugins_path;
use crossterm::event::KeyCode;
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, HighlightSpacing, List, ListItem, ListState},
    Frame,
};
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
        }
    }

    // ── Key handling ──────────────────────────────────────────────────────────

    pub fn handle_key(&mut self, key: KeyCode) -> Option<PluginListAction> {
        match key {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('h') | KeyCode::Left => {
                Some(PluginListAction::Back)
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.navigate_down();
                None
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.navigate_up();
                None
            }
            KeyCode::Enter => {
                if let Some(idx) = self.list_state.selected() {
                    if let Some(path) = self.plugins.get(idx) {
                        return Some(PluginListAction::OpenEditor(path.clone()));
                    }
                }
                None
            }
            _ => None,
        }
    }

    fn navigate_down(&mut self) {
        if self.plugins.is_empty() {
            return;
        }
        let next = match self.list_state.selected() {
            Some(i) => (i + 1) % self.plugins.len(),
            None => 0,
        };
        self.list_state.select(Some(next));
    }

    fn navigate_up(&mut self) {
        if self.plugins.is_empty() {
            return;
        }
        let prev = match self.list_state.selected() {
            Some(0) | None => self.plugins.len().saturating_sub(1),
            Some(i) => i - 1,
        };
        self.list_state.select(Some(prev));
    }

    // ── Rendering ─────────────────────────────────────────────────────────────

    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        let layout = Layout::vertical([
            Constraint::Min(0),    // list
            Constraint::Length(1), // footer
        ])
        .split(area);

        self.render_list(frame, layout[0]);
        self.render_footer(frame, layout[1]);
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

    fn render_footer(&self, frame: &mut Frame, area: Rect) {
        frame.render_widget(
            Line::from(vec![
                Span::raw(" "),
                Span::styled("↵", Style::default().fg(Color::Yellow).bold()),
                Span::raw(" Open in editor  "),
                Span::styled("j/k", Style::default().fg(Color::Yellow).bold()),
                Span::raw(" Navigate  "),
                Span::styled("h/Esc", Style::default().fg(Color::Yellow).bold()),
                Span::raw(" Back  "),
            ]),
            area,
        );
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn load_start_plugins() -> Vec<PathBuf> {
    let plugins_path = get_plugins_path();
    let Ok(entries) = std::fs::read_dir(&plugins_path) else {
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
