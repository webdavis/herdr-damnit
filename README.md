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

Every action works on the pane in the workspace it runs in, and the plugin remembers one pane per
workspace: `toggle` in a workspace whose pane is closed opens one there even when another workspace
already has one, and `focus` reports that there is no pane rather than jumping across workspaces.
A pane the operator closed by hand counts as no pane.

Bind them in `~/.config/herdr/config.toml` as `plugin_action` keys (the fully qualified name is
`herdr-todoist.<action>`), or invoke one directly:

```bash
herdr plugin action invoke toggle --plugin herdr-todoist
herdr plugin action invoke doctor --plugin herdr-todoist
```

`doctor` reports that the token resolved, and which indirection it came from, then that
`GET /user` succeeded. It exits non-zero with the reason when either step fails.

## Configuration

| Key             | Default   | Allowed values                      | Meaning                                                 |
| --------------- | --------- | ----------------------------------- | ------------------------------------------------------- |
| `token_command` | none      | any command                         | argv of a command whose standard output is the token    |
| `token_env`     | none      | any variable name                   | name of an environment variable holding the token       |
| `placement`     | `"split"` | `overlay`, `split`, `tab`, `zoomed` | how `open` and `toggle` place the pane                  |
| `side`          | `"right"` | `right`, `down`                     | which side of the calling pane a `split` takes          |
| `width`         | none      | a fraction above 0 and below 1      | the share of the tab the pane takes                     |
| `default_view`  | none      | a view name                         | the view the pane opens on                              |
| `auto_open`     | `false`   | `true`, `false`                     | whether focusing a workspace opens the pane there       |
| `editor`        | `nvim`    | argv, or `[]` for none              | the editor `e` enters on a task, in this pane           |
| `[[views]]`     | none      | `name` and `filter`                 | the named filter views, in the order they are written   |

An unrecognized `placement` or `side` is a config parse error naming the values above, and so is a
`width` that is not a share of the tab or a `default_view` no view answers to.

## Placement

```toml
placement = "split"
side = "down"
width = 0.3
```

`herdr` splits rightward or downward at an even ratio and takes no ratio of its own, so `side`
picks the direction the open itself splits in and a `width` is one `herdr pane resize` right
after. With no `width`, the open places the pane by itself and no resize is made. A refused resize
leaves the pane at the even split and says so: a pane at the wrong width still lists tasks.

`width` is the share of the tab the Todoist pane gets, so `0.3` is a third of it and the pane the
action ran in keeps the rest.

## Opening the pane

`default_view` is the view the pane opens on, the unfiltered list when it is unset. A `view:<n>`
action outranks it: the action notes the view it was pressed for and the pane reads that note, so
`view:3` shows view 3 whatever `default_view` says.

`auto_open = true` opens the pane in a workspace as that workspace gains focus, without taking the
focus off the pane you switched to. It is `false` by default, which keeps the pane closed until
`open`, `toggle` or a `view` action asks for it.

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
the order they are written. Two views with one name is a config error, as is a view named `all`,
which the unfiltered list already answers to, and as is a view missing its `name` or its
`filter`.

Press `v` for the picker or a number key to switch. A view keeps the cursor on its task when that
task is in the view being switched to, and lands on the view's first task when it is not.

Todoist refuses a malformed filter query with its own message, which the status line shows while
the rows already on screen stay put, so a typo in one view leaves the pane readable rather than
blank. A filter that is merely matching nothing is an empty list and says `0 open tasks`.

### Views as keybindings

herdr declares plugin actions in the manifest and has no runtime action registration, and a
`plugin_action` keybinding passes no arguments, so there can be no `view:<name>` action per
configured view: the views are config and the manifest is not. The actions are numbered instead,
`view:1` to `view:9`. Nine is `views::MAX_NUMBERED_VIEW`, the ceiling the manifest, the number
keys and the pane's hint line all share; raising it means adding entries in all three places.

```toml
[[keys.command]]
key = "prefix+ctrl+t"
type = "plugin_action"
command = "herdr-todoist.view:2"
description = "todoist: show view 2"
```

A number with no view behind it exits non-zero saying how many views the config has.

## The list

