use std::process::Stdio;

use tokio::process::Command;

/// A resolved API token. No `Display`, and `Debug` prints a placeholder, so neither a log line
/// nor a panic message can leak it.
#[derive(Clone)]
pub struct Token(String);

impl Token {
    /// The value of the `Authorization` header this token belongs in.
    pub fn header_value(&self) -> String {
        format!("Bearer {}", self.0)
    }
}

impl std::fmt::Debug for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Token(<redacted>)")
    }
}

/// Where the token comes from. Both variants are indirections the user configures: a command to
/// run, or the name of an environment variable. A token value in a configuration file, and a
/// default that reads a well-known path, are deliberately not expressible.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TokenSource {
    /// Argv of a command whose standard output is the token, for example a vault CLI call.
    Command(Vec<String>),
    /// The name of an environment variable holding the token.
    Env(String),
}

#[derive(Debug, thiserror::Error)]
pub enum TokenError {
    #[error("token_command is empty")]
    EmptyCommand,
    #[error("token_command could not be run: {0}")]
    Spawn(String),
    // Only the program name is reported. The command's output is the token on success and may
    // carry it on failure, and its arguments are the user's to write as they like.
    #[error("token_command `{program}` exited with status {status}")]
    CommandFailed { program: String, status: String },
    #[error("token_command `{0}` produced no output")]
    CommandEmpty(String),
    #[error("environment variable {0} is not set")]
    MissingEnv(String),
    #[error("environment variable {0} is empty")]
    EmptyEnv(String),
}

/// Resolve a token, reading environment variables from the process.
pub async fn resolve(source: &TokenSource) -> Result<Token, TokenError> {
    resolve_with(source, |name| std::env::var(name).ok()).await
}

/// Resolve a token, reading environment variables through `env` so tests need no process state.
pub async fn resolve_with(
    source: &TokenSource,
    env: impl Fn(&str) -> Option<String>,
) -> Result<Token, TokenError> {
    match source {
        TokenSource::Env(name) => match env(name) {
            None => Err(TokenError::MissingEnv(name.clone())),
            Some(value) if value.trim().is_empty() => Err(TokenError::EmptyEnv(name.clone())),
            Some(value) => Ok(Token(value.trim().to_string())),
        },
        TokenSource::Command(argv) => run_token_command(argv).await,
    }
}

async fn run_token_command(argv: &[String]) -> Result<Token, TokenError> {
    let (program, args) = argv.split_first().ok_or(TokenError::EmptyCommand)?;
    let output = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .output()
        .await
        .map_err(|error| TokenError::Spawn(error.to_string()))?;
    if !output.status.success() {
        return Err(TokenError::CommandFailed {
            program: program.clone(),
            status: output
                .status
                .code()
                .map_or_else(|| "signal".to_string(), |code| code.to_string()),
        });
    }
    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if value.is_empty() {
        return Err(TokenError::CommandEmpty(program.clone()));
    }
    Ok(Token(value))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn no_env(_: &str) -> Option<String> {
        None
    }

    #[tokio::test]
    async fn env_source_resolves_and_trims() {
        let source = TokenSource::Env("TOKEN_FOR_TEST".to_string());
        let token = resolve_with(&source, |_| Some(" secret \n".to_string()))
            .await
            .expect("resolves");
        assert_eq!(token.header_value(), "Bearer secret");
    }

    #[tokio::test]
    async fn env_source_reports_a_missing_variable_by_name() {
        let source = TokenSource::Env("TOKEN_FOR_TEST".to_string());
        let error = resolve_with(&source, no_env).await.expect_err("fails");
        assert_eq!(
            error.to_string(),
            "environment variable TOKEN_FOR_TEST is not set"
        );
    }

    #[tokio::test]
    async fn env_source_rejects_a_blank_variable() {
        let source = TokenSource::Env("TOKEN_FOR_TEST".to_string());
        let error = resolve_with(&source, |_| Some("  ".to_string()))
            .await
            .expect_err("fails");
        assert_eq!(
            error.to_string(),
            "environment variable TOKEN_FOR_TEST is empty"
        );
    }

    #[tokio::test]
    async fn command_source_reads_standard_output() {
        let source = TokenSource::Command(vec![
            "sh".to_string(),
            "-c".to_string(),
            "printf 'from-vault\n'".to_string(),
        ]);
        let token = resolve_with(&source, no_env).await.expect("resolves");
        assert_eq!(token.header_value(), "Bearer from-vault");
    }

    #[tokio::test]
    async fn command_source_reports_a_failing_command_without_its_output() {
        let source = TokenSource::Command(vec![
            "sh".to_string(),
            "-c".to_string(),
            "printf leaked; exit 3".to_string(),
        ]);
        let error = resolve_with(&source, no_env).await.expect_err("fails");
        let message = error.to_string();
        assert!(message.contains("exited with status 3"), "{message}");
        assert!(!message.contains("leaked"), "{message}");
    }

    #[tokio::test]
    async fn command_source_rejects_empty_output() {
        let source = TokenSource::Command(vec!["true".to_string()]);
        let error = resolve_with(&source, no_env).await.expect_err("fails");
        assert!(error.to_string().contains("produced no output"));
    }

    #[tokio::test]
    async fn empty_command_is_refused() {
        let error = resolve_with(&TokenSource::Command(vec![]), no_env)
            .await
            .expect_err("fails");
        assert_eq!(error.to_string(), "token_command is empty");
    }

    #[test]
    fn debug_redacts_the_token() {
        assert_eq!(
            format!("{:?}", Token("secret".to_string())),
            "Token(<redacted>)"
        );
    }
}
