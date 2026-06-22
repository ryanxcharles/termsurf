#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RUN_ID="$(date +%Y%m%d-%H%M%S)"
LOG_DIR="$ROOT/logs/issue-756-exp27-surfari-tab-window-focus"
RUN_DIR="$(mktemp -d "${TMPDIR:-/tmp}/termsurf-issue756-exp27.XXXXXX")"
APP="${TERMSURF_GHOSTBOARD_APP:-$ROOT/ghostboard/macos/build/Debug/TermSurf.app}"
APP_BIN="$APP/Contents/MacOS/termsurf"
WEB="${TERMSURF_WEB:-$ROOT/target/debug/web}"
SURFARI="${TERMSURF_SURFARI:-$ROOT/target/debug/surfari}"
WEBKIT_DEBUG="${TERMSURF_WEBKIT_DEBUG:-$ROOT/webkit/src/WebKitBuild/Debug}"
SITE_DIR="$RUN_DIR/site"
CONFIG="$RUN_DIR/config"
COMMAND="$RUN_DIR/run-web-surfari.sh"
FIRST_RUN_MARKER="$RUN_DIR/first-web-ran"
TYPE_FILE="$RUN_DIR/type.txt"
WINDOW_BOUNDS="$RUN_DIR/window-bounds.swift"
APP_WINDOWS="$RUN_DIR/app-windows.swift"
APP_LOG="$LOG_DIR/app-$RUN_ID.log"
HARNESS_LOG="$LOG_DIR/harness-$RUN_ID.log"
SURFARI_TRACE="$LOG_DIR/surfari-trace-$RUN_ID.log"
WEBTUI_TRACE="$LOG_DIR/webtui-$RUN_ID.log"
PID=""

mkdir -p "$LOG_DIR" "$SITE_DIR"

log() {
  printf '%s\n' "$*" | tee -a "$HARNESS_LOG"
}

fail() {
  log "FAIL: $*"
  exit 1
}

delay() {
  osascript -e "delay ${1:-0.5}" >/dev/null
}

cleanup() {
  if [ -n "${PID:-}" ] && kill -0 "$PID" >/dev/null 2>&1; then
    kill "$PID" >/dev/null 2>&1 || true
    delay 0.5 || true
    kill -9 "$PID" >/dev/null 2>&1 || true
  fi
  rm -rf "$RUN_DIR"
}
trap cleanup EXIT

require_executable() {
  [ -x "$1" ] || fail "missing executable: $1"
}

require_path() {
  [ -e "$1" ] || fail "missing path: $1"
}

line_count() {
  local file="$1"
  if [ -r "$file" ]; then
    wc -l <"$file" | tr -d ' '
  else
    printf '0\n'
  fi
}

wait_for_file_pattern_after() {
  local file="$1"
  local start_line="$2"
  local pattern="$3"
  local label="$4"
  local attempts="${5:-60}"
  for _ in $(seq 1 "$attempts"); do
    if tail -n +"$((start_line + 1))" "$file" | grep -E "$pattern" >/dev/null 2>&1; then
      log "PASS: $label"
      return 0
    fi
    delay 1
  done
  fail "timed out waiting for $label"
}

wait_for_line_after() {
  local file="$1"
  local start_line="$2"
  local pattern="$3"
  local label="$4"
  local attempts="${5:-60}"
  local line
  for _ in $(seq 1 "$attempts"); do
    line="$(tail -n +"$((start_line + 1))" "$file" | grep -E "$pattern" | tail -1 || true)"
    if [ -n "$line" ]; then
      printf '%s\n' "$line"
      return 0
    fi
    delay 1
  done
  fail "timed out waiting for $label"
}

require_no_file_pattern_after() {
  local file="$1"
  local start_line="$2"
  local pattern="$3"
  local label="$4"
  if tail -n +"$((start_line + 1))" "$file" | grep -E "$pattern" >/dev/null 2>&1; then
    fail "$label"
  fi
  log "PASS: $label"
}

extract_pane_id() {
  printf '%s\n' "$1" | sed -E 's/.*pane_id[:=]([^ ]+).*/\1/'
}

extract_browser_tab_id() {
  printf '%s\n' "$1" | sed -E 's/.*browser_tab_id:([^ ]+) .*/\1/'
}

extract_ready_tab_id() {
  printf '%s\n' "$1" | sed -E 's/.*tab_id=([0-9]+) socket=.*/\1/'
}

extract_ready_pane_id() {
  printf '%s\n' "$1" | sed -E 's/.*BrowserReady: pane_id=([^ ]+) tab_id=.*/\1/'
}