Open tasks, grouped by project and then by section, with subtasks folded under their parent one
level of indentation deeper. Each line carries the task's content, its due date, its priority as
the `p1` to `p4` the app shows, its labels, and the number of subtasks under it. A project or
section with no open task of its own is left out.

## The completed list

`<Tab>` shows the completed tasks instead of the open ones, and `<Tab>` again goes back. It is a
second screen rather than a tenth view: the numbered views are filter queries over open tasks, and
this list has no filter and no grouping. `1` to `9` and the view picker go back to the open list on
the view they name, so a `view:<n>` keybinding behaves the same whichever list is on screen.

Completed tasks are newest first, one line each, the completion date first so the dates line up
down the pane. The list is paged: the first screen is one request when the newest window has rows,
and reaching the bottom row asks for the next page. An account whose newest windows hold nothing
walks back through them before the first screen draws, since each page is a request of its own.
`u` reopens the task under the cursor, which drops its row at once, since a reopened task is no
longer completed; the cursor takes the row below it. A refused reopen leaves every row where it is
and reports the API's own message in the status line.

The API reads completed tasks in a window of at most three months at a time, so the list walks back
one window per page. It stops about three years back and the status line says so along with the day
it read back to, rather than leaving the bottom of the list looking like the end of the history. How
much of that reads at all is the account's own retention: a free plan keeps a week of completed
tasks.

## Keys in the pane

| Key           | What it does                                        |
| ------------- | --------------------------------------------------- |
| `j`, `<Down>` | move down one task                                  |
| `k`, `<Up>`   | move up one task                                    |
| `R`, `r`      | refresh the list                                    |
| `v`           | open the view picker                                |
| `1` to `9`    | show that view                                      |
| `<Tab>`       | the completed list, and back                        |
| `<CR>`        | the task's detail: its description and its comments |
| `x`           | complete the task                                   |
| `X`           | reopen the task                                     |
| `dd`          | delete the task, after the confirm                  |
| `p`           | cycle the priority one step up in urgency           |
| `s`           | set the due date from a natural-language line       |
| `l`           | toggle a label from a picker                        |
| `m`           | move the task to a project or section from a picker |
| `a`           | Quick Add a task from a whole line of its syntax    |
| `e`           | edit the task in your editor, or in the pane's box  |
| `q`, `<Esc>`  | close the pane                                      |

In the picker, `j` and `k` move, `<CR>` takes the entry under the cursor and `<Esc>` cancels. In
the completed list `j` and `k` move, `u` and `X` reopen, `R` starts the walk again from today and
`q` closes the pane. On the detail screen `j` and `k` scroll, `c` opens the comment box, `R` reads
the thread again, `<Esc>`, `<CR>` and `<Tab>` go back to the list and `q` closes the pane.

## The task detail

`<CR>` on a task opens its detail: the task's title, its description rendered as markdown, and its
comment thread oldest first, so a comment just added is at the bottom where it was typed. The
thread is ordered here rather than taken as it arrives, because the endpoint promises no ordering;
a comment carrying no timestamp sorts after every dated one instead of jumping to the top. Each
comment is headed with the day it was posted and the id of who posted it. Going back leaves the
list exactly as it was, cursor included.

The detail is a third screen rather than a tenth view, the way the completed list is: the numbered
views are the operator's own filter queries and this screen has no filter, so `view:1` to `view:9`
and `default_view` are untouched by it.

An attachment is NAMED, never fetched: the line reads `[attached] <file name>`, and nothing in the
pane downloads a file on a key press. The API sends a comment's attachment as a free-form object,
so the name is read off it when there is one and the line says `[attached] file` when there is not.

### How much markdown is rendered

A description is free text a person typed, often on a phone, and the pane it lands in is about 32
columns wide, so the renderer covers what a person actually writes and leaves the rest as written:

- **Rendered:** ATX headings (bold, one weight at every level), bullet lists (every marker drawn as
  one dash), numbered lists (keeping the numbers written), blockquotes, thematic breaks, bold,
  italics, inline code, and links, which draw their text followed by `<target>` since a pane cannot
  be clicked and the target is the half worth copying. An underscore inside a word is left alone,
  so `a_variable_name` stays as typed.
- **Shown as written:** a table, because 32 columns cannot hold one, and the body of a fenced code
  block, because reflowing code changes what it says. The fences themselves are dropped.

