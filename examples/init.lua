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

	jira.cmd.assign_pbi(pbi.key, account_id)

	jira_print("assigned " .. pbi.key .. " to current user")
end

function change_pbi_status(pbi, status)
	jira.cmd.change_pbi_status(pbi.key, status)

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

-- Estimation analysis helpers
