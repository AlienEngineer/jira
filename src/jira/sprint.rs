use crate::api::JsonValueExt;
use crate::config;
use crate::jira::api;
use chrono::{DateTime, Utc};
use std::error::Error;
use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;

pub trait JiraGateway {
    fn get_agile(&self, endpoint: &str) -> Result<json::JsonValue, Box<dyn Error>>;
}

struct JiraRequest;

impl JiraGateway for JiraRequest {
    fn get_agile(&self, endpoint: &str) -> Result<json::JsonValue, Box<dyn Error>> {
        api::get_agile_call(endpoint.to_string())
    }
}

/// A single Product Backlog Item (PBI) from a sprint.
#[derive(Debug, Clone)]
pub struct Pbi {
    pub key: String,
    pub summary: String,
    pub status: String,
    pub assignee: String,
    pub issue_type: String,
    // Rich details — populated when the user presses f/F
    pub description: Option<String>,
    pub priority: Option<String>,
    pub story_points: Option<f64>,
    pub labels: Vec<String>,
    pub loaded: bool,
    // Timestamps (ISO-8601 strings from the Jira API)
    pub in_progress_at: Option<String>,
    pub resolved_at: Option<String>,
}

/// Parse a Jira ISO-8601 datetime string (e.g. "2026-03-01T10:30:00.000+0000").
fn parse_jira_datetime(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_str(s, "%FT%T%.f%z")
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

/// Scan a Jira changelog (the `changelog` node of an issue response fetched
/// with `?expand=changelog`) and return the timestamp of the **last** time the
/// issue was transitioned into any "in progress" status.
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

/// Return a human-readable elapsed duration for a PBI, or an empty string when
/// the status is "new" or "open" (i.e. work has not started).
///
/// - < 60 min  → "Xm"
/// - < 24 h    → "Xh"
/// - ≥ 24 h    → "Xd"
///
/// The start time is when the PBI was **last moved to "In Progress"**
/// (`in_progress_at`). For done/closed/resolved items the end time is
/// `resolved_at` (falling back to now). For all other active statuses the
/// end time is now. Returns an empty string when `in_progress_at` is absent.
pub fn pbi_elapsed_display(pbi: &Pbi) -> String {
    let s = pbi.status.to_lowercase();
    if s == "new" || s == "open" {
        return String::new();
    }

    let started = match pbi.in_progress_at.as_deref().and_then(parse_jira_datetime) {
        Some(dt) => dt,
        None => return String::new(),
    };

    let is_done = s.contains("done") || s.contains("closed") || s.contains("resolved");
    let end = if is_done {
        pbi.resolved_at
            .as_deref()
            .and_then(parse_jira_datetime)
            .unwrap_or_else(Utc::now)
    } else {
        Utc::now()
    };

    let minutes = (end - started).num_minutes().max(0);
    if minutes < 60 {
        format!("{}m", minutes)
    } else if minutes < 1440 {
        format!("{}h", minutes / 60)
    } else {
        format!("{}d", minutes / 1440)
    }
}

/// Return the total elapsed minutes for a PBI (used by the plugin system).
pub fn pbi_elapsed_minutes(pbi: &Pbi) -> Option<i64> {
    let s = pbi.status.to_lowercase();
    if s == "new" || s == "open" {
        return None;
    }
    let started = pbi
        .in_progress_at
        .as_deref()
        .and_then(parse_jira_datetime)?;
    let is_done = s.contains("done") || s.contains("closed") || s.contains("resolved");
    let end = if is_done {
        pbi.resolved_at
            .as_deref()
            .and_then(parse_jira_datetime)
            .unwrap_or_else(Utc::now)
    } else {
        Utc::now()
    };
    Some((end - started).num_minutes().max(0))
}

#[derive(Debug, Clone)]
pub struct Sprint {
    pub name: String,
    pub goal: String,
    pub end_date: String,
    pub pbis: Vec<Pbi>,
    pub board_id: String,
}

fn status_sort_key(status: &str) -> u8 {
    match status.to_lowercase().as_str() {
        "closed" => 10,
        "resolved" => 9,
        "blocked" => 8,
        "pending" => 8,
        "in review" => 7,
        "in progress" => 6,
        "open" => 4,
        "new" => 2,
        _ => 0,
    }
}

pub fn sort_by_status(pbis: &mut [Pbi]) {
    pbis.sort_by_key(|p| status_sort_key(&p.status));
}

fn cache_path(board_id: &str) -> PathBuf {
    let config_file = config::get_config_file_name();
    let config_dir = PathBuf::from(&config_file)
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_default();
    config_dir.join(format!("sprint_cache_{board_id}.json"))
}

/// Load sprint data from the on-disk cache. Returns `None` if the cache does
/// not exist or is malformed.
pub fn load_sprint_cache(board_id: &str) -> Option<Sprint> {
    let path = cache_path(board_id);
    let mut file = fs::File::open(path).ok()?;
    let mut contents = String::new();
    file.read_to_string(&mut contents).ok()?;
    let data = json::parse(&contents).ok()?;

    let mut pbis = Vec::new();
    for item in data["pbis"].members() {
        let labels = item["labels"]
            .members()
            .filter_map(|l| l.as_str().map(|s| s.to_string()))
            .collect();
        pbis.push(Pbi {
            key: item["key"].as_string_or(""),
            summary: item["summary"].as_string_or(""),
            status: item["status"].as_string_or(""),
            assignee: item["assignee"].as_string_or("Unassigned"),
            issue_type: item["issue_type"].as_string_or(""),
            description: item["description"].as_str().map(|s| s.to_string()),
            priority: item["priority"].as_str().map(|s| s.to_string()),
            story_points: item["story_points"].as_f64(),
            labels,
            loaded: item["loaded"].as_bool().unwrap_or(false),
            in_progress_at: item["in_progress_at"].as_str().map(|s| s.to_string()),
            resolved_at: item["resolved_at"].as_str().map(|s| s.to_string()),
        });
    }

    sort_by_status(&mut pbis);
    Some(Sprint {
        name: data["sprint_name"].as_string_or(""),
        goal: data["sprint_goal"].as_string_or(""),
        end_date: data["sprint_end_date"].as_string_or(""),
        pbis,
        board_id: data["board_id"].as_string_or(board_id),
    })
}

pub fn save_sprint_cache(sprint: &Sprint) {
    let pbis_json = convert_pbis_to_json(&sprint.pbis);

    let data = json::object! {
        "sprint_name": sprint.name.as_str(),
        "sprint_goal": sprint.goal.as_str(),
        "sprint_end_date": sprint.end_date.as_str(),
        "pbis": pbis_json,
        "board_id": sprint.board_id.as_str()
    };

    let path = cache_path(sprint.board_id.as_str());
    if let Ok(mut file) = fs::File::create(path) {
        let _ = file.write_all(json::stringify_pretty(data, 2).as_bytes());
    }
}

fn convert_pbis_to_json(pbis: &[Pbi]) -> json::JsonValue {
    let mut pbis_json = json::JsonValue::new_array();
    for pbi in pbis {
        let mut labels_json = json::JsonValue::new_array();
        for label in &pbi.labels {
            let _ = labels_json.push(label.as_str());
        }
        let mut obj = json::object! {
            "key": pbi.key.as_str(),
            "summary": pbi.summary.as_str(),
            "status": pbi.status.as_str(),
            "assignee": pbi.assignee.as_str(),
            "issue_type": pbi.issue_type.as_str(),
            "loaded": pbi.loaded,
            "labels": labels_json,
        };
        obj["description"] = match &pbi.description {
            Some(d) => json::JsonValue::String(d.clone()),
            None => json::JsonValue::Null,
        };
        obj["priority"] = match &pbi.priority {
            Some(p) => json::JsonValue::String(p.clone()),
            None => json::JsonValue::Null,
        };
        obj["story_points"] = match pbi.story_points {
            Some(sp) => json::JsonValue::Number(sp.into()),
            None => json::JsonValue::Null,
        };
        obj["in_progress_at"] = match &pbi.in_progress_at {
            Some(ts) => json::JsonValue::String(ts.clone()),
            None => json::JsonValue::Null,
        };
        obj["resolved_at"] = match &pbi.resolved_at {
            Some(ts) => json::JsonValue::String(ts.clone()),
            None => json::JsonValue::Null,
        };
        let _ = pbis_json.push(obj);
    }
    pbis_json
}

// ── API helpers ──────────────────────────────────────────────────────────────

/// Fetch issues for the active sprint on the given board.
///
/// Returns a tuple of (sprint_name, sprint_goal, sprint_end_date, Vec<Pbi>).
/// `sprint_end_date` is an ISO-8601 date string (e.g. "2026-03-20") or empty
/// when the field is absent.
pub fn fetch_active_sprint_issues(board_id: &str) -> Result<Sprint, Box<dyn Error>> {
    fetch_active_sprint_issues_with_client(&JiraRequest, board_id)
}

fn fetch_active_sprint_issues_with_client(
    gateway: &impl JiraGateway,
    board_id: &str,
) -> Result<Sprint, Box<dyn Error>> {
    let sprints_response = gateway.get_agile(&format!("board/{board_id}/sprint?state=active"))?;
    let sprints = &sprints_response["values"];
    if !sprints.is_array() || sprints.is_empty() {
        return Err("No active sprint found for the given board.".into());
    }
    let sprint = &sprints[0];
    let sprint_id = sprint["id"].as_u64_or_0();
    Ok(Sprint {
        name: sprint["name"].as_string_or("Active Sprint"),
        goal: sprint["goal"].as_string_or(""),
        end_date: sprint["endDate"]
            .as_str()
            .map(|s| s.chars().take(10).collect::<String>())
            .unwrap_or_default(),
        pbis: fetch_sprint_pbis_with_client(gateway, sprint_id)?,
        board_id: board_id.to_string(),
    })
}

fn fetch_sprint_pbis_with_client(
    gateway: &impl JiraGateway,
    sprint_id: u64,
) -> Result<Vec<Pbi>, Box<dyn Error + 'static>> {
    let issues_response = gateway.get_agile(&format!(
        "sprint/{sprint_id}/issue?maxResults=500&expand=changelog"
    ))?;
    let issues = &issues_response["issues"];
    let mut pbis = Vec::new();
    if issues.is_array() {
        for issue in issues.members() {
            let fields = &issue["fields"];
            pbis.push(Pbi {
                key: issue["key"].as_string_or(""),
                summary: fields["summary"].as_string_or(""),
                status: fields["status"]["name"].as_string_or("-"),
                assignee: fields["assignee"]["displayName"].as_string_or("Unassigned"),
                issue_type: fields["issuetype"]["name"].as_string_or("-"),
                description: fields["description"].as_str().map(|s| s.to_string()),
                priority: fields["priority"]["name"].as_str().map(|s| s.to_string()),
                story_points: extract_story_points(fields),
                labels: fields["labels"]
                    .members()
                    .filter_map(|l| l.as_str().map(|s| s.to_string()))
                    .collect(),
                loaded: false,
                in_progress_at: last_in_progress_at(&issue["changelog"]),
                resolved_at: fields["resolutiondate"].as_str().map(|s| s.to_string()),
            });
        }
    }
    sort_by_status(&mut pbis);
    Ok(pbis)
}

