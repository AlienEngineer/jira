use crate::config::JiraConfig;
use crate::jira::sprint::{Pbi, Sprint};
use mlua::Lua;

#[derive(Debug, Clone)]
struct JiraPlugin {
    lua_script: String,
}

#[derive(Debug, Clone)]
pub struct JiraContext {
    pub config: JiraConfig,
    pub sprint: Sprint,
    pub selected_pbi: Option<Pbi>,
}

/// Inject the full [`JiraContext`] as the `jira_context` global table.
///
/// Lua scripts receive:
/// - `jira_context.config`       — connection settings (namespace, token, …)
/// - `jira_context.sprint`       — active sprint (name, goal, end_date, pbis[])
/// - `jira_context.selected_pbi` — the currently selected PBI, or `nil`
fn inject_context(lua: &Lua, ctx: &JiraContext) -> crate::prelude::Result<()> {
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
    let sprint_tbl = lua.create_table()?;
    sprint_tbl.set("name", ctx.sprint.name.clone())?;
    sprint_tbl.set("goal", ctx.sprint.goal.clone())?;
    sprint_tbl.set("end_date", ctx.sprint.end_date.clone())?;
    sprint_tbl.set("board_id", ctx.sprint.board_id.clone())?;
    let pbis_tbl = lua.create_table()?;
    for (i, pbi) in ctx.sprint.pbis.iter().enumerate() {
        pbis_tbl.set(i + 1, pbi_to_lua(lua, pbi)?)?;
    }
    sprint_tbl.set("pbis", pbis_tbl)?;
    root.set("sprint", sprint_tbl)?;

    // selected_pbi
    match &ctx.selected_pbi {
        Some(pbi) => root.set("selected_pbi", pbi_to_lua(lua, pbi)?)?,
        None => root.set("selected_pbi", mlua::Value::Nil)?,
    }

    lua.globals().set("jira_context", root)?;
    Ok(())
}

fn pbi_to_lua(lua: &Lua, pbi: &Pbi) -> crate::prelude::Result<mlua::Table> {
    let tbl = lua.create_table()?;
    tbl.set("key", pbi.key.clone())?;
    tbl.set("summary", pbi.summary.clone())?;
    tbl.set("status", pbi.status.clone())?;
    tbl.set("assignee", pbi.assignee.clone())?;
    tbl.set("issue_type", pbi.issue_type.clone())?;
    tbl.set("description", pbi.description.clone())?;
    tbl.set("priority", pbi.priority.clone())?;
    tbl.set("story_points", pbi.story_points)?;
    tbl.set("labels", pbi.labels.clone())?;
    tbl.set("in_progress_at", pbi.in_progress_at.clone())?;
    tbl.set("resolved_at", pbi.resolved_at.clone())?;
    tbl.set(
        "elapsed_minutes",
        crate::jira::sprint::pbi_elapsed_minutes(pbi),
    )?;
    Ok(tbl)
}

fn execute_lua_script(script: &str, ctx: &JiraContext) -> crate::prelude::Result<String> {
    let lua = Lua::new();
    inject_context(&lua, ctx)?;
    let result: String = lua.load(script).eval()?;
    Ok(result)
}

fn load_plugins_from_path(path: &str) -> crate::prelude::Result<Vec<JiraPlugin>> {
    let mut plugins = Vec::new();
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let entry_path = entry.path();
        if entry_path.extension().and_then(|s| s.to_str()) == Some("lua") {
            let file_name = entry_path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("");
            if file_name.starts_with("start_") {
                let script = std::fs::read_to_string(&entry_path)?;
                plugins.push(JiraPlugin { lua_script: script });
            }
        }
    }
    Ok(plugins)
}

pub fn get_plugins_path() -> String {
    let plugins_folder = "plugins";
    match home::home_dir() {
        Some(path) => format!("{}/{}", path.display(), plugins_folder),
        None => plugins_folder.to_string(),
    }
}

const PLUGIN_TEMPLATE: &str = r#"-- Plugin name: {name}
-- Triggered when starting work on a PBI (prefix with "start_" to enable this).
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
--   jira_context.selected_pbi.in_progress_at  -- ISO-8601 timestamp of last "In Progress" transition (may be nil)
--   jira_context.selected_pbi.resolved_at     -- ISO-8601 resolution timestamp (may be nil)
--   jira_context.selected_pbi.elapsed_minutes -- minutes since last "In Progress" (nil for new/open)

local pbi = jira_context.selected_pbi
if not pbi then
    return "error: no PBI selected"
end

-- TODO: implement your plugin logic here.
return "ok"
"#;

/// Create a new plugin file from the template and open it in the default editor.
pub fn create_plugin(name: &str) -> crate::prelude::Result<()> {
    let name = if name.starts_with("start_") {
        name.to_string()
    } else {
        format!("start_{name}")
    };

    let plugins_path = get_plugins_path();
    std::fs::create_dir_all(&plugins_path)?;

    let file_name = format!("{name}.lua");
    let file_path = format!("{plugins_path}/{file_name}");

    if std::path::Path::new(&file_path).exists() {
        return Err(format!("Plugin already exists: {file_path}").into());
    }

    let content = PLUGIN_TEMPLATE.replace("{name}", &name);
    std::fs::write(&file_path, content)?;
    println!("Created plugin: {file_path}");

    let editor = std::env::var("VISUAL")
        .or_else(|_| std::env::var("EDITOR"))
        .unwrap_or_else(|_| "vi".to_string());

    std::process::Command::new(&editor)
        .arg(&file_path)
        .status()
        .map_err(|e| format!("Failed to open editor '{editor}': {e}"))?;

    Ok(())
}

/// Execute all Lua plugins found in `~/plugins/`, injecting the full
/// application context as the `jira_context` global table.
///
/// # Example (Lua)
/// ```lua
/// local pbi = jira_context.selected_pbi
/// return "Selected: " .. (pbi and pbi.key or "none")
/// ```
pub fn execute_plugins<F>(ctx: &JiraContext, mut callback: F) -> crate::prelude::Result<()>
where
    F: FnMut(Result<String, String>),
{
    let plugins_path = get_plugins_path();
    let plugins = load_plugins_from_path(&plugins_path)?;
    for plugin in plugins {
        match execute_lua_script(&plugin.lua_script, ctx) {
            Ok(result) => callback(Ok(result)),
            Err(e) => callback(Err(e.to_string())),
        }
    }
    Ok(())
}
