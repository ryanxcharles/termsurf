#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RUN_ID="$(date +%Y%m%d-%H%M%S)"
LOG_DIR="$ROOT/logs/issue-756-exp28-surfari-click-drag-input-details"
RUN_DIR="$(mktemp -d "${TMPDIR:-/tmp}/termsurf-issue756-exp28.XXXXXX")"
APP="${TERMSURF_GHOSTBOARD_APP:-$ROOT/ghostboard/macos/build/Debug/TermSurf.app}"
APP_BIN="$APP/Contents/MacOS/termsurf"
WEB="${TERMSURF_WEB:-$ROOT/target/debug/web}"
SURFARI="${TERMSURF_SURFARI:-$ROOT/target/debug/surfari}"
WEBKIT_DEBUG="${TERMSURF_WEBKIT_DEBUG:-$ROOT/webkit/src/WebKitBuild/Debug}"
CONFIG="$RUN_DIR/config"
COMMAND="$RUN_DIR/run-web-surfari.sh"
SITE_DIR="$RUN_DIR/site"
TYPE_FILE="$RUN_DIR/type-token.txt"
APP_LOG="$LOG_DIR/app-$RUN_ID.log"
HARNESS_LOG="$LOG_DIR/harness-$RUN_ID.log"
SURFARI_TRACE="$LOG_DIR/surfari-trace-$RUN_ID.log"
WEBTUI_TRACE="$LOG_DIR/webtui-$RUN_ID.log"
PID=""
HTTP_PID=""

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

wait_for_file_pattern() {
  local file="$1"
  local pattern="$2"
  local label="$3"
  local attempts="${4:-60}"
  for _ in $(seq 1 "$attempts"); do
    if grep -E "$pattern" "$file" >/dev/null 2>&1; then
      log "PASS: $label"
      return 0
    fi
    delay 1
  done
  fail "timed out waiting for $label"
}

wait_for_file_pattern_after() {
  local file="$1"
  local start_line="$2"
  local pattern="$3"
  local label="$4"
  local attempts="${5:-45}"
  for _ in $(seq 1 "$attempts"); do
    if tail -n +"$((start_line + 1))" "$file" | grep -E "$pattern" >/dev/null 2>&1; then
      log "PASS: $label"
      return 0
    fi
    delay 1
  done
  fail "timed out waiting for $label"
}

extract_first_match() {
  local file="$1"
  local pattern="$2"
  grep -E "$pattern" "$file" | head -1 || true
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

extract_root_frame_size() {
  printf '%s\n' "$1" | sed -E 's/.*root_frame=\{\{[^}]+\}, \{([^,]+), ([^}]+)\}\}.*/\1x\2/'
}

pair_height() {
  printf '%s\n' "$1" | awk -Fx '{print $2}'
}

exact_window_bounds() {
  local window_id="$1"
  swift - "$window_id" <<'SWIFT'
import CoreGraphics
import Foundation

let target = Int(CommandLine.arguments[1])!
guard let info = CGWindowListCopyWindowInfo([.optionAll], kCGNullWindowID) as? [[String: Any]] else {
    exit(1)
}

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
SWIFT
}

activate_pid() {
  local pid="$1"
  local label="$2"
  local front_pid
  front_pid="$(osascript \
    -e 'tell application "System Events" to set frontmost of first process whose unix id is '"$pid"' to true' \
    -e 'delay 0.25' \
    -e 'tell application "System Events" to unix id of first process whose frontmost is true')"
  if [ "$front_pid" != "$pid" ]; then
    fail "$label frontmost PID mismatch: got=$front_pid expected=$pid"
  fi
  log "PASS: $label frontmost pid=$front_pid"
}

global_point_for_web_point() {
  local win_line="$1"
  local present_line="$2"
  local web_x="$3"
  local web_y="$4"
  local _wid wx wy _ww wh frame_x frame_y root_frame_size root_height content_y_offset
  IFS=$'\t' read -r _wid wx wy _ww wh <<<"$win_line"
  frame_x="$(extract_frame_x "$present_line")"
  frame_y="$(extract_frame_y "$present_line")"
  root_frame_size="$(extract_root_frame_size "$present_line")"
  root_height="$(pair_height "$root_frame_size")"
  content_y_offset="$(awk -v wh="$wh" -v root_h="$root_height" 'BEGIN { print int(wh - root_h) }')"
  awk \
    -v wx="$wx" \
    -v wy="$wy" \
    -v content_y="$content_y_offset" \
    -v frame_x="$frame_x" \
    -v frame_y="$frame_y" \
    -v web_x="$web_x" \
    -v web_y="$web_y" \
    'BEGIN {
      print int(wx + frame_x + web_x + 0.5) "\t" int(wy + content_y + frame_y + web_y + 0.5)
    }'
}

