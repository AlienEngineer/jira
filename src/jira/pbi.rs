use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct Pbi {
    pub key: String,
    pub summary: String,
    pub status: String,
    pub assignee: String,
    pub issue_type: String,
    pub description: Option<String>,
    pub priority: Option<String>,
    pub story_points: Option<f64>,
    pub labels: Vec<String>,
    pub loaded: bool,
    pub in_progress_at: Option<String>,
    pub resolved_at: Option<String>,
    pub raw: String,
    pub resolution: Option<String>,
    pub components: Vec<String>,
    pub creator: String,
    pub reporter: String,
    pub project: String,
}

impl Pbi {
    pub fn elapsed_minutes(&self) -> Option<i64> {
        let s = self.status.to_lowercase();
        if s == "new" || s == "open" {
            return None;
        }
        let started = self
            .in_progress_at
            .as_deref()
            .and_then(parse_jira_datetime)?;
        let end = if self.is_done() {
            self.resolved_at
                .as_deref()
                .and_then(parse_jira_datetime)
                .unwrap_or_else(Utc::now)
        } else {
            Utc::now()
        };
        Some((end - started).num_minutes().max(0))
    }

    fn is_done(&self) -> bool {
        let status = self.status.to_lowercase();
        status.contains("closed") || status.contains("resolved")
    }
}

fn parse_jira_datetime(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_str(s, "%FT%T%.f%z")
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

pub fn pbi_elapsed_display(pbi: &Pbi) -> String {
    if let Some(minutes) = pbi.elapsed_minutes() {
        if minutes < 60 {
            return format!("{}m", minutes);
        } else if minutes < 1440 {
            return format!("{}h", minutes / 60);
        } else {
            return format!("{}d", minutes / 1440);
        }
    }
    "".to_string()
}
