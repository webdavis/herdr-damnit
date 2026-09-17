//! The plugin binary. With no arguments it is the pane; the subcommands are the plugin actions.

mod config;
mod cursor;
mod doctor;
mod list;
mod pane;
mod tui;

use config::Config;
use pane::Mode;

const USAGE: &str = "\
usage: herdr-todoist [<command>]

  (no command)   run the Todoist pane
  open           open the pane in this workspace, or focus it when it is already open
  toggle         open the pane, or close it when it is already open
  focus          focus the pane in this workspace
  doctor         check that the token resolves and one API request succeeds
";

#[tokio::main]
async fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [] => match Config::load() {
            Ok(config) => match tui::run(&config, todoist::DEFAULT_BASE_URL).await {
                Ok(()) => std::process::ExitCode::SUCCESS,
                Err(error) => fail(&error),
            },
            Err(error) => fail(&error),
        },
        [command] => match command.as_str() {
            "--help" | "-h" | "help" => {
                print!("{USAGE}");
                std::process::ExitCode::SUCCESS
            }
            "doctor" => match Config::load() {
                Ok(config) => report(doctor::run(&config, todoist::DEFAULT_BASE_URL).await),
                Err(error) => fail(&error),
            },
            "open" => with_config(|config| pane::run(Mode::Open, config)),
            "toggle" => with_config(|config| pane::run(Mode::Toggle, config)),
            "focus" => with_config(|config| pane::run(Mode::Focus, config)),
            other => fail(&format!("unknown command '{other}'\n{USAGE}")),
        },
        _ => fail(&format!("too many arguments\n{USAGE}")),
    }
}

/// Load the config and run a synchronous action against it, so a bad config only breaks the
/// commands that actually need one.
fn with_config(run: impl FnOnce(&Config) -> Result<String, String>) -> std::process::ExitCode {
    report(Config::load().and_then(|config| run(&config)))
}

fn report(outcome: Result<String, String>) -> std::process::ExitCode {
    match outcome {
        Ok(message) => {
            println!("{message}");
            std::process::ExitCode::SUCCESS
        }
        Err(error) => fail(&error),
    }
}

fn fail(error: &str) -> std::process::ExitCode {
    eprintln!("herdr-todoist: {error}");
    std::process::ExitCode::FAILURE
}
