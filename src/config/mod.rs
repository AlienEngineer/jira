use std::collections::HashMap;
use std::fs;
use std::io;
use std::io::Read;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::ioc;
use crate::jira;
use crate::prelude::Result;

/// Typed representation of the JIRA configuration file.
#[derive(Debug, Clone)]
pub struct JiraConfig {
    pub namespace: String,
    pub email: String,
    pub token: String,
    pub auth_mode: String,
    pub account_id: String,
    pub board_id: Option<String>,
    pub jira_version: Option<String>,
    /// Alias map: short name → full status name.
    pub alias: HashMap<String, String>,
    /// Transitions map: project code → (transition name → transition ID).
    pub transitions: HashMap<String, HashMap<String, i64>>,
}

impl JiraConfig {
    /// Load and parse the configuration file into a [`JiraConfig`].
    pub fn load() -> Result<JiraConfig> {
        let raw = parse_config();
        let mut alias = HashMap::new();
        for (k, v) in raw["alias"].entries() {
            if let Some(val) = v.as_str() {
                alias.insert(k.to_string(), val.to_string());
            }
        }
        let mut transitions: HashMap<String, HashMap<String, i64>> = HashMap::new();
        for (project, mapping) in raw["transitions"].entries() {
            let mut inner = HashMap::new();
            for (name, id) in mapping.entries() {
                if let Some(n) = id.as_i64() {
                    inner.insert(name.to_string(), n);
                }
            }
            transitions.insert(project.to_string(), inner);
        }
        Ok(JiraConfig {
            namespace: raw["namespace"].as_str().unwrap_or("").to_string(),
            email: raw["email"].as_str().unwrap_or("").to_string(),
            token: raw["token"].as_str().unwrap_or("").to_string(),
            auth_mode: raw["auth_mode"].as_str().unwrap_or("Basic").to_string(),
            account_id: raw["account_id"].as_str().unwrap_or("").to_string(),
            board_id: raw["board-id"].as_str().map(str::to_string),
            jira_version: raw["jira-version"].as_str().map(str::to_string),
            alias,
            transitions,
        })
    }
}

impl Default for JiraConfig {
    fn default() -> Self {
        JiraConfig {
            namespace: String::new(),
            email: String::new(),
            token: String::new(),
            auth_mode: "Basic".to_string(),
            account_id: String::new(),
            board_id: None,
            jira_version: None,
            alias: HashMap::new(),
            transitions: HashMap::new(),
        }
    }
}

/// Capitalize first letter of a word.
pub fn str_cap(s: String) -> String {
    format!("{}{}", (s[..1]).to_uppercase(), &s[1..])
}

fn get_old_config_file_name() -> Option<PathBuf> {
    home::home_dir().map(|path| path.join(".jira_configuration.json"))
}

pub fn get_config_file_name() -> String {
    use directories_next::ProjectDirs;

    // Try to get XDG-compliant config directory
    if let Some(proj_dirs) = ProjectDirs::from("", "", "jira") {
        let config_dir = proj_dirs.config_dir();
        let new_config_path = config_dir.join("configuration.json");

        // Create config directory if it doesn't exist
        if !config_dir.exists() {
            if let Err(e) = fs::create_dir_all(config_dir) {
                eprintln!("Warning: Failed to create config directory: {}", e);
            }
        }

        // Check if we need to migrate from old location
        if !new_config_path.exists() {
            if let Some(old_path) = get_old_config_file_name() {
                if old_path.exists() {
                    migrate_config(&old_path, &new_config_path);
                }
            }
        }

        return new_config_path.to_string_lossy().to_string();
    }

    // Fallback to old behavior if ProjectDirs fails
    let config_file_name: String = String::from(".jira_configuration.json");
    match home::home_dir() {
        Some(path) => format!("{}/{}", path.display(), config_file_name),
        None => config_file_name,
    }
}

/// Migrate config file from old location to new XDG-compliant location.
fn migrate_config(old_path: &Path, new_path: &Path) {
    match fs::copy(old_path, new_path) {
        Ok(_) => {
            println!("Configuration migrated to XDG-compliant location:");
            println!("  From: {}", old_path.display());
            println!("  To:   {}", new_path.display());
            println!("The old config file has been kept for backup purposes.");
            println!("You can safely delete it if the new location works correctly.");
        }
        Err(e) => {
            eprintln!("Warning: Failed to migrate config file: {}", e);
            eprintln!("Continuing with old location.");
        }
    }
}

/// Check if the config file exists.
fn check_config_exists() -> Result<bool> {
    Ok(fs::metadata(get_config_file_name()).is_ok())
}