extract_context_id() {
  printf '%s\n' "$1" | sed -E 's/.*context_id=([^ ]+).*/\1/'
}

extract_selected_tab_id() {
  printf '%s\n' "$1" | sed -E 's/.*selected_tab_id:([^ ]+) .*/\1/'
}

extract_window_id() {
  printf '%s\n' "$1" | sed -E 's/.*identity=window_id:([0-9]+).*/\1/'
}

extract_overlay_frame() {
  printf '%s\n' "$1" | sed -E 's/.*overlay_frame=(\{\{[^}]+\}, \{[^}]+\}\}).*/\1/'
}

extract_frame_x() {
  printf '%s\n' "$1" | sed -E 's/.*overlay_frame=\{\{([^,]+), [^}]+\}, \{[^}]+\}\}.*/\1/'
}

extract_frame_y() {
  printf '%s\n' "$1" | sed -E 's/.*overlay_frame=\{\{[^,]+, ([^}]+)\}, \{[^}]+\}\}.*/\1/'
}

extract_frame_size() {
  printf '%s\n' "$1" | sed -E 's/.*overlay_frame=\{\{[^}]+\}, \{([^,]+), ([^}]+)\}\}.*/\1x\2/'
}

extract_root_frame_size() {
  printf '%s\n' "$1" | sed -E 's/.*root_frame=\{\{[^}]+\}, \{([^,]+), ([^}]+)\}\}.*/\1x\2/'
}

pair_width() {
  printf '%s\n' "$1" | awk -Fx '{print $1}'
}

pair_height() {
  printf '%s\n' "$1" | awk -Fx '{print $2}'
}

window_bounds_for() {
  swift "$WINDOW_BOUNDS" "$1"
}

app_windows() {
  swift "$APP_WINDOWS" "$PID"
}

pointer_move() {
  swift "$ROOT/scripts/ghostty-app/inject.swift" move "$1" "$2" >>"$HARNESS_LOG" 2>&1
}

pointer_click() {
  swift "$ROOT/scripts/ghostty-app/inject.swift" click "$1" "$2" left 1 >>"$HARNESS_LOG" 2>&1
}

click_global_point() {
  local x="$1"
  local y="$2"
  local label="$3"
  log "${label}_input_point=${x},${y}"
  pointer_move "$x" "$y"
  delay 0.25
  pointer_click "$x" "$y"
}

click_frame_center() {
  local window_id="$1"
  local present_line="$2"
  local label="$3"
  local win_line wx wy ww wh frame_x frame_y frame_size frame_width frame_height root_size root_height content_y click_x click_y
  win_line="$(window_bounds_for "$window_id")" || fail "failed to resolve window bounds for $label window_id=$window_id"
  IFS=$'\t' read -r _wid wx wy ww wh <<<"$win_line"
  frame_x="$(extract_frame_x "$present_line")"
  frame_y="$(extract_frame_y "$present_line")"
  frame_size="$(extract_frame_size "$present_line")"
  frame_width="$(pair_width "$frame_size")"
  frame_height="$(pair_height "$frame_size")"
  root_size="$(extract_root_frame_size "$present_line")"
  root_height="$(pair_height "$root_size")"
  content_y="$(awk -v wh="$wh" -v root_h="$root_height" 'BEGIN { print int(wh - root_h) }')"
  click_x="$(awk -v wx="$wx" -v fx="$frame_x" -v fw="$frame_width" 'BEGIN { print int(wx + fx + (fw / 2) + 0.5) }')"
  click_y="$(awk -v wy="$wy" -v cy="$content_y" -v fy="$frame_y" -v fh="$frame_height" 'BEGIN { print int(wy + cy + fy + (fh / 2) + 0.5) }')"
  click_global_point "$click_x" "$click_y" "$label"
}

click_window_center() {
  local bounds="$1"
  local label="$2"
  local wid wx wy ww wh
  IFS=$'\t' read -r wid wx wy ww wh <<<"$bounds"
  click_global_point "$((wx + ww / 2))" "$((wy + wh / 2))" "$label"
}

