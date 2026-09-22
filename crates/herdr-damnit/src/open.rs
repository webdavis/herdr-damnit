//! What the pane does before it draws rows: the version handshake, and the first two reads. The
//! handshake is two process starts at open and is not repeated on a refresh.

use herdr_damnit_application::{JobKind, argv};

use crate::app::App;

/// Ask `dam` its version. Everything else follows from the answer.
pub fn start(app: &mut App) {
    app.submit(JobKind::Version, argv::version());
}

#[cfg(test)]
mod tests;
