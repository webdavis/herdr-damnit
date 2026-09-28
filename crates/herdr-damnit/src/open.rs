use herdr_damnit_application::{JobKind, argv};

use crate::app::App;

pub fn start(app: &mut App) {
    app.submit(JobKind::Version, argv::version());
}

#[cfg(test)]
mod tests;
