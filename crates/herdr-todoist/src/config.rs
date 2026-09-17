use std::path::PathBuf;

use serde::Deserialize;
use todoist::TokenSource;

/// The plugin's configuration, read from `config.toml` in the herdr plugin config directory. A
/// missing file is the default configuration; the token keys have no default, so the plugin
/// refuses to guess where a token lives.
#[derive(Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Argv of a command whose standard output is the token, for example a vault CLI call.
    pub token_command: Option<Vec<String>>,
    /// The name of an environment variable holding the token.
    pub token_env: Option<String>,
    /// How the `open` and `toggle` actions place the pane.
    #[serde(default)]
    pub placement: Placement,
    /// Which way a `split` placement splits.
    #[serde(default)]
    pub direction: Direction,
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

/// The split directions `herdr plugin pane open --direction` accepts.
#[derive(Debug, Default, Deserialize, PartialEq, Eq, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    #[default]
    Right,
    Down,
}

impl Direction {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Right => "right",
            Self::Down => "down",
        }
    }
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
        Ok(config)
    }

    /// Two views with one name would make a number key and a picker entry ambiguous, so the
    /// second one is a config error naming the collision.
    fn check_view_names(&self) -> Result<(), String> {
        let mut seen: Vec<&str> = Vec::new();
        for view in &self.views {
            if seen.contains(&view.name.as_str()) {
                return Err(format!("two views are named '{}'", view.name));
            }
            seen.push(&view.name);
        }
        Ok(())
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
/// outside herdr, such as `herdr-todoist doctor` from a shell.
fn config_path() -> PathBuf {
    let dir = match std::env::var_os("HERDR_PLUGIN_CONFIG_DIR") {
        Some(dir) => PathBuf::from(dir),
        None => base_config_dir().join("herdr/plugins/config/herdr-todoist"),
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
        assert_eq!(config.direction, Direction::Right);
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
    fn an_unrecognized_direction_is_a_parse_error_naming_the_alternatives() {
        let error = Config::parse(r#"direction = "sideways""#)
            .expect_err("refuses")
            .to_string();
        assert!(error.contains("right"), "{error}");
        assert!(error.contains("down"), "{error}");
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
    fn a_view_without_a_filter_is_a_config_error_naming_the_field() {
        let error = Config::parse("[[views]]\nname = \"today\"\n").expect_err("refuses");

        assert!(error.contains("filter"), "{error}");
    }

    #[test]
    fn a_view_without_a_name_is_a_config_error_naming_the_field() {
        let error = Config::parse("[[views]]\nfilter = \"today\"\n").expect_err("refuses");

        assert!(error.contains("name"), "{error}");
    }
}
