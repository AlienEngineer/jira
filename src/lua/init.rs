use crate::{
    config::{get_config_file_name, JiraConfig},
    lua::execution::{LuaIntegration, ScriptIntegration},
    prelude::Result,
};
use std::fs;

pub fn init_lua_config() -> Result<()> {
    let destination_dir = get_init_lua_config_path();
    let scripts = load_config_scripts(&destination_dir);
    let integration = LuaIntegration::new();
    let jira = integration.make_table();

    let config = integration.make_table();

    let jira_config = JiraConfig::load()?;
    config.set("namespace", jira_config.namespace.clone())?;
    config.set("email", jira_config.email.clone())?;
    config.set("token", jira_config.token.clone())?;
    config.set("auth_mode", jira_config.auth_mode.clone())?;
    config.set("account_id", jira_config.account_id.clone())?;
    config.set("board_id", jira_config.board_id.clone())?;
    config.set("jira_version", jira_config.jira_version.clone())?;
    config.set("alias", jira_config.alias.clone())?;
    config.set("transitions", jira_config.transitions.clone())?;

    jira.set("config", config)?;
    jira.set("version", "2.4.20")?;

    integration.set_global("jira", jira);
    for script in scripts.unwrap_or_default() {
        if let Err(e) = integration.exec_script(&script) {
            eprintln!("Error executing Lua script: {}", e);
        }
    }
    println!("Lua configuration initialized from {}", destination_dir);
    Ok(())
}

pub fn get_init_lua_config_path() -> String {
    let plugins_folder = ".config";
    match home::home_dir() {
        Some(path) => format!("{}/{}/jira", path.display(), plugins_folder),
        None => plugins_folder.to_string(),
    }
}

fn load_config_scripts(path: &str) -> Result<Vec<String>> {
    let mut plugins = Vec::new();
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let entry_path = entry.path();
        if entry_path.extension().and_then(|s| s.to_str()) == Some("lua") {
            let script = fs::read_to_string(&entry_path)?;
            plugins.push(script);
        }
    }
    Ok(plugins)
}