wait_for_hit_after() {
  local start_line="$1"
  local context_id="$2"
  local label="$3"
  local line
  for _ in $(seq 1 30); do
    line="$(tail -n +"$((start_line + 1))" "$APP_LOG" | grep -E "TermSurf geometry layer=appkit event=hit_test .*context_id=${context_id} .*hit=true .*web_point=\\{" | tail -1 || true)"
    if [ -n "$line" ]; then
      printf '%s\n' "$line"
      return 0
    fi
    delay 1
  done
  fail "timed out waiting for $label"
}

wait_for_changed_appkit_frame_after() {
  local start_line="$1"
  local pane_id="$2"
  local context_id="$3"
  local old_frame="$4"
  local label="$5"
  local line frame
  for _ in $(seq 1 30); do
    while IFS= read -r line; do
      frame="$(extract_overlay_frame "$line")"
      if [ -n "$frame" ] && [ "$frame" != "$line" ] && [ "$frame" != "$old_frame" ]; then
        printf '%s\n' "$line"
        return 0
      fi
    done < <(tail -n +"$((start_line + 1))" "$APP_LOG" |
      grep -E "TermSurf geometry layer=appkit event=presented .*pane_id:${pane_id} .*context_id=${context_id}" || true)
    delay 1
  done
  fail "timed out waiting for $label"
}

wait_for_negative_hit_after() {
  local start_line="$1"
  local context_id="$2"
  local label="$3"
  for _ in $(seq 1 10); do
    if tail -n +"$((start_line + 1))" "$APP_LOG" | grep -E "TermSurf geometry layer=appkit event=hit_test .*context_id=${context_id} .*hit=true" >/dev/null 2>&1; then
      fail "$label routed to inactive Surfari context"
    fi
    if tail -n +"$((start_line + 1))" "$APP_LOG" | grep -E "TermSurf geometry layer=appkit event=hit_test .*context_id=${context_id} .*hit=false" >/dev/null 2>&1; then
      log "PASS: observed $label with explicit hit=false"
      return 0
    fi
    delay 1
  done
  log "PASS: $label did not route to inactive Surfari context"
}

click_expect_hit() {
  local window_id="$1"
  local present_line="$2"
  local context_id="$3"
  local label="$4"
  local start
  start="$(line_count "$APP_LOG")"
  click_frame_center "$window_id" "$present_line" "$label"
  wait_for_hit_after "$start" "$context_id" "$label hit-test" >/dev/null
  log "PASS: $label hit-test reached expected Surfari context"
}

click_expect_no_hit() {
  local window_id="$1"
  local present_line="$2"
  local context_id="$3"
  local label="$4"
  local start
  start="$(line_count "$APP_LOG")"
  click_frame_center "$window_id" "$present_line" "$label"
  wait_for_negative_hit_after "$start" "$context_id" "$label negative hit-test"
}

type_text() {
  local text="$1"
  printf '%s' "$text" >"$TYPE_FILE"
  swift "$ROOT/scripts/ghostty-app/inject.swift" type "$TYPE_FILE" >>"$HARNESS_LOG" 2>&1
}

press_key() {
  local key="$1"
  local modifier="${2:-}"
  if [ -n "$modifier" ]; then
    swift "$ROOT/scripts/ghostty-app/inject.swift" key "$key" "$modifier" >>"$HARNESS_LOG" 2>&1
  else
    swift "$ROOT/scripts/ghostty-app/inject.swift" key "$key" >>"$HARNESS_LOG" 2>&1
  fi
}

enter_browse() {
  local label="$1"
  local pane_id="$2"
  local tab_id="$3"
  local start_app start_trace
  start_app="$(line_count "$APP_LOG")"
  start_trace="$(line_count "$SURFARI_TRACE")"
  log "${label}_enter_browse_key=enter"
  press_key 36
  wait_for_file_pattern_after "$APP_LOG" "$start_app" "ModeChanged: pane_id=${pane_id} browsing=true" "$label entered browse mode"
  wait_for_file_pattern_after "$SURFARI_TRACE" "$start_trace" "focus-changed tab=${tab_id} pane=${pane_id} ffi=ts_set_focus focused=true" "$label Surfari focus=true"
}

leave_browse() {
  local label="$1"
  local pane_id="$2"
  local tab_id="$3"
  local start_app start_trace
  start_app="$(line_count "$APP_LOG")"
  start_trace="$(line_count "$SURFARI_TRACE")"
  log "${label}_leave_browse_key=escape"
  press_key 53
  wait_for_file_pattern_after "$APP_LOG" "$start_app" "ModeChanged: pane_id=${pane_id} browsing=false" "$label left browse mode"
  wait_for_file_pattern_after "$SURFARI_TRACE" "$start_trace" "focus-changed tab=${tab_id} pane=${pane_id} ffi=ts_set_focus focused=false" "$label Surfari focus=false"
}

type_marker_require_only() {
  local label="$1"
  local marker="$2"
  local active_tab="$3"
  local active_pane="$4"
  local inactive_tab_1="${5:-}"
  local inactive_pane_1="${6:-}"
  local inactive_tab_2="${7:-}"
  local inactive_pane_2="${8:-}"
  local start_trace
  start_trace="$(line_count "$SURFARI_TRACE")"
  type_text "$marker"
  wait_for_file_pattern_after "$SURFARI_TRACE" "$start_trace" "key-event tab=${active_tab} pane=${active_pane}" "$label reached active Surfari"
  if [ -n "$inactive_tab_1" ] && [ -n "$inactive_pane_1" ]; then
    require_no_file_pattern_after "$SURFARI_TRACE" "$start_trace" "key-event tab=${inactive_tab_1} pane=${inactive_pane_1}" "$label did not reach inactive Surfari 1"
  fi
  if [ -n "$inactive_tab_2" ] && [ -n "$inactive_pane_2" ]; then
    require_no_file_pattern_after "$SURFARI_TRACE" "$start_trace" "key-event tab=${inactive_tab_2} pane=${inactive_pane_2}" "$label did not reach inactive Surfari 2"
  fi
}

wait_for_selected_tab_change_after() {
  local start_line="$1"
  local old_tab="$2"
  local label="$3"
  local line selected
  for _ in $(seq 1 30); do
    while IFS= read -r line; do
      selected="$(extract_selected_tab_id "$line")"
      case "$selected" in
        ""|"$line"|"$old_tab"|unknown:*|-1) ;;
        *)
          printf '%s\n' "$line"
          return 0
          ;;
      esac
    done < <(tail -n +"$((start_line + 1))" "$APP_LOG" | grep -E "TermSurf geometry layer=appkit event=.*selected_tab_id:" || true)
    delay 1
  done
  fail "timed out waiting for $label"
}

