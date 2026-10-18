use herdr_damnit_application::Request;

use crate::app::App;

pub fn start(app: &mut App) {
    app.submit(Request::Version);
}

#[cfg(test)]
mod tests;
