use std::path::{Path, PathBuf};

use serde::Deserialize;
use todoist::TokenSource;

use crate::icons::IconSet;
use crate::placement::Side;

/// The plugin's configuration, read from `config.toml` in the herdr plugin config directory. A
/// missing file is the default configuration; the token keys have no default, so the plugin
/// refuses to guess where a token lives.
#[derive(Debug, Default, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Argv of a command whose standard output is the token, for example a vault CLI call.
    pub token_command: Option<Vec<String>>,
    /// The name of an environment variable holding the token.
    pub token_env: Option<String>,
    /// How the `open` and `toggle` actions place the pane.
    #[serde(default)]
    pub placement: Placement,
    /// Which side of the calling pane a `split` placement takes.
    #[serde(default)]
    pub side: Side,
    /// The share of the tab the pane takes, left to herdr's own even split when unset.
    pub width: Option<f32>,
    /// The view the pane opens on, the unfiltered list when unset.
    pub default_view: Option<String>,
    /// Argv of the editor `e` enters on a task, Neovim when unset and the pane's own box when it
    /// is an empty list.
    pub editor: Option<Vec<String>>,
    /// Whether focusing a workspace opens the pane there on its own.
    #[serde(default)]
    pub auto_open: bool,
    /// The theme the pane paints with, by the name herdr and reviewr know it by, so the panes of
    /// one workspace match. The default theme when unset.
    pub theme: Option<String>,
    /// Which set of marks a task line carries: Nerd Font glyphs, or plain characters for a
    /// terminal whose font has none.
    #[serde(default)]
    pub icons: IconSet,
    /// How often the pane reads the API on its own, in seconds. Zero turns the interval off,
    /// leaving `R` and the read that follows every write.
    pub refresh_seconds: Option<u64>,
    /// Named filter views, in the order the pane numbers them.
    #[serde(default)]
    pub views: Vec<View>,
}

/// One named view: a name to pick it by and a Todoist filter query, the language the app's
/// Filters feature uses.
#[derive(Debug, Deserialize, PartialEq, Eq, Clone)]
#[serde(deny_unknown_fields)]
pub struct View {
    pub name: String,
    pub filter: String,
}

/// The pane placements `herdr plugin pane open --placement` accepts. An unrecognized value is a
/// config parse error naming these, rather than a raw error from `herdr` at action time.
#[derive(Debug, Default, Deserialize, PartialEq, Eq, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum Placement {
    Overlay,
    #[default]
    Split,
    Tab,
    Zoomed,
}

impl Config {
    /// Read the configuration file, or the defaults when there is none.
    pub fn load() -> Result<Self, String> {
        let path = config_path();
        match std::fs::read_to_string(&path) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(error) => Err(format!("{}: {error}", path.display())),
            Ok(text) => Self::parse(&text).map_err(|error| format!("{}: {error}", path.display())),
        }
    }

    pub fn parse(text: &str) -> Result<Self, String> {
        let config: Self = toml::from_str(text).map_err(|error| error.to_string())?;
        config.check_view_names()?;
        config.check_width()?;
        config.check_default_view()?;
        config.check_theme()?;
        Ok(config)
    }

    /// Two views with one name would make a picker entry and a `view` action ambiguous, so the
    /// second one is a config error naming the collision. The pane's own unfiltered list holds
    /// the first name, so a view may not take it either.
    fn check_view_names(&self) -> Result<(), String> {
        let mut seen: Vec<&str> = Vec::new();
        for view in &self.views {
            if view.name.trim().is_empty() {
                return Err("a view needs a name".to_string());
            }
            if view.filter.trim().is_empty() {
                return Err(format!("view '{}' has an empty filter", view.name));
            }
            if view.name == crate::views::ALL {
                return Err(format!(
                    "a view cannot be named '{}': that is the pane's own unfiltered list",
                    crate::views::ALL
                ));
            }
            if seen.contains(&view.name.as_str()) {
                return Err(format!("two views are named '{}'", view.name));
            }
            seen.push(&view.name);
        }
        Ok(())
    }

    /// A theme this pane has no palette for would draw half of it in the default colors, so an
    /// unknown name is a config error listing the names that resolve.
    fn check_theme(&self) -> Result<(), String> {
        match &self.theme {
            Some(name) if !crate::theme::is_known(name) => Err(format!(
                "unknown theme '{name}': expected one of {}",
                crate::theme::NAMES.join(", ")
            )),
            _ => Ok(()),
        }
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
        let known = name == crate::views::ALL || self.views.iter().any(|view| &view.name == name);
        if known {
            return Ok(());
        }
        let mut names = vec![crate::views::ALL.to_string()];
        names.extend(self.views.iter().map(|view| view.name.clone()));
        Err(format!(
            "default_view '{name}' is not a view: the views are {}",
            names.join(", ")
        ))
    }

    /// The interval the pane refreshes on, the default when the config names none.
    pub fn refresh_seconds(&self) -> u64 {
        self.refresh_seconds
            .unwrap_or(crate::refresh::DEFAULT_SECONDS)
    }

    /// Where the token comes from. `token_command` wins when both keys are set.
    pub fn token_source(&self) -> Result<TokenSource, String> {
        match (&self.token_command, &self.token_env) {
            (Some(argv), _) => Ok(TokenSource::Command(argv.clone())),
            (None, Some(name)) => Ok(TokenSource::Env(name.clone())),
            (None, None) => Err(
                "no token source: set token_command (a command printing the token) or token_env \
                 (the name of an environment variable holding it) in the plugin config"
                    .to_string(),
            ),
        }
    }
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
mod tests {
    use super::*;

