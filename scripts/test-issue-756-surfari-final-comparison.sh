#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RUN_ID="$(date +%Y%m%d-%H%M%S)"
LOG_DIR="$ROOT/logs/issue-756-exp31-surfari-roamium-comparison"
HARNESS_LOG="$LOG_DIR/harness-$RUN_ID.log"
APP="${TERMSURF_GHOSTBOARD_APP:-$ROOT/ghostboard/macos/build/Debug/TermSurf.app}"
APP_BIN="$APP/Contents/MacOS/termsurf"

mkdir -p "$LOG_DIR"

log() {
  printf '%s\n' "$*" | tee -a "$HARNESS_LOG"
}

fail() {
  log "FAIL: $*"
  exit 1
}

require_executable() {
  [ -x "$1" ] || fail "missing executable: $1"
}

require_path() {
  [ -e "$1" ] || fail "missing path: $1"
}

require_fresh_app_bundle() {
  require_executable "$APP_BIN"
  local newer
  newer="$(
    find \
      "$ROOT/ghostboard/src" \
      "$ROOT/ghostboard/macos/Sources" \
      "$ROOT/ghostboard/build.zig" \
      "$ROOT/ghostboard/macos/build.nu" \
      -type f -newer "$APP_BIN" -print -quit 2>/dev/null || true
  )"
  [ -z "$newer" ] || fail "Debug TermSurf.app is stale; newer input: $newer"
}

run_child() {
  local name="$1"
  local script="$2"
  local out="$LOG_DIR/${name}-${RUN_ID}.log"
  local before after run_id child_log

  require_executable "$script"
  before="$(date +%s)"
  log "child_start name=$name script=$script output=$out"
  if "$script" >"$out" 2>&1; then
    after="$(date +%s)"
    log "child_pass name=$name seconds=$((after - before)) output=$out"
  else
    cat "$out" >>"$HARNESS_LOG" || true
    fail "child failed name=$name output=$out"
  fi

  run_id="$(grep -E '^run_id=' "$out" | tail -1 | cut -d= -f2- || true)"
  [ -n "$run_id" ] || fail "child did not print run_id name=$name output=$out"
  log "child_run_id name=$name run_id=$run_id"

  while IFS= read -r child_log; do
    child_log="${child_log#"${child_log%%[![:space:]]*}"}"
    child_log="${child_log#*=}"
    [ -n "$child_log" ] || continue
    [ -e "$child_log" ] || fail "child reported missing log name=$name path=$child_log"
    log "child_log name=$name path=$child_log"
  done < <(grep -E '^[[:space:]]*(app|app_log|surfari_trace|webtui_trace|harness|scenario_app_log|scenario_surfari_trace|scenario_webtui_trace)=' "$out" || true)
}

run_fake_gui_devtools() {
  local out_dir="$ROOT/logs/i756e31fg-$RUN_ID"
  local out="$LOG_DIR/fake-gui-devtools-$RUN_ID.log"
  mkdir -p "$out_dir"
  log "child_start name=fake-gui-devtools output=$out log_dir=$out_dir"
  if "$ROOT/scripts/test-issue-756-surfari-fake-gui.py" \
    "file://$ROOT/surfari/libtermsurf_webkit/test-content/navigation.html" \
    --log-dir "$out_dir" >"$out" 2>&1; then
    log "child_pass name=fake-gui-devtools output=$out log_dir=$out_dir"
  else
    cat "$out" >>"$HARNESS_LOG" || true
    fail "child failed name=fake-gui-devtools output=$out"
  fi
  grep -E 'SMOKE_PASS .*devtools_supported=1 .*clean_exit=1' "$out" >/dev/null ||
    fail "fake GUI did not prove DevTools support output=$out"
  [ -s "$out_dir/messages.log" ] || fail "fake GUI messages log missing path=$out_dir/messages.log"
  [ -s "$out_dir/surfari-trace.log" ] || fail "fake GUI Surfari trace missing path=$out_dir/surfari-trace.log"
  [ -e "$out_dir/surfari.stdout" ] || fail "fake GUI Surfari stdout missing path=$out_dir/surfari.stdout"
  [ -e "$out_dir/surfari.stderr" ] || fail "fake GUI Surfari stderr missing path=$out_dir/surfari.stderr"
  log "child_log name=fake-gui-devtools path=$out"
  log "child_log name=fake-gui-devtools path=$out_dir/messages.log"
  log "child_log name=fake-gui-devtools path=$out_dir/surfari-trace.log"
  log "child_log name=fake-gui-devtools path=$out_dir/surfari.stdout"
  log "child_log name=fake-gui-devtools path=$out_dir/surfari.stderr"
}

log "run_id=$RUN_ID"
log "harness=$HARNESS_LOG"
log "app_bin=$APP_BIN"

require_fresh_app_bundle
require_executable "$ROOT/target/debug/web"
require_executable "$ROOT/target/debug/surfari"
require_path "$ROOT/webkit/src/WebKitBuild/Debug/WebKit.framework"
require_path "$ROOT/surfari/libtermsurf_webkit/build/libtermsurf_webkit.dylib"

run_fake_gui_devtools
run_child input-regression "$ROOT/scripts/test-issue-756-surfari-input-regression.sh"
run_child lifecycle "$ROOT/scripts/test-issue-756-surfari-lifecycle-tranche.sh"
run_child pane-split "$ROOT/scripts/test-issue-756-surfari-pane-split-geometry.sh"
run_child tab-window-focus "$ROOT/scripts/test-issue-756-surfari-tab-window-focus-geometry.sh"
run_child click-drag "$ROOT/scripts/test-issue-756-surfari-click-drag-input-details.sh"
run_child profile-isolation "$ROOT/scripts/test-issue-756-surfari-profile-isolation.sh"
run_child crash-handling "$ROOT/scripts/test-issue-756-surfari-crash-handling.sh"

log "PASS: Surfari final Roamium comparison aggregate completed"
log "logs:"
log "  harness=$HARNESS_LOG"
