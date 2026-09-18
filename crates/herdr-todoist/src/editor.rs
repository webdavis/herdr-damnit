//! Entering an editor on one task and coming back to the list.
//!
//! The editor runs in the pane's own terminal as a child of this process, so the pane leaves the
//! alternate screen before it starts and enters it again once it has gone. That restore is done
//! by the caller around [`run`], which is what keeps it on the path a failed spawn takes too.
//!
//! The command is [`todoist.nvim`](https://github.com/webdavis/todoist.nvim)'s own entry point:
//! `nvim +"Todoist task <id>"` opens that one task in an editor holding nothing else.

use crate::config::Config;
use crate::reload::Screen;

/// The editor the pane enters when the config names none.
pub const DEFAULT: &str = "nvim";

/// The argv `e` runs for a task, or `None` when the config has turned the editor off and the
/// pane's own box is what opens instead.
pub fn argv(config: &Config, id: &str) -> Option<Vec<String>> {
    let mut argv = match &config.editor {
        Some(editor) if editor.is_empty() => return None,
        Some(editor) => editor.clone(),
        None => vec![DEFAULT.to_string()],
    };
    argv.push(entry_command(id));
    Some(argv)
}

/// The Neovim command the editor is entered on, as one argument: a `+<command>` word carries its
/// spaces itself, since nothing here goes through a shell.
fn entry_command(id: &str) -> String {
    format!("+Todoist task {id}")
}

/// Run the editor and wait for it. The exit code is not read: a person who quit with an error
/// still may have saved, and the refresh afterwards is what says what the task looks like now.
/// A command that cannot be started at all is the one failure the pane reports.
pub fn run(argv: &[String]) -> Result<(), String> {
    let Some((program, arguments)) = argv.split_first() else {
        return Err("no editor command to run".to_string());
    };
    std::process::Command::new(program)
        .args(arguments)
        .status()
        .map(|_| ())
        .map_err(|error| format!("{program}: {error}"))
}

/// Read the list again now the editor has gone, whatever it left behind, and report the status
/// line: the refusal when the editor could not be started, the list's own count otherwise.
pub async fn after(outcome: Result<(), String>, screen: &mut Screen<'_>) -> String {
    let refreshed = screen.refresh().await;
    match outcome {
        Ok(()) => refreshed,
        Err(error) => error,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::Connection;
    use crate::cursor::List;
    use crate::edit::tests::{Double, serve};
    use crate::views::Views;

    #[test]
    fn an_unset_editor_enters_neovim_on_the_task() {
        let config = Config::default();

        assert_eq!(
            argv(&config, "6X").expect("an editor"),
            vec!["nvim".to_string(), "+Todoist task 6X".to_string()]
        );
    }

    #[test]
    fn a_configured_editor_keeps_its_own_arguments_before_the_task() {
        let config = Config::parse(r#"editor = ["/Applications/My Editor/nvim", "--clean"]"#)
            .expect("parses");

        assert_eq!(
            argv(&config, "6X").expect("an editor"),
            vec![
                "/Applications/My Editor/nvim".to_string(),
                "--clean".to_string(),
                "+Todoist task 6X".to_string()
            ]
        );
    }

    #[test]
    fn an_editor_turned_off_leaves_the_pane_its_own_box() {
        let config = Config::parse("editor = []").expect("parses");

        assert_eq!(argv(&config, "6X"), None);
    }

    #[test]
    fn a_child_that_exits_non_zero_is_not_a_failure_the_pane_reports() {
        let argv = ["sh".to_string(), "-c".to_string(), "exit 3".to_string()];

        assert_eq!(run(&argv), Ok(()));
    }

    #[test]
    fn an_editor_that_cannot_be_started_is_reported_by_its_name() {
        let argv = ["herdr-todoist-no-such-editor".to_string()];

        let error = run(&argv).expect_err("refuses");

        assert!(
            error.starts_with("herdr-todoist-no-such-editor: "),
            "{error}"
        );
    }

    /// The one assertion that pins the refresh: both the editor that ran and the editor that
    /// could not be started re-read the list, since either way the task may have changed.
    #[tokio::test]
    async fn the_list_is_read_again_however_the_editor_ended() {
        for (outcome, expected) in [
            (Ok(()), "0 open tasks".to_string()),
            (
                Err("nvim: not found".to_string()),
                "nvim: not found".to_string(),
            ),
        ] {
            let double = serve("200 OK", "null").await;
            let status = refreshing(&double, outcome).await;

            assert_eq!(status, expected);
            assert!(
                double.requests().iter().any(|line| line.starts_with("GET")),
                "the list was not read again: {:?}",
                double.requests()
            );
        }
    }

    async fn refreshing(double: &Double, outcome: Result<(), String>) -> String {
        let config = crate::reload::tests::config_with_token_command("printf test-token");
        let mut connection = Connection::build(&config, &double.base_url).await;
        let mut list = List::new(Vec::new());
        let mut views = Views::new(&[]);
        let mut screen = Screen {
            connection: &mut connection,
            config: &config,
            base_url: &double.base_url,
            list: &mut list,
            views: &mut views,
        };

        after(outcome, &mut screen).await
    }
}
