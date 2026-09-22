//! The draw loop: draw every tick, drain the job channel, poll for a key within the window the
//! model asks for. Nothing here waits on `dam`.

use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyEvent, KeyEventKind};

use crate::app::{After, App};

pub fn run(app: &mut App) -> Result<(), String> {
    let mut terminal = ratatui::init();
    let outcome = loop {
        if let Err(error) = terminal.draw(|frame| crate::screens::draw(frame, app)) {
            break Err(error.to_string());
        }
        app.tick(Instant::now());
        match next_key(app.poll_window()) {
            Err(error) => break Err(error),
            Ok(None) => continue,
            Ok(Some(key)) => match app.key(key) {
                After::Stay => {}
            },
        }
    };
    ratatui::restore();
    outcome
}

fn next_key(window: Duration) -> Result<Option<KeyEvent>, String> {
    if !event::poll(window).map_err(|error| error.to_string())? {
        return Ok(None);
    }
    match event::read().map_err(|error| error.to_string())? {
        Event::Key(key) if key.kind == KeyEventKind::Press => Ok(Some(key)),
        _ => Ok(None),
    }
}
