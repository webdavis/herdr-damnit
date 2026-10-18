# herdr-damnit

A task pane for the [herdr](https://herdr.dev) terminal multiplexer over
[`dam`](https://github.com/webdavis/damnit), the local task store.

The pane lists the objects of a view grouped by path, shows `dam`'s staging model on a second
screen and the completed tasks on a third, and stages, commits, pushes and pulls the way `dam` does
from a shell.

## Install

```bash
herdr plugin install webdavis/herdr-damnit
```

It needs herdr 0.7.5 or later, a Rust toolchain for the build, and `dam` 0.2 or later on the
`PATH` (`cargo install damnit`). When `dam` is missing or too old, the pane says so in one line and
`R` retries.

A local checkout is built and linked:

```bash
cargo build --release --locked -p herdr-damnit
mkdir -p bin && cp target/release/herdr-damnit bin/
herdr plugin link .
```

## Actions

| Action               | What it does                                                          |
| -------------------- | --------------------------------------------------------------------- |
| `open`               | open the pane in this workspace, or focus it when it is already open  |
| `toggle`             | open the pane, or close it when it is already open                    |
| `focus`              | focus the pane in this workspace                                      |
| `status`             | open or focus the pane on its Status screen                           |
| `view:1` to `view:9` | show that view, opening the pane when it is closed                    |
| `doctor`             | check that `dam` answers, that its status reads, and list its remotes |

Each action works on the pane of the workspace it runs in, one pane per workspace. `focus` with no
pane in this workspace says so. A pane closed by hand counts as no pane.

Bind them as `plugin_action` keys named `herdr-damnit.<action>`, or invoke one directly:

```bash
herdr plugin action invoke toggle --plugin herdr-damnit
```

```toml
[[keys.command]]
key = "prefix+ctrl+t"
type = "plugin_action"
command = "herdr-damnit.view:2"
description = "damnit: show view 2"
```

A `view:<n>` with no view behind it exits non-zero saying how many views the config has.

`doctor` prints the `dam` version, confirms `dam status --json` carries every key the pane reads,
and lists `dam remote list`. It exits non-zero with the reason when `dam` is too old or its status
lacks a key, and names the program and the `PATH` it searched when `dam` cannot be started.

## Configuration

`config.toml` in the directory `herdr plugin config-dir herdr-damnit` prints. A missing file is
the default configuration.

| Key               | Default              | Meaning                                         |
| ----------------- | -------------------- | ----------------------------------------------- |
| `dam`             | `["dam"]`            | the argv run as `dam`; its first word not blank |
| `placement`       | `"split"`            | `overlay`, `split`, `tab` or `zoomed`           |
| `side`            | `"right"`            | `right` or `down`: the side a `split` takes     |
| `width`           | none                 | the pane's share of a `split`, between 0 and 1  |
| `default_view`    | none                 | the view the pane opens on, else `open`         |
| `auto_open`       | `false`              | open the pane in a workspace as it gains focus  |
| `theme`           | `"catppuccin-mocha"` | the palette; `catppuccin-mocha` is the only one |
| `icons`           | `"nerd-font"`        | `nerd-font` or `ascii`                          |
| `refresh_seconds` | `300`                | seconds between reads of `dam`; `0` for never   |
| `handoff_label`   | `"handed-off"`       | the label `S` writes; `""` for none             |
| `[[views]]`       | none                 | `name` and `query` per view                     |

A value outside the ones listed, a `default_view` no view answers to, and a key the pane does not
read are each a config error.

With `placement = "split"` and `width = 0.3`, the pane takes 30% of the split and the pane the
action ran in keeps 70%. With no `width` the split is even. A refused resize leaves the even split
and says so.

`auto_open` opens the pane without taking focus from the pane you switched to.

## Views

A view is a name and a `dam` query passed to `dam ls` as it is:

```toml
[[views]]
name = "today"
query = "!done & (due:today | overdue)"
```

View 1 is always `open`, the query `!done`. The configured views follow in the order they are
written. Two views with one name, a view named `open`, and a view missing its `name` or its `query`
are config errors.

`v` opens the view picker and `1` to `9` switch. A `view:<n>` action outranks `default_view`. A
query `dam` refuses leaves the rows on screen and puts `dam`'s message on the bottom line.

## Screens

`<Tab>` cycles List, Status and Done, and `<S-Tab>` goes back. Each keeps its own cursor.

- **List** is the showing view, grouped by path with children indented.
- **Status** is `dam status`: Staged, Working, Unpushed and Notices. An empty section is left out,
  and an empty screen says `nothing staged, nothing changed`.
- **Done** is `dam ls done`, newest completion first with the date from `dam log` leading. A
  completion not yet committed sorts to the top under `not committed`.

The header names the screen or the view, with the counts at the right. While a `dam` command is out
it carries a spinner, the verb and the elapsed time: `dam  today  ⠙ push 3.2s`. The bottom line
shows a few keys for the screen, or the latest message until the next key.

The pane reads `dam` again every `refresh_seconds`, after every write, and on `R`. The interval
waits while a picker, a box or the Detail is open, or while Status or Done is showing.

## Marks

A List row is its marks, the subject, then the label count. A subject too long for the pane is cut
with an ellipsis.

| Mark       | Plain | Color  | Meaning                                    |
| ---------- | ----- | ------ | ------------------------------------------ |
| priority 1 | `!`   | red    | `dam`'s priority 1, the most urgent        |
| priority 2 | `^`   | orange | `dam`'s priority 2                         |
| priority 3 | `-`   | blue   | `dam`'s priority 3                         |
| overdue    | `<`   | red    | due before today, with the date beside it  |
| today      | `*`   | yellow | due today                                  |
| upcoming   | `>`   | blue   | due later, with the date beside it         |
| recurring  | `~`   | green  | the object carries a recurrence rule       |
| labels     | `@`   | purple | how many labels the object carries         |
| working    | `*`   | yellow | changed here, not staged                   |
| staged     | `+`   | green  | staged, not committed                      |
| unpushed   | `^`   | cyan   | committed here, not yet on the remote      |
| conflict   | `x`   | red    | both sides changed it                      |
| notice     | `!`   | orange | a notice on the Status screen              |

Priority 4 and no due date carry no mark. `icons = "nerd-font"` draws Nerd Font glyphs in place of
the plain marks; set `icons = "ascii"` when they come out as boxes. Every color is Catppuccin Mocha.

## Keys

A key acts on the object under the cursor. `c` with nothing staged, `1` to `9` with no such view
and `S` with no agent pane each say so on the bottom line.

| Key             | What it does                                                         |
| --------------- | -------------------------------------------------------------------- |
| `j`, `<Down>`   | move down one row                                                    |
| `k`, `<Up>`     | move up one row                                                      |
| `<Tab>`         | the next screen; `<S-Tab>` the one before                            |
| `v`             | the view picker, on List or Done                                     |
| `1` to `9`      | the List on that view                                                |
| `R`, `r`        | read `dam` again                                                     |
| `<CR>`          | the object's detail                                                  |
| `<Space>`       | stage the object, or unstage it when it is staged                    |
| `A`, `U`        | stage everything, unstage everything                                 |
| `c`             | commit what is staged, with a one-line message                       |
| `P`, `L`        | push, pull                                                           |
| `<C-c>`         | cancel the `dam` command the header names                            |
| `x`             | complete the task                                                    |
| `X`             | complete the task past open children and dependencies                |
| `dd`            | remove the object, after the confirm                                 |
| `!`             | discard the object's working change, after the confirm               |
| `p`             | cycle the task's priority 4, 3, 2, 1 and back to 4                   |
| `s`, `D`        | set the task's due date, deadline                                    |
| `l`             | add or remove labels from a picker                                   |
| `m`             | move the object to another path from a picker                        |
| `a`             | add a task under the object's path, or with no path                  |
| `S`             | hand the object to this workspace's agent pane                       |
| `e`             | edit the object in your editor                                       |
| `o`, `t`        | resolve the conflict toward ours, toward theirs                      |
| `q`             | close the pane                                                       |
| `<Esc>`         | close the picker, box or detail that is open; else the pane          |

In a picker, `j` and `k` move, `<CR>` takes the entry and `<Esc>` cancels. In a one-line box,
`<CR>` sends and `<Esc>` cancels. A confirm takes its own key again; any other key cancels.

`q` while a push, pull or commit runs asks once; a second `q` closes the pane and leaves `dam` to
finish. `<C-c>` stops it.

- A blank commit message or new subject is refused in the pane.
- The `l` picker offers every label in the showing view with the object's own marked, and stays
  open so several labels go on or off at once.
- The `m` picker offers every path in the showing view and its parents.
- `a` names the object `dam` made: `made 7a8b9c0`.
- A box whose command `dam` refuses comes back with its line in it and `dam`'s message below. A
  store held by another `dam` closes the box and asks for `R`.

A read keeps the cursor on the same object. When that object is gone, the cursor takes the next one
below, or the one above when it was the last.

## The detail

`<CR>` draws the object from `dam show`: the subject, its path, kind, priority, due date, deadline,
attached event, recurrence, labels, dependencies with their subjects, and its body as markdown. An
event adds its start, end, time zone, location, status, transparency and attendees. A field with
nothing in it is left out. The detail does not scroll. `<Esc>` goes back.

## Sending a task to the agent

`S` opens a note box: `<CR>` starts a new line, `<C-d>` sends and `<Esc>` drops the draft. The
send pastes the task and the note into the workspace's agent pane and focuses it, then writes
`handoff_label` on the task as a working change. A refused label says
`sent to <name>, label refused: <dam's message>.`

## Development

```bash
just gates
```

runs `cargo fmt --all --check`, `cargo clippy --locked --workspace --all-targets -- -D warnings`,
`cargo doc` with warnings denied and `cargo test --locked --workspace`, which is what CI runs.

The workspace is four crates whose dependencies point inward only:

- `herdr-damnit-domain`: objects, rows, marks, palette slots, the staging model, views, the
  cursor, dates, the brief, the version rules and the failure mapping.
- `herdr-damnit-application`: the ports, every `dam` argv, the job table, the handshake and the
  hand-off, with fakes of every port behind the `fakes` feature.
- `herdr-damnit-adapters`: the `dam` process runner, the herdr CLI, the config, the state
  directory, the clock and the parsing of `dam`'s JSON.
- `herdr-damnit`: the binary, with the pane actions, `doctor`, the draw loop, the screens, the
  keys, the legend and the theme.

## License

MIT, see [LICENSE](LICENSE).
