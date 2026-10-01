use herdr_damnit_adapters::config::Placement;
use herdr_damnit_adapters::{Config, herdr_cli as herdr, state};

mod requests;

pub use requests::{STATUS_REQUEST, open_on_status, view};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Open,
    Toggle,
    Focus,
    AutoOpen,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Decision {
    Open,
    Close(String),
    Focus(String),
    LeaveOpen(String),
    NothingToFocus,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Focus {
    Take,
    Leave,
}

impl Focus {
    fn as_flag(self) -> &'static str {
        match self {
            Self::Take => "--focus",
            Self::Leave => "--no-focus",
        }
    }
}

pub fn decide(mode: Mode, remembered: Option<&str>, live: &[String]) -> Decision {
    let open_pane = remembered.filter(|pane| live.iter().any(|live| live == pane));
    match (mode, open_pane) {
        (Mode::Toggle, Some(pane)) => Decision::Close(pane.to_string()),
        (Mode::AutoOpen, Some(pane)) => Decision::LeaveOpen(pane.to_string()),
        (Mode::Open | Mode::Focus, Some(pane)) => Decision::Focus(pane.to_string()),
        (Mode::Open | Mode::Toggle | Mode::AutoOpen, None) => Decision::Open,
        (Mode::Focus, None) => Decision::NothingToFocus,
    }
}

enum Recovery {
    RetryOpen,
    ReportGone,
}

fn recovery_from_a_rejected_pane(mode: Mode) -> Recovery {
    match mode {
        Mode::Focus => Recovery::ReportGone,
        Mode::Open | Mode::Toggle | Mode::AutoOpen => Recovery::RetryOpen,
    }
}

fn focus_of(mode: Mode) -> Focus {
    match mode {
        Mode::AutoOpen => Focus::Leave,
        Mode::Open | Mode::Toggle | Mode::Focus => Focus::Take,
    }
}

pub fn run(mode: Mode, config: &Config) -> Result<String, String> {
    let workspace = std::env::var("HERDR_WORKSPACE_ID")
        .map_err(|_| "no workspace context: run this action from inside herdr".to_string())?;
    let live = herdr::live_panes(&workspace)?;
    let remembered = state::remembered_pane(&workspace);
    match decide(mode, remembered.as_deref(), &live) {
        Decision::Focus(pane) => match herdr::focus_plugin_pane(&pane) {
            Ok(_) => Ok(format!("focused {pane}")),
            Err(_) => {
                state::forget_pane(&workspace);
                match recovery_from_a_rejected_pane(mode) {
                    Recovery::RetryOpen => open_and_remember(&workspace, config, focus_of(mode)),
                    Recovery::ReportGone => {
                        Ok(format!("no dam pane in {workspace}: {pane} is gone"))
                    }
                }
            }
        },
        Decision::Close(pane) => {
            let closed = herdr::close_plugin_pane(&pane);
            state::forget_pane(&workspace);
            match closed {
                Ok(_) => Ok(format!("closed {pane}")),
                Err(_) => open_and_remember(&workspace, config, focus_of(mode)),
            }
        }
        Decision::LeaveOpen(pane) => Ok(format!("{pane} is already open")),
        Decision::NothingToFocus => Ok(format!("no dam pane in {workspace}")),
        Decision::Open => open_and_remember(&workspace, config, focus_of(mode)),
    }
}

pub fn auto_open(config: &Config) -> Result<String, String> {
    if !config.auto_open {
        return Ok("auto_open is off".to_string());
    }
    run(Mode::AutoOpen, config)
}

fn open_and_remember(workspace: &str, config: &Config, focus: Focus) -> Result<String, String> {
    let neighbor = std::env::var("HERDR_PANE_ID").unwrap_or_default();
    let pane = herdr::open_plugin_pane(workspace, &neighbor, focus.as_flag(), config)?;
    let note = resize_the_calling_pane_to_leave_the_configured_width(&neighbor, config);
    state::remember_pane(workspace, &pane);
    Ok(match note {
        Some(note) => format!("opened {pane}, {note}"),
        None => format!("opened {pane}"),
    })
}

fn calling_panes_share_after(width: f32) -> f32 {
    1.0 - width
}

fn resize_the_calling_pane_to_leave_the_configured_width(
    neighbor: &str,
    config: &Config,
) -> Option<String> {
    if config.placement != Placement::Split || neighbor.is_empty() {
        return None;
    }
    let target_ratio = calling_panes_share_after(config.width?);
    let direction = config.side.split_direction();
    match herdr::resize_leading_pane(neighbor, direction, target_ratio) {
        Ok(true) => None,
        Ok(false) => Some("placement refused: herdr did not resize the pane".to_string()),
        Err(error) => Some(format!("placement refused: {error}")),
    }
}

#[cfg(test)]
mod tests;
