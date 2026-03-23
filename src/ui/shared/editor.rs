use crate::config::JiraConfig;
use crate::jira::pbi::Pbi;
use std::env;
use std::fs;
use std::process::Command;

/// Open the raw JSON of a PBI in the user's preferred editor.
///
/// Uses $VISUAL, $EDITOR, or falls back to "vi".
/// Creates a temporary file, opens it, then cleans up.
pub fn open_raw_in_editor(pbi: &Pbi) {
    let json = pbi.raw.clone();
    let key = pbi.key.as_str();
    let tmp_path = env::temp_dir().join(format!("jira_raw_{key}.json"));

    if let Err(e) = fs::write(&tmp_path, &json) {
        eprintln!("Failed to write temp file: {e}");
        return;
    }

    let editor = env::var("VISUAL")
        .or_else(|_| env::var("EDITOR"))
        .unwrap_or_else(|_| "vi".to_string());

    let _ = Command::new(&editor)
        .arg(&tmp_path)
        .status()
        .map_err(|e| eprintln!("Failed to open editor '{editor}': {e}"));

    let _ = fs::remove_file(&tmp_path);
}

/// Open a PBI in the browser by key.
///
/// Returns Ok with a status message, or Err with an error message.
pub fn open_pbi_in_browser(key: &str) -> Result<String, String> {
    let config = JiraConfig::load().unwrap_or_default();
    let url = format!("{}/browse/{}", config.namespace, key);

    #[cfg(target_os = "macos")]
    let result = Command::new("open").arg(&url).status();
    #[cfg(target_os = "linux")]
    let result = Command::new("xdg-open").arg(&url).status();
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    let result: std::result::Result<std::process::ExitStatus, std::io::Error> = Err(
        std::io::Error::new(std::io::ErrorKind::Unsupported, "unsupported platform"),
    );

    match result {
        Ok(_) => Ok(format!("Opened {url}")),
        Err(e) => Err(format!("Failed to open browser: {e}")),
    }
}
