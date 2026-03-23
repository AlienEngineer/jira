use crate::{
    config::{keymaps::KeyMapCollection, JiraConfig},
    prelude::Result,
};
use mlua::{Function, Lua, Value};
use std::path::Path;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::{fs, sync::OnceLock};

/// Default init.lua configuration content
const DEFAULT_INIT_LUA: &str = r#"-- keymaps can be added for different funcion calls in jira TUI.
-- e.g. jira.keymaps.set(key, function, [description], [scope])
-- key as string - only single key bindings are allows.
-- function as lua function, can bind to a jira.cmd
-- jira.cmd - holds pointers to all provided jira TUI behaviours.
-- jira.cmd.go_down, jira.go_up, jira.go_left, jira.go_right - move cursor in the list of plugins and plugin details.
-- jira.cmd.quit - quit the TUI or the current screen
-- jira.cmd.refresh - refresh the line of the currently selected pbi
-- jira.cmd.refresh_all - refresh all lines in the list of pbi's
-- jira.cmd.open_in_browser - open the currently selected pbi in the browser
-- jira.cmd.open_filter - open the filter menu to filter the list of pbi's
-- jira.cmd.start_work - start work on the currently selected pbi
--
-- GLOBAL
jira.keymaps.set("j", jira.cmd.go_down)
jira.keymaps.set("k", jira.cmd.go_up)
jira.keymaps.set("h", jira.cmd.go_left)
jira.keymaps.set("l", jira.cmd.go_right)

jira.keymaps.set("<DOWN>", jira.cmd.go_down)
jira.keymaps.set("<UP>", jira.cmd.go_up)
jira.keymaps.set("<LEFT>", jira.cmd.go_left)
jira.keymaps.set("<RIGHT>", jira.cmd.go_right)

function go_up_5()
	jira.cmd.go_up()
	jira.cmd.go_up()
	jira.cmd.go_up()
	jira.cmd.go_up()
	jira.cmd.go_up()
end
jira.keymaps.set("K", go_up_5)
function go_down_5()
	jira.cmd.go_down()
	jira.cmd.go_down()
	jira.cmd.go_down()
	jira.cmd.go_down()
	jira.cmd.go_down()
end
jira.keymaps.set("J", go_down_5)

jira.keymaps.set("q", jira.cmd.quit, "Quit")
jira.keymaps.set("<ESC>", jira.cmd.quit)

jira.keymaps.set("F", jira.cmd.refresh_all, "Refresh all", "Sprint")
jira.keymaps.set("p", jira.cmd.open_plugin_list, "Plugins", "Sprint")
jira.keymaps.set("e", jira.cmd.edit_selected_plugin, "Edit", "Plugin")

-- PBI LIST
jira.keymaps.set("/", jira.cmd.open_filter, "Filter", "PbiList")
jira.keymaps.set("F", jira.cmd.refresh_all, "Refresh all", "PbiList")

-- PBI
jira.keymaps.set("r", jira.cmd.open_raw_pbi_json, "Raw Json", "Pbi")
jira.keymaps.set("f", jira.cmd.refresh, "Refresh line", "Pbi")
jira.keymaps.set("o", jira.cmd.open_in_browser, "Browser", "Pbi")
jira.keymaps.set("<CR>", jira.cmd.start_work, "Start", "Pbi")
"#;

/// Commands that can be triggered from Lua and executed by SprintApp
#[derive(Debug, Clone)]
pub enum JiraCommand {
    GoUp,
    GoDown,
    GoLeft,
    GoRight,
    OpenPbiDetails,
    Quit,
    Refresh,
    RefreshAll,
    OpenInBrowser,
    OpenPluginList,
    OpenRawPbiJson,
    StartWork,
    OpenFilter,
    EditPluginSelected,
    Back,
    AssignToMe,
    Print(String),
}

static KEYMAP_COLLECTION: OnceLock<Arc<Mutex<KeyMapCollection>>> = OnceLock::new();
static LUA_RUNTIME: OnceLock<Lua> = OnceLock::new();
static COMMAND_SENDER: OnceLock<Mutex<Sender<JiraCommand>>> = OnceLock::new();
static COMMAND_RECEIVER: OnceLock<Mutex<Option<Receiver<JiraCommand>>>> = OnceLock::new();

/// Ensure the command channel is initialized without taking the receiver
fn ensure_command_channel_initialized() {
    COMMAND_RECEIVER.get_or_init(|| {
        let (tx, rx) = mpsc::channel();
        COMMAND_SENDER.get_or_init(|| Mutex::new(tx));
        Mutex::new(Some(rx))
    });
}

pub fn take_command_receiver() -> Option<Receiver<JiraCommand>> {
    ensure_command_channel_initialized();
    COMMAND_RECEIVER
        .get()
        .and_then(|m| m.lock().ok())
        .and_then(|mut guard| guard.take())
}

fn send_command(cmd: JiraCommand) {
    if let Some(sender) = COMMAND_SENDER.get() {
        if let Ok(tx) = sender.lock() {
            let _ = tx.send(cmd);
        }
    }
}

pub fn get_lua_runtime() -> Option<&'static Lua> {
    LUA_RUNTIME.get()
}

pub fn get_keymap_collection() -> Option<&'static Arc<Mutex<KeyMapCollection>>> {
    KEYMAP_COLLECTION.get()
}

