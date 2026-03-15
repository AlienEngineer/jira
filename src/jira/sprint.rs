use crate::api::JsonValueExt;
use crate::config::{self, get_alias};
use crate::ioc::interface::Interface;
use crate::jira::api::JiraApi;
use crate::jira::pbi::Pbi;
use std::error::Error;
use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::Arc;

pub trait SprintService: Interface {
    fn fetch_active_sprint_issues(&self, board_id: &str) -> Result<Sprint, Box<dyn Error>>;
    fn fetch_pbi_details(&self, pbi: &mut Pbi) -> Result<(), Box<dyn Error>>;
}

pub struct DefaultSprintService {
    jira_api: Arc<dyn JiraApi>,
}

impl DefaultSprintService {
    pub fn new(jira_api: Arc<dyn JiraApi>) -> Self {
        Self { jira_api }
    }

    fn fetch_sprint_pbis(&self, sprint_id: u64) -> Result<Vec<Pbi>, Box<dyn Error>> {
        let issues_response = self.jira_api.get_agile(&format!(
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
                    raw: fields.clone().dump(),
                    resolution: fields["resolution"]["name"].as_str().map(|s| s.to_string()),
                    components: fields["components"]
                        .members()
                        .filter_map(|c| c["name"].as_str().map(|s| s.to_string()))
                        .collect(),
                    creator: fields["creator"]["displayName"].as_string_or(""),
                    reporter: fields["reporter"]["displayName"].as_string_or(""),
                    project: fields["project"]["name"].as_string_or(""),
                });
            }
        }
        sort_by_status(&mut pbis);
        Ok(pbis)
    }
}

impl SprintService for DefaultSprintService {
    fn fetch_active_sprint_issues(&self, board_id: &str) -> Result<Sprint, Box<dyn Error>> {
        let sprints_response = self
            .jira_api
            .get_agile(&format!("board/{board_id}/sprint?state=active"))?;
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
            pbis: self.fetch_sprint_pbis(sprint_id)?,
            board_id: board_id.to_string(),
        })
    }

    fn fetch_pbi_details(&self, pbi: &mut Pbi) -> Result<(), Box<dyn Error>> {
        let response = self
            .jira_api
            .get(&format!("issue/{}?expand=changelog", pbi.key), 2)?;
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
}

fn last_in_progress_at(changelog: &json::JsonValue) -> Option<String> {
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
            raw: item.dump(),
            resolution: item["resolution"].as_str().map(|s| s.to_string()),
            components: item["components"]
                .members()
                .filter_map(|c| c.as_str().map(|s| s.to_string()))
                .collect(),
            creator: item["creator"].as_string_or(""),
            reporter: item["reporter"].as_string_or(""),
            project: item["project"].as_string_or(""),
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

fn extract_story_points(fields: &json::JsonValue) -> Option<f64> {
    fields["story_points"].as_f64().or_else(|| {
        let alias_field = get_alias("story_points".to_string())?;
        fields[alias_field.as_str()].as_f64()
    })
}

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
