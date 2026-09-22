# herdr-damnit Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Turn `herdr-todoist` into `herdr-damnit`, a herdr pane over the `dam` task store, by renaming
this repository's crates in place and replacing the Todoist HTTP client with a `dam` subprocess
boundary.

**Architecture:** Four crates whose dependencies point inward only. `herdr-damnit-domain` holds rows,
marks, the staging model, views, the cursor, the brief text and the version rules, over `std` and
`jiff`. `herdr-damnit-application` holds the ports and the use cases: every `dam` argv, the job table
and the handshake. `herdr-damnit-adapters` is the only place `std::process::Command` names `dam`, and
also carries the herdr CLI client, the config reader, the state directory, the clock and the wire
parsing. `herdr-damnit` is the binary: argument parsing, the draw loop, the ratatui widgets and the
composition root. Every `dam` call runs on a `std::thread` and answers on an `mpsc` channel, so the
draw loop renders and reads keys throughout.

**Tech Stack:** Rust 2024, ratatui 0.30, crossterm 0.29, serde, serde_json, toml, jiff,
unicode-width, libc. No async runtime and no HTTP client.

**Spec:** `docs/superpowers/specs/2026-09-20-herdr-damnit-design.md`

## Global Constraints

Every task's requirements implicitly include this section.

- Rust follows the clean-code standard at `~/.agents/skills/clean-code-rust/SKILL.md`. Read it and its
  parent `~/.agents/skills/clean-code/SKILL.md` before the first task.
- Every `.rs` file is 300 lines ideal and 500 lines hard cap, unit tests included, with no waiver.
  `main.rs` targets 50 to 150 lines and must be under 150 at completion.
- Every test runs under one second. No network, no real herdr, no real `dam` store: the plugin's tests
  fake the `dam` subprocess.
- No em-dashes anywhere, in code, comments, tests, fixtures, documentation, commit messages or pull
  request bodies.
- Comments say what the code does or why it is the way it is. A comment never says what was rejected,
  what the file does not do, or anything about the conversation that produced it.
- Conventional commits, every commit made with `SKIP_AI_COMMIT=1` in the environment and no
  co-author trailer of any kind.
- The plugin never holds or reads a Todoist token. `dam` owns the token and resolves it from its own
  remote config.
- This is a public repository. No home directory paths, machine names, tokens or personal data in
  code, tests, fixtures or documentation.
- `Cargo.lock` stays committed and every build and test run passes `--locked`.
- CI is `.github/workflows/ci.yml` on `ubuntu-latest`: `cargo fmt --all --check`,
  `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked`
  and `cargo doc --workspace --no-deps --locked`. All four keep `--workspace`. There is no markdown
  formatter in this repository's CI.
- The dotfiles side is out of this plan. The `packages.herdr_plugins` roster row and the config leaf
  `dot_config/herdr/plugins/config/herdr-todoist/config.toml` in `webdavis/dotfiles` get their own
  pull request once the manifest id changes. Task 2 records exactly which values move.

## File Structure

New crates, created over the course of the plan:

```
crates/
  herdr-damnit-domain/      oid, priority, due, marks, theme, markdown, object, rows,
                            cursor, views, stage, brief, version, failure
  herdr-damnit-application/ ports, argv, jobs, handshake, handoff
  herdr-damnit-adapters/    dam_runner, cancel, wire, config, state, clock, herdr_cli,
                            tests/bin/fake-dam.rs, tests/fixtures/*.json
  herdr-damnit/             main.rs, app (draw loop, keys), screens, overlay, pane, doctor
```

Removed over the course of the plan: `crates/todoist` in full, and the modules of the command crate
that carried the Todoist path (`apply`, `cache`, `completed`, `connection`, `draft`, `edit`, `editor`,
`history`, `queue`, `reload`, `refresh`, `list`, `icons`, `detail`, `render`, `send`, `prompt`,
`tui`, `views`, `theme`, `markdown`, `cursor`, `config`, `state`, `placement`, `herdr`), each either
moved into a new crate first or deleted with the cutover in Task 31.

---

## Phase A: the rename in place

The repository is already `webdavis/herdr-damnit` and its remote already points at the new name. What
is still spelled `herdr-todoist` is the workspace member, the crate, the binary, the manifest, the two
directories the plugin reads and writes, and the README. Phase A renames all of it with the Todoist
code still in place, so every step ends on a green `cargo test --workspace --locked`.

### Task 1: Rename the command crate and its binary

**Files:**
- Move: `crates/herdr-todoist/` to `crates/herdr-damnit/` (`git mv`)
- Modify: `Cargo.toml`
- Modify: `crates/herdr-damnit/Cargo.toml`
- Modify: `crates/herdr-damnit/src/main.rs:36,97`
- Modify: `crates/herdr-damnit/src/herdr.rs:29`
- Modify: `crates/todoist/src/client.rs:29`
- Modify: `crates/herdr-damnit/src/cache.rs:153`, `src/pane.rs:303`, `src/state.rs:86`,
  `src/editor.rs:107,112`, `src/queue/tests.rs:7`, `src/reload/tests.rs:54,112`,
  `src/tui/tests.rs:55`, `src/theme.rs:271`

**Interfaces:**
- Consumes: nothing.
- Produces: the package `herdr-damnit` with `[[bin]] name = "herdr-damnit"`, built from
  `crates/herdr-damnit/src/main.rs`. Every later task's paths start `crates/herdr-damnit/`.

- [ ] **Step 1: Move the crate directory**

```bash
git mv crates/herdr-todoist crates/herdr-damnit
```

- [ ] **Step 2: Run the build to verify it fails**

Run: `cargo build --workspace --locked`
Expected: FAIL with `failed to load manifest for workspace member .../crates/herdr-todoist`

- [ ] **Step 3: Rewrite the workspace manifest**

`Cargo.toml`:

```toml
[workspace]
resolver = "3"
members = ["crates/todoist", "crates/herdr-damnit"]
default-members = ["crates/herdr-damnit"]

[workspace.package]
edition = "2024"
version = "0.1.0"
license = "MIT"
repository = "https://github.com/webdavis/herdr-damnit"

[workspace.dependencies]
reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"
tokio = { version = "1", default-features = false, features = ["macros", "process", "rt", "io-util", "net", "time"] }
```

- [ ] **Step 4: Rewrite the package manifest**

`crates/herdr-damnit/Cargo.toml`, first eleven lines:

```toml
[package]
name = "herdr-damnit"
description = "A task pane for the herdr terminal multiplexer, over the dam task store."
edition.workspace = true
version.workspace = true
license.workspace = true
repository.workspace = true

[[bin]]
name = "herdr-damnit"
path = "src/main.rs"
```

- [ ] **Step 5: Rename the word in every source file**

```bash
grep -rl 'herdr-todoist' crates/ --include='*.rs' | xargs sed -i '' 's/herdr-todoist/herdr-damnit/g'
```

That rewrites: the usage banner and the failure prefix in `main.rs`, the `HERDR_PLUGIN_ID` fallback in
`herdr.rs`, the HTTP user agent in `crates/todoist/src/client.rs`, the scratch file prefixes in
`cache.rs`, `pane.rs`, `state.rs`, `queue/tests.rs`, `reload/tests.rs` and `tui/tests.rs`, the fake
editor name in `editor.rs`, and the comment naming this file in `theme.rs`. The two directory strings
in `config.rs:194` and `state.rs:12` are rewritten by the same pass, which is what Task 3 then pins
with a test.

- [ ] **Step 6: Run the whole suite to verify it passes**

Run: `cargo test --workspace --locked`
Expected: PASS, every test, with no `herdr-todoist` left:

```bash
! grep -rn 'herdr-todoist' crates/ Cargo.toml
```

- [ ] **Step 7: Run the formatter and the linter**

Run: `cargo fmt --all --check && cargo clippy --workspace --all-targets --locked -- -D warnings`
Expected: both clean.

- [ ] **Step 8: Commit**

```bash
git add -A
SKIP_AI_COMMIT=1 git commit -m "refactor: rename the command crate and its binary to herdr-damnit"
```

---

### Task 2: Rename the plugin manifest and add the `status` action

**Files:**
- Modify: `herdr-plugin.toml` (in full)

**Interfaces:**
- Consumes: the binary name `herdr-damnit` from Task 1.
- Produces: manifest `id = "herdr-damnit"`, fourteen actions (`open`, `toggle`, `focus`, `status`,
  `view:1` to `view:9`, `doctor`), and the binary path `bin/herdr-damnit`. The binary must answer the
  subcommand `status`, which Task 41 implements; until then it exits non-zero with the usage banner,
  which is what an unknown command already does.

**The dotfiles values that move, for the separate pull request against `webdavis/dotfiles`:**

| File | Old value | New value |
|---|---|---|
| `dot_config/herdr/config.toml` | `command = "herdr-todoist.toggle"` on the `prefix+d` binding, under a `herdr-todoist` banner comment | `command = "herdr-damnit.toggle"`, banner `herdr-damnit`, description `damnit: toggle the task pane` |
| `.chezmoidata/system_packages_autoinstall.yaml` | `- id: herdr-todoist` / `repo: webdavis/herdr-todoist` / `ref: bef263d7d86f5aabb6aba35c3be56bc619e4ee21` | `- id: herdr-damnit` / `repo: webdavis/herdr-damnit` / `ref:` the revision of the first `herdr-damnit` release, with the comment's token sentence removed |
| `dot_config/herdr/plugins/config/herdr-todoist/config.toml` | the whole file, including `token_command`, `width = 0.3`, `default_view = "today"` and three `[[views]]` with `filter =` | moves to `dot_config/herdr/plugins/config/herdr-damnit/config.toml`, drops `token_command` and its comment block, keeps `width` and `default_view`, and rewrites each `filter =` as `query =` in `dam`'s grammar |
| `~/.config/dam/config.toml` | absent | gains `[remote.todoist]` with `url = "todoist::"` and the same `security find-generic-password` argv the plugin config held |

The three one-time operator steps belong in that pull request's body: `herdr plugin uninstall
herdr-todoist`, `trash ~/.config/herdr/plugins/config/herdr-todoist`, and a full `chezmoi apply`.

- [ ] **Step 1: Rewrite the manifest head**

`herdr-plugin.toml`, replacing everything above the first `[[actions]]`:

```toml
id = "herdr-damnit"
name = "damnit"
version = "0.1.0"
# `herdr plugin pane open --entrypoint` and the plugin pane registry this plugin's actions drive
# are present from 0.7.0. `S` also reads `herdr agent list`, whose `pane_id`, `workspace_id` and
# `agent` fields are documented from 0.7.5, which is what sets the floor.
min_herdr_version = "0.7.5"
platforms = ["macos", "linux"]
description = "A task pane over the dam task store: the terminal UI, and a doctor check for dam."

# Compiled at install time into bin/, which every entry point below runs by absolute path since the
# binary is not on PATH. `herdr plugin link` skips this step, so a local checkout runs the same two
# commands by hand.
[[build]]
command = [
  "sh",
  "-c",
  "cargo build --release --locked && mkdir -p bin && cp target/release/herdr-damnit bin/herdr-damnit",
]

[[panes]]
id = "pane"
title = "damnit"
placement = "split"
command = ["sh", "-c", "exec \"$HERDR_PLUGIN_ROOT/bin/herdr-damnit\""]

# A workspace gained focus. The command reads the config and exits when `auto_open` is off, which
# is the default, so the pane stays closed until an action asks for it.
[[events]]
on = "workspace.focused"
command = ["sh", "-c", "exec \"$HERDR_PLUGIN_ROOT/bin/herdr-damnit\" auto-open"]
```

- [ ] **Step 2: Rewrite every action**

Each of the thirteen `[[actions]]` blocks takes the same shape. The three pane actions:

```toml
[[actions]]
id = "open"
title = "damnit: open pane"
contexts = ["pane", "workspace"]
command = ["sh", "-c", "exec \"$HERDR_PLUGIN_ROOT/bin/herdr-damnit\" open"]

[[actions]]
id = "toggle"
title = "damnit: toggle pane"
contexts = ["pane", "workspace"]
command = ["sh", "-c", "exec \"$HERDR_PLUGIN_ROOT/bin/herdr-damnit\" toggle"]

[[actions]]
id = "focus"
title = "damnit: focus pane"
contexts = ["pane", "workspace"]
command = ["sh", "-c", "exec \"$HERDR_PLUGIN_ROOT/bin/herdr-damnit\" focus"]
```

The new one, which opens the pane on the Status screen:

```toml
# The staging model is the reason this plugin exists, so it gets a chord of its own rather than
# two `<Tab>` presses.
[[actions]]
id = "status"
title = "damnit: open pane on the status screen"
contexts = ["pane", "workspace"]
command = ["sh", "-c", "exec \"$HERDR_PLUGIN_ROOT/bin/herdr-damnit\" status"]
```

The nine numbered views keep their comment and their shape, with `view:1` shown here and `view:2`
through `view:9` written the same way with the number changed in the `id`, the `title` and the
command's trailing argument:

```toml
[[actions]]
id = "view:1"
title = "damnit: show view 1"
contexts = ["pane", "workspace"]
command = ["sh", "-c", "exec \"$HERDR_PLUGIN_ROOT/bin/herdr-damnit\" view 1"]
```

And the doctor, whose title changes because what it checks changed:

```toml
[[actions]]
id = "doctor"
title = "damnit: check dam and the configured remotes"
contexts = ["pane", "workspace"]
command = ["sh", "-c", "exec \"$HERDR_PLUGIN_ROOT/bin/herdr-damnit\" doctor"]
```

- [ ] **Step 3: Verify the manifest parses and holds what it should**

```bash
python3 -c "
import tomllib, sys
m = tomllib.load(open('herdr-plugin.toml','rb'))
assert m['id'] == 'herdr-damnit', m['id']
ids = [a['id'] for a in m['actions']]
want = ['open','toggle','focus','status'] + [f'view:{n}' for n in range(1,10)] + ['doctor']
assert ids == want, ids
assert 'herdr-todoist' not in open('herdr-plugin.toml').read()
print('manifest ok:', len(ids), 'actions')
"
```

Expected: `manifest ok: 14 actions`

- [ ] **Step 4: Commit**

```bash
git add herdr-plugin.toml
SKIP_AI_COMMIT=1 git commit -m "refactor(manifest): rename the plugin id and add the status action"
```

---

### Task 3: Rename the config and state directories the plugin reads

**Files:**
- Modify: `crates/herdr-damnit/src/config.rs:188-195`
- Modify: `crates/herdr-damnit/src/state.rs:8-14`
- Test: `crates/herdr-damnit/src/config.rs` (its `mod tests`)
- Test: `crates/herdr-damnit/src/state.rs` (its `mod tests`)

**Interfaces:**
- Consumes: the renamed crate from Task 1.
- Produces: `config::config_path()` resolving to `<config dir>/herdr/plugins/config/herdr-damnit/config.toml`
  and `state::state_dir()` resolving to `<state dir>/herdr/plugins/state/herdr-damnit`, both overridden
  by `HERDR_PLUGIN_CONFIG_DIR` and `HERDR_PLUGIN_STATE_DIR` when herdr sets them.

Task 1's `sed` pass already rewrote both strings. This task is the test that pins them, because the
directory name is the one rename an operator sees as a file on disk and the one a later refactor could
silently undo.

- [ ] **Step 1: Write the failing tests**

Add to `crates/herdr-damnit/src/config.rs`, inside `mod tests`, and make `config_path` and
`base_config_dir` take the override explicitly so the test needs no environment mutation:

```rust
    #[test]
    fn the_config_directory_is_the_plugins_own_name_under_herdr() {
        let path = config_path_in(None, Path::new("/x/.config"));
        assert_eq!(
            path,
            Path::new("/x/.config/herdr/plugins/config/herdr-damnit/config.toml")
        );
    }

    #[test]
    fn herdrs_own_config_directory_wins_when_it_names_one() {
        let path = config_path_in(Some(Path::new("/given")), Path::new("/x/.config"));
        assert_eq!(path, Path::new("/given/config.toml"));
    }
```

Add to `crates/herdr-damnit/src/state.rs`, inside `mod tests`:

```rust
    #[test]
    fn the_state_directory_is_the_plugins_own_name_under_herdr() {
        assert_eq!(
            state_dir_in(None, Path::new("/x/.local/state")),
            Path::new("/x/.local/state/herdr/plugins/state/herdr-damnit")
        );
    }

    #[test]
    fn herdrs_own_state_directory_wins_when_it_names_one() {
        assert_eq!(
            state_dir_in(Some(Path::new("/given")), Path::new("/x/.local/state")),
            Path::new("/given")
        );
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --workspace --locked config_directory state_directory`
Expected: FAIL with `cannot find function config_path_in` and `cannot find function state_dir_in`

- [ ] **Step 3: Split the environment read off the path arithmetic**

In `crates/herdr-damnit/src/config.rs`, replace `config_path` and keep `base_config_dir` as it is:

```rust
/// herdr hands the plugin its own config directory; the documented path is the fallback for a run
/// outside herdr, such as `herdr-damnit doctor` from a shell.
fn config_path() -> PathBuf {
    let given = std::env::var_os("HERDR_PLUGIN_CONFIG_DIR").map(PathBuf::from);
    config_path_in(given.as_deref(), &base_config_dir())
}

fn config_path_in(given: Option<&Path>, base: &Path) -> PathBuf {
    let dir = match given {
        Some(dir) => dir.to_path_buf(),
        None => base.join("herdr/plugins/config/herdr-damnit"),
    };
    dir.join("config.toml")
}
```

and widen the import at the top of the file to `use std::path::{Path, PathBuf};`.

In `crates/herdr-damnit/src/state.rs`, replace `state_dir` the same way:

```rust
/// herdr hands the plugin its own state directory; the documented path is the fallback for a run
/// outside herdr.
pub fn state_dir() -> PathBuf {
    let given = std::env::var_os("HERDR_PLUGIN_STATE_DIR").map(PathBuf::from);
    state_dir_in(given.as_deref(), &state_home())
}

fn state_dir_in(given: Option<&Path>, base: &Path) -> PathBuf {
    match given {
        Some(dir) => dir.to_path_buf(),
        None => base.join("herdr/plugins/state/herdr-damnit"),
    }
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --workspace --locked`
Expected: PASS, every test.

- [ ] **Step 5: Commit**

```bash
git add crates/herdr-damnit/src/config.rs crates/herdr-damnit/src/state.rs
SKIP_AI_COMMIT=1 git commit -m "refactor(config): pin the renamed config and state directories with tests"
```

---

## Phase B: the domain crate

`herdr-damnit-domain` is `std` and `jiff` only. It holds no serde derive, no ratatui type and no
process call, which is what makes every rule in it testable with a literal.

Two pieces of the current code deliberately stay in the binary crate rather than moving down here,
because both produce ratatui values: `src/theme.rs`, whose `Palette` holds `ratatui::style::Color`,
and `src/markdown.rs`, which produces `ratatui::text::Line`. Only the `Slot` enum moves down, in
Task 4, and the binary's `theme.rs` imports it from there.

### Task 4: Create the domain crate with `Oid`, `Priority` and `Slot`

**Files:**
- Create: `crates/herdr-damnit-domain/Cargo.toml`
- Create: `crates/herdr-damnit-domain/src/lib.rs`
- Create: `crates/herdr-damnit-domain/src/oid.rs`
- Create: `crates/herdr-damnit-domain/src/priority.rs`
- Create: `crates/herdr-damnit-domain/src/slot.rs`
- Modify: `Cargo.toml`
- Modify: `crates/herdr-damnit/Cargo.toml`
- Modify: `crates/herdr-damnit/src/theme.rs` (delete its `Slot`, import the domain one)

**Interfaces:**
- Consumes: the renamed workspace from Task 1.
- Produces:
  - `herdr_damnit_domain::Oid`, with `Oid::new(impl Into<String>) -> Oid`, `as_str(&self) -> &str`,
    `short(&self) -> &str` (the first seven characters), `Display`, `Clone`, `PartialEq`, `Eq`, `Hash`.
  - `herdr_damnit_domain::Priority`, with `Priority::new(u8) -> Option<Priority>`,
    `get(self) -> u8`, `next(self) -> Priority`, `is_lowest(self) -> bool`, `Default` of 4,
    `Copy`, `PartialEq`, `Eq`.
  - `herdr_damnit_domain::Slot`, the eight-variant colour-role enum moved out of the binary's
    `theme.rs` unchanged: `Text`, `Dim1`, `Red`, `Green`, `Yellow`, `Orange`, `Purple`, `Blue`.

- [ ] **Step 1: Write the failing tests**

`crates/herdr-damnit-domain/src/oid.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_short_oid_is_the_first_seven_characters() {
        assert_eq!(Oid::new("1a2b3c4d5e6f").short(), "1a2b3c4");
    }

    #[test]
    fn an_oid_shorter_than_seven_characters_is_its_whole_self() {
        assert_eq!(Oid::new("1a2b").short(), "1a2b");
    }
}
```

`crates/herdr-damnit-domain/src/priority.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dam_numbers_one_as_the_most_urgent_and_four_as_none() {
        assert_eq!(Priority::default().get(), 4);
        assert!(Priority::default().is_lowest());
        assert!(!Priority::new(1).expect("a priority").is_lowest());
    }

    #[test]
    fn nothing_outside_one_to_four_is_a_priority() {
        assert_eq!(Priority::new(0), None);
        assert_eq!(Priority::new(5), None);
        assert_eq!(Priority::new(1).map(Priority::get), Some(1));
        assert_eq!(Priority::new(4).map(Priority::get), Some(4));
    }

    #[test]
    fn the_cycle_runs_four_three_two_one_and_back_to_four() {
        let mut seen = Vec::new();
        let mut priority = Priority::default();
        for _ in 0..5 {
            seen.push(priority.get());
            priority = priority.next();
        }
        assert_eq!(seen, vec![4, 3, 2, 1, 4]);
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p herdr-damnit-domain --locked`
Expected: FAIL with `error: package ID specification 'herdr-damnit-domain' did not match any packages`

- [ ] **Step 3: Create the crate**

`crates/herdr-damnit-domain/Cargo.toml`:

```toml
[package]
name = "herdr-damnit-domain"
description = "The pane's rules: rows, marks, the staging model, views, the cursor and the brief."
edition.workspace = true
version.workspace = true
license.workspace = true
repository.workspace = true

[dependencies]
jiff.workspace = true
```

In the workspace `Cargo.toml`, add the member and the dependency:

```toml
members = ["crates/todoist", "crates/herdr-damnit-domain", "crates/herdr-damnit"]
```

and under `[workspace.dependencies]`:

```toml
herdr-damnit-domain = { path = "crates/herdr-damnit-domain" }
jiff = "0.2"
```

In `crates/herdr-damnit/Cargo.toml`, under `[dependencies]`:

```toml
herdr-damnit-domain.workspace = true
```

`crates/herdr-damnit-domain/src/lib.rs`:

```rust
//! The pane's rules, over `std` and `jiff` alone: no serde, no ratatui, no process call.

mod oid;
mod priority;
mod slot;

pub use oid::Oid;
pub use priority::Priority;
pub use slot::Slot;
```

- [ ] **Step 4: Write the three modules**

`crates/herdr-damnit-domain/src/oid.rs`, above its test module:

```rust
//! An object's identity as `dam` prints it. Rows, the cursor and every write argv name one.

/// The number of characters `dam` itself shows a prefix in, which is what a row has room for.
const SHORT: usize = 7;

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Oid(String);

impl Oid {
    pub fn new(text: impl Into<String>) -> Self {
        Self(text.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The prefix a row draws, or the whole oid when it is shorter than that.
    pub fn short(&self) -> &str {
        match self.0.char_indices().nth(SHORT) {
            Some((at, _)) => &self.0[..at],
            None => &self.0,
        }
    }
}

impl std::fmt::Display for Oid {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}
```

`crates/herdr-damnit-domain/src/priority.rs`, above its test module:

```rust
//! A task's priority the way `dam` numbers it: 1 is the most urgent and 4 is the default, which
//! carries no mark at all.

const HIGHEST: u8 = 1;
const LOWEST: u8 = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Priority(u8);

impl Default for Priority {
    fn default() -> Self {
        Self(LOWEST)
    }
}

impl Priority {
    pub fn new(value: u8) -> Option<Self> {
        (HIGHEST..=LOWEST).contains(&value).then_some(Self(value))
    }

    pub fn get(self) -> u8 {
        self.0
    }

    pub fn is_lowest(self) -> bool {
        self.0 == LOWEST
    }

    /// The next priority `p` cycles to: 4, 3, 2, 1 and back to 4.
    pub fn next(self) -> Self {
        match self.0 {
            HIGHEST => Self(LOWEST),
            value => Self(value - 1),
        }
    }
}
```

`crates/herdr-damnit-domain/src/slot.rs`:

```rust
//! The palette slot a drawn piece of the pane asks for by name. The colours themselves are the
//! binary crate's, resolved from the herdr theme the config names.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Slot {
    Text,
    Dim1,
    Red,
    Green,
    Yellow,
    Orange,
    Purple,
    Blue,
}
```

- [ ] **Step 5: Point the binary's theme at the domain slot**

In `crates/herdr-damnit/src/theme.rs`, delete the `pub enum Slot { ... }` declaration and add, under
the existing `use ratatui::style::Color;`:

```rust
pub use herdr_damnit_domain::Slot;
```

The `impl Palette { pub fn color(&self, slot: Slot) -> Color }` match below it is unchanged: it now
matches on the domain enum, which has the same eight variants.

- [ ] **Step 6: Run the tests to verify they pass**

Run: `cargo test --workspace --locked`
Expected: PASS, every test in both packages.

- [ ] **Step 7: Commit**

```bash
git add -A
SKIP_AI_COMMIT=1 git commit -m "feat(domain): add the domain crate with the oid, the priority and the colour slot"
```

---

### Task 5: The due state and the dates a row draws

**Files:**
- Create: `crates/herdr-damnit-domain/src/dates.rs`
- Modify: `crates/herdr-damnit-domain/src/lib.rs`

**Interfaces:**
- Consumes: the domain crate from Task 4.
- Produces:
  - `DueState`, an enum of `Overdue`, `Today`, `Upcoming` and `None`.
  - `due_state(due: Option<Date>, today: Date) -> DueState`.
  - `parse_date(text: &str) -> Option<Date>`, which reads both `YYYY-MM-DD` and a
    `YYYY-MM-DDTHH:MM[:SS][offset]` timestamp down to the civil date it leads with, so an offset
    carries no day of its own.
  - `short(date: Date) -> String`, the `MM-DD` a row draws.
  - `long(date: Date) -> String`, the `YYYY-MM-DD` the Detail screen draws.
  - `Date` is re-exported from `jiff::civil::Date` as `herdr_damnit_domain::Date`.

- [ ] **Step 1: Write the failing tests**

`crates/herdr-damnit-domain/src/dates.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn day(text: &str) -> Date {
        parse_date(text).expect("a date")
    }

    #[test]
    fn a_due_date_sits_before_on_or_after_today() {
        let today = day("2026-09-20");
        assert_eq!(due_state(Some(day("2026-09-18")), today), DueState::Overdue);
        assert_eq!(due_state(Some(day("2026-09-20")), today), DueState::Today);
        assert_eq!(due_state(Some(day("2026-10-02")), today), DueState::Upcoming);
        assert_eq!(due_state(None, today), DueState::None);
    }

    #[test]
    fn a_timestamp_is_read_down_to_the_day_it_falls_on() {
        assert_eq!(parse_date("2026-09-20T14:30"), Some(day("2026-09-20")));
        assert_eq!(parse_date("2026-09-20T14:30:00Z"), Some(day("2026-09-20")));
        assert_eq!(parse_date("2026-09-20T14:30:00-07:00"), Some(day("2026-09-20")));
    }

    #[test]
    fn an_offset_that_crosses_midnight_keeps_the_day_it_names() {
        assert_eq!(
            parse_date("2026-09-20T23:00:00-07:00"),
            Some(day("2026-09-20"))
        );
        assert_eq!(
            parse_date("2026-09-20T01:00:00+09:00"),
            Some(day("2026-09-20"))
        );
    }

    #[test]
    fn something_that_is_not_a_date_is_no_date() {
        assert_eq!(parse_date(""), None);
        assert_eq!(parse_date("tomorrow"), None);
        assert_eq!(parse_date("2026-13-40"), None);
    }

    #[test]
    fn a_row_draws_the_month_and_the_day_and_the_detail_draws_the_year_too() {
        assert_eq!(short(day("2026-09-18")), "09-18");
        assert_eq!(long(day("2026-09-18")), "2026-09-18");
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p herdr-damnit-domain --locked dates`
Expected: FAIL with `unresolved module or unlinked crate 'dates'`

- [ ] **Step 3: Write the module**

`crates/herdr-damnit-domain/src/dates.rs`, above its test module:

```rust
//! Where a date sits against today, and the two shapes the pane draws one in. The day is handed
//! in rather than read from the clock, which is what keeps a test from depending on when it runs.

pub use jiff::civil::Date;

/// Where a task's due date sits against today.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DueState {
    Overdue,
    Today,
    Upcoming,
    None,
}

pub fn due_state(due: Option<Date>, today: Date) -> DueState {
    match due {
        None => DueState::None,
        Some(date) if date < today => DueState::Overdue,
        Some(date) if date == today => DueState::Today,
        Some(_) => DueState::Upcoming,
    }
}

/// The length of the `YYYY-MM-DD` every `dam` date and timestamp leads with.
const CIVIL_DATE: usize = 10;

/// A `dam` date or timestamp read down to the civil day it names. The day is the text's own
/// leading `YYYY-MM-DD`, so a timestamp names the same day whatever offset it carries.
pub fn parse_date(text: &str) -> Option<Date> {
    text.get(..CIVIL_DATE)?.parse::<Date>().ok()
}

/// The `MM-DD` a row draws beside a due mark.
pub fn short(date: Date) -> String {
    format!("{:02}-{:02}", date.month(), date.day())
}

/// The `YYYY-MM-DD` the Detail screen and the Done screen draw.
pub fn long(date: Date) -> String {
    format!("{:04}-{:02}-{:02}", date.year(), date.month(), date.day())
}
```

Add to `crates/herdr-damnit-domain/src/lib.rs`:

```rust
mod dates;

pub use dates::{Date, DueState, due_state, long, parse_date, short};
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p herdr-damnit-domain --locked`
Expected: PASS, ten tests: the five from Task 4 and the five here.

- [ ] **Step 5: Commit**

```bash
git add -A
SKIP_AI_COMMIT=1 git commit -m "feat(domain): read a dam date and say where it sits against today"
```

---

### Task 6: The mark set and the two icon sets

**Files:**
- Create: `crates/herdr-damnit-domain/src/marks.rs`
- Modify: `crates/herdr-damnit-domain/src/lib.rs`

**Interfaces:**
- Consumes: `Priority`, `Slot` and `DueState` from Tasks 4 and 5.
- Produces:
  - `IconSet`, an enum of `NerdFont` and `Ascii`, with `IconSet::default()` of `NerdFont`. It carries
    no serde derive; the config reader in Task 25 maps its own string onto it.
  - `Mark`, an enum of `Priority(Priority)`, `Overdue`, `Today`, `Upcoming`, `Recurring`,
    `Labels(usize)`, `Working`, `Staged`, `Unpushed`, `Conflict` and `Notice`.
  - `Mark::glyph(self, set: IconSet) -> String` and `Mark::slot(self) -> Slot`.
  - `Mark::of_due(state: DueState) -> Option<Mark>` and
    `Mark::of_priority(priority: Priority) -> Option<Mark>`, both `None` where the spec draws
    nothing.

The priority inversion the Todoist API forced is gone: `dam` numbers 1 as highest, so priority 1 takes
the red flag, 2 the orange one and 3 the blue one, and 4 has no mark.

- [ ] **Step 1: Write the failing tests**

`crates/herdr-damnit-domain/src/marks.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn priority(value: u8) -> Priority {
        Priority::new(value).expect("a priority")
    }

    #[test]
    fn priority_one_is_the_urgent_one_and_four_carries_no_mark() {
        assert_eq!(
            Mark::of_priority(priority(1)).map(Mark::slot),
            Some(Slot::Red)
        );
        assert_eq!(
            Mark::of_priority(priority(2)).map(Mark::slot),
            Some(Slot::Orange)
        );
        assert_eq!(
            Mark::of_priority(priority(3)).map(Mark::slot),
            Some(Slot::Blue)
        );
        assert_eq!(Mark::of_priority(priority(4)), None);
    }

    #[test]
    fn the_plain_set_tells_the_three_priorities_apart_by_character() {
        let plain = |value: u8| {
            Mark::of_priority(priority(value))
                .expect("a mark")
                .glyph(IconSet::Ascii)
        };
        assert_eq!(plain(1), "!");
        assert_eq!(plain(2), "^");
        assert_eq!(plain(3), "-");
    }

    #[test]
    fn the_four_staging_marks_have_their_own_characters_and_colours() {
        for (mark, glyph, slot) in [
            (Mark::Working, "*", Slot::Yellow),
            (Mark::Staged, "+", Slot::Green),
            (Mark::Unpushed, "^", Slot::Cyan),
            (Mark::Conflict, "x", Slot::Red),
        ] {
            assert_eq!(mark.glyph(IconSet::Ascii), glyph);
            assert_eq!(mark.slot(), slot);
        }
    }

    /// A notice is a decision waiting rather than the refusal a conflict is, so it draws below
    /// red, and it draws the `!` the spec's Status screen shows in both sets: the Font Awesome
    /// block has no glyph that reads as a notice more plainly than the character itself.
    #[test]
    fn a_notice_draws_the_same_bang_whichever_set_is_chosen() {
        assert_eq!(Mark::Notice.glyph(IconSet::Ascii), "!");
        assert_eq!(Mark::Notice.glyph(IconSet::NerdFont), "!");
        assert_eq!(Mark::Notice.slot(), Slot::Orange);
    }

    #[test]
    fn a_label_count_is_the_sigil_and_the_number() {
        assert_eq!(Mark::Labels(2).glyph(IconSet::Ascii), "@2");
        assert_eq!(Mark::Labels(0).glyph(IconSet::Ascii), "@0");
    }

    #[test]
    fn a_due_state_takes_its_own_mark_except_when_there_is_no_date() {
        assert_eq!(Mark::of_due(DueState::Overdue), Some(Mark::Overdue));
        assert_eq!(Mark::of_due(DueState::Today), Some(Mark::Today));
        assert_eq!(Mark::of_due(DueState::Upcoming), Some(Mark::Upcoming));
        assert_eq!(Mark::of_due(DueState::None), None);
    }

    /// Every Nerd Font glyph is one cell wide, which is what keeps the columns of the pane lined
    /// up; a glyph a font has none of is drawn as a two-cell replacement box and puts every column
    /// after it out by one.
    #[test]
    fn every_nerd_font_glyph_is_a_single_character() {
        for mark in [
            Mark::Priority(priority(1)),
            Mark::Overdue,
            Mark::Today,
            Mark::Upcoming,
            Mark::Recurring,
            Mark::Working,
            Mark::Staged,
            Mark::Unpushed,
            Mark::Conflict,
            Mark::Notice,
        ] {
            let glyph = mark.glyph(IconSet::NerdFont);
            assert_eq!(glyph.chars().count(), 1, "{mark:?} drew {glyph:?}");
        }
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p herdr-damnit-domain --locked marks`
Expected: FAIL with `unresolved module or unlinked crate 'marks'`

- [ ] **Step 3: Add the cyan slot**

`Mark::Unpushed` is cyan and the palette has no cyan slot yet. In
`crates/herdr-damnit-domain/src/slot.rs`, add one variant:

```rust
    Cyan,
```

and in `crates/herdr-damnit/src/theme.rs`, add a `pub cyan: Color` field to `Palette`, a
`Slot::Cyan => self.cyan` arm to `Palette::color`, and a cyan value to every palette literal in the
file. The value for each theme is its own blue lightened toward green; where a theme's own table
already names a cyan, use that colour.

- [ ] **Step 4: Write the module**

`crates/herdr-damnit-domain/src/marks.rs`, above its test module:

```rust
//! The marks a row carries: the object's own state, and the staging state `dam status` reports for
//! it. Two sets draw them. The Nerd Font set uses glyphs from the Font Awesome block every Nerd
//! Font patches in, each one cell wide, except the notice, which draws a plain `!` in both sets;
//! the plain set uses one character per mark, for a terminal whose font has none of those glyphs.

use crate::{DueState, Priority, Slot};

/// Which set of marks the pane draws, chosen in the config file.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum IconSet {
    #[default]
    NerdFont,
    Ascii,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mark {
    Priority(Priority),
    Overdue,
    Today,
    Upcoming,
    Recurring,
    Labels(usize),
    Working,
    Staged,
    Unpushed,
    Conflict,
    Notice,
}

impl Mark {
    /// The mark a priority draws, or `None` for 4, which `dam` treats as no priority at all.
    pub fn of_priority(priority: Priority) -> Option<Self> {
        (!priority.is_lowest()).then_some(Self::Priority(priority))
    }

    /// The mark a due state draws, or `None` when the object has no date.
    pub fn of_due(state: DueState) -> Option<Self> {
        match state {
            DueState::Overdue => Some(Self::Overdue),
            DueState::Today => Some(Self::Today),
            DueState::Upcoming => Some(Self::Upcoming),
            DueState::None => None,
        }
    }

    pub fn slot(self) -> Slot {
        match self {
            Self::Priority(priority) => match priority.get() {
                1 => Slot::Red,
                2 => Slot::Orange,
                _ => Slot::Blue,
            },
            Self::Overdue | Self::Conflict => Slot::Red,
            Self::Today | Self::Working => Slot::Yellow,
            Self::Upcoming => Slot::Blue,
            Self::Recurring | Self::Staged => Slot::Green,
            Self::Labels(_) => Slot::Purple,
            Self::Unpushed => Slot::Cyan,
            Self::Notice => Slot::Orange,
        }
    }

    pub fn glyph(self, set: IconSet) -> String {
        match self {
            Self::Labels(count) => format!("{}{count}", Self::labels_sigil(set)),
            other => other.single(set).to_string(),
        }
    }

    fn labels_sigil(set: IconSet) -> &'static str {
        match set {
            IconSet::NerdFont => "\u{f02c}",
            IconSet::Ascii => "@",
        }
    }

    fn single(self, set: IconSet) -> &'static str {
        match (self, set) {
            (Self::Priority(_), IconSet::NerdFont) => "\u{f024}",
            (Self::Priority(priority), IconSet::Ascii) => match priority.get() {
                1 => "!",
                2 => "^",
                _ => "-",
            },
            (Self::Overdue, IconSet::NerdFont) => "\u{f071}",
            (Self::Overdue, IconSet::Ascii) => "<",
            (Self::Today, IconSet::NerdFont) => "\u{f017}",
            (Self::Today, IconSet::Ascii) => "*",
            (Self::Upcoming, IconSet::NerdFont) => "\u{f073}",
            (Self::Upcoming, IconSet::Ascii) => ">",
            (Self::Recurring, IconSet::NerdFont) => "\u{f021}",
            (Self::Recurring, IconSet::Ascii) => "~",
            (Self::Working, IconSet::NerdFont) => "\u{f040}",
            (Self::Working, IconSet::Ascii) => "*",
            (Self::Staged, IconSet::NerdFont) => "\u{f067}",
            (Self::Staged, IconSet::Ascii) => "+",
            (Self::Unpushed, IconSet::NerdFont) => "\u{f062}",
            (Self::Unpushed, IconSet::Ascii) => "^",
            (Self::Conflict, IconSet::NerdFont) => "\u{f00d}",
            (Self::Conflict, IconSet::Ascii) => "x",
            (Self::Notice, _) => "!",
            (Self::Labels(_), _) => Self::labels_sigil(set),
        }
    }
}
```

Add to `crates/herdr-damnit-domain/src/lib.rs`:

```rust
mod marks;

pub use marks::{IconSet, Mark};
```

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test --workspace --locked`
Expected: PASS, every test.

- [ ] **Step 6: Commit**

```bash
git add -A
SKIP_AI_COMMIT=1 git commit -m "feat(domain): add the mark set with dam's own priority direction"
```

---

### Task 7: The object model

**Files:**
- Create: `crates/herdr-damnit-domain/src/object.rs`
- Modify: `crates/herdr-damnit-domain/src/lib.rs`

**Interfaces:**
- Consumes: `Oid`, `Priority`, `Date` from Tasks 4 and 5.
- Produces the types every screen reads and Task 24 builds out of `dam`'s wire documents:

```rust
pub enum Kind { Task, Event }

pub struct Object {
    pub oid: Oid,
    pub kind: Kind,
    pub subject: String,
    pub body: String,
    pub path: String,
    pub labels: Vec<String>,
    pub depends: Vec<Oid>,
    pub recurrence: Option<String>,
    pub task: Option<TaskFields>,
    pub event: Option<EventFields>,
}

pub struct TaskFields {
    pub done: bool,
    pub priority: Priority,
    pub due: Option<Date>,
    pub deadline: Option<Date>,
    pub attached: Option<Oid>,
}

pub struct EventFields {
    pub start: String,
    pub end: String,
    pub timezone: Option<String>,
    pub location: Option<String>,
    pub status: String,
    pub transparency: String,
    pub attendees: Vec<Attendee>,
}

pub struct Attendee { pub email: String, pub response: String }
```

- plus `Object::is_done(&self) -> bool`, `Object::priority(&self) -> Priority` and
  `Object::due(&self) -> Option<Date>`, each answering for a task and answering the default for an
  event, so a row builder never matches on `kind` to ask a question every row asks.

The field names and the optionality come from `dam`'s own `WireObject`, `WireTask` and `WireEvent`
(`crates/dam-protocol/src/wire.rs` in `webdavis/damnit`): `oid`, `kind`, `subject`, `body`, `path`,
`labels`, `depends`, `recurrence`, `task` and `event` at the top level, `done`, `priority`, `due`,
`deadline` and `event` inside a task, and `start`, `end`, `timezone`, `location`, `attendees`,
`status` and `transparency` inside an event. `WireTask::event` is the oid of the event a task is
attached to and is called `attached` here, because `Object::event` already names the event fields.

- [ ] **Step 1: Write the failing tests**

`crates/herdr-damnit-domain/src/object.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_date;

    fn task() -> Object {
        Object {
            oid: Oid::new("1a2b3c4"),
            kind: Kind::Task,
            subject: "ship the pin bump".to_string(),
            body: String::new(),
            path: "proj/dotfiles".to_string(),
            labels: vec!["slow".to_string()],
            depends: Vec::new(),
            recurrence: None,
            task: Some(TaskFields {
                done: false,
                priority: Priority::new(1).expect("a priority"),
                due: parse_date("2026-09-18"),
                deadline: None,
                attached: None,
            }),
            event: None,
        }
    }

    #[test]
    fn a_task_answers_for_its_own_fields() {
        let object = task();
        assert!(!object.is_done());
        assert_eq!(object.priority().get(), 1);
        assert_eq!(object.due(), parse_date("2026-09-18"));
    }

    #[test]
    fn an_event_answers_the_defaults_rather_than_forcing_a_match_on_kind() {
        let object = Object {
            kind: Kind::Event,
            task: None,
            event: Some(EventFields {
                start: "2026-09-20T09:00".to_string(),
                end: "2026-09-20T10:00".to_string(),
                timezone: None,
                location: None,
                status: "confirmed".to_string(),
                transparency: "busy".to_string(),
                attendees: Vec::new(),
            }),
            ..task()
        };
        assert!(!object.is_done());
        assert_eq!(object.priority(), Priority::default());
        assert_eq!(object.due(), None);
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p herdr-damnit-domain --locked object`
Expected: FAIL with `unresolved module or unlinked crate 'object'`

- [ ] **Step 3: Write the module**

`crates/herdr-damnit-domain/src/object.rs`, above its test module, is the type block in the
Interfaces section verbatim, with this header and these three methods:

```rust
//! One object as `dam` describes it. The field names and the optionality are `dam`'s own wire
//! shape; what the pane adds is the three questions every row asks whatever the kind.

use crate::{Date, Oid, Priority};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Task,
    Event,
}

// ... the five structs from the Interfaces section, each deriving Clone, Debug, PartialEq, Eq ...

impl Object {
    pub fn is_done(&self) -> bool {
        self.task.as_ref().is_some_and(|task| task.done)
    }

    pub fn priority(&self) -> Priority {
        self.task
            .as_ref()
            .map_or_else(Priority::default, |task| task.priority)
    }

    pub fn due(&self) -> Option<Date> {
        self.task.as_ref().and_then(|task| task.due)
    }
}
```

Add to `crates/herdr-damnit-domain/src/lib.rs`:

```rust
mod object;

pub use object::{Attendee, EventFields, Kind, Object, TaskFields};
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p herdr-damnit-domain --locked`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add -A
SKIP_AI_COMMIT=1 git commit -m "feat(domain): model a dam object and the three questions every row asks"
```

---

### Task 8: The rows and the path grouping

**Files:**
- Create: `crates/herdr-damnit-domain/src/rows.rs`
- Create: `crates/herdr-damnit-domain/src/rows/tests.rs`
- Modify: `crates/herdr-damnit-domain/src/lib.rs`

**Interfaces:**
- Consumes: `Object`, `Mark`, `IconSet`, `Slot`, `Date`, `due_state`, `short`.
- Produces:

```rust
pub struct Segment { pub text: String, pub slot: Slot }
pub struct ObjectRow { pub oid: Oid, pub subject: String, pub segments: Vec<Segment> }
pub enum Row { Heading(String), Object(ObjectRow) }

impl Row {
    pub fn oid(&self) -> Option<&Oid>;
    pub fn text(&self) -> String;
}

/// The staging mark an oid carries, supplied by the Status model so a row builder
/// needs no second read.
pub trait StagingMarks {
    fn mark_of(&self, oid: &Oid) -> Option<Mark>;
}

pub struct RowStyle { pub icons: IconSet, pub today: Date }

pub fn rows(objects: &[Object], marks: &dyn StagingMarks, style: RowStyle) -> Vec<Row>;
```

The shape, from the spec's List mock: one heading per distinct `path`, headings in path order and
drawn flush left in full, each object under its heading indented two spaces, objects within a heading
ordered by subject. An object whose `path` is empty is grouped under a heading of `(no path)`. The
marks lead the line in this order: the staging mark, the priority mark, the due mark with its `MM-DD`
beside it for overdue and upcoming and nothing beside it for today, the recurrence mark, then the
subject, then the label count at the end.

- [ ] **Step 1: Write the failing tests**

`crates/herdr-damnit-domain/src/rows/tests.rs`:

```rust
use super::*;
use crate::{Kind, Object, Priority, TaskFields, parse_date};

struct NoMarks;

impl StagingMarks for NoMarks {
    fn mark_of(&self, _oid: &Oid) -> Option<Mark> {
        None
    }
}

struct OneMark(Oid, Mark);

impl StagingMarks for OneMark {
    fn mark_of(&self, oid: &Oid) -> Option<Mark> {
        (oid == &self.0).then_some(self.1)
    }
}

fn task(oid: &str, path: &str, subject: &str) -> Object {
    Object {
        oid: Oid::new(oid),
        kind: Kind::Task,
        subject: subject.to_string(),
        body: String::new(),
        path: path.to_string(),
        labels: Vec::new(),
        depends: Vec::new(),
        recurrence: None,
        task: Some(TaskFields {
            done: false,
            priority: Priority::default(),
            due: None,
            deadline: None,
            attached: None,
        }),
        event: None,
    }
}

fn style() -> RowStyle {
    RowStyle {
        icons: IconSet::Ascii,
        today: parse_date("2026-09-20").expect("a date"),
    }
}

fn drawn(rows: &[Row]) -> Vec<String> {
    rows.iter().map(Row::text).collect()
}

#[test]
fn every_path_gets_one_heading_and_its_objects_sit_under_it() {
    let objects = vec![
        task("2", "proj/home", "water the plants"),
        task("1", "proj/dotfiles", "ship the pin bump"),
        task("3", "proj/dotfiles", "refresh the roster row"),
    ];

    assert_eq!(
        drawn(&rows(&objects, &NoMarks, style())),
        vec![
            "proj/dotfiles".to_string(),
            "  refresh the roster row".to_string(),
            "  ship the pin bump".to_string(),
            "proj/home".to_string(),
            "  water the plants".to_string(),
        ]
    );
}

#[test]
fn an_object_with_no_path_is_grouped_rather_than_dropped() {
    let rows = rows(&[task("1", "", "file taxes")], &NoMarks, style());
    assert_eq!(
        drawn(&rows),
        vec!["(no path)".to_string(), "  file taxes".to_string()]
    );
}

#[test]
fn the_marks_lead_the_line_in_one_order() {
    let mut object = task("1", "home", "pay the rent");
    object.labels = vec!["home".to_string(), "slow".to_string()];
    object.recurrence = Some("every month".to_string());
    if let Some(fields) = object.task.as_mut() {
        fields.priority = Priority::new(1).expect("a priority");
        fields.due = parse_date("2026-09-18");
    }

    let rows = rows(
        &[object],
        &OneMark(Oid::new("1"), Mark::Staged),
        style(),
    );

    assert_eq!(rows[1].text(), "  + ! < 09-18 ~ pay the rent @2");
}

#[test]
fn a_date_due_today_carries_its_mark_and_no_date_beside_it() {
    let mut object = task("1", "home", "call the bank");
    if let Some(fields) = object.task.as_mut() {
        fields.due = parse_date("2026-09-20");
    }

    let rows = rows(&[object], &NoMarks, style());
    assert_eq!(rows[1].text(), "  * call the bank");
}

#[test]
fn a_mark_is_painted_by_what_it_means_and_the_subject_is_plain_text() {
    let mut object = task("1", "home", "call the bank");
    if let Some(fields) = object.task.as_mut() {
        fields.priority = Priority::new(1).expect("a priority");
    }

    let rows = rows(&[object], &NoMarks, style());
    let Row::Object(row) = &rows[1] else {
        panic!("expected an object row");
    };
    assert_eq!(row.segments[1].slot, Slot::Red);
    assert_eq!(
        row.segments.last().expect("a segment").slot,
        Slot::Text
    );
}

#[test]
fn a_heading_names_no_oid_and_an_object_row_does() {
    let rows = rows(&[task("1a2b3c4", "home", "x")], &NoMarks, style());
    assert_eq!(rows[0].oid(), None);
    assert_eq!(rows[1].oid(), Some(&Oid::new("1a2b3c4")));
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p herdr-damnit-domain --locked rows`
Expected: FAIL with `unresolved module or unlinked crate 'rows'`

- [ ] **Step 3: Write the module**

`crates/herdr-damnit-domain/src/rows.rs`:

```rust
//! The rows the pane draws: every object of the showing view grouped by its path, with the marks
//! leading so the subject is what gets cut when the pane is narrow.

use crate::{Date, DueState, IconSet, Mark, Object, Oid, Slot, due_state, short};

/// The heading an object with no path of its own is grouped under.
const NO_PATH: &str = "(no path)";

/// A run of a row drawn in one colour.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Segment {
    pub text: String,
    pub slot: Slot,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectRow {
    pub oid: Oid,
    /// The object's own subject, without the indentation and the marks, which is what a brief
    /// handed to an agent names it by.
    pub subject: String,
    pub segments: Vec<Segment>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Row {
    Heading(String),
    Object(ObjectRow),
}

impl Row {
    pub fn oid(&self) -> Option<&Oid> {
        match self {
            Self::Heading(_) => None,
            Self::Object(row) => Some(&row.oid),
        }
    }

    /// The whole line as plain text, which is what a width is measured over and what a test
    /// compares.
    pub fn text(&self) -> String {
        match self {
            Self::Heading(text) => text.clone(),
            Self::Object(row) => row.segments.iter().map(|part| part.text.as_str()).collect(),
        }
    }
}

/// The staging mark an oid carries. The Status model implements it, so a row builder needs no
/// second read of `dam status`.
pub trait StagingMarks {
    fn mark_of(&self, oid: &Oid) -> Option<Mark>;
}

pub struct RowStyle {
    pub icons: IconSet,
    pub today: Date,
}

pub fn rows(objects: &[Object], marks: &dyn StagingMarks, style: RowStyle) -> Vec<Row> {
    let mut paths: Vec<&str> = objects.iter().map(|object| heading(object)).collect();
    paths.sort_unstable();
    paths.dedup();

    let mut rows = Vec::new();
    for path in paths {
        rows.push(Row::Heading(path.to_string()));
        let mut under: Vec<&Object> = objects
            .iter()
            .filter(|object| heading(object) == path)
            .collect();
        under.sort_by(|left, right| left.subject.cmp(&right.subject));
        rows.extend(under.into_iter().map(|object| object_row(object, marks, &style)));
    }
    rows
}

fn heading(object: &Object) -> &str {
    match object.path.trim_end_matches('/') {
        "" => NO_PATH,
        path => path,
    }
}

fn object_row(object: &Object, marks: &dyn StagingMarks, style: &RowStyle) -> Row {
    let mut segments = vec![Segment {
        text: "  ".to_string(),
        slot: Slot::Text,
    }];
    let mut push = |mark: Mark| {
        segments.push(Segment {
            text: format!("{} ", mark.glyph(style.icons)),
            slot: mark.slot(),
        });
    };
    if let Some(mark) = marks.mark_of(&object.oid) {
        push(mark);
    }
    if let Some(mark) = Mark::of_priority(object.priority()) {
        push(mark);
    }
    let state = due_state(object.due(), style.today);
    if let Some(mark) = Mark::of_due(state) {
        push(mark);
        if let (Some(date), DueState::Overdue | DueState::Upcoming) = (object.due(), state) {
            segments.push(Segment {
                text: format!("{} ", short(date)),
                slot: mark.slot(),
            });
        }
    }
    if object.recurrence.is_some() {
        push(Mark::Recurring);
    }
    segments.push(Segment {
        text: object.subject.clone(),
        slot: Slot::Text,
    });
    if !object.labels.is_empty() {
        let mark = Mark::Labels(object.labels.len());
        segments.push(Segment {
            text: format!(" {}", mark.glyph(style.icons)),
            slot: mark.slot(),
        });
    }
    Row::Object(ObjectRow {
        oid: object.oid.clone(),
        subject: object.subject.clone(),
        segments,
    })
}

#[cfg(test)]
mod tests;
```

Add to `crates/herdr-damnit-domain/src/lib.rs`:

```rust
mod rows;

pub use rows::{ObjectRow, Row, RowStyle, Segment, StagingMarks, rows};
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p herdr-damnit-domain --locked`
Expected: PASS, six new tests.

- [ ] **Step 5: Commit**

```bash
git add -A
SKIP_AI_COMMIT=1 git commit -m "feat(domain): group rows by dam's path and lead them with their marks"
```

---

### Task 9: The cursor, keyed by oid

**Files:**
- Create: `crates/herdr-damnit-domain/src/cursor.rs`
- Modify: `crates/herdr-damnit-domain/src/lib.rs`
- Reference: `crates/herdr-damnit/src/cursor.rs`, whose rules this reproduces over `Row` and `Oid`

**Interfaces:**
- Consumes: `Row` and `Oid` from Task 8.
- Produces:

```rust
pub struct Cursor { /* rows and the selected index */ }

impl Cursor {
    pub fn new(rows: Vec<Row>) -> Cursor;
    pub fn rows(&self) -> &[Row];
    pub fn selected(&self) -> usize;
    pub fn selected_oid(&self) -> Option<&Oid>;
    pub fn object_count(&self) -> usize;
    pub fn move_by(&mut self, steps: isize) -> bool;
    pub fn replace(&mut self, rows: Vec<Row>);
}
```

The rules, carried over unchanged from the existing `cursor.rs`: the cursor starts on the first object
row; `move_by` steps over object rows only, skipping headings, and stops at either end reporting
whether it moved; `replace` keeps the cursor on the object it was on, and when that object is gone
takes the nearest surviving object below it in the old order, or above it when it was the last.

- [ ] **Step 1: Write the failing tests**

`crates/herdr-damnit-domain/src/cursor.rs`, inside `mod tests`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ObjectRow, Segment, Slot};

    fn row(oid: &str) -> Row {
        Row::Object(ObjectRow {
            oid: Oid::new(oid),
            subject: oid.to_string(),
            segments: vec![Segment {
                text: oid.to_string(),
                slot: Slot::Text,
            }],
        })
    }

    fn listing(oids: &[&str]) -> Vec<Row> {
        let mut rows = vec![Row::Heading("home".to_string())];
        rows.extend(oids.iter().map(|oid| row(oid)));
        rows
    }

    #[test]
    fn the_cursor_starts_on_the_first_object_rather_than_the_heading() {
        let cursor = Cursor::new(listing(&["a", "b"]));
        assert_eq!(cursor.selected_oid(), Some(&Oid::new("a")));
    }

    #[test]
    fn moving_skips_headings_and_stops_at_either_end() {
        let mut cursor = Cursor::new(listing(&["a", "b"]));
        assert!(cursor.move_by(1));
        assert_eq!(cursor.selected_oid(), Some(&Oid::new("b")));
        assert!(!cursor.move_by(1), "it moved past the last object");
        assert!(cursor.move_by(-1));
        assert_eq!(cursor.selected_oid(), Some(&Oid::new("a")));
        assert!(!cursor.move_by(-1), "it moved past the first object");
    }

    #[test]
    fn a_reread_keeps_the_cursor_on_the_object_it_was_on() {
        let mut cursor = Cursor::new(listing(&["a", "b", "c"]));
        cursor.move_by(2);
        cursor.replace(listing(&["z", "a", "b", "c"]));
        assert_eq!(cursor.selected_oid(), Some(&Oid::new("c")));
    }

    #[test]
    fn an_object_that_is_gone_hands_the_cursor_to_the_next_one_below_it() {
        let mut cursor = Cursor::new(listing(&["a", "b", "c"]));
        cursor.move_by(1);
        cursor.replace(listing(&["a", "c"]));
        assert_eq!(cursor.selected_oid(), Some(&Oid::new("c")));
    }

    #[test]
    fn the_last_object_hands_the_cursor_upward_when_it_goes() {
        let mut cursor = Cursor::new(listing(&["a", "b"]));
        cursor.move_by(1);
        cursor.replace(listing(&["a"]));
        assert_eq!(cursor.selected_oid(), Some(&Oid::new("a")));
    }

    #[test]
    fn an_empty_list_selects_nothing_and_moves_nowhere() {
        let mut cursor = Cursor::new(Vec::new());
        assert_eq!(cursor.selected_oid(), None);
        assert!(!cursor.move_by(1));
        assert_eq!(cursor.object_count(), 0);
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p herdr-damnit-domain --locked cursor`
Expected: FAIL with `unresolved module or unlinked crate 'cursor'`

- [ ] **Step 3: Write the module**

`crates/herdr-damnit-domain/src/cursor.rs`, above its test module:

```rust
//! The pane's selection. The cursor belongs to an object's identity rather than to a row number,
//! so a re-read that inserts, removes or reorders rows leaves the highlight on the same object.

use crate::{Oid, Row};

pub struct Cursor {
    rows: Vec<Row>,
    selected: usize,
}

impl Cursor {
    pub fn new(rows: Vec<Row>) -> Self {
        let mut cursor = Self { rows, selected: 0 };
        cursor.selected = cursor.object_indices().first().copied().unwrap_or(0);
        cursor
    }

    pub fn rows(&self) -> &[Row] {
        &self.rows
    }

    pub fn selected(&self) -> usize {
        self.selected
    }

    pub fn selected_oid(&self) -> Option<&Oid> {
        self.rows.get(self.selected).and_then(Row::oid)
    }

    pub fn object_count(&self) -> usize {
        self.rows.iter().filter(|row| row.oid().is_some()).count()
    }

    /// Move by `steps` object rows, skipping headings and stopping at either end. Reports whether
    /// it moved.
    pub fn move_by(&mut self, steps: isize) -> bool {
        let objects = self.object_indices();
        if objects.is_empty() {
            return false;
        }
        let at = objects
            .iter()
            .position(|index| *index >= self.selected)
            .unwrap_or(objects.len() - 1) as isize;
        let target = (at + steps).clamp(0, objects.len() as isize - 1) as usize;
        let moved = objects[target] != self.selected;
        self.selected = objects[target];
        moved
    }

    /// Replace the rows, keeping the cursor on the object it was on. When that object is gone the
    /// cursor takes the nearest surviving object below it in the old order, or above it when it
    /// was the last.
    pub fn replace(&mut self, rows: Vec<Row>) {
        let preferred: Vec<Oid> = self.preferences();
        self.rows = rows;
        let objects = self.object_indices();
        self.selected = preferred
            .iter()
            .find_map(|oid| {
                objects
                    .iter()
                    .find(|index| self.rows[**index].oid() == Some(oid))
                    .copied()
            })
            .unwrap_or_else(|| objects.first().copied().unwrap_or(0));
    }

    /// The oid under the cursor, then every one below it in the old order, then every one above it
    /// in reverse, which is the order the cursor falls back through.
    fn preferences(&self) -> Vec<Oid> {
        let objects = self.object_indices();
        let at = objects.iter().position(|index| *index == self.selected);
        let Some(at) = at else {
            return Vec::new();
        };
        let oid = |index: &usize| self.rows[*index].oid().cloned();
        objects[at..]
            .iter()
            .filter_map(oid)
            .chain(objects[..at].iter().rev().filter_map(oid))
            .collect()
    }

    fn object_indices(&self) -> Vec<usize> {
        self.rows
            .iter()
            .enumerate()
            .filter(|(_, row)| row.oid().is_some())
            .map(|(index, _)| index)
            .collect()
    }
}
```

Add to `crates/herdr-damnit-domain/src/lib.rs`:

```rust
mod cursor;

pub use cursor::Cursor;
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p herdr-damnit-domain --locked`
Expected: PASS, six new tests.

- [ ] **Step 5: Commit**

```bash
git add -A
SKIP_AI_COMMIT=1 git commit -m "feat(domain): carry the cursor by oid across a re-read"
```

---

### Task 10: The views, as dam queries

**Files:**
- Create: `crates/herdr-damnit-domain/src/views.rs`
- Modify: `crates/herdr-damnit-domain/src/lib.rs`

**Interfaces:**
- Consumes: the domain crate.
- Produces:

```rust
pub const OPEN: &str = "open";
pub const OPEN_QUERY: &str = "!done";
pub const DONE_QUERY: &str = "done";
pub const MAX_NUMBERED_VIEW: usize = 9;

pub struct View { pub name: String, pub query: String }

pub struct Views { /* the views and the showing index */ }

impl Views {
    pub fn new(configured: &[View]) -> Views;
    pub fn current(&self) -> &View;
    pub fn names(&self) -> impl Iterator<Item = &str>;
    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;
    pub fn showing(&self) -> usize;
    pub fn select(&mut self, index: usize) -> bool;
    pub fn select_named(&mut self, name: &str) -> bool;
    pub fn name_of_number(&self, number: usize) -> Option<&str>;
}
```

View 1 is always present and is the query `!done`, because `dam ls` with no query returns every
object, completed ones included. Its name is `open`, which is also the word the status line counts in.

- [ ] **Step 1: Write the failing tests**

`crates/herdr-damnit-domain/src/views.rs`, inside `mod tests`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn view(name: &str, query: &str) -> View {
        View {
            name: name.to_string(),
            query: query.to_string(),
        }
    }

    #[test]
    fn view_one_is_the_open_list_because_dam_ls_with_no_query_includes_done_objects() {
        let views = Views::new(&[]);
        assert_eq!(views.len(), 1);
        assert!(!views.is_empty());
        assert_eq!(views.current().name, OPEN);
        assert_eq!(views.current().query, OPEN_QUERY);
    }

    #[test]
    fn the_configured_views_follow_it_in_the_order_they_were_written() {
        let views = Views::new(&[
            view("today", "!done & due:today"),
            view("deep", "effort:deep"),
        ]);
        assert_eq!(
            views.names().collect::<Vec<_>>(),
            vec!["open", "today", "deep"]
        );
        assert_eq!(views.name_of_number(1), Some("open"));
        assert_eq!(views.name_of_number(3), Some("deep"));
        assert_eq!(views.name_of_number(4), None);
        assert_eq!(views.name_of_number(0), None);
    }

    #[test]
    fn selecting_reports_whether_the_showing_view_actually_changed() {
        let mut views = Views::new(&[view("today", "!done & due:today")]);
        assert!(views.select(1));
        assert_eq!(views.showing(), 1);
        assert!(!views.select(1), "it reported a change that did not happen");
        assert!(!views.select(9), "it selected a view that is not there");
        assert!(views.select_named("open"));
        assert!(!views.select_named("nowhere"));
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p herdr-damnit-domain --locked views`
Expected: FAIL with `unresolved module or unlinked crate 'views'`

- [ ] **Step 3: Write the module**

`crates/herdr-damnit-domain/src/views.rs`, above its test module:

```rust
//! The views the pane can show: the open list first, then the named queries from the config.
//! Numbering, selection and the picker are pure functions over that list.

/// The name of the view every pane has, whatever the config says.
pub const OPEN: &str = "open";

/// `dam ls` with no query returns every object, completed ones included, so the open list is a
/// query rather than an absent one.
pub const OPEN_QUERY: &str = "!done";

/// The query the Done screen reads.
pub const DONE_QUERY: &str = "done";

/// The highest view number a number key or a `view:<n>` action reaches. herdr declares plugin
/// actions in the manifest with no runtime registration, so the manifest carries exactly this
/// many numbered actions.
pub const MAX_NUMBERED_VIEW: usize = 9;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct View {
    pub name: String,
    pub query: String,
}

#[derive(Debug)]
pub struct Views {
    views: Vec<View>,
    current: usize,
}

impl Views {
    pub fn new(configured: &[View]) -> Self {
        let mut views = vec![View {
            name: OPEN.to_string(),
            query: OPEN_QUERY.to_string(),
        }];
        views.extend(configured.iter().cloned());
        Self { views, current: 0 }
    }

    pub fn current(&self) -> &View {
        &self.views[self.current]
    }

    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.views.iter().map(|view| view.name.as_str())
    }

    pub fn len(&self) -> usize {
        self.views.len()
    }

    pub fn is_empty(&self) -> bool {
        false
    }

    pub fn showing(&self) -> usize {
        self.current
    }

    /// Show the view at `index`, reporting whether the showing view changed.
    pub fn select(&mut self, index: usize) -> bool {
        if index >= self.views.len() {
            return false;
        }
        let changed = index != self.current;
        self.current = index;
        changed
    }

    pub fn select_named(&mut self, name: &str) -> bool {
        match self.views.iter().position(|view| view.name == name) {
            Some(index) => self.select(index),
            None => false,
        }
    }

    /// The name of the nth view, counting from 1 the way the number keys and the `view` actions do.
    pub fn name_of_number(&self, number: usize) -> Option<&str> {
        number
            .checked_sub(1)
            .and_then(|index| self.views.get(index))
            .map(|view| view.name.as_str())
    }
}
```

Add to `crates/herdr-damnit-domain/src/lib.rs`:

```rust
mod views;

pub use views::{DONE_QUERY, MAX_NUMBERED_VIEW, OPEN, OPEN_QUERY, View, Views};
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p herdr-damnit-domain --locked`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add -A
SKIP_AI_COMMIT=1 git commit -m "feat(domain): number the views over dam queries with the open list first"
```

---

### Task 11: The staging model

**Files:**
- Create: `crates/herdr-damnit-domain/src/stage.rs`
- Create: `crates/herdr-damnit-domain/src/stage/tests.rs`
- Modify: `crates/herdr-damnit-domain/src/lib.rs`

**Interfaces:**
- Consumes: `Oid`, `Mark`, `StagingMarks` from Tasks 4, 6 and 8.
- Produces:

```rust
pub enum Op { Create, Update, Delete }

pub struct Change { pub oid: Oid, pub op: Op, pub subject: String, pub fields: Vec<String> }
pub struct Unpushed { pub remote: String, pub commits: u64, pub oids: Vec<Oid> }
pub struct Conflict { pub oid: Oid, pub remote: String, pub ours: String, pub theirs: String }
pub struct Notice { pub kind: String, pub oid: Option<Oid>, pub remote: Option<String>, pub message: String }

pub struct Stage {
    pub staged: Vec<Change>,
    pub unstaged: Vec<Change>,
    pub unpushed: Vec<Unpushed>,
    pub conflicts: Vec<Conflict>,
    pub notices: Vec<Notice>,
}

pub enum StatusRow {
    Heading(String),
    Change { oid: Oid, mark: Mark, text: String },
    Line { mark: Option<Mark>, text: String },
}

impl Stage {
    pub fn is_clean(&self) -> bool;
    pub fn is_staged(&self, oid: &Oid) -> bool;
    pub fn staged_count(&self) -> usize;
    pub fn is_unpushed(&self, oid: &Oid) -> bool;
    pub fn unpushed_commits(&self) -> u64;
    pub fn rows(&self) -> Vec<StatusRow>;
    pub fn summary(&self) -> String;
}

impl StagingMarks for Stage { fn mark_of(&self, oid: &Oid) -> Option<Mark> }
```

`mark_of` resolves in one order, most urgent first: a conflict beats staged, staged beats working,
and working beats unpushed, which is the whole ladder now that an unpushed row names the objects
its commits touch. An `Unpushed` row carries `oids`, the objects whose changes sit in that remote's
unpushed commits, and the row a list draws for one of them leads with the up arrow. A `dam` that
does not publish them leaves the set empty, which costs those rows their mark and nothing else.
`Stage::rows` draws the four sections the spec names, in `dam`'s own order, leaving an
empty section out; an entirely empty stage is one line reading `nothing staged, nothing changed`,
which is `dam`'s own wording.

**The Unpushed row carries the up arrow the spec's mock draws.** Every other row of that screen now
takes its mark from a field, so the one plain row that shows one takes it the same way rather than
leaving the renderer to know which heading it sits under: `StatusRow::Line` carries an
`Option<Mark>`, `Some(Mark::Unpushed)` on a remote's row and `None` everywhere else.

**A notice that names an object draws that object and is a cursor target.** The spec's Status mock
draws a notice as `! 7a8b9c0  removed on todoist: ...`, with a mark and the short oid, and the
sentence under the mock makes every row that names an oid a cursor target. The four kinds that name
one (`removed_upstream`, `push_failed`, `event_cancelled`, `kind_changed`) therefore draw as a
`StatusRow::Change` under `Mark::Notice` from Task 6; `pull_failed` is about a remote rather than an
object, carries no oid, draws its message alone and is not a target. The pinned target count over
the fixture is the number of rows that name an object, which is five.

`Stage::summary` counts the arrays it is handed, so over the `full()` fixture below, which holds
two staged changes and one unstaged one, it reads `2 staged  1 changed  1 unpushed  1 notice`. The
spec's Status screen mockup heads the same screen `3 staged  2 changed`; that header is stale
against the two Staged rows and one Working row drawn under it, and the arrays win.

Three of the fixture's oids run past seven characters, one per row builder that calls `Oid::short`,
so a row drawn from the whole oid instead of its prefix fails rather than reading the same either
way.

`Change::fields` is the list of changed field names, which `dam` 0.2.0 carries on every change
document as `fields` and Task 23 maps straight across. An update names the fields that moved, a
create names the fields the new object carries beyond its defaults, and a delete names none, so a
deleted row draws with no parenthesis.

- [ ] **Step 1: Write the failing tests**

`crates/herdr-damnit-domain/src/stage/tests.rs`:

```rust
use super::*;

fn change(oid: &str, op: Op, subject: &str, fields: &[&str]) -> Change {
    Change {
        oid: Oid::new(oid),
        op,
        subject: subject.to_string(),
        fields: fields.iter().map(|field| field.to_string()).collect(),
    }
}

fn full() -> Stage {
    Stage {
        staged: vec![
            change("1a2b3c4", Op::Create, "ship the pin bump", &[]),
            change(
                "5d6e7f8a9b0",
                Op::Update,
                "refresh the roster row",
                &["due", "priority"],
            ),
        ],
        unstaged: vec![change(
            "9a0b1c2",
            Op::Update,
            "water the plants",
            &["subject"],
        )],
        unpushed: vec![
            Unpushed {
                remote: "todoist".to_string(),
                commits: 1,
                oids: Vec::new(),
            },
            Unpushed {
                remote: "work".to_string(),
                commits: 2,
                oids: vec![Oid::new("c3d4e5f")],
            },
        ],
        conflicts: vec![Conflict {
            oid: Oid::new("3d4e5f6a1b2"),
            remote: "todoist".to_string(),
            ours: "mine".to_string(),
            theirs: "theirs".to_string(),
        }],
        notices: vec![Notice {
            kind: "removed_upstream".to_string(),
            oid: Some(Oid::new("7a8b9c0d1e2")),
            remote: Some("todoist".to_string()),
            message: "removed on todoist: \"old task\" is kept here".to_string(),
        }],
    }
}

fn drawn(stage: &Stage) -> Vec<String> {
    stage
        .rows()
        .iter()
        .map(|row| match row {
            StatusRow::Heading(text) => text.clone(),
            StatusRow::Line { text, .. } | StatusRow::Change { text, .. } => text.clone(),
        })
        .collect()
}

#[test]
fn the_four_sections_are_drawn_in_dams_own_order() {
    assert_eq!(
        drawn(&full()),
        vec![
            "Staged".to_string(),
            "  new      1a2b3c4  ship the pin bump".to_string(),
            "  changed  5d6e7f8  refresh the roster row  (due, priority)".to_string(),
            "Working".to_string(),
            "  changed  9a0b1c2  water the plants  (subject)".to_string(),
            "Unpushed".to_string(),
            "  todoist  1 commit".to_string(),
            "  work  2 commits".to_string(),
            "Notices".to_string(),
            "  3d4e5f6  todoist  ours: \"mine\"  theirs: \"theirs\"".to_string(),
            "  7a8b9c0  removed on todoist: \"old task\" is kept here".to_string(),
        ]
    );
}

#[test]
fn an_empty_section_is_left_out_entirely() {
    let stage = Stage {
        unstaged: Vec::new(),
        unpushed: Vec::new(),
        conflicts: Vec::new(),
        notices: Vec::new(),
        ..full()
    };
    assert!(
        !stage.is_clean(),
        "a stage with staged changes is not clean"
    );
    let drawn = drawn(&stage);
    assert_eq!(drawn.first().map(String::as_str), Some("Staged"));
    assert!(!drawn.iter().any(|line| line == "Working"), "{drawn:?}");
    assert!(!drawn.iter().any(|line| line == "Notices"), "{drawn:?}");
}

#[test]
fn an_empty_stage_says_so_in_dams_own_words() {
    let stage = Stage {
        staged: Vec::new(),
        unstaged: Vec::new(),
        unpushed: Vec::new(),
        conflicts: Vec::new(),
        notices: Vec::new(),
    };
    assert!(stage.is_clean());
    assert!(!full().is_clean());
    assert_eq!(
        stage.rows(),
        vec![StatusRow::Line {
            mark: None,
            text: "nothing staged, nothing changed".to_string(),
        }]
    );
}

#[test]
fn a_conflict_outranks_staged_and_staged_outranks_working() {
    let stage = full();
    assert_eq!(stage.staged_count(), 2);
    assert_eq!(
        stage.mark_of(&Oid::new("3d4e5f6a1b2")),
        Some(Mark::Conflict)
    );
    assert_eq!(stage.mark_of(&Oid::new("1a2b3c4")), Some(Mark::Staged));
    assert_eq!(stage.mark_of(&Oid::new("9a0b1c2")), Some(Mark::Working));
    assert_eq!(stage.mark_of(&Oid::new("nothing")), None);
}

#[test]
fn an_object_only_in_an_unpushed_commit_carries_the_unpushed_mark() {
    assert_eq!(full().mark_of(&Oid::new("c3d4e5f")), Some(Mark::Unpushed));
}

#[test]
fn a_working_change_outranks_an_unpushed_commit() {
    let mut stage = full();
    stage.unpushed[1].oids.push(Oid::new("9a0b1c2"));
    assert_eq!(stage.mark_of(&Oid::new("9a0b1c2")), Some(Mark::Working));
}

#[test]
fn a_dam_that_sends_no_oids_leaves_the_unpushed_set_empty() {
    let stage = Stage {
        unpushed: vec![Unpushed {
            remote: "work".to_string(),
            commits: 2,
            oids: Vec::new(),
        }],
        ..full()
    };
    assert_eq!(stage.mark_of(&Oid::new("c3d4e5f")), None);
}

#[test]
fn an_object_both_staged_and_in_conflict_shows_the_conflict_mark() {
    let mut stage = full();
    stage.staged.push(change(
        "3d4e5f6a1b2",
        Op::Update,
        "the conflicted one",
        &["due"],
    ));
    assert_eq!(
        stage.mark_of(&Oid::new("3d4e5f6a1b2")),
        Some(Mark::Conflict)
    );
}

#[test]
fn an_object_both_staged_and_changed_again_shows_the_staged_mark() {
    let mut stage = full();
    stage.unstaged.push(change(
        "1a2b3c4",
        Op::Update,
        "ship the pin bump",
        &["body"],
    ));
    assert_eq!(stage.mark_of(&Oid::new("1a2b3c4")), Some(Mark::Staged));
}

#[test]
fn every_row_naming_an_oid_is_a_cursor_target() {
    let targets = full()
        .rows()
        .into_iter()
        .filter(|row| matches!(row, StatusRow::Change { .. }))
        .count();
    assert_eq!(
        targets, 5,
        "staged two, working one, conflict one, notice one"
    );
}

/// The spec's Status mock leads the unpushed row with the up arrow, and every other plain row with
/// nothing, so the mark is a field rather than a character the renderer has to know to add.
#[test]
fn the_unpushed_row_carries_its_own_mark_and_no_other_plain_row_does() {
    let mut stage = full();
    stage.notices.push(Notice {
        kind: "pull_failed".to_string(),
        oid: None,
        remote: Some("todoist".to_string()),
        message: "pull failed on todoist: the service did not answer".to_string(),
    });

    let marks: Vec<Option<Mark>> = stage
        .rows()
        .into_iter()
        .filter_map(|row| match row {
            StatusRow::Line { mark, .. } => Some(mark),
            _ => None,
        })
        .collect();

    assert_eq!(
        marks,
        vec![Some(Mark::Unpushed), Some(Mark::Unpushed), None],
        "two unpushed remotes and one notice about no object"
    );
}

#[test]
fn a_notice_about_no_object_in_particular_draws_its_message_alone() {
    let stage = Stage {
        notices: vec![Notice {
            kind: "pull_failed".to_string(),
            oid: None,
            remote: Some("todoist".to_string()),
            message: "pull failed on todoist: the service did not answer".to_string(),
        }],
        ..full()
    };

    assert!(
        drawn(&stage).contains(&"  pull failed on todoist: the service did not answer".to_string()),
        "{:?}",
        drawn(&stage)
    );
    assert_eq!(
        stage
            .rows()
            .into_iter()
            .filter(|row| matches!(row, StatusRow::Change { .. }))
            .count(),
        4,
        "a notice naming no object is not a cursor target"
    );
}

#[test]
fn a_notice_that_names_an_object_carries_its_mark() {
    let marked = full()
        .rows()
        .into_iter()
        .find(|row| matches!(row, StatusRow::Change { oid, .. } if oid.as_str() == "7a8b9c0d1e2"));
    let Some(StatusRow::Change { mark, .. }) = marked else {
        panic!("the notice row is not a cursor target");
    };
    assert_eq!(mark, Mark::Notice);
}

#[test]
fn a_conflict_value_is_quoted_without_leaking_rust_escaping() {
    let stage = Stage {
        conflicts: vec![Conflict {
            oid: Oid::new("3d4e5f6"),
            remote: "todoist".to_string(),
            ours: "say \"hi\"".to_string(),
            theirs: "a\\b".to_string(),
        }],
        ..full()
    };

    assert!(
        drawn(&stage)
            .contains(&"  3d4e5f6  todoist  ours: \"say \"hi\"\"  theirs: \"a\\b\"".to_string()),
        "{:?}",
        drawn(&stage)
    );
}

#[test]
fn a_conflict_value_that_spans_lines_still_draws_as_one_row() {
    let stage = Stage {
        conflicts: vec![Conflict {
            oid: Oid::new("3d4e5f6"),
            remote: "todoist".to_string(),
            ours: "line\nbreak".to_string(),
            theirs: "carriage\rreturn".to_string(),
        }],
        ..full()
    };

    assert!(
        drawn(&stage)
            .iter()
            .all(|line| !line.contains('\n') && !line.contains('\r')),
        "{:?}",
        drawn(&stage)
    );
    assert!(
        drawn(&stage).contains(
            &"  3d4e5f6  todoist  ours: \"line break\"  theirs: \"carriage return\"".to_string()
        ),
        "{:?}",
        drawn(&stage)
    );
}

#[test]
fn the_summary_counts_what_the_status_line_carries() {
    assert_eq!(
        full().summary(),
        "2 staged  1 changed  3 unpushed  1 notice"
    );
    assert_eq!(
        full().unpushed_commits(),
        3,
        "two remotes, one and two commits"
    );
}

#[test]
fn one_commit_and_two_commits_are_both_spelled_correctly() {
    let one = Stage {
        unpushed: vec![Unpushed {
            remote: "todoist".to_string(),
            commits: 1,
            oids: Vec::new(),
        }],
        ..full()
    };
    let two = Stage {
        unpushed: vec![Unpushed {
            remote: "todoist".to_string(),
            commits: 2,
            oids: Vec::new(),
        }],
        ..full()
    };
    assert!(drawn(&one).contains(&"  todoist  1 commit".to_string()));
    assert!(drawn(&two).contains(&"  todoist  2 commits".to_string()));
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p herdr-damnit-domain --locked stage`
Expected: FAIL with `unresolved module or unlinked crate 'stage'`

- [ ] **Step 3: Write the module**

`crates/herdr-damnit-domain/src/stage.rs`, whose types are the block in the Interfaces section, each
deriving `Clone`, `Debug`, `PartialEq` and `Eq`, plus:

```rust
//! The staging model `dam status` reports, and the Status screen drawn from it. A section with
//! nothing in it is left out, and an entirely empty stage says so in `dam`'s own words.

use crate::{Mark, Oid, StagingMarks};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Op {
    Create,
    Update,
    Delete,
}

/// One object `dam` reports as changed, and the field names the change touches.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Change {
    pub oid: Oid,
    pub op: Op,
    pub subject: String,
    pub fields: Vec<String>,
}

/// How far one remote is behind the local commits, and which objects those commits touch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Unpushed {
    pub remote: String,
    pub commits: u64,
    /// The objects whose changes sit in this remote's unpushed commits. Empty when the `dam` that
    /// answered does not publish them, which costs the rows their unpushed mark and nothing else.
    pub oids: Vec<Oid>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Conflict {
    pub oid: Oid,
    pub remote: String,
    pub ours: String,
    pub theirs: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Notice {
    pub kind: String,
    pub oid: Option<Oid>,
    pub remote: Option<String>,
    pub message: String,
}

/// The five arrays `dam status --json` answers with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stage {
    pub staged: Vec<Change>,
    pub unstaged: Vec<Change>,
    pub unpushed: Vec<Unpushed>,
    pub conflicts: Vec<Conflict>,
    pub notices: Vec<Notice>,
}

/// One line of the Status screen. A `Change` row names an oid and is therefore a cursor target; a
/// `Line` names none, and carries a mark only where the screen draws one beside it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StatusRow {
    Heading(String),
    Change { oid: Oid, mark: Mark, text: String },
    Line { mark: Option<Mark>, text: String },
}

impl Stage {
    pub fn is_clean(&self) -> bool {
        self.staged.is_empty()
            && self.unstaged.is_empty()
            && self.unpushed.iter().all(|remote| remote.commits == 0)
            && self.conflicts.is_empty()
            && self.notices.is_empty()
    }

    pub fn is_staged(&self, oid: &Oid) -> bool {
        self.staged.iter().any(|change| &change.oid == oid)
    }

    pub fn staged_count(&self) -> usize {
        self.staged.len()
    }

    /// Whether an object's changes sit in some remote's unpushed commits.
    pub fn is_unpushed(&self, oid: &Oid) -> bool {
        self.unpushed
            .iter()
            .any(|remote| remote.oids.iter().any(|unpushed| unpushed == oid))
    }

    pub fn unpushed_commits(&self) -> u64 {
        self.unpushed.iter().map(|remote| remote.commits).sum()
    }

    pub fn summary(&self) -> String {
        format!(
            "{} staged  {} changed  {} unpushed  {} notice{}",
            self.staged.len(),
            self.unstaged.len(),
            self.unpushed_commits(),
            self.notices.len(),
            if self.notices.len() == 1 { "" } else { "s" }
        )
    }

    pub fn rows(&self) -> Vec<StatusRow> {
        let mut rows = Vec::new();
        section(&mut rows, "Staged", &self.staged, Mark::Staged);
        section(&mut rows, "Working", &self.unstaged, Mark::Working);
        if self.unpushed.iter().any(|remote| remote.commits > 0) {
            rows.push(StatusRow::Heading("Unpushed".to_string()));
            for remote in self.unpushed.iter().filter(|remote| remote.commits > 0) {
                rows.push(StatusRow::Line {
                    mark: Some(Mark::Unpushed),
                    text: format!(
                        "  {}  {} commit{}",
                        remote.remote,
                        remote.commits,
                        if remote.commits == 1 { "" } else { "s" }
                    ),
                });
            }
        }
        if !self.conflicts.is_empty() || !self.notices.is_empty() {
            rows.push(StatusRow::Heading("Notices".to_string()));
            for conflict in &self.conflicts {
                rows.push(StatusRow::Change {
                    oid: conflict.oid.clone(),
                    mark: Mark::Conflict,
                    text: format!(
                        "  {}  {}  ours: \"{}\"  theirs: \"{}\"",
                        conflict.oid.short(),
                        conflict.remote,
                        one_line(&conflict.ours),
                        one_line(&conflict.theirs)
                    ),
                });
            }
            rows.extend(self.notices.iter().map(Notice::row));
        }
        if rows.is_empty() {
            rows.push(StatusRow::Line {
                mark: None,
                text: "nothing staged, nothing changed".to_string(),
            });
        }
        rows
    }
}

/// A conflicting value as one row can carry it. The quotes are the spec's; a newline becomes a
/// space so a multi-line value cannot break the row it is drawn in.
fn one_line(value: &str) -> String {
    value.replace(['\n', '\r'], " ")
}

fn section(rows: &mut Vec<StatusRow>, heading: &str, changes: &[Change], mark: Mark) {
    if changes.is_empty() {
        return;
    }
    rows.push(StatusRow::Heading(heading.to_string()));
    rows.extend(changes.iter().map(|change| StatusRow::Change {
        oid: change.oid.clone(),
        mark,
        text: change.line(),
    }));
}

impl Change {
    /// One change as the Status screen draws it, in `dam`'s own column shape.
    fn line(&self) -> String {
        let word = match self.op {
            Op::Create => "new     ",
            Op::Update => "changed ",
            Op::Delete => "removed ",
        };
        let fields = if self.fields.is_empty() {
            String::new()
        } else {
            format!("  ({})", self.fields.join(", "))
        };
        format!("  {word} {}  {}{fields}", self.oid.short(), self.subject)
    }
}

impl Notice {
    /// A notice that names an object is a cursor target, the way a conflict is. One about a remote
    /// rather than an object, such as a failed pull, names none and draws its message alone.
    fn row(&self) -> StatusRow {
        match &self.oid {
            Some(oid) => StatusRow::Change {
                oid: oid.clone(),
                mark: Mark::Notice,
                text: format!("  {}  {}", oid.short(), self.message),
            },
            None => StatusRow::Line {
                mark: None,
                text: format!("  {}", self.message),
            },
        }
    }
}

impl StagingMarks for Stage {
    /// Most urgent first: a conflict beats staged, staged beats working, and working beats
    /// unpushed.
    fn mark_of(&self, oid: &Oid) -> Option<Mark> {
        if self.conflicts.iter().any(|conflict| &conflict.oid == oid) {
            return Some(Mark::Conflict);
        }
        if self.is_staged(oid) {
            return Some(Mark::Staged);
        }
        if self.unstaged.iter().any(|change| &change.oid == oid) {
            return Some(Mark::Working);
        }
        if self.is_unpushed(oid) {
            return Some(Mark::Unpushed);
        }
        None
    }
}

#[cfg(test)]
mod tests;
```

Add to `crates/herdr-damnit-domain/src/lib.rs`:

```rust
mod stage;

pub use stage::{Change, Conflict, Notice, Op, Stage, StatusRow, Unpushed};
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p herdr-damnit-domain --locked`
Expected: PASS.

- [ ] **Step 5: Check the file is inside the cap**

Run: `wc -l crates/herdr-damnit-domain/src/stage.rs crates/herdr-damnit-domain/src/stage/tests.rs`
Expected: both under 300. If `stage.rs` is over 300, move the `rows` builder and `section` into
`crates/herdr-damnit-domain/src/stage/screen.rs` and re-export.

- [ ] **Step 6: Commit**

```bash
git add -A
SKIP_AI_COMMIT=1 git commit -m "feat(domain): model dam's staging report and the status screen it draws"
```

---

### Task 12: The agent brief

**Files:**
- Create: `crates/herdr-damnit-domain/src/brief.rs`
- Modify: `crates/herdr-damnit-domain/src/lib.rs`
- Reference: `crates/herdr-damnit/src/send.rs`, whose `brief` and `with_note` this replaces

**Interfaces:**
- Consumes: `Object`, `long` from Tasks 5 and 7.
- Produces: `brief(object: &Object, note: &str) -> String`.

The brief is plain text because an agent pane is a shell. There is no URL, because a `dam` object has
none: the oid takes its place, and it is what `dam show` and every other client accept. A field the
object has nothing for is left out rather than written empty.

- [ ] **Step 1: Write the failing tests**

`crates/herdr-damnit-domain/src/brief.rs`, inside `mod tests`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Kind, Oid, Priority, TaskFields, parse_date};

    fn task() -> Object {
        Object {
            oid: Oid::new("1a2b3c4d"),
            kind: Kind::Task,
            subject: "file taxes".to_string(),
            body: "receipts are in the drawer".to_string(),
            path: "home/admin/".to_string(),
            labels: vec!["home".to_string(), "slow".to_string()],
            depends: Vec::new(),
            recurrence: None,
            task: Some(TaskFields {
                done: false,
                priority: Priority::new(1).expect("a priority"),
                due: parse_date("2026-09-20"),
                deadline: None,
                attached: None,
            }),
            event: None,
        }
    }

    #[test]
    fn the_brief_names_the_task_by_its_oid_and_carries_the_note_last() {
        assert_eq!(
            brief(&task(), "start with the receipts"),
            "dam task: file taxes\n\
             oid: 1a2b3c4d\n\
             path: home/admin/\n\
             due: 2026-09-20\n\
             priority: p1\n\
             labels: home, slow\n\
             \n\
             receipts are in the drawer\n\
             \n\
             note: start with the receipts"
        );
    }

    #[test]
    fn a_field_the_object_has_nothing_for_is_left_out_rather_than_written_empty() {
        let bare = Object {
            body: String::new(),
            path: String::new(),
            labels: Vec::new(),
            task: Some(TaskFields {
                done: false,
                priority: Priority::default(),
                due: None,
                deadline: None,
                attached: None,
            }),
            ..task()
        };

        assert_eq!(brief(&bare, ""), "dam task: file taxes\noid: 1a2b3c4d");
    }

    #[test]
    fn an_event_says_so_in_its_first_line() {
        let event = Object {
            kind: Kind::Event,
            task: None,
            body: String::new(),
            path: String::new(),
            labels: Vec::new(),
            ..task()
        };

        assert!(brief(&event, "").starts_with("dam event: file taxes\n"));
    }

    #[test]
    fn a_deadline_sits_under_the_due_date_when_the_task_carries_one() {
        let dated = Object {
            task: Some(TaskFields {
                deadline: parse_date("2026-10-01"),
                ..task().task.expect("a task")
            }),
            ..task()
        };

        assert!(
            brief(&dated, "").contains("due: 2026-09-20\ndeadline: 2026-10-01\n"),
            "{}",
            brief(&dated, "")
        );
    }

    #[test]
    fn a_blank_note_leaves_no_note_line_behind() {
        assert!(!brief(&task(), "   ").contains("note:"));
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p herdr-damnit-domain --locked brief`
Expected: FAIL with `unresolved module or unlinked crate 'brief'`

- [ ] **Step 3: Write the module**

`crates/herdr-damnit-domain/src/brief.rs`, above its test module:

```rust
//! The text `S` hands to the workspace's agent pane. Plain text, because an agent pane is a shell:
//! it is pasted into that pane's input and the operator presses return themselves.

use crate::{Kind, Object, Oid, long};

pub fn brief(object: &Object, note: &str) -> String {
    let kind = match object.kind {
        Kind::Task => "task",
        Kind::Event => "event",
    };
    let mut lines = vec![
        format!("dam {kind}: {}", object.subject),
        format!("oid: {}", object.oid),
    ];
    if !object.path.is_empty() {
        lines.push(format!("path: {}", object.path));
    }
    if let Some(due) = object.due() {
        lines.push(format!("due: {}", long(due)));
    }
    if let Some(deadline) = object.task.as_ref().and_then(|task| task.deadline) {
        lines.push(format!("deadline: {}", long(deadline)));
    }
    if !object.priority().is_lowest() {
        lines.push(format!("priority: p{}", object.priority().get()));
    }
    if !object.labels.is_empty() {
        lines.push(format!("labels: {}", object.labels.join(", ")));
    }
    let mut text = lines.join("\n");
    if !object.body.trim().is_empty() {
        text.push_str("\n\n");
        text.push_str(object.body.trim_end());
    }
    if !note.trim().is_empty() {
        text.push_str("\n\nnote: ");
        text.push_str(note.trim());
    }
    text
}
```

The unused `Oid` import is there because `object.oid` formats through `Display`; drop the import if
clippy reports it unused.

Add to `crates/herdr-damnit-domain/src/lib.rs`:

```rust
mod brief;

pub use brief::brief;
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p herdr-damnit-domain --locked && cargo clippy -p herdr-damnit-domain --all-targets --locked -- -D warnings`
Expected: PASS and clean.

- [ ] **Step 5: Commit**

```bash
git add -A
SKIP_AI_COMMIT=1 git commit -m "feat(domain): write the agent brief from a dam object"
```

---

### Task 13: The dam version rules

**Files:**
- Create: `crates/herdr-damnit-domain/src/version.rs`
- Modify: `crates/herdr-damnit-domain/src/lib.rs`

**Interfaces:**
- Consumes: the domain crate.
- Produces:

```rust
pub struct DamVersion { pub major: u32, pub minor: u32, pub patch: u32 }

pub const DAM_MINIMUM: DamVersion = DamVersion { major: 0, minor: 2, patch: 0 };
pub const DAM_KNOWN: DamVersion = DamVersion { major: 0, minor: 2, patch: 0 };
/// The version assumed to add `dam restore`, which is what the `!` key is gated on.
pub const DAM_RESTORE: DamVersion = DamVersion { major: 0, minor: 2, patch: 0 };

pub fn parse_version(line: &str) -> Option<DamVersion>;

pub enum Verdict { Fine, Warn(String), Refuse(String) }

pub fn verdict(found: DamVersion) -> Verdict;
```

`DAM_MINIMUM` and `DAM_KNOWN` are both 0.2.0, the version `dam` prints today (`crates/dam-cli`
declares `version = "0.2.0"` at `webdavis/damnit` `84937a3`). 0.2 is also the floor on its own
merits: the JSON error document, exit 4 for every refusal, and a change document with no embedded
object all arrived there, and Task 14 reads all three. `DAM_RESTORE` is 0.2.0 as
well: `dam restore` ships in that release (`crates/dam-cli/src/commands/restore.rs` at
`webdavis/damnit` `84937a3`, declared at `args.rs` `Restore(RestoreArgs)` and dispatched in
`commands/mod.rs`), so the gate is met by every `dam` the pane agrees to draw against. The gate
stays written and tested: the key it guards is destructive, and the day a floor moves is not the day
to rediscover that. Below 1.0 the minor is the breaking axis, so
`verdict` refuses below the minimum, warns when the minor is above the known one, and says nothing
in between.

- [ ] **Step 1: Write the failing tests**

`crates/herdr-damnit-domain/src/version.rs`, inside `mod tests`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn at(major: u32, minor: u32, patch: u32) -> DamVersion {
        DamVersion {
            major,
            minor,
            patch,
        }
    }

    #[test]
    fn dams_own_version_line_is_read() {
        assert_eq!(parse_version("dam 0.1.0"), Some(at(0, 1, 0)));
        assert_eq!(parse_version("dam 0.1.0\n"), Some(at(0, 1, 0)));
        assert_eq!(parse_version("dam 12.3.45"), Some(at(12, 3, 45)));
    }

    #[test]
    fn anything_that_is_not_a_version_line_is_no_version() {
        assert_eq!(parse_version(""), None);
        assert_eq!(parse_version("dam"), None);
        assert_eq!(parse_version("dam 0.1"), None);
        assert_eq!(parse_version("dam version one"), None);
        assert_eq!(parse_version("dam 0.1.0.4"), None);
    }

    #[test]
    fn a_dam_below_the_floor_is_refused_by_both_numbers() {
        let Verdict::Refuse(message) = verdict(at(0, 0, 9)) else {
            panic!("expected a refusal");
        };
        assert_eq!(
            message,
            "dam 0.0.9 is older than the 0.2 this pane needs; run cargo install damnit to update it."
        );
        assert!(
            matches!(verdict(at(0, 1, 9)), Verdict::Refuse(_)),
            "the error document and the exit codes this pane reads arrived in 0.2"
        );
    }

    #[test]
    fn a_newer_minor_draws_and_warns_once() {
        let Verdict::Warn(message) = verdict(at(0, 4, 0)) else {
            panic!("expected a warning");
        };
        assert_eq!(
            message,
            "dam 0.4.0 is newer than this pane knows; some keys may be refused."
        );
    }

    #[test]
    fn a_newer_patch_of_a_known_minor_says_nothing() {
        assert!(matches!(verdict(at(0, 2, 7)), Verdict::Fine));
        assert!(matches!(verdict(DAM_KNOWN), Verdict::Fine));
    }

    /// `dam restore` ships in the same 0.2.0 that sets the floor, so every `dam` the pane agrees to
    /// draw against clears the gate. The gate stays because the key it guards is destructive and
    /// the day a floor moves is not the day to rediscover that.
    #[test]
    fn every_dam_the_pane_accepts_clears_the_restore_gate() {
        assert!(DAM_MINIMUM >= DAM_RESTORE);
        assert!(at(0, 2, 0) >= DAM_RESTORE);
        assert!(at(0, 2, 9) >= DAM_RESTORE);
        assert!(at(0, 3, 0) >= DAM_RESTORE);
        assert!(at(0, 1, 9) < DAM_RESTORE, "and that dam is refused anyway");
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p herdr-damnit-domain --locked version`
Expected: FAIL with `unresolved module or unlinked crate 'version'`

- [ ] **Step 3: Write the module**

`crates/herdr-damnit-domain/src/version.rs`, above its test module:

```rust
//! The version of `dam` the pane was written against, and what to do about the one it found.
//! Below 1.0 the minor is the breaking axis, so that is the number the two rules compare.

/// The lowest version whose command surface this pane was written against. 0.2 is where the error
/// document, exit 4 for every refusal, and a change document with no embedded object arrived.
pub const DAM_MINIMUM: DamVersion = DamVersion {
    major: 0,
    minor: 2,
    patch: 0,
};

/// The highest version this pane was tested against.
pub const DAM_KNOWN: DamVersion = DamVersion {
    major: 0,
    minor: 2,
    patch: 0,
};

/// The version the discard key is gated on. `dam restore` ships in 0.2.0, the same release that
/// sets the floor, so the gate is met by every `dam` the pane agrees to draw against.
pub const DAM_RESTORE: DamVersion = DamVersion {
    major: 0,
    minor: 2,
    patch: 0,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct DamVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl std::fmt::Display for DamVersion {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Verdict {
    Fine,
    Warn(String),
    Refuse(String),
}

/// `dam --version` prints `dam <major>.<minor>.<patch>`, which is clap's standard line.
pub fn parse_version(line: &str) -> Option<DamVersion> {
    let number = line.trim().strip_prefix("dam ")?.trim();
    let mut parts = number.split('.');
    let mut next = || parts.next()?.parse::<u32>().ok();
    let version = DamVersion {
        major: next()?,
        minor: next()?,
        patch: next()?,
    };
    parts.next().is_none().then_some(version)
}

pub fn verdict(found: DamVersion) -> Verdict {
    if found < DAM_MINIMUM {
        return Verdict::Refuse(format!(
            "dam {found} is older than the {}.{} this pane needs; run cargo install damnit to \
             update it.",
            DAM_MINIMUM.major, DAM_MINIMUM.minor
        ));
    }
    if (found.major, found.minor) > (DAM_KNOWN.major, DAM_KNOWN.minor) {
        return Verdict::Warn(format!(
            "dam {found} is newer than this pane knows; some keys may be refused."
        ));
    }
    Verdict::Fine
}
```

Add to `crates/herdr-damnit-domain/src/lib.rs`:

```rust
mod version;

pub use version::{DAM_KNOWN, DAM_MINIMUM, DAM_RESTORE, DamVersion, Verdict, parse_version, verdict};
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p herdr-damnit-domain --locked`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add -A
SKIP_AI_COMMIT=1 git commit -m "feat(domain): compare the dam this pane found against the one it was built for"
```

---

### Task 14: The failure mapping

**Files:**
- Create: `crates/herdr-damnit-domain/src/failure.rs`
- Create: `crates/herdr-damnit-domain/src/failure/kind.rs`
- Create: `crates/herdr-damnit-domain/src/failure/rule.rs`
- Create: `crates/herdr-damnit-domain/src/failure/tests.rs`
- Modify: `crates/herdr-damnit-domain/src/lib.rs`

**Interfaces:**
- Consumes: `Oid` from Task 4.
- Produces:

```rust
pub enum Rule { /* dam's eighteen rule words, plus Unknown(String) */ }
pub enum ErrorKind { /* dam's eight kind words, plus Unknown(String) */ }

pub struct ErrorDocument {
    pub kind: ErrorKind,
    pub message: String,
    pub rule: Option<Rule>,
    pub oids: Vec<Oid>,
}

pub enum Failure {
    Refused { rule: Rule, said: String, oids: Vec<Oid> },
    Store(String),
    Other(String),
    Cancelled { killed: bool },
    Unreadable,
    NotInstalled,
    Deadline(String),
}

pub fn classify(code: Option<i32>, error: Option<ErrorDocument>, fallback: &str) -> Failure;
pub fn message(failure: &Failure) -> String;
pub fn leaves_model_untouched(failure: &Failure) -> bool;
```

The exit codes are `dam` 0.2.0's own: 0 did what it said, 1 every other failure including a dead
editor, 2 the command line alone, 3 cancelled, 4 a rule `dam` refused. **Every rule reports 4 and
nothing else does**, so the mapping reads the code once rather than per verb.

Under `--json` a failure is one document on standard error and nothing else, so the pane reads
`message` rather than stripping a prefix off a line. The adapters crate owns the parse, because the
domain crate takes no serde; `classify` is handed the parsed document. `fallback` is standard error's
first line and is used only when no document parsed: clap's usage text at exit 2, or a `dam` too old
to print one.

`rule` is what a key branches on when it does more than print. `Rule::Unknown` carries the word, so
a rule `dam` adds later reaches the status line as its message rather than being dropped.

A `Store` failure is an exit 1 whose document says `store` and whose message names the SQLite busy
timeout; it takes four extra words, because the usual cause is a long `dam pull` in another pane and
a retry loop would queue behind it. **Both halves are required.** `kind` of `store` covers every
store, config and io failure, so a disk error would otherwise be told to wait for a writer that is
not there, and the message alone would attach the advice to a helper quoting SQLite's sentence back.
A failure with no document at all is judged on its line, which is what a `dam` too old to print one
leaves behind.

- [ ] **Step 1: Write the failing tests**

`crates/herdr-damnit-domain/src/failure/tests.rs`, which is a file of its own because the module
and its tests together run past the 300 line ideal:

```rust
use super::*;

fn refusal(rule: &str, message: &str, oids: &[&str]) -> ErrorDocument {
    ErrorDocument {
        kind: ErrorKind::Refused,
        message: message.to_string(),
        rule: Some(Rule::named(rule)),
        oids: oids.iter().map(|oid| Oid::new(*oid)).collect(),
    }
}

#[test]
fn a_refusal_carries_dams_own_sentence_its_rule_and_its_oids() {
    let failure = classify(
        Some(4),
        Some(refusal(
            "blocked",
            "98d8780 cannot be completed: child a9db854 is open",
            &["98d8780", "a9db854"],
        )),
        "",
    );
    assert_eq!(
        failure,
        Failure::Refused {
            rule: Rule::Blocked,
            said: "98d8780 cannot be completed: child a9db854 is open".to_string(),
            oids: vec![Oid::new("98d8780"), Oid::new("a9db854")],
        }
    );
    assert_eq!(
        message(&failure),
        "98d8780 cannot be completed: child a9db854 is open"
    );
    assert!(leaves_model_untouched(&failure));
}

#[test]
fn every_rule_word_dam_publishes_has_a_variant_of_its_own() {
    for word in [
        "blocked",
        "cycle",
        "exclusive_label",
        "unknown_category",
        "no_such_object",
        "no_working_object",
        "no_such_remote",
        "not_a_task",
        "not_an_event",
        "not_completed",
        "not_committed",
        "dirty_on_pull",
        "move_inside_itself",
        "nothing_to_commit",
        "needs_an_answer",
        "needs_an_editor",
        "unresolved_conflicts",
        "missing_credential",
    ] {
        assert!(
            !matches!(Rule::named(word), Rule::Unknown(_)),
            "{word} has no variant"
        );
    }
}

#[test]
fn a_rule_this_pane_has_not_heard_of_keeps_its_word() {
    assert_eq!(
        Rule::named("invented_tomorrow"),
        Rule::Unknown("invented_tomorrow".to_string())
    );
}

#[test]
fn an_empty_stage_is_a_refusal_now_rather_than_an_ordinary_failure() {
    let failure = classify(
        Some(4),
        Some(refusal("nothing_to_commit", "nothing to commit", &[])),
        "",
    );
    assert!(leaves_model_untouched(&failure));
    assert_eq!(message(&failure), "nothing to commit");
}

#[test]
fn a_held_store_says_who_is_holding_it_and_what_to_press() {
    let failure = classify(
        Some(1),
        Some(ErrorDocument {
            kind: ErrorKind::Store,
            message: "database is locked".to_string(),
            rule: None,
            oids: Vec::new(),
        }),
        "",
    );
    assert_eq!(
        message(&failure),
        "database is locked; another dam is writing, press R to retry."
    );
    let without_document = classify(Some(1), None, "dam: database is locked\n");
    assert_eq!(
        message(&without_document),
        "database is locked; another dam is writing, press R to retry."
    );
}

/// clap answers a bad command line before `dam` runs, so there is no document to read.
#[test]
fn a_failure_whose_own_kind_is_not_the_store_takes_no_retry_advice() {
    let failure = classify(
        Some(1),
        Some(ErrorDocument {
            kind: ErrorKind::Helper,
            message: "the todoist helper says the database is locked".to_string(),
            rule: None,
            oids: Vec::new(),
        }),
        "",
    );
    assert_eq!(
        message(&failure),
        "the todoist helper says the database is locked"
    );
}

#[test]
fn a_store_failure_that_is_not_the_busy_timeout_takes_no_retry_advice() {
    let failure = classify(
        Some(1),
        Some(ErrorDocument {
            kind: ErrorKind::Store,
            message: "disk I/O error".to_string(),
            rule: None,
            oids: Vec::new(),
        }),
        "",
    );
    assert_eq!(message(&failure), "disk I/O error");
}

#[test]
fn a_command_line_dam_would_not_read_falls_back_to_its_first_line() {
    let failure = classify(
        Some(2),
        None,
        "error: unexpected argument '--nope'\n\nUsage: dam ls [QUERY]\n",
    );
    assert_eq!(message(&failure), "error: unexpected argument '--nope'");
    assert!(
        !leaves_model_untouched(&failure),
        "a bad command line is this pane's own bug, not a rule dam kept"
    );
}

#[test]
fn a_dam_too_old_to_print_a_document_still_reaches_the_status_line() {
    let failure = classify(Some(1), None, "dam: no object matches \"zzzzzzz\"\n");
    assert_eq!(message(&failure), "no object matches \"zzzzzzz\"");
}

#[test]
fn a_cancelled_run_is_never_an_error_banner() {
    assert_eq!(
        classify(Some(3), None, ""),
        Failure::Cancelled { killed: false }
    );
    assert!(leaves_model_untouched(&classify(Some(3), None, "")));
    assert_eq!(message(&classify(Some(3), None, "")), "cancelled");
    assert_eq!(
        message(&Failure::Cancelled { killed: true }),
        "cancelled (killed)"
    );
    assert!(leaves_model_untouched(&Failure::Cancelled {
        killed: false
    }));
}

#[test]
fn a_dam_that_is_not_there_says_how_to_get_one() {
    assert_eq!(
        message(&Failure::NotInstalled),
        "dam is not on PATH; install it with cargo install damnit, then press R."
    );
}

#[test]
fn output_that_will_not_parse_says_so_without_quoting_it() {
    assert_eq!(
        message(&Failure::Unreadable),
        "dam answered with something this pane could not read."
    );
}

#[test]
fn a_read_past_its_deadline_names_the_command_and_the_wait() {
    assert_eq!(
        message(&Failure::Deadline("dam ls".to_string())),
        "dam ls took longer than 30s and was cancelled."
    );
}

#[test]
fn a_signal_death_with_no_code_is_reported_rather_than_swallowed() {
    assert_eq!(
        message(&classify(None, None, "")),
        "dam was killed before it answered."
    );
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p herdr-damnit-domain --locked failure`
Expected: FAIL, the test module naming types that do not exist yet.

- [ ] **Step 3: Write the kind words**

`crates/herdr-damnit-domain/src/failure/kind.rs`:

```rust
//! The `kind` `dam` names on an error document. One word per kind, and the word itself for one
//! `dam` adds later, so a parser has somewhere to put a word this pane has not heard of.

/// Every kind word `dam` 0.2.0 publishes, and the word itself for one it adds later.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ErrorKind {
    Refused,
    Store,
    Helper,
    Credential,
    Parse,
    Usage,
    Cancelled,
    Editor,
    Unknown(String),
}

impl ErrorKind {
    pub fn named(word: &str) -> Self {
        match word {
            "refused" => Self::Refused,
            "store" => Self::Store,
            "helper" => Self::Helper,
            "credential" => Self::Credential,
            "parse" => Self::Parse,
            "usage" => Self::Usage,
            "cancelled" => Self::Cancelled,
            "editor" => Self::Editor,
            other => Self::Unknown(other.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_kind_word_dam_publishes_has_a_variant_of_its_own() {
        for word in [
            "refused",
            "store",
            "helper",
            "credential",
            "parse",
            "usage",
            "cancelled",
            "editor",
        ] {
            assert!(
                !matches!(ErrorKind::named(word), ErrorKind::Unknown(_)),
                "{word} has no variant"
            );
        }
    }

    #[test]
    fn a_kind_this_pane_has_not_heard_of_keeps_its_word_rather_than_failing_the_parse() {
        assert_eq!(
            ErrorKind::named("invented_tomorrow"),
            ErrorKind::Unknown("invented_tomorrow".to_string())
        );
    }
}
```

`dam`'s `kind()` emits eight words (`crates/dam-cli/src/error.rs`), `editor` among them, and
`Unknown` is what keeps a ninth reaching the status line with `dam`'s own sentence instead of being
dropped, the way `Rule::Unknown` does for a rule word.

- [ ] **Step 4: Write the rule words**

`crates/herdr-damnit-domain/src/failure/rule.rs`:

```rust
//! The rule `dam` names when it refuses. One stable snake_case word per rule, which is what lets a
//! key branch on the reason rather than on the sentence.

/// Every rule word `dam` 0.2.0 publishes, and the word itself for one it adds later.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Rule {
    Blocked,
    Cycle,
    ExclusiveLabel,
    UnknownCategory,
    NoSuchObject,
    NoWorkingObject,
    NoSuchRemote,
    NotATask,
    NotAnEvent,
    NotCompleted,
    NotCommitted,
    DirtyOnPull,
    MoveInsideItself,
    NothingToCommit,
    NeedsAnAnswer,
    NeedsAnEditor,
    UnresolvedConflicts,
    MissingCredential,
    Unknown(String),
}

impl Rule {
    pub fn named(word: &str) -> Self {
        match word {
            "blocked" => Self::Blocked,
            "cycle" => Self::Cycle,
            "exclusive_label" => Self::ExclusiveLabel,
            "unknown_category" => Self::UnknownCategory,
            "no_such_object" => Self::NoSuchObject,
            "no_working_object" => Self::NoWorkingObject,
            "no_such_remote" => Self::NoSuchRemote,
            "not_a_task" => Self::NotATask,
            "not_an_event" => Self::NotAnEvent,
            "not_completed" => Self::NotCompleted,
            "not_committed" => Self::NotCommitted,
            "dirty_on_pull" => Self::DirtyOnPull,
            "move_inside_itself" => Self::MoveInsideItself,
            "nothing_to_commit" => Self::NothingToCommit,
            "needs_an_answer" => Self::NeedsAnAnswer,
            "needs_an_editor" => Self::NeedsAnEditor,
            "unresolved_conflicts" => Self::UnresolvedConflicts,
            "missing_credential" => Self::MissingCredential,
            other => Self::Unknown(other.to_string()),
        }
    }
}
```

- [ ] **Step 5: Write the module**

`crates/herdr-damnit-domain/src/failure.rs`, above its test module:

```rust
//! What a non-zero `dam` exit means, and the one sentence the status line carries for it. Under
//! `--json` a failure is one document on standard error, so the text and the rule are `dam`'s own.

use crate::Oid;

mod kind;
mod rule;

pub use kind::ErrorKind;
pub use rule::Rule;

/// The read deadline the pane cancels a local read at. A SQLite read that takes this long is a
/// wedged store rather than a slow one.
pub const READ_DEADLINE_SECONDS: u64 = 30;

/// `dam`'s error document, parsed by the adapters crate and handed here.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ErrorDocument {
    pub kind: ErrorKind,
    pub message: String,
    /// The rule a refusal broke, and `None` for every other kind.
    pub rule: Option<Rule>,
    /// The objects the message names, in the order it names them.
    pub oids: Vec<Oid>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Failure {
    Refused {
        rule: Rule,
        said: String,
        oids: Vec<Oid>,
    },
    Store(String),
    Other(String),
    Cancelled {
        killed: bool,
    },
    Unreadable,
    NotInstalled,
    Deadline(String),
}

/// The exit code `dam` reports for a rule of its own, and for nothing else.
const REFUSED: i32 = 4;
/// The exit code `dam` reports when it was interrupted or a prompt went unanswered.
const CANCELLED: i32 = 3;

pub fn classify(code: Option<i32>, error: Option<ErrorDocument>, fallback: &str) -> Failure {
    let said = |error: &Option<ErrorDocument>| match error {
        Some(document) => document.message.clone(),
        None => first_line(fallback),
    };
    match (code, error) {
        (Some(REFUSED), Some(document)) => Failure::Refused {
            rule: document
                .rule
                .unwrap_or_else(|| Rule::Unknown(String::new())),
            said: document.message,
            oids: document.oids,
        },
        (Some(CANCELLED), _) => Failure::Cancelled { killed: false },
        (Some(_), error) if is_store(&error, fallback) => Failure::Store(said(&error)),
        (Some(_), error) => Failure::Other(said(&error)),
        (None, _) => Failure::Other(String::new()),
    }
}

pub fn message(failure: &Failure) -> String {
    match failure {
        Failure::Other(said) if said.is_empty() => "dam was killed before it answered.".to_string(),
        Failure::Refused { said, .. } | Failure::Other(said) => said.clone(),
        Failure::Store(said) => format!("{said}; another dam is writing, press R to retry."),
        Failure::Cancelled { killed: false } => "cancelled".to_string(),
        Failure::Cancelled { killed: true } => "cancelled (killed)".to_string(),
        Failure::Unreadable => "dam answered with something this pane could not read.".to_string(),
        Failure::NotInstalled => {
            "dam is not on PATH; install it with cargo install damnit, then press R.".to_string()
        }
        Failure::Deadline(command) => {
            format!("{command} took longer than {READ_DEADLINE_SECONDS}s and was cancelled.")
        }
    }
}

/// Whether the pane's model and any open prompt survive this failure untouched, which is what a
/// refused write leaves behind.
pub fn leaves_model_untouched(failure: &Failure) -> bool {
    matches!(
        failure,
        Failure::Refused { .. } | Failure::Cancelled { .. } | Failure::Deadline(_)
    )
}

fn first_line(stderr: &str) -> String {
    let line = stderr.lines().next().unwrap_or_default().trim();
    line.strip_prefix("dam: ").unwrap_or(line).to_string()
}

/// A held store is both halves `dam` reports: its own `store` kind, and SQLite's words for a
/// writer holding the store past the five-second busy timeout. A disk error under the same kind
/// takes no advice about waiting for another writer. A failure with no document is judged on its
/// line alone, which is what a `dam` too old to print one leaves behind.
fn is_store(error: &Option<ErrorDocument>, fallback: &str) -> bool {
    let said = match error {
        Some(document) => {
            if document.kind != ErrorKind::Store {
                return false;
            }
            document.message.as_str()
        }
        None => fallback,
    }
    .to_ascii_lowercase();
    said.contains("database is locked") || said.contains("database table is locked")
}

#[cfg(test)]
mod tests;
```

Add to `crates/herdr-damnit-domain/src/lib.rs`:

```rust
mod failure;

pub use failure::{
    ErrorDocument, ErrorKind, Failure, READ_DEADLINE_SECONDS, Rule, classify,
    leaves_model_untouched, message,
};
```

- [ ] **Step 6: Run the tests to verify they pass**

Run: `cargo test --workspace --locked && cargo clippy --workspace --all-targets --locked -- -D warnings`
Expected: PASS and clean.

- [ ] **Step 7: Commit**

```bash
git add -A
SKIP_AI_COMMIT=1 git commit -m "feat(domain): map a dam exit and its error document to one sentence"
```

---

## Phase C: the application crate

`herdr-damnit-application` owns the ports and the use cases. It names no process, no file and no
terminal: `ProcessDamRunner` in Phase D is what makes `DamRunner` real, and every use case here is
tested against a fake runner that records the argv it was handed and answers with a literal.

### Task 15: Create the application crate with its ports

**Files:**
- Create: `crates/herdr-damnit-application/Cargo.toml`
- Create: `crates/herdr-damnit-application/src/lib.rs`
- Create: `crates/herdr-damnit-application/src/ports.rs`
- Modify: `Cargo.toml`

**Interfaces:**
- Consumes: `herdr-damnit-domain`.
- Produces:

```rust
#[derive(Debug)]
pub struct Finished {
    pub code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub elapsed: std::time::Duration,
}

#[derive(Debug)]
pub enum SpawnError { NotFound, Io(String) }

/// A `dam` run in flight: the handle that signals it, and the one result it will send.
pub struct RunningJob {
    pub cancel: Box<dyn Fn() + Send + Sync>,
    pub results: std::sync::mpsc::Receiver<Finished>,
}

pub trait DamRunner: Send + Sync {
    fn spawn(&self, argv: &[String]) -> Result<RunningJob, SpawnError>;
}

pub trait Herdr: Send + Sync {
    fn call(&self, args: &[&str]) -> Result<String, String>;
}

pub trait Clock: Send + Sync {
    fn today(&self) -> herdr_damnit_domain::Date;
    fn now(&self) -> std::time::Instant;
}
```

`RunningJob::cancel` is a closure rather than a process id, because everything the adapter needs to
signal the child and its group stays inside the adapter, and the application crate never learns that
`dam` is a process at all.

Both derives are load-bearing. `spawn(...).expect(...)` in this task's own test needs
`SpawnError: Debug`, and Task 17's `Completion` derives `Debug` while carrying a `Finished`.
`RunningJob` derives nothing, because no caller asks it for anything.

- [ ] **Step 1: Write the failing test**

`crates/herdr-damnit-application/src/ports.rs`, inside `mod tests`. It is a compile-level test: it
proves a fake runner satisfies the port, which is the whole reason the port exists.

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use std::sync::mpsc::channel;

    struct FakeRunner {
        log: Mutex<Vec<Vec<String>>>,
    }

    impl DamRunner for FakeRunner {
        fn spawn(&self, argv: &[String]) -> Result<RunningJob, SpawnError> {
            self.log.lock().expect("the log").push(argv.to_vec());
            let (sender, results) = channel();
            sender
                .send(Finished {
                    code: Some(0),
                    stdout: "{}".to_string(),
                    stderr: String::new(),
                    elapsed: std::time::Duration::from_millis(9),
                })
                .expect("the receiver is alive");
            Ok(RunningJob {
                cancel: Box::new(|| {}),
                results,
            })
        }
    }

    #[test]
    fn a_runner_records_the_argv_it_was_handed_and_answers_once() {
        let runner = FakeRunner {
            log: Mutex::new(Vec::new()),
        };
        let job = runner
            .spawn(&["status".to_string(), "--json".to_string()])
            .expect("it spawned");

        assert_eq!(
            runner.log.lock().expect("the log").as_slice(),
            [vec!["status".to_string(), "--json".to_string()]]
        );
        assert_eq!(job.results.recv().expect("one result").code, Some(0));
    }
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p herdr-damnit-application --locked`
Expected: FAIL with `package ID specification 'herdr-damnit-application' did not match any packages`

- [ ] **Step 3: Create the crate**

`crates/herdr-damnit-application/Cargo.toml`:

```toml
[package]
name = "herdr-damnit-application"
description = "The pane's use cases over its ports: every dam argv, the job table and the handshake."
edition.workspace = true
version.workspace = true
license.workspace = true
repository.workspace = true

[dependencies]
herdr-damnit-domain.workspace = true
serde_json.workspace = true
```

In the workspace `Cargo.toml`, add `"crates/herdr-damnit-application"` to `members` and
`herdr-damnit-application = { path = "crates/herdr-damnit-application" }` to
`[workspace.dependencies]`, then add `herdr-damnit-application.workspace = true` to the binary
crate's `[dependencies]`.

`crates/herdr-damnit-application/src/lib.rs`:

```rust
//! The pane's use cases. Nothing here names a process, a file or a terminal.

mod ports;

pub use ports::{Clock, DamRunner, Finished, Herdr, RunningJob, SpawnError};
```

- [ ] **Step 4: Write the ports**

`crates/herdr-damnit-application/src/ports.rs`, above its test module, is the type block in the
Interfaces section verbatim, with this header:

```rust
//! What the pane needs from the world outside it: a way to run `dam`, a way to call `herdr`, and
//! a clock. Each has exactly one production implementation, in the adapters crate.
```

- [ ] **Step 5: Run the test to verify it passes**

Run: `cargo test -p herdr-damnit-application --locked`
Expected: PASS, one test.

- [ ] **Step 6: Commit**

```bash
git add -A
SKIP_AI_COMMIT=1 git commit -m "feat(application): add the application crate with the dam, herdr and clock ports"
```

---

### Task 16: Every dam argv the pane spawns

**Files:**
- Create: `crates/herdr-damnit-application/src/argv.rs`
- Create: `crates/herdr-damnit-application/src/argv/tests.rs`
- Modify: `crates/herdr-damnit-application/src/lib.rs`

**Interfaces:**
- Consumes: `Oid`, `Priority` from the domain crate.
- Produces one function per `dam` command, each returning the arguments that follow the configured
  `dam` argv, so a test compares a whole line and a flag typo fails it:

```rust
pub fn version() -> Vec<String>;
pub fn list(query: &str) -> Vec<String>;
pub fn status() -> Vec<String>;
pub fn show(oid: &Oid) -> Vec<String>;
pub fn log() -> Vec<String>;
pub fn stage(oid: &Oid) -> Vec<String>;
pub fn stage_all() -> Vec<String>;
pub fn unstage(oid: &Oid) -> Vec<String>;
pub fn unstage_all() -> Vec<String>;
pub fn commit(message: &str) -> Vec<String>;
pub fn push() -> Vec<String>;
pub fn pull() -> Vec<String>;
pub fn done(oid: &Oid, force: bool) -> Vec<String>;
pub fn remove(oid: &Oid) -> Vec<String>;
pub fn set_priority(oid: &Oid, priority: Priority) -> Vec<String>;
pub fn set_due(oid: &Oid, when: Option<&str>) -> Vec<String>;
pub fn set_deadline(oid: &Oid, when: Option<&str>) -> Vec<String>;
pub fn label(oid: &Oid, name: &str) -> Vec<String>;
pub fn unlabel(oid: &Oid, name: &str) -> Vec<String>;
pub fn move_to(oid: &Oid, path: &str) -> Vec<String>;
pub fn create(subject: &str, path: &str) -> Vec<String>;
pub fn edit_in_editor(oid: &Oid) -> Vec<String>;
pub enum Side { Ours, Theirs }
pub fn resolve(oid: &Oid, side: Side) -> Vec<String>;
pub fn restore(oid: &Oid) -> Vec<String>;
```

Every flag is `dam`'s own, read from `crates/dam-cli/src/args.rs` in `webdavis/damnit`: `--json` is a
global flag, `ls` takes its query as a positional, `new` takes the subject as a positional and the
path as `--path`, `add` takes oids or `-A`, `reset` takes oids or none, `commit` takes `-m`, `edit`
takes `-p`, `--due`, `--no-due`, `--deadline`, `--no-deadline`, `--label`, `--unlabel` and `-e`, `mv`
takes two positionals, and `resolve` takes `--ours` or `--theirs`.

`--json` buys two things and both matter: the report on standard output, and the error document on
standard error, which is the only place the pane reads a refusal's rule and oids. So every command
that can fail in a way the pane reports carries it.

`edit_in_editor` is the one command that carries no `--json`, and cannot: `dam edit -e --json` is
refused as `needs_an_editor` rather than run, because a machine format never opens an editor. It owns
the terminal, prints no report, and a run that fails prints the plain `dam: <message>` line, which
Task 14's `fallback` reads.

`status` needs no `--full`. Since `dam` 0.2.0 the default change document carries the oid, the
operation, a `fields` list and the state the change left behind, with no embedded object, which is
what a pane polling `status` per render wants. `--full` adds `before` and `after` for a client that
needs them and this pane does not.

- [ ] **Step 1: Write the failing tests**

`crates/herdr-damnit-application/src/argv/tests.rs`:

```rust
use super::*;

fn oid() -> Oid {
    Oid::new("1a2b3c4")
}

fn line(argv: Vec<String>) -> String {
    argv.join(" ")
}

#[test]
fn the_reads_carry_their_query_and_the_json_flag() {
    assert_eq!(line(version()), "--version");
    assert_eq!(line(list("!done")), "ls !done --json");
    assert_eq!(line(status()), "status --json");
    assert_eq!(line(show(&oid())), "show 1a2b3c4 --json");
    assert_eq!(line(log()), "log --json");
}

#[test]
fn staging_is_one_oid_or_all_of_them() {
    assert_eq!(line(stage(&oid())), "add 1a2b3c4 --json");
    assert_eq!(line(stage_all()), "add -A --json");
    assert_eq!(line(unstage(&oid())), "reset 1a2b3c4 --json");
    assert_eq!(line(unstage_all()), "reset --json");
}

#[test]
fn a_commit_message_stays_one_argument_however_many_spaces_it_has() {
    assert_eq!(
        commit("refresh the roster row"),
        vec![
            "commit".to_string(),
            "-m".to_string(),
            "refresh the roster row".to_string(),
            "--json".to_string(),
        ]
    );
}

#[test]
fn a_push_and_a_pull_name_no_remote_because_dam_acts_on_every_one() {
    assert_eq!(line(push()), "push --json");
    assert_eq!(line(pull()), "pull --json");
}

#[test]
fn completing_is_one_flag_apart_from_forcing_it() {
    assert_eq!(line(done(&oid(), false)), "done 1a2b3c4 --json");
    assert_eq!(line(done(&oid(), true)), "done 1a2b3c4 --force --json");
}

/// The exact line the spec pins: `p` on a priority-2 task.
#[test]
fn the_priority_key_spells_dams_own_short_flag() {
    let next = Priority::new(2).expect("a priority").next();
    assert_eq!(
        set_priority(&oid(), next),
        vec![
            "edit".to_string(),
            "1a2b3c4".to_string(),
            "-p".to_string(),
            "1".to_string(),
            "--json".to_string(),
        ]
    );
}

#[test]
fn a_date_is_set_or_cleared_by_two_different_flags() {
    assert_eq!(line(set_due(&oid(), Some("2026-09-20"))), "edit 1a2b3c4 --due 2026-09-20 --json");
    assert_eq!(line(set_due(&oid(), None)), "edit 1a2b3c4 --no-due --json");
    assert_eq!(
        line(set_deadline(&oid(), Some("2026-10-02"))),
        "edit 1a2b3c4 --deadline 2026-10-02 --json"
    );
    assert_eq!(line(set_deadline(&oid(), None)), "edit 1a2b3c4 --no-deadline --json");
}

#[test]
fn a_label_goes_on_and_comes_off_by_name() {
    assert_eq!(line(label(&oid(), "home")), "edit 1a2b3c4 --label home --json");
    assert_eq!(line(unlabel(&oid(), "home")), "edit 1a2b3c4 --unlabel home --json");
}

#[test]
fn moving_and_removing_and_creating_are_their_own_verbs() {
    assert_eq!(line(move_to(&oid(), "home/admin/")), "mv 1a2b3c4 home/admin/ --json");
    assert_eq!(line(remove(&oid())), "rm 1a2b3c4 --json");
    assert_eq!(line(create("file taxes", "home/admin/")), "new file taxes --path home/admin/ --json");
}

#[test]
fn a_new_object_under_no_path_leaves_the_flag_off_rather_than_sending_an_empty_one() {
    assert_eq!(line(create("file taxes", "")), "new file taxes --json");
}

/// `dam edit -e --json` is refused as `needs_an_editor`, so the editor round trip is the one
/// command that must not carry the flag.
#[test]
fn the_editor_round_trip_asks_for_no_report_because_it_owns_the_terminal() {
    assert_eq!(line(edit_in_editor(&oid())), "edit 1a2b3c4 -e");
    assert!(!edit_in_editor(&oid()).contains(&JSON.to_string()));
}

/// Every other command carries `--json`, which is what makes a failure answer with the error
/// document Task 14 maps instead of a line to match substrings against.
#[test]
fn every_command_that_reports_a_failure_asks_for_the_document() {
    for argv in [
        list("!done"),
        status(),
        show(&oid()),
        log(),
        stage(&oid()),
        stage_all(),
        unstage(&oid()),
        unstage_all(),
        commit("m"),
        push(),
        pull(),
        done(&oid(), false),
        remove(&oid()),
        set_priority(&oid(), Priority::default()),
        set_due(&oid(), None),
        set_deadline(&oid(), None),
        label(&oid(), "home"),
        unlabel(&oid(), "home"),
        move_to(&oid(), "home/"),
        create("s", ""),
        resolve(&oid(), Side::Ours),
        restore(&oid()),
    ] {
        assert!(argv.contains(&JSON.to_string()), "{argv:?}");
    }
}

#[test]
fn a_conflict_is_resolved_toward_one_side_by_name() {
    assert_eq!(line(resolve(&oid(), Side::Ours)), "resolve 1a2b3c4 --ours --json");
    assert_eq!(line(resolve(&oid(), Side::Theirs)), "resolve 1a2b3c4 --theirs --json");
}

#[test]
fn discarding_a_working_change_is_dams_restore() {
    assert_eq!(line(restore(&oid())), "restore 1a2b3c4 --json");
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p herdr-damnit-application --locked argv`
Expected: FAIL with `unresolved module or unlinked crate 'argv'`

- [ ] **Step 3: Write the module**

`crates/herdr-damnit-application/src/argv.rs`:

```rust
//! Every `dam` command the pane spawns, as the arguments that follow the configured `dam` argv.
//! One function per command, so a test compares the whole line and a flag typo fails it.

use herdr_damnit_domain::{Oid, Priority};

/// `--json` is a global flag on `dam`. It selects the report on standard output and the error
/// document on standard error, so every command whose failure the pane reports ends with it.
pub const JSON: &str = "--json";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Ours,
    Theirs,
}

pub fn version() -> Vec<String> {
    words(&["--version"])
}

pub fn list(query: &str) -> Vec<String> {
    words(&["ls", query, JSON])
}

pub fn status() -> Vec<String> {
    words(&["status", JSON])
}

pub fn show(oid: &Oid) -> Vec<String> {
    words(&["show", oid.as_str(), JSON])
}

pub fn log() -> Vec<String> {
    words(&["log", JSON])
}

pub fn stage(oid: &Oid) -> Vec<String> {
    words(&["add", oid.as_str(), JSON])
}

pub fn stage_all() -> Vec<String> {
    words(&["add", "-A", JSON])
}

pub fn unstage(oid: &Oid) -> Vec<String> {
    words(&["reset", oid.as_str(), JSON])
}

pub fn unstage_all() -> Vec<String> {
    words(&["reset", JSON])
}

pub fn commit(message: &str) -> Vec<String> {
    words(&["commit", "-m", message, JSON])
}

pub fn push() -> Vec<String> {
    words(&["push", JSON])
}

pub fn pull() -> Vec<String> {
    words(&["pull", JSON])
}

pub fn done(oid: &Oid, force: bool) -> Vec<String> {
    match force {
        true => words(&["done", oid.as_str(), "--force", JSON]),
        false => words(&["done", oid.as_str(), JSON]),
    }
}

pub fn remove(oid: &Oid) -> Vec<String> {
    words(&["rm", oid.as_str(), JSON])
}

pub fn set_priority(oid: &Oid, priority: Priority) -> Vec<String> {
    words(&["edit", oid.as_str(), "-p", &priority.get().to_string(), JSON])
}

pub fn set_due(oid: &Oid, when: Option<&str>) -> Vec<String> {
    edit_date(oid, when, "--due", "--no-due")
}

pub fn set_deadline(oid: &Oid, when: Option<&str>) -> Vec<String> {
    edit_date(oid, when, "--deadline", "--no-deadline")
}

fn edit_date(oid: &Oid, when: Option<&str>, set: &str, clear: &str) -> Vec<String> {
    match when {
        Some(text) => words(&["edit", oid.as_str(), set, text, JSON]),
        None => words(&["edit", oid.as_str(), clear, JSON]),
    }
}

pub fn label(oid: &Oid, name: &str) -> Vec<String> {
    words(&["edit", oid.as_str(), "--label", name, JSON])
}

pub fn unlabel(oid: &Oid, name: &str) -> Vec<String> {
    words(&["edit", oid.as_str(), "--unlabel", name, JSON])
}

pub fn move_to(oid: &Oid, path: &str) -> Vec<String> {
    words(&["mv", oid.as_str(), path, JSON])
}

/// A new object under the cursor's path. An empty path leaves the flag off, because `dam`'s own
/// default for `--path` is the empty string.
pub fn create(subject: &str, path: &str) -> Vec<String> {
    match path.is_empty() {
        true => words(&["new", subject, JSON]),
        false => words(&["new", subject, "--path", path, JSON]),
    }
}

/// `dam edit -e` owns the terminal and prints no report the pane reads.
pub fn edit_in_editor(oid: &Oid) -> Vec<String> {
    words(&["edit", oid.as_str(), "-e"])
}

pub fn resolve(oid: &Oid, side: Side) -> Vec<String> {
    let word = match side {
        Side::Ours => "--ours",
        Side::Theirs => "--theirs",
    };
    words(&["resolve", oid.as_str(), word, JSON])
}

pub fn restore(oid: &Oid) -> Vec<String> {
    words(&["restore", oid.as_str(), JSON])
}

fn words(argv: &[&str]) -> Vec<String> {
    argv.iter().map(|word| word.to_string()).collect()
}

#[cfg(test)]
mod tests;
```

Add to `crates/herdr-damnit-application/src/lib.rs`:

```rust
pub mod argv;

pub use argv::Side;
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p herdr-damnit-application --locked`
Expected: PASS, fifteen tests.

- [ ] **Step 5: Commit**

```bash
git add -A
SKIP_AI_COMMIT=1 git commit -m "feat(application): spell every dam command the pane spawns"
```

---

### Task 17: The job table: generations, the exclusive rule, and the follow-up reads

**Files:**
- Create: `crates/herdr-damnit-application/src/jobs.rs`
- Create: `crates/herdr-damnit-application/src/jobs/tests.rs`
- Modify: `crates/herdr-damnit-application/src/lib.rs`

**Interfaces:**
- Consumes: `DamRunner`, `RunningJob`, `Finished` from Task 15; the argv builders from Task 16.
- Produces:

```rust
pub enum SyncKind { Commit, Push, Pull }

pub enum JobKind {
    ReadList,
    ReadStatus,
    ReadShow(Oid),
    ReadLog,
    Write,
    Exclusive(SyncKind),
}

pub struct JobId(u64);

pub enum Submitted {
    Started(JobId),
    Refused(String),
    NotInstalled,
    Failed(String),
}

pub struct Completion {
    pub kind: JobKind,
    pub finished: Finished,
    pub follow_up: Vec<JobKind>,
}

pub struct Jobs { /* the runner, the table and the next id */ }

impl Jobs {
    pub fn new(runner: Box<dyn DamRunner>) -> Jobs;
    pub fn submit(&mut self, kind: JobKind, argv: Vec<String>) -> Submitted;
    pub fn drain(&mut self) -> Vec<Completion>;
    pub fn in_flight(&self) -> usize;
    pub fn exclusive(&self) -> Option<SyncKind>;
    pub fn cancel_current(&mut self) -> bool;
    pub fn elapsed_of_current(&self, now: Instant) -> Option<Duration>;
}
```

Three rules, each a test:

1. **At most one `Exclusive` job.** A second is refused with a status line, never queued:
   `a push is already running; <C-c> cancels it.` `dam push` with no remote sends to every remote,
   so a second push is almost never a different request, and a silent queue would be a promise the
   pane could not keep across a close.
1. **At most one read of each kind.** A newer read of a kind supersedes the older one; the older
   one's result is dropped on arrival rather than overwriting a fresher model. Writes run
   concurrently with reads; `dam` serialises them at the store with its own busy timeout.
1. **Completion re-reads rather than patching.** A `Write` or an `Exclusive` completion carries a
   follow-up of `[ReadStatus, ReadList]`; a read carries none. The rows come from the store rather
   than from a guess at what the write did.

`drain` is non-blocking: it uses `try_recv` on every job in the table and returns whatever arrived.

- [ ] **Step 1: Write the failing tests**

`crates/herdr-damnit-application/src/jobs/tests.rs`:

```rust
use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, Mutex};

use super::*;
use crate::{Finished, SpawnError};

/// A runner that records every argv and hands the test the sender for each job, so a test decides
/// when a job answers and with what.
#[derive(Default)]
struct Recorder {
    log: Arc<Mutex<Vec<Vec<String>>>>,
    senders: Arc<Mutex<Vec<Sender<Finished>>>>,
    refuse_spawn: bool,
}

impl DamRunner for Recorder {
    fn spawn(&self, argv: &[String]) -> Result<RunningJob, SpawnError> {
        if self.refuse_spawn {
            return Err(SpawnError::NotFound);
        }
        self.log.lock().expect("the log").push(argv.to_vec());
        let (sender, results) = channel();
        self.senders.lock().expect("the senders").push(sender);
        Ok(RunningJob {
            cancel: Box::new(|| {}),
            results,
        })
    }
}

fn ok(stdout: &str) -> Finished {
    Finished {
        code: Some(0),
        stdout: stdout.to_string(),
        stderr: String::new(),
        elapsed: std::time::Duration::from_millis(9),
    }
}

struct Harness {
    jobs: Jobs,
    log: Arc<Mutex<Vec<Vec<String>>>>,
    senders: Arc<Mutex<Vec<Sender<Finished>>>>,
}

fn harness() -> Harness {
    let recorder = Recorder::default();
    let log = Arc::clone(&recorder.log);
    let senders = Arc::clone(&recorder.senders);
    Harness {
        jobs: Jobs::new(Box::new(recorder)),
        log,
        senders,
    }
}

impl Harness {
    /// Answer one spawned job. A send to a superseded job fails because dropping it from the table
    /// dropped its receiver, which is the mechanism by which its result never reaches the model, so
    /// the answer is offered rather than required.
    fn answer(&self, which: usize, finished: Finished) {
        let _ = self.senders.lock().expect("the senders")[which].send(finished);
    }

    fn lines(&self) -> Vec<String> {
        self.log
            .lock()
            .expect("the log")
            .iter()
            .map(|argv| argv.join(" "))
            .collect()
    }
}

#[test]
fn a_second_push_is_refused_with_a_sentence_and_never_spawned() {
    let mut harness = harness();
    assert!(matches!(
        harness.jobs.submit(JobKind::Exclusive(SyncKind::Push), crate::argv::push()),
        Submitted::Started(_)
    ));

    let Submitted::Refused(message) =
        harness.jobs.submit(JobKind::Exclusive(SyncKind::Push), crate::argv::push())
    else {
        panic!("expected a refusal");
    };

    assert_eq!(message, "a push is already running; <C-c> cancels it.");
    assert_eq!(harness.lines(), vec!["push --json".to_string()]);
}

#[test]
fn a_pull_is_refused_while_a_push_runs_and_names_the_one_that_is_running() {
    let mut harness = harness();
    harness.jobs.submit(JobKind::Exclusive(SyncKind::Push), crate::argv::push());

    let Submitted::Refused(message) =
        harness.jobs.submit(JobKind::Exclusive(SyncKind::Pull), crate::argv::pull())
    else {
        panic!("expected a refusal");
    };
    assert_eq!(message, "a push is already running; <C-c> cancels it.");
}

#[test]
fn a_write_runs_alongside_a_push_because_dam_serialises_them_at_the_store() {
    let mut harness = harness();
    harness.jobs.submit(JobKind::Exclusive(SyncKind::Push), crate::argv::push());
    assert!(matches!(
        harness.jobs.submit(JobKind::Write, crate::argv::stage_all()),
        Submitted::Started(_)
    ));
    assert_eq!(harness.jobs.in_flight(), 2);
}

#[test]
fn a_newer_read_of_a_kind_supersedes_the_older_one_and_its_result_is_dropped() {
    let mut harness = harness();
    harness.jobs.submit(JobKind::ReadList, crate::argv::list("!done"));
    harness.jobs.submit(JobKind::ReadList, crate::argv::list("done"));

    harness.answer(0, ok(r#"{"objects":[]}"#));
    harness.answer(1, ok(r#"{"objects":[{"oid":"1"}]}"#));

    let completions = harness.jobs.drain();
    assert_eq!(completions.len(), 1, "a superseded read reached the model");
    assert_eq!(completions[0].finished.stdout, r#"{"objects":[{"oid":"1"}]}"#);
}

#[test]
fn a_write_asks_for_a_fresh_status_and_a_fresh_list_rather_than_patching_the_model() {
    let mut harness = harness();
    harness.jobs.submit(JobKind::Write, crate::argv::stage_all());
    harness.answer(0, ok("{}"));

    let completions = harness.jobs.drain();
    assert_eq!(
        completions[0].follow_up,
        vec![JobKind::ReadStatus, JobKind::ReadList]
    );
}

#[test]
fn a_read_asks_for_nothing_after_itself() {
    let mut harness = harness();
    harness.jobs.submit(JobKind::ReadStatus, crate::argv::status());
    harness.answer(0, ok("{}"));

    assert_eq!(harness.jobs.drain()[0].follow_up, Vec::new());
}

#[test]
fn draining_does_not_block_on_a_job_that_has_not_answered() {
    let mut harness = harness();
    harness.jobs.submit(JobKind::ReadStatus, crate::argv::status());

    assert!(harness.jobs.drain().is_empty());
    assert_eq!(harness.jobs.in_flight(), 1);
}

#[test]
fn a_finished_job_leaves_the_table() {
    let mut harness = harness();
    harness.jobs.submit(JobKind::ReadStatus, crate::argv::status());
    harness.answer(0, ok("{}"));
    harness.jobs.drain();

    assert_eq!(harness.jobs.in_flight(), 0);
    assert_eq!(harness.jobs.exclusive(), None);
}

#[test]
fn a_dam_that_is_not_there_is_reported_rather_than_started() {
    let recorder = Recorder {
        refuse_spawn: true,
        ..Recorder::default()
    };
    let mut jobs = Jobs::new(Box::new(recorder));

    assert!(matches!(
        jobs.submit(JobKind::ReadStatus, crate::argv::status()),
        Submitted::NotInstalled
    ));
    assert_eq!(jobs.in_flight(), 0);
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p herdr-damnit-application --locked jobs`
Expected: FAIL with `unresolved module or unlinked crate 'jobs'`

- [ ] **Step 3: Write the module**

`crates/herdr-damnit-application/src/jobs.rs`:

```rust
//! The jobs in flight. One `dam` call per job, at most one exclusive job at a time, at most one
//! read of each kind, and a completion that re-reads rather than patching the model.

use std::sync::mpsc::TryRecvError;
use std::time::{Duration, Instant};

use herdr_damnit_domain::Oid;

use crate::{DamRunner, Finished, RunningJob, SpawnError};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SyncKind {
    Commit,
    Push,
    Pull,
}

impl SyncKind {
    pub fn verb(self) -> &'static str {
        match self {
            Self::Commit => "commit",
            Self::Push => "push",
            Self::Pull => "pull",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JobKind {
    ReadList,
    ReadStatus,
    ReadShow(Oid),
    ReadLog,
    Write,
    Exclusive(SyncKind),
}

impl JobKind {
    /// Whether a newer job of this kind supersedes an older one. Two writes are two different
    /// intentions and both run; two reads of one kind are the same question asked twice.
    fn supersedes_its_own_kind(&self) -> bool {
        matches!(
            self,
            Self::ReadList | Self::ReadStatus | Self::ReadShow(_) | Self::ReadLog
        )
    }

    /// The reads a completion of this kind enqueues. The rows come from the store rather than from
    /// a guess at what the write did.
    fn follow_up(&self) -> Vec<JobKind> {
        match self {
            Self::Write | Self::Exclusive(_) => vec![Self::ReadStatus, Self::ReadList],
            _ => Vec::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JobId(u64);

#[derive(Debug)]
pub enum Submitted {
    Started(JobId),
    Refused(String),
    NotInstalled,
    Failed(String),
}

#[derive(Debug)]
pub struct Completion {
    pub kind: JobKind,
    pub finished: Finished,
    pub follow_up: Vec<JobKind>,
}

struct Running {
    id: JobId,
    kind: JobKind,
    started: Instant,
    job: RunningJob,
}

pub struct Jobs {
    runner: Box<dyn DamRunner>,
    running: Vec<Running>,
    next: u64,
}

impl Jobs {
    pub fn new(runner: Box<dyn DamRunner>) -> Self {
        Self {
            runner,
            running: Vec::new(),
            next: 0,
        }
    }

    pub fn submit(&mut self, kind: JobKind, argv: Vec<String>) -> Submitted {
        if let (JobKind::Exclusive(_), Some(running)) = (&kind, self.exclusive()) {
            return Submitted::Refused(format!(
                "a {} is already running; <C-c> cancels it.",
                running.verb()
            ));
        }
        if kind.supersedes_its_own_kind() {
            self.running.retain(|job| job.kind != kind);
        }
        let job = match self.runner.spawn(&argv) {
            Ok(job) => job,
            Err(SpawnError::NotFound) => return Submitted::NotInstalled,
            Err(SpawnError::Io(error)) => return Submitted::Failed(error),
        };
        self.next += 1;
        let id = JobId(self.next);
        self.running.push(Running {
            id,
            kind,
            started: Instant::now(),
            job,
        });
        Submitted::Started(id)
    }

    /// Every job that has answered since the last call, without blocking on the ones that have not.
    pub fn drain(&mut self) -> Vec<Completion> {
        let mut completions = Vec::new();
        let mut finished = Vec::new();
        for running in &self.running {
            match running.job.results.try_recv() {
                Ok(result) => {
                    finished.push(running.id);
                    completions.push(Completion {
                        kind: running.kind.clone(),
                        follow_up: running.kind.follow_up(),
                        finished: result,
                    });
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => finished.push(running.id),
            }
        }
        self.running.retain(|job| !finished.contains(&job.id));
        completions
    }

    pub fn in_flight(&self) -> usize {
        self.running.len()
    }

    pub fn exclusive(&self) -> Option<SyncKind> {
        self.running.iter().find_map(|job| match job.kind {
            JobKind::Exclusive(sync) => Some(sync),
            _ => None,
        })
    }

    /// Cancel the job the header names: the exclusive one when there is one, the oldest otherwise.
    pub fn cancel_current(&mut self) -> bool {
        let Some(running) = self.current() else {
            return false;
        };
        (running.job.cancel)();
        true
    }

    pub fn elapsed_of_current(&self, now: Instant) -> Option<Duration> {
        self.current()
            .map(|running| now.saturating_duration_since(running.started))
    }

    fn current(&self) -> Option<&Running> {
        self.running
            .iter()
            .find(|job| matches!(job.kind, JobKind::Exclusive(_)))
            .or_else(|| self.running.first())
    }
}

#[cfg(test)]
mod tests;
```

Add to `crates/herdr-damnit-application/src/lib.rs`:

```rust
mod jobs;

pub use jobs::{Completion, JobId, JobKind, Jobs, Submitted, SyncKind};
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p herdr-damnit-application --locked`
Expected: PASS, nine new tests.

- [ ] **Step 5: Check the file is inside the cap**

Run: `wc -l crates/herdr-damnit-application/src/jobs.rs`
Expected: under 300.

- [ ] **Step 6: Commit**

```bash
git add -A
SKIP_AI_COMMIT=1 git commit -m "feat(application): run one exclusive dam job at a time and re-read after every write"
```

---

### Task 18: The handshake

**Files:**
- Create: `crates/herdr-damnit-application/src/handshake.rs`
- Modify: `crates/herdr-damnit-application/src/lib.rs`

**Interfaces:**
- Consumes: `parse_version`, `verdict`, `Verdict`, `DamVersion` from Task 13.
- Produces:

```rust
pub enum Handshake {
    Ready { version: DamVersion, warning: Option<String> },
    Refuse(String),
}

pub fn handshake(version_output: &str, status_output: &str) -> Handshake;
```

On start, before the first read, the pane runs `dam --version` and then one `dam status --json`, and
requires the five top-level keys `staged`, `unstaged`, `conflicts`, `notices` and `unpushed`. A
document missing one of them fails the handshake with the same refusal screen, which catches a `dam`
that answers `--version` but was built from a fork. The handshake is not repeated on a refresh.

- [ ] **Step 1: Write the failing tests**

`crates/herdr-damnit-application/src/handshake.rs`, inside `mod tests`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    const STATUS: &str = r#"{"staged":[],"unstaged":[],"conflicts":[],"notices":[],"unpushed":[]}"#;

    #[test]
    fn a_dam_at_the_floor_with_the_five_keys_is_ready_and_quiet() {
        let Handshake::Ready { version, warning } = handshake("dam 0.2.0\n", STATUS) else {
            panic!("expected ready");
        };
        assert_eq!(version.to_string(), "0.2.0");
        assert_eq!(warning, None);
    }

    #[test]
    fn a_newer_minor_is_ready_and_carries_one_warning() {
        let Handshake::Ready { warning, .. } = handshake("dam 0.4.0", STATUS) else {
            panic!("expected ready");
        };
        assert_eq!(
            warning.as_deref(),
            Some("dam 0.4.0 is newer than this pane knows; some keys may be refused.")
        );
    }

    #[test]
    fn a_dam_below_the_floor_refuses_to_draw() {
        let Handshake::Refuse(message) = handshake("dam 0.0.9", STATUS) else {
            panic!("expected a refusal");
        };
        assert!(message.contains("is older than the 0.2 this pane needs"), "{message}");
    }

    #[test]
    fn a_version_line_that_does_not_parse_refuses_rather_than_guessing() {
        let Handshake::Refuse(message) = handshake("not a version", STATUS) else {
            panic!("expected a refusal");
        };
        assert_eq!(
            message,
            "dam did not print a version this pane could read; run cargo install damnit to update it."
        );
    }

    #[test]
    fn a_status_document_missing_a_key_fails_the_handshake_and_names_it() {
        let missing = r#"{"staged":[],"unstaged":[],"conflicts":[],"notices":[]}"#;
        let Handshake::Refuse(message) = handshake("dam 0.2.0", missing) else {
            panic!("expected a refusal");
        };
        assert_eq!(
            message,
            "dam status answered without \"unpushed\"; this pane needs a dam built from upstream."
        );
    }

    #[test]
    fn a_status_document_that_is_not_json_fails_the_handshake() {
        let Handshake::Refuse(message) = handshake("dam 0.2.0", "not json") else {
            panic!("expected a refusal");
        };
        assert_eq!(message, "dam answered with something this pane could not read.");
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p herdr-damnit-application --locked handshake`
Expected: FAIL with `unresolved module or unlinked crate 'handshake'`

- [ ] **Step 3: Write the module**

`crates/herdr-damnit-application/src/handshake.rs`, above its test module:

```rust
//! What the pane checks before it draws anything: the version of `dam` it found, and that one
//! `dam status --json` carries the five keys every screen reads.

use herdr_damnit_domain::{DamVersion, Verdict, parse_version, verdict};

/// The five top-level keys `dam status --json` returns. A document missing one is a `dam` built
/// from a fork rather than the one this pane was written against.
const STATUS_KEYS: [&str; 5] = ["staged", "unstaged", "conflicts", "notices", "unpushed"];

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Handshake {
    Ready {
        version: DamVersion,
        warning: Option<String>,
    },
    Refuse(String),
}

pub fn handshake(version_output: &str, status_output: &str) -> Handshake {
    let Some(version) = parse_version(version_output) else {
        return Handshake::Refuse(
            "dam did not print a version this pane could read; run cargo install damnit to \
             update it."
                .to_string(),
        );
    };
    let warning = match verdict(version) {
        Verdict::Refuse(message) => return Handshake::Refuse(message),
        Verdict::Warn(message) => Some(message),
        Verdict::Fine => None,
    };
    let Ok(document) = serde_json::from_str::<serde_json::Value>(status_output) else {
        return Handshake::Refuse(herdr_damnit_domain::message(
            &herdr_damnit_domain::Failure::Unreadable,
        ));
    };
    for key in STATUS_KEYS {
        if document.get(key).is_none() {
            return Handshake::Refuse(format!(
                "dam status answered without {key:?}; this pane needs a dam built from upstream."
            ));
        }
    }
    Handshake::Ready { version, warning }
}
```

Add to `crates/herdr-damnit-application/src/lib.rs`:

```rust
mod handshake;

pub use handshake::{Handshake, handshake};
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p herdr-damnit-application --locked`
Expected: PASS, six new tests.

- [ ] **Step 5: Commit**

```bash
git add -A
SKIP_AI_COMMIT=1 git commit -m "feat(application): check dam's version and its status document before drawing"
```

---

### Task 19: The hand-off to the agent pane

**Files:**
- Create: `crates/herdr-damnit-application/src/handoff.rs`
- Create: `crates/herdr-damnit-application/src/handoff/tests.rs`
- Modify: `crates/herdr-damnit-application/src/lib.rs`
- Modify: `crates/herdr-damnit-application/Cargo.toml` (add `serde` with `derive`)
- Reference: `crates/herdr-damnit/src/send.rs`, whose `agent_in`, `named`, `hand_off` and `pasted`
  this reproduces over the `Herdr` port

**Interfaces:**
- Consumes: `Herdr` from Task 15, `brief` from Task 12, `label` from Task 16.
- Produces:

```rust
pub struct Agent { pub pane: String, pub name: String }

pub struct Workspace { pub workspace: Option<String>, pub me: String }

pub enum HandOff {
    Sent { agent: Agent, label: Option<Vec<String>> },
    Refused(String),
}

pub fn hand_off(
    herdr: &dyn Herdr,
    here: &Workspace,
    object: &Object,
    note: &str,
    handoff_label: &str,
) -> HandOff;
```

The mechanism is unchanged: the brief goes into the agent pane's input with
`herdr pane send-text` as one bracketed paste and is never submitted, so the operator reads it, adds
to it and presses return themselves; `herdr agent focus` then puts the cursor there, and a refused
focus does not fail the send, because the agent has the work either way. A pane herdr lists no agent
for is not a candidate, and neither is this one.

What changes is the record. The old pane posted a Todoist comment; `dam` has no comments, so
`HandOff::Sent::label` carries the argv of `dam edit <oid> --label <handoff_label>` for the caller to
submit as an ordinary write job, and is `None` when `handoff_label` is empty. That is a working-layer
change, which is the right place for it, because a hand-off is a change to the task.

The fake records through a `Mutex`, not a `RefCell`: `Herdr: Send + Sync` is the port's own bound in
Task 15, and a fake holding a `RefCell` fails to compile with
`error[E0277]: RefCell<...> cannot be shared between threads safely`.

- [ ] **Step 1: Write the failing tests**

`crates/herdr-damnit-application/src/handoff/tests.rs`:

```rust
use std::sync::Mutex;

use herdr_damnit_domain::{Kind, Object, Oid, Priority, TaskFields};

use super::*;

const LISTING: &str = r#"{"result":{"agents":[
  {"pane_id":"w1:p1","workspace_id":"w1","agent":null},
  {"pane_id":"w1:p2","workspace_id":"w1","agent":"claude","name":"planner"},
  {"pane_id":"w2:p9","workspace_id":"w2","agent":"codex"}
]}}"#;

/// `Herdr` is `Send + Sync`, so the fake records through a `Mutex`.
struct FakeHerdr {
    calls: Mutex<Vec<Vec<String>>>,
    listing: String,
    refuse_send: bool,
    refuse_focus: bool,
}

impl FakeHerdr {
    fn new() -> Self {
        Self {
            calls: Mutex::new(Vec::new()),
            listing: LISTING.to_string(),
            refuse_send: false,
            refuse_focus: false,
        }
    }

    fn verbs(&self) -> Vec<String> {
        self.calls
            .lock()
            .expect("the calls")
            .iter()
            .map(|call| call[..2.min(call.len())].join(" "))
            .collect()
    }

    fn sent_text(&self) -> String {
        self.calls
            .lock()
            .expect("the calls")
            .iter()
            .find(|call| call.first().map(String::as_str) == Some("pane"))
            .and_then(|call| call.last().cloned())
            .unwrap_or_default()
    }
}

impl Herdr for FakeHerdr {
    fn call(&self, args: &[&str]) -> Result<String, String> {
        self.calls
            .lock()
            .expect("the calls")
            .push(args.iter().map(|word| word.to_string()).collect());
        match args {
            ["agent", "list"] => Ok(self.listing.clone()),
            ["pane", "send-text", ..] if self.refuse_send => Err("refused".to_string()),
            ["agent", "focus", ..] if self.refuse_focus => Err("refused".to_string()),
            _ => Ok(r#"{"result":{}}"#.to_string()),
        }
    }
}

fn here() -> Workspace {
    Workspace {
        workspace: Some("w1".to_string()),
        me: "w1:p1".to_string(),
    }
}

fn object() -> Object {
    Object {
        oid: Oid::new("1a2b3c4"),
        kind: Kind::Task,
        subject: "file taxes".to_string(),
        body: String::new(),
        path: String::new(),
        labels: Vec::new(),
        depends: Vec::new(),
        recurrence: None,
        task: Some(TaskFields {
            done: false,
            priority: Priority::default(),
            due: None,
            deadline: None,
            attached: None,
        }),
        event: None,
    }
}

#[test]
fn the_brief_goes_to_the_named_agent_of_this_workspace() {
    let herdr = FakeHerdr::new();
    let HandOff::Sent { agent, .. } = hand_off(&herdr, &here(), &object(), "", "") else {
        panic!("expected a send");
    };

    assert_eq!(agent.pane, "w1:p2");
    assert_eq!(agent.name, "planner");
    assert_eq!(
        herdr.verbs(),
        vec![
            "agent list".to_string(),
            "pane send-text".to_string(),
            "agent focus".to_string(),
        ]
    );
}

#[test]
fn the_brief_is_one_bracketed_paste_and_carries_no_return() {
    let herdr = FakeHerdr::new();
    hand_off(&herdr, &here(), &object(), "start here", "");

    let sent = herdr.sent_text();
    assert!(sent.starts_with("\u{1b}[200~"), "{sent:?}");
    assert!(sent.ends_with("\u{1b}[201~"), "{sent:?}");
    assert!(sent.contains("dam task: file taxes"), "{sent:?}");
    assert!(sent.contains("note: start here"), "{sent:?}");
    assert!(!sent.trim_end_matches("\u{1b}[201~").ends_with('\n'), "{sent:?}");
}

#[test]
fn a_paste_terminator_inside_the_brief_cannot_end_the_frame_early() {
    let herdr = FakeHerdr::new();
    let mut object = object();
    object.body = "before \u{1b}[201~ after".to_string();
    hand_off(&herdr, &here(), &object, "", "");

    let sent = herdr.sent_text();
    assert_eq!(sent.matches("\u{1b}[201~").count(), 1, "{sent:?}");
}

#[test]
fn a_workspace_with_no_agent_pane_says_so_and_sends_nothing() {
    let herdr = FakeHerdr {
        listing: r#"{"result":{"agents":[]}}"#.to_string(),
        ..FakeHerdr::new()
    };
    let HandOff::Refused(message) = hand_off(&herdr, &here(), &object(), "", "") else {
        panic!("expected a refusal");
    };

    assert_eq!(message, "no agent pane in this workspace.");
    assert_eq!(herdr.verbs(), vec!["agent list".to_string()]);
}

#[test]
fn this_pane_is_never_its_own_agent() {
    let herdr = FakeHerdr::new();
    let mine = Workspace {
        workspace: Some("w1".to_string()),
        me: "w1:p2".to_string(),
    };
    let HandOff::Refused(message) = hand_off(&herdr, &mine, &object(), "", "") else {
        panic!("expected a refusal");
    };
    assert_eq!(message, "no agent pane in this workspace.");
}

#[test]
fn a_refused_send_is_a_refusal_and_a_refused_focus_is_not() {
    let refused_send = FakeHerdr {
        refuse_send: true,
        ..FakeHerdr::new()
    };
    assert!(matches!(
        hand_off(&refused_send, &here(), &object(), "", ""),
        HandOff::Refused(_)
    ));

    let refused_focus = FakeHerdr {
        refuse_focus: true,
        ..FakeHerdr::new()
    };
    assert!(matches!(
        hand_off(&refused_focus, &here(), &object(), "", ""),
        HandOff::Sent { .. }
    ));
}

#[test]
fn a_configured_label_comes_back_as_the_write_the_caller_submits() {
    let herdr = FakeHerdr::new();
    let HandOff::Sent { label, .. } = hand_off(&herdr, &here(), &object(), "", "handed-off") else {
        panic!("expected a send");
    };

    assert_eq!(
        label,
        Some(vec![
            "edit".to_string(),
            "1a2b3c4".to_string(),
            "--label".to_string(),
            "handed-off".to_string(),
            "--json".to_string(),
        ])
    );
}

#[test]
fn an_empty_label_writes_nothing_and_the_status_line_is_the_whole_record() {
    let herdr = FakeHerdr::new();
    let HandOff::Sent { label, .. } = hand_off(&herdr, &here(), &object(), "", "  ") else {
        panic!("expected a send");
    };
    assert_eq!(label, None);
}

#[test]
fn a_pane_outside_herdr_says_so_rather_than_guessing_a_workspace() {
    let herdr = FakeHerdr::new();
    let nowhere = Workspace {
        workspace: None,
        me: String::new(),
    };
    let HandOff::Refused(message) = hand_off(&herdr, &nowhere, &object(), "", "") else {
        panic!("expected a refusal");
    };
    assert_eq!(message, "no workspace context: run this pane inside herdr.");
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p herdr-damnit-application --locked handoff`
Expected: FAIL with `unresolved module or unlinked crate 'handoff'`

- [ ] **Step 3: Write the module**

Add `serde = { workspace = true }` to `crates/herdr-damnit-application/Cargo.toml`, then
`crates/herdr-damnit-application/src/handoff.rs`:

```rust
//! `S`: hand the object under the cursor to the workspace's agent pane. The brief goes into that
//! pane's input as one bracketed paste and is never submitted, so the operator reads it, adds to
//! it and presses return themselves.

use herdr_damnit_domain::{Object, brief};
use serde::Deserialize;

use crate::Herdr;
use crate::argv;

const PASTE_START: &str = "\x1b[200~";
const PASTE_END: &str = "\x1b[201~";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Agent {
    pub pane: String,
    pub name: String,
}

/// The two pane facts herdr puts in this process's environment.
#[derive(Clone, Debug)]
pub struct Workspace {
    pub workspace: Option<String>,
    pub me: String,
}

#[derive(Debug)]
pub enum HandOff {
    Sent {
        agent: Agent,
        /// The argv of the label write that records the hand-off, for the caller to submit as an
        /// ordinary write job.
        label: Option<Vec<String>>,
    },
    Refused(String),
}

/// One row of `herdr agent list`. `name` is absent until herdr sets one; `display_agent` is the
/// auth profile a pane authenticated with rather than the agent, so the true kind outranks it.
#[derive(Deserialize)]
struct Listed {
    pane_id: String,
    workspace_id: String,
    #[serde(default)]
    agent: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    display_agent: Option<String>,
}

#[derive(Deserialize)]
struct Listing {
    result: Agents,
}

#[derive(Deserialize)]
struct Agents {
    agents: Vec<Listed>,
}

pub fn hand_off(
    herdr: &dyn Herdr,
    here: &Workspace,
    object: &Object,
    note: &str,
    handoff_label: &str,
) -> HandOff {
    let Some(workspace) = here.workspace.as_deref() else {
        return HandOff::Refused("no workspace context: run this pane inside herdr.".to_string());
    };
    let listing = match herdr.call(&["agent", "list"]) {
        Ok(listing) => listing,
        Err(error) => return HandOff::Refused(error),
    };
    let agent = match agent_in(&listing, workspace, &here.me) {
        Ok(agent) => agent,
        Err(error) => return HandOff::Refused(error),
    };
    let text = pasted(&brief(object, note));
    if herdr
        .call(&["pane", "send-text", &agent.pane, &text])
        .is_err()
    {
        return HandOff::Refused(format!("herdr refused the send to {}.", agent.pane));
    }
    // Focus is a convenience once the text is delivered: a refused focus leaves the brief in the
    // agent's input, so failing here would report a hand-off that did happen as one that did not.
    let _ = herdr.call(&["agent", "focus", &agent.pane]);
    let label = (!handoff_label.trim().is_empty())
        .then(|| argv::label(&object.oid, handoff_label.trim()));
    HandOff::Sent { agent, label }
}

/// The agent pane of this workspace. A pane herdr lists no agent for is not a candidate, and
/// neither is this one; with several, the first herdr names wins.
fn agent_in(listing: &str, workspace: &str, me: &str) -> Result<Agent, String> {
    let listing: Listing =
        serde_json::from_str(listing).map_err(|error| format!("herdr agent list: {error}"))?;
    listing
        .agents()
        .filter(|listed| listed.agent.is_some() && listed.pane_id != me)
        .find(|listed| listed.workspace_id == workspace)
        .map(|listed| Agent {
            name: named(listed),
            pane: listed.pane_id.clone(),
        })
        .ok_or_else(|| "no agent pane in this workspace.".to_string())
}

impl Listing {
    fn agents(&self) -> impl Iterator<Item = &Listed> {
        self.result.agents.iter()
    }
}

fn named(listed: &Listed) -> String {
    [&listed.name, &listed.agent, &listed.display_agent]
        .into_iter()
        .flatten()
        .find(|name| !name.is_empty())
        .cloned()
        .unwrap_or_else(|| "the agent".to_string())
}

/// The brief as one bracketed paste: a paste is inserted verbatim by any input, where the raw
/// bytes of a multi-line brief would each be read as a key and a newline would submit it. A
/// terminator inside the brief would end the frame early, so the body is rebuilt with a suffix
/// check per character and none can survive, not even one spliced together by an earlier removal.
fn pasted(text: &str) -> String {
    let mut body = String::with_capacity(text.len());
    for character in text.chars() {
        body.push(character);
        if body.ends_with(PASTE_END) {
            body.truncate(body.len() - PASTE_END.len());
        }
    }
    format!("{PASTE_START}{body}{PASTE_END}")
}

#[cfg(test)]
mod tests;
```

Add to `crates/herdr-damnit-application/src/lib.rs`:

```rust
mod handoff;

pub use handoff::{Agent, HandOff, Workspace, hand_off};
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --workspace --locked`
Expected: PASS, nine new tests.

- [ ] **Step 5: Check the file is inside the cap**

Run: `wc -l crates/herdr-damnit-application/src/handoff.rs crates/herdr-damnit-application/src/handoff/tests.rs`
Expected: both under 300.

- [ ] **Step 6: Commit**

```bash
git add -A
SKIP_AI_COMMIT=1 git commit -m "feat(application): hand a dam object to the workspace's agent pane"
```

---

## Phase D: the adapters crate

`herdr-damnit-adapters` is the only place `std::process::Command` names `dam`. It also carries the
herdr CLI client, the config reader, the state directory, the clock and the mapping from `dam`'s wire
documents into the domain types.

### Task 20: The fake dam and the fixtures

**Files:**
- Create: `crates/herdr-damnit-adapters/Cargo.toml`
- Create: `crates/herdr-damnit-adapters/src/lib.rs`
- Create: `crates/herdr-damnit-adapters/src/bin/fake_dam.rs`
- Create: `crates/herdr-damnit-adapters/tests/support/mod.rs`
- Create: `crates/herdr-damnit-adapters/tests/fixtures/capture.sh`
- Create: `crates/herdr-damnit-adapters/tests/fixtures/*.json` (the fifteen documents below)
- Create: `crates/herdr-damnit-adapters/tests/fake_dam.rs`
- Modify: `Cargo.toml`

**Interfaces:**
- Consumes: nothing. This task builds the test double every later adapter and binary test drives.
- Produces:
  - A binary target `fake-dam` in the adapters crate, reachable from that crate's integration tests
    as `env!("CARGO_BIN_EXE_fake-dam")`. The workspace's `default-members` is the binary crate
    alone, so a release build of the plugin never builds it.
  - Its four environment knobs: `FAKE_DAM_LOG` (a file it appends one JSON line of full argv to per
    call), `FAKE_DAM_FIXTURE_DIR` (the directory it replays `<subcommand>.json` from),
    `FAKE_DAM_EXIT` (an exit code) with `FAKE_DAM_STDERR` (what to print on standard error, which
    for a refusal is the whole error document `dam` prints under `--json`), and `FAKE_DAM_SLEEP_MS`
    (a sleep before answering). It installs a `SIGINT` handler that appends a `{"signal":"SIGINT"}`
    line to the log and exits 3.
  - Sixteen fixtures: `ls.json`, `ls-empty.json`, `ls-done.json`, `status-clean.json`,
    `status-full.json`, `show-task.json`, `show-event.json`, `log.json`, `log-completion.json`,
    `push-ok.json`, `push-partial-failure.json`, `pull-ok.json`, `pull-conflict.json`,
    `commit.json`, `new.json`, `version.txt`. `log-completion.json` is the log of a store whose
    task was completed and committed, which is the only fixture that carries the shape
    `completions` walks.

Fixtures are owned by this repository and are byte copies of documents the built `dam` actually
produced. A `dam` change that moves the bytes fails a test here, which is the point: the pane pins
the contract from its own side, the way `dam` pins it from the other.

- [ ] **Step 1: Write the capture script**

`crates/herdr-damnit-adapters/tests/fixtures/capture.sh`, the reproducible record of where every
fixture came from. It clones and builds `dam`, drives a scratch store with neutral subjects, and
writes each document beside itself.

```bash
#!/usr/bin/env bash
# Regenerate every fixture in this directory from a built dam.
# Usage: ./capture.sh [<dam revision>]
set -euo pipefail

revision="${1:-main}"
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

if [[ -n "${DAM_BIN:-}" ]]; then
  dam="$DAM_BIN"
else
  git clone --quiet https://github.com/webdavis/damnit "$work/damnit"
  git -C "$work/damnit" checkout --quiet "$revision"
  (cd "$work/damnit" && cargo build --release --locked --quiet)
  dam="$work/damnit/target/release/dam"
fi

# dam reads the store and the config from these when its flags name none, and it stamps an event
# with the local zone, so the capture runs in UTC against a store nothing else can reach.
export HOME="$work/home"
export XDG_CONFIG_HOME="$work/home/.config"
export XDG_DATA_HOME="$work/home/.local/share"
export TZ=UTC
mkdir -p "$XDG_CONFIG_HOME" "$XDG_DATA_HOME"

export DAM_STORE="$work/store.sqlite"
export DAM_CONFIG="$work/config.toml"
cat >"$DAM_CONFIG" <<'CONFIG'
[remote.example]
url = "example::"
CONFIG

"$dam" --version >"$here/version.txt"

# A clean status is one with nothing staged, nothing working and no unpushed commit, which is the
# empty store: the first commit below leaves one commit unpushed for the life of the capture.
"$dam" status --json >"$here/status-clean.json"

"$dam" new "ship the pin bump" --path "proj/dotfiles" -p 1 --due 2026-09-18 >/dev/null
"$dam" new "refresh the roster row" --path "proj/dotfiles" --label slow >/dev/null
"$dam" new "water the plants" --path "proj/home" >/dev/null
plants="$("$dam" ls "path:proj/home/" --json |
  python3 -c 'import json,sys; print(json.load(sys.stdin)["objects"][0]["oid"])')"
# Recurrence is an edit flag rather than a new flag, so the weekly task takes two calls.
"$dam" edit "$plants" --recurrence "every week" >/dev/null
"$dam" ls "!done" --json >"$here/ls.json"
"$dam" ls "path:nowhere/" --json >"$here/ls-empty.json"
"$dam" status --json >"$here/status-full.json"

"$dam" add -A >/dev/null
"$dam" commit -m "the first commit" --json >"$here/commit.json"
"$dam" log --json >"$here/log.json"

first="$("$dam" ls "!done" --json |
  python3 -c 'import json,sys; print(json.load(sys.stdin)["objects"][0]["oid"])')"
"$dam" show "$first" --json >"$here/show-task.json"
"$dam" done "$first" >/dev/null
"$dam" add -A >/dev/null
"$dam" commit -m "the completion" --json >/dev/null
"$dam" log --json >"$here/log-completion.json"
"$dam" ls "done" --json >"$here/ls-done.json"
"$dam" new "stand-up" --event --start 2026-09-21T09:00 --end 2026-09-21T09:15 --json \
  >"$here/new.json"
event="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["oid"])' "$here/new.json")"
"$dam" show "$event" --json >"$here/show-event.json"

printf 'capture the four sync documents by hand against a helper: see the header of each\n' >&2
```

The four sync documents (`push-ok.json`, `push-partial-failure.json`, `pull-ok.json`,
`pull-conflict.json`) need a remote helper, so they are written by hand from `push_json` and
`pull_json` in `crates/dam-cli/src/commands/sync.rs`, which is the shape `dam` builds them with:

`push-ok.json`:

```json
{
  "remotes": [
    { "remote": "example", "sent": 3, "succeeded": 3, "skipped": 0, "failed": [] }
  ]
}
```

`push-partial-failure.json`:

```json
{
  "remotes": [
    {
      "remote": "example",
      "sent": 3,
      "succeeded": 2,
      "skipped": 0,
      "failed": [{ "oid": "9a0b1c2", "why": "the service refused the write" }]
    }
  ]
}
```

`pull-ok.json`:

```json
{
  "remotes": [
    {
      "remote": "example",
      "created": 2,
      "updated": 1,
      "unchanged": 40,
      "conflicts": 0,
      "removed_upstream": 0
    }
  ]
}
```

`pull-conflict.json` is the same document with `"conflicts": 1`.

- [ ] **Step 2: Create the crate and capture the fixtures**

`crates/herdr-damnit-adapters/Cargo.toml`:

```toml
[package]
name = "herdr-damnit-adapters"
description = "The only spawner of dam, plus the herdr CLI, the config, the state and the clock."
edition.workspace = true
version.workspace = true
license.workspace = true
repository.workspace = true

[[bin]]
name = "fake-dam"
path = "src/bin/fake_dam.rs"

[dependencies]
herdr-damnit-application.workspace = true
herdr-damnit-domain.workspace = true
libc = "0.2"
serde.workspace = true
serde_json.workspace = true
toml = "0.9"
```

Add `"crates/herdr-damnit-adapters"` to the workspace `members` and
`herdr-damnit-adapters = { path = "crates/herdr-damnit-adapters" }` to `[workspace.dependencies]`,
then add `herdr-damnit-adapters.workspace = true` to the binary crate. `crates/herdr-damnit-adapters/src/lib.rs`
starts as one line:

```rust
//! Everything the pane touches outside itself: `dam`, `herdr`, the config, the state and the clock.
```

Then run the capture:

```bash
chmod +x crates/herdr-damnit-adapters/tests/fixtures/capture.sh
crates/herdr-damnit-adapters/tests/fixtures/capture.sh
```

- [ ] **Step 3: Write the shared scratch fixture**

Every temp path a test in this crate writes goes inside a directory that test owns, so a run leaves
nothing behind in the system temp directory. A `<name>-<pid>` path per test does not: this lane's
own runs left twenty-odd `herdr-damnit-*` files there before the fixture existed, because nothing
ever removed one.

`crates/herdr-damnit-adapters/tests/support/mod.rs`:

```rust
//! Fixtures the integration tests of this crate share.

use std::path::{Path, PathBuf};

/// A directory of one test's own, removed when the value drops. Every temp path a test writes goes
/// inside it, so a run leaves nothing behind in the system temp directory, and a panicking test
/// cleans up on the way out because `Drop` runs while the stack unwinds.
pub struct Scratch {
    dir: PathBuf,
}

impl Scratch {
    /// `name` distinguishes the scratches of one test binary from each other; the process id
    /// distinguishes concurrent binaries.
    pub fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("herdr-damnit-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a scratch directory");
        Self { dir }
    }

    // Each test binary compiles its own copy of this module, so a method only one of them calls
    // reads as dead code in the others.
    #[allow(dead_code)]
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// A path inside this scratch. Nothing outside this test writes there.
    pub fn file(&self, name: &str) -> PathBuf {
        self.dir.join(name)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}
```

Each integration test file reaches it with `mod support;` and `use support::Scratch;`. Cargo builds
one copy per test binary, which is why `dir` carries a narrow `allow(dead_code)`: a method only one
binary calls is dead code in the others.

- [ ] **Step 4: Write the failing test for the fake**

`crates/herdr-damnit-adapters/tests/fake_dam.rs`:

```rust
//! The fake `dam` every other test drives. This file proves the double itself behaves, so a
//! failure in a later test is a failure in the pane rather than in its double.

use std::path::{Path, PathBuf};
use std::process::Command;

fn fake() -> &'static str {
    env!("CARGO_BIN_EXE_fake-dam")
}

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn scratch(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("herdr-damnit-{name}-{}", std::process::id()));
    let _ = std::fs::remove_file(&path);
    path
}

#[test]
fn the_fake_replays_the_fixture_named_by_its_subcommand() {
    let output = Command::new(fake())
        .args(["status", "--json"])
        .env("FAKE_DAM_FIXTURE_DIR", fixtures())
        .env("FAKE_DAM_FIXTURE", "status-clean")
        .output()
        .expect("the fake ran");

    assert!(output.status.success());
    let document: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("the fixture is JSON");
    assert!(document.get("staged").is_some(), "{document}");
}

#[test]
fn the_fake_appends_one_json_line_of_argv_per_call() {
    let log = scratch("argv-log");
    for argv in [
        vec!["status", "--json"],
        vec!["ls", "!done", "--json"],
    ] {
        Command::new(fake())
            .args(&argv)
            .env("FAKE_DAM_LOG", &log)
            .env("FAKE_DAM_FIXTURE_DIR", fixtures())
            .env("FAKE_DAM_FIXTURE", "status-clean")
            .output()
            .expect("the fake ran");
    }

    let lines: Vec<Vec<String>> = std::fs::read_to_string(&log)
        .expect("a log")
        .lines()
        .map(|line| serde_json::from_str(line).expect("a json line"))
        .collect();
    assert_eq!(lines[0], vec!["status", "--json"]);
    assert_eq!(lines[1], vec!["ls", "!done", "--json"]);
}

/// A refusal is exit 4 and one error document on standard error, which is `dam` 0.2.0's contract
/// under `--json`.
#[test]
fn the_fake_refuses_with_the_code_and_the_document_it_was_given() {
    let document = r#"{"error":{"kind":"refused","rule":"no_such_object","message":"no object matches \"zzzzzzz\"","oids":[]}}"#;
    let output = Command::new(fake())
        .args(["show", "zzzzzzz", "--json"])
        .env("FAKE_DAM_EXIT", "4")
        .env("FAKE_DAM_STDERR", document)
        .output()
        .expect("the fake ran");

    assert_eq!(output.status.code(), Some(4));
    assert_eq!(String::from_utf8_lossy(&output.stderr).trim(), document);
    assert!(output.stdout.is_empty(), "a failing dam prints no report");
}

/// clap answers a bad command line before `dam` runs, so exit 2 carries usage text and no
/// document. The pane has to read that shape too.
#[test]
fn the_fake_can_also_answer_the_way_clap_does() {
    let output = Command::new(fake())
        .args(["ls", "--nope"])
        .env("FAKE_DAM_EXIT", "2")
        .env("FAKE_DAM_STDERR", "error: unexpected argument '--nope'")
        .output()
        .expect("the fake ran");

    assert_eq!(output.status.code(), Some(2));
    assert_eq!(
        String::from_utf8_lossy(&output.stderr).trim(),
        "error: unexpected argument '--nope'"
    );
}

#[test]
fn the_fake_sleeps_before_answering_when_it_is_asked_to() {
    let started = std::time::Instant::now();
    Command::new(fake())
        .args(["push", "--json"])
        .env("FAKE_DAM_SLEEP_MS", "150")
        .env("FAKE_DAM_FIXTURE_DIR", fixtures())
        .env("FAKE_DAM_FIXTURE", "push-ok")
        .output()
        .expect("the fake ran");

    assert!(started.elapsed() >= std::time::Duration::from_millis(150));
}

#[test]
fn a_fixture_that_is_not_there_is_an_empty_object_rather_than_a_panic() {
    let output = Command::new(fake())
        .args(["log", "--json"])
        .env("FAKE_DAM_FIXTURE_DIR", fixtures())
        .env("FAKE_DAM_FIXTURE", "nothing-like-this")
        .output()
        .expect("the fake ran");

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "{}");
}

#[test]
fn the_version_flag_answers_the_captured_line() {
    let output = Command::new(fake())
        .arg("--version")
        .env("FAKE_DAM_FIXTURE_DIR", fixtures())
        .output()
        .expect("the fake ran");

    assert!(
        String::from_utf8_lossy(&output.stdout).starts_with("dam "),
        "{:?}",
        output.stdout
    );
}
```

- [ ] **Step 5: Run the tests to verify they fail**

Run: `cargo test -p herdr-damnit-adapters --locked`
Expected: FAIL, `couldn't read src/bin/fake_dam.rs`

- [ ] **Step 6: Write the fake**

`crates/herdr-damnit-adapters/src/bin/fake_dam.rs`:

```rust
//! A stand-in for `dam`, driven entirely by its environment. Every test that needs a `dam` points
//! the pane's configured `dam` argv at this binary, so no test mangles `PATH` and no test needs a
//! real store.

use std::io::Write;
use std::path::PathBuf;

fn main() -> std::process::ExitCode {
    install_interrupt_handler();
    let argv: Vec<String> = std::env::args().skip(1).collect();
    log(&argv);

    if let Ok(millis) = std::env::var("FAKE_DAM_SLEEP_MS") {
        let millis = millis.parse().unwrap_or(0);
        std::thread::sleep(std::time::Duration::from_millis(millis));
    }

    if let Ok(code) = std::env::var("FAKE_DAM_EXIT") {
        let line = std::env::var("FAKE_DAM_STDERR").unwrap_or_default();
        if !line.is_empty() {
            eprintln!("{line}");
        }
        return std::process::ExitCode::from(code.parse::<u8>().unwrap_or(1));
    }

    if argv.first().map(String::as_str) == Some("--version") {
        print!("{}", read("version.txt").unwrap_or_else(|| "dam 0.2.0\n".to_string()));
        return std::process::ExitCode::SUCCESS;
    }

    let name = std::env::var("FAKE_DAM_FIXTURE")
        .unwrap_or_else(|_| argv.first().cloned().unwrap_or_default());
    println!(
        "{}",
        read(&format!("{name}.json"))
            .unwrap_or_else(|| "{}".to_string())
            .trim()
    );
    std::process::ExitCode::SUCCESS
}

fn fixture_dir() -> Option<PathBuf> {
    std::env::var_os("FAKE_DAM_FIXTURE_DIR").map(PathBuf::from)
}

fn read(name: &str) -> Option<String> {
    std::fs::read_to_string(fixture_dir()?.join(name)).ok()
}

/// One JSON line of full argv per call, appended so a test reads every call one run made.
fn log(argv: &[String]) {
    append(&serde_json::to_string(argv).unwrap_or_default());
}

fn append(line: &str) {
    let Some(path) = std::env::var_os("FAKE_DAM_LOG") else {
        return;
    };
    let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    else {
        return;
    };
    let _ = writeln!(file, "{line}");
}

/// `dam` maps a cancelled run to exit 3 whatever the abandoned work reported, so the fake does the
/// same and records the signal for the test to assert on.
fn install_interrupt_handler() {
    if std::env::var_os("FAKE_DAM_IGNORE_SIGINT").is_some() {
        unsafe { libc::signal(libc::SIGINT, libc::SIG_IGN) };
        return;
    }
    unsafe extern "C" fn on_interrupt(_: libc::c_int) {
        append(r#"{"signal":"SIGINT"}"#);
        std::process::exit(3);
    }
    unsafe {
        libc::signal(libc::SIGINT, on_interrupt as *const () as libc::sighandler_t);
    }
}
```

The cast through `*const ()` is load-bearing: clippy's `function_casts_as_integer` refuses a
function item cast straight to an integer type, and this crate builds under `-D warnings`. The
fifth knob, `FAKE_DAM_IGNORE_SIGINT`, plays a `dam` that never notices the interrupt, which is what
Task 22 drives the kill escalation with.

The `fake-dam` binary needs `serde_json` and `libc`, which the crate already depends on.

- [ ] **Step 7: Run the tests to verify they pass**

Run: `cargo test -p herdr-damnit-adapters --locked`
Expected: PASS, six tests.

- [ ] **Step 8: Prove the fixtures carry no personal data**

```bash
! grep -rniE 'stephen|webdavis|/Users/|todoist api|token' crates/herdr-damnit-adapters/tests/fixtures/*.json
```

Expected: no match. A fixture that matches was captured against a real store and must be recaptured
by `capture.sh`.

- [ ] **Step 9: Commit**

```bash
git add -A
SKIP_AI_COMMIT=1 git commit -m "test(adapters): add the fake dam and the fixtures it replays"
```

---

### Task 21: ProcessDamRunner, the thread and the channel

**Files:**
- Create: `crates/herdr-damnit-adapters/src/dam_runner.rs`
- Create: `crates/herdr-damnit-adapters/tests/dam_runner.rs`
- Modify: `crates/herdr-damnit-adapters/src/lib.rs`

**Interfaces:**
- Consumes: `DamRunner`, `RunningJob`, `Finished`, `SpawnError` from Task 15; the fake from Task 20.
- Produces:

```rust
pub struct ProcessDamRunner { /* the configured dam argv */ }

impl ProcessDamRunner {
    pub fn new(dam: Vec<String>) -> ProcessDamRunner;
}

impl DamRunner for ProcessDamRunner { /* ... */ }
```

`spawn` starts the child in its own process group, hands the wait to a `std::thread`, and returns
immediately with a receiver the draw loop polls. The thread runs `Child::wait_with_output`, which
reads both pipes to end of file, and sends one `Finished`. A spawn whose error kind is `NotFound`
becomes `SpawnError::NotFound`, which is what draws the install line.

The pane makes no HTTP request, so `tokio` and `reqwest` are not in this crate's tree and never
enter it.

- [ ] **Step 1: Write the failing tests**

`crates/herdr-damnit-adapters/tests/dam_runner.rs`:

```rust
use std::path::{Path, PathBuf};
use std::time::Duration;

use herdr_damnit_adapters::ProcessDamRunner;
use herdr_damnit_application::{DamRunner, SpawnError};

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn runner() -> ProcessDamRunner {
    ProcessDamRunner::new(vec![env!("CARGO_BIN_EXE_fake-dam").to_string()])
}

fn words(argv: &[&str]) -> Vec<String> {
    argv.iter().map(|word| word.to_string()).collect()
}

#[test]
fn a_run_answers_once_with_its_code_and_its_output() {
    unsafe {
        std::env::set_var("FAKE_DAM_FIXTURE_DIR", fixtures());
        std::env::set_var("FAKE_DAM_FIXTURE", "status-clean");
    }
    let job = runner()
        .spawn(&words(&["status", "--json"]))
        .expect("it spawned");

    let finished = job
        .results
        .recv_timeout(Duration::from_secs(5))
        .expect("one result");
    assert_eq!(finished.code, Some(0));
    assert!(finished.stdout.contains("\"staged\""), "{}", finished.stdout);
    assert!(finished.stderr.is_empty());
}

#[test]
fn spawning_does_not_wait_for_the_child() {
    unsafe {
        std::env::set_var("FAKE_DAM_SLEEP_MS", "400");
        std::env::set_var("FAKE_DAM_FIXTURE_DIR", fixtures());
        std::env::set_var("FAKE_DAM_FIXTURE", "push-ok");
    }
    let started = std::time::Instant::now();
    let job = runner().spawn(&words(&["push", "--json"])).expect("it spawned");
    let returned = started.elapsed();

    assert!(returned < Duration::from_millis(200), "spawn blocked for {returned:?}");
    job.results
        .recv_timeout(Duration::from_secs(5))
        .expect("one result");
    unsafe { std::env::remove_var("FAKE_DAM_SLEEP_MS") };
}

#[test]
fn a_failing_run_carries_its_code_and_its_standard_error() {
    let document = r#"{"error":{"kind":"refused","rule":"no_such_object","message":"no object matches \"zzzzzzz\"","oids":[]}}"#;
    unsafe {
        std::env::set_var("FAKE_DAM_EXIT", "4");
        std::env::set_var("FAKE_DAM_STDERR", document);
    }
    let job = runner()
        .spawn(&words(&["show", "zzzzzzz", "--json"]))
        .expect("it spawned");
    let finished = job
        .results
        .recv_timeout(Duration::from_secs(5))
        .expect("one result");

    assert_eq!(finished.code, Some(4));
    assert_eq!(finished.stderr.trim(), document);
    unsafe {
        std::env::remove_var("FAKE_DAM_EXIT");
        std::env::remove_var("FAKE_DAM_STDERR");
    }
}

#[test]
fn a_dam_that_is_not_there_is_a_not_found_rather_than_an_error_string() {
    let runner = ProcessDamRunner::new(vec!["no-such-dam-anywhere".to_string()]);
    assert!(matches!(
        runner.spawn(&words(&["status"])),
        Err(SpawnError::NotFound)
    ));
}

#[test]
fn the_configured_argv_leads_and_the_commands_arguments_follow_it() {
    let log = std::env::temp_dir().join(format!("herdr-damnit-runner-{}", std::process::id()));
    let _ = std::fs::remove_file(&log);
    unsafe {
        std::env::set_var("FAKE_DAM_LOG", &log);
        std::env::set_var("FAKE_DAM_FIXTURE_DIR", fixtures());
        std::env::set_var("FAKE_DAM_FIXTURE", "ls");
    }
    let job = runner()
        .spawn(&words(&["ls", "!done", "--json"]))
        .expect("it spawned");
    job.results
        .recv_timeout(Duration::from_secs(5))
        .expect("one result");

    let line = std::fs::read_to_string(&log).expect("a log");
    let argv: Vec<String> = serde_json::from_str(line.lines().next().expect("a line")).expect("json");
    assert_eq!(argv, words(&["ls", "!done", "--json"]));
    unsafe { std::env::remove_var("FAKE_DAM_LOG") };
}
```

Each test sets the fake's environment on the process it shares with every other test in the file, so
every test in it opens by taking one file-local lock that also clears all five knobs:

```rust
/// The fake's knobs live in this process's environment, which every test in this file shares.
/// A test holds this lock for its whole body and starts from a cleared environment.
fn fake_env() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let guard = LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    for knob in [
        "FAKE_DAM_FIXTURE_DIR",
        "FAKE_DAM_FIXTURE",
        "FAKE_DAM_EXIT",
        "FAKE_DAM_STDERR",
        "FAKE_DAM_SLEEP_MS",
        "FAKE_DAM_LOG",
        "FAKE_DAM_IGNORE_SIGINT",
    ] {
        unsafe { std::env::remove_var(knob) };
    }
    guard
}
```

The lock rather than `--test-threads=1`, because CI runs the plain
`cargo test --workspace --locked` and nothing there passes that flag; the poison recovery is what
keeps one failing test from cascading into the rest of the file. Clearing on entry rather than on
exit is the other half: a test that panics mid-body never reaches its own cleanup, and under the
plan's original per-test cleanup that leaked `FAKE_DAM_EXIT` into every later test in the file.

Two more tests beyond the plan's five. The argv test above has to hand `ProcessDamRunner::new` a
configured argv of **two** words, one of them a leading argument, or it cannot fail when `spawn`
drops `.args(leading)`: with a one-word argv the assertion holds either way. And an argv with no
word in it at all is its own arm, `split_first().ok_or(SpawnError::NotFound)`, which needs
`an_empty_argv_is_a_not_found_rather_than_a_panic`.

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p herdr-damnit-adapters --locked --test dam_runner -- --test-threads=1`
Expected: FAIL with `unresolved import 'herdr_damnit_adapters::ProcessDamRunner'`

- [ ] **Step 3: Write the runner**

`crates/herdr-damnit-adapters/src/dam_runner.rs`:

```rust
//! The only place in this repository that names `dam` to `std::process::Command`. Every call runs
//! on its own thread and answers on a channel, so the draw loop never waits on one.

use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};
use std::sync::mpsc::channel;
use std::time::Instant;

use herdr_damnit_application::{DamRunner, Finished, RunningJob, SpawnError};

mod cancel;

pub use cancel::Cancel;

pub struct ProcessDamRunner {
    dam: Vec<String>,
}

impl ProcessDamRunner {
    /// The argv the config names, `["dam"]` by default, which a test points at its own fake.
    pub fn new(dam: Vec<String>) -> Self {
        Self { dam }
    }
}

impl DamRunner for ProcessDamRunner {
    fn spawn(&self, argv: &[String]) -> Result<RunningJob, SpawnError> {
        let (binary, leading) = self.dam.split_first().ok_or(SpawnError::NotFound)?;
        let mut command = Command::new(binary);
        command
            .args(leading)
            .args(argv)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        // Its own process group, so a signal reaches the helper `dam` spawned as well as `dam`.
        command.process_group(0);
        let child = command.spawn().map_err(|error| match error.kind() {
            std::io::ErrorKind::NotFound => SpawnError::NotFound,
            _ => SpawnError::Io(error.to_string()),
        })?;

        let cancel = Cancel::of(child.id());
        let (sender, results) = channel();
        let started = Instant::now();
        std::thread::spawn(move || {
            // `wait_with_output` reads both pipes to end of file, so a child writing more than a
            // pipe buffer cannot deadlock against a parent that is not reading.
            let finished = match child.wait_with_output() {
                Ok(output) => Finished {
                    code: output.status.code(),
                    stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                    stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                    elapsed: started.elapsed(),
                },
                Err(error) => Finished {
                    code: None,
                    stdout: String::new(),
                    stderr: error.to_string(),
                    elapsed: started.elapsed(),
                },
            };
            let _ = sender.send(finished);
        });

        Ok(RunningJob {
            cancel: Box::new(move || cancel.interrupt()),
            results,
        })
    }
}
```

`crates/herdr-damnit-adapters/src/dam_runner/cancel.rs` starts as the smallest thing that compiles;
Task 22 fills it in:

```rust
//! Signalling a running `dam` and the group it spawned its helper into.

#[derive(Clone, Copy, Debug)]
pub struct Cancel {
    group: i32,
}

impl Cancel {
    pub fn of(pid: u32) -> Self {
        Self { group: pid as i32 }
    }

    pub fn interrupt(self) {
        unsafe { libc::kill(-self.group, libc::SIGINT) };
    }
}
```

Add to `crates/herdr-damnit-adapters/src/lib.rs`:

```rust
mod dam_runner;

pub use dam_runner::{Cancel, ProcessDamRunner};
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p herdr-damnit-adapters --locked --test dam_runner -- --test-threads=1`
Expected: PASS, five tests.

- [ ] **Step 5: Prove no HTTP client entered this crate**

```bash
! cargo tree -p herdr-damnit-adapters --locked --edges normal | grep -E 'reqwest|hyper|rustls|tokio'
```

Expected: no match.

- [ ] **Step 6: Commit**

```bash
git add -A
SKIP_AI_COMMIT=1 git commit -m "feat(adapters): spawn dam on a thread and answer the draw loop on a channel"
```

---

### Task 22: Cancellation and the read deadline

**Files:**
- Modify: `crates/herdr-damnit-adapters/src/dam_runner/cancel.rs`
- Modify: `crates/herdr-damnit-adapters/src/dam_runner.rs`
- Create: `crates/herdr-damnit-adapters/tests/cancel.rs`

**Interfaces:**
- Consumes: `Cancel` and `ProcessDamRunner` from Task 21.
- Produces:
  - `Cancel::interrupt(self)`, which sends `SIGINT` to the child's whole process group, waits a
    two-second grace in 50 ms ticks, and sends `SIGKILL` to the group if the child is still there.
  - `ProcessDamRunner::spawn_with_deadline(&self, argv: &[String], deadline: Option<Duration>)`,
    which cancels the job itself once the deadline passes, and `spawn` calling it with `None`.
  - `Finished::code` of `Some(3)` for a `dam` that took the interrupt, and `None` for one that took
    the kill.

The signal is `SIGINT` rather than `SIGTERM` because `dam`'s `main` installs a `SIGINT` handler that
sets a cancellation flag, every wait loop reads it, the helper conversation checks it on a 25 ms tick
and kills its child before returning, and `main` maps a cancelled run to exit 3. `dam` installs no
`SIGTERM` handler, so a `SIGTERM` is death at an arbitrary point with no chance to reap the helper.
The group matters because `dam push` spawns the remote helper, which is where the time actually goes.

**Nothing in this file waits for a guessed interval.** The block below is the file as it shipped,
so it is safe to copy verbatim. Five properties of it are deliberate and each was measured:

1. **No elapsed budget is asserted.** `finished.elapsed < Duration::from_secs(2)` and its
   one-second sibling are wall-clock upper bounds on a machine that runs many agents at once. What
   they are for, proving the cancellation fired rather than the fake finishing on its own, is
   carried instead by a fake that sleeps far past every deadline and grace under test
   (`FAKE_DAM_SLEEP_MS` of 30000) against a `recv_timeout` of 20 seconds: a run that was not
   cancelled times out and fails. That is both stricter and load-proof.
1. **`sleep(150ms)` before cancelling becomes a blocking wait on the fake's own argv line.** The
   fake logs its argv before it sleeps, so a line in the log proves the child is past its startup
   and has its handler installed. Under the sleep a loaded machine can interrupt a child that has
   not installed one yet, whose default action kills it: code `None` rather than `Some(3)`.
1. **The kill half of this task's own interface gains a test.** `Finished::code` of `None` for a
   `dam` that took the kill is stated above and pinned by nothing. It cannot be driven through
   `spawn`, because the production grace is two seconds and no test here may take that long, so
   `Cancel` takes its grace at construction: `Cancel::of(pid)` keeps the constant and
   `Cancel::with_grace(pid, grace)` is what the test drives, with a 100 ms grace and the fake in
   its `FAKE_DAM_IGNORE_SIGINT` mode.

1. **Every fake gets a log of its own and no pipe at all.** A `Fake` owns its child, its log path
   and a name, so a wait says which child is missing rather than reporting a total across both, and
   a child that exited without logging is reported at once rather than waited out. Its standard
   output and standard error are `Stdio::null()`, because nothing here reads them: a fake cannot
   stall on a pipe buffer this file would never drain. This closes a flake observed once at 20.41 s
   under a full-workspace build load, where the shared-log wait could only report `N of 2 lines`.
1. **Every wait on a signalled fake is bounded** by `SIGNAL_PATIENCE`, five seconds, and the
   timeout path kills the child. An unbounded `wait` on a fake that sleeps `FOREVER_MS` turns a
   broken signal ladder into a suite that hangs for thirty seconds: measured 31.04 s with the
   `SIGKILL` deleted, against 6.46 s and the message `the deaf dam was still running 5s after the
   signal` once bounded. `PATIENCE` stays at twenty seconds for the waits that are load-sensitive
   rather than ladder-sensitive.

Four tests beyond the plan's three: the kill escalation above, a group that has already gone (the
`alive` poll's early return, so an interrupted `dam` is never killed on top of the interrupt),
`the_interrupt_reaches_the_group_rather_than_its_leader_alone` over a second fake joined to the
first's group with `process_group(leader)`, which is the only thing that catches a `kill` sent to a
positive pid, and a job with no deadline at all, which is what `spawn` hands down.

Every test in the file runs in under a second, measured one at a time: 0.05 s to 0.52 s.

- [ ] **Step 1: Write the failing tests**

`crates/herdr-damnit-adapters/tests/cancel.rs`:

```rust
//! Cancelling a `dam` run, both ways it happens: the pane asking, and a read outliving its
//! deadline. Nothing here waits for a guessed interval: a test blocks on the fake's own argv line
//! to know the child is running, and on the result channel to know it answered. A loaded machine
//! makes these tests slower and never flakier.

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

mod support;

use herdr_damnit_adapters::{Cancel, ProcessDamRunner};
use herdr_damnit_application::DamRunner;
use std::os::unix::process::CommandExt;
use support::Scratch;

/// Longer than any fake in this file needs, so a machine under load waits rather than fails.
const PATIENCE: Duration = Duration::from_secs(20);

/// How long a fake meant to be cancelled pretends to work for. Well past every deadline and grace
/// under test, so a passing test is proof the cancellation fired rather than that the fake
/// happened to finish.
const FOREVER_MS: &str = "30000";

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn runner() -> ProcessDamRunner {
    ProcessDamRunner::new(vec![env!("CARGO_BIN_EXE_fake-dam").to_string()])
}

fn words(argv: &[&str]) -> Vec<String> {
    argv.iter().map(|word| word.to_string()).collect()
}

/// The fake's knobs live in this process's environment, which every test in this file shares.
/// A test holds this lock for its whole body and starts from a cleared environment, so the file
/// is correct under any thread count rather than only under `--test-threads=1`.
fn fake_env() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let guard = LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    for knob in [
        "FAKE_DAM_FIXTURE_DIR",
        "FAKE_DAM_FIXTURE",
        "FAKE_DAM_EXIT",
        "FAKE_DAM_STDERR",
        "FAKE_DAM_SLEEP_MS",
        "FAKE_DAM_LOG",
        "FAKE_DAM_IGNORE_SIGINT",
    ] {
        unsafe { std::env::remove_var(knob) };
    }
    guard
}

fn set(knob: &str, value: impl AsRef<std::ffi::OsStr>) {
    unsafe { std::env::set_var(knob, value) };
}

/// How long a signalled fake gets to actually exit. Short next to `PATIENCE` on purpose: past it
/// the signal ladder is broken, and a broken ladder must read as a red rather than as a suite that
/// hangs until the fake's own sleep runs out.
const SIGNAL_PATIENCE: Duration = Duration::from_secs(5);

/// The gap between two looks at an observable event. Nothing here sleeps in place of
/// synchronization; this is only how often a wait re-reads what it is waiting on.
const POLL: Duration = Duration::from_millis(5);

/// One fake, the log it writes its own argv line to, and a name for the failure messages. Each
/// fake gets a log of its own so a wait can say which child is missing rather than reporting a
/// total across both.
struct Fake {
    name: &'static str,
    child: Child,
    log: PathBuf,
}

impl Fake {
    /// Spawn a fake into the process group `group` names, `0` making it a leader of its own.
    fn spawn(name: &'static str, scratch: &Scratch, group: i32) -> Self {
        let log = scratch.file(&format!("{name}.jsonl"));
        set("FAKE_DAM_LOG", &log);
        let child = Command::new(env!("CARGO_BIN_EXE_fake-dam"))
            .args(["push", "--json"])
            .stdin(Stdio::null())
            // Nothing here reads the child's output, so it is given no pipe: a fake cannot stall
            // on a buffer this test would never drain.
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .process_group(group)
            .spawn()
            .expect("it spawned");
        Self { name, child, log }
    }

    fn pid(&self) -> u32 {
        self.child.id()
    }

    /// Block until this fake has logged its argv line, which proves it is past its own startup and
    /// has its interrupt handler installed. A fake that exited without logging is reported at once
    /// rather than waited out, because there is nothing left to wait for.
    fn await_start(&mut self) {
        let deadline = Instant::now() + PATIENCE;
        while Instant::now() < deadline {
            if logged(&self.log) >= 1 {
                return;
            }
            if let Some(status) = self.child.try_wait().expect("the child's state") {
                panic!("{} exited {status} before logging its argv", self.name);
            }
            std::thread::sleep(POLL);
        }
        panic!("{} logged no argv within {PATIENCE:?}", self.name);
    }

    /// This fake's exit status, or a failure naming it once `SIGNAL_PATIENCE` runs out. The
    /// timeout path kills the child so a failing run leaves nothing sleeping behind it.
    fn await_exit(&mut self) -> std::process::ExitStatus {
        let deadline = Instant::now() + SIGNAL_PATIENCE;
        while Instant::now() < deadline {
            if let Some(status) = self.child.try_wait().expect("the child's state") {
                return status;
            }
            std::thread::sleep(POLL);
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
        panic!(
            "{} was still running {SIGNAL_PATIENCE:?} after the signal",
            self.name
        );
    }

    fn interrupts(&self) -> usize {
        interrupts(&self.log)
    }
}

/// Block until `log` holds one argv line, for a child this test did not spawn itself and so cannot
/// ask about: `RunningJob` hands out a cancel and a receiver, never a pid.
fn await_line(log: &Path, whose: &str) {
    let deadline = Instant::now() + PATIENCE;
    while Instant::now() < deadline {
        if logged(log) >= 1 {
            return;
        }
        std::thread::sleep(POLL);
    }
    panic!("{whose} logged no argv within {PATIENCE:?}");
}

fn logged(log: &Path) -> usize {
    std::fs::read_to_string(log)
        .map(|text| text.lines().filter(|line| !line.trim().is_empty()).count())
        .unwrap_or(0)
}

fn interrupts(log: &Path) -> usize {
    std::fs::read_to_string(log)
        .expect("a log")
        .matches("SIGINT")
        .count()
}

#[test]
fn cancelling_sends_sigint_and_dam_answers_with_its_cancelled_code() {
    let _env = fake_env();
    let scratch = Scratch::new("cancel-log");
    let log = scratch.file("argv.jsonl");
    set("FAKE_DAM_LOG", &log);
    set("FAKE_DAM_SLEEP_MS", FOREVER_MS);
    set("FAKE_DAM_FIXTURE_DIR", fixtures());
    set("FAKE_DAM_FIXTURE", "push-ok");

    let job = runner()
        .spawn(&words(&["push", "--json"]))
        .expect("it spawned");
    await_line(&log, "the push");
    (job.cancel)();

    let finished = job.results.recv_timeout(PATIENCE).expect("one result");
    assert_eq!(finished.code, Some(3));
    assert_eq!(interrupts(&log), 1, "the fake recorded no interrupt");
}

#[test]
fn a_read_past_its_deadline_is_cancelled_without_the_pane_asking() {
    let _env = fake_env();
    set("FAKE_DAM_SLEEP_MS", FOREVER_MS);
    set("FAKE_DAM_FIXTURE_DIR", fixtures());
    set("FAKE_DAM_FIXTURE", "ls");

    let job = runner()
        .spawn_with_deadline(
            &words(&["ls", "!done", "--json"]),
            Some(Duration::from_millis(200)),
        )
        .expect("it spawned");

    assert_eq!(
        job.results.recv_timeout(PATIENCE).expect("one result").code,
        Some(3)
    );
}

#[test]
fn a_job_inside_its_deadline_is_left_alone() {
    let _env = fake_env();
    set("FAKE_DAM_FIXTURE_DIR", fixtures());
    set("FAKE_DAM_FIXTURE", "status-clean");

    let job = runner()
        .spawn_with_deadline(&words(&["status", "--json"]), Some(PATIENCE))
        .expect("it spawned");

    let finished = job.results.recv_timeout(PATIENCE).expect("one result");
    assert_eq!(finished.code, Some(0));
    assert!(
        finished.stdout.contains("\"staged\""),
        "{}",
        finished.stdout
    );
}

/// A run with no deadline is never cancelled on its own, which is what an exclusive job gets:
/// `dam`'s own per-remote deadline bounds it.
#[test]
fn a_job_with_no_deadline_answers_for_itself() {
    let _env = fake_env();
    set("FAKE_DAM_FIXTURE_DIR", fixtures());
    set("FAKE_DAM_FIXTURE", "push-ok");

    let job = runner()
        .spawn_with_deadline(&words(&["push", "--json"]), None)
        .expect("it spawned");

    assert_eq!(
        job.results.recv_timeout(PATIENCE).expect("one result").code,
        Some(0)
    );
}

/// The signal reaches the whole process group, which is where `dam` puts the remote helper that
/// is doing the waiting, so a second member of the group takes it too.
#[test]
fn the_interrupt_reaches_the_group_rather_than_its_leader_alone() {
    let _env = fake_env();
    let scratch = Scratch::new("group");
    set("FAKE_DAM_SLEEP_MS", FOREVER_MS);

    let mut leader = Fake::spawn("the leader", &scratch, 0);
    let group = i32::try_from(leader.pid()).expect("a pid");
    let mut helper = Fake::spawn("the helper", &scratch, group);
    leader.await_start();
    helper.await_start();

    Cancel::of(leader.pid()).interrupt();

    assert_eq!(leader.await_exit().code(), Some(3));
    assert_eq!(helper.await_exit().code(), Some(3));
    assert_eq!(leader.interrupts(), 1, "the leader took no interrupt");
    assert_eq!(helper.interrupts(), 1, "the helper was left running");
}

/// A `dam` deaf to `SIGINT` is killed once the grace runs out, and a killed child carries no exit
/// code at all. Driven at `Cancel` with a grace of its own, because the production grace is two
/// seconds and no test in this repository may take that long.
#[test]
fn a_dam_that_ignores_the_interrupt_is_killed_and_carries_no_code() {
    let _env = fake_env();
    let scratch = Scratch::new("kill");
    set("FAKE_DAM_SLEEP_MS", FOREVER_MS);
    set("FAKE_DAM_IGNORE_SIGINT", "1");

    let mut deaf = Fake::spawn("the deaf dam", &scratch, 0);
    deaf.await_start();

    let started = Instant::now();
    Cancel::with_grace(deaf.pid(), Duration::from_millis(100)).interrupt();

    assert_eq!(
        deaf.await_exit().code(),
        None,
        "a killed child reports a signal rather than a code"
    );
    assert!(
        started.elapsed() >= Duration::from_millis(100),
        "the kill skipped the grace: {:?}",
        started.elapsed()
    );
    assert_eq!(deaf.interrupts(), 0, "the fake was not deaf after all");
}

/// A group that has already gone is not waited out, which is what the grace loop polls for so an
/// interrupted `dam` is never killed on top of the interrupt it already took.
#[test]
fn a_group_that_has_already_gone_is_not_waited_out() {
    let _env = fake_env();
    let scratch = Scratch::new("already-gone");
    let mut gone = Fake::spawn("the finished dam", &scratch, 0);
    let cancel = Cancel::with_grace(gone.pid(), Duration::from_secs(20));
    gone.await_exit();

    let started = Instant::now();
    cancel.interrupt();

    assert!(
        started.elapsed() < Duration::from_secs(2),
        "interrupt waited out a grace nobody needed: {:?}",
        started.elapsed()
    );
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p herdr-damnit-adapters --locked --test cancel -- --test-threads=1`
Expected: FAIL with `no method named 'spawn_with_deadline'`

- [ ] **Step 3: Write the cancellation**

`crates/herdr-damnit-adapters/src/dam_runner/cancel.rs`:

```rust
//! Signalling a running `dam` and the group it spawned its helper into.
//!
//! `SIGINT` rather than `SIGTERM`: `dam` installs a `SIGINT` handler that sets a cancellation flag,
//! its wait loops read it, the helper conversation kills its own child before returning, and `dam`
//! maps a cancelled run to exit 3. It installs no `SIGTERM` handler.

use std::time::{Duration, Instant};

/// How long a `dam` gets to notice the interrupt and reap its helper before it is killed.
const GRACE: Duration = Duration::from_secs(2);

/// How often the grace is checked. Bash has no wait-with-timeout and neither does `std`, so this
/// is a poll.
const TICK: Duration = Duration::from_millis(50);

#[derive(Clone, Copy, Debug)]
pub struct Cancel {
    group: i32,
    grace: Duration,
}

impl Cancel {
    pub fn of(pid: u32) -> Self {
        Self::with_grace(pid, GRACE)
    }

    /// The same signalling with a grace of the caller's choosing, which is how a test drives the
    /// escalation without waiting out the production one.
    pub fn with_grace(pid: u32, grace: Duration) -> Self {
        Self {
            group: pid as i32,
            grace,
        }
    }

    /// `SIGINT` to the group, the grace, then `SIGKILL` to the group.
    pub fn interrupt(self) {
        self.signal(libc::SIGINT);
        let deadline = Instant::now() + self.grace;
        while Instant::now() < deadline {
            if !self.alive() {
                return;
            }
            std::thread::sleep(TICK);
        }
        self.signal(libc::SIGKILL);
    }

    /// A negative pid names the whole process group, which is where the remote helper is.
    fn signal(self, signal: libc::c_int) {
        unsafe { libc::kill(-self.group, signal) };
    }

    /// Signal 0 delivers nothing and only reports whether the group is still there.
    fn alive(self) -> bool {
        unsafe { libc::kill(-self.group, 0) == 0 }
    }
}
```

- [ ] **Step 4: Add the deadline to the runner**

In `crates/herdr-damnit-adapters/src/dam_runner.rs`, rename the body of `spawn` to
`spawn_with_deadline` and give it the deadline thread:

```rust
impl ProcessDamRunner {
    /// A run that cancels itself once `deadline` passes. Reads carry one; an exclusive job does
    /// not, because `dam`'s own per-remote deadline bounds it.
    pub fn spawn_with_deadline(
        &self,
        argv: &[String],
        deadline: Option<std::time::Duration>,
    ) -> Result<RunningJob, SpawnError> {
        // ... the body Task 21 wrote, up to the `Cancel::of` line ...
        if let Some(deadline) = deadline {
            std::thread::spawn(move || {
                std::thread::sleep(deadline);
                if cancel.alive_group() {
                    cancel.interrupt();
                }
            });
        }
        // ... the wait thread and the RunningJob, unchanged ...
    }
}

impl DamRunner for ProcessDamRunner {
    fn spawn(&self, argv: &[String]) -> Result<RunningJob, SpawnError> {
        self.spawn_with_deadline(argv, None)
    }
}
```

`Cancel` needs one more public method for that check:

```rust
    /// Whether the group is still running, which is what a deadline thread asks before signalling.
    pub fn alive_group(self) -> bool {
        self.alive()
    }
```

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test -p herdr-damnit-adapters --locked -- --test-threads=1`
Expected: PASS, every test in the crate.

- [ ] **Step 6: Commit**

```bash
git add -A
SKIP_AI_COMMIT=1 git commit -m "feat(adapters): cancel a dam run with SIGINT to its group and a deadline of its own"
```

---

### Task 23: dam's wire documents, mapped into the domain

**Files:**
- Create: `crates/herdr-damnit-adapters/src/wire.rs`
- Create: `crates/herdr-damnit-adapters/src/wire/tests.rs`
- Modify: `crates/herdr-damnit-adapters/src/lib.rs`

**Interfaces:**
- Consumes: `Object`, `Stage`, `Change`, `Conflict`, `Notice`, `Unpushed`, `Op`, `Oid`, `Priority`,
  `parse_date` from Phase B; the fixtures from Task 20.
- Produces:

```rust
pub fn objects(json: &str) -> Result<Vec<Object>, String>;   // dam ls --json
pub fn object(json: &str) -> Result<Object, String>;          // dam show <oid> --json
pub fn stage(json: &str) -> Result<Stage, String>;            // dam status --json
pub fn completions(json: &str) -> Result<HashMap<Oid, Date>, String>;  // dam log --json
pub fn push_summary(json: &str) -> Result<String, String>;    // dam push --json
pub fn pull_summary(json: &str) -> Result<String, String>;    // dam pull --json
pub fn error_document(stderr: &str) -> Option<ErrorDocument>; // standard error under --json
```

The serde structs here mirror `dam`'s own `WireObject`, `WireTask` and `WireEvent`
(`crates/dam-protocol/src/wire.rs`) and the four `json!` literals in `dam`'s CLI
(`status.rs` for the five status arrays, `output.rs` for `{"objects": [...]}`, `commit.rs` for
`{"commits": [...]}` and `sync.rs` for `{"remotes": [...]}`). Every one of them is read from a
fixture in the tests, so a `dam` change that moves a byte fails here.

`push_summary` and `pull_summary` rebuild `dam`'s own lines from the JSON rather than parsing its
human output: `todoist: 3 sent, 3 ok, 0 failed, 0 skipped` and `todoist: 2 new, 1 updated, 40
unchanged, 0 conflict(s), 0 removed upstream`.

`error_document` is the one function that reads standard error rather than standard output. Under
`--json` a failure prints exactly one document there and nothing else, so the parse is of the whole
stream. It answers `None` for anything that is not that document, which is clap's usage text at exit
2 and a `dam` too old to print one; Task 14's `classify` falls back to the first line in that case.
`kind` and `rule` are mapped onto the domain's own enums, and a `rule` word this pane has not heard
of becomes `Rule::Unknown` rather than an error, so a rule `dam` adds still reaches the status line.

`Change::fields` is `dam`'s own `fields` array, mapped straight across. Since 0.2.0 every change
document carries it: an update names the fields that moved, a create names the fields the new object
carries beyond its defaults, and a delete names none.

`completions` is how the Done screen gets its dates, and it reads `dam log --json`, whose change
documents carry no `before` and `after` either. The commit that completed a task is the one whose
change names `done` among its `fields` and whose row carries `done` of true. The date is that
change's own `completed_at`, which `dam` 0.2.0 sets when `done` becomes true, and the commit's `at`
for a `dam` that sends none. A task completed in the working layer and not yet committed has no
commit and therefore no date.

**Both halves of that gate are load-bearing and each needs its own test.** Dropping the `fields`
check makes every later edit of an already-complete task a fresh completion, which re-dates it;
dropping the `done` check makes a `dam edit --undone` a completion, because a reopen is a change
that names `done` and leaves the task open (measured: `fields ["done"]`, `done false`,
`completed_at null`). A mutation sweep found both survivors against the plan's own test set.

- [ ] **Step 1: Write the failing tests**

`crates/herdr-damnit-adapters/src/wire/tests.rs`:

```rust
use super::*;

fn fixture(name: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

#[test]
fn a_listing_becomes_objects_with_their_oids_paths_and_priorities() {
    let objects = objects(&fixture("ls.json")).expect("it parsed");

    assert_eq!(objects.len(), 3);
    let bump = objects
        .iter()
        .find(|object| object.subject == "ship the pin bump")
        .expect("the captured task");
    assert_eq!(bump.path, "proj/dotfiles/");
    assert_eq!(bump.priority().get(), 1);
    assert_eq!(bump.due(), herdr_damnit_domain::parse_date("2026-09-18"));
    assert!(!bump.is_done());
}

#[test]
fn an_empty_listing_is_no_objects_rather_than_an_error() {
    assert!(objects(&fixture("ls-empty.json")).expect("it parsed").is_empty());
}

#[test]
fn a_done_listing_carries_completed_tasks() {
    let objects = objects(&fixture("ls-done.json")).expect("it parsed");
    assert!(objects.iter().all(herdr_damnit_domain::Object::is_done), "{objects:?}");
}

#[test]
fn one_object_is_read_from_show() {
    let object = object(&fixture("show-task.json")).expect("it parsed");
    assert_eq!(object.kind, herdr_damnit_domain::Kind::Task);
    assert!(!object.oid.as_str().is_empty());
}

#[test]
fn an_event_carries_its_start_end_status_and_transparency() {
    let object = object(&fixture("show-event.json")).expect("it parsed");
    let event = object.event.as_ref().expect("the event fields");
    assert!(!event.start.is_empty());
    assert!(!event.end.is_empty());
    assert!(!event.status.is_empty());
    assert!(!event.transparency.is_empty());
}

#[test]
fn a_clean_status_is_a_clean_stage() {
    assert!(stage(&fixture("status-clean.json")).expect("it parsed").is_clean());
}

#[test]
fn a_full_status_fills_every_array_it_carries() {
    let stage = stage(&fixture("status-full.json")).expect("it parsed");
    assert!(!stage.is_clean());
    assert!(!stage.unstaged.is_empty(), "the captured store had working changes");
    assert!(
        stage.unstaged.iter().all(|change| !change.subject.is_empty()),
        "a change lost its subject"
    );
}

#[test]
fn an_operation_word_becomes_its_own_variant() {
    let document = r#"{"staged":[
      {"oid":"1","op":"create","before":null,"after":{"oid":"1","kind":"task","subject":"a"}},
      {"oid":"2","op":"update","before":{"oid":"2","kind":"task","subject":"b"},
       "after":{"oid":"2","kind":"task","subject":"b"}},
      {"oid":"3","op":"delete","before":{"oid":"3","kind":"task","subject":"c"},"after":null}
    ],"unstaged":[],"conflicts":[],"notices":[],"unpushed":[]}"#;
    let ops: Vec<Op> = stage(document)
        .expect("it parsed")
        .staged
        .into_iter()
        .map(|change| change.op)
        .collect();
    assert_eq!(ops, vec![Op::Create, Op::Update, Op::Delete]);
}

#[test]
fn a_conflict_and_a_notice_carry_the_text_the_status_screen_draws() {
    let document = r#"{"staged":[],"unstaged":[],"unpushed":[{"remote":"example","commits":2}],
      "conflicts":[{"oid":"3d4e5f6","remote":"example",
        "ours":{"oid":"3d4e5f6","kind":"task","subject":"mine"},
        "theirs":{"oid":"3d4e5f6","kind":"task","subject":"theirs"}}],
      "notices":[{"kind":"pull_failed","remote":"example","why":"the service is unreachable"}]}"#;
    let stage = stage(document).expect("it parsed");

    assert_eq!(stage.conflicts[0].ours, "mine");
    assert_eq!(stage.conflicts[0].theirs, "theirs");
    assert_eq!(stage.unpushed[0].commits, 2);
    assert!(
        stage.unpushed[0].oids.is_empty(),
        "a dam that sends no oids leaves the set empty"
    );
    assert_eq!(stage.notices[0].kind, "pull_failed");
    assert_eq!(
        stage.notices[0].message,
        "example: pull failed: the service is unreachable"
    );
}

#[test]
fn a_removed_upstream_notice_reads_as_a_sentence_rather_than_a_kind() {
    let document = r#"{"staged":[],"unstaged":[],"conflicts":[],"unpushed":[],
      "notices":[{"kind":"removed_upstream","remote":"example","oid":"7a8b9c0",
                  "subject":"old task"}]}"#;
    assert_eq!(
        stage(document).expect("it parsed").notices[0].message,
        "removed on example: \"old task\" is kept here"
    );
}

#[test]
fn a_log_names_the_day_each_completion_was_committed_on() {
    let document = r#"{"commits":[
      {"id":"c1","at":"2026-09-19T08:00:00Z","message":"first","changes":[
        {"oid":"1","op":"create","fields":["subject"],"kind":"task","subject":"a",
         "done":false,"completed_at":null,"priority":4}]},
      {"id":"c2","at":"2026-09-20T09:30:00Z","message":"second","changes":[
        {"oid":"1","op":"update","fields":["done"],"kind":"task","subject":"a",
         "done":true,"completed_at":"2026-09-20T09:29:11Z","priority":4}]}
    ]}"#;
    let completed = completions(document).expect("it parsed");

    assert_eq!(
        completed.get(&herdr_damnit_domain::Oid::new("1")),
        herdr_damnit_domain::parse_date("2026-09-20").as_ref()
    );
}

#[test]
fn a_task_that_was_never_completed_has_no_date() {
    assert!(completions(&fixture("log.json")).expect("it parsed").is_empty());
}

#[test]
fn the_sync_summaries_are_rebuilt_from_json_rather_than_read_from_human_output() {
    assert_eq!(
        push_summary(&fixture("push-ok.json")).expect("it parsed"),
        "example: 3 sent, 3 ok, 0 failed, 0 skipped"
    );
    assert_eq!(
        push_summary(&fixture("push-partial-failure.json")).expect("it parsed"),
        "example: 3 sent, 2 ok, 1 failed, 0 skipped"
    );
    assert_eq!(
        pull_summary(&fixture("pull-ok.json")).expect("it parsed"),
        "example: 2 new, 1 updated, 40 unchanged, 0 conflict(s), 0 removed upstream"
    );
    assert_eq!(
        pull_summary(&fixture("pull-conflict.json")).expect("it parsed"),
        "example: 2 new, 1 updated, 40 unchanged, 1 conflict(s), 0 removed upstream"
    );
}

#[test]
fn something_that_is_not_json_is_an_error_rather_than_an_empty_model() {
    assert!(objects("not json").is_err());
    assert!(stage("not json").is_err());
    assert!(object("{}").is_err(), "an object with no oid is not an object");
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p herdr-damnit-adapters --locked wire`
Expected: FAIL with `unresolved module or unlinked crate 'wire'`

- [ ] **Step 3: Write the module**

`crates/herdr-damnit-adapters/src/wire.rs`. The serde structs mirror `dam`'s own; every optional
field defaults, so a `dam` that adds one does not break the read.

```rust
//! `dam`'s documents, read into the pane's own types. The structs here mirror `dam`'s wire shape
//! field for field; the fixtures beside them are byte copies of what the built `dam` produced.

use std::collections::HashMap;

use herdr_damnit_domain::{
    Attendee, Change, Conflict, Date, EventFields, Kind, Notice, Object, Oid, Op, Priority, Stage,
    TaskFields, Unpushed, parse_date,
};
use serde::Deserialize;

#[derive(Deserialize)]
struct WireObject {
    oid: String,
    kind: String,
    subject: String,
    #[serde(default)]
    body: String,
    #[serde(default)]
    path: String,
    #[serde(default)]
    labels: Vec<String>,
    #[serde(default)]
    depends: Vec<String>,
    #[serde(default)]
    recurrence: Option<String>,
    #[serde(default)]
    task: Option<WireTask>,
    #[serde(default)]
    event: Option<WireEvent>,
}

#[derive(Deserialize)]
struct WireTask {
    done: bool,
    priority: u8,
    #[serde(default)]
    due: Option<String>,
    #[serde(default)]
    deadline: Option<String>,
    #[serde(default)]
    event: Option<String>,
}

#[derive(Deserialize)]
struct WireEvent {
    start: String,
    end: String,
    #[serde(default)]
    timezone: Option<String>,
    #[serde(default)]
    location: Option<String>,
    #[serde(default)]
    attendees: Vec<WireAttendee>,
    status: String,
    transparency: String,
}

#[derive(Deserialize)]
struct WireAttendee {
    email: String,
    response: String,
}

#[derive(Deserialize)]
struct Listing {
    objects: Vec<WireObject>,
}

pub fn objects(json: &str) -> Result<Vec<Object>, String> {
    let listing: Listing = read(json)?;
    Ok(listing.objects.into_iter().map(into_object).collect())
}

pub fn object(json: &str) -> Result<Object, String> {
    read::<WireObject>(json).map(into_object)
}

fn into_object(wire: WireObject) -> Object {
    Object {
        oid: Oid::new(wire.oid),
        kind: match wire.kind.as_str() {
            "event" => Kind::Event,
            _ => Kind::Task,
        },
        subject: wire.subject,
        body: wire.body,
        path: wire.path,
        labels: wire.labels,
        depends: wire.depends.into_iter().map(Oid::new).collect(),
        recurrence: wire.recurrence,
        task: wire.task.map(|task| TaskFields {
            done: task.done,
            priority: Priority::new(task.priority).unwrap_or_default(),
            due: task.due.as_deref().and_then(parse_date),
            deadline: task.deadline.as_deref().and_then(parse_date),
            attached: task.event.map(Oid::new),
        }),
        event: wire.event.map(|event| EventFields {
            start: event.start,
            end: event.end,
            timezone: event.timezone,
            location: event.location,
            status: event.status,
            transparency: event.transparency,
            attendees: event
                .attendees
                .into_iter()
                .map(|attendee| Attendee {
                    email: attendee.email,
                    response: attendee.response,
                })
                .collect(),
        }),
    }
}

fn read<T: serde::de::DeserializeOwned>(json: &str) -> Result<T, String> {
    serde_json::from_str(json).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests;
```

**The settled unpushed row.** `dam` publishes two keys on it: `oids`, the distinct objects the
unpushed commits touch in order of first appearance walking newest first, and `commit_ids`, the
commit ids themselves newest first, with `commits` equal to that list's length. This pane reads
`oids` as object ids and reads neither of the other two beyond `commits`, so `WireUnpushed` declares
no `deny_unknown_fields` and a 0.2.0 document carrying neither key still parses on the serde
default. `an_unpushed_row_reads_the_objects_and_ignores_the_commit_ids_beside_them` pins both halves:
a `deny_unknown_fields` on that struct, or a mapping that reached for the commit ids, fails it.

`kind_changed` is the fifth kind and the plan first listed only four arms for it: a real one
carries `ours` and `theirs` as kind words and no `why` at all, so the fallback drew
`kind_changed: ` with nothing after the colon.

The status, log and sync readers go in a sibling file to keep both inside the line cap:
`crates/herdr-damnit-adapters/src/wire/reports.rs`, re-exported from `wire.rs` with
`mod reports; pub use reports::{completions, pull_summary, push_summary, stage};`. The error
document is a third sibling, `wire/failures.rs`, exporting `error_document`, which the Interfaces
block names and the plan's code listing left out. Each of the three keeps its own
`#[cfg(test)] mod tests;` beside it rather than one shared test file: the tests in one file reached
404 lines, past the point the Rust standard asks for decomposition.

```rust
//! `dam status --json`, `dam log --json` and the two sync reports.

use super::{WireObject, into_object, read};
use herdr_damnit_domain::{Change, Conflict, Date, Notice, Oid, Op, Stage, Unpushed};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize)]
struct WireStatus {
    staged: Vec<WireChange>,
    unstaged: Vec<WireChange>,
    conflicts: Vec<WireConflict>,
    notices: Vec<serde_json::Value>,
    unpushed: Vec<WireUnpushed>,
}

#[derive(Deserialize)]
struct WireChange {
    oid: String,
    op: String,
    #[serde(default)]
    subject: String,
    #[serde(default)]
    fields: Vec<String>,
    /// True on the object as the change leaves it, which is what marks a completion in a log.
    #[serde(default)]
    done: bool,
    /// The instant the task was completed, since `dam` 0.2.0. A `dam` without it leaves the
    /// commit's own day as the completion date.
    #[serde(default)]
    completed_at: Option<String>,
    /// Present only under `--full`, which this pane never asks for.
    #[serde(default)]
    after: Option<WireObject>,
}

#[derive(Deserialize)]
struct WireConflict {
    oid: String,
    remote: String,
    ours: WireObject,
    theirs: WireObject,
}

#[derive(Deserialize)]
struct WireUnpushed {
    remote: String,
    commits: u64,
    /// The distinct objects this remote's unpushed commits touch, newest commit first. A `dam`
    /// that sends none leaves the set empty, which costs the rows their unpushed mark and nothing
    /// else.
    #[serde(default)]
    oids: Vec<String>,
}

pub fn stage(json: &str) -> Result<Stage, String> {
    let status: WireStatus = read(json)?;
    Ok(Stage {
        staged: status.staged.into_iter().map(into_change).collect(),
        unstaged: status.unstaged.into_iter().map(into_change).collect(),
        unpushed: status
            .unpushed
            .into_iter()
            .map(|remote| Unpushed {
                remote: remote.remote,
                commits: remote.commits,
                oids: remote.oids.into_iter().map(Oid::new).collect(),
            })
            .collect(),
        conflicts: status
            .conflicts
            .into_iter()
            .map(|conflict| Conflict {
                oid: Oid::new(conflict.oid),
                remote: conflict.remote,
                ours: conflict.ours.subject.clone(),
                theirs: conflict.theirs.subject.clone(),
            })
            .collect(),
        notices: status.notices.iter().map(into_notice).collect(),
    })
}

fn into_change(wire: WireChange) -> Change {
    let subject = match wire.subject.is_empty() {
        false => wire.subject,
        true => wire
            .after
            .as_ref()
            .map(|object| object.subject.clone())
            .unwrap_or_default(),
    };
    Change {
        oid: Oid::new(wire.oid),
        op: match wire.op.as_str() {
            "create" => Op::Create,
            "delete" => Op::Delete,
            _ => Op::Update,
        },
        subject,
        fields: wire.fields,
    }
}

/// One notice as a sentence. `dam` names five kinds and each carries its own keys, so the text is
/// built per kind rather than printed as a kind and a blob.
fn into_notice(wire: &serde_json::Value) -> Notice {
    let text = |key: &str| wire.get(key).and_then(|value| value.as_str()).unwrap_or("");
    let kind = text("kind").to_string();
    let remote = text("remote");
    let subject = text("subject");
    let why = text("why");
    let message = match kind.as_str() {
        "removed_upstream" => format!("removed on {remote}: {subject:?} is kept here"),
        "event_cancelled" => format!("event cancelled: {subject:?}"),
        "push_failed" => format!("{remote}: push failed: {why}"),
        "pull_failed" => format!("{remote}: pull failed: {why}"),
        "kind_changed" => format!(
            "kind changed: a {} here and an {} upstream",
            text("ours"),
            text("theirs")
        ),
        other => format!("{other}: {why}"),
    };
    Notice {
        kind,
        oid: wire
            .get("oid")
            .and_then(|value| value.as_str())
            .map(Oid::new),
        remote: (!remote.is_empty()).then(|| remote.to_string()),
        message,
    }
}

#[derive(Deserialize)]
struct WireLog {
    commits: Vec<WireCommit>,
}

#[derive(Deserialize)]
struct WireCommit {
    at: String,
    changes: Vec<WireChange>,
}

/// The day each task was completed on: the commit whose change flipped `after.task.done` to true.
pub fn completions(json: &str) -> Result<HashMap<Oid, Date>, String> {
    let log: WireLog = read(json)?;
    let mut completed = HashMap::new();
    for commit in log.commits {
        for change in commit.changes {
            if !change.done || !change.fields.iter().any(|field| field == "done") {
                continue;
            }
            let at = change
                .completed_at
                .as_deref()
                .and_then(parse_date)
                .or_else(|| parse_date(&commit.at));
            if let Some(at) = at {
                completed.insert(Oid::new(change.oid), at);
            }
        }
    }
    Ok(completed)
}

#[derive(Deserialize)]
struct WireSync<T> {
    remotes: Vec<T>,
}

#[derive(Deserialize)]
struct WirePush {
    remote: String,
    sent: u64,
    succeeded: u64,
    skipped: u64,
    failed: Vec<serde_json::Value>,
}

#[derive(Deserialize)]
struct WirePull {
    remote: String,
    created: u64,
    updated: u64,
    unchanged: u64,
    conflicts: u64,
    removed_upstream: u64,
}

pub fn push_summary(json: &str) -> Result<String, String> {
    let report: WireSync<WirePush> = read(json)?;
    Ok(report
        .remotes
        .iter()
        .map(|remote| {
            format!(
                "{}: {} sent, {} ok, {} failed, {} skipped",
                remote.remote,
                remote.sent,
                remote.succeeded,
                remote.failed.len(),
                remote.skipped
            )
        })
        .collect::<Vec<_>>()
        .join("; "))
}

pub fn pull_summary(json: &str) -> Result<String, String> {
    let report: WireSync<WirePull> = read(json)?;
    Ok(report
        .remotes
        .iter()
        .map(|remote| {
            format!(
                "{}: {} new, {} updated, {} unchanged, {} conflict(s), {} removed upstream",
                remote.remote,
                remote.created,
                remote.updated,
                remote.unchanged,
                remote.conflicts,
                remote.removed_upstream
            )
        })
        .collect::<Vec<_>>()
        .join("; "))
}
```

`WireObject`, `into_object` and `read` need `pub(super)` or `pub(crate)` visibility for the sibling
file to use them, and `WireTask` needs `pub(super) done: bool` for the completion walk.

Add to `crates/herdr-damnit-adapters/src/lib.rs`:

```rust
pub mod wire;
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p herdr-damnit-adapters --locked wire`
Expected: PASS, fourteen tests. If a fixture assertion fails on a subject or a path, correct the
assertion to what `capture.sh` actually produced rather than editing the fixture: the fixture is the
byte record and the test is what reads it.

- [ ] **Step 5: Check every file is inside the cap**

Run: `wc -l crates/herdr-damnit-adapters/src/wire.rs crates/herdr-damnit-adapters/src/wire/reports.rs crates/herdr-damnit-adapters/src/wire/failures.rs crates/herdr-damnit-adapters/src/wire/tests.rs crates/herdr-damnit-adapters/src/wire/reports/tests.rs crates/herdr-damnit-adapters/src/wire/failures/tests.rs`
Expected: all six inside the 300 line ideal, none over 500.

- [ ] **Step 6: Commit**

```bash
git add -A
SKIP_AI_COMMIT=1 git commit -m "feat(adapters): read dam's documents into the pane's own types"
```

---

### Task 24: The config reader

**Files:**
- Create: `crates/herdr-damnit-adapters/src/config.rs`
- Create: `crates/herdr-damnit-adapters/src/config/tests.rs`
- Modify: `crates/herdr-damnit-adapters/src/lib.rs`
- Reference: `crates/herdr-damnit/src/config.rs`, whose validation this carries over

**Interfaces:**
- Consumes: `View`, `IconSet`, `OPEN` from the domain crate.
- Produces:

```rust
pub struct Config {
    pub dam: Vec<String>,
    pub placement: Placement,
    pub side: Side,
    pub width: Option<f32>,
    pub default_view: Option<String>,
    pub auto_open: bool,
    pub theme: Option<String>,
    pub icons: Icons,
    pub refresh_seconds: Option<u64>,
    pub handoff_label: String,
    pub views: Vec<ConfigView>,
}

pub enum Placement { Overlay, Split, Tab, Zoomed }
pub enum Side { Right, Down }
pub enum Icons { NerdFont, Ascii }
pub struct ConfigView { pub name: String, pub query: String }

impl Config {
    pub fn load() -> Result<Config, String>;
    pub fn parse(text: &str) -> Result<Config, String>;
    pub fn refresh_seconds(&self) -> u64;
    pub fn views(&self) -> Vec<View>;
    pub fn icons(&self) -> IconSet;
    pub fn check_theme_against(&self, names: &[&str]) -> Result<(), String>;
}

pub const DEFAULT_REFRESH_SECONDS: u64 = 300;
```

`icons` and `views` are the config's own serde types rather than the domain's, because
`IconSet` and `View` carry no serde derive and the domain crate takes no serde dependency. The two
reader methods, `Config::icons()` and `Config::views()`, are what the rest of the pane calls, so
the tests for those keys assert on the reader rather than on the field.

`Side` here is the pane's placement side, Right or Down. It shares a name with
`herdr_damnit_application::Side`, which is the side a conflict resolves toward; nothing imports
both into one scope.

What changed from the Todoist config: `token_command` and `token_env` are gone, because the token is
`dam`'s; `editor` is gone, because `dam edit -e` chooses the editor and falls back to `vi`; each
`[[views]]` entry takes `query` instead of `filter`; `dam` is new and defaults to `["dam"]`; and
`handoff_label` is new and defaults to `"handed-off"`, an empty string turning the record off.

What is unchanged: `placement`, `side`, `width`, `default_view`, `auto_open`, `theme`, `icons` and
`refresh_seconds`, along with every validation rule. `deny_unknown_fields` stays, which is what makes
a stale `token_command` a parse error naming the field rather than a silently ignored key.

The reserved view name is now `open` rather than `all`, because that is the name of view 1.

`IconSet` and `Side` are domain types with no serde derive, so the config declares its own
string-shaped enums and maps them, which is also what keeps the parse error naming the alternatives.

- [ ] **Step 1: Write the failing tests**

`crates/herdr-damnit-adapters/src/config/tests.rs`, which carries over the existing config tests that
still apply and adds one per changed key:

```rust
use super::*;

#[test]
fn an_empty_file_is_the_defaults() {
    let config = Config::parse("").expect("parses");
    assert_eq!(config.dam, vec!["dam".to_string()]);
    assert_eq!(config.placement, Placement::Split);
    assert_eq!(config.side, Side::Right);
    assert_eq!(config.width, None);
    assert_eq!(config.default_view, None);
    assert!(!config.auto_open);
    assert_eq!(config.icons, IconSet::NerdFont);
    assert_eq!(config.handoff_label, "handed-off");
    assert_eq!(config.refresh_seconds(), DEFAULT_REFRESH_SECONDS);
    assert!(config.views.is_empty());
}

#[test]
fn the_dam_argv_is_read_as_argv_so_a_path_with_a_space_stays_one_word() {
    let config = Config::parse(r#"dam = ["/opt/My Tools/dam"]"#).expect("parses");
    assert_eq!(config.dam, vec!["/opt/My Tools/dam".to_string()]);
}

#[test]
fn an_empty_dam_argv_is_refused_because_nothing_could_be_spawned() {
    let error = Config::parse("dam = []").expect_err("refuses");
    assert!(error.contains("dam"), "{error}");
}

#[test]
fn a_view_carries_a_query_in_dams_grammar() {
    let config = Config::parse(
        "[[views]]\nname = \"today\"\nquery = \"!done & (due:today | overdue)\"\n\
         [[views]]\nname = \"deep\"\nquery = \"!done & effort:deep\"\n",
    )
    .expect("parses");

    assert_eq!(
        config.views,
        vec![
            View { name: "today".to_string(), query: "!done & (due:today | overdue)".to_string() },
            View { name: "deep".to_string(), query: "!done & effort:deep".to_string() },
        ]
    );
}

#[test]
fn the_old_filter_key_is_a_parse_error_naming_the_field_rather_than_an_ignored_key() {
    let error = Config::parse("[[views]]\nname = \"today\"\nfilter = \"today\"\n")
        .expect_err("refuses");
    assert!(error.contains("filter") || error.contains("query"), "{error}");
}

#[test]
fn a_stale_token_key_is_a_parse_error_because_the_token_is_dams_business_now() {
    for text in [
        r#"token_command = ["security", "find-generic-password"]"#,
        r#"token_env = "SOME_TOKEN""#,
        r#"editor = ["nvim"]"#,
    ] {
        let error = Config::parse(text).expect_err("refuses");
        assert!(error.contains("unknown field"), "{text}: {error}");
    }
}

#[test]
fn the_handoff_label_is_configurable_and_an_empty_one_turns_the_record_off() {
    assert_eq!(
        Config::parse(r#"handoff_label = "sent""#).expect("parses").handoff_label,
        "sent"
    );
    assert_eq!(
        Config::parse(r#"handoff_label = """#).expect("parses").handoff_label,
        ""
    );
}

#[test]
fn two_views_with_one_name_are_a_config_error() {
    let error = Config::parse(
        "[[views]]\nname = \"today\"\nquery = \"!done\"\n\
         [[views]]\nname = \"today\"\nquery = \"done\"\n",
    )
    .expect_err("refuses");
    assert!(error.contains("two views are named 'today'"), "{error}");
}

#[test]
fn a_view_named_after_the_panes_own_open_list_is_a_config_error() {
    let error = Config::parse("[[views]]\nname = \"open\"\nquery = \"!done\"\n")
        .expect_err("refuses");
    assert!(error.contains("cannot be named 'open'"), "{error}");
}

#[test]
fn a_view_missing_either_half_is_a_config_error_naming_the_field() {
    assert!(
        Config::parse("[[views]]\nname = \"today\"\n")
            .expect_err("refuses")
            .contains("query")
    );
    assert!(
        Config::parse("[[views]]\nquery = \"!done\"\n")
            .expect_err("refuses")
            .contains("name")
    );
    assert!(
        Config::parse("[[views]]\nname = \"\"\nquery = \"!done\"\n")
            .expect_err("refuses")
            .contains("needs a name")
    );
    assert!(
        Config::parse("[[views]]\nname = \"today\"\nquery = \"\"\n")
            .expect_err("refuses")
            .contains("empty query")
    );
}

#[test]
fn an_opening_view_no_view_carries_is_refused_and_names_the_views() {
    let error = Config::parse(
        "default_view = \"work\"\n[[views]]\nname = \"today\"\nquery = \"!done\"\n",
    )
    .expect_err("refuses");
    assert!(error.contains("default_view 'work' is not a view"), "{error}");
    assert!(error.contains("open, today"), "{error}");
}

#[test]
fn a_width_outside_the_tab_is_refused_by_the_number_it_was_given() {
    for width in ["0.0", "0", "1.0", "1.5", "-0.2"] {
        let error = Config::parse(&format!("width = {width}")).expect_err("refuses");
        assert!(error.contains("share of the tab"), "{width}: {error}");
    }
    assert_eq!(Config::parse("width = 0.3").expect("parses").width, Some(0.3));
}

#[test]
fn a_placement_and_a_side_are_taken_by_name_and_an_unknown_one_names_the_alternatives() {
    let error = Config::parse(r#"placement = "popup""#).expect_err("refuses");
    for word in ["overlay", "split", "tab", "zoomed"] {
        assert!(error.contains(word), "{error}");
    }
    let error = Config::parse(r#"side = "left""#).expect_err("refuses");
    assert!(error.contains("right") && error.contains("down"), "{error}");
}

#[test]
fn the_marks_are_nerd_font_glyphs_until_the_plain_set_is_asked_for() {
    assert_eq!(Config::parse(r#"icons = "ascii""#).expect("parses").icons, IconSet::Ascii);
    assert!(Config::parse(r#"icons = "emoji""#).expect_err("refuses").contains("icons"));
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p herdr-damnit-adapters --locked config`
Expected: FAIL with `unresolved module or unlinked crate 'config'`

- [ ] **Step 3: Write the module**

`crates/herdr-damnit-adapters/src/config.rs` follows the structure of
`crates/herdr-damnit/src/config.rs` with the keys above. The shape of the new parts:

```rust
//! The plugin's configuration, read from `config.toml` in the herdr plugin config directory. A
//! missing file is the default configuration. No key here names a credential: the token is `dam`'s.

use std::path::{Path, PathBuf};

use herdr_damnit_domain::{IconSet, OPEN, View};
use serde::Deserialize;

/// The interval read, which is two local reads rather than three network requests.
pub const DEFAULT_REFRESH_SECONDS: u64 = 300;

/// The label a successful hand-off writes, an empty one turning the record off.
const DEFAULT_HANDOFF_LABEL: &str = "handed-off";

#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Argv of the `dam` the pane spawns, so a test points it at a fixture executable and no test
    /// mangles `PATH`.
    #[serde(default = "default_dam")]
    pub dam: Vec<String>,
    #[serde(default)]
    pub placement: Placement,
    #[serde(default)]
    pub side: Side,
    pub width: Option<f32>,
    pub default_view: Option<String>,
    #[serde(default)]
    pub auto_open: bool,
    pub theme: Option<String>,
    #[serde(default = "default_icons")]
    pub icons: Icons,
    pub refresh_seconds: Option<u64>,
    #[serde(default = "default_handoff_label")]
    pub handoff_label: String,
    #[serde(default)]
    pub views: Vec<ConfigView>,
}

/// One view: a name to pick it by and a query in `dam`'s own grammar, which `dam` resolves against
/// its saved filters first and parses as a query otherwise.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ConfigView {
    pub name: String,
    pub query: String,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Icons {
    #[default]
    NerdFont,
    Ascii,
}

impl From<Icons> for IconSet {
    fn from(icons: Icons) -> Self {
        match icons {
            Icons::NerdFont => IconSet::NerdFont,
            Icons::Ascii => IconSet::Ascii,
        }
    }
}

fn default_dam() -> Vec<String> {
    vec!["dam".to_string()]
}

fn default_icons() -> Icons {
    Icons::NerdFont
}

fn default_handoff_label() -> String {
    DEFAULT_HANDOFF_LABEL.to_string()
}

#[cfg(test)]
mod tests;
```

`Placement` and `Side` carry over from `crates/herdr-damnit/src/placement.rs` and
`crates/herdr-damnit/src/config.rs` unchanged, including `Side::split_direction` and
`Placement::as_str`. `Config::parse` runs four checks, with `ALL` replaced by `OPEN` and `filter`
replaced by `query`:

```rust
    pub fn parse(text: &str) -> Result<Self, String> {
        let config: Self = toml::from_str(text).map_err(|error| error.to_string())?;
        config.check_dam()?;
        config.check_view_names()?;
        config.check_width()?;
        config.check_default_view()?;
        Ok(config)
    }

    fn check_dam(&self) -> Result<(), String> {
        match self.dam.first() {
            Some(word) if !word.trim().is_empty() => Ok(()),
            _ => Err("dam needs at least one word: the binary to spawn".to_string()),
        }
    }
```

`Config::views()` is what the rest of the pane reads, mapping each `ConfigView` onto the domain's
`View`, and `Config::icons()` returns `IconSet`.

**The theme check is not one of the four.** Its vocabulary belongs to the crate that paints, so
neither `parse` nor `load` can run it without taking a dependency on the binary crate, and `load`
takes no arguments in the Interfaces block above. The composition root calls
`config.check_theme_against(herdr_damnit::theme::NAMES)?` after `Config::load()`, which is where
the palettes are. `parse` gains no dependency on ratatui either way.

`check_dam` also refuses a first word that is only whitespace, not just an empty list:
`Command::new("")` fails at the first read rather than at load time, which is the wrong place to
learn it.

`Placement`, `Side` and `Icons` live in `crates/herdr-damnit-adapters/src/config/kinds.rs`, not in
`config.rs`: with all three inline the module reached 271 implementation lines, past the point the
Rust standard asks for decomposition.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p herdr-damnit-adapters --locked config`
Expected: PASS, twenty-three tests. Nine beyond the plan's fourteen, each closing a behaviour the
plan states in prose and pins with no test: a blank `dam` binary, a leading-argument `dam` argv,
`open` as an opening view, every side's own direction, every placement's own word, `nerd-font` by
name, a refresh interval of zero staying zero, the theme check against a handed-in vocabulary, and
a config naming no theme at all.

- [ ] **Step 5: Commit**

```bash
git add -A
SKIP_AI_COMMIT=1 git commit -m "feat(adapters): read the pane's config with dam's query grammar and no token key"
```

---

### Task 25: The state directory, the clock and the herdr CLI

**Files:**
- Create: `crates/herdr-damnit-adapters/src/state.rs`
- Create: `crates/herdr-damnit-adapters/src/clock.rs`
- Create: `crates/herdr-damnit-adapters/src/herdr_cli.rs`
- Modify: `crates/herdr-damnit-adapters/src/lib.rs`
- Reference: `crates/herdr-damnit/src/state.rs` and `crates/herdr-damnit/src/herdr.rs`

**Interfaces:**
- Consumes: `Clock` and `Herdr` from Task 15.
- Produces:
  - `state`, the module moved from the binary crate unchanged apart from its crate path: `state_dir`,
    `view_request_path`, `remembered_pane`, `remember_pane`, `forget_pane`, `request_view`,
    `clear_view_request`, `take_requested_view`, with their existing tests.
  - `SystemClock`, implementing `Clock` with `jiff::Zoned::now().date()` and `Instant::now()`.
  - `CliHerdr`, implementing `Herdr` over `HERDR_BIN_PATH`, plus the pane calls moved from
    `herdr.rs`: `open_plugin_pane`, `focus_plugin_pane`, `close_plugin_pane`, `resize_leading_pane`
    and `live_panes`, with their existing envelope-reading tests.

Every `herdr` read checks the envelope under `result` rather than the exit code, because
`herdr plugin list` returns exit 0 even when that envelope is an error. That behaviour is already in
`herdr.rs` and moves with it.

**Carried obligation from Task 24, for whoever reaches the composition root.**
`Config::check_theme_against(&self, names: &[&str])` exists and has no production caller. The theme
vocabulary is `herdr_damnit::theme::NAMES`, which lives in the binary crate, and the adapters crate
cannot depend on the binary crate, so the parse cannot make this check and no adapters test can
cover the wiring. Every `Config::load()` must be followed by
`config.check_theme_against(theme::NAMES)?`, which Task 28 discharges in one place with its
`load_config` helper. If it never lands, an unknown theme name is accepted in silence and the pane
paints half of itself in the default colors.

- [ ] **Step 1: Write the failing test**

`crates/herdr-damnit-adapters/src/clock.rs`, inside `mod tests`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_system_clock_answers_a_real_day_and_a_monotonic_instant() {
        let clock = SystemClock;
        assert!(clock.today().year() >= 2026);
        let first = clock.now();
        assert!(clock.now() >= first);
    }
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p herdr-damnit-adapters --locked clock`
Expected: FAIL with `unresolved module or unlinked crate 'clock'`

- [ ] **Step 3: Move the three modules**

```bash
git mv crates/herdr-damnit/src/state.rs crates/herdr-damnit-adapters/src/state.rs
git mv crates/herdr-damnit/src/herdr.rs crates/herdr-damnit-adapters/src/herdr_cli.rs
```

In `state.rs`, nothing changes but the module doc; its tests move with it.

In `herdr_cli.rs`, change `use crate::config::{Config, Placement};` to
`use crate::config::{Config, Placement};` against the adapters crate's own config, and add the port
implementation at the top:

```rust
/// The production herdr client: the CLI herdr names for its plugins.
pub struct CliHerdr;

impl herdr_damnit_application::Herdr for CliHerdr {
    fn call(&self, args: &[&str]) -> Result<String, String> {
        run(args)
    }
}
```

Write `crates/herdr-damnit-adapters/src/clock.rs`:

```rust
//! The wall clock, behind a port so every date rule is tested against a literal day.

use std::time::Instant;

use herdr_damnit_application::Clock;
use herdr_damnit_domain::Date;

pub struct SystemClock;

impl Clock for SystemClock {
    fn today(&self) -> Date {
        jiff::Zoned::now().date()
    }

    fn now(&self) -> Instant {
        Instant::now()
    }
}
```

That needs `jiff.workspace = true` in the adapters crate's `[dependencies]`.

Add to `crates/herdr-damnit-adapters/src/lib.rs`:

```rust
mod clock;
pub mod config;
mod herdr_cli;
pub mod state;

pub use clock::SystemClock;
pub use config::Config;
pub use herdr_cli::CliHerdr;
```

- [ ] **Step 4: Run the whole suite to verify it passes**

Run: `cargo test --workspace --locked -- --test-threads=1`
Expected: PASS. The binary crate still compiles: it now reaches `state` and the herdr calls through
`herdr_damnit_adapters`, so replace its `mod state;` and `mod herdr;` declarations with
`use herdr_damnit_adapters::{state, herdr_cli as herdr};` and fix the call sites the compiler names.

**Ruling 19.** `herdr_cli` is declared `pub mod herdr_cli;` rather than `mod herdr_cli;`. `pane.rs`
calls `open_plugin_pane`, `focus_plugin_pane`, `close_plugin_pane`, `resize_leading_pane` and
`live_panes` by path, so the module itself has to be reachable and not only the `CliHerdr` re-export.

**Ruling 20.** Step 4's "fix the call sites the compiler names" is larger than one import.
`herdr_cli::open_plugin_pane` takes the adapters crate's own `Config`, so `pane.rs` moves onto
`herdr_damnit_adapters::Config` and `herdr_damnit_domain::Views` in this task, and `main.rs` loads
that config for the five pane actions while `tui::run` and `doctor::run` keep loading the Todoist-era
`config::Config` until Task 28 deletes it. That re-aim orphaned three items, each deleted here with
the one test that only exercised it, because `clippy -D warnings` fails on dead code and all three
files leave the tree in Task 28: `Placement::as_str` in `crates/herdr-damnit/src/config.rs`,
`Side::split_direction` in `placement.rs`, and `Views::name_of_number` in `views.rs`. `send.rs` calls
the herdr client through the port (`herdr_damnit_application::Herdr::call(&CliHerdr, args)`) because
the free `herdr::call` became that trait method.

- [ ] **Step 5: Commit**

```bash
git add -A
SKIP_AI_COMMIT=1 git commit -m "feat(adapters): move the state directory, the clock and the herdr client into the adapters"
```

---

## Phase E: the binary

The binary crate holds the draw loop, the ratatui widgets and the composition root. The pane model
and every key are a plain struct with no terminal in it, so the key tests drive it directly with a
fake runner and assert the argv it produced; the render tests draw that same model through
`ratatui::TestBackend` at 32 columns, the width a side pane opens at.

**One deliberate deviation from the spec, in one place.** The spec illustrates the non-blocking test
with a `dam` that sleeps three seconds. Every test in this repository runs under one second, which is
a Global Constraint, so the fake sleeps 600 ms and the loop is driven for 800 ms with the render
threshold scaled to the same 50 ms window. The assertion is unchanged: a loop that awaited the push
inline renders once and fails on the first clause.

### Task 26: The draw loop that renders while a job runs

**Files:**
- Create: `crates/herdr-damnit/src/app.rs`
- Create: `crates/herdr-damnit/src/app/tests.rs`
- Create: `crates/herdr-damnit/src/loop_.rs`
- Modify: `crates/herdr-damnit/src/main.rs` (add the two module declarations only)
- Modify: `crates/herdr-damnit/Cargo.toml`

**Interfaces:**
- Consumes: `Jobs`, `JobKind`, `SyncKind`, `Submitted`, `Completion` from Task 17; `Cursor`,
  `Views`, `Stage`, `Object` from Phase B; `Config` from Task 24.
- Produces:

```rust
pub enum Screen { List, Status, Done, Detail }

pub struct App {
    pub screen: Screen,
    pub views: Views,
    pub list: Cursor,
    pub status: Cursor,
    pub done: Cursor,
    pub stage: Stage,
    pub detail: Option<Object>,
    pub overlay: Option<Overlay>,
    pub message: String,
    pub spinner: usize,
    /* the job table, the config, the clock and the completion dates */
}

pub enum After { Stay, Quit, Editor(Vec<String>) }

impl App {
    pub fn new(config: Config, jobs: Jobs, today: Date) -> App;
    /// Drain the job channel, apply every completion, step the spinner. Never blocks.
    pub fn tick(&mut self, now: Instant);
    /// Handle one key. Never blocks and never spawns a thread of its own.
    pub fn key(&mut self, key: KeyEvent) -> After;
    /// 50 ms while any job is in flight, 200 ms when none is.
    pub fn poll_window(&self) -> Duration;
    /// The header's left half: the view, the spinner and the elapsed time.
    pub fn header(&self, now: Instant) -> String;
    pub fn submit(&mut self, kind: JobKind, argv: Vec<String>);
}
```

**The clock arrives here.** Task 17's `Jobs::submit` records `started: Instant::now()` itself, so
`elapsed_of_current` cannot be driven by a fake. This task routes it through the `Clock` port Task 15
declares: `Jobs::new` takes the clock, `submit` sets `started: self.clock.now()`, and the header
timer's test drives a fake clock rather than a real one. Task 17 is otherwise unchanged, and no
behaviour before this task reads an elapsed time.

**The same superseded-job trap as Task 17.** A superseded read is dropped from the table before its
replacement spawns, which drops its receiver, so an `expect` on the send panics the moment a test
answers one. This task's `Harness::answer` therefore ignores the `SendError`, as Task 17's does.
Keep it that way.

`loop_.rs` is the terminal-facing half, which no test drives:

```rust
pub fn run(app: &mut App) -> Result<(), String>;
```

It draws every tick unconditionally, calls `App::tick`, then polls for a key within
`App::poll_window` and hands it to `App::key`. An `After::Editor(argv)` leaves the alternate screen,
runs that argv to completion, enters it again and re-reads, on every path including a failure to
start, which Task 37 fills in.

`App::key` never blocks. A key that needs `dam` calls `submit` and returns; the job's result arrives
on a later tick.

- [ ] **Step 1: Write the failing tests**

`crates/herdr-damnit/src/app/tests.rs`:

```rust
use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent};
use herdr_damnit_application::{DamRunner, Finished, JobKind, Jobs, RunningJob, SpawnError, SyncKind};
use herdr_damnit_domain::parse_date;

use super::*;

#[derive(Default)]
pub(crate) struct Recorder {
    pub(crate) log: Arc<Mutex<Vec<Vec<String>>>>,
    pub(crate) senders: Arc<Mutex<Vec<Sender<Finished>>>>,
}

impl DamRunner for Recorder {
    fn spawn(&self, argv: &[String]) -> Result<RunningJob, SpawnError> {
        self.log.lock().expect("the log").push(argv.to_vec());
        let (sender, results) = channel();
        self.senders.lock().expect("the senders").push(sender);
        Ok(RunningJob {
            cancel: Box::new(|| {}),
            results,
        })
    }
}

pub(crate) struct Harness {
    pub(crate) app: App,
    log: Arc<Mutex<Vec<Vec<String>>>>,
    senders: Arc<Mutex<Vec<Sender<Finished>>>>,
}

pub(crate) fn harness() -> Harness {
    harness_with(Config::parse("").expect("the default config"))
}

pub(crate) fn harness_with(config: Config) -> Harness {
    let recorder = Recorder::default();
    let log = Arc::clone(&recorder.log);
    let senders = Arc::clone(&recorder.senders);
    Harness {
        app: App::new(
            config,
            Jobs::new(Box::new(recorder)),
            parse_date("2026-09-20").expect("a date"),
        ),
        log,
        senders,
    }
}

impl Harness {
    pub(crate) fn press(&mut self, code: KeyCode) -> After {
        self.app.key(KeyEvent::from(code))
    }

    /// Answer one spawned job. A send to a superseded job fails because dropping it from the table
    /// dropped its receiver, which is the mechanism by which its result never reaches the model, so
    /// the answer is offered rather than required.
    pub(crate) fn answer(&mut self, which: usize, code: i32, stdout: &str, stderr: &str) {
        let _ = self.senders.lock().expect("the senders")[which].send(Finished {
            code: Some(code),
            stdout: stdout.to_string(),
            stderr: stderr.to_string(),
            elapsed: Duration::from_millis(9),
        });
        self.app.tick(Instant::now());
    }

    pub(crate) fn lines(&self) -> Vec<String> {
        self.log
            .lock()
            .expect("the log")
            .iter()
            .map(|argv| argv.join(" "))
            .collect()
    }

    pub(crate) fn last(&self) -> String {
        self.lines().last().cloned().unwrap_or_default()
    }
}

#[test]
fn a_key_that_needs_dam_returns_before_the_job_answers() {
    let mut harness = harness();
    let started = Instant::now();
    harness.app.submit(
        JobKind::Exclusive(SyncKind::Push),
        herdr_damnit_application::argv::push(),
    );

    assert!(started.elapsed() < Duration::from_millis(50), "the key blocked");
    assert_eq!(harness.lines(), vec!["push --json".to_string()]);
}

#[test]
fn the_poll_window_narrows_while_a_job_is_in_flight() {
    let mut harness = harness();
    assert_eq!(harness.app.poll_window(), Duration::from_millis(200));

    harness.app.submit(JobKind::ReadStatus, herdr_damnit_application::argv::status());
    assert_eq!(harness.app.poll_window(), Duration::from_millis(50));

    harness.answer(0, 0, r#"{"staged":[],"unstaged":[],"conflicts":[],"notices":[],"unpushed":[]}"#, "");
    assert_eq!(harness.app.poll_window(), Duration::from_millis(200));
}

#[test]
fn a_tick_with_nothing_in_flight_does_not_block() {
    let mut harness = harness();
    let started = Instant::now();
    for _ in 0..100 {
        harness.app.tick(Instant::now());
    }
    assert!(started.elapsed() < Duration::from_millis(100), "a tick blocked");
}

#[test]
fn the_header_names_the_exclusive_job_its_spinner_and_its_elapsed_time() {
    let mut harness = harness();
    let started = Instant::now();
    harness.app.submit(
        JobKind::Exclusive(SyncKind::Push),
        herdr_damnit_application::argv::push(),
    );
    harness.app.tick(started);

    let header = harness.app.header(started + Duration::from_millis(3200));
    assert!(header.contains("push"), "{header}");
    assert!(header.contains("3.2s"), "{header}");
}

#[test]
fn elapsed_is_tenths_under_ten_seconds_and_whole_seconds_after_it() {
    let mut harness = harness();
    let started = Instant::now();
    harness.app.submit(
        JobKind::Exclusive(SyncKind::Pull),
        herdr_damnit_application::argv::pull(),
    );

    assert!(harness.app.header(started + Duration::from_millis(1500)).contains("1.5s"));
    assert!(harness.app.header(started + Duration::from_secs(42)).contains("42s"));
}

#[test]
fn two_reads_in_flight_are_counted_rather_than_named() {
    let mut harness = harness();
    harness.app.submit(JobKind::ReadStatus, herdr_damnit_application::argv::status());
    harness.app.submit(JobKind::ReadLog, herdr_damnit_application::argv::log());

    let header = harness.app.header(Instant::now());
    assert!(header.contains("2 reads"), "{header}");
}

#[test]
fn the_spinner_steps_one_frame_per_tick() {
    let mut harness = harness();
    harness.app.submit(JobKind::ReadStatus, herdr_damnit_application::argv::status());

    let first = harness.app.spinner;
    harness.app.tick(Instant::now());
    assert_ne!(harness.app.spinner, first);
}

#[test]
fn a_write_completion_re_reads_the_status_and_the_list_rather_than_patching_the_model() {
    let mut harness = harness();
    harness.app.submit(JobKind::Write, herdr_damnit_application::argv::stage_all());
    harness.answer(0, 0, "{}", "");

    let lines = harness.lines();
    assert_eq!(lines[0], "add -A --json");
    assert!(lines.contains(&"status --json".to_string()), "{lines:?}");
    assert!(lines.contains(&"ls !done --json".to_string()), "{lines:?}");
}

#[test]
fn a_refusal_puts_dams_own_sentence_in_the_status_line_and_leaves_the_model_alone() {
    let mut harness = harness();
    let before = harness.app.list.object_count();
    harness.app.submit(JobKind::Write, herdr_damnit_application::argv::stage_all());
    harness.answer(0, 2, "", "dam: no object matches \"zzzzzzz\"\n");

    assert_eq!(harness.app.message, "no object matches \"zzzzzzz\"");
    assert_eq!(harness.app.list.object_count(), before);
}

#[test]
fn a_held_store_adds_the_retry_sentence() {
    let mut harness = harness();
    harness.app.submit(JobKind::Write, herdr_damnit_application::argv::stage_all());
    harness.answer(0, 1, "", "dam: database is locked");

    assert_eq!(
        harness.app.message,
        "database is locked; another dam is writing, press R to retry."
    );
}

#[test]
fn output_that_will_not_parse_is_reported_without_quoting_it() {
    let mut harness = harness();
    harness.app.submit(JobKind::ReadStatus, herdr_damnit_application::argv::status());
    harness.answer(0, 0, "this is not json", "");

    assert_eq!(
        harness.app.message,
        "dam answered with something this pane could not read."
    );
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p herdr-damnit --locked app`
Expected: FAIL with `unresolved module or unlinked crate 'app'`

- [ ] **Step 3: Write the app**

`crates/herdr-damnit/src/app.rs` holds `App`, `Screen`, `After` and `Overlay` (an empty enum for now,
filled by Task 31 onward), `tick`, `submit`, `poll_window` and `header`. `key` starts as a match that
handles nothing and returns `After::Stay`; every later task adds arms to it.

```rust
//! The pane's model and its keys. Nothing here touches a terminal, so every key is tested by the
//! argv it produced and the sentence it left in the status line.

use std::time::{Duration, Instant};

use crossterm::event::KeyEvent;
use herdr_damnit_adapters::{Config, wire};
use herdr_damnit_application::{Completion, JobKind, Jobs, Submitted, SyncKind};
use herdr_damnit_domain::{
    Cursor, Date, Failure, Object, Stage, Views, classify, message, rows,
};

/// The poll window while a job is in flight, which is what makes the spinner animate.
const BUSY_WINDOW: Duration = Duration::from_millis(50);

/// The poll window with nothing in flight, so an idle pane costs what it always did.
const IDLE_WINDOW: Duration = Duration::from_millis(200);

const BRAILLE: [&str; 8] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧"];
const PLAIN: [&str; 4] = ["|", "/", "-", "\\"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Screen {
    List,
    Status,
    Done,
    Detail,
}

#[derive(Debug, PartialEq, Eq)]
pub enum After {
    Stay,
    Quit,
    Editor(Vec<String>),
}

pub struct App {
    pub screen: Screen,
    pub views: Views,
    pub list: Cursor,
    pub status: Cursor,
    pub done: Cursor,
    pub stage: Stage,
    pub detail: Option<Object>,
    pub overlay: Option<Overlay>,
    pub message: String,
    pub spinner: usize,
    pub config: Config,
    pub today: Date,
    jobs: Jobs,
}

impl App {
    pub fn poll_window(&self) -> Duration {
        match self.jobs.in_flight() {
            0 => IDLE_WINDOW,
            _ => BUSY_WINDOW,
        }
    }

    pub fn submit(&mut self, kind: JobKind, argv: Vec<String>) {
        match self.jobs.submit(kind, argv) {
            Submitted::Started(_) => {}
            Submitted::Refused(said) => self.message = said,
            Submitted::NotInstalled => self.message = message(&Failure::NotInstalled),
            Submitted::Failed(said) => self.message = said,
        }
    }

    pub fn tick(&mut self, _now: Instant) {
        if self.jobs.in_flight() > 0 {
            self.spinner = self.spinner.wrapping_add(1);
        }
        for completion in self.jobs.drain() {
            self.apply(completion);
        }
    }

    fn apply(&mut self, completion: Completion) {
        if completion.finished.code != Some(0) {
            self.message = message(&classify(
                completion.finished.code,
                &completion.finished.stderr,
            ));
            return;
        }
        if let Err(said) = self.read(&completion) {
            self.message = said;
            return;
        }
        for kind in completion.follow_up {
            let argv = self.argv_of(&kind);
            self.submit(kind, argv);
        }
    }

    /// Put one successful document into the model. A document that will not parse is the pane's
    /// own failure rather than `dam`'s.
    fn read(&mut self, completion: &Completion) -> Result<(), String> {
        let unreadable = || message(&Failure::Unreadable);
        match &completion.kind {
            JobKind::ReadStatus => {
                self.stage = wire::stage(&completion.finished.stdout).map_err(|_| unreadable())?;
                self.redraw_list();
            }
            JobKind::ReadList => {
                let objects =
                    wire::objects(&completion.finished.stdout).map_err(|_| unreadable())?;
                self.objects = objects;
                self.redraw_list();
            }
            JobKind::Exclusive(SyncKind::Push) => {
                self.message =
                    wire::push_summary(&completion.finished.stdout).map_err(|_| unreadable())?;
            }
            JobKind::Exclusive(SyncKind::Pull) => {
                self.message =
                    wire::pull_summary(&completion.finished.stdout).map_err(|_| unreadable())?;
            }
            _ => {}
        }
        Ok(())
    }

    /// The header's left half. With more than one job in flight the exclusive one is named; with
    /// none, the count is shown.
    pub fn header(&self, now: Instant) -> String {
        let Some(elapsed) = self.jobs.elapsed_of_current(now) else {
            return format!("dam  {}", self.views.current().name);
        };
        let what = match self.jobs.exclusive() {
            Some(sync) => sync.verb().to_string(),
            None => format!("{} read{}", self.jobs.in_flight(),
                if self.jobs.in_flight() == 1 { "" } else { "s" }),
        };
        format!(
            "dam  {}  {} {what} {}",
            self.views.current().name,
            self.frame(),
            elapsed_text(elapsed)
        )
    }

    fn frame(&self) -> &'static str {
        match self.config.icons.into() {
            herdr_damnit_domain::IconSet::NerdFont => BRAILLE[self.spinner % BRAILLE.len()],
            herdr_damnit_domain::IconSet::Ascii => PLAIN[self.spinner % PLAIN.len()],
        }
    }

    pub fn key(&mut self, _key: KeyEvent) -> After {
        After::Stay
    }
}

/// Whole tenths up to ten seconds and whole seconds after that, so the number stops flickering
/// once a job is genuinely slow.
fn elapsed_text(elapsed: Duration) -> String {
    match elapsed.as_secs() < 10 {
        true => format!("{:.1}s", elapsed.as_secs_f32()),
        false => format!("{}s", elapsed.as_secs()),
    }
}

#[cfg(test)]
mod tests;
```

`App` also holds `objects: Vec<Object>`, the model the List screen is drawn from, and
`redraw_list` rebuilds `self.list` through `rows(&self.objects, &self.stage, style)` and
`Cursor::replace`, which is what keeps the cursor on its oid. `argv_of` maps a `JobKind` onto the
argv builder for it, using `self.views.current().query` for `ReadList`.

Add `crossterm = "0.29"` and the three workspace crates to `crates/herdr-damnit/Cargo.toml` if they
are not there, and declare both modules in `main.rs`:

```rust
mod app;
mod loop_;
```

- [ ] **Step 4: Write the loop**

`crates/herdr-damnit/src/loop_.rs`:

```rust
//! The draw loop: draw every tick, drain the job channel, poll for a key within the window the
//! model asks for. Nothing here waits on `dam`.

use std::time::Instant;

use crossterm::event::{self, Event, KeyEventKind};

use crate::app::{After, App};

pub fn run(app: &mut App) -> Result<(), String> {
    let mut terminal = ratatui::init();
    let outcome = loop {
        if let Err(error) = terminal.draw(|frame| crate::screens::draw(frame, app)) {
            break Err(error.to_string());
        }
        app.tick(Instant::now());
        match next_key(app.poll_window()) {
            Err(error) => break Err(error),
            Ok(None) => continue,
            Ok(Some(key)) => match app.key(key) {
                After::Stay => {}
                After::Quit => break Ok(()),
                After::Editor(argv) => crate::editor::round_trip(&mut terminal, app, &argv),
            },
        }
    };
    ratatui::restore();
    outcome
}

fn next_key(window: std::time::Duration) -> Result<Option<event::KeyEvent>, String> {
    if !event::poll(window).map_err(|error| error.to_string())? {
        return Ok(None);
    }
    match event::read().map_err(|error| error.to_string())? {
        Event::Key(key) if key.kind == KeyEventKind::Press => Ok(Some(key)),
        _ => Ok(None),
    }
}
```

`crate::screens::draw` and `crate::editor::round_trip` are written in Tasks 27 and 37; until then
stub both with a function that draws an empty frame and one that does nothing, each one line.

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test -p herdr-damnit --locked app`
Expected: PASS, eleven tests.

- [ ] **Step 6: Check the file is inside the cap**

Run: `wc -l crates/herdr-damnit/src/app.rs`
Expected: under 300. If it is over, move `header`, `frame` and `elapsed_text` into
`crates/herdr-damnit/src/app/header.rs`.

**Ruling 21.** `loop_.rs` is written in Task 28 rather than here. Its body calls
`crate::screens::draw`, which Task 27 writes, and nothing calls `loop_::run` until `main.rs` does at
the cutover, so landing it here would either break the build or need two throwaway stub modules Task
27 immediately replaces. Task 26 lands the model and its tests; the terminal half lands with the
composition root that calls it.

**Ruling 22.** `Jobs::new(runner, clock)` takes the `Clock` port and `submit` sets
`started: self.clock.now()`, as this task's preamble requires, and the harness exposes
`started()` so the two header tests read the instant the fake clock answers instead of calling
`Instant::now()` themselves. Against a `started` recorded a moment later,
`header(started + Duration::from_secs(42))` measures 42 seconds minus that moment, whose `as_secs()`
is 41, so the test as first written failed on every run. The four existing `Jobs::new` call sites in
`jobs/tests.rs` and `jobs/tests/unanswered.rs` take a `TestClock` declared beside them.

**Ruling 23.** `Screen` and `After` declare only the variants a task constructs. Here that is
`Screen::List` and `After::Stay`; Task 29 adds `Screen::Status` and `Screen::Done` with the Tab
cycle, Task 30 adds `Screen::Detail` and the `detail: Option<Object>` field with the `<CR>` key, Task
31 adds `overlay: Option<Overlay>` with the pickers, Task 37 adds `After::Editor(Vec<String>)` and
Task 39 adds `After::Quit`. `clippy -D warnings` rejects a variant nothing constructs and a field
nothing reads, and a suppression would be a lint that is right rather than wrong.

**Ruling 24.** `classify` takes three arguments, not the two the `App::apply` sketch above passes:
`classify(completion.finished.code, wire::error_document(&completion.finished.stderr),
&completion.finished.stderr)`. Task 22 gave it the parsed error document and the raw stream as the
fallback for a `dam` too old to print one.

**Ruling 25.** `cargo clippy --workspace --all-targets -- -D warnings` is not green at Tasks 26 and
27 and is not expected to be. Both tasks add a model and a renderer that nothing in `main.rs` reaches
until Task 28 wires them, so the binary target carries the whole of `app.rs` and `screens.rs` as dead
code until the cutover. The plan's gate order already says this: Task 28 Step 6 is the first step
that runs clippy.

- [ ] **Step 7: Commit**

```bash
git add -A
SKIP_AI_COMMIT=1 git commit -m "feat(pane): draw and read keys while a dam job runs"
```

---

### Task 27: The List screen and the header

**Files:**
- Create: `crates/herdr-damnit/src/screens.rs`
- Create: `crates/herdr-damnit/src/screens/list.rs`
- Create: `crates/herdr-damnit/src/screens/tests.rs`
- Modify: `crates/herdr-damnit/src/main.rs` (one module declaration)

**Interfaces:**
- Consumes: `App`, `Screen` from Task 26; `Row`, `Segment`, `Slot` from Phase B; the binary's own
  `theme::Palette`.
- Produces:

```rust
pub fn draw(frame: &mut ratatui::Frame<'_>, app: &App);
/// The whole pane as plain text at `width` columns, which is what a golden compares.
pub fn render_to_text(app: &App, width: u16, height: u16) -> String;
```

The frame is a status line, the body and a hint line, unchanged from the pane that exists. The status
line is `App::header` on the left and the counts on the right; the hint line is the key list. Each
row's segments are drawn in the palette colour their slot resolves to; the subject is plain text, so
the marks are what carry colour.

A golden is a text block in the test file rather than a separate file, so a diff in review shows the
screen rather than a path.

- [ ] **Step 1: Write the failing tests**

`crates/herdr-damnit/src/screens/tests.rs`:

```rust
use super::*;
use crate::app::tests::{harness, harness_with};
use herdr_damnit_adapters::Config;

const LS: &str = r#"{"objects":[
  {"oid":"1a2b3c4","kind":"task","subject":"ship the pin bump","path":"proj/dotfiles",
   "labels":["a","b"],"task":{"done":false,"priority":1,"due":"2026-09-18"}},
  {"oid":"5d6e7f8","kind":"task","subject":"refresh the roster row","path":"proj/dotfiles",
   "recurrence":"every week","task":{"done":false,"priority":4}},
  {"oid":"9a0b1c2","kind":"task","subject":"water the plants","path":"proj/home",
   "task":{"done":false,"priority":4,"due":"2026-10-02"}}
]}"#;

const CLEAN: &str = r#"{"staged":[],"unstaged":[],"conflicts":[],"notices":[],"unpushed":[]}"#;

fn ascii_config() -> Config {
    Config::parse("icons = \"ascii\"\n").expect("parses")
}

fn loaded() -> crate::app::tests::Harness {
    let mut harness = harness_with(ascii_config());
    harness.app.submit(
        herdr_damnit_application::JobKind::ReadStatus,
        herdr_damnit_application::argv::status(),
    );
    harness.answer(0, 0, CLEAN, "");
    harness.app.submit(
        herdr_damnit_application::JobKind::ReadList,
        herdr_damnit_application::argv::list("!done"),
    );
    harness.answer(1, 0, LS, "");
    harness
}

#[test]
fn the_list_screen_draws_its_headings_marks_and_hint_line() {
    let harness = loaded();
    assert_eq!(
        render_to_text(&harness.app, 32, 8),
        "\
dam  open              3 open
proj/dotfiles
  ~ refresh the roster row
  ! < 09-18 ship the pin b…@2
proj/home
  > 10-02 water the plants

x X dd p s D l m a S e <CR>…"
    );
}

#[test]
fn an_empty_list_says_so_rather_than_drawing_nothing() {
    let mut harness = harness_with(ascii_config());
    harness.app.submit(
        herdr_damnit_application::JobKind::ReadList,
        herdr_damnit_application::argv::list("!done"),
    );
    harness.answer(0, 0, r#"{"objects":[]}"#, "");

    assert!(
        render_to_text(&harness.app, 32, 6).contains("nothing in this view"),
        "{}",
        render_to_text(&harness.app, 32, 6)
    );
}

#[test]
fn every_mark_at_once_still_fits_one_line_and_cuts_the_subject() {
    let mut harness = harness_with(ascii_config());
    harness.app.submit(
        herdr_damnit_application::JobKind::ReadStatus,
        herdr_damnit_application::argv::status(),
    );
    harness.answer(
        0,
        0,
        r#"{"staged":[{"oid":"1a2b3c4","op":"update","before":null,
          "after":{"oid":"1a2b3c4","kind":"task","subject":"x"}}],
          "unstaged":[],"conflicts":[],"notices":[],"unpushed":[]}"#,
        "",
    );
    harness.app.submit(
        herdr_damnit_application::JobKind::ReadList,
        herdr_damnit_application::argv::list("!done"),
    );
    harness.answer(
        1,
        0,
        r#"{"objects":[{"oid":"1a2b3c4","kind":"task",
          "subject":"a subject long enough to need cutting","path":"p",
          "labels":["a","b","c"],"recurrence":"every day",
          "task":{"done":false,"priority":1,"due":"2026-09-01"}}]}"#,
        "",
    );

    let drawn = render_to_text(&harness.app, 32, 6);
    for line in drawn.lines() {
        assert!(line.chars().count() <= 32, "{line:?} is wider than the pane");
    }
    assert!(drawn.contains("+ ! < 09-01 ~"), "{drawn}");
}

#[test]
fn the_header_carries_the_spinner_and_the_elapsed_time_mid_push() {
    let mut harness = loaded();
    harness.app.submit(
        herdr_damnit_application::JobKind::Exclusive(herdr_damnit_application::SyncKind::Push),
        herdr_damnit_application::argv::push(),
    );

    let drawn = render_to_text(&harness.app, 32, 3);
    let header = drawn.lines().next().expect("a header");
    assert!(header.contains("push"), "{header}");
    assert!(header.contains('|') || header.contains('/'), "{header}");
}

#[test]
fn a_mark_takes_its_own_colour_and_the_subject_stays_plain() {
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    let harness = loaded();
    let mut terminal = Terminal::new(TestBackend::new(32, 8)).expect("a terminal");
    terminal
        .draw(|frame| draw(frame, &harness.app))
        .expect("it drew");
    let buffer = terminal.backend().buffer();

    let palette = crate::theme::resolve(None);
    let row = buffer
        .content()
        .chunks(32)
        .position(|line| line.iter().map(|cell| cell.symbol()).collect::<String>().contains("ship the pin"))
        .expect("the row is on screen");
    let priority = &buffer.content()[row * 32 + 2];
    assert_eq!(priority.fg, palette.color(herdr_damnit_domain::Slot::Red));
}
```

The golden in the first test is the exact screen. Write it by running the test once, reading the
`assert_eq` failure output, and pasting the left-hand side in, **after** checking line by line that
what it drew is what the spec's List mock describes. A golden pasted without that check pins a bug.

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p herdr-damnit --locked screens`
Expected: FAIL with `unresolved module or unlinked crate 'screens'`

- [ ] **Step 3: Write the screens module**

`crates/herdr-damnit/src/screens.rs`:

```rust
//! Drawing the pane. One module per screen; this file is the frame every screen shares and the
//! text renderer the golden tests compare.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout};

use crate::app::{App, Screen};
use crate::theme::Palette;

mod list;

/// The keys the hint line offers, cut to the pane's width.
const HINTS: &str = "x X dd p s D l m a S e <CR> <Space> c P L v Tab";

pub fn draw(frame: &mut Frame<'_>, app: &App) {
    let palette = crate::theme::resolve(app.config.theme.as_deref());
    let [header, body, hints] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .areas(frame.area());

    frame.render_widget(status_line(app, header.width), header);
    match app.screen {
        Screen::List => list::draw(frame, body, app, &palette),
        _ => list::draw(frame, body, app, &palette),
    }
    frame.render_widget(hint_line(hints.width), hints);
}

/// The whole pane as plain text, which is what a golden compares. Trailing blanks are trimmed per
/// line so a golden is a picture of the screen rather than a block of padding.
pub fn render_to_text(app: &App, width: u16, height: u16) -> String {
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(width, height))
        .expect("a test terminal");
    terminal.draw(|frame| draw(frame, app)).expect("it drew");
    let buffer = terminal.backend().buffer().clone();
    (0..height)
        .map(|row| {
            (0..width)
                .map(|column| buffer[(column, row)].symbol())
                .collect::<String>()
                .trim_end()
                .to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
        .trim_end()
        .to_string()
}

#[cfg(test)]
mod tests;
```

`status_line` puts `App::header(Instant::now())` on the left and the counts on the right, cutting the
left half first when the two do not fit; `hint_line` renders `HINTS` cut to the width with an
ellipsis. `crates/herdr-damnit/src/screens/list.rs` renders `app.list.rows()` as one
`ratatui::widgets::List`, each row a `Line` of `Span`s built from its `Segment`s with
`palette.color(segment.slot)`, the selected row highlighted, and the empty case one dim line reading
`nothing in this view`.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p herdr-damnit --locked screens`
Expected: PASS, five tests.

**Ruling 27.** The golden above was regenerated at the width the test asks for and differs from the
one sketched here. At 32 columns the counts sit at the right edge rather than nine columns in, the
first object row is exactly 32 cells and needs no cut at all, and the hint line ends
`<CR> <Sp` plus the ellipsis. Every line was checked against the spec's List mock before it was
pasted: the path headings, the recurring, priority, overdue and upcoming marks, the counted labels,
and the subject-last order that lets the subject be what gets cut.

**Ruling 28.** `the_header_carries_the_spinner_and_the_elapsed_time_mid_push` asserts against all
four frames of the plain spinner rather than the two written here. `loaded()` leaves two reads in
flight for two ticks, so the frame on screen is the third one, and pinning a particular frame would
pin how many ticks the reads before the push happened to take.

**Ruling 29.** `spans` and `cut` are moved from `render.rs` rather than written fresh: the
cut-the-line-and-end-in-an-ellipsis rule the spec asks for already exists there, measured in
terminal cells rather than characters so a double-width subject is cut where the terminal would wrap
it. `cut_to` is the same rule over a bare string, which the status line and the hint line both take.

- [ ] **Step 5: Commit**

```bash
git add -A
SKIP_AI_COMMIT=1 git commit -m "feat(pane): draw the list screen, the status line and the hints"
```

---

### Task 28: The cutover

**Files:**
- Rewrite: `crates/herdr-damnit/src/main.rs`
- Create: `crates/herdr-damnit/src/screens/refusal.rs`
- Create: `crates/herdr-damnit/src/open.rs` (the handshake plus the first reads)
- Delete: `crates/todoist/` in full
- Delete: `crates/herdr-damnit/src/{apply,cache,completed,connection,draft,edit,editor,history,list,icons,detail,queue,refresh,reload,render,send,prompt,tui,views,cursor,config}.rs`
  and their `tests` children
- Keep: `crates/herdr-damnit/src/{theme,markdown,placement,doctor,pane}.rs`
- Modify: `Cargo.toml`, `crates/herdr-damnit/Cargo.toml`

**Interfaces:**
- Consumes: everything built so far.
- Produces: a pane that opens, runs the handshake, reads `dam status` and `dam ls`, and draws the
  List screen. `main.rs` is under 150 lines.

This is the one task that deletes. It is one commit because the tree does not compile between the
deletion of the Todoist path and the wiring of the `dam` one.

`theme.rs` and `markdown.rs` stay in this crate because both produce ratatui values.
`placement.rs` stays because `Side` is a config type the adapters crate now re-exports; delete it and
import `herdr_damnit_adapters::config::Side` instead if the compiler makes that simpler.
`pane.rs` and `doctor.rs` stay and are re-aimed in Tasks 41 and 42; until then they may reference the
old config, so update their imports to `herdr_damnit_adapters::Config` in this task and leave their
behaviour alone.

- [ ] **Step 1: Write the failing test**

`crates/herdr-damnit/src/open.rs`, inside `mod tests`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::tests::harness;

    const CLEAN: &str = r#"{"staged":[],"unstaged":[],"conflicts":[],"notices":[],"unpushed":[]}"#;

    #[test]
    fn opening_asks_for_the_version_and_the_status_before_anything_else() {
        let mut harness = harness();
        start(&mut harness.app);
        harness.answer(0, 0, "dam 0.2.0\n", "");
        harness.answer(1, 0, CLEAN, "");

        let lines = harness.lines();
        assert_eq!(lines[0], "--version");
        assert_eq!(lines[1], "status --json");
        assert_eq!(lines[2], "ls !done --json");
    }

    #[test]
    fn a_dam_below_the_floor_draws_the_refusal_and_makes_no_further_call() {
        let mut harness = harness();
        start(&mut harness.app);
        harness.answer(0, 0, "dam 0.0.9\n", "");

        assert_eq!(harness.lines(), vec!["--version".to_string()]);
        assert!(harness.app.refusal.is_some());
        assert!(
            crate::screens::render_to_text(&harness.app, 32, 6)
                .contains("is older than the 0.2 this pane needs"),
            "{}",
            crate::screens::render_to_text(&harness.app, 32, 6)
        );
    }

    #[test]
    fn a_newer_dam_warns_once_rather_than_once_per_read() {
        let mut harness = harness();
        start(&mut harness.app);
        harness.answer(0, 0, "dam 0.4.0\n", "");
        harness.answer(1, 0, CLEAN, "");
        let warned = harness.app.message.clone();

        harness.app.submit(
            herdr_damnit_application::JobKind::ReadStatus,
            herdr_damnit_application::argv::status(),
        );
        harness.answer(3, 0, CLEAN, "");

        assert!(warned.contains("newer than this pane knows"), "{warned}");
        assert!(
            !harness.app.message.contains("newer than this pane knows"),
            "it warned twice: {}",
            harness.app.message
        );
    }

    #[test]
    fn a_status_document_missing_a_key_refuses_to_draw() {
        let mut harness = harness();
        start(&mut harness.app);
        harness.answer(0, 0, "dam 0.2.0\n", "");
        harness.answer(
            1,
            0,
            r#"{"staged":[],"unstaged":[],"conflicts":[],"notices":[]}"#,
            "",
        );

        assert!(harness.app.refusal.is_some());
        assert_eq!(harness.lines().len(), 2, "it read on past a failed handshake");
    }

    #[test]
    fn a_dam_that_is_not_there_draws_the_install_line_rather_than_a_refusal_screen() {
        let mut harness = crate::app::tests::harness_with_missing_dam();
        start(&mut harness.app);

        assert_eq!(
            harness.app.message,
            "dam is not on PATH; install it with cargo install damnit, then press R."
        );
        assert!(harness.app.refusal.is_none());
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p herdr-damnit --locked open`
Expected: FAIL with `unresolved module or unlinked crate 'open'`

- [ ] **Step 3: Write the opening sequence**

`crates/herdr-damnit/src/open.rs`:

```rust
//! What the pane does before it draws rows: the version handshake, and the first two reads. The
//! handshake is two process starts at open and is not repeated on a refresh.

use herdr_damnit_application::{JobKind, argv, handshake};

use crate::app::App;

/// Ask `dam` its version. Everything else follows from the answer.
pub fn start(app: &mut App) {
    app.submit(JobKind::Version, argv::version());
}

#[cfg(test)]
mod tests;
```

`JobKind` gains two variants for the handshake, `Version` and `Handshake`, and `App` gains
`refusal: Option<String>` plus `version: Option<DamVersion>` and `warned: bool`. `App::apply` gets
three new arms:

- a `Version` completion holds the version line until the `Handshake` status answers. It submits
  `JobKind::Handshake` with `argv::status()`.
- a `Handshake` completion calls `handshake(&held_version_line, &stdout)`. `Handshake::Refuse(said)`
  sets `app.refusal = Some(said)` and submits nothing further. `Handshake::Ready { version, warning }`
  stores the version, puts the warning in the status line once with `app.warned = true`, applies the
  status document as an ordinary `ReadStatus`, and submits the first `ReadList`.
- every other kind is unchanged.

`crates/herdr-damnit/src/screens/refusal.rs` draws `app.refusal` as one centred paragraph and no
hint line; `screens::draw` checks `app.refusal` first and draws that instead of any screen.

- [ ] **Step 4: Rewrite main.rs**

**The theme check lands here**, discharging the obligation Task 24 leaves and Task 25 carries.
`Config::parse` runs four checks and not the theme one, because the theme names live in this crate
rather than in the adapters crate (Task 24, Ruling 15). `load_config` below is the single funnel
both entry points take, so the check cannot be wired at one of them and forgotten at the other.

```rust
//! The plugin binary. With no arguments it is the pane; the subcommands are the plugin actions.

mod app;
mod doctor;
mod loop_;
mod markdown;
mod open;
mod pane;
mod screens;
mod theme;

use herdr_damnit_adapters::{Config, ProcessDamRunner, SystemClock};
use herdr_damnit_application::{Clock, Jobs};
use pane::Mode;

const USAGE: &str = "\
usage: herdr-damnit [<command>]

  (no command)   run the task pane
  open           open the pane in this workspace, or focus it when it is already open
  toggle         open the pane, or close it when it is already open
  focus          focus the pane in this workspace
  status         open the pane on the status screen
  auto-open      open the pane when the config asks for it, the workspace-focus hook
  view <n>       show the nth configured view, opening the pane when it is closed
  doctor         check that dam answers and the configured remotes resolve
";

fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [] => run_pane(),
        [command] => match command.as_str() {
            "--help" | "-h" | "help" => {
                print!("{USAGE}");
                std::process::ExitCode::SUCCESS
            }
            "doctor" => with_config(doctor::run),
            "open" => with_config(|config| pane::run(Mode::Open, config)),
            "toggle" => with_config(|config| pane::run(Mode::Toggle, config)),
            "focus" => with_config(|config| pane::run(Mode::Focus, config)),
            "status" => with_config(pane::open_on_status),
            "auto-open" => with_config(pane::auto_open),
            other => fail(&format!("unknown command '{other}'\n{USAGE}")),
        },
        [command, argument] if command == "view" => {
            with_config(|config| pane::view(argument, config))
        }
        _ => fail(&format!("too many arguments\n{USAGE}")),
    }
}

/// The configuration plus the one check the adapters crate cannot make for itself: the theme
/// vocabulary is this crate's, so `Config::parse` never sees it and every load goes through here.
/// Skip this and an unknown theme name is accepted in silence and the pane paints with defaults.
fn load_config() -> Result<Config, String> {
    let config = Config::load()?;
    config.check_theme_against(theme::NAMES)?;
    Ok(config)
}

fn run_pane() -> std::process::ExitCode {
    let config = match load_config() {
        Ok(config) => config,
        Err(error) => return fail(&error),
    };
    let clock = SystemClock;
    let jobs = Jobs::new(Box::new(ProcessDamRunner::new(config.dam.clone())));
    let mut app = app::App::new(config, jobs, clock.today());
    open::start(&mut app);
    match loop_::run(&mut app) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => fail(&error),
    }
}

fn with_config(run: impl FnOnce(&Config) -> Result<String, String>) -> std::process::ExitCode {
    report(load_config().and_then(|config| run(&config)))
}

fn report(outcome: Result<String, String>) -> std::process::ExitCode {
    match outcome {
        Ok(message) => {
            println!("{message}");
            std::process::ExitCode::SUCCESS
        }
        Err(error) => fail(&error),
    }
}

fn fail(error: &str) -> std::process::ExitCode {
    eprintln!("herdr-damnit: {error}");
    std::process::ExitCode::FAILURE
}
```

- [ ] **Step 5: Delete the Todoist path**

```bash
git rm -r crates/todoist
git rm crates/herdr-damnit/src/apply.rs crates/herdr-damnit/src/cache.rs \
  crates/herdr-damnit/src/completed.rs crates/herdr-damnit/src/config.rs \
  crates/herdr-damnit/src/connection.rs crates/herdr-damnit/src/cursor.rs \
  crates/herdr-damnit/src/detail.rs crates/herdr-damnit/src/draft.rs \
  crates/herdr-damnit/src/edit.rs crates/herdr-damnit/src/editor.rs \
  crates/herdr-damnit/src/history.rs crates/herdr-damnit/src/icons.rs \
  crates/herdr-damnit/src/list.rs crates/herdr-damnit/src/placement.rs \
  crates/herdr-damnit/src/prompt.rs crates/herdr-damnit/src/queue.rs \
  crates/herdr-damnit/src/refresh.rs crates/herdr-damnit/src/reload.rs \
  crates/herdr-damnit/src/render.rs crates/herdr-damnit/src/send.rs \
  crates/herdr-damnit/src/tui.rs crates/herdr-damnit/src/views.rs
git rm -r crates/herdr-damnit/src/apply crates/herdr-damnit/src/detail \
  crates/herdr-damnit/src/edit crates/herdr-damnit/src/list \
  crates/herdr-damnit/src/queue crates/herdr-damnit/src/reload \
  crates/herdr-damnit/src/render crates/herdr-damnit/src/send \
  crates/herdr-damnit/src/tui
```

Then remove `crates/todoist` from the workspace `members`, and remove `todoist`, `reqwest`, `tokio`
and `chrono` from every `[dependencies]` and from `[workspace.dependencies]`.

- [ ] **Step 6: Run the whole suite to verify it passes**

Run: `cargo test --workspace --locked -- --test-threads=1 && cargo clippy --workspace --all-targets --locked -- -D warnings`
Expected: PASS and clean.

- [ ] **Step 7: Prove the network left the tree**

```bash
! cargo tree --workspace --locked --edges normal | grep -E 'reqwest|hyper|rustls|tokio|native-tls'
wc -l crates/herdr-damnit/src/main.rs
```

Expected: no match, and `main.rs` under 150 lines.

**Ruling 30.** The version verdict is decided on the `Version` completion, not on the `Handshake`
one. `a_dam_below_the_floor_draws_the_refusal_and_makes_no_further_call` requires
`lines() == ["--version"]`, so a `dam` this pane refuses is never sent a status read, which is also
the right behaviour: a command surface the pane does not know is not one to read from.
`handshake(version, status)` could not give that ordering, so it was replaced by the two checks the
flow needs, both in `handshake.rs`: `check_version(version_output) -> Handshake` and
`check_status(status_output) -> Result<(), String>`. The pane and `doctor` call those checks in
order, and no production caller holds both documents for a combined helper.

**Ruling 31.** `a_newer_dam_warns_once_rather_than_once_per_read` asserts the warning does not come
back rather than that a later read replaced it. As written it cleared nothing and then required
`message` not to contain the warning after an unrelated `ReadStatus`, which no implementation
satisfies: a successful read leaves the status line alone, and it has to, or the follow-up reads a
push enqueues would wipe the push summary the operator just earned. The test now clears the message
itself and asserts it stays clear, which is exactly the invariant `warned` exists for.

**Ruling 32.** `a_dam_below_the_floor_draws_the_refusal` compares the drawn refusal with its line
breaks taken back out. The sentence is 100 characters and the pane is 32 columns, so no layout keeps
it on one line and `contains` over the raw render could never pass.

**Ruling 33.** `doctor.rs` is rewritten here rather than kept. Keeping it was impossible: its whole
body is `todoist::Client`, `config.token_source()` and two `#[tokio::test]`s, and this task deletes
the crate, the config method and the runtime. It is rewritten to the half of Task 41's doctor that
the handshake already provides, `dam`'s version and one `dam status --json` judged on its five keys,
under four tests over a pure `report_from`. Task 41 adds the remote list and the `PATH` report and
will find these four already green.

**Ruling 34.** `main.rs` carries no `status` command and no `status` line in its usage text. The
`status` arm above calls `pane::open_on_status`, which Task 41 produces; wiring a command here would
mean either a compile error or landing Task 41's behaviour untested. Task 41 adds the arm, the usage
line and the test together.

**Ruling 35.** `placement.rs` is deleted, resolving this task's own contradiction: the Keep line
names it and Step 5's `git rm` list also names it. Its `Side` was already dead after Task 25 moved
`pane.rs` onto the adapters config, and its one remaining rule, `leading_share`, is four lines that
only `pane::arrange_pane` calls, so it moves into `pane.rs` as a private function with its comment
and its test.

**Ruling 36.** `markdown.rs` stays in the tree and is NOT declared in `main.rs`. Nothing reaches it
until Task 30 draws the Detail screen from it, and a declared `mod markdown;` whose only item
nothing calls fails `clippy -D warnings` on dead code, which this task's Step 6 runs. Task 30 adds
the `mod markdown;` line along with the screen that renders through it.

**Ruling 37.** Three more items went with the cutover for the same dead-code reason:
`theme::is_known`, which the Todoist-era config called and `check_theme_against(theme::NAMES)`
replaces; `screens::render_to_text`, now `#[cfg(test)]` because it is the goldens' renderer and
nothing in the binary draws through it; and `Harness::press` and `Harness::last` in `app/tests.rs`.
Task 29 re-adds `press` with the Tab-cycle tests that call it.

- [ ] **Step 8: Commit**

```bash
git add -A
SKIP_AI_COMMIT=1 git commit -m "feat(pane): open on dam and delete the Todoist client"
```

---

### Task 29: The Status and Done screens, and the Tab cycle

**Files:**
- Create: `crates/herdr-damnit/src/screens/status.rs`
- Create: `crates/herdr-damnit/src/screens/done.rs`
- Modify: `crates/herdr-damnit/src/screens.rs`, `crates/herdr-damnit/src/app.rs`
- Modify: `crates/herdr-damnit/src/screens/tests.rs`

**Interfaces:**
- Consumes: `Stage`, `StatusRow` from Task 11; `completions` from Task 23.
- Produces: `status::draw` and `done::draw` with the same signature as `list::draw`;
  `App::key` handling `<Tab>` and `<S-Tab>`; `App::done_rows`, which builds the Done screen from
  `dam ls "done" --json` and the completion dates from `dam log --json`.

`<Tab>` cycles List, Status, Done forward and `<S-Tab>` back, each screen keeping its own cursor.
Detail is not in the cycle. Entering the Done screen submits `ReadDone` and `ReadLog` when the pane
has not read them yet; both are local.

Done rows are newest completion first, the date first so the dates line up down the pane. A task
completed in the working layer and not yet committed has no commit and therefore no date; it sorts to
the top under the heading `not committed`.

- [ ] **Step 1: Write the failing tests**

Add to `crates/herdr-damnit/src/screens/tests.rs`:

```rust
const FULL_STATUS: &str = r#"{
  "staged":[
    {"oid":"1a2b3c4","op":"create","before":null,
     "after":{"oid":"1a2b3c4","kind":"task","subject":"ship the pin bump"}}],
  "unstaged":[
    {"oid":"9a0b1c2","op":"update",
     "before":{"oid":"9a0b1c2","kind":"task","subject":"water the plants"},
     "after":{"oid":"9a0b1c2","kind":"task","subject":"water the plants"}}],
  "unpushed":[{"remote":"example","commits":1}],
  "conflicts":[{"oid":"3d4e5f6","remote":"example",
    "ours":{"oid":"3d4e5f6","kind":"task","subject":"mine"},
    "theirs":{"oid":"3d4e5f6","kind":"task","subject":"theirs"}}],
  "notices":[{"kind":"pull_failed","remote":"example","why":"unreachable"}]}"#;

#[test]
fn tab_cycles_the_three_screens_forward_and_shift_tab_back() {
    let mut harness = loaded();
    assert_eq!(harness.app.screen, Screen::List);

    harness.press(KeyCode::Tab);
    assert_eq!(harness.app.screen, Screen::Status);
    harness.press(KeyCode::Tab);
    assert_eq!(harness.app.screen, Screen::Done);
    harness.press(KeyCode::Tab);
    assert_eq!(harness.app.screen, Screen::List);
    harness.press(KeyCode::BackTab);
    assert_eq!(harness.app.screen, Screen::Done);
}

#[test]
fn each_screen_keeps_its_own_cursor_across_the_cycle() {
    let mut harness = loaded();
    harness.press(KeyCode::Char('j'));
    let on_list = harness.app.list.selected_oid().cloned();

    harness.press(KeyCode::Tab);
    harness.press(KeyCode::Tab);
    harness.press(KeyCode::Tab);

    assert_eq!(harness.app.list.selected_oid().cloned(), on_list);
}

#[test]
fn the_status_screen_draws_the_four_sections_in_dams_order() {
    let mut harness = harness_with(ascii_config());
    harness.app.submit(
        herdr_damnit_application::JobKind::ReadStatus,
        herdr_damnit_application::argv::status(),
    );
    harness.answer(0, 0, FULL_STATUS, "");
    harness.app.screen = Screen::Status;

    let drawn = render_to_text(&harness.app, 32, 12);
    let headings: Vec<&str> = drawn
        .lines()
        .filter(|line| ["Staged", "Working", "Unpushed", "Notices"].contains(line))
        .collect();
    assert_eq!(headings, vec!["Staged", "Working", "Unpushed", "Notices"]);
    assert!(drawn.contains("example: pull failed: unreachable"), "{drawn}");
}

#[test]
fn a_clean_status_screen_says_so_in_dams_own_words() {
    let mut harness = harness_with(ascii_config());
    harness.app.submit(
        herdr_damnit_application::JobKind::ReadStatus,
        herdr_damnit_application::argv::status(),
    );
    harness.answer(0, 0, CLEAN, "");
    harness.app.screen = Screen::Status;

    assert!(
        render_to_text(&harness.app, 32, 6).contains("nothing staged, nothing changed"),
        "{}",
        render_to_text(&harness.app, 32, 6)
    );
}

#[test]
fn entering_the_done_screen_reads_the_done_list_and_the_log_once() {
    let mut harness = loaded();
    harness.press(KeyCode::Tab);
    harness.press(KeyCode::Tab);

    let lines = harness.lines();
    assert!(lines.contains(&"ls done --json".to_string()), "{lines:?}");
    assert!(lines.contains(&"log --json".to_string()), "{lines:?}");

    let before = lines.len();
    harness.press(KeyCode::Tab);
    harness.press(KeyCode::Tab);
    harness.press(KeyCode::Tab);
    assert_eq!(harness.lines().len(), before, "it re-read a screen it already had");
}

#[test]
fn the_done_screen_draws_the_date_first_newest_first_with_the_uncommitted_ones_on_top() {
    let mut harness = harness_with(ascii_config());
    harness.app.screen = Screen::Done;
    harness.app.submit(
        herdr_damnit_application::JobKind::ReadDone,
        herdr_damnit_application::argv::list("done"),
    );
    harness.answer(
        0,
        0,
        r#"{"objects":[
          {"oid":"aaa","kind":"task","subject":"older","task":{"done":true,"priority":4}},
          {"oid":"bbb","kind":"task","subject":"newer","task":{"done":true,"priority":4}},
          {"oid":"ccc","kind":"task","subject":"just now","task":{"done":true,"priority":4}}]}"#,
        "",
    );
    harness.app.submit(
        herdr_damnit_application::JobKind::ReadLog,
        herdr_damnit_application::argv::log(),
    );
    harness.answer(
        1,
        0,
        r#"{"commits":[
          {"id":"c1","at":"2026-09-18T08:00:00Z","message":"one","changes":[
            {"oid":"aaa","op":"update",
             "before":{"oid":"aaa","kind":"task","subject":"older","task":{"done":false,"priority":4}},
             "after":{"oid":"aaa","kind":"task","subject":"older","task":{"done":true,"priority":4}}}]},
          {"id":"c2","at":"2026-09-20T08:00:00Z","message":"two","changes":[
            {"oid":"bbb","op":"update",
             "before":{"oid":"bbb","kind":"task","subject":"newer","task":{"done":false,"priority":4}},
             "after":{"oid":"bbb","kind":"task","subject":"newer","task":{"done":true,"priority":4}}}]}]}"#,
        "",
    );

    let drawn = render_to_text(&harness.app, 32, 10);
    let body: Vec<&str> = drawn.lines().skip(1).filter(|line| !line.is_empty()).collect();
    assert_eq!(body[0], "not committed");
    assert_eq!(body[1], "  just now");
    assert_eq!(body[2], "2026-09-20  newer");
    assert_eq!(body[3], "2026-09-18  older");
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p herdr-damnit --locked screens`
Expected: FAIL with `no variant named 'ReadDone'`

- [ ] **Step 3: Write the two screens and the cycle**

`JobKind` gains `ReadDone`. `App` gains `done_objects: Vec<Object>`, `completed: HashMap<Oid, Date>`,
`read_done: bool` and `read_log: bool`. `App::key` gains:

```rust
            KeyCode::Tab => self.show(self.screen.next()),
            KeyCode::BackTab => self.show(self.screen.previous()),
```

with `Screen::next` and `Screen::previous` cycling `List`, `Status`, `Done` and leaving `Detail`
alone, and `App::show` submitting `ReadDone` and `ReadLog` on the first entry to `Done`.

`screens/status.rs` renders `app.stage.rows()`, each `StatusRow::Change` coloured by its mark, each
`StatusRow::Line` drawing its `Option<Mark>` in the same mark column when it carries one, and a
`Heading` plain. No row's mark is inferred from the section it sits under. `screens/done.rs` groups by completion date, newest first, with the uncommitted ones
under a `not committed` heading at the top, and draws `YYYY-MM-DD  <subject>` with the date first.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p herdr-damnit --locked screens`
Expected: PASS, eleven tests.

**Ruling 39.** The log fixture above is rewritten to the shape `dam` 0.2.0 actually prints. It uses
`before` and `after` objects per change, which the spec's own Done section describes, but Task 23
measured the real document and landed a parser over `fields`, `done` and `completed_at`: a change is
a completion when its `fields` names `done` and it leaves the object complete, and the day is its
`completed_at` or the commit's own `at`. The captured
`crates/herdr-damnit-adapters/tests/fixtures/log-completion.json` is the proof. Against the fixture
as written, `completions` finds nothing and all three tasks sort under `not committed`.

**Ruling 40.** `the_status_screen_draws_the_four_sections_in_dams_order` asserts
`contains("example: pull failed")` rather than the whole sentence. `dam`'s notice reads
`example: pull failed: unreachable` and the row carries its own two-space indent, which is 35 cells
against a 32-column pane, so the last word is cut by width on any implementation. The notice's full
text is already pinned by the wire tests in `wire/reports/tests.rs`.

**Ruling 41.** `each_screen_keeps_its_own_cursor_across_the_cycle` moves the cursor with
`app.list.move_by(1)` rather than by pressing `j`. Navigation keys are Task 31's produce, and with
`j` unbound the test as written compared the first row with the first row and would have passed
against any implementation, including one that reset the cursor on every Tab.

**Ruling 42.** No `status: Cursor` or `done: Cursor` field lands here. Nothing in this task reads
one, so both would be dead fields under `clippy -D warnings`, and Task 31 owns the navigation that
moves them. The one cursor that exists, the List's, is what
`each_screen_keeps_its_own_cursor_across_the_cycle` pins.

**Ruling 43.** Two behaviours the spec specifies and this task's own tests did not reach were added
with tests of their own. The status line names the screen on show, `dam  status` and `dam  done`
against the spec's two mocks, and its counts are per screen: the List screen's own open, staged and
unpushed marks, `Stage::summary()` on the Status screen, which the fixture pins as
`1 staged  1 changed  1 unpushed  1 notice`, and a count of rows on the Done screen. And
`a_status_row_draws_its_own_mark_and_a_row_with_none_draws_none` pins this task's stated rule that
no row's mark is inferred from the section above it: without it, marking every `StatusRow::Line`
with a fabricated staged mark passed the whole suite, measured.

**Ruling 44.** A marked Status row draws its mark inside the row's own two-space indent, which is
where the spec's Status mock puts it (`  + new      1a2b3c4  ...`, `  ^ todoist  1 commit`). A row
with no mark draws its text as the staging model wrote it, indent and all, which is also what keeps
the clean line's 31-character sentence inside a 32-column pane.

**Ruling 45.** `app.rs` reached 407 lines with the cycle, the done rows and the header in it, past
the 250-implementation-line decomposition threshold, so three cohesive child modules were split out:
`app/screen.rs` (the screens and the order Tab walks them), `app/done.rs` (the Done screen's rows,
grouped by completion day) and `app/header.rs` (the status line's left half, the spinner and the
timer). `app.rs` is 276 lines after the split. `screens/list.rs::spans` became `pub(super)`: three
screens now draw coloured segment runs through it.

- [ ] **Step 5: Commit**

```bash
git add -A
SKIP_AI_COMMIT=1 git commit -m "feat(pane): add the status and done screens to the tab cycle"
```

---

### Task 30: The Detail screen

**Ruling 38 (carried in from Task 28).** `crates/herdr-damnit/src/markdown.rs` is in the tree but is
not declared in `main.rs`: Task 28 left the declaration out because nothing called it and
`clippy -D warnings` rejects a module whose only item is dead. This task adds `mod markdown;` to
`main.rs` with the Detail screen that renders through it.

**Files:**
- Create: `crates/herdr-damnit/src/screens/detail.rs`
- Modify: `crates/herdr-damnit/src/screens.rs`, `crates/herdr-damnit/src/app.rs`
- Modify: `crates/herdr-damnit/src/screens/tests.rs`

**Interfaces:**
- Consumes: `Object`, `EventFields`, `Attendee` from Task 7; `markdown` from the binary crate.
- Produces: `detail::draw`; `App::key` handling `<CR>` on a row naming an oid, which submits
  `ReadShow(oid)`, and `<Esc>` on the Detail screen, which returns to the screen under it.

The fields drawn, in order: subject as the heading, then `path`, `kind`, `priority`, `due`,
`deadline`, `recurrence`, `labels` named in full, `depends` as short oids with their subjects resolved
from the model, `event` when the task is attached to one, then `body` rendered as markdown. For an
event: `start`, `end`, `timezone`, `location`, `status`, `transparency` and the attendees with their
responses.

The markdown renderer is kept exactly as it is and every line wraps rather than being cut, which the
existing 32-column test already pins. There is no comment thread and no attachment line, because
`dam` version one stores neither.

- [ ] **Step 1: Write the failing tests**

Add to `crates/herdr-damnit/src/screens/tests.rs`:

```rust
#[test]
fn enter_on_a_row_reads_that_object_and_opens_the_detail() {
    let mut harness = loaded();
    harness.press(KeyCode::Enter);
    assert_eq!(harness.last(), "show 5d6e7f8 --json");

    harness.answer(
        2,
        0,
        r#"{"oid":"5d6e7f8","kind":"task","subject":"refresh the roster row",
          "path":"proj/dotfiles","body":"# why\n\nthe pin moved","labels":["slow","home"],
          "recurrence":"every week","depends":["1a2b3c4"],
          "task":{"done":false,"priority":2,"due":"2026-09-22","deadline":"2026-09-30"}}"#,
        "",
    );

    assert_eq!(harness.app.screen, Screen::Detail);
    let drawn = render_to_text(&harness.app, 32, 20);
    for expected in [
        "refresh the roster row",
        "path: proj/dotfiles",
        "kind: task",
        "priority: p2",
        "due: 2026-09-22",
        "deadline: 2026-09-30",
        "recurrence: every week",
        "labels: slow, home",
        "why",
        "the pin moved",
    ] {
        assert!(drawn.contains(expected), "{expected:?} missing from\n{drawn}");
    }
}

#[test]
fn a_dependency_is_drawn_as_its_short_oid_and_the_subject_the_model_already_holds() {
    let mut harness = loaded();
    harness.press(KeyCode::Enter);
    harness.answer(
        2,
        0,
        r#"{"oid":"5d6e7f8","kind":"task","subject":"refresh the roster row",
          "depends":["1a2b3c4"],"task":{"done":false,"priority":4}}"#,
        "",
    );

    assert!(
        render_to_text(&harness.app, 32, 20).contains("1a2b3c4  ship the pin bump"),
        "{}",
        render_to_text(&harness.app, 32, 20)
    );
}

#[test]
fn an_event_draws_its_own_fields_and_its_attendees_with_their_responses() {
    let mut harness = loaded();
    harness.press(KeyCode::Enter);
    harness.answer(
        2,
        0,
        r#"{"oid":"eee","kind":"event","subject":"stand-up","event":{
          "start":"2026-09-21T09:00","end":"2026-09-21T09:15","timezone":"UTC",
          "location":"the kitchen","status":"confirmed","transparency":"busy",
          "attendees":[{"email":"a@example.test","response":"accepted"}]}}"#,
        "",
    );

    let drawn = render_to_text(&harness.app, 32, 20);
    for expected in [
        "start: 2026-09-21T09:00",
        "end: 2026-09-21T09:15",
        "timezone: UTC",
        "location: the kitchen",
        "status: confirmed",
        "transparency: busy",
        "a@example.test: accepted",
    ] {
        assert!(drawn.contains(expected), "{expected:?} missing from\n{drawn}");
    }
}

#[test]
fn a_field_the_object_has_nothing_for_is_left_out_of_the_detail() {
    let mut harness = loaded();
    harness.press(KeyCode::Enter);
    harness.answer(
        2,
        0,
        r#"{"oid":"5d6e7f8","kind":"task","subject":"bare","task":{"done":false,"priority":4}}"#,
        "",
    );

    let drawn = render_to_text(&harness.app, 32, 20);
    for absent in ["due:", "deadline:", "recurrence:", "labels:", "depends:", "priority:"] {
        assert!(!drawn.contains(absent), "{absent:?} was drawn empty in\n{drawn}");
    }
}

#[test]
fn escape_leaves_the_detail_for_the_screen_under_it_with_its_cursor_untouched() {
    let mut harness = loaded();
    harness.press(KeyCode::Char('j'));
    let under = harness.app.list.selected_oid().cloned();
    harness.press(KeyCode::Enter);
    harness.answer(
        2,
        0,
        r#"{"oid":"5d6e7f8","kind":"task","subject":"x","task":{"done":false,"priority":4}}"#,
        "",
    );

    harness.press(KeyCode::Esc);
    assert_eq!(harness.app.screen, Screen::List);
    assert_eq!(harness.app.list.selected_oid().cloned(), under);
}

#[test]
fn enter_on_a_heading_does_nothing_and_says_nothing() {
    let mut harness = loaded();
    harness.app.list.replace(harness.app.list.rows().to_vec());
    let before = harness.lines().len();
    harness.app.select_row(0);
    harness.press(KeyCode::Enter);

    assert_eq!(harness.lines().len(), before);
    assert!(harness.app.message.is_empty());
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p herdr-damnit --locked screens::tests::enter_on_a_row`
Expected: FAIL, the detail is never drawn.

- [ ] **Step 3: Write the screen**

`App` gains `detail: Option<Object>` and `under_detail: Screen`. `App::key` gains an `Enter` arm that
reads the oid under the cursor of the showing screen and submits `ReadShow(oid)`; the `ReadShow`
completion parses with `wire::object`, stores it and sets `screen = Screen::Detail`. `Esc` on Detail
restores `under_detail`.

`screens/detail.rs` builds a `Vec<Line>`: the subject as the heading in `Slot::Text`, then one line
per present field, then the attendees, then `crate::markdown::render(&object.body)`, wrapped at the
pane width rather than cut. A dependency line resolves its subject out of `app.objects` by oid and
falls back to the short oid alone when the model does not hold it.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p herdr-damnit --locked screens`
Expected: PASS, seventeen tests.

- [ ] **Step 5: Commit**

```bash
git add -A
SKIP_AI_COMMIT=1 git commit -m "feat(pane): draw one object's detail from dam show"
```

---

### Task 31: Navigation, the view picker, the number keys and the interval

**Ruling 26 (carried in from Task 26).** `config.default_view` has no owner anywhere in this plan.
`Config::check_default_view` proves the name is a view that exists and nothing ever selects it, so
`default_view = "today"` in the operator's own config file opens the pane on the open list in
silence. This task owns view selection, so it discharges it: `App::new` follows `Views::new` with
`views.select_named(name)` for a configured `default_view`, under a test that asserts the showing
view.

**Files:**
- Create: `crates/herdr-damnit/src/overlay.rs`
- Create: `crates/herdr-damnit/src/overlay/picker.rs`
- Modify: `crates/herdr-damnit/src/app.rs`, `crates/herdr-damnit/src/screens.rs`
- Create: `crates/herdr-damnit/src/app/keys_tests.rs`

**Interfaces:**
- Consumes: `Views`, `MAX_NUMBERED_VIEW` from Task 10; `state::take_requested_view` from Task 25.
- Produces:

```rust
pub struct Picker { pub title: String, pub entries: Vec<PickerEntry>, pub selected: usize }
pub struct PickerEntry { pub text: String, pub marked: bool }

pub enum Overlay { View(Picker), Label(Picker), Path(Picker), Line(LineBox), Note(NoteBox), Confirm(Confirm) }

impl Picker { pub fn move_by(&mut self, steps: isize); pub fn current(&self) -> Option<&PickerEntry>; }
```

and these keys on `App`:

| Key | Behaviour |
|---|---|
| `j`, `<Down>` | the cursor of the showing screen moves to the next row; at the last row it stays |
| `k`, `<Up>` | the cursor moves to the previous row; at the first row it stays |
| `v` | on List or Done, the view picker opens over the body with the showing view under the cursor |
| `1` to `9` | the List screen is drawn on that view, read with `dam ls <query> --json`; a number with no view behind it says `the config has <n> views.` |
| `R`, `r` | `dam ls` and `dam status` are both re-read and the interval timer is put back to a full interval |

The `view` action runs as its own process, so its request arrives as a file rather than as a key:
`App::tick` takes it with `state::take_requested_view` and selects that view. The read is consuming,
so the pane does not pull itself back to it after the operator has moved on.

**`Views::select` and `Views::select_named` cannot tell a refusal from a no-op.** Both answer `false`
for an index past the end, a name that is not there, and a selection that was already showing, so
the number keys must not read that `false` as "no such view": pressing `7` with six views configured
is the message `the config has 6 views.`, while pressing the number of the view already showing is
silence. Ask `name_of_number` first, which answers `None` only when the view is absent, and use
`select` for the move.

The interval read defaults to 300 seconds and is held back while an overlay is open or a screen other
than List is showing, so a half-typed line is never redrawn away.

- [ ] **Step 1: Write the failing tests**

`crates/herdr-damnit/src/app/keys_tests.rs`:

```rust
use crossterm::event::KeyCode;
use herdr_damnit_adapters::Config;

use super::tests::{harness, harness_with};
use super::*;

fn three_views() -> Config {
    Config::parse(
        "icons = \"ascii\"\n\
         [[views]]\nname = \"today\"\nquery = \"!done & due:today\"\n\
         [[views]]\nname = \"deep\"\nquery = \"!done & effort:deep\"\n",
    )
    .expect("parses")
}

#[test]
fn j_and_k_move_the_cursor_and_stop_at_either_end() {
    let mut harness = super::super::screens::tests::loaded();
    let first = harness.app.list.selected_oid().cloned();

    harness.press(KeyCode::Char('j'));
    assert_ne!(harness.app.list.selected_oid().cloned(), first);
    harness.press(KeyCode::Char('k'));
    assert_eq!(harness.app.list.selected_oid().cloned(), first);
    harness.press(KeyCode::Char('k'));
    assert_eq!(harness.app.list.selected_oid().cloned(), first, "it moved past the first row");
}

#[test]
fn a_number_key_shows_that_view_and_reads_its_query() {
    let mut harness = harness_with(three_views());
    harness.press(KeyCode::Char('2'));

    assert_eq!(harness.app.views.current().name, "today");
    assert_eq!(harness.last(), "ls !done & due:today --json");
    assert_eq!(harness.app.screen, Screen::List);
}

#[test]
fn a_number_with_no_view_behind_it_says_how_many_there_are_and_reads_nothing() {
    let mut harness = harness_with(three_views());
    let before = harness.lines().len();
    harness.press(KeyCode::Char('7'));

    assert_eq!(harness.app.message, "the config has 3 views.");
    assert_eq!(harness.lines().len(), before);
}

#[test]
fn v_opens_the_view_picker_with_the_showing_view_under_the_cursor() {
    let mut harness = harness_with(three_views());
    harness.press(KeyCode::Char('3'));
    harness.press(KeyCode::Char('v'));

    let Some(Overlay::View(picker)) = harness.app.overlay.as_ref() else {
        panic!("expected the view picker");
    };
    assert_eq!(
        picker.entries.iter().map(|entry| entry.text.as_str()).collect::<Vec<_>>(),
        vec!["open", "today", "deep"]
    );
    assert_eq!(picker.selected, 2);
}

#[test]
fn the_view_picker_takes_the_entry_under_the_cursor_and_closes() {
    let mut harness = harness_with(three_views());
    harness.press(KeyCode::Char('v'));
    harness.press(KeyCode::Char('j'));
    harness.press(KeyCode::Enter);

    assert!(harness.app.overlay.is_none());
    assert_eq!(harness.app.views.current().name, "today");
    assert_eq!(harness.last(), "ls !done & due:today --json");
}

#[test]
fn escape_closes_the_picker_and_changes_no_view() {
    let mut harness = harness_with(three_views());
    harness.press(KeyCode::Char('v'));
    harness.press(KeyCode::Char('j'));
    let before = harness.lines().len();
    harness.press(KeyCode::Esc);

    assert!(harness.app.overlay.is_none());
    assert_eq!(harness.app.views.current().name, "open");
    assert_eq!(harness.lines().len(), before);
}

#[test]
fn r_re_reads_both_the_list_and_the_status() {
    let mut harness = harness();
    let before = harness.lines().len();
    harness.press(KeyCode::Char('R'));

    let asked = &harness.lines()[before..];
    assert!(asked.contains(&"ls !done --json".to_string()), "{asked:?}");
    assert!(asked.contains(&"status --json".to_string()), "{asked:?}");
}

#[test]
fn a_view_request_file_outranks_the_showing_view_and_is_read_once() {
    let mut harness = harness_with(three_views());
    let path = std::env::temp_dir().join(format!("herdr-damnit-request-{}", std::process::id()));
    std::fs::write(&path, "deep").expect("the request is written");
    harness.app.view_request_path = path.clone();

    harness.app.tick(std::time::Instant::now());
    assert_eq!(harness.app.views.current().name, "deep");

    harness.app.views.select_named("open");
    harness.app.tick(std::time::Instant::now());
    assert_eq!(
        harness.app.views.current().name,
        "open",
        "the request outlived its read"
    );
}

#[test]
fn the_interval_is_held_back_while_an_overlay_is_open() {
    let mut harness = harness_with(three_views());
    harness.press(KeyCode::Char('v'));
    let before = harness.lines().len();

    harness.app.force_interval_due();
    harness.app.tick(std::time::Instant::now());

    assert_eq!(harness.lines().len(), before, "it re-read under an open picker");
}

#[test]
fn a_query_dam_refuses_leaves_the_rows_on_screen() {
    let mut harness = super::super::screens::tests::loaded();
    let before = harness.app.list.object_count();
    harness.app.submit(
        herdr_damnit_application::JobKind::ReadList,
        herdr_damnit_application::argv::list("bogus:x"),
    );
    harness.answer(2, 2, "", "dam: \"bogus\" is not a declared category");

    assert_eq!(harness.app.message, "\"bogus\" is not a declared category");
    assert_eq!(harness.app.list.object_count(), before);
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p herdr-damnit --locked keys_tests`
Expected: FAIL with `unresolved module or unlinked crate 'keys_tests'`

- [ ] **Step 3: Write the overlay and the keys**

`crates/herdr-damnit/src/overlay.rs` declares `Overlay` with its six variants (the last three are
empty placeholders filled by Tasks 32 and 36) and `overlay/picker.rs` holds `Picker`, `PickerEntry`,
`move_by` and `current`. `screens::draw` renders an open overlay over the body: the picker as a
bordered list, marked entries carrying the staged mark.

`App::key` grows a two-level match: an open overlay takes the key first, and only an absent one
reaches the screen keys. `App` gains `view_request_path: PathBuf`, an interval `Schedule` with
`force_interval_due()` for the test, and `reread()` which submits `ReadList` and `ReadStatus` and
marks the schedule.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p herdr-damnit --locked`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add -A
SKIP_AI_COMMIT=1 git commit -m "feat(pane): move the cursor, pick a view and re-read on demand"
```

---

### Task 32: The staging keys and the commit box

**Files:**
- Create: `crates/herdr-damnit/src/overlay/line.rs`
- Modify: `crates/herdr-damnit/src/app.rs`, `crates/herdr-damnit/src/overlay.rs`,
  `crates/herdr-damnit/src/screens.rs`, `crates/herdr-damnit/src/app/keys_tests.rs`

**Interfaces:**
- Consumes: `Stage::is_staged`, `staged_count` from Task 11; the staging argv from Task 16.
- Produces:

```rust
pub struct LineBox { pub title: String, pub hint: String, pub text: String, pub purpose: LinePurpose }
pub enum LinePurpose { Commit, Due(Oid), Deadline(Oid), New(String) }

impl LineBox {
    pub fn push(&mut self, character: char);
    pub fn backspace(&mut self);
    pub fn text(&self) -> &str;
}
```

and these keys:

| Key | Given | Then |
|---|---|---|
| `<Space>` | a row naming an oid, not staged | `dam add <oid>` runs |
| `<Space>` | a row naming an oid, staged | `dam reset <oid>` runs |
| `A` | any screen | `dam add -A` runs |
| `U` | any screen | `dam reset` with no oid runs |
| `c` | the stage is not empty | a one-line box opens over the body, headed with the count `dam status` reported |
| `c` | the box is open, `<CR>` | `dam commit -m "<line>" --json` runs; a blank line is refused in the pane with `a commit needs a message.` and nothing is spawned |
| `c` | the box is open, `<Esc>` | the box closes and nothing is spawned |
| `c` | the stage is empty | `nothing staged; press <Space> on a row or A to stage everything.` and no box opens |

- [ ] **Step 1: Write the failing tests**

Add to `crates/herdr-damnit/src/app/keys_tests.rs`:

```rust
fn staged_harness() -> super::tests::Harness {
    let mut harness = super::super::screens::tests::loaded();
    harness.app.submit(
        herdr_damnit_application::JobKind::ReadStatus,
        herdr_damnit_application::argv::status(),
    );
    harness.answer(
        2,
        0,
        r#"{"staged":[{"oid":"5d6e7f8","op":"update","before":null,
          "after":{"oid":"5d6e7f8","kind":"task","subject":"refresh the roster row"}}],
          "unstaged":[],"conflicts":[],"notices":[],"unpushed":[]}"#,
        "",
    );
    harness
}

#[test]
fn space_stages_an_unstaged_row_and_unstages_a_staged_one() {
    let mut harness = staged_harness();
    harness.app.select_oid(&herdr_damnit_domain::Oid::new("1a2b3c4"));
    harness.press(KeyCode::Char(' '));
    assert_eq!(harness.last(), "add 1a2b3c4 --json");

    harness.app.select_oid(&herdr_damnit_domain::Oid::new("5d6e7f8"));
    harness.press(KeyCode::Char(' '));
    assert_eq!(harness.last(), "reset 5d6e7f8 --json");
}

#[test]
fn space_on_a_heading_does_nothing_and_says_nothing() {
    let mut harness = staged_harness();
    harness.app.select_row(0);
    let before = harness.lines().len();
    harness.press(KeyCode::Char(' '));

    assert_eq!(harness.lines().len(), before);
    assert!(harness.app.message.is_empty());
}

#[test]
fn shift_a_stages_everything_and_shift_u_unstages_everything() {
    let mut harness = staged_harness();
    harness.press(KeyCode::Char('A'));
    assert_eq!(harness.last(), "add -A --json");
    harness.press(KeyCode::Char('U'));
    assert_eq!(harness.last(), "reset --json");
}

#[test]
fn c_opens_the_commit_box_headed_with_the_count_dam_reported() {
    let mut harness = staged_harness();
    harness.press(KeyCode::Char('c'));

    let Some(Overlay::Line(box_)) = harness.app.overlay.as_ref() else {
        panic!("expected the commit box");
    };
    assert_eq!(box_.title, "commit 1 staged change");
    assert_eq!(box_.purpose, LinePurpose::Commit);
}

#[test]
fn a_commit_sends_the_line_as_one_argument() {
    let mut harness = staged_harness();
    harness.press(KeyCode::Char('c'));
    for character in "refresh the roster row".chars() {
        harness.press(KeyCode::Char(character));
    }
    harness.press(KeyCode::Enter);

    assert_eq!(harness.last(), "commit -m refresh the roster row --json");
    assert!(harness.app.overlay.is_none());
}

#[test]
fn a_blank_commit_message_is_refused_in_the_pane_and_nothing_is_spawned() {
    let mut harness = staged_harness();
    harness.press(KeyCode::Char('c'));
    harness.press(KeyCode::Char(' '));
    let before = harness.lines().len();
    harness.press(KeyCode::Enter);

    assert_eq!(harness.app.message, "a commit needs a message.");
    assert_eq!(harness.lines().len(), before);
    assert!(harness.app.overlay.is_some(), "the box closed on a refusal");
}

#[test]
fn escape_closes_the_commit_box_and_spawns_nothing() {
    let mut harness = staged_harness();
    harness.press(KeyCode::Char('c'));
    let before = harness.lines().len();
    harness.press(KeyCode::Esc);

    assert!(harness.app.overlay.is_none());
    assert_eq!(harness.lines().len(), before);
}

#[test]
fn c_with_nothing_staged_says_what_to_press_and_opens_no_box() {
    let mut harness = super::super::screens::tests::loaded();
    harness.press(KeyCode::Char('c'));

    assert_eq!(
        harness.app.message,
        "nothing staged; press <Space> on a row or A to stage everything."
    );
    assert!(harness.app.overlay.is_none());
}

#[test]
fn backspace_takes_a_character_off_the_line() {
    let mut harness = staged_harness();
    harness.press(KeyCode::Char('c'));
    harness.press(KeyCode::Char('a'));
    harness.press(KeyCode::Char('b'));
    harness.press(KeyCode::Backspace);
    harness.press(KeyCode::Enter);

    assert_eq!(harness.last(), "commit -m a --json");
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p herdr-damnit --locked keys_tests`
Expected: FAIL with `no variant or associated item named 'Line'`

- [ ] **Step 3: Write the box and the keys**

`overlay/line.rs` holds `LineBox` and `LinePurpose` with `push`, `backspace` and `text`.
`App::key` gains the six arms above; the overlay branch routes a character, a backspace, `Enter` and
`Esc` into the open box. `screens::draw` renders an open `LineBox` as a bordered one-line box over the
body with its title and its hint.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p herdr-damnit --locked`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add -A
SKIP_AI_COMMIT=1 git commit -m "feat(pane): stage, unstage and commit from the pane"
```

---

### Task 33: Push, pull, the refusal and cancellation

**Files:**
- Modify: `crates/herdr-damnit/src/app.rs`, `crates/herdr-damnit/src/app/keys_tests.rs`
- Create: `crates/herdr-damnit/tests/non_blocking.rs`

**Interfaces:**
- Consumes: `Jobs::exclusive`, `cancel_current` from Task 17; the fake `dam` from Task 20; the
  `ProcessDamRunner` from Task 21.
- Produces: `P`, `L` and `<C-c>` on `App`, and the three integration tests the spec names.

| Key | Given | Then |
|---|---|---|
| `P` | no exclusive job in flight | `dam push --json` starts on a worker thread; the header shows a spinner and the elapsed time |
| `P` | a push or pull is already running | the status line says `a push is already running; <C-c> cancels it.` and nothing is spawned |
| `L` | no exclusive job in flight | `dam pull --json` starts the same way |
| `<C-c>` | a dam job is in flight | the job the header names is cancelled |

A push or a pull is treated as successful whenever `dam` exits 0: the counts go in the status line,
rebuilt from the JSON, and any failures are in the Notices section where they persist until they are
dealt with. A network outage never produces an error banner that disappears on the next key press.

**The one deviation from the spec named at the top of this phase applies here.** The fake sleeps
600 ms rather than three seconds and the loop is driven for 800 ms, so the test stays under one
second. The render threshold scales with it: at the 50 ms in-flight window, 800 ms is sixteen frames,
and the assertion is at least eight, halved for slack the way the spec halves its own.

- [ ] **Step 1: Write the failing key tests**

Add to `crates/herdr-damnit/src/app/keys_tests.rs`:

```rust
#[test]
fn p_pushes_and_l_pulls() {
    let mut harness = harness();
    harness.press(KeyCode::Char('P'));
    assert_eq!(harness.last(), "push --json");
    harness.answer(0, 0, r#"{"remotes":[]}"#, "");

    harness.press(KeyCode::Char('L'));
    assert!(harness.last().starts_with("pull --json"), "{}", harness.last());
}

#[test]
fn a_second_push_is_refused_with_a_sentence_and_never_spawned() {
    let mut harness = harness();
    harness.press(KeyCode::Char('P'));
    let before = harness.lines().len();
    harness.press(KeyCode::Char('P'));

    assert_eq!(harness.app.message, "a push is already running; <C-c> cancels it.");
    assert_eq!(harness.lines().len(), before);
}

#[test]
fn a_finished_push_reports_dams_own_counts_rebuilt_from_json() {
    let mut harness = harness();
    harness.press(KeyCode::Char('P'));
    harness.answer(
        0,
        0,
        r#"{"remotes":[{"remote":"example","sent":3,"succeeded":3,"skipped":0,"failed":[]}]}"#,
        "",
    );

    assert_eq!(harness.app.message, "example: 3 sent, 3 ok, 0 failed, 0 skipped");
}

#[test]
fn a_push_with_a_failed_mutation_is_still_a_success_and_the_failure_is_a_notice() {
    let mut harness = harness();
    harness.press(KeyCode::Char('P'));
    harness.answer(
        0,
        0,
        r#"{"remotes":[{"remote":"example","sent":3,"succeeded":2,"skipped":0,
          "failed":[{"oid":"9a0b1c2","why":"the service refused the write"}]}]}"#,
        "",
    );

    assert_eq!(harness.app.message, "example: 3 sent, 2 ok, 1 failed, 0 skipped");
}

#[test]
fn a_pull_reports_its_own_counts() {
    let mut harness = harness();
    harness.press(KeyCode::Char('L'));
    harness.answer(
        0,
        0,
        r#"{"remotes":[{"remote":"example","created":2,"updated":1,"unchanged":40,
          "conflicts":0,"removed_upstream":0}]}"#,
        "",
    );

    assert_eq!(
        harness.app.message,
        "example: 2 new, 1 updated, 40 unchanged, 0 conflict(s), 0 removed upstream"
    );
}

#[test]
fn a_finished_push_re_reads_the_status_and_the_list() {
    let mut harness = harness();
    harness.press(KeyCode::Char('P'));
    let before = harness.lines().len();
    harness.answer(0, 0, r#"{"remotes":[]}"#, "");

    let asked = &harness.lines()[before..];
    assert!(asked.contains(&"status --json".to_string()), "{asked:?}");
    assert!(asked.contains(&"ls !done --json".to_string()), "{asked:?}");
}

#[test]
fn control_c_cancels_the_job_the_header_names() {
    use crossterm::event::{KeyEvent, KeyModifiers};

    let mut harness = harness();
    harness.press(KeyCode::Char('P'));
    harness
        .app
        .key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));

    harness.answer(0, 3, "", "");
    assert_eq!(harness.app.message, "cancelled");
}

#[test]
fn control_c_with_nothing_in_flight_says_nothing() {
    use crossterm::event::{KeyEvent, KeyModifiers};

    let mut harness = harness();
    harness
        .app
        .key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));

    assert!(harness.app.message.is_empty());
}
```

- [ ] **Step 2: Write the failing non-blocking test**

`crates/herdr-damnit/tests/non_blocking.rs`. It drives the real `App` over the real
`ProcessDamRunner` against the fake `dam`, counting the frames `screens::render_to_text` produced.
The binary crate exposes `app`, `screens` and `open` behind a `pub mod` for this, declared in a
`src/lib.rs` the binary's `main.rs` also uses.

```rust
//! The test that proves the loop renders and reads keys while a push runs. A loop that awaited the
//! push inline renders once and fails on the first clause.

use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent};
use herdr_damnit::app::App;
use herdr_damnit::screens::render_to_text;
use herdr_damnit_adapters::{Config, ProcessDamRunner};
use herdr_damnit_application::Jobs;

/// 600 ms rather than the spec's three seconds, so the test stays inside this repository's
/// one-second budget. The window the loop polls on is unchanged, so the frame count scales with it.
const SLEEP_MS: &str = "600";
const DRIVE: Duration = Duration::from_millis(800);

fn app() -> App {
    let fixtures = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../herdr-damnit-adapters/tests/fixtures");
    unsafe {
        std::env::set_var("FAKE_DAM_FIXTURE_DIR", &fixtures);
        std::env::set_var("FAKE_DAM_FIXTURE", "push-ok");
        std::env::set_var("FAKE_DAM_SLEEP_MS", SLEEP_MS);
    }
    let config = Config::parse("icons = \"ascii\"\n").expect("parses");
    let runner = ProcessDamRunner::new(vec![
        std::env::var("FAKE_DAM_BIN").expect("the fake is on the environment"),
    ]);
    App::new(
        config,
        Jobs::new(Box::new(runner)),
        herdr_damnit_domain::parse_date("2026-09-20").expect("a date"),
    )
}

/// Drive the loop the way `loop_::run` does, without a terminal: render, tick, then wait the poll
/// window. Returns one string per frame.
fn drive(app: &mut App, over: Duration, keys: &[(Duration, KeyCode)]) -> Vec<String> {
    let started = Instant::now();
    let mut frames = Vec::new();
    let mut pending: Vec<(Duration, KeyCode)> = keys.to_vec();
    while started.elapsed() < over {
        frames.push(render_to_text(app, 32, 8));
        app.tick(Instant::now());
        let due: Vec<KeyCode> = pending
            .iter()
            .filter(|(at, _)| started.elapsed() >= *at)
            .map(|(_, key)| *key)
            .collect();
        pending.retain(|(at, _)| started.elapsed() < *at);
        for key in due {
            app.key(KeyEvent::from(key));
        }
        std::thread::sleep(app.poll_window());
    }
    frames
}

#[test]
fn the_pane_renders_while_a_push_runs() {
    let mut app = app();
    app.key(KeyEvent::from(KeyCode::Char('P')));

    let frames = drive(&mut app, DRIVE, &[]);

    assert!(frames.len() >= 8, "it rendered {} frames", frames.len());
    let mid = &frames[frames.len() / 2];
    assert!(mid.contains("push"), "{mid}");
    let last = frames.last().expect("a frame");
    assert!(last.contains("3 sent, 3 ok"), "{last}");
    assert!(!last.contains("push 0."), "the spinner outlived the job: {last}");
}

#[test]
fn a_key_pressed_mid_push_moves_the_cursor_on_the_very_next_frame() {
    let mut app = app();
    app.key(KeyEvent::from(KeyCode::Char('P')));
    let frames = drive(
        &mut app,
        DRIVE,
        &[(Duration::from_millis(200), KeyCode::Tab)],
    );

    assert!(
        frames.iter().any(|frame| frame.contains("status")),
        "the tab never took effect while the push ran"
    );
}

#[test]
fn a_second_push_mid_push_is_refused_and_the_fake_records_exactly_one() {
    let log = std::env::temp_dir().join(format!("herdr-damnit-nb-{}", std::process::id()));
    let _ = std::fs::remove_file(&log);
    unsafe { std::env::set_var("FAKE_DAM_LOG", &log) };

    let mut app = app();
    app.key(KeyEvent::from(KeyCode::Char('P')));
    drive(&mut app, DRIVE, &[(Duration::from_millis(200), KeyCode::Char('P'))]);

    let pushes = std::fs::read_to_string(&log)
        .expect("a log")
        .lines()
        .filter(|line| line.contains("\"push\""))
        .count();
    assert_eq!(pushes, 1, "the second push reached dam");
    unsafe { std::env::remove_var("FAKE_DAM_LOG") };
}
```

`FAKE_DAM_BIN` is set by a `build.rs` in the binary crate that writes
`cargo::rustc-env=FAKE_DAM_BIN=...`, or, more simply, by declaring the fake as a dev-dependency-free
path: add `herdr-damnit-adapters` to `[dev-dependencies]` with the same path and read
`env!("CARGO_BIN_EXE_fake-dam")` from a helper the adapters crate exports for tests:

```rust
// crates/herdr-damnit-adapters/src/lib.rs
/// The fake `dam` this workspace builds, for a test in another crate that needs one.
pub fn fake_dam_path() -> &'static str {
    env!("CARGO_BIN_EXE_fake-dam")
}
```

Use that and drop `FAKE_DAM_BIN` from the test.

- [ ] **Step 3: Run the tests to verify they fail**

Run: `cargo test -p herdr-damnit --locked`
Expected: FAIL, `P` does nothing and `herdr_damnit::app` is not a library path.

- [ ] **Step 4: Write the keys and the library split**

Add `crates/herdr-damnit/src/lib.rs` re-exporting the modules the integration test needs
(`pub mod app; pub mod open; pub mod screens; pub mod theme; pub mod markdown; mod overlay;`), and
have `main.rs` `use herdr_damnit::...` rather than declaring those modules itself. `Cargo.toml`
gains a `[lib] name = "herdr_damnit"` beside the existing `[[bin]]`.

`App::key` gains:

```rust
            KeyCode::Char('P') => self.submit(JobKind::Exclusive(SyncKind::Push), argv::push()),
            KeyCode::Char('L') => self.submit(JobKind::Exclusive(SyncKind::Pull), argv::pull()),
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.jobs.cancel_current();
            }
```

with the `<C-c>` arm placed above the plain `c` arm so the modifier wins.

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test -p herdr-damnit --locked -- --test-threads=1`
Expected: PASS, eleven new tests.

- [ ] **Step 6: Commit**

```bash
git add -A
SKIP_AI_COMMIT=1 git commit -m "feat(pane): push and pull without freezing the pane"
```

---

### Task 34: Complete, force-complete and delete

**Files:**
- Create: `crates/herdr-damnit/src/overlay/confirm.rs`
- Modify: `crates/herdr-damnit/src/app.rs`, `crates/herdr-damnit/src/overlay.rs`,
  `crates/herdr-damnit/src/screens.rs`, `crates/herdr-damnit/src/app/keys_tests.rs`

**Interfaces:**
- Produces:

```rust
pub struct Confirm { pub question: String, pub key: char, pub purpose: ConfirmPurpose }
pub enum ConfirmPurpose { Delete(Oid), Discard(Oid), QuitMidJob }
```

and these keys:

| Key | Given | Then |
|---|---|---|
| `x` | a task row | `dam done <oid>` runs; a refusal lists the blockers in the status line and changes nothing |
| `X` | a task row | `dam done <oid> --force` runs, completing past open children and dependencies |
| `dd` | a row naming an oid, first `d` | a confirm names the object |
| `dd` | the confirm is up, second `d` | `dam rm <oid>` runs, removing it from the working layer |
| `dd` | the confirm is up, any other key | the confirm is dismissed and nothing is sent |

`X` is force-complete rather than reopen, because `dam` version one has no verb that clears `done`.
It is the only key whose meaning changed from `herdr-todoist`, so it gets a line in the release
notes.

- [ ] **Step 1: Write the failing tests**

Add to `crates/herdr-damnit/src/app/keys_tests.rs`:

```rust
#[test]
fn x_completes_and_shift_x_forces_it() {
    let mut harness = super::super::screens::tests::loaded();
    harness.app.select_oid(&herdr_damnit_domain::Oid::new("1a2b3c4"));

    harness.press(KeyCode::Char('x'));
    assert_eq!(harness.last(), "done 1a2b3c4 --json");
    harness.press(KeyCode::Char('X'));
    assert_eq!(harness.last(), "done 1a2b3c4 --force --json");
}

#[test]
fn a_refused_completion_lists_dams_own_blockers_and_changes_nothing() {
    let mut harness = super::super::screens::tests::loaded();
    let before = harness.app.list.object_count();
    harness.press(KeyCode::Char('x'));
    harness.answer(
        2,
        2,
        "",
        "dam: 5d6e7f8 has 2 open children; use --force to complete them too",
    );

    assert_eq!(
        harness.app.message,
        "5d6e7f8 has 2 open children; use --force to complete them too"
    );
    assert_eq!(harness.app.list.object_count(), before);
}

#[test]
fn the_first_d_asks_and_the_second_removes() {
    let mut harness = super::super::screens::tests::loaded();
    harness.app.select_oid(&herdr_damnit_domain::Oid::new("1a2b3c4"));

    let before = harness.lines().len();
    harness.press(KeyCode::Char('d'));
    assert_eq!(harness.lines().len(), before, "the first d sent something");
    let Some(Overlay::Confirm(confirm)) = harness.app.overlay.as_ref() else {
        panic!("expected a confirm");
    };
    assert!(confirm.question.contains("ship the pin bump"), "{}", confirm.question);

    harness.press(KeyCode::Char('d'));
    assert_eq!(harness.last(), "rm 1a2b3c4 --json");
    assert!(harness.app.overlay.is_none());
}

#[test]
fn any_other_key_dismisses_the_delete_confirm_and_sends_nothing() {
    let mut harness = super::super::screens::tests::loaded();
    harness.press(KeyCode::Char('d'));
    let before = harness.lines().len();
    harness.press(KeyCode::Char('j'));

    assert!(harness.app.overlay.is_none());
    assert_eq!(harness.lines().len(), before);
}

#[test]
fn x_on_a_heading_does_nothing_and_says_nothing() {
    let mut harness = super::super::screens::tests::loaded();
    harness.app.select_row(0);
    let before = harness.lines().len();
    harness.press(KeyCode::Char('x'));

    assert_eq!(harness.lines().len(), before);
    assert!(harness.app.message.is_empty());
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p herdr-damnit --locked keys_tests`
Expected: FAIL with `no variant named 'Confirm'`

- [ ] **Step 3: Write the confirm and the keys**

`overlay/confirm.rs` holds `Confirm` and `ConfirmPurpose`. The overlay branch of `App::key` matches
the confirm's own `key` character and acts, and treats every other key as a dismissal.
`screens::draw` renders an open confirm as a bordered two-line box with the question and the key.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p herdr-damnit --locked -- --test-threads=1`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add -A
SKIP_AI_COMMIT=1 git commit -m "feat(pane): complete, force-complete and remove an object"
```

---

### Task 35: Priority, due and deadline

**Files:**
- Modify: `crates/herdr-damnit/src/app.rs`, `crates/herdr-damnit/src/app/keys_tests.rs`

**Interfaces:**
- Consumes: `Priority::next` from Task 4; `set_priority`, `set_due`, `set_deadline` from Task 16;
  `LineBox` from Task 32.
- Produces these keys:

| Key | Given | Then |
|---|---|---|
| `p` | a task row | `dam edit <oid> -p <next>` runs, where next cycles 4, 3, 2, 1 and back to 4 |
| `s` | a task row | a one-line box opens, hinting `today, tomorrow, YYYY-MM-DD or YYYY-MM-DDTHH:MM` |
| `s` | the box holds a line, `<CR>` | `dam edit <oid> --due <line>` runs; a line `dam` cannot read comes back as `dam`'s own message with the line still in the box |
| `s` | the box is empty, `<CR>` | `dam edit <oid> --no-due` runs, clearing the due date |
| `D` | a task row | the same box for `--deadline`, which `dam` keeps distinct from `due` |

The hint names exactly the words `dam` accepts today. Richer date words are item 8 of the spec's
Needed from dam; when `dam` grows them the hint is the one line that changes here.

- [ ] **Step 1: Write the failing tests**

Add to `crates/herdr-damnit/src/app/keys_tests.rs`:

```rust
#[test]
fn p_cycles_down_the_priorities_and_wraps_at_one() {
    let mut harness = super::super::screens::tests::loaded();
    // 1a2b3c4 is priority 1 and 5d6e7f8 is the default 4.
    harness.app.select_oid(&herdr_damnit_domain::Oid::new("1a2b3c4"));
    harness.press(KeyCode::Char('p'));
    assert_eq!(harness.last(), "edit 1a2b3c4 -p 4 --json");

    harness.app.select_oid(&herdr_damnit_domain::Oid::new("5d6e7f8"));
    harness.press(KeyCode::Char('p'));
    assert_eq!(harness.last(), "edit 5d6e7f8 -p 3 --json");
}

#[test]
fn s_opens_a_box_hinting_exactly_the_words_dam_accepts() {
    let mut harness = super::super::screens::tests::loaded();
    harness.press(KeyCode::Char('s'));

    let Some(Overlay::Line(box_)) = harness.app.overlay.as_ref() else {
        panic!("expected the date box");
    };
    assert_eq!(box_.hint, "today, tomorrow, YYYY-MM-DD or YYYY-MM-DDTHH:MM");
}

#[test]
fn a_date_typed_into_the_box_becomes_the_due_flag() {
    let mut harness = super::super::screens::tests::loaded();
    harness.app.select_oid(&herdr_damnit_domain::Oid::new("1a2b3c4"));
    harness.press(KeyCode::Char('s'));
    for character in "tomorrow".chars() {
        harness.press(KeyCode::Char(character));
    }
    harness.press(KeyCode::Enter);

    assert_eq!(harness.last(), "edit 1a2b3c4 --due tomorrow --json");
}

#[test]
fn an_empty_date_box_clears_the_date_rather_than_sending_an_empty_one() {
    let mut harness = super::super::screens::tests::loaded();
    harness.app.select_oid(&herdr_damnit_domain::Oid::new("1a2b3c4"));
    harness.press(KeyCode::Char('s'));
    harness.press(KeyCode::Enter);

    assert_eq!(harness.last(), "edit 1a2b3c4 --no-due --json");
}

#[test]
fn shift_d_is_the_same_box_aimed_at_the_deadline() {
    let mut harness = super::super::screens::tests::loaded();
    harness.app.select_oid(&herdr_damnit_domain::Oid::new("1a2b3c4"));
    harness.press(KeyCode::Char('D'));
    for character in "2026-10-02".chars() {
        harness.press(KeyCode::Char(character));
    }
    harness.press(KeyCode::Enter);

    assert_eq!(harness.last(), "edit 1a2b3c4 --deadline 2026-10-02 --json");
}

#[test]
fn a_date_dam_cannot_read_comes_back_with_the_line_still_in_the_box() {
    let mut harness = super::super::screens::tests::loaded();
    harness.app.select_oid(&herdr_damnit_domain::Oid::new("1a2b3c4"));
    harness.press(KeyCode::Char('s'));
    for character in "next mon".chars() {
        harness.press(KeyCode::Char(character));
    }
    harness.press(KeyCode::Enter);
    harness.answer(
        2,
        2,
        "",
        "dam: a date is today, tomorrow, YYYY-MM-DD or YYYY-MM-DDTHH:MM",
    );

    assert_eq!(
        harness.app.message,
        "a date is today, tomorrow, YYYY-MM-DD or YYYY-MM-DDTHH:MM"
    );
    let Some(Overlay::Line(box_)) = harness.app.overlay.as_ref() else {
        panic!("the box closed on a refusal");
    };
    assert_eq!(box_.text(), "next mon");
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p herdr-damnit --locked keys_tests`
Expected: FAIL, `p` does nothing.

- [ ] **Step 3: Write the keys**

`App::key` gains `p`, `s` and `D`. The `LinePurpose::Due(oid)` and `LinePurpose::Deadline(oid)` arms
of the overlay branch build their argv with `set_due` and `set_deadline`, passing `None` when the
line is empty.

A refused write leaves the overlay open with its text, which is what `Failure::leaves_model_untouched`
already answers: `App::apply` keeps the overlay when it is true and closes it when it is not, so a
store error clears a half-typed line rather than trapping the operator in it.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p herdr-damnit --locked -- --test-threads=1`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add -A
SKIP_AI_COMMIT=1 git commit -m "feat(pane): cycle a priority and set a due date or a deadline"
```

---

### Task 36: The label picker, the path picker and add

**Files:**
- Modify: `crates/herdr-damnit/src/app.rs`, `crates/herdr-damnit/src/overlay/picker.rs`,
  `crates/herdr-damnit/src/app/keys_tests.rs`

**Interfaces:**
- Consumes: `Picker` from Task 31; `label`, `unlabel`, `move_to`, `create` from Task 16.
- Produces these keys:

| Key | Given | Then |
|---|---|---|
| `l` | a row naming an oid | a picker of every label in the store opens, the ones this object carries marked |
| `l` | the picker is open, `<CR>` | `dam edit <oid> --label <name>` or `--unlabel <name>` runs for the entry under the cursor, and the picker stays open |
| `m` | a row naming an oid | a picker of every `path` in the store opens, plus one entry per parent prefix, the object's own path under the cursor |
| `m` | the picker is open, `<CR>` | `dam mv <oid> <path>` runs, moving the object and its children |
| `a` | any screen | a one-line box opens for a new subject |
| `a` | the box holds a line, `<CR>` | `dam new "<line>" --path <path of the row under the cursor> --json` runs and the status line names the oid it made |

Neither picker needs a `dam` command of its own: the pane already holds every object of the showing
view, and both read the union of what it holds. A label the object carries that no other object does
is offered too, so it can be taken off.

The label picker is flat in version one: it does not group by category, because reading `dam`'s
category catalogue needs a command that does not exist yet, which is item 6 of the spec's Needed from
dam. A write that puts two values of an exclusive category on one object is refused by `dam` with the
rule named, and the picker stays open, which is annoying and never wrong.

- [ ] **Step 1: Write the failing tests**

Add to `crates/herdr-damnit/src/app/keys_tests.rs`:

```rust
#[test]
fn the_label_picker_offers_every_label_in_the_view_and_marks_the_ones_this_object_carries() {
    let mut harness = super::super::screens::tests::loaded();
    harness.app.select_oid(&herdr_damnit_domain::Oid::new("1a2b3c4"));
    harness.press(KeyCode::Char('l'));

    let Some(Overlay::Label(picker)) = harness.app.overlay.as_ref() else {
        panic!("expected the label picker");
    };
    assert_eq!(
        picker.entries.iter().map(|entry| entry.text.as_str()).collect::<Vec<_>>(),
        vec!["a", "b"]
    );
    assert!(picker.entries.iter().all(|entry| entry.marked));
}

#[test]
fn taking_a_marked_label_off_sends_unlabel_and_adding_one_sends_label() {
    let mut harness = super::super::screens::tests::loaded();
    harness.app.select_oid(&herdr_damnit_domain::Oid::new("1a2b3c4"));
    harness.press(KeyCode::Char('l'));
    harness.press(KeyCode::Enter);
    assert_eq!(harness.last(), "edit 1a2b3c4 --unlabel a --json");

    harness.app.select_oid(&herdr_damnit_domain::Oid::new("5d6e7f8"));
    harness.press(KeyCode::Char('l'));
    harness.press(KeyCode::Enter);
    assert_eq!(harness.last(), "edit 5d6e7f8 --label a --json");
}

#[test]
fn the_label_picker_stays_open_after_a_pick() {
    let mut harness = super::super::screens::tests::loaded();
    harness.press(KeyCode::Char('l'));
    harness.press(KeyCode::Enter);

    assert!(matches!(harness.app.overlay, Some(Overlay::Label(_))));
}

#[test]
fn the_path_picker_offers_every_path_and_every_parent_prefix() {
    let mut harness = super::super::screens::tests::loaded();
    harness.app.select_oid(&herdr_damnit_domain::Oid::new("1a2b3c4"));
    harness.press(KeyCode::Char('m'));

    let Some(Overlay::Path(picker)) = harness.app.overlay.as_ref() else {
        panic!("expected the path picker");
    };
    assert_eq!(
        picker.entries.iter().map(|entry| entry.text.as_str()).collect::<Vec<_>>(),
        vec!["proj", "proj/dotfiles", "proj/home"]
    );
    assert_eq!(picker.selected, 1, "the object's own path is under the cursor");
}

#[test]
fn picking_a_path_moves_the_object_and_closes_the_picker() {
    let mut harness = super::super::screens::tests::loaded();
    harness.app.select_oid(&herdr_damnit_domain::Oid::new("1a2b3c4"));
    harness.press(KeyCode::Char('m'));
    harness.press(KeyCode::Char('j'));
    harness.press(KeyCode::Enter);

    assert_eq!(harness.last(), "mv 1a2b3c4 proj/home --json");
    assert!(harness.app.overlay.is_none());
}

#[test]
fn a_creates_an_object_under_the_path_of_the_row_the_cursor_is_on() {
    let mut harness = super::super::screens::tests::loaded();
    harness.app.select_oid(&herdr_damnit_domain::Oid::new("9a0b1c2"));
    harness.press(KeyCode::Char('a'));
    for character in "file taxes".chars() {
        harness.press(KeyCode::Char(character));
    }
    harness.press(KeyCode::Enter);

    assert_eq!(harness.last(), "new file taxes --path proj/home --json");
}

#[test]
fn a_new_object_is_named_in_the_status_line_by_the_oid_dam_made() {
    let mut harness = super::super::screens::tests::loaded();
    harness.press(KeyCode::Char('a'));
    for character in "file taxes".chars() {
        harness.press(KeyCode::Char(character));
    }
    harness.press(KeyCode::Enter);
    harness.answer(
        2,
        0,
        r#"{"oid":"7a8b9c0","kind":"task","subject":"file taxes","task":{"done":false,"priority":4}}"#,
        "",
    );

    assert_eq!(harness.app.message, "made 7a8b9c0");
}

#[test]
fn an_empty_new_subject_is_refused_in_the_pane_and_nothing_is_spawned() {
    let mut harness = super::super::screens::tests::loaded();
    harness.press(KeyCode::Char('a'));
    let before = harness.lines().len();
    harness.press(KeyCode::Enter);

    assert_eq!(harness.app.message, "a new task needs a subject.");
    assert_eq!(harness.lines().len(), before);
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p herdr-damnit --locked keys_tests`
Expected: FAIL with `no variant named 'Label'`

- [ ] **Step 3: Write the pickers and the keys**

`Overlay::Label` and `Overlay::Path` take a `Picker`. Two builders on `App`:

```rust
    /// Every label any object of the showing view carries, sorted, with the ones this object
    /// carries marked. A label only this object has is offered too, so it can be taken off.
    fn label_picker(&self, oid: &Oid) -> Picker;

    /// Every path any object of the showing view sits under, plus one entry per parent prefix, so
    /// a move to a parent needs no typing.
    fn path_picker(&self, oid: &Oid) -> Picker;
```

`LinePurpose::New(path)` carries the path the box was opened over; its `Enter` arm refuses a blank
subject in the pane and otherwise submits `create`. The `ReadShow`-shaped completion of a `new` is a
`Write` whose stdout is the created object, so `App::apply` reads the `oid` out of it and says
`made <short oid>`.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p herdr-damnit --locked -- --test-threads=1`
Expected: PASS.

- [ ] **Step 5: Check the key file is inside the cap**

Run: `wc -l crates/herdr-damnit/src/app.rs`
Expected: under 500, ideally under 300. The key match is the part that grew; move it into
`crates/herdr-damnit/src/app/keys.rs` as `impl App { pub fn key(...) }` if it is over 300.

- [ ] **Step 6: Commit**

```bash
git add -A
SKIP_AI_COMMIT=1 git commit -m "feat(pane): pick a label, move a path and add an object"
```

---

### Task 37: The editor round trip

**Files:**
- Create: `crates/herdr-damnit/src/editor.rs`
- Modify: `crates/herdr-damnit/src/app.rs`, `crates/herdr-damnit/src/loop_.rs`,
  `crates/herdr-damnit/src/app/keys_tests.rs`

**Interfaces:**
- Consumes: `edit_in_editor` from Task 16; `After::Editor` from Task 26.
- Produces:

```rust
/// Leave the alternate screen, run the argv to completion, enter it again, and re-read.
pub fn round_trip(terminal: &mut ratatui::DefaultTerminal, app: &mut App, argv: &[String]);
```

and `e` on `App`, which returns `After::Editor(argv)` rather than spawning anything itself.

`dam edit <oid> -e` renders the object as a commented TOML template, spawns the editor, parses the
file back on save, and on a parse failure names the problem and reopens with the text intact. Nothing
reaches the working layer until it parses, and `dam` chooses the editor: `VISUAL`, then `EDITOR`, then
`vi`.

This is the one key deliberately not run on a worker thread: it owns the terminal, and an editor drawn
under a pane still drawing is the one thing the non-blocking model must not do. The pane enters the
alternate screen again on every path, including a `dam` that exited non-zero and a `dam` that could
not be started, so neither leaves the terminal in raw mode. The list is re-read whatever it exited
with, since a person who quit in a hurry may still have saved.

- [ ] **Step 1: Write the failing tests**

Add to `crates/herdr-damnit/src/app/keys_tests.rs`:

```rust
#[test]
fn e_hands_the_loop_the_editor_argv_and_spawns_nothing_itself() {
    let mut harness = super::super::screens::tests::loaded();
    harness.app.select_oid(&herdr_damnit_domain::Oid::new("1a2b3c4"));
    let before = harness.lines().len();

    let after = harness.press(KeyCode::Char('e'));

    assert_eq!(
        after,
        After::Editor(vec![
            "edit".to_string(),
            "1a2b3c4".to_string(),
            "-e".to_string()
        ])
    );
    assert_eq!(harness.lines().len(), before, "the key spawned dam itself");
}

#[test]
fn e_on_a_heading_stays_put() {
    let mut harness = super::super::screens::tests::loaded();
    harness.app.select_row(0);
    assert_eq!(harness.press(KeyCode::Char('e')), After::Stay);
}

#[test]
fn the_header_says_editing_while_the_editor_is_out() {
    let mut harness = super::super::screens::tests::loaded();
    harness.app.editing = true;

    assert!(
        harness.app.header(std::time::Instant::now()).contains("editing"),
        "{}",
        harness.app.header(std::time::Instant::now())
    );
}
```

And a test of the round trip itself, in `crates/herdr-damnit/tests/editor.rs`, which drives the real
spawn against a script that succeeds and one that does not exist:

```rust
//! The editor round trip: the pane comes back to a drawn screen and a fresh read whatever the
//! editor did.

use herdr_damnit::app::App;
use herdr_damnit_adapters::Config;
use herdr_damnit_application::Jobs;

fn app(dam: Vec<String>) -> App {
    let config = Config::parse("icons = \"ascii\"\n").expect("parses");
    App::new(
        config,
        Jobs::new(Box::new(herdr_damnit_adapters::ProcessDamRunner::new(dam))),
        herdr_damnit_domain::parse_date("2026-09-20").expect("a date"),
    )
}

#[test]
fn a_finished_editor_leaves_the_pane_reading_again() {
    let mut app = app(vec![herdr_damnit_adapters::fake_dam_path().to_string()]);
    herdr_damnit::editor::run_and_reread(&mut app, &["edit".to_string(), "x".to_string(), "-e".to_string()]);

    assert!(app.pending_reads() > 0, "the pane did not re-read after the editor");
}

#[test]
fn an_editor_that_could_not_be_started_still_leaves_the_pane_reading_again() {
    let mut app = app(vec!["no-such-dam-anywhere".to_string()]);
    herdr_damnit::editor::run_and_reread(&mut app, &["edit".to_string(), "x".to_string(), "-e".to_string()]);

    assert_eq!(
        app.message,
        "dam is not on PATH; install it with cargo install damnit, then press R."
    );
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p herdr-damnit --locked editor`
Expected: FAIL with `unresolved import 'herdr_damnit::editor'`

- [ ] **Step 3: Write the editor**

`crates/herdr-damnit/src/editor.rs` splits in two: `run_and_reread`, which spawns the argv with
inherited standard input, output and error, waits, and re-reads whatever it exited with; and
`round_trip`, which is `run_and_reread` wrapped in the terminal's own leave and enter.

```rust
//! `e`: `dam edit <oid> -e`. `dam` owns the template, the editor choice and the reopen-on-parse
//! failure loop. The pane's whole job is to give up the terminal and take it back.

use std::process::Command;

use herdr_damnit_domain::{Failure, message};

use crate::app::App;

/// Spawn the editor command and wait for it, then re-read. Terminal-free, so a test drives it.
pub fn run_and_reread(app: &mut App, argv: &[String]) {
    let dam = app.config.dam.clone();
    let Some((binary, leading)) = dam.split_first() else {
        app.message = message(&Failure::NotInstalled);
        return;
    };
    match Command::new(binary).args(leading).args(argv).status() {
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            app.message = message(&Failure::NotInstalled);
        }
        Err(error) => app.message = error.to_string(),
    }
    app.reread();
}

/// The same thing with the alternate screen given up and taken back, on every path.
pub fn round_trip(terminal: &mut ratatui::DefaultTerminal, app: &mut App, argv: &[String]) {
    app.editing = true;
    ratatui::restore();
    run_and_reread(app, argv);
    *terminal = ratatui::init();
    app.editing = false;
}
```

`App` gains `editing: bool`, which `header` reports as `editing`, and `pending_reads()`, which is
`jobs.in_flight()` counted over the read kinds, for the test above.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p herdr-damnit --locked -- --test-threads=1`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add -A
SKIP_AI_COMMIT=1 git commit -m "feat(pane): hand the terminal to dam edit and take it back"
```

---

### Task 38: Send to the agent

**Files:**
- Create: `crates/herdr-damnit/src/overlay/note.rs`
- Modify: `crates/herdr-damnit/src/app.rs`, `crates/herdr-damnit/src/overlay.rs`,
  `crates/herdr-damnit/src/screens.rs`, `crates/herdr-damnit/src/app/keys_tests.rs`

**Interfaces:**
- Consumes: `hand_off`, `HandOff`, `Workspace` from Task 19; `Herdr` from Task 15.
- Produces:

```rust
pub struct NoteBox { pub oid: Oid, pub lines: Vec<String>, pub cursor: usize }

impl NoteBox {
    pub fn push(&mut self, character: char);
    pub fn newline(&mut self);
    pub fn backspace(&mut self);
    pub fn text(&self) -> String;
}
```

and these keys:

| Key | Given | Then |
|---|---|---|
| `S` | a row naming an oid | a multi-line note box opens over the body |
| `S` | the note box is open, `<C-d>` | the brief is pasted into the workspace's agent pane and the status line names the agent; a blank box sends the brief with no note |
| `S` | the note box is open, `<Esc>` | the box closes and nothing is sent |

In the note box `<CR>` opens a line and `<C-d>` sends, which is why sending has its own key.

The record of the hand-off is the configured label: a successful send submits
`dam edit <oid> --label <handoff_label>` as an ordinary write, which shows up under Working on the
Status screen. A refused label says `sent to <name>, label refused: <message>.` and does not pretend
the send failed, because the agent has the work either way. With `handoff_label` empty, nothing is
written and the status line is the whole record.

`App` takes the `Herdr` port in its constructor, so the tests supply their own. Update the harness in
`app/tests.rs` to build one with a recording fake and add `harness_with_herdr`.

- [ ] **Step 1: Write the failing tests**

Add to `crates/herdr-damnit/src/app/keys_tests.rs`:

```rust
#[test]
fn shift_s_opens_the_note_box_over_the_object_under_the_cursor() {
    let mut harness = super::super::screens::tests::loaded();
    harness.app.select_oid(&herdr_damnit_domain::Oid::new("1a2b3c4"));
    harness.press(KeyCode::Char('S'));

    let Some(Overlay::Note(note)) = harness.app.overlay.as_ref() else {
        panic!("expected the note box");
    };
    assert_eq!(note.oid, herdr_damnit_domain::Oid::new("1a2b3c4"));
}

#[test]
fn enter_opens_a_line_in_the_note_box_rather_than_sending() {
    let mut harness = super::super::screens::tests::loaded();
    harness.press(KeyCode::Char('S'));
    harness.press(KeyCode::Char('a'));
    harness.press(KeyCode::Enter);
    harness.press(KeyCode::Char('b'));

    let Some(Overlay::Note(note)) = harness.app.overlay.as_ref() else {
        panic!("the note box closed on Enter");
    };
    assert_eq!(note.text(), "a\nb");
    assert!(harness.herdr_calls().is_empty(), "it sent on Enter");
}

#[test]
fn control_d_sends_the_brief_and_names_the_agent() {
    use crossterm::event::{KeyEvent, KeyModifiers};

    let mut harness = super::super::screens::tests::loaded();
    harness.app.select_oid(&herdr_damnit_domain::Oid::new("1a2b3c4"));
    harness.press(KeyCode::Char('S'));
    for character in "start here".chars() {
        harness.press(KeyCode::Char(character));
    }
    harness
        .app
        .key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL));

    let sent = harness.herdr_calls();
    assert!(sent.iter().any(|call| call.starts_with("pane send-text")), "{sent:?}");
    assert!(harness.app.message.starts_with("sent to planner"), "{}", harness.app.message);
    assert!(harness.app.overlay.is_none());
}

#[test]
fn a_successful_send_writes_the_configured_handoff_label() {
    use crossterm::event::{KeyEvent, KeyModifiers};

    let mut harness = super::super::screens::tests::loaded();
    harness.app.select_oid(&herdr_damnit_domain::Oid::new("1a2b3c4"));
    harness.press(KeyCode::Char('S'));
    harness
        .app
        .key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL));

    assert_eq!(harness.last(), "edit 1a2b3c4 --label handed-off --json");
}

#[test]
fn a_refused_label_does_not_pretend_the_send_failed() {
    use crossterm::event::{KeyEvent, KeyModifiers};

    let mut harness = super::super::screens::tests::loaded();
    harness.press(KeyCode::Char('S'));
    harness
        .app
        .key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL));
    harness.answer(2, 2, "", "dam: \"handed-off\" clashes with \"handed-back\"");

    assert_eq!(
        harness.app.message,
        "sent to planner, label refused: \"handed-off\" clashes with \"handed-back\"."
    );
}

#[test]
fn an_empty_handoff_label_writes_nothing_and_the_status_line_is_the_whole_record() {
    use crossterm::event::{KeyEvent, KeyModifiers};

    let config = herdr_damnit_adapters::Config::parse(
        "icons = \"ascii\"\nhandoff_label = \"\"\n",
    )
    .expect("parses");
    let mut harness = super::super::screens::tests::loaded_with(config);
    harness.press(KeyCode::Char('S'));
    let before = harness.lines().len();
    harness
        .app
        .key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL));

    assert_eq!(harness.lines().len(), before);
    assert_eq!(harness.app.message, "sent to planner");
}

#[test]
fn escape_throws_the_draft_away_and_sends_nothing() {
    let mut harness = super::super::screens::tests::loaded();
    harness.press(KeyCode::Char('S'));
    harness.press(KeyCode::Char('x'));
    harness.press(KeyCode::Esc);

    assert!(harness.app.overlay.is_none());
    assert!(harness.herdr_calls().is_empty());
}

#[test]
fn a_workspace_with_no_agent_pane_says_so() {
    use crossterm::event::{KeyEvent, KeyModifiers};

    let mut harness = super::super::screens::tests::loaded_without_agent();
    harness.press(KeyCode::Char('S'));
    harness
        .app
        .key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL));

    assert_eq!(harness.app.message, "no agent pane in this workspace.");
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p herdr-damnit --locked keys_tests`
Expected: FAIL with `no variant named 'Note'`

- [ ] **Step 3: Write the note box and the key**

`overlay/note.rs` holds `NoteBox`. `App` gains `herdr: Box<dyn Herdr>` and `here: Workspace`, both
supplied by the composition root from `CliHerdr` and the two environment variables herdr sets.
`S` opens the box; `<C-d>` calls `hand_off` and, on `HandOff::Sent`, puts `sent to <name>` in the
status line and submits the label write when there is one, recording the agent's name so a refusal of
that write can name it. `screens::draw` renders an open `NoteBox` as a bordered multi-line box with
the hint `<C-d> sends, <Esc> cancels`.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p herdr-damnit --locked -- --test-threads=1`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add -A
SKIP_AI_COMMIT=1 git commit -m "feat(pane): hand a task to the agent pane and record it as a label"
```

---

### Task 39: Resolve, quit and Esc

**Files:**
- Modify: `crates/herdr-damnit/src/app.rs`, `crates/herdr-damnit/src/app/keys_tests.rs`

**Interfaces:**
- Consumes: `resolve`, `Side` from Task 16; `Confirm`, `ConfirmPurpose::QuitMidJob` from Task 34.
- Produces these keys:

| Key | Given | Then |
|---|---|---|
| `o` | a conflict row on Status | `dam resolve <oid> --ours` runs |
| `t` | a conflict row on Status | `dam resolve <oid> --theirs` runs |
| `q` | no exclusive job in flight | the pane closes |
| `q` | a push, pull or commit in flight, pressed once | a confirm says `a push is running; q again quits and lets it finish` |
| `q` | that confirm is up, `q` again | the pane closes and the `dam` child is left to finish |
| `<Esc>` | Detail, a picker or a box is open | that overlay closes and the screen under it is drawn |
| `<Esc>` | nothing is open | the pane closes |

Quitting mid-push detaches rather than killing. `dam push` commits its ledger per mutation as results
arrive, so killing it halfway leaves some mutations sent and recorded and others sent and unrecorded,
which is the one state that makes the next push send a duplicate. Letting it finish is strictly safer
and costs the operator nothing; `<C-c>` is what stops it when they actually want it stopped.

A read job, a write job and the editor are not worth that ceremony and are left on the way out.

- [ ] **Step 1: Write the failing tests**

Add to `crates/herdr-damnit/src/app/keys_tests.rs`:

```rust
fn conflicted() -> super::tests::Harness {
    let mut harness = harness_with(three_views());
    harness.app.submit(
        herdr_damnit_application::JobKind::ReadStatus,
        herdr_damnit_application::argv::status(),
    );
    harness.answer(
        0,
        0,
        r#"{"staged":[],"unstaged":[],"unpushed":[],
          "conflicts":[{"oid":"3d4e5f6","remote":"example",
            "ours":{"oid":"3d4e5f6","kind":"task","subject":"mine"},
            "theirs":{"oid":"3d4e5f6","kind":"task","subject":"theirs"}}],
          "notices":[]}"#,
        "",
    );
    harness.app.screen = Screen::Status;
    harness.app.select_oid(&herdr_damnit_domain::Oid::new("3d4e5f6"));
    harness
}

#[test]
fn o_and_t_resolve_the_conflict_under_the_cursor_toward_one_side() {
    let mut harness = conflicted();
    harness.press(KeyCode::Char('o'));
    assert_eq!(harness.last(), "resolve 3d4e5f6 --ours --json");

    harness.press(KeyCode::Char('t'));
    assert_eq!(harness.last(), "resolve 3d4e5f6 --theirs --json");
}

#[test]
fn o_on_a_row_that_is_not_a_conflict_does_nothing() {
    let mut harness = super::super::screens::tests::loaded();
    let before = harness.lines().len();
    harness.press(KeyCode::Char('o'));

    assert_eq!(harness.lines().len(), before);
    assert!(harness.app.message.is_empty());
}

#[test]
fn q_closes_the_pane_when_nothing_exclusive_is_running() {
    let mut harness = harness();
    assert_eq!(harness.press(KeyCode::Char('q')), After::Quit);
}

#[test]
fn q_mid_push_asks_once_and_quits_on_the_second_press() {
    let mut harness = harness();
    harness.press(KeyCode::Char('P'));

    assert_eq!(harness.press(KeyCode::Char('q')), After::Stay);
    let Some(Overlay::Confirm(confirm)) = harness.app.overlay.as_ref() else {
        panic!("expected a confirm");
    };
    assert_eq!(confirm.question, "a push is running; q again quits and lets it finish");

    assert_eq!(harness.press(KeyCode::Char('q')), After::Quit);
}

#[test]
fn quitting_mid_push_never_cancels_the_child() {
    let mut harness = harness();
    harness.press(KeyCode::Char('P'));
    harness.press(KeyCode::Char('q'));
    harness.press(KeyCode::Char('q'));

    assert!(!harness.app.cancelled_anything(), "it killed the push on the way out");
}

#[test]
fn q_mid_read_closes_without_asking() {
    let mut harness = harness();
    harness.app.submit(
        herdr_damnit_application::JobKind::ReadStatus,
        herdr_damnit_application::argv::status(),
    );
    assert_eq!(harness.press(KeyCode::Char('q')), After::Quit);
}

#[test]
fn escape_closes_an_overlay_and_closes_the_pane_when_nothing_is_open() {
    let mut harness = harness_with(three_views());
    harness.press(KeyCode::Char('v'));
    assert_eq!(harness.press(KeyCode::Esc), After::Stay);
    assert!(harness.app.overlay.is_none());

    assert_eq!(harness.press(KeyCode::Esc), After::Quit);
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p herdr-damnit --locked keys_tests`
Expected: FAIL, `o` does nothing.

- [ ] **Step 3: Write the keys**

`App::key` gains `o`, `t`, `q` and the `Esc` fall-through. `o` and `t` look the oid under the cursor
up in `self.stage.conflicts` and do nothing when it is not there. `q` checks `jobs.exclusive()` and
raises `ConfirmPurpose::QuitMidJob` the first time. `App` gains `cancelled_anything()` for the test,
which reports whether `cancel_current` was ever called.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --workspace --locked -- --test-threads=1`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add -A
SKIP_AI_COMMIT=1 git commit -m "feat(pane): resolve a conflict and leave a running push to finish"
```

---

### Task 40: The discard key, gated on the dam version

**Depends on** `webdavis/damnit` PR A, which adds `dam restore <oid>` (item 2 of the spec's Needed
from dam). Every task before this one is independent of it. If PR A has not merged when this task
comes up, do the version gate and the unbound case now and come back for the bound case: the gate is
what makes the key appear the day the operator updates `dam`, with no pane release needed for it.

**Files:**
- Modify: `crates/herdr-damnit/src/app.rs`, `crates/herdr-damnit/src/app/keys_tests.rs`
- Modify: `crates/herdr-damnit-domain/src/version.rs` if PR A shipped `restore` at a minor other
  than 0.2

**Interfaces:**
- Consumes: `DAM_RESTORE` from Task 13; `restore` from Task 16; `Confirm` from Task 34.
- Produces these keys:

| Key | Given | Then |
|---|---|---|
| `!` | a row with a working change, and a `dam` at or above `DAM_RESTORE`, pressed once | a confirm names the object and the fields that would be lost |
| `!` | the confirm is up, `!` again | `dam restore <oid>` runs and the working change is discarded |
| `!` | the confirm is up, any other key | the confirm is dismissed and nothing is sent |

That gate is the only place a key depends on a `dam` version, and it exists because a confirm followed
by a refusal is the worst shape a destructive key can have.

`Change::fields` carries `dam`'s own `fields` array from 0.2.0 on, so the confirm names the fields
that would be lost. A change that names none, which is a delete, keeps `its working change`.

- [ ] **Step 1: Write the failing tests**

Add to `crates/herdr-damnit/src/app/keys_tests.rs`:

```rust
fn with_working_change(version: &str) -> super::tests::Harness {
    let mut harness = super::super::screens::tests::loaded();
    harness.app.dam_version = herdr_damnit_domain::parse_version(version);
    harness.app.submit(
        herdr_damnit_application::JobKind::ReadStatus,
        herdr_damnit_application::argv::status(),
    );
    harness.answer(
        2,
        0,
        r#"{"staged":[],"unstaged":[{"oid":"1a2b3c4","op":"update",
          "before":{"oid":"1a2b3c4","kind":"task","subject":"ship the pin bump"},
          "after":{"oid":"1a2b3c4","kind":"task","subject":"ship the pin bump"}}],
          "conflicts":[],"notices":[],"unpushed":[]}"#,
        "",
    );
    harness.app.select_oid(&herdr_damnit_domain::Oid::new("1a2b3c4"));
    harness
}

#[test]
fn the_discard_key_asks_once_and_restores_on_the_second_press() {
    let mut harness = with_working_change("dam 0.2.0");
    harness.press(KeyCode::Char('!'));

    let Some(Overlay::Confirm(confirm)) = harness.app.overlay.as_ref() else {
        panic!("expected a confirm");
    };
    assert!(confirm.question.contains("ship the pin bump"), "{}", confirm.question);

    harness.press(KeyCode::Char('!'));
    assert_eq!(harness.last(), "restore 1a2b3c4 --json");
}

#[test]
fn any_other_key_dismisses_the_discard_confirm_and_sends_nothing() {
    let mut harness = with_working_change("dam 0.2.0");
    harness.press(KeyCode::Char('!'));
    let before = harness.lines().len();
    harness.press(KeyCode::Char('j'));

    assert!(harness.app.overlay.is_none());
    assert_eq!(harness.lines().len(), before);
}

#[test]
fn the_discard_key_does_nothing_on_a_row_with_no_working_change() {
    let mut harness = with_working_change("dam 0.2.0");
    harness.app.select_oid(&herdr_damnit_domain::Oid::new("5d6e7f8"));
    let before = harness.lines().len();
    harness.press(KeyCode::Char('!'));

    assert!(harness.app.overlay.is_none());
    assert_eq!(harness.lines().len(), before);
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p herdr-damnit --locked keys_tests`
Expected: FAIL, `!` does nothing at any version.

- [ ] **Step 3: Write the key**

`App::key` gains:

```rust
            // The one key gated on a `dam` verb: a confirm followed by a refusal is the worst
            // shape a destructive key can have, so the key simply is not bound without it.
            KeyCode::Char('!') if self.has_restore() => self.ask_discard(),
```

with `has_restore` reading `self.dam_version.is_some_and(|found| found >= DAM_RESTORE)` and
`ask_discard` raising the confirm only when the oid under the cursor appears in
`self.stage.unstaged`.

**There is no test for a `dam` without `restore`, because no such `dam` reaches a key.** The verb
ships in 0.2.0 and the handshake refuses anything below that floor before a single key is bound, so
the unbound case is unreachable and a test for it would assert on a state the pane cannot be in. The
gate is kept anyway: it costs one comparison, it is pinned in Task 13 against the floor, and it is
what makes a future floor move visible at the key rather than at the first refusal.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --workspace --locked -- --test-threads=1`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add -A
SKIP_AI_COMMIT=1 git commit -m "feat(pane): bind the discard key only on a dam that has restore"
```

---

### Task 41: Doctor, the status action and the pane actions

**Files:**
- Rewrite: `crates/herdr-damnit/src/doctor.rs`
- Modify: `crates/herdr-damnit/src/pane.rs`
- Modify: `crates/herdr-damnit/src/screens.rs` (the Status screen opening)

**Interfaces:**
- Consumes: `Config`, `ProcessDamRunner`, `handshake` and the state directory.
- Produces:
  - `doctor::run(config: &Config) -> Result<String, String>`, which checks that `dam` is on the
    configured argv, reports its version and the verdict, runs one `dam status --json` and requires
    the five keys, and lists the remotes `dam` has configured with the age of their last pull. The
    last two are read from `dam` rather than guessed, and a `dam` that is not there reports the
    `PATH` it searched.
  - `pane::open_on_status(config: &Config) -> Result<String, String>`, the `status` action: it opens
    or focuses the pane the way `open` does, and writes `status` into the view-request file so the
    running pane shows the Status screen on its next tick.
  - `App::tick` reading a `status` request as the Status screen rather than as a view name.

The remotes and their last-pull times are item 6 of the spec's Needed from dam in part:
`dam remote list` exists and prints the configured remotes, and until it reports a last-pull time the
doctor prints the remote names alone and says the age is not available. Do not read `dam`'s config
file directly: the pane would own a second parser for a file `dam` owns.

- [ ] **Step 1: Write the failing tests**

`crates/herdr-damnit/src/doctor.rs`, inside `mod tests`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_dam_that_answers_reports_its_version_and_the_verdict() {
        let report = report_from("dam 0.2.0\n", Some(CLEAN), "example\n");
        assert!(report.contains("dam 0.2.0"), "{report}");
        assert!(report.contains("status: ok"), "{report}");
        assert!(report.contains("example"), "{report}");
    }

    #[test]
    fn a_dam_below_the_floor_is_reported_as_the_problem_it_is() {
        let report = report_from("dam 0.0.9\n", Some(CLEAN), "");
        assert!(report.contains("is older than the 0.2 this pane needs"), "{report}");
    }

    #[test]
    fn a_status_missing_a_key_is_reported_by_name() {
        let report = report_from(
            "dam 0.2.0\n",
            Some(r#"{"staged":[],"unstaged":[],"conflicts":[],"notices":[]}"#),
            "",
        );
        assert!(report.contains("unpushed"), "{report}");
    }

    #[test]
    fn a_dam_that_is_not_there_names_the_path_it_searched() {
        let report = missing_dam_report();
        assert!(report.contains("dam is not on PATH"), "{report}");
        assert!(report.contains("PATH:"), "{report}");
    }

    #[test]
    fn no_configured_remote_is_reported_rather_than_left_blank() {
        let report = report_from("dam 0.2.0\n", Some(CLEAN), "");
        assert!(report.contains("no remotes configured"), "{report}");
    }
}
```

`report_from` and `missing_dam_report` build the report from canned answers rather than spawning,
which means `doctor::run` splits in two the way `editor.rs` did: `run` gathers the three answers and
`report` renders them.

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p herdr-damnit --locked doctor`
Expected: FAIL, the doctor still checks a Todoist token.

- [ ] **Step 3: Write the doctor and the status action**

`doctor.rs` becomes `run` plus `report(version: &str, status: Option<&str>, remotes: &str) -> String`,
with `run` spawning `--version`, `status --json` and `remote list` through the configured argv and
handing the three outputs to `report`. `report` carries the `PATH` line only on the missing case.

`pane.rs` gains `open_on_status`, which is `run(Mode::Open, config)` followed by
`state::request_view(&state::view_request_path(), "status")`. `App::tick` treats the request word
`status` as `self.show(Screen::Status)` rather than as a view name, so an operator with a view
actually named `status` still reaches their view by number, and the reserved word is documented in
the README in Task 42.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --workspace --locked -- --test-threads=1`
Expected: PASS.

- [ ] **Step 5: Verify the binary answers every action the manifest declares**

```bash
cargo build --locked
for action in open toggle focus status auto-open doctor; do
  ./target/debug/herdr-damnit "$action" >/dev/null 2>&1
  printf '%s -> %s\n' "$action" "$?"
done
./target/debug/herdr-damnit view 1 >/dev/null 2>&1; printf 'view 1 -> %s\n' "$?"
./target/debug/herdr-damnit nonsense 2>&1 | head -1
```

Expected: every action runs and reports a herdr or `dam` failure rather than a usage error, since
there is no herdr and no `dam` in this shell; `nonsense` prints `herdr-damnit: unknown command
'nonsense'`.

- [ ] **Step 6: Commit**

```bash
git add -A
SKIP_AI_COMMIT=1 git commit -m "feat(pane): check dam in the doctor and open the pane on the status screen"
```

---

### Task 42: The README

**Files:**
- Rewrite: `README.md`

**Interfaces:**
- Consumes: the finished pane.
- Produces: a README that describes `herdr-damnit` over `dam` rather than `herdr-todoist` over the
  Todoist API.

The existing README has twenty sections. Their fate, one row each:

| Section | Fate |
|---|---|
| `# herdr-todoist` | Renamed `# herdr-damnit`, the opening paragraph naming `dam` and linking `webdavis/damnit` |
| `## Install` | Kept, with the binary name changed and one added line: `dam` must be installed, with `cargo install damnit` |
| `## The token` | **Deleted.** The token is `dam`'s, resolved from `[remote.todoist]` in `~/.config/dam/config.toml`. One sentence replaces the section, under Install, pointing there. |
| `## Actions` | Kept, with `herdr-damnit.` prefixes, the `status` action added and the doctor's description changed |
| `## Configuration` | Kept, rewritten for the new keys: `dam`, `handoff_label`, `query` in place of `filter`, and no `token_command`, `token_env` or `editor` |
| `## Placement` | Kept unchanged |
| `## Opening the pane` | Kept, with the `status` action added |
| `## Cache, refresh and writes made offline` | **Replaced** by `## Reading, refreshing and writes made offline`, describing the store as `dam`'s, an unpushed commit as the offline write, and the interval as two local reads |
| `## Views` | Kept, rewritten in `dam`'s query grammar, with the note that view 1 is `!done` and that a bare word resolves against `dam`'s saved filters first |
| `### Views as keybindings` | Kept, with the action names changed |
| `## The list` | Kept, rewritten for grouping by path and the corrected priority direction, with the four staging marks added to the mark table |
| `## Colors` | Kept, with the cyan slot added |
| `## The completed list` | **Replaced** by `## The three screens`, describing List, Status and Done and the `<Tab>` cycle |
| `## Keys in the pane` | Kept, rewritten as the spec's key table |
| `## The task detail` | Kept, with the comment thread removed and the event fields added |
| `### How much markdown is rendered` | Kept unchanged |
| `## Adding a comment` | **Deleted.** `dam` stores no comments and the key is freed. |
| `## Sending a task to the agent` | Kept, with the brief's fields changed and the record described as the `handoff_label` |
| `## Editing a task in an editor` | Kept, rewritten as `dam edit -e` owning the template, the parse loop and the editor choice |
| `## Quick edits` | Kept, one row per `dam` verb |
| `## Development` | Kept, with the four crates named and the fake `dam` described |
| `## License` | Kept unchanged |

Three things the README must say that the old one did not:

1. **`X` is force-complete, not reopen.** It is the only key whose meaning changed silently from
   `herdr-todoist`, so it gets its own line under Keys.
1. **`!` appears only on a `dam` that has `restore`.** An operator who does not see the key needs to
   know why.
1. **`status` is a reserved view-request word.** A view named `status` is still reachable by its
   number and by the picker; the `status` action's request word opens the Status screen.

- [ ] **Step 1: Rewrite the README**

Work section by section down the table. Every command line in it names `herdr-damnit` and every
config example uses the new keys.

- [ ] **Step 2: Verify nothing Todoist-shaped survived**

```bash
! grep -niE 'herdr-todoist|token_command|token_env|"filter"|filter =|comment' README.md
grep -c 'dam' README.md
```

Expected: no match on the first, and a non-zero count on the second. The word `comment` is in the
first pattern because the comment feature is gone; a legitimate use of the word elsewhere is fine to
keep, in which case narrow the grep rather than the README.

- [ ] **Step 3: Verify the whole repository has no personal data in it**

```bash
! grep -rniE 'stephen|/Users/|dresden|Todoist API Token' README.md crates/ herdr-plugin.toml
```

Expected: no match.

- [ ] **Step 4: Run every gate CI runs**

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked -- --test-threads=1
cargo doc --workspace --no-deps --locked
```

Expected: all four clean.

- [ ] **Step 5: Verify every file is inside the size cap**

```bash
find crates -name '*.rs' -exec wc -l {} + | sort -rn | head -20
```

Expected: no file over 500 lines, `main.rs` under 150, and nothing over 400 without a reason recorded
in its own commit.

- [ ] **Step 6: Commit**

```bash
git add README.md
SKIP_AI_COMMIT=1 git commit -m "docs: describe the pane over dam rather than the Todoist API"
```

---

## Self-review

Run after the last task, against the spec with fresh eyes.

**Spec coverage.** Every section of `docs/superpowers/specs/2026-09-20-herdr-damnit-design.md` maps
onto a task:

| Spec section | Tasks |
|---|---|
| 1, the split and the fate table | 28 deletes the Todoist path; every fate row lands in the task that rebuilds it |
| 2, naming and migration | 1, 2, 3; the dotfiles values are tabulated in Task 2 and are out of this plan |
| 3, the dam boundary, the handshake, the error mapping | 14, 15, 18, 20, 21, 28 |
| 4, the three screens, the keys, the views, the pickers, the detail, the hand-off, the editor, colours | 4 to 13, 27, 29 to 40 |
| 5, non-blocking operations, jobs, the header, cancellation, completion, closing mid-push | 17, 21, 22, 26, 33, 39 |
| 6, the four crates and the dependency direction | 4, 15, 20, 28 |
| 7, the fake dam, the fixtures, the golden renders, CI | 20, 23, 27, 29, 30, 42 |
| 8, targets, messages, out of scope, decisions, Needed from dam | 14, 40 for the gated key; items 3, 4 and 7 arrived in `dam` 0.2.0 and are built on rather than waited for; the four remaining are named where they bite |

**The gaps this plan leaves on purpose**, each because the spec puts it out of scope for version one:
events beyond reading them, a history browser, editing a recurrence rule, choosing a remote,
multi-select, categories as first-class UI, and anything Todoist-specific.

**Needed from dam, and where each one is named in this plan:**

| Item | Where |
|---|---|
| 1, clear `done` on a task | Not bound. `X` is force-complete, Task 34 |
| 2, `dam restore` | Task 40, behind the version gate |
| 3, a JSON error envelope | DELIVERED in `dam` 0.2.0, with a `rule` besides. Task 23 parses it, Task 14 maps it |
| 4, one exit code for every refusal | DELIVERED in `dam` 0.2.0 as exit 4, `nothing_to_commit` among them. Task 14 |
| 5, a completion timestamp | Task 23's `completions`, which walks the log instead |
| 6, the category catalogue and saved filters | Task 36's flat label picker; Task 41's doctor prints remote names alone |
| 7, a `status` without embedded objects | DELIVERED in `dam` 0.2.0 as the default. Task 23 reads the row shape and Task 16 asks for no `--full` |
| 8, richer date words | Task 35's hint names exactly the words `dam` accepts today |

**Type consistency.** The names used across tasks: `Oid::short`, `Priority::next`, `Priority::get`,
`Priority::is_lowest`, `Mark::of_priority`, `Mark::of_due`, `Mark::glyph`, `Mark::slot`,
`due_state`, `parse_date`, `short`, `long`, `Object::is_done`, `Object::priority`, `Object::due`,
`Row::oid`, `Row::text`, `StagingMarks::mark_of`, `rows`, `RowStyle`, `Cursor::move_by`,
`Cursor::replace`, `Cursor::selected_oid`, `Views::select_named`, `Views::name_of_number`,
`Stage::is_clean`, `Stage::is_staged`, `Stage::rows`, `Stage::summary`, `brief`, `parse_version`,
`verdict`, `classify`, `message`, `leaves_model_untouched`, `DamRunner::spawn`, `Herdr::call`,
`Clock::today`, `Jobs::submit`, `Jobs::drain`, `Jobs::exclusive`, `Jobs::cancel_current`,
`handshake`, `hand_off`, `ProcessDamRunner::new`, `ProcessDamRunner::spawn_with_deadline`,
`Cancel::interrupt`, `wire::objects`, `wire::object`, `wire::stage`, `wire::completions`,
`wire::push_summary`, `wire::pull_summary`, `Config::parse`, `App::key`, `App::tick`, `App::submit`,
`App::header`, `App::poll_window`, `App::reread`, `screens::draw`, `screens::render_to_text`,
`editor::run_and_reread`, `editor::round_trip`. Each is defined in exactly one task and used with
that spelling everywhere else.

**Three places a later task changes an earlier task's type, named so the change is not a surprise:**

1. Task 28 adds `Version` and `Handshake` to `JobKind`, defined in Task 17.
1. Task 29 adds `ReadDone` to the same enum.
1. Task 38 adds `herdr: Box<dyn Herdr>` and `here: Workspace` to `App::new`, defined in Task 26, so
   every harness built before it gains two arguments. Build the harness through a helper from the
   start so that change is one function.

## Execution handoff

Plan complete and saved to `docs/superpowers/plans/2026-09-20-herdr-damnit-implementation.md`.

Tasks 1 to 39 and 41 to 42 are independent of `webdavis/damnit`. Task 40 waits on that repository's
PR A, which adds `dam restore`, and is the only task that does.
