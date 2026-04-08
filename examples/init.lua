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

function filter(array, filterIterator)
	local result = {}
	for key, value in pairs(array) do
		if filterIterator(value, key, array) then
			table.insert(result, value)
		end
	end
	return result
end

function Get_story_points(pbi)
	return jira.json.get(pbi.raw, "customfield_10006") or "N/A"
end

function Get_pbis_by_story_points(pbis, pbi)
	local points = Get_story_points(pbi)
	return filter(pbis, function(pbi_iter, key, index)
		return pbi.key ~= pbi_iter.key and Get_story_points(pbi_iter) == points
	end)
end

function Calculate_average_days(pbis)
	local total_points = 0
	local count = 0
	for _, pbi in ipairs(pbis) do
		local days = pbi.elapsed_minutes / 24.0 / 60
		if type(days) == "number" then
			total_points = total_points + days
			count = count + 1
		end
	end
	return count > 0 and math.ceil(total_points / count) or "N/A"
end

function Find_story_points(pbis)
	local points_count = {}
	for _, pbi in ipairs(pbis) do
		local points = Get_story_points(pbi)
		if points ~= "N/A" then
			points_count[points] = (points_count[points] or 0) + 1
		end
	end
	return points_count
end

function Get_days_per_story_point(pbis)
	local results = {}
	local story_points = Find_story_points(pbis)

	for points, count in pairs(story_points) do
		results[points] = {}
		local filtered_pbis = filter(pbis, function(pbi)
			return Get_story_points(pbi) == points
		end)

		for _, pbi in ipairs(filtered_pbis) do
			local days = math.ceil(pbi.elapsed_minutes / 24.0 / 60)
			if type(days) == "number" and days < 14 then
				table.insert(results[points], days)
			end
		end
	end

	return results
end

function Calculate_mean(values)
	if #values == 0 then
		return "N/A"
	end
	local sum = 0
	for _, v in ipairs(values) do
		sum = sum + v
	end
	return sum / #values
end

function Calculate_average_days_per_story_point(pbis)
	local days_per_point = Get_days_per_story_point(pbis)
	local averages = {}

	for sp, days in pairs(days_per_point) do
		averages[sp] = Calculate_mean(days)
	end

	return averages
end

function Calculate_stdev(values)
	local mean = Calculate_mean(values)
	if mean == "N/A" then
		return "N/A"
	end

	local sum_of_squares = 0
	local count = #values

	for _, value in ipairs(values) do
		sum_of_squares = sum_of_squares + (value - mean) ^ 2
	end

	return count > 1 and math.sqrt(sum_of_squares / (count - 1)) or 0
end

function Calculate_stdev_per_story_point(pbis)
	local days_per_point = Get_days_per_story_point(pbis)
	local stdevs = {}

	for sp, days in pairs(days_per_point) do
		stdevs[sp] = Calculate_stdev(days)
	end

	return stdevs
end

function Find_closest_story_point(pbi_days, avg_per_sp)
	local closest_sp = nil
	local min_distance = math.huge

	for sp, avg in pairs(avg_per_sp) do
		if avg ~= "N/A" then
			local distance = math.abs(pbi_days - avg)
			if distance < min_distance then
				min_distance = distance
				closest_sp = sp
			end
		end
	end

	return closest_sp
end

-- Columns

jira.columns.add(" SPs ", function(view)
	return Get_story_points(view.pbi)
end)

jira.columns.add("Estimation Accuracy", function(view)
	local sp = Get_story_points(view.pbi)
	if sp == "N/A" then
		return "N/A"
	end

	local days = math.ceil(view.pbi.elapsed_minutes / 24.0 / 60)
	if days >= 14 then
		return "Failure"
	end

	local avg_per_sp = Calculate_average_days_per_story_point(view.pbis)
	local stdevs = Calculate_stdev_per_story_point(view.pbis)
	local mean = avg_per_sp[sp]
	local stdev = stdevs[sp]

	if mean == "N/A" then
		return "N/A"
	end

	local closest_sp = Find_closest_story_point(days, avg_per_sp)

	-- Check if actual duration fits a different SP category better
	if closest_sp and closest_sp ~= sp then
		if closest_sp < sp then
			return "Over (fits ~" .. closest_sp .. " SP)"
		else
			return "Under (fits ~" .. closest_sp .. " SP)"
		end
	end

	-- Z-score: how many std devs from the mean of the PBI's own SP category
	if stdev ~= "N/A" and stdev > 0 then
		local z = math.abs(days - mean) / stdev
		if z <= 0.5 then
			return "Perfect"
		elseif z <= 1.0 then
			return "Good"
		elseif z <= 2.0 then
			return "Fair"
		else
			return "Poor"
		end
	end

	-- stdev is 0 (all PBIs with this SP took exactly the same time)
	local diff = math.abs(days - mean)
	if diff == 0 then
		return "Perfect"
	elseif diff <= 1 then
		return "Good"
	elseif diff <= 2 then
		return "Fair"
	else
		return "Poor"
	end
end)
