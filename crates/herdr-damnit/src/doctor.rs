use std::time::Duration;

use herdr_damnit_adapters::{Config, ProcessDamRunner};
use herdr_damnit_application::{Finished, Handshake, argv, check_status, check_version};
use herdr_damnit_domain::{Failure, READ_DEADLINE_SECONDS, message};

pub fn run(config: &Config) -> Result<String, String> {
    let runner = ProcessDamRunner::new(config.dam.clone());
    let Some(version) = ask(&runner, &argv::version()) else {
        let path = std::env::var("PATH").unwrap_or_default();
        return Err(missing_dam_report(&config.dam, &path));
    };
    let status = ask(&runner, &argv::status())
        .map(|done| done.stdout)
        .unwrap_or_default();
    let remotes = ask(&runner, &argv::remotes()).map(|done| match done.code {
        Some(0) => Ok(done.stdout),
        _ => Err(done.stderr.lines().next().unwrap_or_default().to_string()),
    });
    report_from(
        &version.stdout,
        &status,
        remotes.unwrap_or_else(|| Err(String::new())),
    )
}

fn ask(runner: &ProcessDamRunner, argv: &[String]) -> Option<Finished> {
    let job = runner
        .spawn_with_deadline(argv, Some(Duration::from_secs(READ_DEADLINE_SECONDS)))
        .ok()?;
    job.results.recv().ok()
}

fn missing_dam_report(dam: &[String], path: &str) -> String {
    format!(
        "{}\nsearched for: {}\nPATH: {path}",
        message(&Failure::NotInstalled),
        dam.first().map(String::as_str).unwrap_or_default()
    )
}

fn report_from(
    version_output: &str,
    status_output: &str,
    remotes: Result<String, String>,
) -> Result<String, String> {
    let (version, warning) = match check_version(version_output) {
        Handshake::Ready { version, warning } => (version, warning),
        Handshake::Refuse(said) => return Err(said),
    };
    check_status(status_output)?;
    let mut report = format!("dam:     {version}\nstatus:  ok, every key the pane reads is there");
    if let Some(warning) = warning {
        report.push_str(&format!("\nnote:    {warning}"));
    }
    let remotes = match remotes {
        Ok(listed) if listed.trim().is_empty() => "none configured".to_string(),
        Ok(listed) => listed.trim().replace('\n', "\n         "),
        Err(why) => format!("not available: {why}"),
    };
    report.push_str(&format!("\nremotes: {remotes}"));
    Ok(report)
}

#[cfg(test)]
mod tests;
