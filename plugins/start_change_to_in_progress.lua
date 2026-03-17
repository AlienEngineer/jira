-- start_change_to_in_progress.lua
-- Transitions the currently selected PBI to the "In Progress" status.
-- Resolves the status name through the alias config before calling the CLI,
-- so whatever alias maps to "In Progress" in the user's config is honoured.
-- Triggered when starting work on a PBI from the sprint view.

local pbi = jira_context.selected_pbi
if not pbi then
	return "error: no PBI selected"
end

local status = jira_context.config.alias["ip"] or "In Progress"

-- -s for silent mode
local cmd = string.format("jira transition '%s' -t %s -s", status, pbi.key)
local ok = os.execute(cmd)

if ok ~= 0 then
	return "error: failed to transition " .. pbi.key .. " to '" .. status .. "'"
end

return "transitioned " .. pbi.key .. " to '" .. status .. "'"
