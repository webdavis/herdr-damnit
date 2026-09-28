use herdr_damnit_adapters::config::Placement;
use herdr_damnit_adapters::{Config, herdr_cli as herdr, state};
use herdr_damnit_domain::Views;

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

fn note_then_open(
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
mod tests {
    use super::*;

    fn default_config() -> Config {
        Config::parse("").expect("the default config")
    }

    fn panes(ids: &[&str]) -> Vec<String> {
        ids.iter().map(|id| id.to_string()).collect()
    }

    #[test]
    fn the_calling_panes_share_is_the_rest_of_the_tab() {
        assert_eq!(calling_panes_share_after(0.3), 0.7);
        assert_eq!(calling_panes_share_after(0.7), 0.3);
    }

    #[test]
    fn open_opens_when_no_pane_is_live() {
        assert_eq!(decide(Mode::Open, None, &panes(&["w:p1"])), Decision::Open);
    }

    #[test]
    fn open_focuses_the_pane_it_already_has() {
        assert_eq!(
            decide(Mode::Open, Some("w:p2"), &panes(&["w:p1", "w:p2"])),
            Decision::Focus("w:p2".to_string())
        );
    }

    #[test]
    fn toggle_closes_a_live_pane_and_opens_when_there_is_none() {
        assert_eq!(
            decide(Mode::Toggle, Some("w:p2"), &panes(&["w:p2"])),
            Decision::Close("w:p2".to_string())
        );
        assert_eq!(
            decide(Mode::Toggle, None, &panes(&["w:p1"])),
            Decision::Open
        );
    }

    #[test]
    fn a_pane_open_in_another_workspace_is_not_this_workspaces_pane_so_toggle_opens_a_second() {
        assert_eq!(
            decide(Mode::Toggle, None, &panes(&["w2:p1"])),
            Decision::Open
        );
    }

    #[test]
    fn auto_open_opens_a_closed_pane_and_leaves_an_open_one_alone() {
        assert_eq!(
            decide(Mode::AutoOpen, None, &panes(&["w:p1"])),
            Decision::Open
        );
        assert_eq!(
            decide(Mode::AutoOpen, Some("w:p2"), &panes(&["w:p2"])),
            Decision::LeaveOpen("w:p2".to_string())
        );
    }

    #[test]
    fn an_auto_open_never_takes_the_focus_and_every_other_action_does() {
        assert_eq!(focus_of(Mode::AutoOpen), Focus::Leave);
        assert_eq!(focus_of(Mode::Open), Focus::Take);
        assert_eq!(focus_of(Mode::Toggle), Focus::Take);
        assert_eq!(focus_of(Mode::Focus), Focus::Take);
    }

    #[test]
    fn auto_open_does_nothing_at_all_when_the_config_has_not_asked_for_it() {
        let outcome = auto_open(&default_config()).expect("no herdr call at all");

        assert_eq!(outcome, "auto_open is off");
    }

    #[test]
    fn a_remembered_pane_that_exited_counts_as_no_pane() {
        assert_eq!(
            decide(Mode::Toggle, Some("w:pGone"), &panes(&["w:p1"])),
            Decision::Open
        );
        assert_eq!(
            decide(Mode::Focus, Some("w:pGone"), &panes(&["w:p1"])),
            Decision::NothingToFocus
        );
    }

    #[test]
    fn focus_never_opens_a_pane() {
        assert_eq!(
            decide(Mode::Focus, None, &panes(&["w:p1"])),
            Decision::NothingToFocus
        );
    }

    #[test]
    fn a_failed_focus_or_close_retries_as_open_except_in_focus_mode() {
        assert!(matches!(
            recovery_from_a_rejected_pane(Mode::Open),
            Recovery::RetryOpen
        ));
        assert!(matches!(
            recovery_from_a_rejected_pane(Mode::Toggle),
            Recovery::RetryOpen
        ));
        assert!(matches!(
            recovery_from_a_rejected_pane(Mode::AutoOpen),
            Recovery::RetryOpen
        ));
        assert!(matches!(
            recovery_from_a_rejected_pane(Mode::Focus),
            Recovery::ReportGone
        ));
    }

    #[test]
    fn a_view_number_past_the_end_names_how_many_views_there_are() {
        let error = view("4", &default_config()).expect_err("refuses");

        assert!(error.contains("no view 4"), "{error}");
        assert!(error.contains("1 views"), "{error}");
    }

    #[test]
    fn a_view_argument_that_is_not_a_number_is_refused() {
        let error = view("today", &default_config()).expect_err("refuses");

        assert!(error.contains("not a view number"), "{error}");
    }

    fn scratch(name: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!("herdr-damnit-{name}-{}", std::process::id()));
        let _ = std::fs::remove_file(&path);
        path
    }

    #[test]
    fn a_failed_view_open_clears_its_own_request() {
        let request = scratch("failed-view");

        let error = note_then_open("today", &request, || Err("herdr is not there".to_string()))
            .expect_err("failed");

        assert_eq!(error, "herdr is not there");
        assert_eq!(
            state::take_requested_view(&request),
            None,
            "a failed open must clear its request"
        );
    }

    #[test]
    fn a_view_that_opened_leaves_its_note_for_the_pane_and_says_what_it_shows() {
        let request = scratch("opened-view");

        let outcome = note_then_open("work", &request, || Ok("opened w:p3".to_string()))
            .expect("the open succeeded");

        assert_eq!(outcome, "opened w:p3, showing work");
        assert_eq!(
            state::take_requested_view(&request),
            Some("work".to_string())
        );
    }
}