wait_for_different_ca_context_after() {
  local start_line="$1"
  local old_pane="$2"
  local label="$3"
  wait_for_ca_context_excluding_after "$start_line" "$label" "$old_pane"
}

wait_for_browser_ready_excluding_after() {
  local start_line="$1"
  local label="$2"
  shift 2
  local line excluded skip
  for _ in $(seq 1 60); do
    while IFS= read -r line; do
      skip=false
      for excluded in "$@"; do
        if [[ "$line" == *"pane_id=${excluded}"* ]]; then
          skip=true
          break
        fi
      done
      if [ "$skip" = false ]; then
        printf '%s\n' "$line"
        return 0
      fi
    done < <(tail -n +"$((start_line + 1))" "$APP_LOG" |
      grep -E "BrowserReady: pane_id=.* browser=surfari" || true)
    delay 1
  done
  fail "timed out waiting for $label"
}

wait_for_surfari_ca_context_after() {
  local start_line="$1"
  local tab_id="$2"
  local pane_id="$3"
  local label="$4"
  wait_for_line_after "$SURFARI_TRACE" "$start_line" "ca-context tab=${tab_id} pane=${pane_id} .*context_id=[0-9]+" "$label"
}

wait_for_ca_context_excluding_after() {
  local start_line="$1"
  local label="$2"
  shift 2
  local line excluded skip
  for _ in $(seq 1 60); do
    while IFS= read -r line; do
      skip=false
      for excluded in "$@"; do
        if [[ "$line" == *"pane_id:${excluded}"* ]]; then
          skip=true
          break
        fi
      done
      if [ "$skip" = false ]; then
        printf '%s\n' "$line"
        return 0
      fi
    done < <(tail -n +"$((start_line + 1))" "$APP_LOG" |
      grep -E "TermSurf geometry layer=zig event=ca_context " || true)
    delay 1
  done
  fail "timed out waiting for $label"
}

wait_for_appkit_present_for() {
  local start_line="$1"
  local pane_id="$2"
  local context_id="$3"
  local label="$4"
  wait_for_line_after "$APP_LOG" "$start_line" "TermSurf geometry layer=appkit event=presented .*pane_id:${pane_id} .*context_id=${context_id}" "$label"
}

cat >"$WINDOW_BOUNDS" <<'EOF'
import CoreGraphics
import Foundation

guard CommandLine.arguments.count == 2,
      let target = Int(CommandLine.arguments[1]),
      let info = CGWindowListCopyWindowInfo([.optionAll], kCGNullWindowID) as? [[String: Any]]
else { exit(2) }

for window in info {
    guard let id = window[kCGWindowNumber as String] as? Int, id == target else { continue }
    let bounds = (window[kCGWindowBounds as String] as? [String: Any]) ?? [:]
    let x = Int((bounds["X"] as? Double) ?? 0)
    let y = Int((bounds["Y"] as? Double) ?? 0)
    let width = Int((bounds["Width"] as? Double) ?? 0)
    let height = Int((bounds["Height"] as? Double) ?? 0)
    print("\(id)\t\(x)\t\(y)\t\(width)\t\(height)")
    exit(0)
}
exit(1)
EOF

