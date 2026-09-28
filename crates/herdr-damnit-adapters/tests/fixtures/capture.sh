#!/usr/bin/env bash
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

point_dams_default_store_and_config_into_the_scratch_home_and_stamp_events_in_utc() {
  export HOME="$work/home"
  export XDG_CONFIG_HOME="$work/home/.config"
  export XDG_DATA_HOME="$work/home/.local/share"
  export TZ=UTC
  mkdir -p "$XDG_CONFIG_HOME" "$XDG_DATA_HOME"
}

point_dams_default_store_and_config_into_the_scratch_home_and_stamp_events_in_utc

export DAM_STORE="$work/store.sqlite"
export DAM_CONFIG="$work/config.toml"
cat >"$DAM_CONFIG" <<'CONFIG'
[remote.example]
url = "example::"
CONFIG

"$dam" --version >"$here/version.txt"

capture_the_clean_status_before_the_first_commit_leaves_one_unpushed() {
  "$dam" status --json >"$here/status-clean.json"
}

capture_the_clean_status_before_the_first_commit_leaves_one_unpushed

"$dam" new "ship the pin bump" --path "proj/dotfiles" -p 1 --due 2026-09-18 >/dev/null
"$dam" new "refresh the roster row" --path "proj/dotfiles" --label slow >/dev/null
"$dam" new "water the plants" --path "proj/home" >/dev/null
plants="$("$dam" ls "path:proj/home/" --json |
  python3 -c 'import json,sys; print(json.load(sys.stdin)["objects"][0]["oid"])')"
make_weekly_since_recurrence_is_an_edit_flag_and_not_a_new_one() {
  "$dam" edit "$1" --recurrence "every week" >/dev/null
}

make_weekly_since_recurrence_is_an_edit_flag_and_not_a_new_one "$plants"
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
