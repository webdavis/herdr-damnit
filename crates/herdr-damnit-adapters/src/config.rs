use std::path::{Path, PathBuf};

use herdr_damnit_domain::{IconSet, OPEN, View};
use serde::Deserialize;

mod kinds;

pub use kinds::{Icons, Placement, Side};

pub const DEFAULT_REFRESH_SECONDS: u64 = 300;

const DEFAULT_HANDOFF_LABEL: &str = "handed-off";

#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default = "default_dam")]
    pub dam: Vec<String>,
    #[serde(default)]
    pub placement: Placement,
    #[serde(default)]
    pub side: Side,
    pub width: Option<f32>,
    pub default_view: Option<String>,
    #[serde(default)]
    pub auto_open: bool,
    pub theme: Option<String>,
    #[serde(default)]
    pub icons: Icons,
    pub refresh_seconds: Option<u64>,
    #[serde(default = "default_handoff_label")]
    pub handoff_label: String,
    #[serde(default)]
    pub views: Vec<ConfigView>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ConfigView {
    pub name: String,
    pub query: String,
}

impl Config {
    pub fn load() -> Result<Self, String> {
        let path = config_path();
        match std::fs::read_to_string(&path) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Self::parse("")?),
            Err(error) => Err(format!("{}: {error}", path.display())),
            Ok(text) => Self::parse(&text).map_err(|error| format!("{}: {error}", path.display())),
        }
    }

    pub fn parse(text: &str) -> Result<Self, String> {
        let config: Self = toml::from_str(text).map_err(|error| error.to_string())?;
        config.check_dam()?;
        config.check_view_names()?;
        config.check_width()?;
        config.check_default_view()?;
        Ok(config)
    }

    pub fn views(&self) -> Vec<View> {
        self.views
            .iter()
            .map(|view| View {
                name: view.name.clone(),
                query: view.query.clone(),
            })
            .collect()
    }

    pub fn icons(&self) -> IconSet {
        self.icons.into()
    }

    pub fn refresh_seconds(&self) -> u64 {
        self.refresh_seconds.unwrap_or(DEFAULT_REFRESH_SECONDS)
    }

    pub fn check_theme_against(&self, names: &[&str]) -> Result<(), String> {
        match &self.theme {
            Some(name) if !names.contains(&name.as_str()) => Err(format!(
                "unknown theme '{name}': expected one of {}",
                names.join(", ")
            )),
            _ => Ok(()),
        }
    }

    fn check_dam(&self) -> Result<(), String> {
        match self.dam.first() {
            Some(word) if !word.trim().is_empty() => Ok(()),
            _ => Err("dam needs at least one word: the binary to spawn".to_string()),
        }
    }

    fn check_view_names(&self) -> Result<(), String> {
        let mut seen: Vec<&str> = Vec::new();
        for view in &self.views {
            if view.name.trim().is_empty() {
                return Err("a view needs a name".to_string());
            }
            if view.query.trim().is_empty() {
                return Err(format!("view '{}' has an empty query", view.name));
            }
            if view.name == OPEN {
                return Err(format!(
                    "a view cannot be named '{OPEN}': that is the pane's own open list"
                ));
            }
            if seen.contains(&view.name.as_str()) {
                return Err(format!("two views are named '{}'", view.name));
            }
            seen.push(&view.name);
        }
        Ok(())
    }

    fn check_width(&self) -> Result<(), String> {
        match self.width {
            Some(width) if !(width > 0.0 && width < 1.0) => Err(format!(
                "width {width} is not a share of the tab: use a fraction above 0 and below 1"
            )),
            _ => Ok(()),
        }
    }

    fn check_default_view(&self) -> Result<(), String> {
        let Some(name) = &self.default_view else {
            return Ok(());
        };
        if name == OPEN || self.views.iter().any(|view| &view.name == name) {
            return Ok(());
        }
        let mut names = vec![OPEN.to_string()];
        names.extend(self.views.iter().map(|view| view.name.clone()));
        Err(format!(
            "default_view '{name}' is not a view: the views are {}",
            names.join(", ")
        ))
    }
}

fn default_dam() -> Vec<String> {
    vec!["dam".to_string()]
}

fn default_handoff_label() -> String {
    DEFAULT_HANDOFF_LABEL.to_string()
}

fn config_path() -> PathBuf {
    let given = std::env::var_os("HERDR_PLUGIN_CONFIG_DIR").map(PathBuf::from);
    config_path_in(given.as_deref(), &base_config_dir())
}

fn config_path_in(herdr_plugin_config_dir: Option<&Path>, base: &Path) -> PathBuf {
    let dir = match herdr_plugin_config_dir {
        Some(dir) => dir.to_path_buf(),
        None => base.join("herdr/plugins/config/herdr-damnit"),
    };
    dir.join("config.toml")
}

fn base_config_dir() -> PathBuf {
    match std::env::var_os("XDG_CONFIG_HOME") {
        Some(dir) => PathBuf::from(dir),
        None => PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".config"),
    }
}

#[cfg(test)]
mod tests;
