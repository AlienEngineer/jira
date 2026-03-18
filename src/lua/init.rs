use crate::{
    config::{keymaps::KeyMapCollection, JiraConfig},
    lua::execution::{LuaIntegration, ScriptIntegration},
    prelude::Result,
};
use std::sync::{Arc, Mutex};
use std::{fs, sync::OnceLock};

static KEYMAP_COLLECTION: OnceLock<KeyMapCollection> = OnceLock::new();

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

    let keymaps = Arc::new(Mutex::new(KeyMapCollection::new()));
    let keymaps_clone = keymaps.clone();

    let set = integration
        .lua
        .create_function(move |_, (key, script): (String, String)| {
            let mut guard = keymaps
                .lock()
                .map_err(|e| mlua::Error::runtime(format!("Failed to lock keymaps: {}", e)))?;
            guard
                .set(&key, &script, "User defined keymap")
                .map_err(|e| mlua::Error::runtime(format!("Failed to set keymap: {}", e)))?;
            Ok(())
        })?;

    let keymap_functions = integration.make_table();
    keymap_functions.set("set", set)?;
    jira.set("keymaps", keymap_functions)?;

    integration.set_global("jira", jira);
    for script in scripts.unwrap_or_default() {
        if let Err(e) = integration.exec_script(&script) {
            eprintln!("Error executing Lua script: {}", e);
        }
    }

    // Drop the Lua integration to release the Arc reference held by the closure
    drop(integration);

    KEYMAP_COLLECTION
        .set(
            Arc::try_unwrap(keymaps_clone)
                .expect("Arc still has multiple owners")
                .into_inner()
                .expect("Mutex was poisoned"),
        )
        .ok();

    println!("Lua configuration initialized from {}", destination_dir);

    if let Some(collection) = KEYMAP_COLLECTION.get() {
        let keymaps = collection.get_keymaps();
        for keymap in keymaps {
            println!("Registered keymap: {} -> {}", keymap.key, keymap.script);
        }
    } else {
        println!("No keymaps loaded.");
    }

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
