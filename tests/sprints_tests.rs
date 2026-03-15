use jira::jira::api::JiraApi;
use jira::jira::sprint::{DefaultSprintService, SprintService};
use std::sync::Arc;

mod tests {
    use jira::jira::pbi::Pbi;

    use super::*;

    use std::collections::HashMap;
    use std::error::Error;
    use std::io;

    fn agile_key(endpoint: &str) -> String {
        format!("agile:{endpoint}")
    }

    fn api_key(version: u8, endpoint: &str) -> String {
        format!("api:{version}:{endpoint}")
    }

    struct JiraFakeGateway {
        responses: HashMap<String, json::JsonValue>,
    }

    impl JiraFakeGateway {
        fn new(responses: HashMap<String, json::JsonValue>) -> Self {
            Self { responses }
        }

        fn response(&self, key: &str) -> Result<json::JsonValue, Box<dyn Error>> {
            self.responses.get(key).cloned().ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("missing fake Jira response for key '{key}'"),
                )
                .into()
            })
        }
    }

    impl JiraApi for JiraFakeGateway {
        fn get(&self, endpoint: &str, version: u8) -> Result<json::JsonValue, Box<dyn Error>> {
            self.response(&api_key(version, endpoint))
        }

        fn post(
            &self,
            endpoint: &str,
            _json_value: json::JsonValue,
            version: u8,
        ) -> Result<String, Box<dyn Error>> {
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                format!("POST is not faked for api:{version}:{endpoint}"),
            )
            .into())
        }

        fn put(
            &self,
            endpoint: &str,
            _json_value: json::JsonValue,
            version: u8,
        ) -> Result<String, Box<dyn Error>> {
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                format!("PUT is not faked for api:{version}:{endpoint}"),
            )
            .into())
        }

        fn get_agile(&self, endpoint: &str) -> Result<json::JsonValue, Box<dyn Error>> {
            self.response(&agile_key(endpoint))
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

    #[test]
    fn fetch_active_sprint_issues_uses_fake_jira_responses() {
        let service = DefaultSprintService::new(Arc::new(JiraFakeGateway::new(HashMap::from([
            (
                agile_key("board/7/sprint?state=active"),
                active_sprint_response(),
            ),
            (
                agile_key("sprint/42/issue?maxResults=500&expand=changelog"),
                sprint_issues_response(),
            ),
        ]))));

        let sprint = service
            .fetch_active_sprint_issues("7")
            .expect("expected fake Jira responses to build a sprint");

        assert_eq!(sprint.name, "Platform Sprint");
        assert_eq!(sprint.goal, "Ship the sprint fetch refactor");
        assert_eq!(sprint.end_date, "2026-03-20");
        assert_eq!(sprint.board_id, "7");
        assert_eq!(sprint.pbis.len(), 2);
        assert_eq!(sprint.pbis[0].key, "JIRA-2");
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
        let service = DefaultSprintService::new(Arc::new(JiraFakeGateway::new(HashMap::from([(
            agile_key("board/7/sprint?state=active"),
            json::object! { "values": [] },
        )]))));

        let error = service
            .fetch_active_sprint_issues("7")
            .expect_err("expected an error when the fake Jira response has no active sprint");

        assert_eq!(
            error.to_string(),
            "No active sprint found for the given board."
        );
    }

    #[test]
    fn fetch_pbi_details_uses_fake_jira_response() {
        let service = DefaultSprintService::new(Arc::new(JiraFakeGateway::new(HashMap::from([(
            api_key(2, "issue/JIRA-7?expand=changelog"),
            json::object! {
                "fields": {
                    "description": "Defer helper usage to the IoC container",
                    "priority": { "name": "Highest" },
                    "story_points": 8.0,
                    "labels": ["ioc", "refactor"],
                    "status": { "name": "In Progress" },
                    "assignee": { "displayName": "Morgan" },
                    "resolutiondate": null
                },
                "changelog": {
                    "histories": [
                        {
                            "created": "2026-03-11T12:00:00.000+0000",
                            "items": [
                                {
                                    "field": "status",
                                    "toString": "In Progress"
                                }
                            ]
                        }
                    ]
                }
            },
        )]))));
        let mut pbi = Pbi {
            key: "JIRA-7".to_string(),
            summary: "Refactor Jira transport".to_string(),
            status: "Open".to_string(),
            assignee: "Unassigned".to_string(),
            issue_type: "Task".to_string(),
            description: None,
            priority: None,
            story_points: None,
            labels: vec![],
            loaded: false,
            in_progress_at: None,
            resolved_at: None,
            raw: "".to_string(),
        };

        service
            .fetch_pbi_details(&mut pbi)
            .expect("expected fake Jira response to populate PBI details");

        assert_eq!(
            pbi.description.as_deref(),
            Some("Defer helper usage to the IoC container")
        );
        assert_eq!(pbi.priority.as_deref(), Some("Highest"));
        assert_eq!(pbi.story_points, Some(8.0));
        assert_eq!(pbi.labels, vec!["ioc", "refactor"]);
        assert_eq!(pbi.status, "In Progress");
        assert_eq!(pbi.assignee, "Morgan");
        assert!(pbi.loaded);
        assert_eq!(
            pbi.in_progress_at.as_deref(),
            Some("2026-03-11T12:00:00.000+0000")
        );
    }
}
