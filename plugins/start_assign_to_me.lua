-- start_assign_to_me.lua
-- Assigns the currently selected PBI to the authenticated user.
-- Triggered when starting work on a PBI from the sprint view.

local pbi = jira_context.selected_pbi
if not pbi then
    return "error: no PBI selected"
end

local account_id = jira_context.config.account_id
if account_id == "" then
    return "error: account_id not set in config"
end

local cmd = string.format("jira assign -u %s -t %s", account_id, pbi.key)
local ok = os.execute(cmd)

if ok ~= 0 then
    return "error: failed to assign " .. pbi.key
end

return "assigned " .. pbi.key .. " to current user"
