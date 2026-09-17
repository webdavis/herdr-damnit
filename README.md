# herdr-todoist

A Todoist pane for the [herdr](https://herdr.dev) terminal multiplexer: a
[ratatui](https://ratatui.rs) terminal user interface in a plugin-owned pane, talking to the
[Todoist API v1](https://developer.todoist.com/api/v1) directly.

The pane lists every open task grouped by project and section, and shows any API failure (a
rejected token, a rate limit with its retry delay, a network outage) in the status line while the
last good list stays on screen. Named filter views, written in Todoist's own filter language,
switch with a picker or a number key. The editing keys follow.

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
| `view:1` to `view:9` | show that view, opening the pane when it is closed       |
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

| Key             | Default   | Allowed values                            | Meaning                                               |
| --------------- | --------- | ------------------------------------------ | ------------------------------------------------------ |
| `token_command` | none      | any command                               | argv of a command whose standard output is the token  |
| `token_env`     | none      | any variable name                         | name of an environment variable holding the token     |
| `placement`     | `"split"` | `overlay`, `split`, `tab`, `zoomed`       | how `open` and `toggle` place the pane                |
| `direction`     | `"right"` | `right`, `down`                           | which way a `split` placement splits                  |
| `[[views]]`     | none      | `name` and `filter`                       | the named filter views, in the order they are written |

An unrecognized `placement` or `direction` is a config parse error naming the values above.

## Views

A view is a name and a Todoist filter query, the same
[filter language](https://www.todoist.com/help/articles/introduction-to-filters-V98wIH) the app's
Filters feature uses, so anything Todoist accepts as a filter is a view:

```toml
[[views]]
name = "today"
filter = "today | overdue"

[[views]]
name = "work"
filter = "#Work & !@waiting"
```

View 1 is always `all`, the unfiltered list of every open task, and the configured views follow in
the order they are written. Two views with one name is a config error, as is a view missing its
`name` or its `filter`.

Press `v` for the picker or a number key to switch. A view keeps the cursor on its task when that
task is in the view being switched to, and lands on the view's first task when it is not.

Todoist refuses a malformed filter query with its own message, which the status line shows while
the rows already on screen stay put, so a typo in one view leaves the pane readable rather than
blank. A filter that is merely matching nothing is an empty list and says `0 open tasks`.

### Views as keybindings

herdr declares plugin actions in the manifest and has no runtime action registration, and a
`plugin_action` keybinding passes no arguments, so there can be no `view:<name>` action per
configured view: the views are config and the manifest is not. The actions are numbered instead,
`view:1` to `view:9`, each showing the view at that number:

```toml
[[keybindings]]
key = "prefix+ctrl+t"
type = "plugin_action"
action = "herdr-todoist.view:2"
```

A number with no view behind it exits non-zero saying how many views the config has.

## The list

Open tasks, grouped by project and then by section, with subtasks folded under their parent one
level of indentation deeper. Each line carries the task's content, its due date, its priority as
the `p1` to `p4` the app shows, its labels, and the number of subtasks under it. A project or
section with no open task of its own is left out.

## Keys in the pane

| Key             | What it does                  |
| --------------- | ----------------------------- |
| `j`, `<Down>`   | move down one task            |
| `k`, `<Up>`     | move up one task              |
| `R`, `r`        | refresh the list              |
| `v`             | open the view picker          |
| `1` to `9`      | show that view                |
| `q`, `<Esc>`    | close the pane                |

In the picker, `j` and `k` move, `<CR>` shows the view under the cursor and `<Esc>` cancels.

A refresh keeps the cursor on the same task rather than on the same row, so a task added, removed
or reordered above it does not move the highlight. When the task under the cursor is gone, the
cursor takes the next task below it, or the one above when it was the last.

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
