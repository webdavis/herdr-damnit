mod app;
mod doctor;
mod editor;
mod loop_;
mod markdown;
mod open;
mod overlay;
mod pane;
mod screens;
mod theme;

use herdr_damnit_adapters::{CliHerdr, Config, ProcessDamRunner, SystemClock};
use herdr_damnit_application::{Clock, Jobs, Workspace};
use pane::Mode;

const USAGE: &str = "\
usage: herdr-damnit [<command>]

  (no command)   run the task pane
  open           open the pane in this workspace, or focus it when it is already open
  toggle         open the pane, or close it when it is already open
  focus          focus the pane in this workspace
  auto-open      open the pane when the config asks for it, the workspace-focus hook
  status         open or focus the pane on its status screen
  view <n>       show the nth configured view, opening the pane when it is closed
  doctor         check that dam answers, its status carries every key the pane reads, and its remotes
";

fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [] => run_pane(),
        [command] if matches!(command.as_str(), "--help" | "-h" | "help") => {
            print!("{USAGE}");
            std::process::ExitCode::SUCCESS
        }
        [command] => match action(command) {
            Some(run) => with_config(run),
            None => fail(&format!("unknown command '{command}'\n{USAGE}")),
        },
        [command, argument] if command == "view" => {
            with_config(|config| pane::view(argument, config))
        }
        _ => fail(&format!("too many arguments\n{USAGE}")),
    }
}

type Action = fn(&Config) -> Result<String, String>;

fn action(name: &str) -> Option<Action> {
    Some(match name {
        "doctor" => doctor::run,
        "open" => |config| pane::run(Mode::Open, config),
        "toggle" => |config| pane::run(Mode::Toggle, config),
        "focus" => |config| pane::run(Mode::Focus, config),
        "status" => pane::open_on_status,
        "auto-open" => pane::auto_open,
        _ => return None,
    })
}

fn load_config_and_check_its_theme() -> Result<Config, String> {
    load_with_theme_check(Config::load)
}

fn load_with_theme_check(load: impl FnOnce() -> Result<Config, String>) -> Result<Config, String> {
    let config = load()?;
    config.check_theme_against(theme::NAMES)?;
    Ok(config)
}

fn run_pane() -> std::process::ExitCode {
    let config = match load_config_and_check_its_theme() {
        Ok(config) => config,
        Err(error) => return fail(&error),
    };
    let clock = SystemClock;
    let today = clock.today();
    let jobs = Jobs::new(
        Box::new(ProcessDamRunner::new(config.dam.clone())),
        Box::new(clock),
    );
    let here = Workspace {
        workspace: std::env::var("HERDR_WORKSPACE_ID").ok(),
        me: std::env::var("HERDR_PANE_ID").unwrap_or_default(),
    };
    let mut app = app::App::new(config, jobs, Box::new(CliHerdr), here, today);
    app.view_request = Some(herdr_damnit_adapters::state::view_request_path());
    open::start(&mut app);
    match loop_::run(&mut app) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => fail(&error),
    }
}

fn with_config(run: impl FnOnce(&Config) -> Result<String, String>) -> std::process::ExitCode {
    report(load_config_and_check_its_theme().and_then(|config| run(&config)))
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
    eprintln!("herdr-damnit: {error}");
    std::process::ExitCode::FAILURE
}

#[cfg(test)]
mod tests;
