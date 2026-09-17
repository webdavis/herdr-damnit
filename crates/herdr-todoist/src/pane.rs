//! The `open`, `toggle` and `focus` actions. Each one resolves the plugin's pane in the current
//! workspace, then drives the `herdr` CLI.

use std::path::PathBuf;
use std::process::Command;

use crate::config::{Config, Placement};
use crate::views::Views;

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

/// What to do when a remembered pane turns out not to be this plugin's: `herdr pane list` only
/// proves a pane id is live in the workspace, not that it belongs to us, so `plugin pane
/// focus`/`close` is herdr's own registry check and the one that can reject it.
enum Recovery {
    RetryOpen,
    ReportGone,
}

fn recover(mode: Mode) -> Recovery {
    match mode {
        Mode::Focus => Recovery::ReportGone,
        Mode::Open | Mode::Toggle => Recovery::RetryOpen,
    }
}

/// Run one action and report what it did.
pub fn run(mode: Mode, config: &Config) -> Result<String, String> {
    let workspace = std::env::var("HERDR_WORKSPACE_ID")
        .map_err(|_| "no workspace context: run this action from inside herdr".to_string())?;
    let live = live_panes(&workspace)?;
    let remembered = remembered_pane(&workspace);
    match decide(mode, remembered.as_deref(), &live) {
        Decision::Focus(pane) => match herdr(&["plugin", "pane", "focus", &pane]) {
            Ok(_) => Ok(format!("focused {pane}")),
            Err(_) => {
                forget_pane(&workspace);
                match recover(mode) {
                    Recovery::RetryOpen => open_and_remember(&workspace, config),
                    Recovery::ReportGone => {
                        Ok(format!("no todoist pane in {workspace}: {pane} is gone"))
                    }
                }
            }
        },
        Decision::Close(pane) => {
            let closed = herdr(&["plugin", "pane", "close", &pane]);
            forget_pane(&workspace);
            match closed {
                Ok(_) => Ok(format!("closed {pane}")),
                Err(_) => open_and_remember(&workspace, config),
            }
        }
        Decision::NothingToFocus => Ok(format!("no todoist pane in {workspace}")),
        Decision::Open => open_and_remember(&workspace, config),
    }
}

/// The `view <n>` action: note which view was asked for, then make sure the pane is open, which
/// is where that note is read.
pub fn view(argument: &str, config: &Config) -> Result<String, String> {
    let number: usize = argument
        .parse()
        .map_err(|_| format!("'{argument}' is not a view number"))?;
    let views = Views::new(&config.views);
    let name = views.name_of_number(number).ok_or_else(|| {
        format!(
            "no view {number}: this config has {} views, 1 being the unfiltered list",
            views.len()
        )
    })?;
    request_view(name);
    let outcome = run(Mode::Open, config)?;
    Ok(format!("{outcome}, showing {name}"))
}

fn open_and_remember(workspace: &str, config: &Config) -> Result<String, String> {
    let pane = open_pane(workspace, config)?;
    remember_pane(workspace, &pane);
    Ok(format!("opened {pane}"))
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
        config.placement.as_str(),
        "--focus",
    ];
    // A split or zoomed pane attaches to a pane; a tab or overlay one belongs to the workspace.
    let target = std::env::var("HERDR_PANE_ID").unwrap_or_default();
    match config.placement {
        Placement::Split | Placement::Zoomed if !target.is_empty() => {
            args.extend_from_slice(&["--target-pane", &target]);
            if config.placement == Placement::Split {
                args.extend_from_slice(&["--direction", config.direction.as_str()]);
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
/// overwrite each other.
fn pane_state_path(workspace: &str) -> PathBuf {
    state_dir().join("panes").join(workspace)
}

/// herdr hands the plugin its own state directory; the documented path is the fallback for a run
/// outside herdr.
fn state_dir() -> PathBuf {
    match std::env::var_os("HERDR_PLUGIN_STATE_DIR") {
        Some(dir) => PathBuf::from(dir),
        None => state_home().join("herdr/plugins/state/herdr-todoist"),
    }
}

/// The view a `view` action asked for. herdr runs an action as its own process, so the name
/// travels through a file: one file for the plugin, which a pane about to start reads on its
/// first draw and a pane already open reads on its next tick.
fn view_request_path() -> PathBuf {
    state_dir().join("requested-view")
}

pub fn request_view(name: &str) {
    write_request(&view_request_path(), name);
}

/// The requested view, which is consumed by the read so the pane does not pull itself back to it
/// after the operator has moved on.
pub fn take_requested_view() -> Option<String> {
    take_request(&view_request_path())
}

fn write_request(path: &std::path::Path, name: &str) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(path, name);
}

fn take_request(path: &std::path::Path) -> Option<String> {
    let name = std::fs::read_to_string(path).ok()?;
    let _ = std::fs::remove_file(path);
    let name = name.trim().to_string();
    (!name.is_empty()).then_some(name)
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
    fn a_failed_focus_or_close_retries_as_open_except_in_focus_mode() {
        assert!(matches!(recover(Mode::Open), Recovery::RetryOpen));
        assert!(matches!(recover(Mode::Toggle), Recovery::RetryOpen));
        assert!(matches!(recover(Mode::Focus), Recovery::ReportGone));
    }

    #[test]
    fn a_view_number_past_the_end_names_how_many_views_there_are() {
        let error = view("4", &Config::default()).expect_err("refuses");

        assert!(error.contains("no view 4"), "{error}");
        assert!(error.contains("1 views"), "{error}");
    }

    #[test]
    fn a_view_argument_that_is_not_a_number_is_refused() {
        let error = view("today", &Config::default()).expect_err("refuses");

        assert!(error.contains("not a view number"), "{error}");
    }

    #[test]
    fn a_requested_view_is_read_once_and_then_gone() {
        let path = std::env::temp_dir().join(format!(
            "herdr-todoist-requested-view-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);

        assert_eq!(take_request(&path), None);
        write_request(&path, "today");
        assert_eq!(take_request(&path), Some("today".to_string()));
        assert_eq!(take_request(&path), None, "the request outlived its read");
    }

    #[test]
    fn the_opened_pane_id_is_read_from_the_open_envelope() {
        let output = r#"{"result":{"plugin_pane":{"pane":{"pane_id":"w:p7","tab_id":"w:t1"}}}}"#;
        assert_eq!(pane_id_of_open(output), Some("w:p7".to_string()));
        assert_eq!(pane_id_of_open("{}"), None);
    }
}
