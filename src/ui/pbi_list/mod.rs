use crate::jira::pbi::Pbi;

// ── Column definitions ────────────────────────────────────────────────────────

struct ColumnDef {
    title: &'static str,
    width: usize,
}

fn column_def(name: &str) -> Option<ColumnDef> {
    match name {
        "key" => Some(ColumnDef {
            title: "Key",
            width: 10,
        }),
        "resolution" => Some(ColumnDef {
            title: "Resolution",
            width: 10,
        }),
        "priority" => Some(ColumnDef {
            title: "Priority",
            width: 10,
        }),
        "assignee" => Some(ColumnDef {
            title: "Assignee",
            width: 20,
        }),
        "status" => Some(ColumnDef {
            title: "Status",
            width: 15,
        }),
        "components" => Some(ColumnDef {
            title: "Components",
            width: 30,
        }),
        "creator" => Some(ColumnDef {
            title: "Creator",
            width: 15,
        }),
        "reporter" => Some(ColumnDef {
            title: "Reporter",
            width: 15,
        }),
        "issuetype" => Some(ColumnDef {
            title: "Issue Type",
            width: 10,
        }),
        "project" => Some(ColumnDef {
            title: "Project",
            width: 15,
        }),
        "summary" => Some(ColumnDef {
            title: "Summary",
            width: 100,
        }),
        _ => None,
    }
}

fn field_value(pbi: &Pbi, column: &str) -> String {
    match column {
        "key" => pbi.key.clone(),
        "summary" => pbi.summary.clone(),
        "status" => pbi.status.clone(),
        "assignee" => pbi.assignee.clone(),
        "resolution" => pbi.resolution.clone().unwrap_or_else(|| "-".to_string()),
        "priority" => pbi.priority.clone().unwrap_or_else(|| "-".to_string()),
        "components" => pbi.components.join(", "),
        "creator" => pbi.creator.clone(),
        "reporter" => pbi.reporter.clone(),
        "issuetype" => pbi.issue_type.clone(),
        "project" => pbi.project.clone(),
        _ => "-".to_string(),
    }
}

// ── Public entry point ────────────────────────────────────────────────────────

/// Present a list of issues according to `columns` and output format.
///
/// When `show_json` is true the output is pretty-printed JSON; otherwise a
/// fixed-width table is printed to stdout.
pub fn display_issues(issues: &[Pbi], columns: &[&str], show_json: bool) {
    if issues.is_empty() {
        println!("No issues found for the filter.");
        return;
    }

    if show_json {
        display_json(issues, columns);
    } else {
        display_table(issues, columns);
    }
}

// ── JSON output ───────────────────────────────────────────────────────────────

fn display_json(issues: &[Pbi], columns: &[&str]) {
    let mut response = json::JsonValue::new_array();
    for pbi in issues {
        let mut data = json::JsonValue::new_object();
        for &col in columns {
            if col == "components" {
                let mut arr = json::JsonValue::new_array();
                for c in &pbi.components {
                    let _ = arr.push(c.as_str());
                }
                data[col] = arr;
            } else {
                data[col] = field_value(pbi, col).into();
            }
        }
        let _ = response.push(data);
    }
    println!("{}", response.pretty(4));
}

// ── Table output ──────────────────────────────────────────────────────────────

fn display_table(issues: &[Pbi], columns: &[&str]) {
    // Validate columns and collect definitions up front so we can report
    // unknown names before producing any output.
    let defs: Vec<(&str, ColumnDef)> = columns
        .iter()
        .map(|&col| {
            let def = column_def(col).unwrap_or_else(|| {
                eprintln!("Unknown display option '{col}' passed.");
                std::process::exit(1);
            });
            (col, def)
        })
        .collect();

    // Header row
    let mut total_width = 0;
    for (_, def) in &defs {
        print!("{title:width$}|", title = def.title, width = def.width);
        total_width += def.width + 1;
    }
    println!();
    println!("{:->width$}", "", width = total_width);

    // Data rows
    for pbi in issues {
        for (col, def) in &defs {
            let mut value = field_value(pbi, col);
            value.truncate(def.width);
            print!("{value:width$}|", width = def.width);
        }
        println!();
    }
}