pub fn init_lua_config() -> Result<()> {
    // Initialize command channel (don't take the receiver yet)
    ensure_command_channel_initialized();

    // Ensure default config exists
    ensure_default_lua_config()?;

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

    // Create jira.cmd table with command functions that can be bound to keys
    let cmd = lua.create_table()?;

    // Create command functions that send commands through the channel
    cmd.set(
        "go_up",
        lua.create_function(|_, ()| {
            send_command(JiraCommand::GoUp);
            Ok(())
        })?,
    )?;
    cmd.set(
        "go_down",
        lua.create_function(|_, ()| {
            send_command(JiraCommand::GoDown);
            Ok(())
        })?,
    )?;
    cmd.set(
        "go_left",
        lua.create_function(|_, ()| {
            send_command(JiraCommand::GoLeft);
            Ok(())
        })?,
    )?;
    cmd.set(
        "go_right",
        lua.create_function(|_, ()| {
            send_command(JiraCommand::GoRight);
            Ok(())
        })?,
    )?;
    cmd.set(
        "open_pbi_details",
        lua.create_function(|_, ()| {
            send_command(JiraCommand::OpenPbiDetails);
            Ok(())
        })?,
    )?;
    cmd.set(
        "quit",
        lua.create_function(|_, ()| {
            send_command(JiraCommand::Quit);
            Ok(())
        })?,
    )?;
    cmd.set(
        "refresh",
        lua.create_function(|_, ()| {
            send_command(JiraCommand::Refresh);
            Ok(())
        })?,
    )?;
    cmd.set(
        "refresh_all",
        lua.create_function(|_, ()| {
            send_command(JiraCommand::RefreshAll);
            Ok(())
        })?,
    )?;
    cmd.set(
        "open_in_browser",
        lua.create_function(|_, ()| {
            send_command(JiraCommand::OpenInBrowser);
            Ok(())
        })?,
    )?;
    cmd.set(
        "open_plugin_list",
        lua.create_function(|_, ()| {
            send_command(JiraCommand::OpenPluginList);
            Ok(())
        })?,
    )?;
    cmd.set(
        "open_raw_pbi_json",
        lua.create_function(|_, ()| {
            send_command(JiraCommand::OpenRawPbiJson);
            Ok(())
        })?,
    )?;
    cmd.set(
        "start_work",
        lua.create_function(|_, ()| {
            send_command(JiraCommand::StartWork);
            Ok(())
        })?,
    )?;
    cmd.set(
        "open_filter",
        lua.create_function(|_, ()| {
            send_command(JiraCommand::OpenFilter);
            Ok(())
        })?,
    )?;
    cmd.set(
        "edit_selected",
        lua.create_function(|_, ()| {
            send_command(JiraCommand::EditPluginSelected);
            Ok(())
        })?,
    )?;
    cmd.set(
        "edit_selected_plugin",
        lua.create_function(|_, ()| {
            send_command(JiraCommand::EditPluginSelected);
            Ok(())
        })?,
    )?;
    cmd.set(
        "back",
        lua.create_function(|_, ()| {
            send_command(JiraCommand::Back);
            Ok(())
        })?,
    )?;
    cmd.set(
        "assign_to_me",
        lua.create_function(|_, ()| {
            send_command(JiraCommand::AssignToMe);
            Ok(())
        })?,
    )?;
    cmd.set(
        "print",
        lua.create_function(|_, msg: String| {
            send_command(JiraCommand::Print(msg));
            Ok(())
        })?,
    )?;

    jira.set("cmd", cmd)?;

    let keymaps = Arc::new(Mutex::new(KeyMapCollection::new()));
    let keymaps_for_function = keymaps.clone();

    // jira.keymaps.set(key, lua_function, description?) - for Lua function callbacks
    let set_function = lua.create_function(
        move |lua, (key, func, label, scope): (String, Function, Option<String>, Option<String>)| {
            // Store the function in the Lua registry so it persists
            let registry_key = lua
                .create_registry_value(func)
                .map_err(|e| mlua::Error::runtime(format!("Failed to store function: {}", e)))?;

            let mut guard = keymaps_for_function
                .lock()
                .map_err(|e| mlua::Error::runtime(format!("Failed to lock keymaps: {}", e)))?;

            guard
                .set(&key, registry_key, label.as_deref(), scope.as_deref())
                .map_err(|e| mlua::Error::runtime(format!("Failed to set keymap: {}", e)))?;
            Ok(())
        },
    )?;

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

    Ok(())
}

/// Execute a keymap action and return the result.
/// Calls the Lua function associated with the keymap.
pub fn execute_keymap_action(keymap: &crate::config::keymaps::KeyMap) -> Result<String> {
    let lua = get_lua_runtime().ok_or("Lua runtime not initialized")?;
    let func: Function = lua.registry_value(&keymap.func)?;
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

/// Ensure the Lua config directory exists and create default init.lua if not present.
pub fn ensure_default_lua_config() -> Result<()> {
    let config_dir = get_init_lua_config_path();
    let config_path = Path::new(&config_dir);
    let init_lua_path = config_path.join("init.lua");

    // Create config directory if it doesn't exist
    if !config_path.exists() {
        fs::create_dir_all(config_path)?;
        println!("Created Lua config directory: {}", config_dir);
    }

    // Create default init.lua if it doesn't exist
    if !init_lua_path.exists() {
        fs::write(&init_lua_path, DEFAULT_INIT_LUA)?;
        println!("Created default init.lua at: {}", init_lua_path.display());
    }

    Ok(())
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
