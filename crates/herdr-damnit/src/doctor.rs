//! The `doctor` action: prove `dam` answers at the configured argv and that one
//! `dam status --json` carries every key the pane reads.

use std::time::Duration;

use herdr_damnit_adapters::{Config, ProcessDamRunner};
use herdr_damnit_application::{Handshake, check_status, check_version};
use herdr_damnit_domain::READ_DEADLINE_SECONDS;

pub fn run(config: &Config) -> Result<String, String> {
    let runner = ProcessDamRunner::new(config.dam.clone());
    let version = ask(&runner, &herdr_damnit_application::argv::version())?;
    let status = ask(&runner, &herdr_damnit_application::argv::status())?;
    report_from(&version, &status)
}

/// One `dam` call, waited out. The pane never blocks on one; a one-shot command has nothing else
/// to do while it runs, and the runner's own deadline is what bounds the wait.
fn ask(runner: &ProcessDamRunner, argv: &[String]) -> Result<String, String> {
    let job = runner
        .spawn_with_deadline(argv, Some(Duration::from_secs(READ_DEADLINE_SECONDS)))
        .map_err(|_| herdr_damnit_domain::message(&herdr_damnit_domain::Failure::NotInstalled))?;
    let finished = job
        .results
        .recv()
        .map_err(|_| "dam was killed before it answered.".to_string())?;
    Ok(finished.stdout)
}

/// The report, or the refusal the handshake would have drawn. Both are `dam`'s own words about
/// itself, so the check is the pane's own handshake rather than a second opinion about it.
fn report_from(version_output: &str, status_output: &str) -> Result<String, String> {
    let Handshake::Ready { version, warning } = check_version(version_output) else {
        let Handshake::Refuse(said) = check_version(version_output) else {
            unreachable!("check_version answers Ready or Refuse");
        };
        return Err(said);
    };
    check_status(status_output)?;
    let mut report = format!("dam:    {version}\nstatus: ok, every key the pane reads is there");
    if let Some(warning) = warning {
        report.push_str(&format!("\nnote:   {warning}"));
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    const CLEAN: &str = r#"{"staged":[],"unstaged":[],"conflicts":[],"notices":[],"unpushed":[]}"#;

    #[test]
    fn a_dam_that_answers_reports_its_version_and_that_its_status_reads() {
        let report = report_from("dam 0.2.0\n", CLEAN).expect("it answered");

        assert!(report.contains("dam:    0.2.0"), "{report}");
        assert!(report.contains("status: ok"), "{report}");
    }

    #[test]
    fn a_dam_below_the_floor_is_reported_as_the_problem_it_is() {
        let error = report_from("dam 0.0.9\n", CLEAN).expect_err("it refuses");

        assert!(
            error.contains("is older than the 0.2 this pane needs"),
            "{error}"
        );
    }

    #[test]
    fn a_status_missing_a_key_is_reported_by_name() {
        let error = report_from(
            "dam 0.2.0\n",
            r#"{"staged":[],"unstaged":[],"conflicts":[],"notices":[]}"#,
        )
        .expect_err("it refuses");

        assert!(error.contains("\"unpushed\""), "{error}");
    }

    #[test]
    fn a_newer_dam_is_reported_with_the_note_the_handshake_would_have_shown() {
        let report = report_from("dam 0.4.0\n", CLEAN).expect("it answered");

        assert!(report.contains("newer than this pane knows"), "{report}");
    }
}
