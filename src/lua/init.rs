use crate::{
    config::{keymaps::KeyMapCollection, JiraConfig},
    jira::{pbi::Pbi, sprint::Sprint},
    prelude::Result,
};
use mlua::{Function, Lua, Table, Value};
use std::path::Path;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::{fs, sync::OnceLock};

/// Default init.lua configuration content
const DEFAULT_INIT_LUA: &str = r#"
-- keymaps can be added for different funcion calls in jira TUI.
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
--
-- Full context reference (all fields available as jira_context.*):
--
-- CONFIG
--   jira_context.config.namespace      -- e.g. "mycompany.atlassian.net"
--   jira_context.config.email          -- authenticated user's email
--   jira_context.config.token          -- API token (handle with care)
--   jira_context.config.auth_mode      -- "Basic" or "Bearer"
--   jira_context.config.account_id     -- Jira account ID of the current user
--   jira_context.config.board_id       -- active board ID (string or nil)
--   jira_context.config.jira_version   -- "cloud" or "server" (or nil)
--   jira_context.config.alias          -- table: short name → full status name
--   jira_context.config.transitions    -- table: project → (name → id)
--
-- SPRINT
--   jira_context.sprint.name           -- sprint name
--   jira_context.sprint.goal           -- sprint goal text
--   jira_context.sprint.end_date       -- ISO-8601 end date string
--   jira_context.sprint.board_id       -- board ID this sprint belongs to
--   jira_context.sprint.pbis           -- array of all PBI tables (see below)
--
-- SELECTED PBI  (nil when none is selected)
--   jira_context.selected_pbi.key          -- e.g. "PROJ-123"
--   jira_context.selected_pbi.summary      -- issue title
--   jira_context.selected_pbi.status       -- e.g. "In Progress"
--   jira_context.selected_pbi.assignee     -- assignee display name
--   jira_context.selected_pbi.issue_type   -- e.g. "Story", "Bug"
--   jira_context.selected_pbi.description  -- full description (may be nil)
--   jira_context.selected_pbi.priority     -- e.g. "High" (may be nil)
--   jira_context.selected_pbi.story_points -- number (may be nil)
--   jira_context.selected_pbi.labels       -- array of label strings
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

function jira_print(msg)
	jira.cmd.print(msg)
end

function assign_to_me(pbi)
	local account_id = jira_context.config.account_id
	if account_id == "" then
		jira_print("error: account-id not set in config")
		return
	end

	-- -s for silent mode
	local cmd = string.format("jira assign -u %s -t %s -s", account_id, pbi.key)
	local ok = os.execute(cmd)

	if ok ~= 0 then
		jira_print("error: failed to assign " .. pbi.key)
		return
	end

	jira_print("assigned " .. pbi.key .. " to current user")
end

function change_pbi_status(pbi, status)
	-- -s for silent mode
	local cmd = string.format("jira transition '%s' -t %s -s", status, pbi.key)
	local ok = os.execute(cmd)

	if ok ~= 0 then
		jira_print("error: failed to transition " .. pbi.key .. " to '" .. status .. "'")
	end

	jira_print("transitioned " .. pbi.key .. " to '" .. status .. "'")
end

-- Enter to start work (runs plugins)
function start_work()
	local pbi = jira_context.selected_pbi
	if not pbi then
		jira_print("error: no PBI selected")
		return
	end

	assign_to_me(pbi)
	local status = jira_context.config.alias["ip"] or "In Progress"
	change_pbi_status(pbi, status)
end
jira.keymaps.set("<CR>", start_work, "Start", "Sprint")

-- PBI LIST
jira.keymaps.set("/", jira.cmd.open_filter, "Filter", "PbiList")
jira.keymaps.set("F", jira.cmd.refresh_all, "Refresh all", "PbiList")

-- PBI
jira.keymaps.set("r", jira.cmd.open_raw_pbi_json, "Raw Json", "Pbi")
jira.keymaps.set("f", jira.cmd.refresh, "Refresh line", "Pbi")
jira.keymaps.set("o", jira.cmd.open_in_browser, "Browser", "Pbi")
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