/// Ensure `account_id` is populated in the config. If it is empty, attempt to
/// fetch it via the API and persist it. Call this only in commands that actually
/// need the account ID (e.g. sprint view, assign).
pub fn ensure_account_id() {
    let account_id = get_config("account_id".to_string());
    if account_id.is_empty() {
        match crate::get_instance!(ioc::global(), jira::user::CurrentUserService)
            .fetch_current_account_id()
        {
            Some(id) => {
                println!("Fetched account_id automatically: {id}");
                update_config("account_id".to_string(), id);
            }
            None => eprintln!(
                "Warning: account_id is not set and could not be fetched automatically.\n\
                 You can set it manually with: jira config account_id <your-id>"
            ),
        }
    }
}

/// Create configuration file by asking user with the required information.
fn create_config() -> Result<()> {
    let mut namespace = String::new();
    println!("Welcome to JIRA Terminal.");
    println!("Since this is your first run, we will ask you a few questions. ");
    println!("Please enter your hostname of JIRA. (Example: example.atlassian.net): ");
    io::stdin()
        .read_line(&mut namespace)
        .expect("Failed to read input.");

    println!("Please select your authentication mode:");
    println!("  1. Basic (email & password/token)");
    println!("  2. Bearer token");
    let mut auth_mode_input = String::new();
    io::stdin()
        .read_line(&mut auth_mode_input)
        .expect("Failed to read input.");
    let use_bearer = auth_mode_input.trim() == "2";
    let auth_mode = if use_bearer { "Bearer" } else { "Basic" };

    let (email, token) = if use_bearer {
        println!("Please enter your Bearer token: (The characters will not be visible in screen. Press enter after you entered the token) ");
        let bearer_token = rpassword::read_password().unwrap();
        (String::new(), bearer_token.trim().to_string())
    } else {
        let mut email = String::new();
        println!("Please enter your email address: ");
        io::stdin()
            .read_line(&mut email)
            .expect("Failed to read input.");
        println!("Please create an API Token from https://id.atlassian.com/manage-profile/security/api-tokens. If your JIRA setup does not have api tokens plugin, you can enter the password too. ");
        println!("Once created, enter your API Token: (The characters will not be visible in screen. Press enter after you entered the password or token) ");
        let password = rpassword::read_password().unwrap();
        let user_password = format!("{}:{}", email.trim(), password.trim());
        let b64 = base64::encode(user_password);
        (email.trim().to_string(), b64)
    };
    let configuration = json::object! {
        namespace: namespace.trim(),
        email: email.as_str(),
        token: token.as_str(),
        auth_mode: auth_mode,
        account_id: "",
        alias: {},
        transitions: {}
    };

    write_config(configuration);

    Ok(())
}

/// Write the updated configuration to the file.
///
/// # Arguments
///
/// * configuration - Configuration file.
fn write_config(configuration: json::JsonValue) {
    let config_json = json::stringify_pretty(configuration, 4);
    let mut file = fs::File::create(get_config_file_name()).expect("Unable to create config file.");
    file.write_all(config_json.as_bytes())
        .expect("Failed to write to file.");
}

/// Update the single configuration.
///
/// # Arguments
///
/// * key - Config key to update.
/// * value - Value to update with.
///
/// # Example
/// ```
/// update_config("key".to_string(), "value".to_string());
/// assert_eq!("value".to_string(), get_config("key".to_string()));
/// ```
pub fn update_config(key: String, value: String) {
    let mut config_value = parse_config();
    config_value[key] = value.into();
    write_config(config_value);
}

/// Update the object structure configuration.
///
/// # Arguments
///
/// * key - Config key to update.
/// * value - Value to update with.
///
/// # Example
/// ```
/// update_config("key".to_string(), "value".to_string());
/// assert_eq!("value".to_string(), get_config("key".to_string()));
/// ```
pub fn update_config_object(key: String, value: json::JsonValue) {
    let mut config_value = parse_config();
    config_value[key] = value;
    write_config(config_value);
}

/// Parse the config file to json object.
pub fn parse_config() -> json::JsonValue {
    let mut file = fs::File::open(get_config_file_name()).unwrap();
    let mut contents = String::new();
    file.read_to_string(&mut contents).unwrap();
    json::parse(&contents).unwrap()
}

