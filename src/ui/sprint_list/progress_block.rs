use crate::jira::sprint::Pbi;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
    Frame,
};

// ── Data ─────────────────────────────────────────────────────────────────────

/// All the information `ProgressBlock` needs to render itself.
///
/// Created from raw sprint data via [`SprintProgressData::from_sprint`]; the
/// renderer never touches `Pbi` directly.
pub struct SprintProgressData {
    pub resolved: usize,
    pub total: usize,
    /// ISO-8601 date string (YYYY-MM-DD) for the sprint end, empty when unknown.
    pub end_date: String,
}

impl SprintProgressData {
    /// Map raw sprint data into the shape the progress block needs.
    ///
    /// This is the single place that defines which statuses count as "resolved"
    /// and how to extract the end-date string.
    pub fn from_sprint(pbis: &[Pbi], end_date: &str) -> Self {
        let total = pbis.len();
        let resolved = pbis
            .iter()
            .filter(|p| {
                let s = p.status.to_lowercase();
                s.contains("closed") || s.contains("resolved")
            })
            .count();
        Self {
            resolved,
            total,
            end_date: end_date.to_string(),
        }
    }
}

// ── Renderer ──────────────────────────────────────────────────────────────────

/// Stateless renderer for the sprint completion block.
///
/// Has no owned state — all data arrives through [`SprintProgressData`] at
/// render time, built by the coordinator from the live sprint snapshot.
pub struct ProgressBlock;

impl ProgressBlock {
    pub fn new() -> Self {
        Self
    }

