mod app;
mod doctor;
mod editor;
mod legend;
mod loop_;
mod markdown;
mod open;
mod overlay;
mod pane;
mod screens;
mod theme;

use std::path::PathBuf;

use herdr_damnit_adapters::{CliHerdr, Config, ProcessDamRunner, StateDir, SystemClock};
use herdr_damnit_application::{Jobs, Workspace};
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
            Some(run) => with_shell(run),
            None => fail(&format!("unknown command '{command}'\n{USAGE}")),
        },
        [command, argument] if command == "view" => with_shell(|shell| {
            pane::view(
                argument,
                &shell.here,
                &shell.herdr,
                &shell.state,
                &shell.config,
            )
        }),
        _ => fail(&format!("too many arguments\n{USAGE}")),
    }
}

struct Shell {
    config: Config,
    here: Workspace,
    herdr: CliHerdr,
    state: StateDir,
}

impl Shell {
    fn from_environment() -> Result<Self, String> {
        Ok(Self {
            config: Config::load()?,
            here: Workspace {
                workspace: std::env::var("HERDR_WORKSPACE_ID").ok(),
                me: std::env::var("HERDR_PANE_ID").unwrap_or_default(),
            },
            herdr: CliHerdr::new(
                std::env::var("HERDR_BIN_PATH").unwrap_or_else(|_| "herdr".to_string()),
                std::env::var("HERDR_PLUGIN_ID").unwrap_or_else(|_| "herdr-damnit".to_string()),
            ),
            state: StateDir::locate(
                std::env::var_os("HERDR_PLUGIN_STATE_DIR")
                    .map(PathBuf::from)
                    .as_deref(),
                &state_home(),
            ),
        })
    }

    fn place(&self, mode: Mode) -> Result<String, String> {
        pane::run(mode, &self.here, &self.herdr, &self.state, &self.config)
    }
}

fn state_home() -> PathBuf {
    match std::env::var_os("XDG_STATE_HOME") {
        Some(dir) => PathBuf::from(dir),
        None => PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".local/state"),
    }
}

type Action = fn(&Shell) -> Result<String, String>;

fn action(name: &str) -> Option<Action> {
    Some(match name {
        "doctor" => |shell| doctor::run(&shell.config),
        "open" => |shell| shell.place(Mode::Open),
        "toggle" => |shell| shell.place(Mode::Toggle),
        "focus" => |shell| shell.place(Mode::Focus),
        "status" => {
            |shell| pane::open_on_status(&shell.here, &shell.herdr, &shell.state, &shell.config)
        }
        "auto-open" => {
            |shell| pane::auto_open(&shell.here, &shell.herdr, &shell.state, &shell.config)
        }
        _ => return None,
    })
}

fn run_pane() -> std::process::ExitCode {
    let shell = match Shell::from_environment() {
        Ok(shell) => shell,
        Err(error) => return fail(&error),
    };
    let jobs = Jobs::new(
        Box::new(ProcessDamRunner::new(shell.config.dam.clone())),
        Box::new(SystemClock),
    );
    let mut app = app::App::new(
        shell.config,
        jobs,
        Box::new(shell.herdr),
        shell.here,
        Box::new(SystemClock),
    );
    app.watch_view_requests(shell.state.view_request_path());
    app.watch_config(Config::path());
    open::start(&mut app);
    match loop_::run(&mut app) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => fail(&error),
    }
}

fn with_shell(run: impl FnOnce(&Shell) -> Result<String, String>) -> std::process::ExitCode {
    report(Shell::from_environment().and_then(|shell| run(&shell)))
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
