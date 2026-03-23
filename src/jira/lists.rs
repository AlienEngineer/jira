extern crate clap;
use clap::ArgMatches;
use json::JsonValue;
use std::sync::Arc;

use crate::config;
use crate::ioc::interface::Interface;
use crate::jira::api::JiraApi;
use crate::jira::pbi::{last_in_progress_at, Pbi};

// ── Filter fields ─────────────────────────────────────────────────────────────

/// All filterable fields in display order: (key, label).
pub const FILTER_FIELDS: &[(&str, &str)] = &[
    ("sprint", "Sprint"),
    ("project", "Project"),
    ("status", "Status"),
    ("assignee", "Assignee"),
    ("component", "Component"),
    ("labels", "Labels"),
    ("priority", "Priority"),
    ("reporter", "Reporter"),
    ("type", "Issue Type"),
    ("epic", "Epic"),
    ("text", "Text Search"),
    ("jql", "JQL Query"),
];

// ── ListFilter ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct ListFilter {
    pub sprint: Vec<String>,
    pub project: Vec<String>,
    pub status: Vec<String>,
    pub assignee: Vec<String>,
    pub component: Vec<String>,
    pub labels: Vec<String>,
    pub priority: Vec<String>,
    pub reporter: Vec<String>,
    pub issue_type: Vec<String>,
    pub epic: Vec<String>,
    pub text: Option<String>,
    pub jql: Option<String>,
    pub me: bool,
    pub count: u32,
    pub offset: u32,
}

impl ListFilter {
    pub fn from_matches(matches: &ArgMatches) -> Self {
        Self {
            sprint: multi_values(matches, "sprint"),
            project: multi_values(matches, "project"),
            status: multi_values(matches, "status"),
            assignee: multi_values(matches, "assignee"),
            component: multi_values(matches, "component"),
            labels: multi_values(matches, "labels"),
            priority: multi_values(matches, "priority"),
            reporter: multi_values(matches, "reporter"),
            issue_type: multi_values(matches, "type"),
            epic: multi_values(matches, "epic"),
            text: matches.value_of("text").map(|s| s.to_string()),
            jql: matches.value_of("jql").map(|s| s.to_string()),
            me: matches.is_present("me"),
            count: matches
                .value_of("count")
                .unwrap_or("50")
                .parse()
                .unwrap_or(50),
            offset: matches
                .value_of("offset")
                .unwrap_or("0")
                .parse()
                .unwrap_or(0),
        }
    }

    pub fn to_jql(&self) -> String {
        let mut parts: Vec<String> = vec![];
        if self.me {
            parts.push("assignee = currentUser()".to_string());
        }
        push_jql_in(&mut parts, "sprint", &self.sprint);
        push_jql_in(&mut parts, "project", &self.project);
        push_jql_in(&mut parts, "status", &self.status);
        push_jql_in(&mut parts, "assignee", &self.assignee);
        push_jql_in(&mut parts, "component", &self.component);
        push_jql_in(&mut parts, "labels", &self.labels);
        push_jql_in(&mut parts, "priority", &self.priority);
        push_jql_in(&mut parts, "reporter", &self.reporter);
        push_jql_in(&mut parts, "issuetype", &self.issue_type);
        if !self.epic.is_empty() {
            let vals: Vec<String> = self
                .epic
                .iter()
                .map(|v| format!("\"{}\"", config::get_alias_or(v.clone())))
                .collect();
            parts.push(format!("\"epic link\" in ({})", vals.join(",")));
        }
        if let Some(text) = &self.text {
            parts.push(format!("text ~ \"{}\"", config::get_alias_or(text.clone())));
        }
        if let Some(jql) = &self.jql {
            parts.push(config::get_alias_or(jql.clone()));
        }
        parts.join(" AND ")
    }

    /// Current value for a field as a display string (comma-joined for multi-value).
    pub fn get_display(&self, key: &str) -> String {
        match key {
            "sprint" => self.sprint.join(", "),
            "project" => self.project.join(", "),
            "status" => self.status.join(", "),
            "assignee" => self.assignee.join(", "),
            "component" => self.component.join(", "),
            "labels" => self.labels.join(", "),
            "priority" => self.priority.join(", "),
            "reporter" => self.reporter.join(", "),
            "type" => self.issue_type.join(", "),
            "epic" => self.epic.join(", "),
            "text" => self.text.clone().unwrap_or_default(),
            "jql" => self.jql.clone().unwrap_or_default(),
            _ => String::new(),
        }
    }