send_browser_close() {
  local socket_path="$1"
  local tab_id="$2"
  python3 - "$socket_path" "$tab_id" <<'PY'
import socket
import struct
import sys

socket_path = sys.argv[1]
tab_id = int(sys.argv[2])

def varint(value):
    out = bytearray()
    while value >= 0x80:
        out.append((value & 0x7F) | 0x80)
        value >>= 7
    out.append(value)
    return bytes(out)

def field(number, wire_type):
    return varint((number << 3) | wire_type)

payload = field(1, 0) + varint(tab_id)
message = field(4, 2) + varint(len(payload)) + payload
with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as sock:
    sock.connect(socket_path)
    sock.sendall(struct.pack("<I", len(message)) + message)
PY
}

HTTP_PORT="$(python3 - <<'PY'
import socket

with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
    s.bind(("127.0.0.1", 0))
    print(s.getsockname()[1])
PY
)"
URL="http://127.0.0.1:${HTTP_PORT}/index.html"

cat >"$SITE_DIR/index.html" <<'EOF'
<!doctype html>
<html>
  <head>
    <meta charset="utf-8">
    <title>Issue 756 Surfari Click Drag Initial</title>
    <style>
      html,
      body {
        margin: 0;
        background: white;
        color: #111;
        font: 16px -apple-system, BlinkMacSystemFont, sans-serif;
      }

      #field {
        position: absolute;
        left: 60px;
        top: 60px;
        width: 420px;
        height: 34px;
        font: 18px -apple-system, BlinkMacSystemFont, sans-serif;
      }

      #click-zone {
        position: absolute;
        left: 60px;
        top: 168px;
        width: 320px;
        height: 54px;
        border: 1px solid #333;
        line-height: 54px;
        user-select: none;
      }

      #drag-input {
        position: absolute;
        left: 60px;
        top: 248px;
        width: 420px;
        height: 36px;
        font: 18px -apple-system, BlinkMacSystemFont, sans-serif;
        line-height: 36px;
      }

      #result {
        position: absolute;
        left: 60px;
        top: 320px;
      }
    </style>
    <script>
      let seq = 0;

      function safe(value) {
        return String(value).replace(/[^A-Za-z0-9_.:-]/g, "_");
      }

      function report(kind, detail = "") {
        seq += 1;
        const marker = `ISSUE756_EXP28_INPUT seq=${seq} kind=${kind} ${detail}`.trim();
        console.log(marker);
        document.title = `Issue 756 Surfari Click Drag ${kind} ${seq}`;
        document.getElementById("result").textContent = marker;
      }

      window.addEventListener("DOMContentLoaded", () => {
        const input = document.getElementById("field");
        const clickZone = document.getElementById("click-zone");
        const dragInput = document.getElementById("drag-input");

        input.addEventListener("focus", () => {
          report("focus", `active=${safe(document.activeElement.id)}`);
        });
        input.addEventListener("input", () => {
          report("input", `value=${safe(input.value)} start=${input.selectionStart} end=${input.selectionEnd} active=${safe(document.activeElement.id)}`);
        });
        clickZone.addEventListener("click", (event) => {
          report("click", `detail=${event.detail} shift=${event.shiftKey} active=${safe(document.activeElement.id)}`);
        });
        dragInput.addEventListener("mouseup", () => {
          setTimeout(() => {
            const text = dragInput.value.substring(dragInput.selectionStart, dragInput.selectionEnd);
            report("selection", `text=${safe(text)} start=${dragInput.selectionStart} end=${dragInput.selectionEnd} active=${safe(document.activeElement.id)}`);
          }, 0);
        });
        window.addEventListener("wheel", (event) => {
          report("wheel", `deltaY=${Math.round(event.deltaY)} scrollY=${Math.round(window.scrollY)}`);
        }, { passive: true });

        input.focus();
        report("ready", `url=${safe(window.location.href)}`);
      });
    </script>
  </head>
  <body>
    <input id="field" autocomplete="off" spellcheck="false">
    <div id="click-zone">ISSUE756_EXP28_CLICK_ZONE</div>
    <input id="drag-input" readonly value="ISSUE756_EXP28_BROWSER_DRAG_TEXT" />
    <div id="result">ISSUE756_EXP28_BOOT</div>
  </body>