/// Reset the command channel with a new receiver.
///
/// This is intended for testing scenarios where multiple apps need to be created.
/// In production, `take_command_receiver` should only be called once.
pub fn reset_command_channel() {
    if let Some(receiver_mutex) = COMMAND_RECEIVER.get() {
        let (tx, rx) = mpsc::channel();
        // Replace the sender
        if let Some(sender_mutex) = COMMAND_SENDER.get() {
            if let Ok(mut sender) = sender_mutex.lock() {
                *sender = tx;
            }
        }
        // Restore the receiver
        if let Ok(mut guard) = receiver_mutex.lock() {
            *guard = Some(rx);
        }
    }
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
    add_lua_function(&lua, &cmd, JiraCommand::GoUp, "go_up")?;
    add_lua_function(&lua, &cmd, JiraCommand::GoDown, "go_down")?;
    add_lua_function(&lua, &cmd, JiraCommand::GoLeft, "go_left")?;
    add_lua_function(&lua, &cmd, JiraCommand::GoRight, "go_right")?;
    add_lua_function(&lua, &cmd, JiraCommand::OpenPbiDetails, "open_pbi_details")?;
    add_lua_function(&lua, &cmd, JiraCommand::Quit, "quit")?;
    add_lua_function(&lua, &cmd, JiraCommand::Refresh, "refresh")?;
    add_lua_function(&lua, &cmd, JiraCommand::RefreshAll, "refresh_all")?;
    add_lua_function(&lua, &cmd, JiraCommand::OpenInBrowser, "open_in_browser")?;
    add_lua_function(&lua, &cmd, JiraCommand::OpenPluginList, "open_plugin_list")?;
    add_lua_function(&lua, &cmd, JiraCommand::OpenRawPbiJson, "open_raw_pbi_json")?;
    add_lua_function(&lua, &cmd, JiraCommand::StartWork, "start_work")?;
    add_lua_function(&lua, &cmd, JiraCommand::OpenFilter, "open_filter")?;
    add_lua_function(&lua, &cmd, JiraCommand::EditPluginSelected, "edit_selected")?;
    add_lua_function(
        &lua,
        &cmd,
        JiraCommand::EditPluginSelected,
        "edit_selected_plugin",
    )?;
    add_lua_function(&lua, &cmd, JiraCommand::Back, "back")?;
    add_lua_function(&lua, &cmd, JiraCommand::AssignToMe, "assign_to_me")?;

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

fn add_lua_function(
    lua: &Lua,
    cmd: &mlua::Table,
    jira_command: JiraCommand,
    lua_function: &str,
) -> Result<()> {
    cmd.set(
        lua_function,
        lua.create_function(move |_, ()| {
            send_command(jira_command.clone());
            Ok(())
        })?,
    )?;
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

#[derive(Debug, Clone)]
pub struct JiraContext {
    pub config: JiraConfig,
    pub sprint: Option<Sprint>,
    pub selected_pbi: Option<Pbi>,
}

pub fn create_context(sprint: Option<Sprint>, selected_pbi: Option<Pbi>) -> JiraContext {
    JiraContext {
        config: JiraConfig::load().unwrap_or_default(),
        sprint,
        selected_pbi,
    }
}

/// Inject the full [`JiraContext`] as the `jira_context` global table.
///
/// Lua scripts receive:
/// - `jira_context.config`       — connection settings (namespace, token, …)
/// - `jira_context.sprint`       — active sprint (name, goal, end_date, pbis[])
/// - `jira_context.selected_pbi` — the currently selected PBI, or `nil`
pub fn inject_context(ctx: &JiraContext) -> Result<()> {
    let lua = get_lua_runtime().ok_or("Lua runtime not initialized")?;
    let root = lua.create_table()?;

    // config
    let config_tbl = lua.create_table()?;
    config_tbl.set("namespace", ctx.config.namespace.clone())?;
    config_tbl.set("email", ctx.config.email.clone())?;
    config_tbl.set("token", ctx.config.token.clone())?;
    config_tbl.set("auth_mode", ctx.config.auth_mode.clone())?;
    config_tbl.set("account_id", ctx.config.account_id.clone())?;
    config_tbl.set("board_id", ctx.config.board_id.clone())?;
    config_tbl.set("jira_version", ctx.config.jira_version.clone())?;
    config_tbl.set("alias", ctx.config.alias.clone())?;
    config_tbl.set("transitions", ctx.config.transitions.clone())?;
    root.set("config", config_tbl)?;

    // sprint

    if let Some(sprint) = &ctx.sprint {
        let sprint_tbl = lua.create_table()?;
        sprint_tbl.set("name", sprint.name.clone())?;
        sprint_tbl.set("goal", sprint.goal.clone())?;
        sprint_tbl.set("end_date", sprint.end_date.clone())?;
        sprint_tbl.set("board_id", sprint.board_id.clone())?;
        let pbis_tbl = lua.create_table()?;
        for (i, pbi) in sprint.pbis.iter().enumerate() {
            pbis_tbl.set(i + 1, pbi_to_lua(lua, pbi)?)?;
        }
        sprint_tbl.set("pbis", pbis_tbl)?;
        root.set("sprint", sprint_tbl)?;
    }

    if let Some(selected_pbi) = &ctx.selected_pbi {
        root.set("selected_pbi", pbi_to_lua(lua, selected_pbi)?)?;
    }

    lua.globals().set("jira_context", root)?;
    Ok(())
}

fn pbi_to_lua(lua: &Lua, pbi: &Pbi) -> Result<Table> {
    let table = lua.create_table()?;
    table.set("key", pbi.key.clone())?;
    table.set("summary", pbi.summary.clone())?;
    table.set("status", pbi.status.clone())?;
    table.set("assignee", pbi.assignee.clone())?;
    table.set("issue_type", pbi.issue_type.clone())?;
    table.set("description", pbi.description.clone())?;
    table.set("priority", pbi.priority.clone())?;
    table.set("story_points", pbi.story_points)?;
    table.set("labels", pbi.labels.clone())?;
    table.set("in_progress_at", pbi.in_progress_at.clone())?;
    table.set("resolved_at", pbi.resolved_at.clone())?;
    table.set("elapsed_minutes", pbi.elapsed_minutes())?;
    Ok(table)
}
