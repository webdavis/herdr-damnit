use herdr_damnit_adapters::{Config, StateDir};
use herdr_damnit_application::{PluginPanes, Workspace};
use herdr_damnit_domain::Views;

use super::{Mode, run};

pub const STATUS_REQUEST: &str = "status";

pub fn open_on_status(
    here: &Workspace,
    panes: &dyn PluginPanes,
    state: &StateDir,
    config: &Config,
) -> Result<String, String> {
    note_then_open(STATUS_REQUEST, state, || {
        run(Mode::Open, here, panes, state, config)
    })
}

pub fn view(
    argument: &str,
    here: &Workspace,
    panes: &dyn PluginPanes,
    state: &StateDir,
    config: &Config,
) -> Result<String, String> {
    let number: usize = argument
        .parse()
        .map_err(|_| format!("'{argument}' is not a view number"))?;
    let views = Views::new(&config.views());
    let name = views.name_of_number(number).ok_or_else(|| {
        format!(
            "no view {number}: this config has {} views, 1 being the unfiltered list",
            views.count()
        )
    })?;
    note_then_open(name, state, || run(Mode::Open, here, panes, state, config))
}

pub(super) fn note_then_open(
    name: &str,
    state: &StateDir,
    open: impl FnOnce() -> Result<String, String>,
) -> Result<String, String> {
    state.request_view(name);
    match open() {
        Ok(outcome) => Ok(format!("{outcome}, showing {name}")),
        Err(error) => {
            state.clear_view_request();
            Err(error)
        }
    }
}
