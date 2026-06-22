#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RUN_ID="$(date +%Y%m%d-%H%M%S)"
LOG_DIR="$ROOT/logs/issue-756-exp20-real-app-surfari-smoke"
RUN_DIR="$(mktemp -d "${TMPDIR:-/tmp}/termsurf-issue756-exp20.XXXXXX")"
APP="${TERMSURF_GHOSTBOARD_APP:-$ROOT/ghostboard/macos/build/Debug/TermSurf.app}"
APP_BIN="$APP/Contents/MacOS/termsurf"
WEB="${TERMSURF_WEB:-$ROOT/target/debug/web}"
SURFARI="${TERMSURF_SURFARI:-$ROOT/target/debug/surfari}"
WEBKIT_DEBUG="${TERMSURF_WEBKIT_DEBUG:-$ROOT/webkit/src/WebKitBuild/Debug}"
CONFIG="$RUN_DIR/config"
COMMAND="$RUN_DIR/run-web-surfari.sh"
SITE_DIR="$RUN_DIR/site"
URL="file://$SITE_DIR/index.html"
APP_LOG="$LOG_DIR/app-$RUN_ID.log"
HARNESS_LOG="$LOG_DIR/harness-$RUN_ID.log"
SURFARI_TRACE="$LOG_DIR/surfari-trace-$RUN_ID.log"
WEBTUI_TRACE="$LOG_DIR/webtui-$RUN_ID.log"
SCREENSHOT="$LOG_DIR/screenshot-$RUN_ID.png"
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

try_wait_for_file_pattern() {
  local file="$1"
  local pattern="$2"
  local label="$3"
  local attempts="${4:-10}"
  for _ in $(seq 1 "$attempts"); do
    if grep -E "$pattern" "$file" >/dev/null 2>&1; then
      log "PASS: $label"
      return 0
    fi
    delay 1
  done
  log "WARN: missing $label"
  return 1
}

extract_first_match() {
  local file="$1"
  local pattern="$2"
  grep -E "$pattern" "$file" | head -1 || true
}

capture_window() {
  local line="$1"
  local wid
  wid="$(printf '%s\n' "$line" | sed -E 's/.*identity=window_id:([0-9]+).*/\1/')"
  case "$wid" in
    ''|*[!0-9]*) fail "could not extract window id from: $line" ;;
  esac
  screencapture -x -o -l"$wid" "$SCREENSHOT"
  [ -s "$SCREENSHOT" ] || fail "screenshot not written: $SCREENSHOT"
  log "PASS: screenshot=$SCREENSHOT"
}

