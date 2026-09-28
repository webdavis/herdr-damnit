use std::ffi::OsStr;
use std::process::Command;

use crate::config::{Config, Placement};

pub struct CliHerdr;

impl herdr_damnit_application::Herdr for CliHerdr {
    fn call(&self, args: &[&str]) -> Result<String, String> {
        run(args)
    }
}

pub fn focus_plugin_pane(pane: &str) -> Result<String, String> {
    run(&["plugin", "pane", "focus", pane])
}

pub fn close_plugin_pane(pane: &str) -> Result<String, String> {
    run(&["plugin", "pane", "close", pane])
}

pub fn open_plugin_pane(
    workspace: &str,
    neighbor: &str,
    focus: &str,
    config: &Config,
) -> Result<String, String> {
    let plugin = std::env::var("HERDR_PLUGIN_ID").unwrap_or_else(|_| "herdr-damnit".to_string());
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
    if config
        .placement
        .attaches_to_a_pane_rather_than_the_workspace()
        && !neighbor.is_empty()
    {
        args.extend_from_slice(&["--target-pane", neighbor]);
        if config.placement == Placement::Split {
            args.extend_from_slice(&["--direction", config.side.split_direction()]);
        }
    } else {
        args.extend_from_slice(&["--workspace", workspace]);
    }
    let output = run(&args)?;
    pane_id_of_open(&output).ok_or_else(|| "herdr plugin pane open named no pane".to_string())
}

pub fn resize_leading_pane(pane: &str, direction: &str, target_ratio: f32) -> Result<bool, String> {
    let amount = target_ratio - measured_ratio_rather_than_an_assumed_half(pane)?;
    let output = run(&resize_args(pane, direction, amount))?;
    Ok(resize_changed(&output))
}

fn resize_args(pane: &str, direction: &str, amount: f32) -> Vec<String> {
    [
        "pane",
        "resize",
        "--pane",
        pane,
        "--direction",
        direction,
        "--amount",
    ]
    .iter()
    .map(|argument| argument.to_string())
    .chain(std::iter::once(amount.to_string()))
    .collect()
}

fn measured_ratio_rather_than_an_assumed_half(pane: &str) -> Result<f32, String> {
    let output = run(&["pane", "layout", "--pane", pane])?;
    ratio_of_layout(&output).ok_or_else(|| format!("herdr pane layout named no ratio for {pane}"))
}

fn ratio_of_layout(output: &str) -> Option<f32> {
    let json: serde_json::Value = serde_json::from_str(output).ok()?;
    json.pointer("/result/layout/splits/0/ratio")
        .and_then(serde_json::Value::as_f64)
        .map(|ratio| ratio as f32)
}

fn resize_changed(output: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(output)
        .ok()
        .and_then(|json| json.pointer("/result/resize/changed")?.as_bool())
        .unwrap_or(false)
}

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

    #[test]
    fn the_opened_pane_id_is_read_from_the_open_envelope() {
        let output = r#"{"result":{"plugin_pane":{"pane":{"pane_id":"w:p7","tab_id":"w:t1"}}}}"#;
        assert_eq!(pane_id_of_open(output), Some("w:p7".to_string()));
        assert_eq!(pane_id_of_open("{}"), None);
    }

    #[test]
    fn a_resize_carries_the_pane_the_direction_and_the_rendered_amount() {
        assert_eq!(
            resize_args("w:p1", "right", 0.2).join(" "),
            "pane resize --pane w:p1 --direction right --amount 0.2"
        );
    }

    #[test]
    fn the_current_ratio_is_read_from_the_layouts_leading_split() {
        let output = r#"{"result":{"layout":{"splits":[{"ratio":0.5}]}}}"#;
        assert_eq!(ratio_of_layout(output), Some(0.5));
        assert_eq!(ratio_of_layout("{}"), None);
    }

    #[test]
    fn the_resize_outcome_is_read_from_its_own_changed_flag() {
        let changed = r#"{"result":{"resize":{"changed":true}}}"#;
        let unchanged = r#"{"result":{"resize":{"changed":false,"reason":"unchanged"}}}"#;
        assert!(resize_changed(changed));
        assert!(!resize_changed(unchanged));
        assert!(!resize_changed("{}"));
    }
}