/// Extract the story-points value from a Jira `fields` object.
///
/// Resolution order:
/// 1. `fields["story_points"]`  — standard name
/// 2. The field named by the `story_points` alias in the config (e.g. `"customfield_10006"`)
/// 3. Hardcoded common custom-field fallbacks
fn extract_story_points(fields: &json::JsonValue) -> Option<f64> {
    fields["story_points"].as_f64().or_else(|| {
        let alias_field = crate::config::get_alias("story_points".to_string())?;
        fields[alias_field.as_str()].as_f64()
    })
}
pub fn fetch_pbi_details(pbi: &mut Pbi) -> Result<(), Box<dyn Error>> {
    let response = api::get_call_v2(format!("issue/{}?expand=changelog", pbi.key))?;
    let fields = &response["fields"];

    // Description: v2 returns plain text or ADF; try string first
    let desc = &fields["description"];
    pbi.description = if desc.is_string() {
        desc.as_str().map(|s| s.to_string())
    } else if desc.is_object() {
        Some(extract_adf_text(desc))
    } else {
        None
    };

    pbi.priority = fields["priority"]["name"].as_str().map(|s| s.to_string());

    // Story points: try standard name first, then the configured alias
    pbi.story_points = extract_story_points(fields);

    pbi.labels = fields["labels"]
        .members()
        .filter_map(|l| l.as_str().map(|s| s.to_string()))
        .collect();

    // Refresh status and assignee in case they changed
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

/// Recursively extract plain text from an Atlassian Document Format (ADF) node.
fn extract_adf_text(node: &json::JsonValue) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::io;

    struct JiraFakeGateway {
        responses: HashMap<String, json::JsonValue>,
    }

    impl JiraFakeGateway {
        fn new(responses: HashMap<String, json::JsonValue>) -> Self {
            Self { responses }
        }
    }

    impl JiraGateway for JiraFakeGateway {
        fn get_agile(&self, endpoint: &str) -> Result<json::JsonValue, Box<dyn Error>> {
            self.responses.get(endpoint).cloned().ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("missing fake Jira response for endpoint '{endpoint}'"),
                )
                .into()
            })
        }
    }

    fn active_sprint_response() -> json::JsonValue {
        json::object! {
            "values": [
                {
                    "id": 42,
                    "name": "Platform Sprint",
                    "goal": "Ship the sprint fetch refactor",
                    "endDate": "2026-03-20T10:30:00.000Z"
                }
            ]
        }
    }

    fn sprint_issues_response() -> json::JsonValue {
        json::object! {
            "issues": [
                {
                    "key": "JIRA-2",
                    "fields": {
                        "summary": "Blocked item",
                        "status": { "name": "Blocked" },
                        "assignee": { "displayName": "Taylor" },
                        "issuetype": { "name": "Bug" },
                        "description": "Needs input",
                        "priority": { "name": "High" },
                        "story_points": 5.0,
                        "labels": ["backend", "urgent"],
                        "resolutiondate": null
                    },
                    "changelog": {
                        "histories": []
                    }
                },
                {
                    "key": "JIRA-1",
                    "fields": {
                        "summary": "Closed item",
                        "status": { "name": "Closed" },
                        "assignee": { "displayName": "Alex" },
                        "issuetype": { "name": "Story" },
                        "description": "All done",
                        "priority": { "name": "Medium" },
                        "story_points": 3.0,
                        "labels": ["frontend"],
                        "resolutiondate": "2026-03-10T09:00:00.000+0000"
                    },
                    "changelog": {
                        "histories": [
                            {
                                "created": "2026-03-09T09:00:00.000+0000",
                                "items": [
                                    {
                                        "field": "status",
                                        "toString": "In Progress"
                                    }
                                ]
                            }
                        ]
                    }
                }
            ]
        }
    }

    fn make_gateway(responses: Vec<(&str, json::JsonValue)>) -> JiraFakeGateway {
        JiraFakeGateway::new(
            responses
                .into_iter()
                .map(|(endpoint, body)| (endpoint.to_string(), body))
                .collect(),
        )
    }

    // ── active sprint fetch ───────────────────────────────────────────────────

    #[test]
    fn fetch_active_sprint_issues_uses_fake_jira_responses() {
        let client = make_gateway(vec![
            ("board/7/sprint?state=active", active_sprint_response()),
            (
                "sprint/42/issue?maxResults=500&expand=changelog",
                sprint_issues_response(),
            ),
        ]);

        let sprint = fetch_active_sprint_issues_with_client(&client, "7")
            .expect("expected fake Jira responses to build a sprint");

        assert_eq!(sprint.name, "Platform Sprint");
        assert_eq!(sprint.goal, "Ship the sprint fetch refactor");
        assert_eq!(sprint.end_date, "2026-03-20");
        assert_eq!(sprint.board_id, "7");
        assert_eq!(sprint.pbis.len(), 2);
        assert_eq!(
            sprint.pbis[0].key, "JIRA-2",
            "blocked items should remain ahead of closed ones after sorting"
        );
        assert_eq!(sprint.pbis[0].labels, vec!["backend", "urgent"]);
        assert_eq!(sprint.pbis[1].key, "JIRA-1");
        assert_eq!(sprint.pbis[1].story_points, Some(3.0));
        assert_eq!(
            sprint.pbis[1].in_progress_at.as_deref(),
            Some("2026-03-09T09:00:00.000+0000")
        );
    }

    #[test]
    fn fetch_active_sprint_issues_without_active_sprint_returns_error() {
        let client = make_gateway(vec![(
            "board/7/sprint?state=active",
            json::object! {
                "values": []
            },
        )]);

        let error = fetch_active_sprint_issues_with_client(&client, "7")
            .expect_err("expected an error when the fake Jira response has no active sprint");

        assert_eq!(
            error.to_string(),
            "No active sprint found for the given board."
        );
    }
}
