//! The `open`, `toggle`, `focus` and `auto-open` actions. Each one resolves the plugin's pane in
//! the current workspace, then drives the `herdr` CLI.

use herdr_damnit_adapters::config::Placement;
use herdr_damnit_adapters::{Config, herdr_cli as herdr, state};
use herdr_damnit_domain::Views;

use crate::placement;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Open,
    Toggle,
    Focus,
    /// A workspace gained focus: open the pane there when it is not open, and never take focus.
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

/// Whether the opened pane takes the focus. An `auto-open` follows a workspace switch, so the pane
/// the operator switched to keeps it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Focus {
    Take,
    Leave,
}

impl Focus {
    /// The flag `herdr plugin pane open` takes for it.
    fn as_flag(self) -> &'static str {
        match self {
            Self::Take => "--focus",
            Self::Leave => "--no-focus",
        }
    }
}

/// What an action does, given the pane this plugin last opened in the workspace and the panes the
/// workspace has now. A remembered pane that is no longer live counts as no pane.
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

/// What to do when a remembered pane turns out not to be this plugin's: `herdr pane list` only
/// proves a pane id is live in the workspace, not that it belongs to us, so `plugin pane
/// focus`/`close` is herdr's own registry check and the one that can reject it.
enum Recovery {
    RetryOpen,
    ReportGone,
}

fn recover(mode: Mode) -> Recovery {
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

/// Run one action and report what it did.
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
                match recover(mode) {
                    Recovery::RetryOpen => open_and_remember(&workspace, config, focus_of(mode)),
                    Recovery::ReportGone => {
                        Ok(format!("no todoist pane in {workspace}: {pane} is gone"))
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
        Decision::NothingToFocus => Ok(format!("no todoist pane in {workspace}")),
        Decision::Open => open_and_remember(&workspace, config, focus_of(mode)),
    }
}

/// The `auto-open` event hook: a workspace gained focus, so open the pane there when the operator
/// asked for that. It is off unless the config turns it on.
pub fn auto_open(config: &Config) -> Result<String, String> {
    if !config.auto_open {
        return Ok("auto_open is off".to_string());
    }
    run(Mode::AutoOpen, config)
}

/// The `view <n>` action: note which view was asked for, then make sure the pane is open, which
/// is where that note is read.
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

/// Note the view the pane is to show, then open the pane. A failed open takes the note back, so a
/// later unrelated `open` or `toggle` does not jump to a view nobody asked for.
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
    let note = arrange_pane(&neighbor, config);
    state::remember_pane(workspace, &pane);
    Ok(match note {
        Some(note) => format!("opened {pane}, {note}"),
        None => format!("opened {pane}"),
    })
}

/// Give the pane its configured width, which herdr's own open cannot do: it splits at an even
/// ratio and takes no ratio of its own. This resizes the calling pane rather than the one just
/// opened, since a same-tab `herdr pane move` is a no-op and a resize is the only call that
/// actually changes the split. A refused resize leaves the pane at the even split and says so,
/// because a pane at the wrong width still lists tasks.
fn arrange_pane(neighbor: &str, config: &Config) -> Option<String> {
    if config.placement != Placement::Split || neighbor.is_empty() {
        return None;
    }
    let width = config.width?;
    let target_ratio = placement::leading_share(width);
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
    fn a_pane_open_in_another_workspace_is_not_this_workspaces_pane() {
        // The remembered pane is read per workspace, so a pane open elsewhere is simply absent
        // here: toggle opens a second one rather than closing the far one or jumping to it.
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
        assert!(matches!(recover(Mode::Open), Recovery::RetryOpen));
        assert!(matches!(recover(Mode::Toggle), Recovery::RetryOpen));
        assert!(matches!(recover(Mode::AutoOpen), Recovery::RetryOpen));
        assert!(matches!(recover(Mode::Focus), Recovery::ReportGone));
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
