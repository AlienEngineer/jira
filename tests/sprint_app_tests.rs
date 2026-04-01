//! User interaction tests for SprintApp
//!
//! These tests simulate actual user key presses and verify the resulting behavior.
//! Tests run the app as it would in runtime, using real Lua keymaps with only
//! the Jira API being faked.
//!
//! Pattern (from ratatui tutorial):
//! ```ignore
//! app.handle_key_event(KeyCode::Char('j'));
//! assert_eq!(app.selected_pbi().unwrap().key, "TEST-2");
//! ```
//!
//! **Note:** Tests must run with `--test-threads=1` because they share a global
//! command channel for Lua keymaps.
//!
//! Default keymaps tested:
//! - j/k: Navigate down/up
//! - l/h: Open detail view / go back  
//! - q/ESC: Quit
//! - f: Refresh selected PBI
//! - r: View raw JSON
//! - o: Open in browser

use crossterm::event::KeyCode;
use jira::jira::api::JiraApi;
use jira::jira::pbi::Pbi;
use jira::jira::sprint::{Sprint, SprintService};
use jira::ui::sprint_list::SprintApp;
use std::error::Error;
use std::sync::{Arc, Mutex, Once};

// ── Test serialization ────────────────────────────────────────────────────────
// The Lua command channel is global, so tests must not run in parallel.

static TEST_MUTEX: Mutex<()> = Mutex::new(());
static INIT_LUA: Once = Once::new();

fn ensure_lua_initialized() {
    INIT_LUA.call_once(|| {
        jira::lua::init::init_lua_config().expect("Failed to initialize Lua config");
    });
}

// ── Mock implementations ──────────────────────────────────────────────────────

struct MockJiraApi;

impl JiraApi for MockJiraApi {
    fn get(&self, _endpoint: &str, _version: u8) -> Result<json::JsonValue, Box<dyn Error>> {
        Ok(json::object! {})
    }

    fn post(
        &self,
        _endpoint: &str,
        _json_value: json::JsonValue,
        _version: u8,
    ) -> Result<String, Box<dyn Error>> {
        Ok(String::new())
    }

    fn put(
        &self,
        _endpoint: &str,
        _json_value: json::JsonValue,
        _version: u8,
    ) -> Result<String, Box<dyn Error>> {
        Ok(String::new())
    }

    fn get_agile(&self, _endpoint: &str) -> Result<json::JsonValue, Box<dyn Error>> {
        Ok(json::object! {})
    }
}

struct MockSprintService {
    api: Arc<MockJiraApi>,
}

impl MockSprintService {
    fn new() -> Self {
        Self {
            api: Arc::new(MockJiraApi),
        }
    }
}

impl SprintService for MockSprintService {
    fn fetch_active_sprint_issues(&self, _board_id: &str) -> Result<Sprint, Box<dyn Error>> {
        Ok(Sprint {
            name: "Test Sprint".to_string(),
            goal: "Test Goal".to_string(),
            end_date: "2024-12-31".to_string(),
            pbis: vec![],
            board_id: "123".to_string(),
        })
    }

    fn set_sprint_as_achieved(&self) -> Result<bool, Box<dyn Error>> {
        Ok(true)
    }

    fn jira_api(&self) -> &dyn JiraApi {
        self.api.as_ref()
    }
}

// ── Test fixtures ─────────────────────────────────────────────────────────────

fn make_pbi(key: &str, summary: &str, status: &str, assignee: &str) -> Pbi {
    Pbi {
        key: key.to_string(),
        summary: summary.to_string(),
        status: status.to_string(),
        assignee: assignee.to_string(),
        issue_type: "Story".to_string(),
        description: None,
        priority: Some("Medium".to_string()),
        story_points: Some(3.0),
        labels: vec![],
        loaded: false,
        in_progress_at: None,
        resolved_at: None,
        raw: String::new(),
        resolution: None,
        components: vec![],
        creator: String::new(),
        reporter: String::new(),
        project: "TEST".to_string(),
    }
}

