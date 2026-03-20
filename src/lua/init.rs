use crate::{
    config::{keymaps::KeyMapCollection, JiraConfig},
    prelude::Result,
};
use mlua::{Function, Lua, Value};
use std::sync::{Arc, Mutex};
use std::{fs, sync::OnceLock};

static KEYMAP_COLLECTION: OnceLock<Arc<Mutex<KeyMapCollection>>> = OnceLock::new();
static LUA_RUNTIME: OnceLock<Lua> = OnceLock::new();

pub fn get_lua_runtime() -> Option<&'static Lua> {
    LUA_RUNTIME.get()
}

pub fn get_keymap_collection() -> Option<&'static Arc<Mutex<KeyMapCollection>>> {
    KEYMAP_COLLECTION.get()
}

pub fn init_lua_config() -> Result<()> {
    let destination_dir = get_init_lua_config_path();
    let scripts = load_config_scripts(&destination_dir);

    // Create a long-lived Lua runtime that will be stored statically
    let lua = Lua::new();

    let jira = lua.create_table()?;
    let config = lua.create_table()?;

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
    let keymaps_for_function = keymaps.clone();

    // jira.keymaps.set_function(key, lua_function) - for Lua function callbacks
    let set_function =
        lua.create_function(move |lua, (key, func, label): (String, Function, String)| {
            // Store the function in the Lua registry so it persists
            let registry_key = lua
                .create_registry_value(func)
                .map_err(|e| mlua::Error::runtime(format!("Failed to store function: {}", e)))?;

            let mut guard = keymaps_for_function
                .lock()
                .map_err(|e| mlua::Error::runtime(format!("Failed to lock keymaps: {}", e)))?;

            guard
                .set(&key, registry_key, &label)
                .map_err(|e| mlua::Error::runtime(format!("Failed to set keymap: {}", e)))?;
            Ok(())
        })?;

    let keymap_functions = lua.create_table()?;
    keymap_functions.set("set", set_function)?;
    jira.set("keymaps", keymap_functions)?;

    lua.globals().set("jira", jira)?;

    // Execute all config scripts
    for script in scripts.unwrap_or_default() {
        if let Err(e) = lua.load(&script).exec() {
            eprintln!("Error executing Lua script: {}", e);
        }
    }

    // Store the Lua runtime globally so we can call functions later
    LUA_RUNTIME.set(lua).ok();

    // Store the keymap collection (keep it wrapped in Arc<Mutex<>> since the closure holds a reference)
    KEYMAP_COLLECTION.set(keymaps).ok();

    println!("Lua configuration initialized from {}", destination_dir);

    bump_key_maps();

    Ok(())
}

pub fn bump_key_maps() {
    if let Some(collection) = KEYMAP_COLLECTION.get() {
        let guard = collection.lock().expect("Failed to lock keymaps");
        let keymaps = guard.get_keymaps();
        for keymap in keymaps {
            println!("Registered keymap: {} -> function", keymap.key);
        }
    } else {
        println!("No keymaps loaded.");
    }
}

/// Execute a keymap action and return the result.
/// Calls the Lua function associated with the keymap.
pub fn execute_keymap_action(keymap: &crate::config::keymaps::KeyMap) -> Result<String> {
    let lua = get_lua_runtime().ok_or("Lua runtime not initialized")?;
    let func: Function = lua.registry_value(&*keymap.func)?;
    let result: Value = func.call(())?;
    match result {
        Value::String(s) => Ok(s.to_str()?.to_string()),
        Value::Nil => Ok(String::new()),
        other => Ok(format!("{:?}", other)),
    }
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
