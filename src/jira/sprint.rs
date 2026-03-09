use crate::api::JsonValueExt;
use crate::config;
use crate::jira::api;
use std::error::Error;
use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;

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
}

// TODO: eval if these fields really need to be String or can be &str
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
    let board_id: &str = board_id;
    let sprints_response = api::get_agile_call(format!("board/{board_id}/sprint?state=active"))?;
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
        pbis: fetch_sprint_pbis(sprint_id)?,
        board_id: board_id.to_string(),
    })
}

fn fetch_sprint_pbis(sprint_id: u64) -> Result<Vec<Pbi>, Box<dyn Error + 'static>> {
    let issues_response = api::get_agile_call(format!("sprint/{sprint_id}/issue?maxResults=500"))?;
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
                description: None,
                priority: None,
                story_points: None,
                labels: Vec::new(),
                loaded: false,
            });
        }
    }
    sort_by_status(&mut pbis);
    Ok(pbis)
}

/// Fetch and populate rich details for a single PBI in place.
pub fn fetch_pbi_details(pbi: &mut Pbi) -> Result<(), Box<dyn Error>> {
    let response = api::get_call_v2(format!("issue/{}", pbi.key))?;
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

    // Story points live in different custom fields depending on the JIRA setup
    pbi.story_points = fields["story_points"]
        .as_f64()
        .or_else(|| fields["customfield_10016"].as_f64())
        .or_else(|| fields["customfield_10028"].as_f64());

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
