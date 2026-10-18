use std::process::Command;

use herdr_damnit_domain::{Failure, message};

use crate::app::App;

pub fn run_and_reread(app: &mut App, argv: &[String]) {
    let dam = app.config().dam.clone();
    if let Some((binary, leading)) = dam.split_first() {
        match Command::new(binary).args(leading).args(argv).status() {
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                app.say(message(&Failure::NotInstalled));
            }
            Err(error) => app.say(error.to_string()),
        }
    }
    app.reread();
}

pub fn round_trip(terminal: &mut ratatui::DefaultTerminal, app: &mut App, argv: &[String]) {
    ratatui::restore();
    run_and_reread(app, argv);
    *terminal = ratatui::init();
}
