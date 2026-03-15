#[cfg(test)]
mod test {
    use jira::jira::sprint::Pbi;
    use rstest::rstest;

    #[test]
    fn an_empty_pbi_returns_none_as_elapsed_minutes() {
        let pbi = make_pbi(None, None);

        let minutes = pbi.elapsed_minutes();

        assert_eq!(minutes, None);
    }

    #[rstest]
    #[case("new")]
    #[case("open")]
    #[case("NEW")]
    #[case("OPEN")]
    fn when_the_work_hasnt_started_elapsed_minutes_returns_none(#[case] status: &str) {
        let pbi = make_pbi(Some(status), None);

        let minutes = pbi.elapsed_minutes();

        assert_eq!(minutes, None);
    }

    #[rstest]
    #[case("in progress")]
    #[case("in review")]
    #[case("in progress")]
    #[case("in review")]
    #[case("closed")]
    #[case("resolved")]
    fn when_the_work_has_started_but_start_date_is_missing_elapsed_minutes_returns_none(
        #[case] status: &str,
    ) {
        let pbi = make_pbi(Some(status), None);

        let minutes = pbi.elapsed_minutes();

        assert_eq!(minutes, None);
    }

    #[rstest]
    #[case("in progress")]
    #[case("in review")]
    #[case("blocked")]
    #[case("IN PROGRESS")]
    #[case("IN REVIEW")]
    #[case("BLOCKED")]
    fn when_the_work_has_started_elapsed_minutes_returns_time_since_start(#[case] status: &str) {
        let in_progress_at = calculate_time_ago_str(10);
        let pbi = make_pbi(Some(status), Some(&in_progress_at));

        let minutes = pbi.elapsed_minutes();

        assert_eq!(minutes, Some(10));
    }

    // ── elapsed_minutes: done statuses ───────────────────────────────────────

    #[rstest]
    #[case("closed")]
    #[case("resolved")]
    #[case("CLOSED")]
    #[case("RESOLVED")]
    fn when_status_is_done_elapsed_minutes_returns_time_between_in_progress_and_resolved(
        #[case] status: &str,
    ) {
        let in_progress_at = calculate_time_ago_str(30);
        let resolved_at = calculate_time_ago_str(10);
        let pbi = make_pbi_full(Some(status), Some(&in_progress_at), Some(&resolved_at));

        let minutes = pbi.elapsed_minutes();

        assert_eq!(minutes, Some(20));
    }

    #[rstest]
    #[case("closed", 10)]
    #[case("resolved", 5)]
    fn done_pbi_without_resolved_at_falls_back_to_now(
        #[case] status: &str,
        #[case] minutes_ago: i64,
    ) {
        let in_progress_at = calculate_time_ago_str(minutes_ago);
        let pbi = make_pbi_full(Some(status), Some(&in_progress_at), None);

        let minutes = pbi.elapsed_minutes();

        assert_eq!(minutes, Some(minutes_ago));
    }

    #[rstest]
    #[case("not-a-date")]
    #[case("")]
    fn invalid_in_progress_at_returns_none(#[case] date: &str) {
        let pbi = make_pbi(Some("in progress"), Some(date));

        let minutes = pbi.elapsed_minutes();

        assert_eq!(minutes, None);
    }

    // ── elapsed_minutes: future start time clamping ───────────────────────────

    #[test]
    fn in_progress_at_in_future_returns_zero() {
        let in_progress_at = chrono::Utc::now() + chrono::Duration::minutes(10);
        let pbi = make_pbi(
            Some("in progress"),
            Some(in_progress_at.to_rfc3339().as_str()),
        );

        let minutes = pbi.elapsed_minutes();

        assert_eq!(
            minutes,
            Some(0),
            "elapsed minutes should be clamped to 0 when start is in the future"
        );
    }

    // ── elapsed_minutes: status matching ──────────────────────────────────────

    #[test]
    fn status_containing_closed_is_treated_as_done() {
        let in_progress_at = calculate_time_ago_str(20);
        let resolved_at = calculate_time_ago_str(10);
        let pbi = make_pbi_full(
            Some("partially closed"),
            Some(&in_progress_at),
            Some(&resolved_at),
        );

        let minutes = pbi.elapsed_minutes();

        assert_eq!(
            minutes,
            Some(10),
            "any status containing 'closed' should use resolved_at"
        );
    }

    #[test]
    fn status_containing_resolved_is_treated_as_done() {
        let in_progress_at = calculate_time_ago_str(30);
        let resolved_at = calculate_time_ago_str(10);
        let pbi = make_pbi_full(
            Some("auto-resolved"),
            Some(&in_progress_at),
            Some(&resolved_at),
        );

        let minutes = pbi.elapsed_minutes();

        assert_eq!(
            minutes,
            Some(20),
            "any status containing 'resolved' should use resolved_at"
        );
    }

    fn calculate_time_ago_str(minutes: i64) -> String {
        (chrono::Utc::now() - chrono::Duration::minutes(minutes)).to_rfc3339()
    }

    fn make_pbi(status: Option<&str>, in_progress_at: Option<&str>) -> Pbi {
        make_pbi_full(status, in_progress_at, None)
    }

    fn make_pbi_full(
        status: Option<&str>,
        in_progress_at: Option<&str>,
        resolved_at: Option<&str>,
    ) -> Pbi {
        Pbi {
            key: "".to_string(),
            summary: "".to_string(),
            status: status.unwrap_or_default().to_string(),
            assignee: "".to_string(),
            issue_type: "".to_string(),
            description: None,
            priority: Some("".to_string()),
            story_points: None,
            labels: Vec::new(),
            loaded: false,
            in_progress_at: in_progress_at.map(|s| s.to_string()),
            resolved_at: resolved_at.map(|s| s.to_string()),
        }
    }
}
