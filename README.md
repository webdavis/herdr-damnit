# herdr-todoist

A Todoist pane for the [herdr](https://herdr.dev) terminal multiplexer: a
[ratatui](https://ratatui.rs) terminal user interface in a plugin-owned pane, talking to the
[Todoist API v1](https://developer.todoist.com/api/v1) directly.

This first version opens the pane, proves the connection and shows any API failure (a rejected
token, a rate limit with its retry delay, a network outage) in the status line. The task list, the
named filter views and the editing keys follow.

## Install

```bash
herdr plugin install webdavis/herdr-todoist
```

The install step builds the binary into `bin/herdr-todoist` with cargo, so a Rust toolchain is
needed. A local checkout is linked instead, and builds itself:

```bash
cargo build --release --locked && mkdir -p bin && cp target/release/herdr-todoist bin/
herdr plugin link .
```

## The token

The plugin never stores a token and never reads one from a file. Configure one of two
indirections in `config.toml` in the plugin's config directory (`herdr plugin config-dir
herdr-todoist` prints it):

```toml
# A command whose standard output is the token, for example a password manager CLI.
token_command = ["my-vault-cli", "show", "--field", "token", "Todoist API Token"]

# Or the name of an environment variable holding it.
token_env = "TODOIST_API_TOKEN"
```

`token_command` wins when both are set. Neither key has a default, so with neither set the plugin
refuses to run rather than guessing where a token lives. The token is held in a type with no
`Display` and a redacted `Debug`, and no message the plugin prints carries its value, its length
or any part of it: a failing `token_command` is reported by its program name alone.

Get a token from Todoist under Settings, Integrations, Developer.

## Actions

| Action   | What it does                                                        |
| -------- | ------------------------------------------------------------------- |
| `open`   | open the pane in this workspace, or focus it when it is already open |
| `toggle` | open the pane, or close it when it is already open                   |
| `focus`  | focus the pane in this workspace                                     |
| `doctor` | check that the token resolves and that one API request succeeds      |

Bind them in `~/.config/herdr/config.toml` as `plugin_action` keys (the fully qualified name is
`herdr-todoist.<action>`), or invoke one directly:

```bash
herdr plugin action invoke toggle --plugin herdr-todoist
herdr plugin action invoke doctor --plugin herdr-todoist
```

`doctor` reports that the token resolved, and which indirection it came from, then that
`GET /user` succeeded. It exits non-zero with the reason when either step fails.

## Configuration

| Key             | Default   | Meaning                                                      |
| --------------- | --------- | ------------------------------------------------------------ |
| `token_command` | none      | argv of a command whose standard output is the token         |
| `token_env`     | none      | name of an environment variable holding the token            |
| `placement`     | `"split"` | how `open` and `toggle` place the pane                       |
| `direction`     | `"right"` | which way a `split` placement splits                         |

## Keys in the pane

`r` refreshes, `q` closes the pane.

## Development

```bash
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test --workspace
```

The tests never reach Todoist: the client is proven against a loopback HTTP double, one canned
response per case.

## License

MIT, see [LICENSE](LICENSE).
