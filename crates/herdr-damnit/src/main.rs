//! The plugin binary. With no arguments it is the pane; the subcommands are the plugin actions.

mod app;
mod doctor;
mod loop_;
mod open;
mod pane;
mod screens;
mod theme;

use herdr_damnit_adapters::{Config, ProcessDamRunner, SystemClock};
use herdr_damnit_application::{Clock, Jobs};
use pane::Mode;

const USAGE: &str = "\
usage: herdr-damnit [<command>]

  (no command)   run the task pane
  open           open the pane in this workspace, or focus it when it is already open
  toggle         open the pane, or close it when it is already open
  focus          focus the pane in this workspace
  auto-open      open the pane when the config asks for it, the workspace-focus hook
  view <n>       show the nth configured view, opening the pane when it is closed
  doctor         check that dam answers and its status carries every key the pane reads
";

fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [] => run_pane(),
        [command] => match command.as_str() {
            "--help" | "-h" | "help" => {
                print!("{USAGE}");
                std::process::ExitCode::SUCCESS
            }
            "doctor" => with_config(doctor::run),
            "open" => with_config(|config| pane::run(Mode::Open, config)),
            "toggle" => with_config(|config| pane::run(Mode::Toggle, config)),
            "focus" => with_config(|config| pane::run(Mode::Focus, config)),
            "auto-open" => with_config(pane::auto_open),
            other => fail(&format!("unknown command '{other}'\n{USAGE}")),
        },
        [command, argument] if command == "view" => {
            with_config(|config| pane::view(argument, config))
        }
        _ => fail(&format!("too many arguments\n{USAGE}")),
    }
}

/// The configuration plus the one check the adapters crate cannot make for itself: the theme
/// vocabulary is this crate's, so `Config::parse` never sees it and every load goes through here.
/// Skip this and an unknown theme name is accepted in silence and the pane paints with defaults.
fn load_config() -> Result<Config, String> {
    load_config_with(Config::load)
}

fn load_config_with(load: impl FnOnce() -> Result<Config, String>) -> Result<Config, String> {
    let config = load()?;
    config.check_theme_against(theme::NAMES)?;
    Ok(config)
}

fn run_pane() -> std::process::ExitCode {
    let config = match load_config() {
        Ok(config) => config,
        Err(error) => return fail(&error),
    };
    let clock = SystemClock;
    let today = clock.today();
    let jobs = Jobs::new(
        Box::new(ProcessDamRunner::new(config.dam.clone())),
        Box::new(clock),
    );
    let mut app = app::App::new(config, jobs, today);
    open::start(&mut app);
    match loop_::run(&mut app) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => fail(&error),
    }
}

fn with_config(run: impl FnOnce(&Config) -> Result<String, String>) -> std::process::ExitCode {
    report(load_config().and_then(|config| run(&config)))
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
mod tests {
    use super::*;

    #[test]
    fn the_config_funnel_rejects_a_theme_the_pane_cannot_draw() {
        let error = load_config_with(|| Config::parse("theme = 'unknown'"))
            .expect_err("the theme is unknown");

        assert!(error.contains("unknown theme 'unknown'"), "{error}");
    }
}