fn sample_sprint() -> Sprint {
    Sprint {
        name: "Sprint 42".to_string(),
        goal: "Complete user authentication".to_string(),
        end_date: "2024-12-31".to_string(),
        pbis: vec![
            make_pbi("TEST-101", "Login form", "In Progress", "Alice"),
            make_pbi("TEST-102", "Password reset", "Open", "Bob"),
            make_pbi("TEST-103", "Session timeout", "Blocked", "Carol"),
            make_pbi("TEST-104", "Remember me", "In Review", "Dave"),
            make_pbi("TEST-105", "Logout button", "Closed", "Eve"),
        ],
        board_id: "123".to_string(),
    }
}

fn create_test_app() -> SprintApp {
    ensure_lua_initialized();
    // Reset the command channel so each test gets a fresh receiver
    jira::lua::init::reset_command_channel();
    let sprint = sample_sprint();
    let service = Arc::new(MockSprintService::new());
    SprintApp::new(sprint, service)
}

/// Run a test with exclusive access to the command channel.
fn run_test<F>(test_fn: F)
where
    F: FnOnce(),
{
    let _guard = TEST_MUTEX.lock().unwrap();
    test_fn();
}

// ══════════════════════════════════════════════════════════════════════════════
// SPRINT METADATA TESTS
// ══════════════════════════════════════════════════════════════════════════════

mod sprint_metadata {
    use super::*;

    #[test]
    fn sprint_has_correct_end_date() {
        run_test(|| {
            let app = create_test_app();

            assert_eq!(
                app.sprint_end_date(),
                "2024-12-31",
                "sprint should have the correct end date"
            );
        });
    }