    #[test]
    fn an_empty_file_is_the_default_placement() {
        let config = Config::parse("").expect("parses");
        assert_eq!(config.placement, Placement::Split);
        assert_eq!(config.side, Side::Right);
        assert_eq!(config.width, None);
        assert_eq!(config.default_view, None);
        assert_eq!(config.editor, None);
        assert!(!config.auto_open);
    }

    #[test]
    fn an_unrecognized_placement_is_a_parse_error_naming_the_alternatives() {
        let error = Config::parse(r#"placement = "popup""#)
            .expect_err("refuses")
            .to_string();
        assert!(error.contains("overlay"), "{error}");
        assert!(error.contains("split"), "{error}");
        assert!(error.contains("tab"), "{error}");
        assert!(error.contains("zoomed"), "{error}");
    }

    #[test]
    fn an_unrecognized_side_is_a_parse_error_naming_the_alternatives() {
        let error = Config::parse(r#"side = "sideways""#)
            .expect_err("refuses")
            .to_string();
        assert!(error.contains("right"), "{error}");
        assert!(error.contains("down"), "{error}");
    }

    /// `herdr plugin pane open --direction` only splits rightward or downward, and a same-tab
    /// `herdr pane move` cannot reposition a pane afterward, so left and up are not sides this
    /// plugin can place a pane on at all.
    #[test]
    fn a_side_herdr_cannot_split_toward_is_a_parse_error() {
        for text in ["left", "up"] {
            let error = Config::parse(&format!("side = \"{text}\"")).expect_err("refuses");
            assert!(error.contains("unknown variant"), "{text}: {error}");
        }
    }

    #[test]
    fn every_side_is_read() {
        for (text, side) in [("right", Side::Right), ("down", Side::Down)] {
            let config = Config::parse(&format!("side = \"{text}\"")).expect("parses");
            assert_eq!(config.side, side);
        }
    }

    #[test]
    fn a_width_is_read_as_a_share_of_the_tab() {
        assert_eq!(
            Config::parse("width = 0.3").expect("parses").width,
            Some(0.3)
        );
    }

    #[test]
    fn a_width_outside_the_tab_is_refused_by_the_number_it_was_given() {
        for width in ["0.0", "0", "1.0", "1.5", "-0.2"] {
            let error = Config::parse(&format!("width = {width}")).expect_err("refuses");
            assert!(error.contains("share of the tab"), "{width}: {error}");
        }
    }

    #[test]
    fn auto_open_is_read_and_is_off_unless_it_is_asked_for() {
        assert!(Config::parse("auto_open = true").expect("parses").auto_open);
        assert!(
            !Config::parse("auto_open = false")
                .expect("parses")
                .auto_open
        );
    }

    #[test]
    fn the_opening_view_may_be_a_configured_view_or_the_unfiltered_list() {
        let config = Config::parse(
            "default_view = \"today\"\n[[views]]\nname = \"today\"\nfilter = \"today\"\n",
        )
        .expect("parses");
        assert_eq!(config.default_view, Some("today".to_string()));

        let all = Config::parse("default_view = \"all\"").expect("parses");
        assert_eq!(all.default_view, Some("all".to_string()));
    }

    #[test]
    fn an_opening_view_no_view_carries_is_refused_and_names_the_views() {
        let error = Config::parse(
            "default_view = \"work\"\n[[views]]\nname = \"today\"\nfilter = \"today\"\n",
        )
        .expect_err("refuses");

        assert!(
            error.contains("default_view 'work' is not a view"),
            "{error}"
        );
        assert!(error.contains("all, today"), "{error}");
    }

    #[test]
    fn an_editor_is_read_as_argv_so_a_path_with_a_space_in_it_stays_one_word() {
        let config =
            Config::parse(r#"editor = ["/Applications/My Editor/bin/nvim"]"#).expect("parses");

        assert_eq!(
            config.editor,
            Some(vec!["/Applications/My Editor/bin/nvim".to_string()])
        );
    }

    #[test]
    fn an_empty_editor_list_is_read_as_no_editor_at_all() {
        assert_eq!(
            Config::parse("editor = []").expect("parses").editor,
            Some(Vec::new())
        );
    }

    #[test]
    fn a_command_is_read_as_argv() {
        let config =
            Config::parse(r#"token_command = ["vault", "read", "todoist"]"#).expect("parses");
        assert_eq!(
            config.token_source().expect("a source"),
            TokenSource::Command(vec![
                "vault".to_string(),
                "read".to_string(),
                "todoist".to_string()
            ])
        );
    }

    #[test]
    fn a_command_wins_over_an_environment_variable() {
        let config =
            Config::parse("token_command = [\"vault\"]\ntoken_env = \"TODOIST_API_TOKEN\"")
                .expect("parses");
        assert_eq!(
            config.token_source().expect("a source"),
            TokenSource::Command(vec!["vault".to_string()])
        );
    }

    #[test]
    fn an_environment_variable_name_is_read() {
        let config = Config::parse(r#"token_env = "TODOIST_API_TOKEN""#).expect("parses");
        assert_eq!(
            config.token_source().expect("a source"),
            TokenSource::Env("TODOIST_API_TOKEN".to_string())
        );
    }

    #[test]
    fn no_token_key_names_both_keys_and_guesses_nothing() {
        let error = Config::default().token_source().expect_err("refuses");
        assert!(error.contains("token_command"), "{error}");
        assert!(error.contains("token_env"), "{error}");
    }

    #[test]
    fn a_token_value_in_the_file_is_refused() {
        let error = Config::parse(r#"token = "a-secret""#).expect_err("refuses");
        assert!(error.contains("unknown field"), "{error}");
    }

    #[test]
    fn views_are_read_in_the_order_they_are_written() {
        let config = Config::parse(
            "[[views]]\nname = \"today\"\nfilter = \"today | overdue\"\n\
             [[views]]\nname = \"work\"\nfilter = \"#Work & !@waiting\"\n",
        )
        .expect("parses");

        assert_eq!(
            config.views,
            vec![
                View {
                    name: "today".to_string(),
                    filter: "today | overdue".to_string()
                },
                View {
                    name: "work".to_string(),
                    filter: "#Work & !@waiting".to_string()
                },
            ]
        );
    }

    #[test]
    fn two_views_with_one_name_are_a_config_error() {
        let error = Config::parse(
            "[[views]]\nname = \"today\"\nfilter = \"today\"\n\
             [[views]]\nname = \"today\"\nfilter = \"overdue\"\n",
        )
        .expect_err("refuses");

        assert!(error.contains("two views are named 'today'"), "{error}");
    }

    #[test]
    fn a_view_named_after_the_unfiltered_list_is_a_config_error() {
        let error =
            Config::parse("[[views]]\nname = \"all\"\nfilter = \"today\"\n").expect_err("refuses");

        assert!(error.contains("cannot be named 'all'"), "{error}");
    }

    #[test]
    fn a_view_without_a_filter_is_a_config_error_naming_the_field() {
        let error = Config::parse("[[views]]\nname = \"today\"\n").expect_err("refuses");

        assert!(error.contains("filter"), "{error}");
    }

    #[test]
    fn a_view_without_a_name_is_a_config_error_naming_the_field() {
        let error = Config::parse("[[views]]\nfilter = \"today\"\n").expect_err("refuses");

        assert!(error.contains("name"), "{error}");
    }

    #[test]
    fn a_view_with_an_empty_name_is_a_config_error() {
        let error =
            Config::parse("[[views]]\nname = \"\"\nfilter = \"today\"\n").expect_err("refuses");

        assert!(error.contains("needs a name"), "{error}");
    }

    #[test]
    fn a_view_with_an_empty_filter_is_a_config_error_naming_the_view() {
        let error =
            Config::parse("[[views]]\nname = \"today\"\nfilter = \"\"\n").expect_err("refuses");

        assert!(
            error.contains("view 'today' has an empty filter"),
            "{error}"
        );
    }

    #[test]
    fn a_theme_is_taken_by_name_and_an_unknown_one_is_refused_with_the_names_that_work() {
        let config = Config::parse("theme = \"gruvbox\"\n").expect("parses");
        assert_eq!(config.theme.as_deref(), Some("gruvbox"));

        let error = Config::parse("theme = \"nope\"\n").expect_err("refuses");
        assert!(error.contains("unknown theme 'nope'"), "{error}");
        assert!(error.contains("catppuccin"), "{error}");
    }

    #[test]
    fn the_marks_are_nerd_font_glyphs_until_the_plain_set_is_asked_for() {
        assert_eq!(Config::default().icons, IconSet::NerdFont);
        assert_eq!(
            Config::parse("icons = \"ascii\"\n").expect("parses").icons,
            IconSet::Ascii
        );

        let error = Config::parse("icons = \"emoji\"\n").expect_err("refuses");
        assert!(error.contains("icons"), "{error}");
    }

    #[test]
    fn the_config_directory_is_the_plugins_own_name_under_herdr() {
        let path = config_path_in(None, Path::new("/x/.config"));
        assert_eq!(
            path,
            Path::new("/x/.config/herdr/plugins/config/herdr-damnit/config.toml")
        );
    }

    #[test]
    fn herdrs_own_config_directory_wins_when_it_names_one() {
        let path = config_path_in(Some(Path::new("/given")), Path::new("/x/.config"));
        assert_eq!(path, Path::new("/given/config.toml"));
    }
}
