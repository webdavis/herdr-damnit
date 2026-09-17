//! The `doctor` action: prove the token resolves and that one request succeeds, and say nothing
//! about the token itself.

use todoist::{Client, DEFAULT_BASE_URL, TokenSource};

use crate::config::Config;

pub async fn run(config: &Config) -> Result<String, String> {
    let source = config.token_source()?;
    let token = todoist::resolve(&source)
        .await
        .map_err(|error| error.to_string())?;
    let client = Client::new(DEFAULT_BASE_URL, token).map_err(|error| error.to_string())?;
    client.user().await.map_err(|error| error.to_string())?;
    Ok(format!(
        "token: resolved from {}\napi:   GET /user succeeded",
        describe(&source)
    ))
}

/// How the token was configured, never what it is. A command is named by its program alone, since
/// its arguments are the user's to write.
fn describe(source: &TokenSource) -> String {
    match source {
        TokenSource::Command(argv) => format!(
            "token_command `{}`",
            argv.first().map_or("<empty>", String::as_str)
        ),
        TokenSource::Env(name) => format!("token_env {name}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_command_source_is_named_by_its_program_only() {
        let source = TokenSource::Command(vec![
            "keepassxc-cli".to_string(),
            "a-passphrase-nobody-should-see".to_string(),
        ]);
        let described = describe(&source);
        assert_eq!(described, "token_command `keepassxc-cli`");
        assert!(!described.contains("passphrase"), "{described}");
    }

    #[test]
    fn an_environment_source_is_named_by_its_variable() {
        let described = describe(&TokenSource::Env("TODOIST_API_TOKEN".to_string()));
        assert_eq!(described, "token_env TODOIST_API_TOKEN");
    }
}
