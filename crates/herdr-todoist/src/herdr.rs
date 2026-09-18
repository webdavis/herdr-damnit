//! The `herdr` command surface this plugin drives: the pane calls it makes and the envelopes it
//! reads back. Every call goes through `HERDR_BIN_PATH`, the binary herdr names for its plugins.

use std::ffi::OsStr;
use std::process::Command;

use crate::config::{Config, Placement};
use crate::placement::Arrangement;

pub fn focus_plugin_pane(pane: &str) -> Result<String, String> {
    run(&["plugin", "pane", "focus", pane])
}

pub fn close_plugin_pane(pane: &str) -> Result<String, String> {
    run(&["plugin", "pane", "close", pane])
}

/// Open the plugin's own pane, and report the pane id herdr gave it.
pub fn open_plugin_pane(
    workspace: &str,
    neighbor: &str,
    focus: &str,
    config: &Config,
) -> Result<String, String> {
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
        focus,
    ];
    // A split or zoomed pane attaches to a pane; a tab or overlay one belongs to the workspace.
    match config.placement {
        Placement::Split | Placement::Zoomed if !neighbor.is_empty() => {
            args.extend_from_slice(&["--target-pane", neighbor]);
            if config.placement == Placement::Split {
                args.extend_from_slice(&["--direction", config.side.split_direction()]);
            }
        }
        _ => args.extend_from_slice(&["--workspace", workspace]),
    }
    let output = run(&args)?;
    pane_id_of_open(&output).ok_or_else(|| "herdr plugin pane open named no pane".to_string())
}

/// Move the pane an arrangement names, and report the id of the pane that moved.
pub fn move_pane(arrangement: &Arrangement, tab: &str) -> Result<String, String> {
    let output = run(&move_args(arrangement, tab))?;
    Ok(moved_pane_id(&output).unwrap_or_else(|| arrangement.source.clone()))
}

/// The argv of the one `herdr pane move` an arrangement takes. The ratio is rendered here because
/// the CLI takes strings, and it is passed only when a width was configured.
fn move_args(arrangement: &Arrangement, tab: &str) -> Vec<String> {
    let mut args: Vec<String> = [
        "pane",
        "move",
        &arrangement.source,
        "--tab",
        tab,
        "--split",
        arrangement.direction,
        "--target-pane",
        &arrangement.target,
        "--no-focus",
    ]
    .iter()
    .map(|argument| argument.to_string())
    .collect();
    if let Some(ratio) = arrangement.ratio {
        args.push("--ratio".to_string());
        args.push(ratio.to_string());
    }
    args
}

/// Every pane id live in a workspace, which is what proves a remembered pane is still there.
pub fn live_panes(workspace: &str) -> Result<Vec<String>, String> {
    let output = run(&["pane", "list", "--workspace", workspace])?;
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

fn pane_id_of_open(output: &str) -> Option<String> {
    read_pane_id(output, "/result/plugin_pane/pane/pane_id")
}

/// A move can rename the pane it moved, so the id to remember comes out of the move's own
/// envelope.
fn moved_pane_id(output: &str) -> Option<String> {
    read_pane_id(output, "/result/move_result/pane/pane_id")
}

fn read_pane_id(output: &str, pointer: &str) -> Option<String> {
    let json: serde_json::Value = serde_json::from_str(output).ok()?;
    json.pointer(pointer)?.as_str().map(str::to_string)
}

fn run<S: AsRef<OsStr>>(args: &[S]) -> Result<String, String> {
    let binary = std::env::var("HERDR_BIN_PATH").unwrap_or_else(|_| "herdr".to_string());
    let spelled = || {
        args.iter()
            .map(|argument| argument.as_ref().to_string_lossy().to_string())
            .collect::<Vec<_>>()
            .join(" ")
    };
    let output = Command::new(&binary)
        .args(args)
        .output()
        .map_err(|error| format!("{binary} {}: {error}", spelled()))?;
    if !output.status.success() {
        return Err(format!(
            "{binary} {} failed: {}",
            spelled(),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::placement::{self, Side};

    #[test]
    fn the_opened_pane_id_is_read_from_the_open_envelope() {
        let output = r#"{"result":{"plugin_pane":{"pane":{"pane_id":"w:p7","tab_id":"w:t1"}}}}"#;
        assert_eq!(pane_id_of_open(output), Some("w:p7".to_string()));
        assert_eq!(pane_id_of_open("{}"), None);
    }

    #[test]
    fn the_moved_pane_id_is_read_from_the_move_envelope() {
        let output =
            r#"{"result":{"move_result":{"pane":{"pane_id":"w:p8"},"previous_pane_id":"w:p7"}}}"#;
        assert_eq!(moved_pane_id(output), Some("w:p8".to_string()));
        assert_eq!(moved_pane_id("{}"), None);
    }

    #[test]
    fn a_width_becomes_one_move_with_the_tab_and_the_ratio_on_it() {
        let arrangement =
            placement::arrange(Side::Left, Some(0.25), "w:p2", "w:p1").expect("a move");

        assert_eq!(
            move_args(&arrangement, "w:t1").join(" "),
            "pane move w:p1 --tab w:t1 --split right --target-pane w:p2 --no-focus --ratio 0.25"
        );
    }

    #[test]
    fn a_move_with_no_width_passes_no_ratio() {
        let arrangement = placement::arrange(Side::Up, None, "w:p2", "w:p1").expect("a move");
        let args = move_args(&arrangement, "w:t1");

        assert!(!args.contains(&"--ratio".to_string()), "{args:?}");
    }
}
