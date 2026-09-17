//! The `open`, `toggle` and `focus` actions. Each one resolves the plugin's pane in the current
//! workspace, then drives the `herdr` CLI.

use std::path::PathBuf;
use std::process::Command;

use crate::config::Config;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Open,
    Toggle,
    Focus,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Decision {
    Open,
    Close(String),
    Focus(String),
    NothingToFocus,
}

/// What an action does, given the pane this plugin last opened in the workspace and the panes the
/// workspace has now. A remembered pane that is no longer live counts as no pane.
pub fn decide(mode: Mode, remembered: Option<&str>, live: &[String]) -> Decision {
    let open_pane = remembered.filter(|pane| live.iter().any(|live| live == pane));
    match (mode, open_pane) {
        (Mode::Toggle, Some(pane)) => Decision::Close(pane.to_string()),
        (Mode::Open | Mode::Focus, Some(pane)) => Decision::Focus(pane.to_string()),
        (Mode::Open | Mode::Toggle, None) => Decision::Open,
        (Mode::Focus, None) => Decision::NothingToFocus,
    }
}

/// Run one action and report what it did.
pub fn run(mode: Mode, config: &Config) -> Result<String, String> {
    let workspace = std::env::var("HERDR_WORKSPACE_ID")
        .map_err(|_| "no workspace context: run this action from inside herdr".to_string())?;
    let live = live_panes(&workspace)?;
    let remembered = remembered_pane(&workspace);
    match decide(mode, remembered.as_deref(), &live) {
        Decision::Focus(pane) => {
            herdr(&["plugin", "pane", "focus", &pane])?;
            Ok(format!("focused {pane}"))
        }
        Decision::Close(pane) => {
            herdr(&["plugin", "pane", "close", &pane])?;
            forget_pane(&workspace);
            Ok(format!("closed {pane}"))
        }
        Decision::NothingToFocus => Ok(format!("no todoist pane in {workspace}")),
        Decision::Open => {
            let pane = open_pane(&workspace, config)?;
            remember_pane(&workspace, &pane);
            Ok(format!("opened {pane}"))
        }
    }
}

fn open_pane(workspace: &str, config: &Config) -> Result<String, String> {
    let plugin = std::env::var("HERDR_PLUGIN_ID").unwrap_or_else(|_| "herdr-todoist".to_string());
    let mut args = vec![
        "plugin",
        "pane",
        "open",
        "--plugin",
        &plugin,
        "--entrypoint",
        "pane",
        "--placement",
        &config.placement,
        "--focus",
    ];
    // A split or zoomed pane attaches to a pane; a tab or overlay one belongs to the workspace.
    let target = std::env::var("HERDR_PANE_ID").unwrap_or_default();
    match config.placement.as_str() {
        "split" | "zoomed" if !target.is_empty() => {
            args.extend_from_slice(&["--target-pane", &target]);
            if config.placement == "split" {
                args.extend_from_slice(&["--direction", &config.direction]);
            }
        }
        _ => args.extend_from_slice(&["--workspace", workspace]),
    }
    let output = herdr(&args)?;
    pane_id_of_open(&output).ok_or_else(|| "herdr plugin pane open named no pane".to_string())
}

fn pane_id_of_open(output: &str) -> Option<String> {
    let json: serde_json::Value = serde_json::from_str(output).ok()?;
    json.pointer("/result/plugin_pane/pane/pane_id")?
        .as_str()
        .map(str::to_string)
}

fn live_panes(workspace: &str) -> Result<Vec<String>, String> {
    let output = herdr(&["pane", "list", "--workspace", workspace])?;
    let json: serde_json::Value =
        serde_json::from_str(&output).map_err(|error| format!("herdr pane list: {error}"))?;
    json.pointer("/result/panes")
        .and_then(|panes| panes.as_array())
        .map(|panes| {
            panes
                .iter()
                .filter_map(|pane| pane.get("pane_id")?.as_str().map(str::to_string))
                .collect()
        })
        .ok_or_else(|| format!("herdr pane list named no panes in {workspace}"))
}

fn herdr(args: &[&str]) -> Result<String, String> {
    let binary = std::env::var("HERDR_BIN_PATH").unwrap_or_else(|_| "herdr".to_string());
    let output = Command::new(&binary)
        .args(args)
        .output()
        .map_err(|error| format!("{binary} {}: {error}", args.join(" ")))?;
    if !output.status.success() {
        return Err(format!(
            "{binary} {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// The pane this plugin last opened in a workspace, one file per workspace so two workspaces never
/// overwrite each other. herdr hands the plugin its own state directory; the documented path is the
/// fallback for a run outside herdr.
fn pane_state_path(workspace: &str) -> PathBuf {
    match std::env::var_os("HERDR_PLUGIN_STATE_DIR") {
        Some(dir) => PathBuf::from(dir).join("panes").join(workspace),
        None => state_home()
            .join("herdr/plugins/state/herdr-todoist/panes")
            .join(workspace),
    }
}

fn state_home() -> PathBuf {
    match std::env::var_os("XDG_STATE_HOME") {
        Some(dir) => PathBuf::from(dir),
        None => PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".local/state"),
    }
}

fn remembered_pane(workspace: &str) -> Option<String> {
    let pane = std::fs::read_to_string(pane_state_path(workspace)).ok()?;
    let pane = pane.trim().to_string();
    (!pane.is_empty()).then_some(pane)
}

fn remember_pane(workspace: &str, pane: &str) {
    let path = pane_state_path(workspace);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(path, pane);
}

fn forget_pane(workspace: &str) {
    let _ = std::fs::remove_file(pane_state_path(workspace));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn panes(ids: &[&str]) -> Vec<String> {
        ids.iter().map(|id| id.to_string()).collect()
    }

    #[test]
    fn open_opens_when_no_pane_is_live() {
        assert_eq!(decide(Mode::Open, None, &panes(&["w:p1"])), Decision::Open);
    }

    #[test]
    fn open_focuses_the_pane_it_already_has() {
        assert_eq!(
            decide(Mode::Open, Some("w:p2"), &panes(&["w:p1", "w:p2"])),
            Decision::Focus("w:p2".to_string())
        );
    }

    #[test]
    fn toggle_closes_a_live_pane_and_opens_when_there_is_none() {
        assert_eq!(
            decide(Mode::Toggle, Some("w:p2"), &panes(&["w:p2"])),
            Decision::Close("w:p2".to_string())
        );
        assert_eq!(
            decide(Mode::Toggle, None, &panes(&["w:p1"])),
            Decision::Open
        );
    }

    #[test]
    fn a_remembered_pane_that_exited_counts_as_no_pane() {
        assert_eq!(
            decide(Mode::Toggle, Some("w:pGone"), &panes(&["w:p1"])),
            Decision::Open
        );
        assert_eq!(
            decide(Mode::Focus, Some("w:pGone"), &panes(&["w:p1"])),
            Decision::NothingToFocus
        );
    }

    #[test]
    fn focus_never_opens_a_pane() {
        assert_eq!(
            decide(Mode::Focus, None, &panes(&["w:p1"])),
            Decision::NothingToFocus
        );
    }

    #[test]
    fn the_opened_pane_id_is_read_from_the_open_envelope() {
        let output = r#"{"result":{"plugin_pane":{"pane":{"pane_id":"w:p7","tab_id":"w:t1"}}}}"#;
        assert_eq!(pane_id_of_open(output), Some("w:p7".to_string()));
        assert_eq!(pane_id_of_open("{}"), None);
    }
}
