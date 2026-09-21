//! The plugin's configuration, read from `config.toml` in the herdr plugin config directory. A
//! missing file is the default configuration. No key here names a credential: the token is `dam`'s.

use std::path::{Path, PathBuf};

use herdr_damnit_domain::{IconSet, OPEN, View};
use serde::Deserialize;

/// The interval read, which is two local reads rather than three network requests.
pub const DEFAULT_REFRESH_SECONDS: u64 = 300;

/// The label a successful hand-off writes, an empty one turning the record off.
const DEFAULT_HANDOFF_LABEL: &str = "handed-off";

#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Argv of the `dam` the pane spawns, so a test points it at its own fake and no test mangles
    /// `PATH`.
    #[serde(default = "default_dam")]
    pub dam: Vec<String>,
    /// How the `open` and `toggle` actions place the pane.
    #[serde(default)]
    pub placement: Placement,
    /// Which side of the calling pane a `split` placement takes.
    #[serde(default)]
    pub side: Side,
    /// The share of the tab the pane takes, left to herdr's own even split when unset.
    pub width: Option<f32>,
    /// The view the pane opens on, the open list when unset.
    pub default_view: Option<String>,
    /// Whether focusing a workspace opens the pane there on its own.
    #[serde(default)]
    pub auto_open: bool,
    /// The theme the pane paints with, by the name herdr knows it by, so the panes of one
    /// workspace match. Checked by the crate that owns the palettes.
    pub theme: Option<String>,
    /// Which set of marks a row carries: Nerd Font glyphs, or plain characters for a terminal
    /// whose font has none.
    #[serde(default)]
    pub icons: Icons,
    /// How often the pane reads `dam` on its own, in seconds. Zero turns the interval off,
    /// leaving `R` and the read that follows every write.
    pub refresh_seconds: Option<u64>,
    /// The label a successful hand-off writes on the object.
    #[serde(default = "default_handoff_label")]
    pub handoff_label: String,
    /// Named views, in the order the pane numbers them after its own open list.
    #[serde(default)]
    pub views: Vec<ConfigView>,
}

/// One view: a name to pick it by and a query in `dam`'s own grammar, which `dam` resolves
/// against its saved filters first and parses as a query otherwise.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ConfigView {
    pub name: String,
    pub query: String,
}

/// The pane placements `herdr plugin pane open --placement` accepts. An unrecognized value is a
/// config parse error naming these, rather than a raw error from `herdr` at action time.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Placement {
    Overlay,
    #[default]
    Split,
    Tab,
    Zoomed,
}

impl Placement {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Overlay => "overlay",
            Self::Split => "split",
            Self::Tab => "tab",
            Self::Zoomed => "zoomed",
        }
    }
}

/// The side of the calling pane this pane takes. herdr's own open splits rightward or downward
/// only, so those are the only sides there are.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    #[default]
    Right,
    Down,
}

impl Side {
    /// The direction both `herdr plugin pane open --direction` and `herdr pane resize
    /// --direction` take for this side.
    pub fn split_direction(self) -> &'static str {
        match self {
            Self::Right => "right",
            Self::Down => "down",
        }
    }
}

/// The mark sets, as words in the file. `IconSet` is a domain type with no serde derive, so the
/// config declares its own and maps it, which is also what makes an unknown word name the two
/// that resolve.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Icons {
    #[default]
    NerdFont,
    Ascii,
}

impl From<Icons> for IconSet {
    fn from(icons: Icons) -> Self {
        match icons {
            Icons::NerdFont => IconSet::NerdFont,
            Icons::Ascii => IconSet::Ascii,
        }
    }
}

impl Config {
    /// Read the configuration file, or the defaults when there is none.
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

    /// The views the rest of the pane reads, the open list excluded: `Views::new` puts that first.
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

    /// The interval the pane refreshes on, the default when the config names none.
    pub fn refresh_seconds(&self) -> u64 {
        self.refresh_seconds.unwrap_or(DEFAULT_REFRESH_SECONDS)
    }

    /// A theme this pane has no palette for would draw half of it in the default colors, so an
    /// unknown name is a config error listing the names that resolve. The vocabulary belongs to
    /// the crate that paints, which hands it in.
    pub fn check_theme_against(&self, names: &[&str]) -> Result<(), String> {
        match &self.theme {
            Some(name) if !names.contains(&name.as_str()) => Err(format!(
                "unknown theme '{name}': expected one of {}",
                names.join(", ")
            )),
            _ => Ok(()),
        }
    }

    /// An argv with no word in it, or whose first word is blank, could spawn nothing at all, and
    /// the load is the place to learn that rather than the first read.
    fn check_dam(&self) -> Result<(), String> {
        match self.dam.first() {
            Some(word) if !word.trim().is_empty() => Ok(()),
            _ => Err("dam needs at least one word: the binary to spawn".to_string()),
        }
    }

    /// Two views with one name would make a picker entry and a `view` action ambiguous, so the
    /// second one is a config error naming the collision. The pane's own open list holds the first
    /// name, so a view may not take it either.
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

    /// A width is a share of the tab, so only a fraction between the two ends of it is a width at
    /// all: a zero, a whole tab or anything outside that says the operator meant something else.
    fn check_width(&self) -> Result<(), String> {
        match self.width {
            Some(width) if !(width > 0.0 && width < 1.0) => Err(format!(
                "width {width} is not a share of the tab: use a fraction above 0 and below 1"
            )),
            _ => Ok(()),
        }
    }

    /// The opening view has to be a view that exists, or the pane would open on nothing and say
    /// nothing about why.
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

/// herdr hands the plugin its own config directory; the documented path is the fallback for a run
/// outside herdr, such as `herdr-damnit doctor` from a shell.
fn config_path() -> PathBuf {
    let given = std::env::var_os("HERDR_PLUGIN_CONFIG_DIR").map(PathBuf::from);
    config_path_in(given.as_deref(), &base_config_dir())
}

fn config_path_in(given: Option<&Path>, base: &Path) -> PathBuf {
    let dir = match given {
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