cat >"$APP_WINDOWS" <<'EOF'
import CoreGraphics
import Foundation

guard CommandLine.arguments.count == 2,
      let targetPID = Int(CommandLine.arguments[1]),
      let info = CGWindowListCopyWindowInfo([.optionAll], kCGNullWindowID) as? [[String: Any]]
else { exit(2) }

for window in info {
    let pid = (window[kCGWindowOwnerPID as String] as? Int) ?? -1
    let layer = (window[kCGWindowLayer as String] as? Int) ?? -1
    let onscreen = (window[kCGWindowIsOnscreen as String] as? Bool) ?? false
    guard pid == targetPID, layer == 0, onscreen else { continue }
    guard let id = window[kCGWindowNumber as String] as? Int else { continue }
    let bounds = (window[kCGWindowBounds as String] as? [String: Any]) ?? [:]
    let x = Int((bounds["X"] as? Double) ?? 0)
    let y = Int((bounds["Y"] as? Double) ?? 0)
    let width = Int((bounds["Width"] as? Double) ?? 0)
    let height = Int((bounds["Height"] as? Double) ?? 0)
    guard width >= 50, height >= 50 else { continue }
    print("\(id)\t\(x)\t\(y)\t\(width)\t\(height)")
}
EOF

cat >"$SITE_DIR/a.html" <<'EOF'
<!doctype html>
<meta charset="utf-8">
<title>Issue 756 Surfari Tab A</title>
<input id="field" autofocus value="">
<script>document.getElementById("field").focus(); console.log("ISSUE756_EXP27_A_READY");</script>
EOF

cat >"$SITE_DIR/b.html" <<'EOF'
<!doctype html>
<meta charset="utf-8">
<title>Issue 756 Surfari Tab B</title>
<input id="field" autofocus value="">
<script>document.getElementById("field").focus(); console.log("ISSUE756_EXP27_B_READY");</script>
EOF

cat >"$SITE_DIR/c.html" <<'EOF'
<!doctype html>
<meta charset="utf-8">
<title>Issue 756 Surfari Window C</title>
<input id="field" autofocus value="">
<script>document.getElementById("field").focus(); console.log("ISSUE756_EXP27_C_READY");</script>
EOF

URL_A="file://$SITE_DIR/a.html"
URL_B="file://$SITE_DIR/b.html"
URL_C="file://$SITE_DIR/c.html"

cat >"$COMMAND" <<EOF
#!/usr/bin/env bash
set -euo pipefail
if mkdir "$FIRST_RUN_MARKER" 2>/dev/null; then
  exec "$WEB" --browser surfari "$URL_A"
fi
exec /bin/zsh -f
EOF
chmod +x "$COMMAND"

cat >"$CONFIG" <<EOF
window-save-state = never
initial-command = direct:$COMMAND
keybind = ctrl+t=new_tab
keybind = ctrl+1=goto_tab:1
keybind = ctrl+2=goto_tab:2
keybind = ctrl+p=previous_tab
keybind = ctrl+n=next_tab
keybind = ctrl+b=new_window
EOF

require_executable "$APP_BIN"
require_executable "$WEB"
require_executable "$SURFARI"
require_path "$WEBKIT_DEBUG/WebKit.framework"
require_path "$ROOT/surfari/libtermsurf_webkit/build/libtermsurf_webkit.dylib"

log "run_id=$RUN_ID"
log "app_bin=$APP_BIN"
log "web=$WEB"
log "surfari=$SURFARI"
log "url_a=$URL_A"
log "url_b=$URL_B"
log "url_c=$URL_C"
log "app_log=$APP_LOG"
log "surfari_trace=$SURFARI_TRACE"
log "webtui_trace=$WEBTUI_TRACE"

GHOSTTY_CONFIG_PATH="$CONFIG" \
GHOSTTY_LOG=stderr \
DYLD_FRAMEWORK_PATH="$WEBKIT_DEBUG" \
TERMSURF_SURFARI_PATH="$SURFARI" \
TERMSURF_GEOMETRY_TRACE=1 \
TERMSURF_GEOMETRY_SCENARIO=issue756-exp27-tab-window-focus \
TERMSURF_WEBTUI_STATE_TRACE_FILE="$WEBTUI_TRACE" \
TERMSURF_INPUT_TRACE=1 \
TERMSURF_PDF_INPUT_TRACE=1 \
TERMSURF_PDF_INPUT_TRACE_FILE="$SURFARI_TRACE" \
  "$APP_BIN" >"$APP_LOG" 2>&1 &
PID="$!"
log "pid=$PID"