    /// Set a field from a comma-separated string (empty string clears the field).
    pub fn set_from_str(&mut self, key: &str, value: &str) {
        let vals: Vec<String> = value
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        match key {
            "sprint" => self.sprint = vals,
            "project" => self.project = vals,
            "status" => self.status = vals,
            "assignee" => self.assignee = vals,
            "component" => self.component = vals,
            "labels" => self.labels = vals,
            "priority" => self.priority = vals,
            "reporter" => self.reporter = vals,
            "type" => self.issue_type = vals,
            "epic" => self.epic = vals,
            "text" => self.text = vals.into_iter().next(),
            "jql" => self.jql = vals.into_iter().next(),
            _ => {}
        }
    }

    pub fn clear_field(&mut self, key: &str) {
        self.set_from_str(key, "");
    }
}

// ── Service ───────────────────────────────────────────────────────────────────

pub trait ListService: Interface {
    /// Fetch issues matching `filter` from the Jira API.
    fn fetch_with_filter(&self, filter: &ListFilter) -> Result<Vec<Pbi>, String>;

    /// Get access to the underlying JiraApi.
    fn jira_api(&self) -> &dyn JiraApi;

    /// Convenience wrapper used by the non-TUI code path.
    fn list_issues(&self, matches: &ArgMatches) -> Vec<Pbi> {
        let filter = ListFilter::from_matches(matches);
        self.fetch_with_filter(&filter).unwrap_or_else(|e| {
            eprintln!("Error searching tickets: {e}");
            std::process::exit(1);
        })
    }
}

pub struct DefaultListService {
    jira_api: Arc<dyn JiraApi>,
}

impl DefaultListService {
    pub fn new(jira_api: Arc<dyn JiraApi>) -> Self {
        Self { jira_api }
    }
}

fn pbi_from_json(issue: &json::JsonValue) -> Pbi {
    let fields = &issue["fields"];
    Pbi {
        key: issue["key"].as_str().unwrap_or("-").to_string(),
        summary: fields["summary"].as_str().unwrap_or("-").to_string(),
        status: fields["status"]["name"].as_str().unwrap_or("-").to_string(),
        assignee: fields["assignee"]["displayName"]
            .as_str()
            .unwrap_or("-")
            .to_string(),
        issue_type: fields["issuetype"]["name"]
            .as_str()
            .unwrap_or("-")
            .to_string(),
        description: fields["description"].as_str().map(|s| s.to_string()),
        priority: fields["priority"]["name"].as_str().map(|s| s.to_string()),
        story_points: None,
        labels: fields["labels"]
            .members()
            .filter_map(|l| l.as_str().map(|s| s.to_string()))
            .collect(),
        loaded: false,
        in_progress_at: last_in_progress_at(&issue["changelog"]),
        resolved_at: fields["resolutiondate"].as_str().map(|s| s.to_string()),
        raw: json::stringify_pretty::<JsonValue>(fields.clone(), 2),

        resolution: fields["resolution"]["name"].as_str().map(|s| s.to_string()),
        components: fields["components"]
            .members()
            .filter_map(|c| c["name"].as_str().map(|s| s.to_string()))
            .collect(),
        creator: fields["creator"]["displayName"]
            .as_str()
            .unwrap_or("-")
            .to_string(),
        reporter: fields["reporter"]["displayName"]
            .as_str()
            .unwrap_or("-")
            .to_string(),
        project: fields["project"]["name"]
            .as_str()
            .unwrap_or("-")
            .to_string(),
    }
}

impl ListService for DefaultListService {
    fn fetch_with_filter(&self, filter: &ListFilter) -> Result<Vec<Pbi>, String> {
        let jql = filter.to_jql();
        let count = filter.count;
        let offset = filter.offset;

        let response = self
            .jira_api
            .get_v3(&format!(
                "search?maxResults={count}&startAt={offset}&expand=changelog&jql={jql}"
            ))
            .map_err(|e| e.to_string())?;

        let issues = &response["issues"];
        if !issues.is_array() {
            return Ok(vec![]);
        }
        Ok(issues.members().map(pbi_from_json).collect())
    }

    fn jira_api(&self) -> &dyn JiraApi {
        self.jira_api.as_ref()
    }
}

fn multi_values(matches: &ArgMatches, field: &str) -> Vec<String> {
    matches
        .values_of(field)
        .map(|vals| vals.map(|s| s.to_string()).collect())
        .unwrap_or_default()
}

fn push_jql_in(parts: &mut Vec<String>, field: &str, values: &[String]) {
    if values.is_empty() {
        return;
    }
    let quoted: Vec<String> = values
        .iter()
        .map(|v| format!("\"{}\"", config::get_alias_or(v.clone())))
        .collect();
    parts.push(format!("{field} in ({})", quoted.join(",")));
}
