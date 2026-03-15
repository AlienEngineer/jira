extern crate clap;
use clap::ArgMatches;
use std::sync::Arc;

use crate::config;
use crate::ioc::interface::Interface;
use crate::jira::api::JiraApi;
use crate::jira::pbi::Pbi;

// ── Service ───────────────────────────────────────────────────────────────────

pub trait ListService: Interface {
    fn list_issues(&self, matches: &ArgMatches) -> Vec<Pbi>;
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
        in_progress_at: None,
        resolved_at: fields["resolutiondate"].as_str().map(|s| s.to_string()),
        raw: fields.dump(),
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

fn form_jql(matches: &ArgMatches) -> String {
    let mut criterias: Vec<String> = vec![];
    let fields = vec![
        "assignee",
        "component",
        "labels",
        "parent",
        "filter",
        "priority",
        "project",
        "reporter",
        "sprint",
        "status",
        "type",
        "epic",
        "jql",
        "text",
    ];
    if matches.is_present("me") {
        criterias.push("assignee = currentUser()".to_string());
    }
    for field in fields {
        if field == "jql" {
            let jql_option = matches.value_of("jql");
            if let Some(jql) = jql_option {
                criterias.push(config::get_alias_or(jql.to_string()));
            }
        } else if field == "text" {
            let jql_option = matches.value_of("text");
            if let Some(jql_option) = jql_option {
                criterias.push(format!(
                    "text ~ \"{}\"",
                    config::get_alias_or(jql_option.to_string())
                ));
            }
        } else if let Some(values) = matches.values_of(field) {
            let mut options: Vec<String> = vec![];
            for value in values {
                options.push(format!("\"{}\"", config::get_alias_or(value.to_string())));
            }
            if field == "epic" {
                criterias.push(format!("\"epic link\" in ({})", options.join(",")));
            } else {
                criterias.push(format!("{} in ({})", field, options.join(",")));
            }
        }
    }
    criterias.join(" AND ")
}

impl ListService for DefaultListService {
    fn list_issues(&self, matches: &ArgMatches) -> Vec<Pbi> {
        let jql = form_jql(matches);

        let offset = matches
            .value_of("offset")
            .unwrap_or("0")
            .parse::<u32>()
            .unwrap_or_else(|_| {
                eprintln!("Invalid option passed to offset.");
                std::process::exit(1);
            });
        let count = matches
            .value_of("count")
            .unwrap_or("50")
            .parse::<u32>()
            .unwrap_or_else(|_| {
                eprintln!("Invalid option passed to count.");
                std::process::exit(1);
            });

        let search_response = self
            .jira_api
            .get_v3(&format!(
                "search?maxResults={count}&startAt={offset}&jql={jql}"
            ))
            .unwrap_or_else(|e| {
                eprintln!("Error occurred when searching tickets: {e}");
                std::process::exit(1);
            });

        if matches.is_present("alias") {
            let alias_name = matches.value_of("alias").unwrap();
            config::set_alias(alias_name.to_string(), jql);
            println!("Current filter is now set with value {alias_name}");
            println!("You can use jira list --jql \"{alias_name}\" to reuse this filter.");
        }

        let issues = &search_response["issues"];
        if !issues.is_array() {
            return vec![];
        }

        issues.members().map(pbi_from_json).collect()
    }
}
