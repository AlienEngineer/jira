use crate::config::get_alias;
use crate::jira::api::JiraApi;
use chrono::{DateTime, Utc};

/// Fetch and populate PBI details using the v3 API.
pub fn fetch_pbi_details(api: &dyn JiraApi, pbi: &mut Pbi) -> Result<(), String> {
    let response = api
        .get_v3(&format!("issue/{}?expand=changelog", pbi.key))
        .map_err(|e| e.to_string())?;
    let fields = &response["fields"];

    let desc = &fields["description"];
    pbi.description = if desc.is_string() {
        desc.as_str().map(|s| s.to_string())
    } else if desc.is_object() {
        Some(extract_adf_text(desc))
    } else {
        None
    };

    pbi.priority = fields["priority"]["name"].as_str().map(|s| s.to_string());
    pbi.story_points = extract_story_points(fields);
    pbi.labels = fields["labels"]
        .members()
        .filter_map(|l| l.as_str().map(|s| s.to_string()))
        .collect();

    if let Some(s) = fields["status"]["name"].as_str() {
        pbi.status = s.to_string();
    }
    if let Some(a) = fields["assignee"]["displayName"].as_str() {
        pbi.assignee = a.to_string();
    }
    pbi.in_progress_at = last_in_progress_at(&response["changelog"]);
    pbi.resolved_at = fields["resolutiondate"].as_str().map(|s| s.to_string());
    pbi.loaded = true;
    Ok(())
}

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

/// Extract the last "In Progress" transition timestamp from a Jira changelog.
pub fn last_in_progress_at(changelog: &json::JsonValue) -> Option<String> {
    let mut last: Option<String> = None;
    for history in changelog["histories"].members() {
        for item in history["items"].members() {
            if item["field"].as_str() == Some("status") {
                let to = item["toString"].as_str().unwrap_or("").to_lowercase();
                if to.contains("in progress") {
                    if let Some(created) = history["created"].as_str() {
                        last = Some(created.to_string());
                    }
                }
            }
        }
    }
    last
}

/// Extract story points from a Jira fields object, checking aliases.
pub fn extract_story_points(fields: &json::JsonValue) -> Option<f64> {
    fields["story_points"].as_f64().or_else(|| {
        let alias_field = get_alias("story_points".to_string())?;
        fields[alias_field.as_str()].as_f64()
    })
}

/// Extract text from an Atlassian Document Format (ADF) node.
pub fn extract_adf_text(node: &json::JsonValue) -> String {
    let mut text = String::new();
    if let Some(t) = node["text"].as_str() {
        text.push_str(t);
    }
    for child in node["content"].members() {
        let child_text = extract_adf_text(child);
        if !child_text.is_empty() {
            if !text.is_empty() {
                text.push(' ');
            }
            text.push_str(&child_text);
        }
    }
    text
}
