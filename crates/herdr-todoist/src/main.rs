//! The plugin binary. With no arguments it is the pane; the subcommands are the plugin actions.

mod config;
mod doctor;
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
    let config = match Config::load() {
        Ok(config) => config,
        Err(error) => return fail(&error),
    };
    match args.as_slice() {
        [] => match tui::run(&config).await {
            Ok(()) => std::process::ExitCode::SUCCESS,
            Err(error) => fail(&error),
        },
        [command] => match command.as_str() {
            "doctor" => report(doctor::run(&config).await),
            "open" => report(pane::run(Mode::Open, &config)),
            "toggle" => report(pane::run(Mode::Toggle, &config)),
            "focus" => report(pane::run(Mode::Focus, &config)),
            "--help" | "-h" | "help" => {
                print!("{USAGE}");
                std::process::ExitCode::SUCCESS
            }
            other => fail(&format!("unknown command '{other}'\n{USAGE}")),
        },
        _ => fail(&format!("too many arguments\n{USAGE}")),
    }
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
