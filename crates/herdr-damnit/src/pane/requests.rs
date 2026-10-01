use herdr_damnit_adapters::{Config, state};
use herdr_damnit_domain::Views;

use super::{Mode, run};

pub const STATUS_REQUEST: &str = "status";

pub fn open_on_status(config: &Config) -> Result<String, String> {
    note_then_open(STATUS_REQUEST, &state::view_request_path(), || {
        run(Mode::Open, config)
    })
}

pub fn view(argument: &str, config: &Config) -> Result<String, String> {
    let number: usize = argument
        .parse()
        .map_err(|_| format!("'{argument}' is not a view number"))?;
    let views = Views::new(&config.views());
    let name = views.name_of_number(number).ok_or_else(|| {
        format!(
            "no view {number}: this config has {} views, 1 being the unfiltered list",
            views.len()
        )
    })?;
    note_then_open(name, &state::view_request_path(), || {
        run(Mode::Open, config)
    })
}

pub(super) fn note_then_open(
    name: &str,
    request: &std::path::Path,
    open: impl FnOnce() -> Result<String, String>,
) -> Result<String, String> {
    state::request_view(request, name);
    match open() {
        Ok(outcome) => Ok(format!("{outcome}, showing {name}")),
        Err(error) => {
            state::clear_view_request(request);
            Err(error)
        }
    }
}