/// Get the configuration for specified key. If the key does not exist, empty string is returned.
///
/// # Arguments:
/// * config - Configuration key.
///
/// # Example:
/// ```
/// let value = get_config("email".to_string());
/// ```
///
pub fn get_config(config: String) -> String {
    let config_value = &parse_config()[config];
    if config_value.is_string() {
        return String::from(config_value.as_str().unwrap());
    }
    String::from("")
}

/// Get the alias stored in configuration.
///
/// # Arguments
/// * alias - Alias value.
///
/// # Example
/// ```
/// assert!(get_alias("exists".to_string()).is_some());
/// assert!(get_alias("not_exists".to_string()).is_none());
/// ```
pub fn get_alias(alias: String) -> Option<String> {
    let config_value = &parse_config()["alias"][alias.to_lowercase()];
    if config_value.is_null() {
        None
    } else {
        Some(config_value.as_str().unwrap().to_string())
    }
}

/// Replace the value with alias value if it is alias, otherwise it will return the string as it
/// is.
///
/// # Arguments
/// * alias - Alias to replace or return as it is.
///
/// # Example
/// ```
/// assert_eq!(get_alias_or("ip".to_string()), "In Progress".to_string());
/// assert_eq!(get_alias_or("IP".to_string()), "In Progress".to_string());
/// assert_eq!(get_alias_or("In Progress".to_string(), "In Progress".to_string()));
/// ```
///
pub fn get_alias_or(alias: String) -> String {
    let alias_value = get_alias(alias.clone());
    match alias_value {
        Some(x) => x,
        None => alias,
    }
}

/// Set the alias to provided value and update the configuration.
///
/// # Arguments
///
/// * alias - Case insensitive alias to store or update.
/// * value - Value to associate with alias.
///
/// # Example
/// ```
/// set_alias("ip".to_string(), "In Progress".to_string());
/// ```
pub fn set_alias(alias: String, value: String) {
    let mut config_value = parse_config();
    config_value["alias"][alias.to_lowercase()] = value.into();
    write_config(config_value);
}

/// Remove the alias from configuration.
///
/// # Arguments
///
/// * alias - Name of alias
///
/// # Example
/// ```
/// remove_alias("name".to_string());
/// ```
pub fn remove_alias(alias: String) {
    let mut config_value = parse_config();
    let mut alias_object = config_value["alias"].clone();
    println!(
        "Removing alias ({}) with value: {}",
        alias,
        alias_object[alias.clone()]
    );
    alias_object.remove(alias.to_lowercase().as_str());
    config_value["alias"] = alias_object;
    write_config(config_value);
}

/// Completely replace the transition object with new value.
/// This function will be used to update or store transition codes for a project code.
///
/// # Arguments
///
/// * project_code - Project Code for JIRA. For a ticket ABC-123, project code is ABC.
/// * transitions - Json object for transitions.
///
/// # Example
/// ```
/// use json;
///
/// let transition = json::object! {
///     "backlog": 21,
///     "in progress": 31
/// }
/// set_transitions("ABC".to_string(), transition);
/// ```
pub fn set_transitions(project_code: String, transitions: json::JsonValue) {
    let mut config_value = parse_config();
    config_value["transitions"][project_code] = transitions;
    write_config(config_value);
}

/// Get the transitions for provided project code.
///
/// # Arguments
///
/// * project_code - Project Code for JIRA. For a ticket ABC-123, project code is ABC.
/// # Example
/// ```
/// let transitions = get_transitions("ABC".to_string());
/// ```
pub fn get_transitions(project_code: String) -> json::JsonValue {
    let config_value = &parse_config()["transitions"][project_code];
    config_value.clone()
}

/// Check if the transition exists for provided transition code in config file already.
///
/// # Arguments
///
/// * project_code - Project Code for JIRA. For a ticket ABC-123, project code is ABC.
/// * transition_name - Name of transition.
///
/// # Example
/// ```
/// assert!(transition_exists("ABC".to_string(), "in progress".to_string()));
/// ```
pub fn transition_exists(project_code: String, transition_name: String) -> bool {
    let config_value = &parse_config()["transitions"][project_code][transition_name];
    !config_value.is_null()
}

/// Ensure the config exists.
/// It will first check the config file exists.
/// If it does not, it will ask the user to create one.
pub fn ensure_config() -> Result<()> {
    let config_exists = check_config_exists()?;
    if !config_exists {
        create_config()?;
    }
    Ok(())
}

/// List all the provided alias.
pub fn list_all_alias() {
    let config_value = parse_config();
    println!("Listing alias saved for you: ");
    for (alias, value) in config_value["alias"].entries() {
        println!("* {:20} => {:?}", alias, value.as_str().unwrap_or(""));
    }
}
