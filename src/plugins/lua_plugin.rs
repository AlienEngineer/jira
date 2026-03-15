use crate::config::JiraConfig;
use crate::jira::pbi::Pbi;
use crate::jira::sprint::Sprint;
use crate::prelude::Result;
use include_dir::{include_dir, Dir, DirEntry};
use mlua::{Lua, Table};
use std::fs;
use std::path::{Path, PathBuf};

static BUNDLED_PLUGINS: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/plugins");

#[derive(Debug, Clone)]
struct JiraPlugin {
    lua_script: String,
}

#[derive(Debug, Clone)]
pub struct JiraContext {
    pub config: JiraConfig,
    pub sprint: Sprint,
    pub selected_pbi: Pbi,
}

/// Inject the full [`JiraContext`] as the `jira_context` global table.
///
/// Lua scripts receive:
/// - `jira_context.config`       — connection settings (namespace, token, …)
/// - `jira_context.sprint`       — active sprint (name, goal, end_date, pbis[])
/// - `jira_context.selected_pbi` — the currently selected PBI, or `nil`
fn inject_context(lua: &Lua, ctx: &JiraContext) -> Result<()> {
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
    root.set("selected_pbi", pbi_to_lua(lua, &ctx.selected_pbi)?)?;

    lua.globals().set("jira_context", root)?;
    Ok(())
}

fn pbi_to_lua(lua: &Lua, pbi: &Pbi) -> Result<Table> {
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
    tbl.set("elapsed_minutes", pbi.elapsed_minutes().unwrap())?;
    Ok(tbl)
}

fn execute_lua_script(script: &str, ctx: &JiraContext) -> Result<String> {
    let lua = Lua::new();
    inject_context(&lua, ctx)?;
    let result: String = lua.load(script).eval()?;
    Ok(result)
}

fn load_plugins_from_path(path: &str) -> Result<Vec<JiraPlugin>> {
    let mut plugins = Vec::new();
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let entry_path = entry.path();
        if entry_path.extension().and_then(|s| s.to_str()) == Some("lua") {
            let file_name = entry_path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("");
            if file_name.starts_with("start_") {
                let script = fs::read_to_string(&entry_path)?;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PluginInstallSummary {
    pub copied: usize,
    pub skipped: usize,
}

fn bundled_plugin_files() -> Vec<(&'static str, &'static [u8])> {
    fn collect(dir: &'static Dir<'static>, files: &mut Vec<(&'static str, &'static [u8])>) {
        for entry in dir.entries() {
            match entry {
                DirEntry::Dir(child) => collect(child, files),
                DirEntry::File(file) => {
                    if let Some(name) = file.path().file_name().and_then(|name| name.to_str()) {
                        if name.ends_with(".lua") {
                            files.push((name, file.contents()));
                        }
                    }
                }
            }
        }
    }

    let mut files = Vec::new();
    collect(&BUNDLED_PLUGINS, &mut files);
    files.sort_by(|(left, _), (right, _)| left.cmp(right));
    files
}

fn install_plugin_files<I, N, C>(
    plugin_files: I,
    destination_dir: &Path,
) -> Result<PluginInstallSummary>
where
    I: IntoIterator<Item = (N, C)>,
    N: AsRef<str>,
    C: AsRef<[u8]>,
{
    fs::create_dir_all(destination_dir)?;

    let mut summary = PluginInstallSummary {
        copied: 0,
        skipped: 0,
    };

    for (file_name, contents) in plugin_files {
        let destination = destination_dir.join(file_name.as_ref());
        if destination.exists() {
            summary.skipped += 1;
            continue;
        }

        fs::write(destination, contents.as_ref())?;
        summary.copied += 1;
    }

    Ok(summary)
}

pub fn install_bundled_plugins() -> Result<PluginInstallSummary> {
    let destination_dir = PathBuf::from(get_plugins_path());
    install_plugin_files(bundled_plugin_files(), &destination_dir)
}

/// Execute all Lua plugins found in `~/plugins/`, injecting the full
/// application context as the `jira_context` global table.
///
/// # Example (Lua)
/// ```lua
/// local pbi = jira_context.selected_pbi
/// return "Selected: " .. (pbi and pbi.key or "none")
/// ```
pub fn execute_plugins<F>(ctx: &JiraContext, mut callback: F) -> Result<()>
where
    F: FnMut(std::result::Result<String, String>),
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

#[cfg(test)]
mod tests {
    use super::{install_plugin_files, PluginInstallSummary};
    use std::env;
    use std::fs;
    use std::path::PathBuf;
    use std::process;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn make_temp_dir(test_name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos();
        let dir = env::temp_dir().join(format!(
            "jira-plugin-tests-{test_name}-{}-{unique}",
            process::id()
        ));
        fs::create_dir_all(&dir).expect("temp dir should be created");
        dir
    }

    #[test]
    fn installs_missing_plugin_files() {
        let dir = make_temp_dir("install-missing");

        let result = install_plugin_files(
            vec![
                ("start_alpha.lua", &b"alpha"[..]),
                ("start_beta.lua", &b"beta"[..]),
            ],
            &dir,
        )
        .expect("plugin install should succeed");

        assert_eq!(
            result,
            PluginInstallSummary {
                copied: 2,
                skipped: 0,
            }
        );
        assert_eq!(
            fs::read_to_string(dir.join("start_alpha.lua")).expect("alpha plugin should exist"),
            "alpha"
        );
        assert_eq!(
            fs::read_to_string(dir.join("start_beta.lua")).expect("beta plugin should exist"),
            "beta"
        );

        fs::remove_dir_all(dir).expect("temp dir should be removed");
    }

    #[test]
    fn skips_existing_plugin_files() {
        let dir = make_temp_dir("skip-existing");
        fs::write(dir.join("start_alpha.lua"), "original")
            .expect("existing plugin should be written");

        let result = install_plugin_files(
            vec![
                ("start_alpha.lua", &b"updated"[..]),
                ("start_beta.lua", &b"beta"[..]),
            ],
            &dir,
        )
        .expect("plugin install should succeed");

        assert_eq!(
            result,
            PluginInstallSummary {
                copied: 1,
                skipped: 1,
            }
        );
        assert_eq!(
            fs::read_to_string(dir.join("start_alpha.lua")).expect("existing plugin should remain"),
            "original"
        );
        assert_eq!(
            fs::read_to_string(dir.join("start_beta.lua")).expect("new plugin should exist"),
            "beta"
        );

        fs::remove_dir_all(dir).expect("temp dir should be removed");
    }
}