    #[test]
    fn sprint_has_correct_number_of_pbis() {
        run_test(|| {
            let app = create_test_app();

            assert_eq!(app.pbis().len(), 5, "sprint should have 5 PBIs");
        });
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// USER INTERACTION TESTS: Navigation
// ══════════════════════════════════════════════════════════════════════════════

mod navigation {
    use super::*;

    #[test]
    fn pressing_j_selects_item_below() {
        run_test(|| {
            let mut app = create_test_app();
            assert_eq!(app.selected_pbi().unwrap().key, "TEST-101");

            app.handle_key_event(KeyCode::Char('j'));

            assert_eq!(
                app.selected_pbi().unwrap().key,
                "TEST-102",
                "pressing 'j' should select the item below"
            );
        });
    }

    #[test]
    fn pressing_j_multiple_times_navigates_down() {
        run_test(|| {
            let mut app = create_test_app();

            app.handle_key_event(KeyCode::Char('j'));
            app.handle_key_event(KeyCode::Char('j'));
            app.handle_key_event(KeyCode::Char('j'));

            assert_eq!(
                app.selected_pbi().unwrap().key,
                "TEST-104",
                "navigating down 3 times should reach 4th item"
            );
        });
    }

    #[test]
    fn pressing_k_selects_item_above() {
        run_test(|| {
            let mut app = create_test_app();
            // Navigate to third item first
            app.handle_key_event(KeyCode::Char('j'));
            app.handle_key_event(KeyCode::Char('j'));
            assert_eq!(app.selected_pbi().unwrap().key, "TEST-103");

            app.handle_key_event(KeyCode::Char('k'));

            assert_eq!(
                app.selected_pbi().unwrap().key,
                "TEST-102",
                "pressing 'k' should select the item above"
            );
        });
    }

    #[test]
    fn pressing_down_arrow_navigates() {
        run_test(|| {
            let mut app = create_test_app();

            app.handle_key_event(KeyCode::Down);
            app.handle_key_event(KeyCode::Down);

            assert_eq!(
                app.selected_pbi().unwrap().key,
                "TEST-103",
                "down arrow should navigate through list"
            );
        });
    }

    #[test]
    fn pressing_up_arrow_navigates() {
        run_test(|| {
            let mut app = create_test_app();
            app.handle_key_event(KeyCode::Down);
            app.handle_key_event(KeyCode::Down);

            app.handle_key_event(KeyCode::Up);

            assert_eq!(
                app.selected_pbi().unwrap().key,
                "TEST-102",
                "up arrow should navigate through list"
            );
        });
    }

    #[test]
    fn pressing_j_at_bottom_wraps_to_top() {
        run_test(|| {
            let mut app = create_test_app();
            // Navigate to last item
            for _ in 0..4 {
                app.handle_key_event(KeyCode::Char('j'));
            }
            assert_eq!(app.selected_pbi().unwrap().key, "TEST-105");

            app.handle_key_event(KeyCode::Char('j'));

            assert_eq!(
                app.selected_pbi().unwrap().key,
                "TEST-101",
                "pressing 'j' at bottom should wrap to top"
            );
        });
    }

    #[test]
    fn pressing_k_at_top_wraps_to_bottom() {
        run_test(|| {
            let mut app = create_test_app();
            assert_eq!(app.selected_pbi().unwrap().key, "TEST-101");

            app.handle_key_event(KeyCode::Char('k'));

            assert_eq!(
                app.selected_pbi().unwrap().key,
                "TEST-105",
                "pressing 'k' at top should wrap to bottom"
            );
        });
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// USER INTERACTION TESTS: Actions
// ══════════════════════════════════════════════════════════════════════════════

mod actions {
    use super::*;

    #[test]
    fn pressing_l_opens_detail_view() {
        run_test(|| {
            let mut app = create_test_app();
            assert!(!app.is_detail_view());

            app.handle_key_event(KeyCode::Char('l'));

            assert!(app.is_detail_view(), "pressing 'l' should open detail view");
        });
    }

    #[test]
    fn pressing_right_arrow_opens_detail() {
        run_test(|| {
            let mut app = create_test_app();
            assert!(!app.is_detail_view());

            app.handle_key_event(KeyCode::Right);

            assert!(
                app.is_detail_view(),
                "pressing Right arrow should open detail view"
            );
        });
    }

    #[test]
    fn pressing_q_exits() {
        run_test(|| {
            let mut app = create_test_app();
            assert!(!app.is_exit());

            app.handle_key_event(KeyCode::Char('q'));

            assert!(app.is_exit(), "pressing 'q' should set exit flag");
        });
    }

    #[test]
    fn pressing_esc_exits() {
        run_test(|| {
            let mut app = create_test_app();
            assert!(!app.is_exit());

            app.handle_key_event(KeyCode::Esc);

            assert!(app.is_exit(), "pressing Escape should set exit flag");
        });
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// USER WORKFLOW TESTS
// ══════════════════════════════════════════════════════════════════════════════

mod workflows {
    use super::*;

    #[test]
    fn navigate_to_blocked_item_and_open_details() {
        run_test(|| {
            let mut app = create_test_app();

            // Navigate to blocked item (TEST-103 at index 2)
            app.handle_key_event(KeyCode::Char('j'));
            app.handle_key_event(KeyCode::Char('j'));
            assert_eq!(app.selected_pbi().unwrap().status, "Blocked");

            // Open details
            app.handle_key_event(KeyCode::Char('l'));

            assert!(app.is_detail_view());
        });
    }

    #[test]
    fn cycle_through_all_sprint_items() {
        run_test(|| {
            let mut app = create_test_app();
            let mut visited_keys = vec![];

            for _ in 0..5 {
                visited_keys.push(app.selected_pbi().unwrap().key.clone());
                app.handle_key_event(KeyCode::Char('j'));
            }

            assert_eq!(
                visited_keys,
                vec!["TEST-101", "TEST-102", "TEST-103", "TEST-104", "TEST-105"],
                "should cycle through all sprint items"
            );
        });
    }

    #[test]
    fn navigate_up_and_down_to_find_item() {
        run_test(|| {
            let mut app = create_test_app();

            // Move down to TEST-103
            app.handle_key_event(KeyCode::Char('j'));
            app.handle_key_event(KeyCode::Char('j'));
            let third_item = app.selected_pbi().unwrap().key.clone();

            // Move back up
            app.handle_key_event(KeyCode::Char('k'));
            let second_item = app.selected_pbi().unwrap().key.clone();

            assert_eq!(third_item, "TEST-103");
            assert_eq!(second_item, "TEST-102");
        });
    }

    #[test]
    fn mix_vim_keys_and_arrow_keys() {
        run_test(|| {
            let mut app = create_test_app();

            app.handle_key_event(KeyCode::Char('j'));
            app.handle_key_event(KeyCode::Down);
            app.handle_key_event(KeyCode::Char('k'));
            app.handle_key_event(KeyCode::Down);

            assert_eq!(
                app.selected_pbi().unwrap().key,
                "TEST-103",
                "mixing vim and arrow keys should work"
            );
        });
    }
}
