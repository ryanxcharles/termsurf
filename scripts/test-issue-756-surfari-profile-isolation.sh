#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RUN_ID="$(date +%Y%m%d-%H%M%S)"
LOG_DIR="$ROOT/logs/issue-756-exp29-surfari-profile-isolation"
RUN_DIR="$(mktemp -d "${TMPDIR:-/tmp}/termsurf-issue756-exp29.XXXXXX")"
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
APP_LOG="$LOG_DIR/app-$RUN_ID.log"
HARNESS_LOG="$LOG_DIR/harness-$RUN_ID.log"
SURFARI_TRACE="$LOG_DIR/surfari-trace-$RUN_ID.log"
WEBTUI_TRACE="$LOG_DIR/webtui-$RUN_ID.log"
XDG_DATA="$RUN_DIR/data"
XDG_STATE="$RUN_DIR/state"
PID=""
HTTP_PID=""

mkdir -p "$LOG_DIR" "$SITE_DIR" "$XDG_DATA" "$XDG_STATE"

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
  if [ -n "${HTTP_PID:-}" ] && kill -0 "$HTTP_PID" >/dev/null 2>&1; then
    kill "$HTTP_PID" >/dev/null 2>&1 || true
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

extract_ready_socket() {
  printf '%s\n' "$1" | sed -E 's/.* socket=([^ ]+) browser=.*/\1/'
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

press_key() {
  local key="$1"
  local modifier="${2:-}"
  if [ -n "$modifier" ]; then
    swift "$ROOT/scripts/ghostty-app/inject.swift" key "$key" "$modifier" >>"$HARNESS_LOG" 2>&1
  else
    swift "$ROOT/scripts/ghostty-app/inject.swift" key "$key" >>"$HARNESS_LOG" 2>&1
  fi
}

type_text() {
  local text="$1"
  printf '%s' "$text" >"$TYPE_FILE"
  swift "$ROOT/scripts/ghostty-app/inject.swift" type "$TYPE_FILE" >>"$HARNESS_LOG" 2>&1
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

window_bounds_for() {
  swift "$WINDOW_BOUNDS" "$1"
}

click_frame_center() {
  local window_id="$1"
  local present_line="$2"
  local label="$3"
  local win_line wx wy wh frame_x frame_y frame_size frame_width frame_height root_size root_height content_y click_x click_y
  win_line="$(window_bounds_for "$window_id")" || fail "failed to resolve window bounds for $label window_id=$window_id"
  IFS=$'\t' read -r _wid wx wy _ww wh <<<"$win_line"
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
  local inactive_tab="$5"
  local inactive_pane="$6"
  local start_trace
  start_trace="$(line_count "$SURFARI_TRACE")"
  type_text "$marker"
  wait_for_file_pattern_after "$SURFARI_TRACE" "$start_trace" "key-event tab=${active_tab} pane=${active_pane}" "$label reached active Surfari"
  require_no_file_pattern_after "$SURFARI_TRACE" "$start_trace" "key-event tab=${inactive_tab} pane=${inactive_pane}" "$label did not reach inactive Surfari"
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

wait_for_spawn() {
  local start_line="$1"
  local profile="$2"
  local label="$3"
  local line
  for _ in $(seq 1 60); do
    line="$(tail -n +"$((start_line + 1))" "$APP_LOG" | grep -F "spawned browser path=$SURFARI" | grep -F "profile=$profile browser=surfari" | grep -E "webkit-profiles/${profile}" | tail -1 || true)"
    if [ -n "$line" ]; then
      printf '%s\n' "$line"
      return 0
    fi
    delay 1
  done
  fail "timed out waiting for $label"
}

send_browser_navigate() {
  local socket_path="$1"
  local tab_id="$2"
  local url="$3"
  python3 - "$socket_path" "$tab_id" "$url" <<'PY'
import socket
import struct
import sys

socket_path = sys.argv[1]
tab_id = int(sys.argv[2])
url = sys.argv[3]

def varint(value):
    out = bytearray()
    while value >= 0x80:
        out.append((value & 0x7F) | 0x80)
        value >>= 7
    out.append(value)
    return bytes(out)

def field(number, wire_type):
    return varint((number << 3) | wire_type)

def string_field(number, value):
    data = value.encode("utf-8")
    return field(number, 2) + varint(len(data)) + data

payload = field(1, 0) + varint(tab_id) + string_field(3, url)
message = field(5, 2) + varint(len(payload)) + payload
with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as sock:
    sock.connect(socket_path)
    sock.sendall(struct.pack("<I", len(message)) + message)
PY
}

extract_spawn_pid() {
  printf '%s\n' "$1" | sed -E 's/.* pid=([0-9]+) profile=.*/\1/'
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

cat >"$SITE_DIR/profile.html" <<'EOF'
<!doctype html>
<html>
  <head>
    <meta charset="utf-8">
    <title>Issue 756 Surfari Profile</title>
  </head>
  <body>
    <input id="field" autofocus value="">
    <script>
      function cookieValue(name) {
        const prefix = name + "=";
        const entry = document.cookie
          .split(";")
          .map((part) => part.trim())
          .find((part) => part.startsWith(prefix));
        return entry ? decodeURIComponent(entry.slice(prefix.length)) : "none";
      }

      const profile = new URLSearchParams(location.search).get("profile");
      const beforeLocal = localStorage.getItem("issue756Exp29Profile") || "none";
      const beforeCookie = cookieValue("issue756Exp29Profile");
      localStorage.setItem("issue756Exp29Profile", profile);
      document.cookie = `issue756Exp29Profile=${encodeURIComponent(profile)}; Path=/; SameSite=Lax`;
      console.log(
        `ISSUE756_EXP29_PROFILE_STORAGE profile=${profile} localStorage_before=${beforeLocal} localStorage_after=${profile} cookie_before=${beforeCookie} cookie_after=${profile}`,
      );
      document.getElementById("field").focus();
    </script>
  </body>
</html>
EOF

HTTP_PORT="$(python3 - <<'PY'
import socket

with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
    s.bind(("127.0.0.1", 0))
    print(s.getsockname()[1])
PY
)"
URL_A="http://127.0.0.1:${HTTP_PORT}/profile.html?profile=profilea"
URL_B="http://127.0.0.1:${HTTP_PORT}/profile.html?profile=profileb"

python3 -m http.server "$HTTP_PORT" --bind 127.0.0.1 --directory "$SITE_DIR" >>"$HARNESS_LOG" 2>&1 &
HTTP_PID="$!"
for _ in $(seq 1 30); do
  if python3 - "$URL_A" <<'PY' >/dev/null 2>&1
import sys
import urllib.request

with urllib.request.urlopen(sys.argv[1], timeout=1) as response:
    raise SystemExit(0 if response.status == 200 else 1)
PY
  then
    break
  fi
  delay 0.25
done
python3 - "$URL_A" <<'PY' >/dev/null 2>&1 || fail "HTTP fixture did not become ready"
import sys
import urllib.request

with urllib.request.urlopen(sys.argv[1], timeout=1) as response:
    raise SystemExit(0 if response.status == 200 else 1)
PY

cat >"$COMMAND" <<EOF
#!/usr/bin/env bash
set -euo pipefail
if mkdir "$FIRST_RUN_MARKER" 2>/dev/null; then
  exec "$WEB" --browser surfari --profile profilea "$URL_A"
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
log "xdg_data_home=$XDG_DATA"
log "url_a=$URL_A"
log "url_b=$URL_B"
log "app_log=$APP_LOG"
log "surfari_trace=$SURFARI_TRACE"
log "webtui_trace=$WEBTUI_TRACE"

GHOSTTY_CONFIG_PATH="$CONFIG" \
GHOSTTY_LOG=stderr \
DYLD_FRAMEWORK_PATH="$WEBKIT_DEBUG" \
TERMSURF_SURFARI_PATH="$SURFARI" \
TERMSURF_GEOMETRY_TRACE=1 \
TERMSURF_GEOMETRY_SCENARIO=issue756-exp29-surfari-profile-isolation \
TERMSURF_WEBTUI_STATE_TRACE_FILE="$WEBTUI_TRACE" \
TERMSURF_INPUT_TRACE=1 \
TERMSURF_PDF_INPUT_TRACE=1 \
TERMSURF_PDF_INPUT_TRACE_FILE="$SURFARI_TRACE" \
XDG_DATA_HOME="$XDG_DATA" \
XDG_STATE_HOME="$XDG_STATE" \
  "$APP_BIN" >"$APP_LOG" 2>&1 &
PID="$!"
log "pid=$PID"

A_START="$(line_count "$APP_LOG")"
A_TRACE_START="$(line_count "$SURFARI_TRACE")"
wait_for_file_pattern_after "$APP_LOG" "$A_START" "SetOverlay: pane_id=[^ ]+ profile=profilea browser=surfari" "profile A SetOverlay"
wait_for_file_pattern_after "$APP_LOG" "$A_START" "SetOverlay: created pending server key=profilea/surfari pane_count=1" "profile A server key"
A_SPAWN_LINE="$(wait_for_spawn "$A_START" "profilea" "profile A repo Surfari spawn")"
A_SPAWN_PID="$(extract_spawn_pid "$A_SPAWN_LINE")"
[ -n "$A_SPAWN_PID" ] || fail "failed to extract profile A Surfari pid"
log "profile_a_spawn_pid=$A_SPAWN_PID"
wait_for_file_pattern_after "$APP_LOG" "$A_START" "ServerRegister: profile=profilea browser=surfari" "profile A ServerRegister"
wait_for_file_pattern_after "$APP_LOG" "$A_START" "BrowserReady: pane_id=.* browser=surfari" "profile A BrowserReady"
wait_for_file_pattern_after "$WEBTUI_TRACE" 0 "event=console_message.*message=ISSUE756_EXP29_PROFILE_STORAGE profile=profilea localStorage_before=none localStorage_after=profilea cookie_before=none cookie_after=profilea" "profile A initial localStorage and cookie marker" 60

A_READY_LINE="$(tail -n +"$((A_START + 1))" "$APP_LOG" | grep -E "BrowserReady: pane_id=.* browser=surfari" | tail -1)"
A_PANE_ID="$(extract_pane_id "$A_READY_LINE")"
A_TAB_ID="$(extract_ready_tab_id "$A_READY_LINE")"
A_SOCKET="$(extract_ready_socket "$A_READY_LINE")"
A_PRESENT_LINE="$(wait_for_appkit_present_for "$A_START" "$A_PANE_ID" "[0-9]+" "profile A AppKit presentation")"
A_CONTEXT_ID="$(extract_context_id "$A_PRESENT_LINE")"
A_SELECTED_TAB_ID="$(extract_selected_tab_id "$A_PRESENT_LINE")"
A_WINDOW_ID="$(extract_window_id "$A_PRESENT_LINE")"
log "profile_a_pane_id=$A_PANE_ID"
log "profile_a_tab_id=$A_TAB_ID"
log "profile_a_socket=$A_SOCKET"
log "profile_a_context_id=$A_CONTEXT_ID"
log "profile_a_selected_tab_id=$A_SELECTED_TAB_ID"

B_TAB_START="$(line_count "$APP_LOG")"
B_TRACE_START="$(line_count "$SURFARI_TRACE")"
B_WEBTUI_START="$(line_count "$WEBTUI_TRACE")"
log "profile_b_new_tab_keybind=ctrl+t"
press_key 17 control
delay 2
wait_for_file_pattern_after "$APP_LOG" "$B_TAB_START" "dispatching action target=surface action=.new_tab" "profile B native tab action dispatched"
B_SELECTED_LINE="$(wait_for_selected_tab_change_after "$B_TAB_START" "$A_SELECTED_TAB_ID" "profile B tab selected")"
B_SELECTED_TAB_ID="$(extract_selected_tab_id "$B_SELECTED_LINE")"
[ -n "$B_SELECTED_TAB_ID" ] || fail "failed to extract profile B selected tab id"
log "profile_b_selected_tab_id=$B_SELECTED_TAB_ID"

printf '"%s" --browser surfari --profile profileb "%s"' "$WEB" "$URL_B" >"$TYPE_FILE"
log "profile_b_command=$(cat "$TYPE_FILE")"
swift "$ROOT/scripts/ghostty-app/inject.swift" type "$TYPE_FILE" >>"$HARNESS_LOG" 2>&1
press_key 36
wait_for_file_pattern_after "$APP_LOG" "$B_TAB_START" "SetOverlay: pane_id=[^ ]+ profile=profileb browser=surfari" "profile B SetOverlay"
wait_for_file_pattern_after "$APP_LOG" "$B_TAB_START" "SetOverlay: created pending server key=profileb/surfari pane_count=1" "profile B server key"
B_SPAWN_LINE="$(wait_for_spawn "$B_TAB_START" "profileb" "profile B repo Surfari spawn")"
B_SPAWN_PID="$(extract_spawn_pid "$B_SPAWN_LINE")"
[ -n "$B_SPAWN_PID" ] || fail "failed to extract profile B Surfari pid"
[ "$B_SPAWN_PID" != "$A_SPAWN_PID" ] || fail "profile B reused profile A Surfari pid"
log "profile_b_spawn_pid=$B_SPAWN_PID"
wait_for_file_pattern_after "$APP_LOG" "$B_TAB_START" "ServerRegister: profile=profileb browser=surfari" "profile B ServerRegister"
wait_for_file_pattern_after "$WEBTUI_TRACE" "$B_WEBTUI_START" "event=console_message.*message=ISSUE756_EXP29_PROFILE_STORAGE profile=profileb localStorage_before=none localStorage_after=profileb cookie_before=none cookie_after=profileb" "profile B initial localStorage and cookie marker" 60

B_CA_LINE="$(wait_for_ca_context_excluding_after "$B_TAB_START" "profile B ca_context" "$A_PANE_ID")"
B_PANE_ID="$(extract_pane_id "$B_CA_LINE")"
B_TAB_ID="$(extract_browser_tab_id "$B_CA_LINE")"
B_CONTEXT_ID="$(extract_context_id "$B_CA_LINE")"
[ "$B_PANE_ID" != "$A_PANE_ID" ] || fail "profile B reused profile A pane id"
[ "$B_CONTEXT_ID" != "$A_CONTEXT_ID" ] || fail "profile B reused profile A context id"
if [ "$B_TAB_ID" = "$A_TAB_ID" ]; then
  log "profile_tab_ids_are_process_local=true"
fi
B_PRESENT_LINE="$(wait_for_appkit_present_for "$B_TAB_START" "$B_PANE_ID" "$B_CONTEXT_ID" "profile B AppKit presentation")"
log "profile_b_pane_id=$B_PANE_ID"
log "profile_b_tab_id=$B_TAB_ID"
log "profile_b_context_id=$B_CONTEXT_ID"

click_expect_hit "$B_SELECTED_TAB_ID" "$B_PRESENT_LINE" "$B_CONTEXT_ID" "profile_b_area"
enter_browse "profile_b" "$B_PANE_ID" "$B_TAB_ID"
type_marker_require_only "profile B keyboard" "ISSUE756_EXP29_PROFILE_B_KEY" "$B_TAB_ID" "$B_PANE_ID" "$A_TAB_ID" "$A_PANE_ID"
leave_browse "profile_b" "$B_PANE_ID" "$B_TAB_ID"

A_RETURN_START="$(line_count "$APP_LOG")"
A_RETURN_TRACE_START="$(line_count "$SURFARI_TRACE")"
A_RETURN_WEBTUI_START="$(line_count "$WEBTUI_TRACE")"
log "profile_a_return_keybind=ctrl+p"
press_key 35 control
delay 1
wait_for_file_pattern_after "$APP_LOG" "$A_RETURN_START" "Pane focus changed: pane_id=${A_PANE_ID} focused=true" "profile A pane focused again"
click_expect_hit "$A_SELECTED_TAB_ID" "$A_PRESENT_LINE" "$A_CONTEXT_ID" "profile_a_return_area"
enter_browse "profile_a_return" "$A_PANE_ID" "$A_TAB_ID"
log "profile_a_return_navigate=$URL_A"
send_browser_navigate "$A_SOCKET" "$A_TAB_ID" "$URL_A"
wait_for_file_pattern_after "$WEBTUI_TRACE" "$A_RETURN_WEBTUI_START" "event=console_message.*message=ISSUE756_EXP29_PROFILE_STORAGE profile=profilea localStorage_before=profilea localStorage_after=profilea cookie_before=profilea cookie_after=profilea" "profile A retained localStorage and cookie marker after profile B" 60
type_marker_require_only "profile A return keyboard" "ISSUE756_EXP29_PROFILE_A_KEY" "$A_TAB_ID" "$A_PANE_ID" "$B_TAB_ID" "$B_PANE_ID"
leave_browse "profile_a_return" "$A_PANE_ID" "$A_TAB_ID"

if tail -n +"$((A_TRACE_START + 1))" "$APP_LOG" | grep -E "spawned browser path=.*profile=profile[ab] browser=surfari" | grep -Fv "spawned browser path=$SURFARI" >/dev/null 2>&1; then
  fail "Ghostboard spawned Surfari from a path other than $SURFARI"
fi
if [ ! -d "$XDG_DATA/termsurf/webkit-profiles/profilea" ]; then
  fail "missing profile A user-data directory"
fi
if [ ! -d "$XDG_DATA/termsurf/webkit-profiles/profileb" ]; then
  fail "missing profile B user-data directory"
fi

log "PASS: Surfari profile isolation real-app harness completed"
log "logs:"
log "  app=$APP_LOG"
log "  surfari_trace=$SURFARI_TRACE"
log "  webtui_trace=$WEBTUI_TRACE"
log "  harness=$HARNESS_LOG"