APP_START="$(line_count "$APP_LOG")"
TRACE_START="$(line_count "$SURFARI_TRACE")"
wait_for_file_pattern_after "$APP_LOG" "$APP_START" "BrowserReady: pane_id=.* browser=surfari" "browser A BrowserReady"
wait_for_file_pattern_after "$APP_LOG" "$APP_START" "TermSurf geometry layer=appkit event=presented " "browser A AppKit presented"
wait_for_file_pattern_after "$SURFARI_TRACE" "$TRACE_START" "title-changed tab=.*title=Issue 756 Surfari Tab A" "browser A loaded title"

A_READY_LINE="$(tail -n +"$((APP_START + 1))" "$APP_LOG" | grep -E "BrowserReady: pane_id=.* browser=surfari" | tail -1)"
A_PANE_ID="$(extract_pane_id "$A_READY_LINE")"
A_TAB_ID="$(extract_ready_tab_id "$A_READY_LINE")"
A_PRESENT_LINE="$(wait_for_appkit_present_for "$APP_START" "$A_PANE_ID" "[0-9]+" "browser A initial AppKit presentation")"
A_CONTEXT_ID="$(extract_context_id "$A_PRESENT_LINE")"
A_SELECTED_TAB_ID="$(extract_selected_tab_id "$A_PRESENT_LINE")"
A_WINDOW_ID="$(extract_window_id "$A_PRESENT_LINE")"
A_FRAME="$(extract_overlay_frame "$A_PRESENT_LINE")"
log "browser_a_pane_id=$A_PANE_ID"
log "browser_a_tab_id=$A_TAB_ID"
log "browser_a_context_id=$A_CONTEXT_ID"
log "browser_a_selected_tab_id=$A_SELECTED_TAB_ID"
log "browser_a_window_id=$A_WINDOW_ID"

NEW_TAB_START="$(line_count "$APP_LOG")"
NEW_TAB_TRACE_START="$(line_count "$SURFARI_TRACE")"
log "new_tab_keybind=ctrl+t"
press_key 17 control
delay 2
wait_for_file_pattern_after "$APP_LOG" "$NEW_TAB_START" "dispatching action target=surface action=.new_tab" "new terminal tab action dispatched"
A_TABBED_PRESENT_LINE="$(wait_for_changed_appkit_frame_after "$NEW_TAB_START" "$A_PANE_ID" "$A_CONTEXT_ID" "$A_FRAME" "browser A geometry adjusted for native tab bar")"
TAB2_LINE="$(wait_for_selected_tab_change_after "$NEW_TAB_START" "$A_SELECTED_TAB_ID" "tab 2 selected")"
TAB2_SELECTED_TAB_ID="$(extract_selected_tab_id "$TAB2_LINE")"
log "tab2_selected_tab_id=$TAB2_SELECTED_TAB_ID"
if tail -n +"$((NEW_TAB_START + 1))" "$APP_LOG" | grep -E "TermSurf geometry layer=appkit event=presented .*pane_id:${A_PANE_ID} .*context_id=${A_CONTEXT_ID} .*visible=true .*selected_tab_id:${TAB2_SELECTED_TAB_ID}" >/dev/null 2>&1; then
  fail "browser A overlay was presented as visible in selected terminal tab 2"
fi
log "PASS: browser A was not freshly presented as visible in terminal tab 2"
click_expect_no_hit "$TAB2_SELECTED_TAB_ID" "$A_PRESENT_LINE" "$A_CONTEXT_ID" "tab2_former_browser_a_area"
type_text "ISSUE756_EXP27_TAB2_TERMINAL"
press_key 36
delay 1
require_no_file_pattern_after "$SURFARI_TRACE" "$NEW_TAB_TRACE_START" "key-event tab=${A_TAB_ID} pane=${A_PANE_ID}" "terminal tab keyboard did not reach browser A"

SWITCH_A_START="$(line_count "$APP_LOG")"
log "switch_to_browser_a_keybind=ctrl+p"
press_key 35 control
delay 1
wait_for_file_pattern_after "$APP_LOG" "$SWITCH_A_START" "Pane focus changed: pane_id=${A_PANE_ID} focused=true" "browser A pane focused after tab restore"
click_expect_hit "$A_SELECTED_TAB_ID" "$A_TABBED_PRESENT_LINE" "$A_CONTEXT_ID" "browser_a_restored_area"
enter_browse "browser_a_restored" "$A_PANE_ID" "$A_TAB_ID"
type_marker_require_only "browser A restored keyboard" "a" "$A_TAB_ID" "$A_PANE_ID"
leave_browse "browser_a_restored" "$A_PANE_ID" "$A_TAB_ID"

