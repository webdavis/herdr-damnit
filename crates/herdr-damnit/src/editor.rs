use std::process::Command;

use herdr_damnit_domain::{Failure, message};

use crate::app::App;

/// Runs `dam edit <oid> -e` on the pane's own terminal, then re-reads whatever it exited with,
/// since someone who quit the editor in a hurry may still have saved.
pub fn run_and_reread(app: &mut App, argv: &[String]) {
    if let Some((binary, leading)) = app.config.dam.split_first() {
        match Command::new(binary).args(leading).args(argv).status() {
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                app.message = message(&Failure::NotInstalled);
            }
            Err(error) => app.message = error.to_string(),
        }
    }
    app.reread();
}

pub fn round_trip(terminal: &mut ratatui::DefaultTerminal, app: &mut App, argv: &[String]) {
    ratatui::restore();
    run_and_reread(app, argv);
    *terminal = ratatui::init();
}