</html>
EOF

python3 -m http.server "$HTTP_PORT" --bind 127.0.0.1 --directory "$SITE_DIR" >>"$HARNESS_LOG" 2>&1 &
HTTP_PID="$!"
for _ in $(seq 1 30); do
  if python3 - "$URL" <<'PY' >/dev/null 2>&1
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
python3 - "$URL" <<'PY' >/dev/null 2>&1 || fail "HTTP fixture did not become ready"
import sys
import urllib.request

with urllib.request.urlopen(sys.argv[1], timeout=1) as response:
    raise SystemExit(0 if response.status == 200 else 1)
PY

cat >"$COMMAND" <<EOF
#!/usr/bin/env bash
exec "$WEB" --browser surfari "$URL"
EOF
chmod +x "$COMMAND"

cat >"$CONFIG" <<EOF
window-save-state = never
initial-command = direct:$COMMAND
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
log "url=$URL"
log "app_log=$APP_LOG"
log "surfari_trace=$SURFARI_TRACE"
log "webtui_trace=$WEBTUI_TRACE"

GHOSTTY_CONFIG_PATH="$CONFIG" \
GHOSTTY_LOG=stderr \
DYLD_FRAMEWORK_PATH="$WEBKIT_DEBUG" \
TERMSURF_SURFARI_PATH="$SURFARI" \
TERMSURF_GEOMETRY_TRACE=1 \
TERMSURF_GEOMETRY_SCENARIO=issue756-exp28-surfari-click-drag-input-details \
TERMSURF_WEBTUI_STATE_TRACE_FILE="$WEBTUI_TRACE" \
TERMSURF_INPUT_TRACE=1 \
TERMSURF_PDF_INPUT_TRACE=1 \
TERMSURF_PDF_INPUT_TRACE_FILE="$SURFARI_TRACE" \
  "$APP_BIN" >"$APP_LOG" 2>&1 &
PID="$!"
log "pid=$PID"

wait_for_file_pattern "$APP_LOG" "SetOverlay: pane_id=.* profile=default browser=surfari url=${URL}" "web requested Surfari input fixture"
wait_for_file_pattern "$APP_LOG" "BrowserReady: pane_id=.* browser=surfari" "Ghostboard emitted Surfari BrowserReady"
wait_for_file_pattern "$APP_LOG" "TermSurf geometry layer=appkit event=presented " "AppKit presented Surfari overlay"
wait_for_file_pattern "$WEBTUI_TRACE" "event=console_message.*message=ISSUE756_EXP28_INPUT .*kind=ready" "fixture ready console marker"