SWITCH_TAB2_START="$(line_count "$APP_LOG")"
log "switch_to_tab2_keybind=ctrl+2"
press_key 19 control
delay 1
wait_for_file_pattern_after "$APP_LOG" "$SWITCH_TAB2_START" "Pane focus changed: pane_id=${A_PANE_ID} focused=false" "browser A pane unfocused after switching to tab 2"
BROWSER_B_START="$(line_count "$APP_LOG")"
BROWSER_B_TRACE_START="$(line_count "$SURFARI_TRACE")"
printf '"%s" --browser surfari "%s"' "$WEB" "$URL_B" >"$TYPE_FILE"
log "browser_b_command=$(cat "$TYPE_FILE")"
swift "$ROOT/scripts/ghostty-app/inject.swift" type "$TYPE_FILE" >>"$HARNESS_LOG" 2>&1
press_key 36
B_READY_LINE="$(wait_for_browser_ready_excluding_after "$BROWSER_B_START" "browser B BrowserReady" "$A_PANE_ID")"
B_PANE_ID="$(extract_ready_pane_id "$B_READY_LINE")"
B_TAB_ID="$(extract_ready_tab_id "$B_READY_LINE")"
B_CA_LINE="$(wait_for_surfari_ca_context_after "$BROWSER_B_TRACE_START" "$B_TAB_ID" "$B_PANE_ID" "browser B ca_context")"
B_CONTEXT_ID="$(extract_context_id "$B_CA_LINE")"
[ "$B_PANE_ID" != "$A_PANE_ID" ] || fail "browser B reused browser A pane id"
[ "$B_TAB_ID" != "$A_TAB_ID" ] || fail "browser B reused browser A tab id"
[ "$B_CONTEXT_ID" != "$A_CONTEXT_ID" ] || fail "browser B reused browser A context id"
wait_for_file_pattern_after "$SURFARI_TRACE" "$BROWSER_B_TRACE_START" "title-changed tab=${B_TAB_ID} pane=${B_PANE_ID} title=Issue 756 Surfari Tab B" "browser B loaded title"
B_PRESENT_LINE="$(wait_for_appkit_present_for "$BROWSER_B_START" "$B_PANE_ID" "$B_CONTEXT_ID" "browser B AppKit presentation")"
B_SELECTED_TAB_ID="$(extract_selected_tab_id "$B_PRESENT_LINE")"
[ "$B_SELECTED_TAB_ID" = "$TAB2_SELECTED_TAB_ID" ] || fail "browser B selected tab mismatch: expected=$TAB2_SELECTED_TAB_ID actual=$B_SELECTED_TAB_ID"
if tail -n +"$((BROWSER_B_START + 1))" "$APP_LOG" | grep -E "TermSurf geometry layer=appkit event=presented .*pane_id:${A_PANE_ID} .*context_id=${A_CONTEXT_ID} .*visible=true .*selected_tab_id:${TAB2_SELECTED_TAB_ID}" >/dev/null 2>&1; then
  fail "browser A overlay became visible in browser B tab"
fi
log "PASS: browser A stayed hidden while browser B tab selected"
click_expect_hit "$B_SELECTED_TAB_ID" "$B_PRESENT_LINE" "$B_CONTEXT_ID" "browser_b_area"
enter_browse "browser_b" "$B_PANE_ID" "$B_TAB_ID"
type_marker_require_only "browser B keyboard" "b" "$B_TAB_ID" "$B_PANE_ID" "$A_TAB_ID" "$A_PANE_ID"
leave_browse "browser_b" "$B_PANE_ID" "$B_TAB_ID"

SWITCH_A2_START="$(line_count "$APP_LOG")"
log "switch_back_to_browser_a_keybind=ctrl+p"
press_key 35 control
delay 1
wait_for_file_pattern_after "$APP_LOG" "$SWITCH_A2_START" "Pane focus changed: pane_id=${A_PANE_ID} focused=true" "browser A pane focused after browser B"
A_AFTER_B_PRESENT="$A_TABBED_PRESENT_LINE"
click_expect_hit "$A_SELECTED_TAB_ID" "$A_AFTER_B_PRESENT" "$A_CONTEXT_ID" "browser_a_after_b_area"
enter_browse "browser_a_after_b" "$A_PANE_ID" "$A_TAB_ID"
type_marker_require_only "browser A after B keyboard" "a" "$A_TAB_ID" "$A_PANE_ID" "$B_TAB_ID" "$B_PANE_ID"
leave_browse "browser_a_after_b" "$A_PANE_ID" "$A_TAB_ID"

