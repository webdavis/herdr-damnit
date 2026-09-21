//! The `doctor` action: prove the token resolves and that one request succeeds, and say nothing
//! about the token itself.

use todoist::{Client, TokenSource};

use crate::config::Config;

pub async fn run(config: &Config, base_url: &str) -> Result<String, String> {
    let source = config.token_source()?;
    let token = todoist::resolve(&source)
        .await
        .map_err(|error| error.to_string())?;
    let client = Client::new(base_url, token).map_err(|error| error.to_string())?;
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
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    use super::*;
    use crate::config::Config;

    /// A loopback double serving one canned response, so `doctor::run` is proven end to end
    /// without reaching Todoist.
    async fn serve_once(status: &str, body: &'static str) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let base_url = format!("http://{}", listener.local_addr().expect("addr"));
        let response = format!(
            "HTTP/1.1 {status}\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        );
        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.expect("accept");
            let mut buffer = [0u8; 1024];
            let _ = socket.read(&mut buffer).await;
            let _ = socket.write_all(response.as_bytes()).await;
            let _ = socket.shutdown().await;
        });
        base_url
    }

    fn config_with_token_command() -> Config {
        Config::parse(r#"token_command = ["sh", "-c", "printf test-token"]"#).expect("parses")
    }

    #[tokio::test]
    async fn a_successful_check_names_the_token_source_and_the_request() {
        let base_url = serve_once("200 OK", r#"{"id":"1","email":null,"full_name":null}"#).await;
        let report = run(&config_with_token_command(), &base_url)
            .await
            .expect("succeeds");
        assert!(report.contains("token_command `sh`"), "{report}");
        assert!(report.contains("GET /user succeeded"), "{report}");
    }

    #[tokio::test]
    async fn a_rejected_token_is_reported_as_the_client_error() {
        let base_url = serve_once("401 Unauthorized", "{}").await;
        let error = run(&config_with_token_command(), &base_url)
            .await
            .expect_err("fails");
        assert_eq!(error, "unauthorized: the token was rejected");
    }

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