BROWSER_READY_LINE="$(extract_first_match "$APP_LOG" "BrowserReady: pane_id=.* browser=surfari")"
PANE_ID="$(printf '%s\n' "$BROWSER_READY_LINE" | sed -E 's/.*pane_id=([^ ]+) tab_id=.*/\1/')"
BROWSER_SOCKET="$(printf '%s\n' "$BROWSER_READY_LINE" | sed -E 's/.*socket=([^ ]+) browser=surfari.*/\1/')"
BROWSER_TAB_ID="$(printf '%s\n' "$BROWSER_READY_LINE" | sed -E 's/.*tab_id=([0-9]+) socket=.*/\1/')"
case "$PANE_ID" in
  ''|"$BROWSER_READY_LINE") fail "could not extract pane id from BrowserReady: $BROWSER_READY_LINE" ;;
esac
case "$BROWSER_SOCKET" in
  /*) ;;
  *) fail "could not extract browser socket from BrowserReady: $BROWSER_READY_LINE" ;;
esac
case "$BROWSER_TAB_ID" in
  ''|*[!0-9]*) fail "could not extract tab id from BrowserReady: $BROWSER_READY_LINE" ;;
esac

PRESENTED_LINE="$(extract_first_match "$APP_LOG" "TermSurf geometry layer=appkit event=presented .*pane_id:${PANE_ID}")"
[ -n "$PRESENTED_LINE" ] || fail "missing AppKit presented line for pane $PANE_ID"
PRESENTED_WINDOW_ID="$(extract_window_id "$PRESENTED_LINE")"
WIN_LINE="$(exact_window_bounds "$PRESENTED_WINDOW_ID")" || fail "failed to resolve presented window bounds"
log "presented_window_bounds=$WIN_LINE"
activate_pid "$PID" "pre-browse Ghostboard activation"

MODE_START="$(line_count "$APP_LOG")"
FOCUS_START="$(line_count "$SURFARI_TRACE")"
swift "$ROOT/scripts/ghostty-app/inject.swift" key 36 >>"$HARNESS_LOG" 2>&1
wait_for_file_pattern_after "$APP_LOG" "$MODE_START" "ModeChanged: pane_id=${PANE_ID} browsing=true" "webtui entered Browse mode"
wait_for_file_pattern_after "$SURFARI_TRACE" "$FOCUS_START" "focus-changed tab=${BROWSER_TAB_ID} pane=${PANE_ID} ffi=ts_set_focus focused=true" "Surfari focused after Browse mode"
activate_pid "$PID" "post-browse Ghostboard activation"

read -r FIELD_X FIELD_Y <<<"$(global_point_for_web_point "$WIN_LINE" "$PRESENTED_LINE" 120 75)"
read -r CLICK_X CLICK_Y <<<"$(global_point_for_web_point "$WIN_LINE" "$PRESENTED_LINE" 120 192)"
read -r DRAG_START_X DRAG_START_Y <<<"$(global_point_for_web_point "$WIN_LINE" "$PRESENTED_LINE" 65 266)"
read -r DRAG_END_X DRAG_END_Y <<<"$(global_point_for_web_point "$WIN_LINE" "$PRESENTED_LINE" 470 266)"
read -r SCROLL_X SCROLL_Y <<<"$(global_point_for_web_point "$WIN_LINE" "$PRESENTED_LINE" 260 360)"
log "field_point=${FIELD_X},${FIELD_Y}"
log "click_point=${CLICK_X},${CLICK_Y}"
log "drag_points=${DRAG_START_X},${DRAG_START_Y}-${DRAG_END_X},${DRAG_END_Y}"
log "scroll_point=${SCROLL_X},${SCROLL_Y}"

KEY_TRACE_START="$(line_count "$SURFARI_TRACE")"
KEY_STATE_START="$(line_count "$WEBTUI_TRACE")"
activate_pid "$PID" "pre-keyboard Ghostboard activation"
swift "$ROOT/scripts/ghostty-app/inject.swift" key 0 >>"$HARNESS_LOG" 2>&1
wait_for_file_pattern_after "$SURFARI_TRACE" "$KEY_TRACE_START" "key-event tab=${BROWSER_TAB_ID} pane=${PANE_ID} ffi=ts_forward_key_event type=down" "Surfari received keyboard events"
wait_for_file_pattern_after "$WEBTUI_TRACE" "$KEY_STATE_START" "event=console_message.*message=ISSUE756_EXP28_INPUT .*kind=input .*value=a .*active=field" "page received typed token"

CLICK_TRACE_START="$(line_count "$SURFARI_TRACE")"
CLICK_STATE_START="$(line_count "$WEBTUI_TRACE")"
swift "$ROOT/scripts/ghostty-app/inject.swift" click "$CLICK_X" "$CLICK_Y" left 1 >>"$HARNESS_LOG" 2>&1
wait_for_file_pattern_after "$SURFARI_TRACE" "$CLICK_TRACE_START" "mouse-event tab=${BROWSER_TAB_ID} pane=${PANE_ID} ffi=ts_forward_mouse_event type=down button=left" "Surfari received click-zone mouse event"
wait_for_file_pattern_after "$WEBTUI_TRACE" "$CLICK_STATE_START" "event=console_message.*message=ISSUE756_EXP28_INPUT .*kind=click .*detail=1" "page observed click-zone click"

SCROLL_TRACE_START="$(line_count "$SURFARI_TRACE")"
SCROLL_STATE_START="$(line_count "$WEBTUI_TRACE")"
swift "$ROOT/scripts/ghostty-app/inject.swift" scroll "$SCROLL_X" "$SCROLL_Y" -6 >>"$HARNESS_LOG" 2>&1
wait_for_file_pattern_after "$SURFARI_TRACE" "$SCROLL_TRACE_START" "scroll-event tab=${BROWSER_TAB_ID} pane=${PANE_ID} ffi=ts_forward_scroll_event" "Surfari received wheel input"
wait_for_file_pattern_after "$WEBTUI_TRACE" "$SCROLL_STATE_START" "event=console_message.*message=ISSUE756_EXP28_INPUT .*kind=wheel" "page observed wheel input"

DRAG_TRACE_START="$(line_count "$SURFARI_TRACE")"
DRAG_STATE_START="$(line_count "$WEBTUI_TRACE")"
swift "$ROOT/scripts/ghostty-app/inject.swift" drag "$DRAG_START_X" "$DRAG_START_Y" "$DRAG_END_X" "$DRAG_END_Y" >>"$HARNESS_LOG" 2>&1
wait_for_file_pattern_after "$SURFARI_TRACE" "$DRAG_TRACE_START" "mouse-event tab=${BROWSER_TAB_ID} pane=${PANE_ID} ffi=ts_forward_mouse_event type=down button=left" "Surfari received drag mouse down"
wait_for_file_pattern_after "$SURFARI_TRACE" "$DRAG_TRACE_START" "mouse-move tab=${BROWSER_TAB_ID} pane=${PANE_ID} ffi=ts_forward_mouse_move" "Surfari received drag mouse move"
wait_for_file_pattern_after "$SURFARI_TRACE" "$DRAG_TRACE_START" "mouse-event tab=${BROWSER_TAB_ID} pane=${PANE_ID} ffi=ts_forward_mouse_event type=up button=left" "Surfari received drag mouse up"
wait_for_file_pattern_after "$WEBTUI_TRACE" "$DRAG_STATE_START" "event=console_message.*message=ISSUE756_EXP28_INPUT .*kind=selection .*text=ISSUE756_EXP28_BROWSER_DRAG_TEXT" "page observed browser drag selection"

send_browser_close "$BROWSER_SOCKET" "$BROWSER_TAB_ID"
wait_for_file_pattern "$SURFARI_TRACE" "close-tab tab_id=${BROWSER_TAB_ID} result=removed" "Surfari accepted CloseTab"
wait_for_file_pattern "$SURFARI_TRACE" "close-tab result=no-tabs-remaining" "Surfari began clean shutdown"

log "PASS: issue 756 Surfari click and drag input details"