    /// Render the progress block into `area` using the pre-computed `data`.
    pub fn render(&self, frame: &mut Frame, area: Rect, data: &SprintProgressData) {
        let pct = if data.total > 0 {
            data.resolved * 100 / data.total
        } else {
            0
        };

        const BAR_WIDTH: usize = 28;
        let filled = if data.total > 0 {
            data.resolved * BAR_WIDTH / data.total
        } else {
            0
        };
        let bar = format!("[{}{}]", "█".repeat(filled), "░".repeat(BAR_WIDTH - filled),);

        let days_label = match Self::working_days_remaining(&data.end_date) {
            Some(0) => " ⏱ Sprint ends today!".to_string(),
            Some(d) => format!(" ⏱ {} working day{} left", d, if d == 1 { "" } else { "s" }),
            None => String::new(),
        };

        let bar_color = if pct >= 80 {
            Color::Green
        } else if pct >= 40 {
            Color::Yellow
        } else {
            Color::Red
        };

        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(bar, Style::default().fg(bar_color)),
                Span::styled(
                    format!(" {}% ({}/{} resolved)", pct, data.resolved, data.total),
                    Style::default().fg(Color::White),
                ),
                Span::styled(days_label, Style::default().fg(Color::Cyan)),
            ]))
            .block(
                Block::bordered()
                    .title(Span::styled(
                        " Sprint Progress ",
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ))
                    .border_style(Style::default().fg(Color::DarkGray)),
            ),
            area,
        );
    }

    /// Count weekdays (Mon–Fri) from today through `end_date_str` (inclusive).
    /// Returns `None` when the string is absent or unparseable.
    fn working_days_remaining(end_date_str: &str) -> Option<i64> {
        use chrono::{Datelike, Local, NaiveDate, Weekday};

        if end_date_str.is_empty() {
            return None;
        }
        let end =
            NaiveDate::parse_from_str(&end_date_str[..10.min(end_date_str.len())], "%Y-%m-%d")
                .ok()?;
        let today = Local::now().date().naive_local();
        if today > end {
            return Some(0);
        }
        let mut count = 0i64;
        let mut d = today;
        while d <= end {
            match d.weekday() {
                Weekday::Sat | Weekday::Sun => {}
                _ => count += 1,
            }
            d = d.succ_opt()?;
        }
        Some(count)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jira::sprint::Pbi;

    /// Build a minimal `Pbi` with only the `status` field filled in; all other
    /// fields use sensible defaults that are irrelevant to `SprintProgressData`.
    fn pbi(status: &str) -> Pbi {
        Pbi {
            key: "TEST-1".into(),
            summary: "test item".into(),
            status: status.into(),
            assignee: String::new(),
            issue_type: "Story".into(),
            description: None,
            priority: None,
            story_points: None,
            labels: vec![],
            loaded: false,
        }
    }

    // ── from_sprint: totals ───────────────────────────────────────────────────

    #[test]
    fn empty_list_gives_zero_totals() {
        let data = SprintProgressData::from_sprint(&[], "2026-03-20");
        assert_eq!(data.total, 0);
        assert_eq!(data.resolved, 0);
    }

    #[test]
    fn total_equals_number_of_pbis() {
        let pbis = vec![pbi("New"), pbi("In Progress"), pbi("Done")];
        let data = SprintProgressData::from_sprint(&pbis, "");
        assert_eq!(data.total, 3);
    }

    // ── from_sprint: resolved counting ───────────────────────────────────────

    #[test]
    fn done_status_does_not_count_as_resolved() {
        let data = SprintProgressData::from_sprint(&[pbi("Done")], "");
        assert_eq!(data.resolved, 0);
    }

    #[test]
    fn closed_status_counts_as_resolved() {
        let data = SprintProgressData::from_sprint(&[pbi("Closed")], "");
        assert_eq!(data.resolved, 1);
    }

    #[test]
    fn resolved_status_counts_as_resolved() {
        let data = SprintProgressData::from_sprint(&[pbi("Resolved")], "");
        assert_eq!(data.resolved, 1);
    }

    #[test]
    fn in_progress_does_not_count_as_resolved() {
        let data = SprintProgressData::from_sprint(&[pbi("In Progress")], "");
        assert_eq!(data.resolved, 0);
    }

    #[test]
    fn to_do_does_not_count_as_resolved() {
        let data = SprintProgressData::from_sprint(&[pbi("New")], "");
        assert_eq!(data.resolved, 0);
    }

    #[test]
    fn blocked_does_not_count_as_resolved() {
        let data = SprintProgressData::from_sprint(&[pbi("Blocked")], "");
        assert_eq!(data.resolved, 0);
    }

    #[test]
    fn in_review_does_not_count_as_resolved() {
        let data = SprintProgressData::from_sprint(&[pbi("In Review")], "");
        assert_eq!(data.resolved, 0);
    }

    // ── from_sprint: case-insensitivity ──────────────────────────────────────

    #[test]
    fn resolved_matching_is_case_insensitive() {
        for status in ["resolved", "closed", "Closed", "CLOSED", "RESOLVED"] {
            let data = SprintProgressData::from_sprint(&[pbi(status)], "");
            assert_eq!(
                data.resolved, 1,
                "status '{status}' should be counted as resolved"
            );
        }
    }

    #[test]
    fn unresolved_matching_is_case_insensitive() {
        for status in ["IN PROGRESS", "in progress", "TO DO", "BLOCKED"] {
            let data = SprintProgressData::from_sprint(&[pbi(status)], "");
            assert_eq!(
                data.resolved, 0,
                "status '{status}' should NOT be counted as resolved"
            );
        }
    }

    // ── from_sprint: mixed sprint ─────────────────────────────────────────────

    #[test]
    fn mixed_sprint_counts_correctly() {
        let pbis = vec![
            pbi("Done"),
            pbi("Closed"),
            pbi("Resolved"),
            pbi("In Progress"),
            pbi("New"),
            pbi("any status"),
        ];
        let data = SprintProgressData::from_sprint(&pbis, "");
        assert_eq!(data.total, 6);
        assert_eq!(data.resolved, 2);
    }

    #[test]
    fn all_resolved_sprint() {
        let pbis = vec![pbi("Resolved"), pbi("resolved"), pbi("Closed")];
        let data = SprintProgressData::from_sprint(&pbis, "");
        assert_eq!(data.total, 3);
        assert_eq!(data.resolved, 3);
    }

    #[test]
    fn all_unresolved_sprint() {
        let pbis = vec![pbi("New"), pbi("In Progress"), pbi("Blocked")];
        let data = SprintProgressData::from_sprint(&pbis, "");
        assert_eq!(data.total, 3);
        assert_eq!(data.resolved, 0);
    }

    // ── from_sprint: end_date pass-through ───────────────────────────────────

    #[test]
    fn end_date_is_preserved() {
        let data = SprintProgressData::from_sprint(&[], "2026-03-20");
        assert_eq!(data.end_date, "2026-03-20");
    }

    #[test]
    fn empty_end_date_is_preserved() {
        let data = SprintProgressData::from_sprint(&[], "");
        assert_eq!(data.end_date, "");
    }

    #[test]
    fn full_iso_timestamp_end_date_is_preserved() {
        // The mapping stores the string as-is; the renderer trims to 10 chars.
        let data = SprintProgressData::from_sprint(&[], "2026-03-20T00:00:00.000Z");
        assert_eq!(data.end_date, "2026-03-20T00:00:00.000Z");
    }
}