overlay_center_from_presented() {
  local line="$1"
  python3 - "$ROOT" "$PID" "$line" <<'PY'
import os
import re
import subprocess
import sys

root, pid, line = sys.argv[1:4]
wid_match = re.search(r"identity=window_id:([0-9]+)", line)
if not wid_match:
    raise SystemExit("missing presented window id")
target_wid = wid_match.group(1)
env = dict(os.environ)
env["TS_LIST"] = "1"
listing = subprocess.check_output(
    ["swift", f"{root}/scripts/ghostty-app/winid.swift", pid],
    env=env,
    text=True,
)
wx = wy = None
for row in listing.splitlines():
    fields = row.split("\t")
    if len(fields) >= 6 and fields[0] == target_wid:
        size = fields[4]
        if "x" not in size:
            continue
        # winid list mode prints id, pid, owner, layer, size, onscreen. It does
        # not expose x/y, so ask the non-list path for the frontmost window only
        # if the exact presented id is also the selected candidate.
        selected = subprocess.check_output(
            ["swift", f"{root}/scripts/ghostty-app/winid.swift", pid],
            text=True,
        ).strip().split("\t")
        if len(selected) >= 5 and selected[0] == target_wid:
            _, wx, wy, _, _ = selected[:5]
        break
if wx is None:
    # The appkit overlay frame is in the target window's local top-left
    # coordinate space. In the common single-window harness case, posting to the
    # local overlay center reaches the visible window even when CGWindow list
    # coordinates cannot be recovered for the exact id.
    wx, wy = "0", "0"
m = re.search(r"overlay_frame=\{\{([0-9.-]+), ([0-9.-]+)\}, \{([0-9.-]+), ([0-9.-]+)\}\}", line)
if not m:
    raise SystemExit("missing overlay frame")
ox, oy, ow, oh = map(float, m.groups())
print(f"{float(wx) + ox + ow / 2:.0f}\t{float(wy) + oy + oh / 2:.0f}")
PY
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

cat >"$SITE_DIR/index.html" <<'EOF'
<!doctype html>
<meta charset="utf-8">
<title>Issue 756 Surfari Real App</title>
<style>
  body { font: 18px system-ui, sans-serif; margin: 32px; }
  main { min-height: 2200px; }
  input { font: inherit; padding: 8px; width: 360px; }
</style>
<main>
  <h1>ISSUE756_EXP20_SURFARI_REAL_APP</h1>
  <input id="field" value="ready">
  <p id="marker">Surfari real app smoke fixture.</p>
  <script>
    document.getElementById("field").focus();
    console.log("ISSUE756_EXP20_READY");
  </script>
</main>
EOF

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
log "app=$APP"
log "app_bin=$APP_BIN"
log "web=$WEB"
log "surfari=$SURFARI"
log "webkit_debug=$WEBKIT_DEBUG"
log "config=$CONFIG"
log "command=$COMMAND"
log "url=$URL"
log "app_log=$APP_LOG"
log "surfari_trace=$SURFARI_TRACE"
log "webtui_trace=$WEBTUI_TRACE"
log "screenshot=$SCREENSHOT"

GHOSTTY_CONFIG_PATH="$CONFIG" \
GHOSTTY_LOG=stderr \
DYLD_FRAMEWORK_PATH="$WEBKIT_DEBUG" \
TERMSURF_SURFARI_PATH="$SURFARI" \
TERMSURF_GEOMETRY_TRACE=1 \
TERMSURF_GEOMETRY_SCENARIO=issue756-exp20-real-app-surfari-smoke \
TERMSURF_WEBTUI_STATE_TRACE_FILE="$WEBTUI_TRACE" \
TERMSURF_INPUT_TRACE=1 \
TERMSURF_PDF_INPUT_TRACE=1 \
TERMSURF_PDF_INPUT_TRACE_FILE="$SURFARI_TRACE" \
  "$APP_BIN" >"$APP_LOG" 2>&1 &
PID="$!"
log "pid=$PID"

wait_for_file_pattern "$APP_LOG" "TermSurf message decoded type=HelloRequest" "web discovered TERMSURF_SOCKET"
wait_for_file_pattern "$APP_LOG" "SetOverlay: pane_id=.* profile=default browser=surfari url=${URL}" "web requested Surfari overlay"
wait_for_file_pattern "$APP_LOG" "SetOverlay: named browser resolved browser=surfari env=TERMSURF_SURFARI_PATH path=${SURFARI}" "Ghostboard resolved Surfari from env"
wait_for_file_pattern "$APP_LOG" "spawned browser path=${SURFARI} .* browser=surfari .*--browser-name=surfari .*--user-data-dir=.*webkit-profiles/default" "Ghostboard spawned Surfari with browser name and WebKit profile"
wait_for_file_pattern "$APP_LOG" "ServerRegister: profile=default browser=surfari" "Surfari registered browser identity"
wait_for_file_pattern "$APP_LOG" "sent CreateTab: pane_id=.* url=${URL}" "Ghostboard sent CreateTab"
wait_for_file_pattern "$APP_LOG" "BrowserReady: pane_id=.* browser=surfari" "Ghostboard emitted Surfari BrowserReady"
wait_for_file_pattern "$APP_LOG" "TermSurf geometry layer=appkit event=presented " "AppKit presented visible overlay"
wait_for_file_pattern "$WEBTUI_TRACE" "event=render_state.*browser_ready=true.*browser_label=surfari" "webtui rendered Surfari ready state"
wait_for_file_pattern "$SURFARI_TRACE" "trace-init pid=" "Surfari trace initialized with repo runtime env"
wait_for_file_pattern "$SURFARI_TRACE" "create-tab pane=.* url=${URL}" "Surfari created WebKit tab"
wait_for_file_pattern "$SURFARI_TRACE" "title-changed tab=.*title=Issue 756 Surfari Real App" "Surfari loaded deterministic page title"
wait_for_file_pattern "$SURFARI_TRACE" "ca-context tab=.*context_id=[1-9]" "Surfari exported CAContext"

PRESENTED_LINE="$(extract_first_match "$APP_LOG" "TermSurf geometry layer=appkit event=presented ")"
capture_window "$PRESENTED_LINE"
osascript -e 'tell application "TermSurf" to activate' >>"$HARNESS_LOG" 2>&1 || log "WARN: app activation failed"
delay 0.5
IFS=$'\t' read -r OVERLAY_X OVERLAY_Y < <(overlay_center_from_presented "$PRESENTED_LINE") || fail "failed to compute overlay center"
log "overlay_center=${OVERLAY_X},${OVERLAY_Y}"

if swift "$ROOT/scripts/ghostty-app/inject.swift" scroll "$OVERLAY_X" "$OVERLAY_Y" -6 >>"$HARNESS_LOG" 2>&1; then
  try_wait_for_file_pattern "$SURFARI_TRACE" "scroll-event tab=.*delta=\\(" "Surfari received scroll input" 10 || true
else
  log "WARN: scroll injection failed"
fi

if swift "$ROOT/scripts/ghostty-app/inject.swift" click "$OVERLAY_X" "$OVERLAY_Y" left 1 >>"$HARNESS_LOG" 2>&1 &&
  swift "$ROOT/scripts/ghostty-app/inject.swift" key 0 >>"$HARNESS_LOG" 2>&1; then
  try_wait_for_file_pattern "$SURFARI_TRACE" "key-event tab=.*type=down" "Surfari received keyboard input" 10 || true
else
  log "WARN: keyboard injection failed"
fi

RESIZE_START="$(wc -l <"$SURFARI_TRACE" | tr -d ' ')"
osascript -e "tell application \"System Events\" to set size of first window of application process \"TermSurf\" to {950, 720}" >>"$HARNESS_LOG" 2>&1 || log "WARN: window resize automation failed"
delay 2
if tail -n +"$((RESIZE_START + 1))" "$SURFARI_TRACE" | grep -E "resize tab_id=.*pixel_width=.*pixel_height=" >/dev/null 2>&1; then
  log "PASS: Surfari received resize after real app window resize"
else
  fail "resize evidence missing after automated window resize"
fi

BROWSER_READY_LINE="$(extract_first_match "$APP_LOG" "BrowserReady: pane_id=.* browser=surfari")"
BROWSER_SOCKET="$(printf '%s\n' "$BROWSER_READY_LINE" | sed -E 's/.*socket=([^ ]+) browser=surfari.*/\1/')"
BROWSER_TAB_ID="$(printf '%s\n' "$BROWSER_READY_LINE" | sed -E 's/.*tab_id=([0-9]+) socket=.*/\1/')"
case "$BROWSER_SOCKET" in
  /*) ;;
  *) fail "could not extract browser socket from BrowserReady: $BROWSER_READY_LINE" ;;
esac
case "$BROWSER_TAB_ID" in
  ''|*[!0-9]*) fail "could not extract tab id from BrowserReady: $BROWSER_READY_LINE" ;;
esac

send_browser_close "$BROWSER_SOCKET" "$BROWSER_TAB_ID"
wait_for_file_pattern "$SURFARI_TRACE" "close-tab tab_id=${BROWSER_TAB_ID} result=removed" "Surfari accepted CloseTab"
wait_for_file_pattern "$SURFARI_TRACE" "close-tab result=no-tabs-remaining" "Surfari began clean shutdown"

log "PASS: issue 756 experiment 20 real-app Surfari smoke"
