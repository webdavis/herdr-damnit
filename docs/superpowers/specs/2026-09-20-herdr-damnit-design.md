# herdr-damnit design

`herdr-damnit` is a task pane for the [herdr](https://herdr.dev) terminal multiplexer, drawn with
[ratatui](https://ratatui.rs), over the `dam` task store. It replaces `herdr-todoist`, which talked
to the Todoist API itself.

This document is the design for version one. Its upstream is the `damnit` design, which is the
binding description of `dam`; every claim below about a `dam` command, flag, output shape or exit
code is cited either to that document or to the version one code as built and measured on
2026-09-20. The sibling client `damnit.nvim` gets its own specification and presents the same model
this one does.

Two sources are cited throughout:

- **spec**: `damnit/docs/superpowers/specs/2026-09-18-damnit-design.md`, with a line number.
- **built**: the `dam` version one workspace, with a file path, and where a number is quoted, the
  command that produced it.

Where the two disagree, the built code wins and the disagreement is named.

## 1. Purpose and what changes

### The split

`dam` owns everything that used to be the plugin's second job:

- the Todoist API, reached through the `dam-remote-todoist` helper, which `dam` finds on `PATH` the
  way git finds `git-remote-https` (spec 259 to 270)
- the API token, resolved from the remote's own config table and handed to the helper in its
  environment, never in an argument or a log line (spec 302 to 318)
- the local copy of every task, which is a SQLite store rather than a per-view cache file
  (spec 379)
- writes made while the network is down, which are ordinary working-layer changes waiting for the
  next `dam push` rather than a queue file of their own (spec 27)
- synchronisation, conflicts and their resolution (spec 328 to 339)
- history, as commits (spec 198 to 207)

The pane owns what a pane is for:

- drawing the list, the staging model, the detail of one object and the notices
- navigation, the cursor and the named views
- staging, committing, pushing and pulling as key presses that run `dam`
- quick edits, each one a `dam` write verb
- the hand-off of a task to the workspace's agent pane
- entering an editor on one object and coming back

The line between them is a single rule: **the pane never holds a fact `dam` can answer for.** It
holds a cursor, a screen, a view selection and whatever is in flight. Everything else is re-read.

### Every herdr-todoist feature and its fate

| Feature | Fate | What replaces it, and why |
|---|---|---|
| `crates/todoist`, the API client | Removed | `dam-remote-todoist`. The pane makes no HTTP request and links no HTTP client. |
| Token resolution (`token_command`, `token_env`) | Moved into `dam` | `[remote.todoist]` in `dam`'s config, with the same three indirections (spec 306 to 313). The pane's config loses both keys. |
| The redacted token type | Removed | No token reaches the pane at all, which is stronger than redacting one. |
| The per-view cache under the plugin state directory | Removed | `dam`'s store is the local copy, and it is shared with every other `dam` client. |
| `queue.json`, the offline write queue | Removed | An unpushed `dam` commit is the same idea with history, resolution and a status line `dam` maintains. |
| The `+` waiting mark on a row | Changed | Becomes the unpushed mark, which now means "committed here, not yet on the remote". |
| `stale 5m` in the status line | Changed | Becomes the age of the last pull per remote, read from `dam`. Reads are local, so rows are never stale in the old sense. |
| `refresh_seconds`, the interval read | Kept, meaning changed | The interval now re-reads `dam`, which is local and cheap. A network pull is a separate key. |
| Grouping by project and section | Kept | `dam`'s `path` is the tree, so the grouping is by path segment instead of by project and section. |
| Subtasks folded under their parent | Kept | Same, off `path` depth. |
| The mark set and the icon sets | Kept | Unchanged, except that priority is no longer inverted. See section 4. |
| Themes by herdr palette name | Kept | Unchanged. |
| Named filter views | Changed | The filter language becomes `dam`'s query grammar, or the name of a `dam` saved filter. |
| `view:1` to `view:9` actions | Kept | Unchanged in shape and count. |
| The completed list on `<Tab>` | Changed | Becomes a `dam ls "done"` screen in a three-way screen cycle. See section 4. |
| The task detail | Changed | Drawn from `dam show --json`. The comment thread goes, because comments are out of scope for `dam` version one (spec 435). |
| Adding a comment (`c`) | Dropped | Nothing in `dam` stores one. The key is freed. |
| The markdown renderer | Kept | Renders the object's `body`, which is what the Todoist description became. |
| Sending a task to the agent (`S`) | Kept | Same herdr calls, same bracketed paste, same refusal rules. The brief's fields come from `dam` and the record of the hand-off changes. See section 4. |
| Editing in an editor (`e`) | Changed | Runs `dam edit <oid> -e`, which owns the template and the reopen-on-parse-failure loop. The `editor` config key goes. |
| The pane's own edit box | Dropped | `dam edit -e` falls back to `vi` when neither `VISUAL` nor `EDITOR` is set, so there is no editorless case left to cover. |
| Quick edits `x`, `dd`, `p`, `s`, `l`, `m`, `a` | Kept, re-aimed | Each becomes one `dam` verb. See the table in section 4. |
| `X`, reopen a task | Blocked | `dam` version one has no verb that clears `done`. `X` becomes force-complete and reopen returns when `dam` has it. See section 8. |
| The doctor action | Kept, re-aimed | Checks `dam`, its version, one `dam status --json`, and the configured remotes. |
| Placement, side, width, `auto_open`, `default_view` | Kept | Unchanged. |
| The pane registry under the plugin state directory | Kept | Unchanged: one remembered pane per workspace, and one requested-view file. |

Nothing in the fate table is a feature the operator loses without a named reason, and the two
losses (comments, reopen) are both `dam` gaps rather than pane decisions. Both are in section 8
with the `dam` change that closes them.

## 2. Naming and migration

### Names

| Thing | Name |
|---|---|
| Repository | `webdavis/herdr-damnit`, renamed in place from `webdavis/herdr-todoist` |
| Manifest `id` | `herdr-damnit` |
| Binary, and the file the build step writes | `bin/herdr-damnit` |
| Command crate | `crates/herdr-damnit` |
| Plugin config directory | the directory `herdr plugin config-dir herdr-damnit` prints |

The manifest `id` is what herdr registers a plugin as, verbatim, with no owner namespacing, so
every `plugin_action` keybinding names `herdr-damnit.<action>` and nothing else changes about the
binding shape. GitHub keeps redirecting the old repository path after a rename, so a clone by the
old name still resolves; the roster is still updated to the new one, because a pin that reads
`webdavis/herdr-todoist` for a plugin called `herdr-damnit` is a trap for the next reader.

### Action names

| `herdr-todoist` action | `herdr-damnit` action | Change |
|---|---|---|
| `open` | `open` | None |
| `toggle` | `toggle` | None |
| `focus` | `focus` | None |
| `view:1` to `view:9` | `view:1` to `view:9` | None. Nine stays the ceiling the manifest, the number keys and the hint line share. |
| `doctor` | `doctor` | Checks `dam` rather than the Todoist API. |
| none | `status` | One addition: open the pane on the Status screen. The staging model is the reason this plugin exists, so it gets a chord of its own. |

### What the dotfiles repository must change

Four edits, all in `webdavis/dotfiles`, none of which an agent applies:

1. **`dot_config/herdr/config.toml`**: the one `plugin_action` binding under the `herdr-todoist`
   banner changes its `command` from `herdr-todoist.toggle` to `herdr-damnit.toggle`, and the
   banner comment changes with it. The key stays `prefix+d`. Optionally a second binding for
   `herdr-damnit.status`.
1. **`.chezmoidata/system_packages_autoinstall.yaml`**: the roster row under
   `packages.herdr_plugins` changes `id: herdr-todoist` to `id: herdr-damnit`, `repo:
   webdavis/herdr-todoist` to `repo: webdavis/herdr-damnit`, and its `ref:` to the revision of the
   first `herdr-damnit` release. The comment beside it loses the sentence about the token, which is
   no longer the plugin's business.
1. **The plugin config**: `dot_config/herdr/plugins/config/herdr-todoist/config.toml` becomes
   `dot_config/herdr/plugins/config/herdr-damnit/config.toml`, with `token_command` removed and the
   `[[views]]` filters rewritten in `dam`'s grammar.
1. **`dam`'s own config**, `~/.config/dam/config.toml`, gains the remote and the token source:

   ```toml
   [remote.todoist]
   url = "todoist::"
   api_token_command = ["security", "find-generic-password", "-w", "-s", "Todoist API Token"]
   ```

   A `_command` inherits `dam`'s standard input (spec 375 to 377), and a client spawns `dam` with
   standard input closed, so a vault CLI that prompts for a password cannot resolve here. The
   deployed `herdr-todoist` config records that this was measured for both the pane and the Neovim
   plugin on 2026-09-18 and that the keychain read is what works. The same argv moves across
   unchanged.

### The `run_after_53` reinstall path, and what it does not do

`.chezmoiscripts/run_after_53-install-herdr-third-party-plugins.sh.tmpl` reads the roster, asks
`herdr plugin list --plugin <id> --json` for the revision herdr recorded, and reinstalls whatever
no longer sits at its pinned revision. It installs; **it never uninstalls.** A plugin whose `id`
changed is, to that script, one new plugin to install and one old plugin it has simply stopped
being told about, so `herdr-todoist` stays registered and its `prefix+d` binding would resolve to
whichever of the two herdr matched.

That makes three one-time operator steps, which belong in the migration pull request's body rather
than in any script, because this repository builds no removal mechanisms:

```bash
herdr plugin uninstall herdr-todoist
trash ~/.config/herdr/plugins/config/herdr-todoist
```

and a full `chezmoi apply`, which is what installs `herdr-damnit` at its pin and rewrites the
binding. Chezmoi does not delete a target that left the source, so the old plugin config directory
in the home directory is the operator's to remove.

### No compatibility shim

Neither product has shipped a 1.0. There is no `herdr-todoist` alias action, no config-key
fallback, no reader for the old cache or the old queue file, and no migration of either into `dam`.
The old plugin's state directory is abandoned where it sits and removed by hand. A shim here would
outlive its reason by years and would have to be tested for that whole time.

## 3. The dam boundary

### The decision

**The pane spawns `dam` as a subprocess and reads `--json` on standard output. One process per
action. It does not link `dam-application` or `dam-domain`.**

Four arguments, in the order they carried weight.

**One contract for both clients.** `damnit.nvim` is Lua and can only ever shell out. If the pane
links the library, the two clients read `dam` through two different surfaces, and a change to a use
case signature is invisible to one of them until a version bump. Spawning makes the command-line
surface the single contract, which is also the one `dam`'s own tests pin.

**Version skew is a store hazard, not just an API hazard.** The store is one SQLite file with
versioned migrations, and opening it reports `OpenError::NewerSchema { found, supported }`
(`crates/dam-adapters/src/sqlite/mod.rs`). A linked `dam-application` is a **second writer** with
its own schema opinion, installed and upgraded on its own schedule: the pane is installed by
`herdr plugin install` at a roster pin, and `dam` by `cargo install`. The day those diverge, a pane
built against schema N opens a store the installed `dam` migrated to N plus one, and the failure is
a refusal to open at best. Spawning means exactly one binary ever writes the store, and its version
is a fact the pane can read rather than a fact it embeds.

**The operator's standing ruling.** A shipped product chooses its own subprocess on its own merits,
the way it chooses `git`. The pane already spawns `herdr` for every pane and agent call
(`crates/herdr-todoist/src/herdr.rs`), so a second spawned tool adds no new class of dependency.

**Latency, measured rather than assumed.** Release build, a scratch store of 300 tasks across seven
path roots, N equals 20, median of wall clock including process start, on a machine also running
other work:

| Command | Median | Min | Max | Bytes on standard output |
|---|---|---|---|---|
| `dam ls --json` | 9.5 ms | 8.4 ms | 15.5 ms | 100,714 |
| `dam status --json` | 15.7 ms | 11.4 ms | 35.4 ms | 146,985 |
| `dam ls --toon` | 11.4 ms | 10.3 ms | 21.9 ms | 70,416 |
| `dam status` (human) | 13.8 ms | 10.9 ms | 18.9 ms | 13,104 |

The same four under a debug build ran 29 ms, 42 ms, 41 ms and 40 ms, which is what an unoptimised
`herdr plugin install` build would produce and is the number to beat if the install step ever stops
passing `--release`. The manifest's build line passes `--release` today.

`dam`'s own targets are under 20 ms for `ls --json` and under 30 ms for `status` on a store of ten
thousand objects (spec 385 to 386), so the measured numbers are inside the budget the tool set
itself and the budget is the one to design against.

The 147 KB `status` payload is worth reading carefully: every one of the 300 tasks was an uncommitted
create, and `status` embeds the full `before` and `after` object per change
(`crates/dam-cli/src/commands/status.rs`, `change_json`). After one `dam commit`, the same command
returned **91 bytes**. The payload is proportional to pending work, not to store size, and pending
work is bounded by what a person has changed since their last commit.

**What it costs if this is wrong.** Every key that writes pays one process start, roughly 10 ms,
plus the read that follows it. A key that only moves the cursor pays nothing, because the pane holds
the model between reads. If a store an order of magnitude larger pushes a read past 100 ms, the
non-blocking design in section 5 already keeps the pane drawing through it, and the fallback is
`dam`'s own note that the process start cost is measured before a daemon is reconsidered
(spec 389 to 390). That is `dam`'s decision to make, not the pane's, which is another reason not to
link.

### The version handshake

On start, before the first read, the pane runs `dam --version`. The binary prints `dam 0.2.0`
(`crates/dam-cli/Cargo.toml` at `webdavis/damnit` `84937a3`; `clap`'s standard `-V`). The pane
parses `dam <major>.<minor>.<patch>` and compares against two compiled-in constants:

- `DAM_MINIMUM`, the lowest version whose command surface the pane was written against: 0.2.0,
  which is where the error document, exit 4 for every refusal, and a change document with no
  embedded object arrived, all three of which the pane reads.
- `DAM_KNOWN`, the highest version the pane was tested against: 0.2.0.

Below 1.0, the minor is the breaking axis, so the rule is:

- version below `DAM_MINIMUM`: **refuse to draw.** One screen, one sentence: `dam 0.1.0 is older
  than the 0.2 this pane needs; run cargo install damnit to update it.`
- version above `DAM_KNOWN` in its minor: **draw, and warn once** in the status line: `dam 0.4.0 is
  newer than this pane knows; some keys may be refused.`
- anything in between: no message.

After the version check the pane runs one `dam status --json` and requires the five top-level keys
`staged`, `unstaged`, `conflicts`, `notices` and `unpushed` (`status.rs`). A document missing one of
them fails the handshake with the same refusal screen, which catches a `dam` that answers `--version`
but was built from a fork.

The handshake is one extra process start at open, roughly 20 ms for both calls. It is not repeated
on a refresh. The version it read is also what gates the one key that depends on a `dam` verb not
yet shipped, the discard key in section 4.

### Error mapping

**Exit codes**, from `dam` 0.2.0's own contract (its README and its design spec, `Exit codes`):

| Code | Meaning | Example |
|---|---|---|
| 0 | The command did what it said | any successful read |
| 1 | `dam` failed: a store, config, helper, credential, editor or io failure | a store another writer holds past its busy timeout |
| 2 | The command line was wrong: an unknown argument or subcommand, a flag value `dam` refuses to read, or an oid prefix naming more than one object | a subcommand that does not exist |
| 3 | Cancelled: an interrupt arrived, or a prompt could not be answered | See cancellation in section 5 |
| 4 | `dam` refused by one of its own rules, and the document names the rule | `dam done` on a task with an open child, rule `blocked` |

**Every rule `dam` keeps reports 4 and nothing else does**, so the pane maps the code once rather
than per verb. Code 2 is the command line alone, which for this pane means a flag typo it shipped
rather than anything the operator did.

**Under `--json` a failure is one document on standard error and nothing else**, so the pane parses
the stream rather than hunting a line in it:

```json
{"error": {"kind": "refused", "rule": "blocked",
 "message": "98d8780 cannot be completed: child a9db854 is open",
 "oids": ["98d878013fb0e026d37170e7ceed6707192ae99a",
          "a9db854060d1943ef9eb9f6d7a8ac0b1ace45d77"]}}
```

`kind` is one of `refused`, `store`, `helper`, `credential`, `parse`, `usage` and `cancelled`.
`rule` is one stable snake_case word on a refusal and null on every other kind: `blocked`, `cycle`,
`exclusive_label`, `unknown_category`, `no_such_object`, `no_working_object`, `no_such_remote`,
`not_a_task`, `not_an_event`, `not_completed`, `not_committed`, `dirty_on_pull`,
`move_inside_itself`, `nothing_to_commit`, `needs_an_answer`, `needs_an_editor`,
`unresolved_conflicts` and `missing_credential`. `message` is the sentence the human form prints
after `dam: `. `oids` names the objects the message names, in full and in the order it names them,
which is what lets the pane highlight the blockers a `done` refusal names instead of printing a
sentence about them. Standard output stays empty on a failure, and a run that succeeds writes
nothing on standard error.

The one exception is an argument clap rejects before `dam` runs: that prints clap's own usage text
and exits 2 whatever the format flag says, so the pane reads no document there.

So the pane's mapping is:

1. Exit 0: parse standard output as JSON. A parse failure is itself an error, reported as
   `dam answered with something this pane could not read`, with the first line of output in the
   pane's own log.
1. Non-zero: parse standard error as the error document and put its `message` in the status line as
   it stands. `dam`'s refusals already name the rule and the objects, so rewording them would only
   lose information. Standard error that is not a document, which is clap's usage text at exit 2 or
   a `dam` too old to print one, falls back to its first line with a leading `dam: ` stripped.
1. Exit 4 additionally leaves the pane's model untouched and, where the key had a prompt open,
   leaves the prompt open with its text, the way a refused write does today. The `rule` is what the
   pane branches on when it does more than print: `blocked` highlights the oids the document names,
   `no_such_object` refreshes the list because the row is gone.
1. Exit 3 is reported as `cancelled` and is never an error banner, because the operator asked for
   it.
1. Exit 1 with `kind` of `store` takes the retry sentence below.

**An editor never runs under a machine format.** `dam edit -e --json` is refused as
`needs_an_editor` rather than run, so the pane spells its editor round trip without `--json` and
reads the plain `dam: <message>` line if that run fails. A dead editor is exit 1, not a refusal.

**`dam` missing.** The spawn fails with `NotFound`. The pane draws its empty frame and one status
line: `dam is not on PATH; install it with cargo install damnit, then press R.` Every key that would
spawn `dam` repeats that line rather than retrying silently. `doctor` reports the same thing with the
`PATH` it searched.

**Store locked.** The store opens with a busy timeout of five seconds
(`crates/dam-adapters/src/sqlite/mod.rs`). A writer that holds it longer than that surfaces as a
`StoreError` and exit 1 with SQLite's own message. The pane shows it and adds four words:
`... ; another dam is writing, press R to retry.` It does not retry on its own, because the usual
cause is a long `dam pull` in another pane and a retry loop would just queue behind it.

**Remote unreachable.** This is the case that is not an error. `dam pull` against a helper that
cannot reach the service returns a **report**, exit 0, with the failure recorded as a notice:
`{"kind": "pull_failed", "remote": "todoist", "why": "..."}` (`status.rs`, `notice_json`). The same
holds for a per-mutation push failure, `{"kind": "push_failed", "remote": ..., "oid": ...,
"why": ...}`, and the push report carries `sent`, `succeeded`, `skipped` and a `failed` array
(`sync.rs`, `push_json`). So the pane treats a push or pull as successful whenever `dam` exits 0,
reports the counts in the status line, and draws the failures in the Notices section where they
persist until they are dealt with. A network outage never produces an error banner that disappears
on the next key press.

**A missing helper** is reported by `dam` by name (spec 270) and arrives as exit 1 with that
message, which is exactly what the operator needs to read.

## 4. The pane

### Screens

Three screens, cycled with `<Tab>` forward and `<S-Tab>` back:

1. **List**: the objects of the showing view.
1. **Status**: the staging model, in four labelled sections.
1. **Done**: completed objects, newest first.

A fourth screen, **Detail**, opens on `<CR>` from List or Status and returns with `<Esc>`. It is not
in the cycle, because it is about one object rather than about a set of them.

The screens are screens rather than views, the way the completed list already is in
`herdr-todoist`: the numbered `view:1` to `view:9` actions and `default_view` name the operator's own
queries, and none of these three has a query of its own.

### The List screen

The frame is unchanged from today: a status line, the body, a hint line.

```
dam  today  42 open  +3 staged  ^1 unpushed
proj/dotfiles
  ! < 09-18  ship the pin bump              @2
  ^ *       refresh the roster row
  +   >10-02 write the migration note
proj/home
    ~       water the plants
x X dd p s D l m a S e <CR> <Space> c P L v Tab
```

Rows group by `path`, one heading per path segment that has a visible object under it, children
indented one level deeper, which is the same shape the project and section grouping had. Marks lead
so the subject is what gets cut, with an ellipsis. Labels are counted, not named; the Detail screen
names them.

The mark set is the one that exists today, plus three staging marks and one correction:

| Mark | Nerd Font | Plain | Colour | Meaning |
|---|---|---|---|---|
| priority 1 | flag | `!` | red | `dam`'s priority 1, the most urgent |
| priority 2 | flag | `^` | orange | `dam`'s priority 2 |
| priority 3 | flag | `-` | blue | `dam`'s priority 3 |
| overdue | warning | `<` | red | due before today, with the date beside it |
| today | clock | `*` | yellow | due today, no date beside it |
| upcoming | calendar | `>` | blue | due later, with the date beside it |
| recurring | refresh | `~` | green | the object carries a recurrence rule |
| labels | tags | `@` | purple | how many labels the object carries |
| working | pencil | `*` | yellow | changed here, not staged |
| staged | plus | `+` | green | staged, not committed |
| unpushed | up arrow | `^` | cyan | committed here, not on the remote |
| conflict | cross | `x` | red | both sides changed it |

**The priority correction.** The Todoist API numbered priority the other way round from the app, so
`herdr-todoist` cycled `4, 3, 2, 1` and rendered the reverse. `dam` numbers 1 as highest
(spec 75), which is the app's own wording, so the inversion goes away entirely. `p` cycles
`4, 3, 2, 1` and back to `4`, and a `p1` mark means priority 1.

Priority 4 is `dam`'s default (`Priority::default()`) and carries no mark, the way the app's "no
priority" did.

### The Status screen

```
dam  status  3 staged  2 changed  1 unpushed  1 notice
Staged
  + new      1a2b3c4  ship the pin bump
  + changed  5d6e7f8  refresh the roster row  (due, priority)
Working
  * changed  9a0b1c2  water the plants  (subject)
Unpushed
  ^ todoist  1 commit
Notices
  x 3d4e5f6  todoist  ours: "mine"  theirs: "theirs"
  ! 7a8b9c0  removed on todoist: "old task" is kept here
```

The four sections are the four arrays `dam status --json` returns, drawn in the order `dam`'s own
human output uses (`status.rs`, `run_status`): `staged`, `unstaged` under the heading Working,
`unpushed`, then `conflicts` and `notices` together under Notices, because both are things that need
a decision. An empty section is left out, and an empty screen says `nothing staged, nothing
changed`, which is `dam`'s own wording.

Every row that names an `oid` is a cursor target, so stage, unstage, detail, resolve and remove all
work from here as well as from the List.

### The Done screen

`dam ls "done" --json` is the list, and `dam log --json` is where the completion dates come from: a
commit's `changes` array holds a `before` and an `after` per object (`status.rs`, `change_json`), so
the commit whose change flipped `after.task.done` to true is the one that completed it, and the
commit's `at` is the date. Two reads, both local, both already measured.

Rows are newest completion first, the date first so the dates line up down the pane. A task
completed in the working layer and not yet committed has no commit and therefore no date; it sorts
to the top under the heading `not committed`, which is also a hint that it is waiting to be staged.

There is no paging and no three-year wall. The old list walked back one three-month window per
request because that was the API's shape; `dam ls "done"` is one local query over the whole store.

### Keys

Every key below acts on the object under the cursor unless it says otherwise. The Given column
names the state the key needs; a key pressed outside it does nothing and says nothing, which is the
existing behaviour for a cursor on a heading.

| Key | Given | When | Then |
|---|---|---|---|
| `j`, `<Down>` | a list with rows | pressed | the cursor moves to the next row; at the last row it stays |
| `k`, `<Up>` | a list with rows | pressed | the cursor moves to the previous row; at the first row it stays |
| `<Tab>` | any of the three screens | pressed | the next screen in the cycle List, Status, Done is drawn, each keeping its own cursor |
| `<S-Tab>` | any of the three screens | pressed | the previous screen is drawn |
| `v` | List or Done | pressed | the view picker opens over the body, the showing view under the cursor |
| `1` to `9` | any screen | pressed | the List screen is drawn on that view, read with `dam ls <query> --json`; a number with no view behind it does nothing |
| `R`, `r` | any screen | pressed | `dam ls` and `dam status` are both re-read and the interval timer is put back to a full interval |
| `<CR>` | a row naming an oid | pressed | `dam show <oid> --json` runs and the Detail screen is drawn from it |
| `<Space>` | a row naming an oid, not staged | pressed | `dam add <oid>` runs; on exit 0 the row takes the staged mark after the status re-read |
| `<Space>` | a row naming an oid, staged | pressed | `dam reset <oid>` runs and the row loses the staged mark |
| `A` | any screen | pressed | `dam add -A` runs, staging every change |
| `U` | any screen | pressed | `dam reset` with no oid runs, unstaging everything |
| `!` | a row with a working change, and a `dam` that has `restore` | pressed once | a confirm names the object and the fields that would be lost |
| `!` | the confirm is up | `!` pressed again | `dam restore <oid>` runs and the working change is discarded; any other key dismisses the confirm and sends nothing |
| `!` | a `dam` older than the one that added `restore` | pressed | the key is unbound and nothing happens. See section 8, item 2 |
| `c` | the stage is not empty | pressed | a one-line message box opens over the body, headed with the count `dam status` reported |
| `c` | the message box is open | `<CR>` pressed | `dam commit -m "<line>" --json` runs; a blank or whitespace-only line is refused in the pane with `a commit needs a message` and nothing is spawned |
| `c` | the message box is open | `<Esc>` pressed | the box closes and nothing is spawned |
| `c` | the stage is empty | pressed | the status line says `nothing staged; press <Space> on a row or A to stage everything` and no box opens |
| `P` | no exclusive job in flight | pressed | `dam push --json` starts on a worker thread; the header shows a spinner and the elapsed time |
| `P` | a push or pull is already running | pressed | the status line says `a push is already running; <C-c> cancels it` and nothing is spawned |
| `L` | no exclusive job in flight | pressed | `dam pull --json` starts the same way |
| `x` | a task row | pressed | `dam done <oid>` runs; a refusal (exit 4, rule `blocked`) lists the blockers the document's `oids` names in the status line and changes nothing |
| `X` | a task row | pressed | `dam done <oid> --force` runs, completing past open children and dependencies |
| `dd` | a row naming an oid | first `d` | a confirm names the object |
| `dd` | the confirm is up | second `d` | `dam rm <oid>` runs, removing it from the working layer; any other key dismisses the confirm |
| `p` | a task row | pressed | `dam edit <oid> -p <next>` runs, where next cycles 4, 3, 2, 1 and back to 4 |
| `s` | a task row | pressed | a one-line box opens, hinting `today, tomorrow, YYYY-MM-DD or YYYY-MM-DDTHH:MM` |
| `s` | the box holds a line | `<CR>` pressed | `dam edit <oid> --due <line>` runs; a line `dam` cannot read comes back as `dam`'s own message with the line still in the box |
| `s` | the box is empty | `<CR>` pressed | `dam edit <oid> --no-due` runs, clearing the due date |
| `D` | a task row | pressed | the same box for `--deadline`, which `dam` keeps distinct from `due` |
| `l` | a row naming an oid | pressed | a picker of every label in the store opens, the ones this object carries marked |
| `l` | the picker is open | `<CR>` pressed | `dam edit <oid> --label <name>` or `--unlabel <name>` runs for the entry under the cursor and the picker stays open |
| `m` | a row naming an oid | pressed | a picker of every `path` in the store opens, plus one entry per parent prefix, the object's own path under the cursor |
| `m` | the picker is open | `<CR>` pressed | `dam mv <oid> <path>` runs, moving the object and its children |
| `a` | any screen | pressed | a one-line box opens for a new subject |
| `a` | the box holds a line | `<CR>` pressed | `dam new "<line>" --path <path of the row under the cursor> --json` runs and the status line names the oid it made |
| `e` | a row naming an oid | pressed | the pane leaves the alternate screen, runs `dam edit <oid> -e`, waits, re-enters, and re-reads |
| `S` | a row naming an oid | pressed | a multi-line note box opens over the body |
| `S` | the note box is open | `<C-d>` pressed | the brief is pasted into the workspace's agent pane and the status line names the agent; a blank box sends the brief with no note |
| `S` | the note box is open | `<Esc>` pressed | the box closes and nothing is sent |
| `o` | a conflict row on Status | pressed | `dam resolve <oid> --ours` runs |
| `t` | a conflict row on Status | pressed | `dam resolve <oid> --theirs` runs |
| `<C-c>` | a dam job is in flight | pressed | the job is cancelled. See section 5 |
| `q` | no exclusive job in flight | pressed | the pane closes |
| `q` | a push, pull or commit is in flight | pressed once | a confirm says `a push is running; q again quits and lets it finish` |
| `q` | that confirm is up | `q` pressed again | the pane closes and the `dam` child is left to finish. See section 5 |
| `<Esc>` | Detail, a picker or a box is open | pressed | that overlay closes and the screen under it is drawn |
| `<Esc>` | nothing is open | pressed | the pane closes |

In a picker, `j` and `k` move, `<CR>` takes the entry under the cursor and `<Esc>` cancels. In a
one-line box, `<CR>` sends and `<Esc>` cancels. In the multi-line note box, `<CR>` opens a line,
`<C-d>` sends and `<Esc>` throws the draft away; `<CR>` cannot also send, which is why sending has
its own key.

### Named views as dam queries

A view is a name and a query:

```toml
[[views]]
name = "today"
query = "!done & (due:today | overdue)"

[[views]]
name = "dotfiles"
query = "!done & path:webdavis/dotfiles/"

[[views]]
name = "deep"
query = "!done & effort:deep"
```

The query is passed to `dam ls` as its positional argument. `dam` resolves a bare word against the
saved filters in its own config first and parses it as a query otherwise
(`crates/dam-application/src/use_cases/list.rs`, `list`), so `query = "today"` with
`[filter.today]` in `dam`'s config works and keeps one definition for every client.

The grammar, from `crates/dam-domain/src/query/parse.rs`, is `&`, `|`, `!` and parentheses over
these terms: `done`, `overdue`, `@<label>`, `p<1-4>`, `kind:task|event`, `due:`, `deadline:` and
`start:` over `today`, `tomorrow`, `yesterday`, `this-week`, `next-week`, `none`, `before:<date>`,
`after:<date>` or a date, `path:<path>`, `label:<name>`, `priority:<n>`, `transparency:busy|free`,
`attached:<oid>`, `subject:<text>`, and `<category>:<value>` for any declared category.

**View 1 is `!done`, not the empty query.** `dam ls` with no query returns every object, done ones
included (`list`, which filters only when a query is given), so the unfiltered list would show
completed tasks mixed in. View 1 is defined as the query `!done` and the Done screen is `done`.
These six were verified against the built binary on the scratch store:

| Query | Rows |
|---|---|
| `!done` | 299 |
| `done` | 0 |
| `due:today \| overdue` | 0 |
| `!done & path:proj1/` | 42 |
| `!done & @l2` | 59 |
| `kind:task & !done & p1` | 74 |

A query `dam` refuses comes back as exit 4 with its own message, usually under rule
`unknown_category`, which goes in the status line while the rows already on screen stay put: a typo
in one view leaves the pane readable. Two views with one
name is a config error, as is a view named `all` or one missing its `name` or `query`.

### The label and path pickers

Neither needs a `dam` command of its own. The pane already holds every object of the showing view,
and both pickers read the union of what it holds: `labels` for `l`, and `path` plus every parent
prefix of it for `m`. A label the object carries that no other object does is offered too, so it can
be taken off.

The one thing that cannot be derived is the **category catalogue**. A category is a named set of
label values with an exclusivity rule, declared in `dam`'s config (spec 114 to 130), and `dam`
refuses a write that would put two values of an exclusive category on one object. The pane cannot
group or pre-refuse without that catalogue, so in version one it offers every label flat and lets
`dam` refuse: exit 4, rule `exclusive_label`, the picker left open. Reading the catalogue is in section 8.

### The detail screen

Drawn from `dam show <oid> --json`, which returns the object document (`show.rs`, `object_json`
over `to_wire`). The fields drawn, in order: subject as the heading, then `path`, `kind`, `priority`,
`due`, `deadline`, `recurrence`, `labels` named in full, `depends` as short oids with their subjects
resolved from the model, `event` when the task is attached to one, then `body` rendered as markdown.

For an event: `start`, `end`, `timezone`, `location`, `status`, `transparency`, and the attendees
with their responses.

The markdown renderer is kept exactly as it is: ATX headings, bullet and numbered lists,
blockquotes, thematic breaks, bold, italics, inline code, and links drawn as their text followed by
the target in angle brackets. Tables and fenced code bodies are shown as written. Every line wraps
rather than being cut, pinned by the existing test that draws the screen 32 columns wide.

There is no comment thread and no attachment line. `dam` version one stores neither (spec 435).

### Sending a task to the agent

Unchanged in mechanism. `S` opens the note box; the brief is written into the workspace's agent
pane with `herdr pane send-text` as one bracketed paste, without a return, so the operator submits
it; `herdr agent focus` then puts the cursor there, and a refused focus does not fail the send.
Which pane is the agent pane comes from `herdr agent list`: a pane herdr names an agent for, in this
workspace, other than this one.

Two things change, both because the source changed:

**The brief's fields.** There is no URL, because a `dam` object has none; the oid takes its place,
and it is what `dam show` and every other client accept.

```
dam task: file taxes
oid: 1a2b3c4
path: home/admin/
due: 2026-09-20
priority: p1
labels: home, slow

receipts are in the drawer

note: start with the receipts
```

A field the object has nothing for is left out rather than written empty.

**The record of the hand-off.** The old pane posted a Todoist comment. `dam` has no comments, so the
record is a label, named by config:

```toml
handoff_label = "handed-off"   # "" writes nothing
```

With a label set, a successful send runs `dam edit <oid> --label <handoff_label>`, which is a
working-layer change: it appears under Working on the Status screen and the operator stages it with
everything else. That is the right place for it, because a hand-off is a change to the task and
`dam` is the system of record. A refused label says `sent to <name>, label refused: <message>` and
does not pretend the send failed, because the agent has the work either way. With `handoff_label`
empty, nothing is written and the status line is the whole record.

### Editing in an editor

`e` runs `dam edit <oid> -e`. `dam` renders the object as a commented TOML template, spawns the
editor, parses the file back on save, and on a parse failure names the problem and reopens with the
text intact (`crates/dam-adapters/src/edit_template.rs` and `crates/dam-cli/src/commands/edit.rs`).
Nothing reaches the working layer until it parses.

The pane leaves the alternate screen before the spawn and enters it again once `dam` has gone, on
every path: a `dam` that exited non-zero and a `dam` that could not be started both come back to a
drawn pane rather than a terminal left in raw mode. The list is re-read whatever it exited with,
since a person who quit in a hurry may still have saved.

`dam` chooses the editor: `VISUAL`, then `EDITOR`, then `vi`
(`crates/dam-adapters/src/editor.rs`). The pane's own `editor` config key is removed, and with it
the pane's own edit box, because the fallback to `vi` means there is no editorless case left.

This is the one key that is deliberately **not** run on a worker thread: it owns the terminal, and
an editor drawn under a pane still drawing is the one thing the non-blocking model must not do. The
header says `editing` while it is out.

### Colours

`theme` takes a herdr palette name and resolves to the same palettes herdr and reviewr use, so a
workspace's panes match. The name list and the foreground-only rule are unchanged. The four staging
marks take `green`, `yellow`, `cyan` and `red` from the same palette, so no new colour key is
introduced.

## 5. Non-blocking operations

### Why this is a change rather than a tightening

The existing pane is `#[tokio::main]` and looks asynchronous, but its draw loop **awaits every API
call inline**: `status = screen(&mut pane, config, base_url).refresh().await;` sits between the
`terminal.draw` call and the `next_key` poll (`crates/herdr-todoist/src/tui.rs`). While a request is
out, nothing is drawn and no key is read. A slow network is a frozen pane today.

So "the pane keeps rendering and accepting keys while a push, pull or commit runs" is new behaviour,
and it is the requirement that shapes the whole runtime.

### The model: threads and a channel, no runtime

With `dam` owning every network call, the pane makes no HTTP request, so **`tokio` and `reqwest`
leave the dependency tree entirely.** What replaces them is smaller: one `std::thread` per `dam`
call and one `std::sync::mpsc` channel back to the draw loop. The Rust clean-code standard's own
words apply here: do not introduce an async runtime solely to make the architecture look modern, and
the synchronous, deadline-bounded model is acceptable.

The draw loop becomes:

```
loop {
    terminal.draw(...)                     // every tick, unconditionally
    drain the job channel (try_recv)       // non-blocking
    step the spinner
    if let Some(key) = poll_key(window) { handle(key) }
}
```

`handle` never blocks. A key that needs `dam` submits a job and returns; the job's result arrives on
a later tick.

The poll window is 50 ms while any job is in flight and 200 ms when none is, so an idle pane costs
what it costs today and a busy one animates smoothly. A resize is handled on either window, the way
it is today.

### Jobs

```
enum JobKind {
    ReadList,          // dam ls <query> --json
    ReadStatus,        // dam status --json
    ReadShow(Oid),     // dam show <oid> --json
    ReadLog,           // dam log --json
    Write(WriteKind),  // dam new | done | edit | mv | rm | add | reset | resolve
    Exclusive(Sync),   // dam push | pull | commit
}
```

A job carries its argv, a generation number, and the `Child` handle so it can be signalled. The
worker thread runs `Command::output()`, which reads both pipes to end of file, and sends one
`JobDone { kind, generation, code, stdout, stderr, elapsed }` back.

**The per-store operation queue.** At most one `Exclusive` job runs at a time, and a second is
**refused with a status line rather than queued**: `a push is already running; <C-c> cancels it`.
Two reasons. `dam push` with no remote sends to every remote (spec 219), so a second push is almost
never a different request; and a silent queue is a promise the pane would have to keep across a
close, which is the offline queue this design just deleted.

Reads are different: at most one of each `JobKind` is in flight, and a newer one supersedes an older
one. The generation number is what enforces it, so a result from a superseded read is dropped on
arrival rather than overwriting a fresher model. Writes run concurrently with reads; `dam` serialises
them at the store with its own five-second busy timeout.

### The header

While any job is in flight the status line carries a spinner, the verb and the elapsed time:

```
dam  today  ⠙ push 3.2s        42 open  +3 staged
```

The spinner is a braille cycle with `icons = "nerd-font"` and the four characters `|/-\` with
`icons = "ascii"`, stepping one frame per tick. Elapsed is whole tenths up to 10 s and whole seconds
after that, so the number stops flickering once a job is genuinely slow. With more than one job in
flight, the exclusive one is named; with no exclusive one, the count is shown as `2 reads`.

### Cancellation

`<C-c>` cancels the job the header names.

**The signal is `SIGINT`, not `SIGTERM`.** `dam`'s `main` installs a `SIGINT` handler that sets a
cancellation flag, every wait loop reads it, the helper conversation checks it on a 25 ms tick and
kills its child before returning `HelperError::Cancelled`, and `main` maps a cancelled run to exit 3
whatever error the abandoned work reported (`crates/dam-cli/src/main.rs`,
`crates/dam-adapters/src/cancel.rs`, `crates/dam-adapters/src/helper_process/conversation.rs`).
`dam` installs no `SIGTERM` handler, so a `SIGTERM` is the default action, which is death at an
arbitrary point with no chance to reap the helper.

So the sequence is:

1. The child is spawned into its own process group with `CommandExt::process_group(0)`, so a signal
   reaches the helper `dam` spawned as well as `dam` itself.
1. `<C-c>` sends `SIGINT` to the group.
1. After a two-second grace, if the child has not exited, `SIGKILL` to the group.
1. The status line says `cancelled` on exit 3 and `cancelled (killed)` when it took the second
   signal.

The process group matters because `dam push` spawns `dam-remote-todoist`, which is where the time
actually goes; signalling only `dam` would leave the helper running against the service.

Every read job is also bounded by a deadline of its own, thirty seconds, after which the pane
cancels it the same way and says `dam ls took longer than 30s and was cancelled`. A local SQLite
read that takes thirty seconds is a wedged store, not a slow one. `dam`'s own per-remote deadline for
a helper defaults to 60 s and is configurable as `deadline` under the remote
(`helper_process/conversation.rs`), so push and pull are bounded by `dam` rather than by the pane,
and the pane's own deadline for an exclusive job is deliberately absent.

### Completion

On `JobDone`:

1. Exit 0: the summary goes in the status line. For a push that is `dam`'s own line,
   `todoist: 3 sent, 3 ok, 0 failed, 0 skipped` (`sync.rs`, `push_line`), rebuilt from the JSON so
   the pane is not parsing human text. For a pull it is `todoist: 2 new, 1 updated, 40 unchanged,
   0 conflict(s), 0 removed upstream`.
1. **The model is re-read, never patched.** Completion enqueues a fresh `dam status --json`, and for
   anything that could change the visible set, a fresh `dam ls --json` as well. The old pane's rule
   was the same one for the same reason: the rows come from the store rather than from a guess at
   what the write did, so a completed task leaves the list and a moved one appears under its new
   path without the pane modelling either.
1. Non-zero: the mapping in section 3.

The cursor survives a re-read by oid rather than by row index, which is the existing behaviour: a
task added, removed or reordered above the cursor does not move the highlight, and when the object
under the cursor is gone the cursor takes the next row below it, or the one above when it was the
last.

### Closing mid-push

`q` with an exclusive job in flight asks once. On the second `q` the pane **detaches and exits
without signalling the child.**

`dam push` commits its ledger per mutation as results arrive (`sync.rs`, and spec 293 to 295:
successes leave the unpushed set, failures stay with their reason), so killing it halfway leaves
some mutations sent and recorded and others sent and unrecorded, which is the one state that makes
the next push send a duplicate. Letting it finish is strictly safer, costs the operator nothing, and
is what `<C-c>` is for when they actually want it stopped.

A read job, a write job or the editor is not worth that ceremony: those are killed on the way out.

### The test that proves it

`test_pane_renders_while_a_push_runs`, in the adapters crate:

- **Given** a fake `dam` on the pane's configured command path whose `push` sleeps three seconds and
  then prints a push report, and a pane driven by `ratatui::TestBackend` with a render counter,
- **When** `P` is pressed and the loop is driven for three and a half seconds of the test's own
  clock,
- **Then** the render count is at least 60 (three seconds at the 50 ms in-flight window, halved for
  slack), every frame after the first carries the spinner and a monotonically increasing elapsed
  time, a `j` pressed at the one-second mark moved the cursor on the very next frame, and the frame
  after the job finishes carries the push summary and no spinner.

The counter is the assertion that matters: a loop that awaited the push inline would render once and
fail on the first clause.

Two more in the same family: a second `P` during the sleep asserts the refusal status line and that
the fake `dam` recorded exactly one `push` in its argv log; and `<C-c>` during the sleep asserts the
fake recorded a `SIGINT`, exited 3, and the status line reads `cancelled`.

## 6. Architecture

### Crates

Four, not five. Dependencies point inward only.

```
crates/
  herdr-damnit-domain/        rows, marks, the staging model, view definitions, cursor rules,
                              date and priority formatting, the brief text, query strings.
                              std and jiff only.
  herdr-damnit-application/   use cases over ports: open the pane, read a view, read the status,
                              stage, unstage, commit, push, pull, each quick edit, the hand-off,
                              the editor round trip. Defines every port.
  herdr-damnit-adapters/      the dam runner (the only spawner of dam), the herdr CLI client,
                              the config reader, the state directory, the clock, the terminal.
  herdr-damnit/               package and binary `herdr-damnit`: argument parsing, the draw loop,
                              the ratatui widgets, composition.
```

**Why no protocol crate.** The five-role layout names a protocol crate for a wire the tool owns.
This pane owns no wire. The JSON it parses is `dam`'s, defined in `dam-protocol` and pinned by
`dam`'s own golden fixtures; the pane's obligation is to hold fixtures of the same documents and
fail when they move, which is a test concern rather than a crate. The two formats the pane does
define, the agent brief and the two state files, are a handful of lines each and live in the domain
crate with the rules that produce them. A crate for them would be a folder with one file in it.

This is the smaller set the standard allows, with its justification recorded here.

**The dam adapter is the only spawner.** `ProcessDamRunner` in the adapters crate is the single
place `std::process::Command` names `dam`. The application crate sees a port:

```rust
trait DamRunner: Send + Sync {
    fn spawn(&self, argv: &[String]) -> Result<RunningJob, SpawnError>;
}
```

`RunningJob` carries the child's identity for signalling and a receiver for its result. Nothing
above the adapters crate knows that `dam` is a process at all, which is what makes every use case
testable against a recorded argv list and a canned document, and what would make a future in-process
`dam` a one-crate change if that ever became the right answer.

The `herdr` CLI is a second port, `Herdr`, with the same shape, already factored this way in the
current code (`send.rs` takes a `&dyn Fn(&[&str]) -> Result<String, String>` so a test supplies its
own answer and its own workspace).

### Dependency direction

```
herdr-damnit  ->  herdr-damnit-adapters  ->  herdr-damnit-application  ->  herdr-damnit-domain
      \                                                  ^
       \------------------------------------------------/
                     (composition root wires ports to adapters)
```

`Cargo.toml` is the enforcer: a crate can only use what its own dependencies name, so an inward
dependency from the domain to a concrete adapter fails to compile.

### File sizes

Targets are 200 implementation lines and 300 total per file; 250 implementation or 400 total
normally requires decomposition; no handwritten file exceeds 500 total, unit tests included, with no
waiver. `main.rs` targets 50 to 150 lines and must be under 150 at completion.

The current code has three files over 400 total (`detail/tests.rs` at 500, `prompt.rs` at 502,
`config.rs` at 474) and `prompt.rs` is the one over the cap on implementation as well. The rewrite
splits `prompt.rs` by prompt kind and `config.rs` by concern (parse, validate, defaults), which the
crate split forces anyway.

A large unit-test module lives in a private child file beside its implementation, which is the
pattern already in use here (`src/reload.rs` beside `src/reload/tests.rs`).

### What the herdr plugin API requires

Cited from the current manifest and the code that reads it, since this is the contract the new
plugin inherits unchanged.

**The manifest**, `herdr-plugin.toml` at the repository root, declares `id`, `name`, `version`,
`min_herdr_version`, `platforms` and `description`. `min_herdr_version` stays `0.7.5`: the floor is
set by `herdr plugin pane open --entrypoint` and the plugin pane registry, present from 0.7.0, and by
the `pane_id`, `workspace_id` and `agent` fields of `herdr agent list`, documented from 0.7.5. The
new pane uses no herdr call the old one did not.

**`[[build]]`** is a command herdr runs at install time. The current one is
`cargo build --release --locked && mkdir -p bin && cp target/release/herdr-todoist bin/`, and it
changes only in the binary name. `herdr plugin link` skips this step, so a local checkout runs the
two commands by hand.

**`[[panes]]`** declares the pane entry point with an `id`, a `title`, a `placement` and a `command`;
the command runs the plugin binary by absolute path through `$HERDR_PLUGIN_ROOT`, because the binary
is not on `PATH`.

**`[[actions]]`** declares every action with an `id`, a `title`, a `contexts` list and a `command`.
Two properties of this shape drive the design: **actions are manifest-only**, with no runtime
registration in plugin v1, and **a `plugin_action` keybinding passes no arguments**. That is why the
view actions are numbered `view:1` to `view:9` rather than named per configured view, and why the
number the action was pressed for is written to a file the running pane reads on its next tick
rather than passed as an argument.

**`[[events]]`** declares a hook; the current manifest uses `on = "workspace.focused"` for
`auto_open`, and the new one keeps it.

**The environment herdr provides**, each used today: `HERDR_PLUGIN_ROOT` (the manifest's own command
lines), `HERDR_PLUGIN_ID` (`herdr.rs`, `open_plugin_pane`), `HERDR_PLUGIN_STATE_DIR` (`state.rs`),
`HERDR_BIN_PATH` (`herdr.rs`, `run`), and `HERDR_PANE_ID` plus `HERDR_WORKSPACE_ID` (`send.rs`,
`Host::from_env`).

**The herdr calls** the pane makes: `plugin pane open|focus|close`, `pane list`, `pane layout`,
`pane resize`, `pane send-text`, `agent list` and `agent focus`. Each answers with a JSON envelope
under a `result` key, and `herdr plugin list` returns exit 0 even when that envelope is an error, so
every read checks the envelope rather than the exit code.

## 7. Testing

### The layout that exists

Unit tests live beside their implementation as `#[cfg(test)] mod tests`, with a large module in a
private child file (`src/reload.rs` beside `src/reload/tests.rs`, `src/detail.rs` beside
`src/detail/tests.rs`, and five more). Thirty-one modules carry one today. Integration tests live in
`crates/todoist/tests/`, driven by a loopback HTTP double in `tests/support/mod.rs` that binds
`127.0.0.1:0`, serves one canned response and records the request line.

That layout carries over. What changes is the double: the HTTP double goes with the HTTP client, and
a fake `dam` takes its place.

### The fake dam

The pane resolves its `dam` from a config key:

```toml
dam = ["dam"]          # argv; the default
```

so a test points it at a fixture executable and no test mangles `PATH`. A test that wants the `PATH`
lookup proven instead writes the fake as `dam` into a temporary directory and prepends it, which is
one case rather than the mechanism for all of them.

The fake is a small Rust binary in the adapters crate's `tests/bin/`, built as a test dependency:

- it appends its full argv, one JSON line per call, to the file named by `FAKE_DAM_LOG`
- it replays the fixture named by `FAKE_DAM_FIXTURE` for the subcommand it was given, from
  `crates/herdr-damnit-adapters/tests/fixtures/<name>.json`
- `FAKE_DAM_EXIT` makes it exit with a chosen code and print a chosen line on standard error
- `FAKE_DAM_SLEEP` makes it sleep before answering, which is what the non-blocking tests drive
- it installs a `SIGINT` handler that records the signal and exits 3, so cancellation is observable

Fixtures are **owned by this repository** and are byte copies of documents the built `dam` actually
produced, captured with the commands recorded beside them. A `dam` change that moves the bytes fails a
test here, which is the point: the pane pins the contract from its own side, the way `dam` pins it
from the other.

The fixture set for version one: `ls.json`, `ls-empty.json`, `ls-done.json`, `status-clean.json`,
`status-full.json` (staged, unstaged, a conflict and each of the five notice kinds),
`show-task.json`, `show-event.json`, `log.json`, `push-ok.json`, `push-partial-failure.json`,
`pull-ok.json`, `pull-conflict.json`, `commit.json`, `new.json`.

### Golden renders

One per pane state, drawn through `ratatui::TestBackend` at 32 columns, which is the width a side
pane opens at and the width the existing render tests already use. The states: List with rows, List
empty, List with every mark present at once, Status with all four sections, Status clean, Done,
Detail of a task, Detail of an event, the view picker open, the label picker open, the one-line box
open, the note box open, the commit box open, the delete confirm, the discard confirm, and the
header with a spinner mid-push.

A golden is a text block in the test file rather than a separate file, so a diff in review shows the
screen rather than a path.

### The other obligations

- **The non-blocking test**, in full in section 5, plus the second-push refusal and the cancellation
  case.
- **The handshake refusal**: a fake printing `dam 0.0.9` draws the refusal screen and makes no
  further call, asserted against the argv log.
- **The handshake warning**: a fake printing a version above `DAM_KNOWN` draws the list and the
  warning once, not once per read.
- **Error mapping**, one test per row of the exit-code table: exit 4 with an error document of rule
  `no_such_object` puts its `message` in the status line and leaves the model untouched; exit 1 with
  a `store` document adds the retry sentence; exit 3 reads `cancelled`; exit 2 with clap's usage text
  and no document falls back to its first line; a spawn failure draws the install line.
- **Every key**, asserted as the argv it produces, against the argv log. This is the cheapest test in
  the suite and it is the one that catches a flag typo: `p` on a priority-2 task produces exactly
  `["dam", "edit", "<oid>", "-p", "1", "--json"]`.
- **Nothing reaches the network.** After the rewrite the pane's dependency tree contains no HTTP
  client and no TLS stack at all, which is a stronger statement than any test: `cargo tree` is the
  evidence, and a dependency review is what keeps it true. The old repository needed the loopback
  double precisely because it did have one.

### CI

The existing workflow stays, and it is worth stating what it actually is rather than what it is
often assumed to be. `.github/workflows/ci.yml` runs on **`ubuntu-latest`**, not macOS, on pushes to
`main` and on pull requests, with `permissions: contents: read`, `persist-credentials: false` and
actions pinned to full commit SHAs. Four gates:

```
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo doc --workspace --no-deps --locked
```

All four keep `--workspace`, so the crate split is covered the day it lands with no workflow change.

The manifest declares `platforms = ["macos", "linux"]` and the pane's only platform-specific code is
the signal handling in the dam adapter, which is `unix` rather than macOS. A macOS leg is worth
adding to the matrix only if that changes; until then a second runner buys a slower pipeline and no
new information.

## 8. Targets, errors, scope and open questions

### Performance targets

| Thing | Target | Where the number comes from |
|---|---|---|
| First frame after the pane opens | under 200 ms | two handshake calls plus one `dam ls --json`, measured at 9.5 ms and 15.7 ms, plus process start and the first draw |
| A key that does not spawn `dam` to its frame | under 16 ms | one draw, no I/O |
| A key that spawns `dam` to its spinner | under 50 ms | one poll window |
| A quick edit, key press to updated rows | under 120 ms | one write plus one `ls` plus one `status`, three process starts |
| Renders per second while a job is in flight | at least 15 | the 50 ms in-flight poll window |
| Peak resident memory | under 40 MB | one parsed model plus one in-flight document |

The interval read defaults to 300 seconds, as it does today, and now costs two local reads rather
than three network requests, so it can be lowered safely; it stays at 300 because there is no reason
to re-read a store only this machine writes.

### Errors and messages

One sentence each: what happened, and what to do. No error names a flag the operator did not press.

| Situation | Message |
|---|---|
| `dam` not on `PATH` | `dam is not on PATH; install it with cargo install damnit, then press R.` |
| `dam` older than the floor | `dam 0.1.0 is older than the 0.2 this pane needs; run cargo install damnit to update it.` |
| `dam` newer than known | `dam 0.4.0 is newer than this pane knows; some keys may be refused.` |
| A refusal (exit 4) | the error document's `message`, as it stands |
| Store held by another writer | `<sqlite's message>; another dam is writing, press R to retry.` |
| A second push | `a push is already running; <C-c> cancels it.` |
| A cancelled job | `cancelled`, or `cancelled (killed)` when it took the second signal |
| A read past its deadline | `dam ls took longer than 30s and was cancelled.` |
| A commit with no message | `a commit needs a message.` |
| A commit with nothing staged | `nothing staged; press <Space> on a row or A to stage everything.` |
| Output that will not parse | `dam answered with something this pane could not read.` |
| No agent pane in this workspace | `no agent pane in this workspace.` |
| A send that worked, a label that did not | `sent to <name>, label refused: <dam's message>.` |
| A view number with no view | `the config has <n> views.` |

No message carries a token, a path under the home directory, or a stack trace.

### Out of scope for version one

- **Events beyond reading them.** An event is listed, marked and shown in the Detail screen, and
  that is all. Creating one, editing its attendees, its conference link or its recurrence is `dam`
  flag work with a calendar-shaped UI behind it, and the Google helper ships second anyway
  (spec 322).
- **A history browser.** `dam log` is read for completion dates and nothing else. Browsing commits,
  showing one, and anything resembling a diff view is a screen of its own.
- **Editing a recurrence rule.** `dam edit --recurrence` takes a rule string, and the pane can pass
  one through the `s`-style box, but a picker over `every [n] day|week|month|year [on ...] [until
  ...]` is a form, not a line.
- **Choosing a remote.** `push` and `pull` with no remote act on every remote, which is one remote
  today. A picker arrives with the second one.
- **Multi-select.** Staging is one row at a time plus `A` and `U`. A visual-mode range is the obvious
  next thing and it is not in version one.
- **Categories as first-class UI.** See the label picker decision below.
- **Anything Todoist-specific.** No filter-language translation, no Quick Add syntax, no project or
  section vocabulary. The remote is `dam`'s business.

### Decisions closed 2026-09-20

**`<Tab>` cycles three screens.** Decided 2026-09-20: `<Tab>` cycles List, Status and Done rather
than giving Status a chord of its own, the recommended option. `<Tab>` already toggles two screens,
three is learnable, and it costs no key in a pane that is short of them. If it is wrong, the cost is
one extra press to reach Done, and the fix is a second key with no change to anything else.

**The pane reads `dam status` only after a write.** Decided 2026-09-20: read it only after a write,
on `R`, and on the interval, rather than on every tick, the recommended option. A per-tick read is
twenty reads a second against a store another client may be writing, for a number that changes when
this pane changes it. If it is wrong, a change made in `damnit.nvim` shows up in this pane one
interval late, and the fix is to lower the interval, which is already a config key.

**The label picker does not group by category in version one.** Decided 2026-09-20: ship a flat
list and let `dam`'s own refusal handle a conflict, the recommended option. It needs a `dam` command
that does not exist yet (see Needed from dam, below), and a flat list plus `dam`'s own refusal is
correct, just less pleasant. If it is wrong, an operator with exclusive categories gets a refusal
where a good picker would have shown them the conflict, which is annoying and never wrong.

**`handoff_label` defaults to `"handed-off"`.** Decided 2026-09-20: default to `"handed-off"` rather
than empty, the recommended option. A hand-off that leaves no trace is the failure mode the Todoist
comment existed to prevent, and a label is queryable, which a comment was not. If it is wrong, the
operator gets working-layer changes they did not ask for, and one config line turns it off.

**The pane keeps no written copy of the last model.** Decided 2026-09-20: no cache for a fast first
frame, the recommended option. `dam ls --json` measured 9.5 ms, which is faster than reading and
parsing a cache file would be, and a cache is the thing this design just spent a section deleting.
If it is wrong, the first frame after a cold start is briefly empty, and the fix is a spinner on the
first read rather than a file.

**`X` is force-complete.** Decided 2026-09-20: bind `X` to force-complete rather than leaving it
bound to reopen and refuse, the recommended option. A key that only ever prints "not supported"
teaches nothing and wastes a key in a pane with `q`, `Q`, `d`, `D` and little else left. An operator
with muscle memory from `herdr-todoist` may force-complete a task they meant to reopen; the
confirm-free path makes that worth watching, and the mitigation is that `X` is the only key whose
meaning changed silently, so it gets a line in the release notes.

### Decisions recorded

Made while writing this design, on 2026-09-20:

- The pane spawns `dam` and reads `--json`; it does not link `dam-application` or `dam-domain`.
  Measured latency, one contract for both clients, store-schema skew, and the operator's ruling on a
  shipped product's own subprocess.
- `tokio` and `reqwest` leave the dependency tree. Every `dam` call runs on a `std::thread` with an
  `mpsc` channel to the draw loop, which renders every tick and reads keys throughout.
- Cancellation sends `SIGINT` to the child's process group, then `SIGKILL` after two seconds,
  because `dam` handles `SIGINT` and does not handle `SIGTERM`.
- Quitting with a push, pull or commit in flight detaches rather than killing, after one confirm.
- At most one exclusive job at a time; a second is refused with a status line, never queued.
- Completion re-reads `dam status --json` and `dam ls --json` rather than patching the model.
- Three screens in one `<Tab>` cycle: List, Status, Done. Detail opens over them.
- View 1 is the query `!done`, because `dam ls` with no query includes completed objects.
- Priority is no longer inverted; `dam` numbers 1 as highest, which is the app's own wording.
- The token leaves the plugin entirely and lives in `dam`'s remote config.
- The cache file, the offline queue, the waiting mark's old meaning and the HTTP client are deleted,
  not ported.
- `e` runs `dam edit -e`, so the template, the parse-and-reopen loop and the editor choice are
  `dam`'s. The `editor` config key and the pane's own edit box go.
- The hand-off record becomes a configurable label, defaulting to `handed-off`, because `dam` has no
  comments.
- Four crates, not five: no protocol crate, because the pane owns no wire.
- Fixtures are copies of real `dam` output, owned by this repository, so a moved byte fails a test
  on this side as well as on `dam`'s.
- The repository is renamed in place and no compatibility shim ships, because neither product has
  reached 1.0.

### Needed from dam

Each item names what the pane needs, why, and the `dam` change that would provide it. None of them
blocks the pane shipping; the two marked **blocking a key** are the reason a key is absent or
re-aimed in version one.

**1. Clear `done` on a task. Blocking a key.** `dam` version one has `done` and no inverse:
`dam edit --help` lists no done flag, and there is no `undone` or `reopen` subcommand
(`crates/dam-cli/src/commands/mod.rs`, `dispatch`). A completed task cannot be reopened from any
client. Proposed: `dam edit <oid> --undone`, next to the other paired flags `--no-due`,
`--no-deadline`, `--no-recurrence` and `--detach`, which already establish the shape. The pane would
bind it to `u`, matching the completed list's existing key.

**2. Discard a working change. Blocking a key.** `dam reset` unstages; nothing restores an object to
its last committed state. The pane's `!` key has no verb to call. Proposed: `dam restore <oid>...`,
git's own word for it since 2.23, refusing on an object with no commit behind it and naming that in
the refusal. `dam reset --hard` would also work and reads worse, because `dam reset` already means
unstage.

The pane binds `!` only when the version the handshake read is at or above the one that adds
`restore`, so the key appears the day the operator updates `dam` and no pane release is needed for
it. `DAM_RESTORE` is 0.3.0 by assumption, the minor the pane expects the verb in; it moves if the
verb ships in another one. That gate is the only place a key depends on a `dam` version, and it exists because a confirm
followed by a refusal is the worst shape a destructive key can have.

**3. A JSON error envelope under `--json`. DELIVERED in `dam` 0.2.0.** Under `--json` and `--toon`
a failure is one document on standard error and nothing else,
`{"error": {"kind", "message", "rule", "oids"}}`, with standard output empty. It carries more than
this section asked for: a `rule`, one stable snake_case word per rule, so a client tells a blocked
completion from an object that is gone. An editor failure is not a kind of its own, because `-e`
under a machine format is refused as `needs_an_editor` rather than run. The mapping is in section 3.

**4. One exit code for every refusal. DELIVERED in `dam` 0.2.0.** Every rule `dam` keeps exits 4 and
nothing else does, `nothing_to_commit` among them. Exit 2 is now the command line alone, exit 1 is
every other failure including a dead editor, and exit 3 is cancelled.

**5. A completion timestamp.** The Done screen wants a completion date per task and the object
document has none (`crates/dam-protocol/src/messages.rs`, `WireTask` carries `done`, `priority`,
`due`, `deadline` and `event`). The pane recovers it by reading `dam log --json` and finding the
commit whose change flipped `done`, which is exact but is a second read and misses an uncommitted
completion. Proposed: a `completed_at` field on `WireTask`, set when `done` becomes true.

**6. The category catalogue and the saved filters.** Both live in `dam`'s config and no command
prints either. The pane cannot group a label picker by category, cannot pre-refuse an exclusive
clash, and cannot offer the saved filters as views. Proposed: `dam config --json` printing the
declared categories with their values and their `exclusive` flag, and the saved filter names with
their queries. Reading the config file directly is the alternative and it is wrong: the pane would
own a second parser for a file `dam` owns.

**7. A `status` that does not embed whole objects. DELIVERED in `dam` 0.2.0, as the default.**
`dam status --json` and `dam diff --json` answer one change document per change carrying the oid,
the operation, a `fields` list of what the change touches and the state the change left behind, with
no embedded object. `--full` adds `before` and `after` for a client that wants them; this pane does
not, because it polls `status` per render, which is the measurement that moved the default.

`dam` 0.2.0 carries two more facts this pane does not consume: `dam remote add --json` answers with
a `warnings` list, which matters to a client that adds a remote and this pane never does, and
`--toon` prints the same error document as `--json`, which this pane has no use for because it reads
JSON.

**8. Richer date words.** `--due` accepts `today`, `tomorrow`, `YYYY-MM-DD` and `YYYY-MM-DDTHH:MM`;
`dam` refuses `next mon` with a message naming exactly that set (measured). The old pane leaned on
the server's parser for `next mon` and `in 3 days`, and operators will reach for those. Proposed:
extend the `--due` parser with weekday names, `next <weekday>` and `in <n> <unit>`. The recurrence
parser already handles weekday names (`crates/dam-domain/src/recurrence/parse.rs`), so the vocabulary
exists in the crate.