Every line is WRAPPED rather than cut: a long link, a wide table row and an unbreakable identifier
each break inside the pane, which is pinned by a test that draws the screen 32 columns wide.

## Adding a comment

`c` on the detail screen opens a multi-line box, drawn by the pane, so no editor is entered. `<CR>`
opens a line, `<C-d>` posts, `<Esc>` throws the draft away, and a box holding nothing but
whitespace posts nothing. `<CR>` cannot also send, which is why posting is its own key.

A posted comment arrives by a read of the thread rather than by being added to what is on screen,
so the order and the timestamp drawn are the server's own. A REFUSED post leaves the box open with
every line still in it and the API's own message in the status line: a person has just typed
several lines and losing them to a refusal would be the worst thing this screen could do.

## Editing a task in an editor

`e` on a task runs the editor in this pane, waits for it, and reads the list again once it has
gone. The command is
[todoist.nvim](https://github.com/webdavis/todoist.nvim)'s own entry point:

```bash
nvim +"Todoist task <id>"
```

which opens that one task as an editable buffer in an editor holding nothing else, so the herdr
pane and the Neovim plugin are two halves of the same workflow. Any editor works: `editor` is argv,
so the program is one entry and each argument is its own, and the `+Todoist task <id>` word is
appended to it. An entry is never split on spaces, so a path with a space in it needs no quoting.

With `editor` unset the pane runs `nvim`. `editor = []` turns the editor off and `e` opens the
pane's own multi-line box over the task instead: the first line is the task's content and the lines
under it are its description, `<CR>` opens a line, `<C-d>` saves and `<Esc>` throws the edit away.
A refused save leaves the box open with every line still in it.

The pane leaves the alternate screen before the editor starts and enters it again once the editor
has gone, on every path: an editor that exited non-zero, and an `editor` naming a program that is
not installed, both come back to a drawn pane rather than a terminal left in raw mode. A program
that cannot be started at all is named in the status line.

The list is read again whatever the editor exited with, since a person who quit in a hurry may
still have saved.

## Quick edits

Each edit is one key press, and the three that need words are one line typed in the pane, drawn
over the list in the same box the view picker uses. Only `e` enters an editor.

- `x` completes and `X` reopens the task under the cursor. `X` on a task that is already open is
  refused by the API, which says so in the status line.
- `dd` deletes. The first `d` draws a confirm naming the task; the second `d` sends the delete and
  any other key dismisses it, sending nothing at all. A deleted task takes its subtasks with it and
  there is no undo, which is why the confirm is there.
- `p` cycles the priority one step up in urgency and wraps at the top: `p4`, `p3`, `p2`, `p1`, and
  from `p1` back to `p4`. The API numbers priority the other way round from the app, 4 being the
  app's `p1`; the pane speaks the app's wording throughout.
- `s` takes a due date in Todoist's own words (`tomorrow`, `next mon`, `every 2 weeks`) and sends
  it as `due_string`, so Todoist parses it. The pane has no date parser of its own, and a line it
  cannot parse comes back as Todoist's own complaint with the line still in the box.
- `l` opens a picker of every label, marking the ones the task carries. `<CR>` toggles the one
  under the cursor and writes the task's whole label set; the picker stays open, so several labels
  go on or off without reopening it. A label the account no longer lists but the task still carries
  is offered too, so it can be taken off.
- `m` opens a picker of every project with its sections indented under it, and `<CR>` moves the
  task there.
- `a` takes a whole line of
  [Quick Add syntax](https://www.todoist.com/help/articles/use-task-quick-add-in-todoist-va4Lhpzz)
  (`Pay rent tomorrow 9am p1 #Finances @home`) and sends it as typed, so Todoist parses the date,
  the priority, the project and the labels. The status line names the task Todoist made of it.

An empty line sends nothing, and `<Esc>` leaves any prompt without a request.

Every write is followed by a read of the showing view, so the rows come from the server rather than
from a guess at what the write did: a completed or deleted task leaves the list, a moved task
appears under its new project, and a task that no longer matches the showing filter disappears. A
refused write leaves every row where it is and puts the API's own message in the status line, the
same shape a refused filter query has.

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
