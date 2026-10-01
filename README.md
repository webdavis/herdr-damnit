# herdr-damnit

A task pane for the [herdr](https://herdr.dev) terminal multiplexer over
[`dam`](https://github.com/webdavis/damnit), the local task store: a [ratatui](https://ratatui.rs)
terminal user interface in a plugin-owned pane. Every read and every write is a `dam` command run
on a worker thread, so the pane keeps drawing and reading keys while a push or a pull is out.

The pane lists the objects of a view grouped by path, shows `dam`'s staging model on a second
screen and the completed tasks on a third, and stages, commits, pushes and pulls the way `dam` does
from a shell. Named views are `dam` queries and switch with a picker or a number key.

## Install

```bash
herdr plugin install webdavis/herdr-damnit
```

It needs herdr 0.7.5 or later: plugin panes and their registry arrived in 0.7.0, and `S` reads the
`pane_id`, `workspace_id` and `agent` fields of `herdr agent list`, which herdr documents from 0.7.5.

It needs `dam` 0.2 or later on the `PATH`, installed with `cargo install damnit`. The pane checks
the version as it opens and draws a refusal naming the version it found when it is too old.

The install step builds the binary into `bin/herdr-damnit` with cargo, so a Rust toolchain is
needed. A local checkout is linked instead, and builds itself:

```bash
cargo build --release --locked && mkdir -p bin && cp target/release/herdr-damnit bin/
herdr plugin link .
```

The plugin holds no credential and its config has no key that names one. A remote's token is
`dam`'s, resolved from that remote's own section of `dam`'s config.

## Actions

| Action               | What it does                                                          |
| -------------------- | --------------------------------------------------------------------- |
| `open`               | open the pane in this workspace, or focus it when it is already open  |
| `toggle`             | open the pane, or close it when it is already open                    |
| `focus`              | focus the pane in this workspace                                      |
| `status`             | open or focus the pane on its Status screen                           |
| `view:1` to `view:9` | show that view, opening the pane when it is closed                    |
| `doctor`             | check that `dam` answers, that its status reads, and list its remotes |

Every action works on the pane in the workspace it runs in, and the plugin remembers one pane per
workspace: `toggle` in a workspace whose pane is closed opens one there even when another workspace
already has one, and `focus` reports that there is no pane rather than jumping across workspaces.
A pane the operator closed by hand counts as no pane.

Bind them in `~/.config/herdr/config.toml` as `plugin_action` keys (the fully qualified name is
`herdr-damnit.<action>`), or invoke one directly:

```bash
herdr plugin action invoke toggle --plugin herdr-damnit
herdr plugin action invoke doctor --plugin herdr-damnit
```

`doctor` prints the `dam` version it found, confirms that `dam status --json` carries every key the
pane reads, and lists the remotes `dam remote list` names, each in `dam`'s own words, which include
when it was last pulled and pushed. It exits non-zero with the reason when `dam` is too old or its
status is missing a key, and when `dam` cannot be started it names the program it looked for and
the `PATH` it searched.

## Configuration

The config is `config.toml` in the plugin's config directory (`herdr plugin config-dir
herdr-damnit` prints it). A missing file is the default configuration.

| Key               | Default        | Allowed values                        | Meaning                                                                         |
| ----------------- | -------------- | ------------------------------------- | ------------------------------------------------------------------------------- |
| `dam`             | `["dam"]`      | an argv whose first word is not blank | the command the pane runs as `dam`, each call's own arguments following it      |
| `placement`       | `"split"`      | `overlay`, `split`, `tab`, `zoomed`   | how `open` and `toggle` place the pane                                          |
| `side`            | `"right"`      | `right`, `down`                       | which side of the calling pane a `split` takes                                  |
| `width`           | none           | a fraction above 0 and below 1        | the share of the tab the pane takes                                             |
| `default_view`    | none           | a view name                           | the view the pane opens on                                                      |
| `auto_open`       | `false`        | `true`, `false`                       | whether focusing a workspace opens the pane there                               |
| `theme`           | `"catppuccin"` | a herdr theme name                    | the palette the pane paints with                                                |
| `icons`           | `"nerd-font"`  | `nerd-font`, `ascii`                  | which set of marks a row carries                                                |
| `refresh_seconds` | `300`          | whole seconds, `0` to turn it off     | how often the pane reads `dam` on its own                                       |
| `handoff_label`   | `"handed-off"` | any label, `""` for none              | the label a successful hand-off to the agent writes on the object               |
| `[[views]]`       | none           | `name` and `query`                    | the named views, numbered after the open list in the order they are written     |

An unrecognized `placement` or `side` is a config parse error naming the values above, and so is a
`width` that is not a share of the tab, a `default_view` no view answers to, or a `theme` name no
palette answers to. A key the pane does not read is an error too, so a leftover `token_command`,
`filter` or `editor` from `herdr-todoist` is named rather than ignored.

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

`width` is the share of the tab the pane gets, so `0.3` is a third of it and the pane the action
ran in keeps the rest.

## Opening the pane

`default_view` is the view the pane opens on, the open list when it is unset. A `view:<n>` action
outranks it: the action notes the view it was pressed for and the running pane reads that note on
its next tick, so `view:3` shows view 3 whatever `default_view` says. The `status` action leaves
the note `status`, which the pane reads as the Status screen rather than as a view name. A view
that is itself named `status` is still reached by its number and from the picker.

`auto_open = true` opens the pane in a workspace as that workspace gains focus, without taking the
focus off the pane you switched to. It is `false` by default, which keeps the pane closed until
`open`, `toggle`, `status` or a `view` action asks for it.

## Reading, refreshing and writes made offline

The store is `dam`'s. The pane keeps no cache and no queue of its own: as it opens it asks
`dam --version`, then `dam status --json`, then `dam ls <query> --json` for the showing view, and
every later read is the same two local commands.

The pane reads again every `refresh_seconds`, after every write, and on `R`. Setting
`refresh_seconds` to `0` turns off only that interval. The interval is held back while a picker or
a box is open, or a screen other than the List is showing, so a half-typed line is never redrawn
away.

A write made with the network down is an ordinary `dam` commit: it waits as an unpushed commit,
counted on the status line as `^1 unpushed`, until a push reaches the remote. A failed mutation in
a push stays in the Notices section of the Status screen with `dam`'s reason until it is dealt with.

Every message the pane has for the operator, `dam`'s own refusals included, takes the bottom line
in place of the key hints until the next key is pressed.

## Views

A view is a name and a `dam` query, which the pane passes to `dam ls` as it is:

```toml
[[views]]
name = "today"
query = "!done & (due:today | overdue)"

[[views]]
name = "deep"
query = "!done & effort:deep"
```

`dam` resolves a bare word against the saved filters in its own config first, so `query = "today"`
with a `[filter.today]` in `dam`'s config keeps one definition for every client. The grammar is
`&`, `|`, `!` and parentheses over terms such as `done`, `overdue`, `@<label>`, `p1` to `p4`,
`due:`, `deadline:`, `path:<path>`, `kind:task` and `<category>:<value>`; `dam`'s own
documentation has the full list.

View 1 is always `open`, the query `!done`: `dam ls` with no query includes completed objects. The
configured views follow in the order they are written. Two views with one name is a config error,
as is a view named `open` or one missing its `name` or its `query`.

Press `v` for the picker or a number key to switch. A query `dam` refuses comes back with `dam`'s
own message on the bottom line while the rows already on screen stay put, so a typo in one view
leaves the pane readable rather than blank.

### Views as keybindings

herdr declares plugin actions in the manifest and has no runtime action registration, and a
`plugin_action` keybinding passes no arguments, so there can be no `view:<name>` action per
configured view: the views are config and the manifest is not. The actions are numbered instead,
`view:1` to `view:9`. Nine is `MAX_NUMBERED_VIEW`, the ceiling the manifest and the number keys
share; raising it means adding entries in both places.

```toml
[[keys.command]]
key = "prefix+ctrl+t"
type = "plugin_action"
command = "herdr-damnit.view:2"
description = "damnit: show view 2"
```

A number with no view behind it exits non-zero saying how many views the config has.

## The list

The objects of the showing view, grouped by `path`: one heading per path segment with an object
under it, children indented one level deeper. Each row carries its marks, then the subject, then
the number of labels.

The marks lead so that a subject too long for the pane is what gets cut, with an ellipsis where it
was cut; a side pane is about 32 columns wide. Labels are counted rather than named for the same
reason, and the Detail screen names them.

| Mark       | Nerd Font  | Plain | Color  | Meaning                                    |
| ---------- | ---------- | ----- | ------ | ------------------------------------------ |
| priority 1 | flag       | `!`   | red    | `dam`'s priority 1, the most urgent        |
| priority 2 | flag       | `^`   | orange | `dam`'s priority 2                         |
| priority 3 | flag       | `-`   | blue   | `dam`'s priority 3                         |
| overdue    | warning    | `<`   | red    | due before today, with the date beside it  |
| today      | clock      | `*`   | yellow | due today, which needs no date beside it   |
| upcoming   | calendar   | `>`   | blue   | due later, with the date beside it         |
| recurring  | refresh    | `~`   | green  | the object carries a recurrence rule       |
| labels     | tags       | `@`   | purple | how many labels the object carries         |
| working    | pencil     | `*`   | yellow | changed here, not staged                   |
| staged     | plus       | `+`   | green  | staged, not committed                      |
| unpushed   | up arrow   | `^`   | cyan   | committed here, not yet on the remote      |
| conflict   | cross      | `x`   | red    | both sides changed it                      |

Priority 4 is `dam`'s default and carries no mark, and neither does an object with no due date.

## Colors

`theme` takes a theme name the way herdr and reviewr take one, and the names resolve to the same
palettes, so a workspace's panes match: `catppuccin` (the default), `catppuccin-latte`,
`catppuccin-frappe`, `catppuccin-macchiato`, `dracula`, `github-light`, `gruvbox`, `gruvbox-light`,
`monokai`, `nord`, `one-dark`, `one-light`, `rose-pine`, `rose-pine-dawn`, `solarized`,
`solarized-light`, `tokyo-night` and `tokyo-night-day`. The pane paints foregrounds only: the
background stays the terminal's own. The staging marks take green, yellow, cyan and red from the
same palette.

`icons = "ascii"` draws the plain set instead of the glyphs. A terminal whose font has no Nerd Font
glyph draws a replacement box that is often two cells wide, which puts every column in the pane out
by one, and no terminal reports which font it is using, so a pane cannot tell on its own: set this
key when the glyphs come out as boxes.

## The three screens

`<Tab>` cycles List, Status and Done, and `<S-Tab>` goes back. Each keeps its own cursor.

- **List** is the showing view.
- **Status** is `dam status`: Staged, Working, Unpushed, and Notices (conflicts and the notices a
  pull or push left), drawn in `dam`'s own order. An empty section is left out, and an empty
  screen says `nothing staged, nothing changed`. Every row that names an object is a cursor
  target, so staging, the detail, resolving and removing all work from here.
- **Done** is `dam ls done`, newest completion first with the date leading, the dates read from
  `dam log`. A task completed but not yet committed has no date and sorts to the top under
  `not committed`.

The header names the screen, or the view on the List, and while a `dam` command is out it carries a
spinner, the verb and the elapsed time: `dam  today  ⠙ push 3.2s`.

## Keys in the pane

Every key acts on the object under the cursor unless it says otherwise. A key pressed where it has
nothing to act on, a heading or an empty list, does nothing and says nothing.

| Key             | What it does                                                         |
| --------------- | -------------------------------------------------------------------- |
| `j`, `<Down>`   | move down one row                                                    |
| `k`, `<Up>`     | move up one row                                                      |
| `<Tab>`         | the next screen; `<S-Tab>` the one before                            |
| `v`             | the view picker, on List or Done                                     |
| `1` to `9`      | the List on that view                                                |
| `R`, `r`        | read the list and the status again                                   |
| `<CR>`          | the object's detail                                                  |
| `<Space>`       | stage the object, or unstage it when it is staged                    |
| `A`, `U`        | stage everything, unstage everything                                 |
| `c`             | commit what is staged, with a one-line message                       |
| `P`, `L`        | push, pull                                                           |
| `<C-c>`         | cancel the `dam` command the header names                            |
| `x`             | complete the task                                                    |
| `X`             | complete the task even past open children and dependencies           |
| `dd`            | remove the object, after the confirm                                 |
| `!`             | discard the object's working change, after the confirm              |
| `p`             | cycle the priority 4, 3, 2, 1 and back to 4                          |
| `s`, `D`        | set the due date, the deadline                                       |
| `l`             | add or remove a label from a picker                                  |
| `m`             | move the object to another path from a picker                        |
| `a`             | add a task under the path of the row under the cursor                |
| `S`             | hand the task to this workspace's agent pane                         |
| `e`             | edit the object in your editor through `dam edit -e`                 |
| `o`, `t`        | resolve the conflict under the cursor toward ours, toward theirs     |
| `q`             | close the pane                                                       |
| `<Esc>`         | close the picker, box or Detail screen that is open; else the pane   |

In a picker, `j` and `k` move, `<CR>` takes the entry under the cursor and `<Esc>` cancels. In a
one-line box, `<CR>` sends and `<Esc>` cancels.

**`X` is force-complete, not reopen.** In `herdr-todoist` it reopened a task; `dam` has no verb
that clears `done`, so the key now completes past whatever blocks a plain `x`.

**`!` is there only on a `dam` that has `restore`.** The pane reads the version as it opens and
binds the key from `dam` 0.2.0 on, which is also the oldest `dam` it opens on at all. A destructive
key that asked first and was then refused would be the worst shape a key could have, so without the
verb the key is simply not bound. The confirm names the object and the fields the change would lose.

`q` with a push, pull or commit running asks once, and a second `q` closes the pane and leaves the
`dam` child to finish. `dam push` records each mutation as its result arrives, so killing it halfway
could send a mutation twice next time; `<C-c>` is what stops it.

## The detail

`<CR>` runs `dam show` and draws the object: the subject as the heading, then its path, kind,
priority, due date, deadline, the event it is attached to, recurrence, every label by name, its
dependencies as short oids with their subjects, and its body as markdown. An event draws its start,
end, time zone, location, status, transparency and attendees with their responses. A field the
object has nothing for is left out. `<Esc>` goes back to the screen underneath, cursor included.

### How much markdown is rendered

A body is free text a person typed, and the pane it lands in is about 32 columns wide, so the
renderer covers what a person actually writes and leaves the rest as written:

- **Rendered:** ATX headings (bold, one weight at every level), bullet lists (every marker drawn as
  one dash), numbered lists (keeping the numbers written), blockquotes, thematic breaks, bold,
  italics, inline code, and links, which draw their text followed by `<target>` since a pane cannot
  be clicked and the target is the half worth copying. An underscore inside a word is left alone,
  so `a_variable_name` stays as typed.
- **Shown as written:** a table, because 32 columns cannot hold one, and the body of a fenced code
  block, because reflowing code changes what it says. The fences themselves are dropped.

Every line wraps rather than being cut: a long link, a wide table row and an unbreakable identifier
each break inside the pane.

## Sending a task to the agent

`S` hands the object under the cursor to the agent working in this workspace. It opens a
multi-line box for an optional note: `<CR>` opens a line, `<C-d>` sends and `<Esc>` throws the
draft away. A box sent blank sends the brief with no note. Nothing here happens on its own: no
timer, no event hook, no hand-off the operator did not press `S` for.

The brief is plain text:

```
dam task: file taxes
oid: 1a2b3c4d5e6f
path: home/admin/
due: 2026-09-20
priority: p1
labels: home, slow

receipts are in the drawer

note: start with the receipts
```

A field the object has nothing for is left out rather than written empty. The oid is what
`dam show` and every other `dam` client accept.

The brief reaches the pane through `herdr pane send-text` as one bracketed paste, without a return,
so the agent holds it until the operator submits it; `herdr agent focus` then puts the cursor there,
and a refused focus does not fail the send. Which pane is the agent pane comes from
`herdr agent list`: a pane herdr names an agent for, in this workspace, other than this one. A
workspace with no such pane says so.

The record of the hand-off is a label. With `handoff_label` set, a successful send runs
`dam edit <oid> --label <handoff_label>`, an ordinary working change that shows under Working on
the Status screen and is staged with everything else. A refused label says
`sent to <name>, label refused: <dam's message>.` rather than pretending the send failed, because
the agent has the work either way. With `handoff_label = ""` nothing is written and the bottom line
is the whole record.

## Editing in an editor

`e` runs `dam edit <oid> -e` on the pane's own terminal. `dam` renders the object as a commented
TOML template, opens the editor (`VISUAL`, then `EDITOR`, then `vi`), reads the file back on save,
and reopens it with the text intact when it does not parse, so nothing reaches the working layer
until it does.

The pane leaves the alternate screen before `dam` starts and enters it again once `dam` has gone,
on every path: a `dam` that exited non-zero and a `dam` that could not be started both come back to
a drawn pane. The list is read again whatever `dam` exited with, since a person who quit in a hurry
may still have saved. This is the one key that does not run on a worker thread, because it owns
the terminal.

## Quick edits

Each edit is one `dam` command, and every one is followed by a read of the status and the showing
view, so the rows come from the store rather than from a guess at what the write did.

| Key            | The `dam` command                                                        |
| -------------- | ------------------------------------------------------------------------ |
| `<Space>`      | `dam add <oid>`, or `dam reset <oid>` when it is staged                  |
| `A`, `U`       | `dam add -A`, `dam reset`                                                |
| `c`            | `dam commit -m "<line>"`                                                 |
| `x`, `X`       | `dam done <oid>`, `dam done <oid> --force`                               |
| `dd`           | `dam rm <oid>`                                                           |
| `!`            | `dam restore <oid>`                                                      |
| `p`            | `dam edit <oid> -p <next>`                                               |
| `s`            | `dam edit <oid> --due <line>`, or `--no-due` for an empty line           |
| `D`            | `dam edit <oid> --deadline <line>`, or `--no-deadline` for an empty line |
| `l`            | `dam edit <oid> --label <name>` or `--unlabel <name>`                    |
| `m`            | `dam mv <oid> <path>`                                                    |
| `a`            | `dam new "<line>" --path <path>`                                         |
| `o`, `t`       | `dam resolve <oid> --ours`, `--theirs`                                   |
| `P`, `L`       | `dam push`, `dam pull`                                                   |

- `c` with nothing staged says what to press instead, and a blank message is refused in the pane.
  The box is headed with the count `dam status` reported.
- The `s` and `D` box hints `today, tomorrow, YYYY-MM-DD or YYYY-MM-DDTHH:MM`.
- The `l` picker offers every label the showing view carries, with the object's own marked, and
  stays open after a pick so several labels go on or off at once. A label only this object carries
  is offered too, so it can be taken off. `dam` refuses a second value of an exclusive category and
  says which rule it broke.
- The `m` picker offers every path in the showing view and each of its parents, with the object's
  own path under the cursor.
- `a` names the object `dam` made: `made 7a8b9c0`. A blank subject is refused in the pane.

A box whose command `dam` refuses comes back with its line still in it and `dam`'s message on the
bottom line, so a mistyped date is corrected rather than retyped. The one exception is a store held
by another `dam`, which says to press `R` and closes the box so that `R` reaches the retry.

A read keeps the cursor on the same object rather than on the same row, so an object added, removed
or reordered above it does not move the highlight. When the object under the cursor is gone, the
cursor takes the next one below it, or the one above when it was the last.

## Development

```bash
just gates
```

runs `cargo fmt --all --check`, `cargo clippy --locked --workspace --all-targets -- -D warnings`,
`cargo doc` with warnings denied and `cargo test --locked --workspace`, which is what CI runs.

The workspace is four crates whose dependencies point inward only:

- `herdr-damnit-domain`: rows, marks, the staging model, views, the cursor, the brief, the version
  rules and the failure mapping, over `std` and `jiff`.
- `herdr-damnit-application`: the ports, every `dam` argv, the job table and the handshake.
- `herdr-damnit-adapters`: the only place a `dam` process is spawned, plus the herdr CLI, the config,
  the state directory, the clock and the parsing of `dam`'s JSON.
- `herdr-damnit`: the binary, with the draw loop, the screens and the keys.

No test runs a real `dam`, herdr or editor. The adapters crate builds `fake-dam`, a stand-in that
logs its arguments, replays a fixture named by its subcommand, and can sleep, fail or ignore an
interrupt on request; its knobs are `FAKE_DAM_*` environment variables or the same `NAME=value`
words placed before its arguments. The pane's own tests drive the real draw loop over it, which is
how a push is proven not to freeze the pane.

The `dam` documents the adapters read are fixtures in `crates/herdr-damnit-adapters/tests/fixtures`,
captured from a real `dam`. `capture.sh [<dam revision>]` there regenerates them from `dam` built
at that revision, `main` by default, and `DAM_BIN=/path/to/dam ./capture.sh` replays them against
a `dam` already built. The four sync documents are captured by hand.

## License

MIT, see [LICENSE](LICENSE).
