-- Sample keymaps configuration for jira-cli
-- Place this file in ~/.config/jira/init.lua

-- Navigation (no description = hidden from footer)
jira.keymaps.set("j", jira.cmd.go_down)
jira.keymaps.set("k", jira.cmd.go_up)
jira.keymaps.set("h", jira.cmd.go_left)
jira.keymaps.set("l", jira.cmd.go_right)
jira.keymaps.set("<ESC>", jira.cmd.back)

-- Actions (with description = shown in footer)
jira.keymaps.set("q", jira.cmd.quit, "Quit")
jira.keymaps.set("f", jira.cmd.refresh, "Refresh")
jira.keymaps.set("F", jira.cmd.refresh_all, "Refresh all")
jira.keymaps.set("o", jira.cmd.open_in_browser, "Browser")
jira.keymaps.set("p", jira.cmd.open_plugin_list, "Plugins")
jira.keymaps.set("r", jira.cmd.open_raw_pbi_json, "Raw JSON")
jira.keymaps.set("e", jira.cmd.edit_selected_plugin, "Edit")

-- Enter to start work (runs plugins)
jira.keymaps.set("<CR>", jira.cmd.start_work, "Start")

-- Filter (for pbi list view)
jira.keymaps.set("/", jira.cmd.open_filter, "Filter")
