use std::ffi::OsStr;
use std::process::Command;

use herdr_damnit_application::{PaneFocus, PaneOpen, PanePlacement, PluginPanes, SplitSide};

pub struct CliHerdr {
    binary: String,
    plugin: String,
}

impl CliHerdr {
    pub fn new(binary: String, plugin: String) -> Self {
        Self { binary, plugin }
    }

    fn run<S: AsRef<OsStr>>(&self, args: &[S]) -> Result<String, String> {
        let spelled = || {
            args.iter()
                .map(|argument| argument.as_ref().to_string_lossy().to_string())
                .collect::<Vec<_>>()
                .join(" ")
        };
        let output = Command::new(&self.binary)
            .args(args)
            .output()
            .map_err(|error| format!("{} {}: {error}", self.binary, spelled()))?;
        if !output.status.success() {
            return Err(format!(
                "{} {} failed: {}",
                self.binary,
                spelled(),
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    fn measured_ratio_rather_than_an_assumed_half(&self, pane: &str) -> Result<f32, String> {
        let output = self.run(&["pane", "layout", "--pane", pane])?;
        ratio_of_layout(&output)
            .ok_or_else(|| format!("herdr pane layout named no ratio for {pane}"))
    }
}

impl herdr_damnit_application::Herdr for CliHerdr {
    fn call(&self, args: &[&str]) -> Result<String, String> {
        self.run(args)
    }
}

impl PluginPanes for CliHerdr {
    fn live(&self, workspace: &str) -> Result<Vec<String>, String> {
        let output = self.run(&["pane", "list", "--workspace", workspace])?;
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

    fn open(&self, open: &PaneOpen) -> Result<String, String> {
        let output = self.run(&open_args(&self.plugin, open))?;
        pane_id_of_open(&output).ok_or_else(|| "herdr plugin pane open named no pane".to_string())
    }

    fn focus(&self, pane: &str) -> Result<(), String> {
        self.run(&["plugin", "pane", "focus", pane]).map(|_| ())
    }

    fn close(&self, pane: &str) -> Result<(), String> {
        self.run(&["plugin", "pane", "close", pane]).map(|_| ())
    }

    fn resize(&self, pane: &str, side: SplitSide, share: f32) -> Result<bool, String> {
        let amount = share - self.measured_ratio_rather_than_an_assumed_half(pane)?;
        let output = self.run(&resize_args(pane, direction(side), amount))?;
        Ok(resize_changed(&output))
    }
}

fn open_args<'a>(plugin: &'a str, open: &'a PaneOpen) -> Vec<&'a str> {
    let mut args = vec![
        "plugin",
        "pane",
        "open",
        "--plugin",
        plugin,
        "--entrypoint",
        "pane",
        "--placement",
        placement(open.placement),
        focus_flag(open.focus),
    ];
    match (open.placement, open.neighbor.as_str()) {
        (PanePlacement::Split(side), neighbor) if !neighbor.is_empty() => {
            args.extend(["--target-pane", neighbor, "--direction", direction(side)]);
        }
        (PanePlacement::Zoomed, neighbor) if !neighbor.is_empty() => {
            args.extend(["--target-pane", neighbor]);
        }
        _ => args.extend(["--workspace", &open.workspace]),
    }
    args
}

fn placement(placement: PanePlacement) -> &'static str {
    match placement {
        PanePlacement::Overlay => "overlay",
        PanePlacement::Split(_) => "split",
        PanePlacement::Tab => "tab",
        PanePlacement::Zoomed => "zoomed",
    }
}

fn direction(side: SplitSide) -> &'static str {
    match side {
        SplitSide::Right => "right",
        SplitSide::Down => "down",
    }
}

fn focus_flag(focus: PaneFocus) -> &'static str {
    match focus {
        PaneFocus::Take => "--focus",
        PaneFocus::Leave => "--no-focus",
    }
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

fn pane_id_of_open(output: &str) -> Option<String> {
    let json: serde_json::Value = serde_json::from_str(output).ok()?;
    json.pointer("/result/plugin_pane/pane/pane_id")?
        .as_str()
        .map(str::to_string)
}

#[cfg(test)]
mod tests;
