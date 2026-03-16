use crate::config::{update_config, JiraConfig};
use clap::{App, Arg, SubCommand};
use std::process;

/// Configure user-specific settings for this CLI.
/// For example, use `jira-version` to set the Jira API version used for requests.
pub fn subcommand() -> App<'static, 'static> {
    SubCommand::with_name("config")
        .about("Update or display configuration values.")
        .subcommand(SubCommand::with_name("show").about("Display the current configuration."))
        .arg(
            Arg::with_name("KEY")
                .help("Configuration key to update (e.g. jira-version)")
                .required(false)
                .index(1),
        )
        .arg(
            Arg::with_name("VALUE")
                .help("Value to set (e.g. 2, v2, 3, v3)")
                .required(false)
                .index(2),
        )
}

/// Parse a jira-version string (2, v2, 3, v3) into a canonical version number string.
fn parse_jira_version(value: &str) -> Option<&'static str> {
    match value.to_lowercase().trim_start_matches('v') {
        "2" => Some("2"),
        "3" => Some("3"),
        _ => None,
    }
}

fn show_config() {
    let config = JiraConfig::load();
    match config {
        Err(e) => eprintln!("Failed to load configuration: {e}"),
        Ok(c) => {
            println!("{:<20} {}", "namespace:", c.namespace);
            println!("{:<20} {}", "email:", c.email);
            println!("{:<20} {}", "auth_mode:", c.auth_mode);
            println!("{:<20} {}", "account-id:", c.account_id);
            println!("{:<20} {}", "board-id:", c.board_id.unwrap_or_default());
            println!(
                "{:<20} {}",
                "jira-version:",
                c.jira_version.unwrap_or_default()
            );
            println!("{:<20} {}", "token:", mask_token(&c.token));

            if c.alias.is_empty() {
                println!("{:<20} (none)", "alias:");
            } else {
                println!("alias:");
                let mut aliases: Vec<_> = c.alias.iter().collect();
                aliases.sort_by_key(|(k, _)| k.as_str());
                for (k, v) in aliases {
                    println!("  {k:<18} {v}");
                }
            }

            if c.transitions.is_empty() {
                println!("{:<20} (none)", "transitions:");
            } else {
                println!("transitions:");
                let mut projects: Vec<_> = c.transitions.iter().collect();
                projects.sort_by_key(|(k, _)| k.as_str());
                for (project, mapping) in projects {
                    println!("  {project}:");
                    let mut entries: Vec<_> = mapping.iter().collect();
                    entries.sort_by_key(|(k, _)| k.as_str());
                    for (name, id) in entries {
                        println!("    {name:<16} {id}");
                    }
                }
            }
            println!("{:<20} {}", "config file:", c.path);
        }
    }
}

/// Show only the last 4 characters of the token to avoid leaking credentials.
fn mask_token(token: &str) -> String {
    if token.len() <= 4 {
        return "*".repeat(token.len());
    }
    format!("{}…{}", "*".repeat(8), &token[token.len() - 4..])
}

pub fn handle(matches: &clap::ArgMatches) {
    if matches.subcommand_matches("show").is_some() {
        show_config();
        return;
    }

    let key = match matches.value_of("KEY") {
        Some(k) => k,
        None => {
            show_config();
            return;
        }
    };
    let value = match matches.value_of("VALUE") {
        Some(v) => v,
        None => {
            eprintln!("VALUE is required when setting a config key.");
            process::exit(1);
        }
    };

    match key {
        "jira-version" => match parse_jira_version(value) {
            Some(version) => {
                update_config("jira-version".to_string(), version.to_string());
                println!("Jira API version set to {version}.");
            }
            None => {
                eprintln!("Invalid jira-version '{value}'. Accepted values: 2, 3.");
                process::exit(1);
            }
        },
        "account-id" => {
            update_config("account-id".to_string(), value.to_string());
            println!("Account ID updated to {value}");
        }
        "board-id" => {
            update_config("board-id".to_string(), value.to_string());
            println!("Board id updated to {value}");
            println!("Now you can list your current sprint using 'jira sprint'!");
        }
        _ => {
            eprintln!("Unknown config key '{key}'.");
            process::exit(1);
        }
    }
}