NEW_WINDOW_START="$(line_count "$APP_LOG")"
log "new_window_keybind=ctrl+b"
press_key 11 control
delay 2
wait_for_file_pattern_after "$APP_LOG" "$NEW_WINDOW_START" "dispatching action target=surface action=.new_window" "new window action dispatched"
WINDOW_C_BOUNDS=""
for _ in $(seq 1 30); do
  WINDOW_C_BOUNDS="$(app_windows | awk -F '\t' -v old="$A_WINDOW_ID" '$1 != old { print; exit }' || true)"
  [ -n "$WINDOW_C_BOUNDS" ] && break
  delay 1
done
[ -n "$WINDOW_C_BOUNDS" ] || fail "timed out waiting for second TermSurf window"
IFS=$'\t' read -r C_WINDOW_ID _C_WX _C_WY _C_WW _C_WH <<<"$WINDOW_C_BOUNDS"
log "window_c_bounds=$WINDOW_C_BOUNDS"
click_window_center "$WINDOW_C_BOUNDS" "window_c_shell_focus"
delay 1

BROWSER_C_START="$(line_count "$APP_LOG")"
BROWSER_C_TRACE_START="$(line_count "$SURFARI_TRACE")"
printf '"%s" --browser surfari "%s"' "$WEB" "$URL_C" >"$TYPE_FILE"
log "browser_c_command=$(cat "$TYPE_FILE")"
swift "$ROOT/scripts/ghostty-app/inject.swift" type "$TYPE_FILE" >>"$HARNESS_LOG" 2>&1
press_key 36
C_READY_LINE="$(wait_for_browser_ready_excluding_after "$BROWSER_C_START" "browser C BrowserReady" "$A_PANE_ID" "$B_PANE_ID")"
C_PANE_ID="$(extract_ready_pane_id "$C_READY_LINE")"
C_TAB_ID="$(extract_ready_tab_id "$C_READY_LINE")"
C_CA_LINE="$(wait_for_surfari_ca_context_after "$BROWSER_C_TRACE_START" "$C_TAB_ID" "$C_PANE_ID" "browser C ca_context")"
C_CONTEXT_ID="$(extract_context_id "$C_CA_LINE")"
[ "$C_PANE_ID" != "$A_PANE_ID" ] || fail "browser C reused browser A pane id"
[ "$C_PANE_ID" != "$B_PANE_ID" ] || fail "browser C reused browser B pane id"
[ "$C_TAB_ID" != "$A_TAB_ID" ] || fail "browser C reused browser A tab id"
[ "$C_TAB_ID" != "$B_TAB_ID" ] || fail "browser C reused browser B tab id"
[ "$C_CONTEXT_ID" != "$A_CONTEXT_ID" ] || fail "browser C reused browser A context id"
[ "$C_CONTEXT_ID" != "$B_CONTEXT_ID" ] || fail "browser C reused browser B context id"
wait_for_file_pattern_after "$SURFARI_TRACE" "$BROWSER_C_TRACE_START" "title-changed tab=${C_TAB_ID} pane=${C_PANE_ID} title=Issue 756 Surfari Window C" "browser C loaded title"
C_PRESENT_LINE="$(wait_for_appkit_present_for "$BROWSER_C_START" "$C_PANE_ID" "$C_CONTEXT_ID" "browser C AppKit presentation")"
C_SELECTED_WINDOW_ID="$(extract_window_id "$C_PRESENT_LINE")"
[ "$C_SELECTED_WINDOW_ID" = "$C_WINDOW_ID" ] || fail "browser C window mismatch: expected=$C_WINDOW_ID actual=$C_SELECTED_WINDOW_ID"
if tail -n +"$((BROWSER_C_START + 1))" "$APP_LOG" | grep -E "TermSurf geometry layer=appkit event=presented .*identity=window_id:${C_WINDOW_ID} .*pane_id:${A_PANE_ID} .*context_id=${A_CONTEXT_ID} .*visible=true" >/dev/null 2>&1; then
  fail "browser A overlay became visible in browser C window"
fi
log "PASS: browser A stayed out of browser C window"
click_expect_hit "$C_WINDOW_ID" "$C_PRESENT_LINE" "$C_CONTEXT_ID" "browser_c_area"
enter_browse "browser_c" "$C_PANE_ID" "$C_TAB_ID"
type_marker_require_only "browser C keyboard" "c" "$C_TAB_ID" "$C_PANE_ID" "$A_TAB_ID" "$A_PANE_ID" "$B_TAB_ID" "$B_PANE_ID"
leave_browse "browser_c" "$C_PANE_ID" "$C_TAB_ID"

log "PASS: issue 756 Surfari tab/window/focus geometry"
