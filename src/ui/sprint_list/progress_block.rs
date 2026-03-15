use crate::jira::pbi::Pbi;
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
    /// Number of Mon–Fri working days from today to `end_date` (inclusive).
    /// `None` when `end_date` is absent or unparseable; `Some(0)` when past.
    pub working_days_remaining: Option<i64>,
}

impl SprintProgressData {
    /// Map raw sprint data into the shape the progress block needs.
    ///
    /// This is the single place that defines which statuses count as "resolved",
    /// how to extract the end-date string, and how to compute working days.
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
            working_days_remaining: Self::compute_working_days(end_date),
        }
    }

    fn count_working_days(end: chrono::NaiveDate, today: chrono::NaiveDate) -> Option<i64> {
        use chrono::{Datelike, Weekday};
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
    /// Count weekdays (Mon–Fri) from today through `end_date_str` (inclusive).
    /// Returns `None` when the string is absent or unparseable; `Some(0)` when
    /// the sprint end date is in the past.
    fn compute_working_days(end_date_str: &str) -> Option<i64> {
        use chrono::{Local, NaiveDate};

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
        Self::count_working_days(end, today)
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

        let days_label = match data.working_days_remaining {
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
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jira::pbi::Pbi;

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
            in_progress_at: None,
            resolved_at: None,
            loaded: false,
            raw: "".into(),
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
        assert_eq!(data.resolved, 2); // Closed + Resolved only ("Done" does not count)
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

    // ── working_days_remaining ────────────────────────────────────────────────

    #[test]
    fn empty_end_date_gives_none_working_days() {
        let data = SprintProgressData::from_sprint(&[], "");
        assert_eq!(data.working_days_remaining, None);
    }

    #[test]
    fn unparseable_end_date_gives_none_working_days() {
        let data = SprintProgressData::from_sprint(&[], "not-a-date");
        assert_eq!(data.working_days_remaining, None);
    }

    #[test]
    fn past_end_date_gives_zero_working_days() {
        // A date well in the past should always return Some(0).
        let data = SprintProgressData::from_sprint(&[], "2000-01-01");
        assert_eq!(data.working_days_remaining, Some(0));
    }

    #[test]
    fn future_end_date_gives_positive_working_days() {
        // A date far in the future must have at least one working day.
        let data = SprintProgressData::from_sprint(&[], "2099-12-31");
        assert!(
            data.working_days_remaining.is_some_and(|d| d > 0),
            "expected positive working days for a far-future date"
        );
    }

    #[test]
    fn iso_timestamp_end_date_is_parsed_correctly() {
        // Full ISO-8601 timestamps should be handled (only first 10 chars used).
        let data_ts = SprintProgressData::from_sprint(&[], "2000-01-01T00:00:00.000Z");
        let data_date = SprintProgressData::from_sprint(&[], "2000-01-01");
        assert_eq!(
            data_ts.working_days_remaining, data_date.working_days_remaining,
            "timestamp and date-only formats should yield the same working-day count"
        );
    }
}
