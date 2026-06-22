#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RUN_ID="$(date +%Y%m%d-%H%M%S)"
LOG_DIR="$ROOT/logs/issue-756-exp30-surfari-crash-handling"
RUN_DIR="$(mktemp -d "${TMPDIR:-/tmp}/termsurf-issue756-exp30.XXXXXX")"
APP="${TERMSURF_GHOSTBOARD_APP:-$ROOT/ghostboard/macos/build/Debug/TermSurf.app}"
APP_BIN="$APP/Contents/MacOS/termsurf"
WEB="${TERMSURF_WEB:-$ROOT/target/debug/web}"
SURFARI="${TERMSURF_SURFARI:-$ROOT/target/debug/surfari}"
WEBKIT_DEBUG="${TERMSURF_WEBKIT_DEBUG:-$ROOT/webkit/src/WebKitBuild/Debug}"
SITE_DIR="$RUN_DIR/site"
CONFIG="$RUN_DIR/config"
COMMAND="$RUN_DIR/run-web-surfari.sh"
APP_LOG="$LOG_DIR/app-$RUN_ID.log"
HARNESS_LOG="$LOG_DIR/harness-$RUN_ID.log"
SURFARI_TRACE="$LOG_DIR/surfari-trace-$RUN_ID.log"
WEBTUI_TRACE="$LOG_DIR/webtui-$RUN_ID.log"
CRASH_URL="termsurf://issue756-exp30-renderer-crash"
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

extract_ready_tab_id() {
  printf '%s\n' "$1" | sed -E 's/.*tab_id=([0-9]+) socket=.*/\1/'
}

extract_ready_socket() {
  printf '%s\n' "$1" | sed -E 's/.* socket=([^ ]+) browser=.*/\1/'
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

cat >"$SITE_DIR/index.html" <<'EOF'
<!doctype html>
<html>
  <head>
    <meta charset="utf-8">
    <title>Issue 756 Crash Initial</title>
    <script>
      console.log("ISSUE756_EXP30_CRASH_INITIAL_READY");
      window.addEventListener("load", () => {
        document.title = "Issue 756 Crash Initial Ready";
      });
    </script>
  </head>
  <body>ISSUE756_EXP30_CRASH_INITIAL</body>
</html>
EOF

cat >"$SITE_DIR/recovery.html" <<'EOF'
<!doctype html>
<html>
  <head>
    <meta charset="utf-8">
    <title>Issue 756 Crash Recovery</title>
    <script>
      console.log("ISSUE756_EXP30_CRASH_RECOVERY");
      window.addEventListener("load", () => {
        document.body.dataset.issue756Recovery = "ready";
      });
    </script>
  </head>
  <body>ISSUE756_EXP30_CRASH_RECOVERY_BODY</body>
</html>
EOF

HTTP_PORT="$(python3 - <<'PY'
import socket

with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
    s.bind(("127.0.0.1", 0))
    print(s.getsockname()[1])
PY
)"
URL="http://127.0.0.1:${HTTP_PORT}/index.html"
RECOVERY_URL="http://127.0.0.1:${HTTP_PORT}/recovery.html"

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

require_fresh_app_bundle
require_executable "$WEB"
require_executable "$SURFARI"
require_path "$WEBKIT_DEBUG/WebKit.framework"
require_path "$ROOT/surfari/libtermsurf_webkit/build/libtermsurf_webkit.dylib"

log "run_id=$RUN_ID"
log "app_bin=$APP_BIN"
log "web=$WEB"
log "surfari=$SURFARI"
log "url=$URL"
log "crash_url=$CRASH_URL"
log "recovery_url=$RECOVERY_URL"
log "app_log=$APP_LOG"
log "surfari_trace=$SURFARI_TRACE"
log "webtui_trace=$WEBTUI_TRACE"

GHOSTTY_CONFIG_PATH="$CONFIG" \
GHOSTTY_LOG=stderr \
DYLD_FRAMEWORK_PATH="$WEBKIT_DEBUG" \
TERMSURF_SURFARI_PATH="$SURFARI" \
TERMSURF_SURFARI_TEST_RENDERER_CRASH_URL="$CRASH_URL" \
TERMSURF_GEOMETRY_TRACE=1 \
TERMSURF_GEOMETRY_SCENARIO=issue756-exp30-surfari-crash-handling \
TERMSURF_WEBTUI_STATE_TRACE_FILE="$WEBTUI_TRACE" \
TERMSURF_INPUT_TRACE=1 \
TERMSURF_PDF_INPUT_TRACE=1 \
TERMSURF_PDF_INPUT_TRACE_FILE="$SURFARI_TRACE" \
  "$APP_BIN" >"$APP_LOG" 2>&1 &
PID="$!"
log "pid=$PID"

APP_START="$(line_count "$APP_LOG")"
TRACE_START="$(line_count "$SURFARI_TRACE")"
STATE_START="$(line_count "$WEBTUI_TRACE")"

wait_for_file_pattern_after "$APP_LOG" "$APP_START" "BrowserReady: pane_id=.* browser=surfari" "Surfari BrowserReady"
wait_for_file_pattern_after "$WEBTUI_TRACE" "$STATE_START" "event=url_changed[[:space:]]+url=${URL}" "webtui initial URL"
wait_for_file_pattern_after "$WEBTUI_TRACE" "$STATE_START" "event=title_changed[[:space:]]+title=Issue 756 Crash Initial Ready" "webtui initial title"
wait_for_file_pattern_after "$WEBTUI_TRACE" "$STATE_START" "event=console_message.*message=ISSUE756_EXP30_CRASH_INITIAL_READY" "webtui initial console marker"

READY_LINE="$(tail -n +"$((APP_START + 1))" "$APP_LOG" | grep -E "BrowserReady: pane_id=.* browser=surfari" | tail -1)"
PANE_ID="$(extract_pane_id "$READY_LINE")"
BROWSER_TAB_ID="$(extract_ready_tab_id "$READY_LINE")"
BROWSER_SOCKET="$(extract_ready_socket "$READY_LINE")"
[ -n "$PANE_ID" ] || fail "failed to extract Surfari pane id"
[ -n "$BROWSER_TAB_ID" ] || fail "failed to extract Surfari browser tab id"
[ -n "$BROWSER_SOCKET" ] || fail "failed to extract Surfari browser socket"
log "pane_id=$PANE_ID"
log "browser_tab_id=$BROWSER_TAB_ID"
log "browser_socket=$BROWSER_SOCKET"

NO_RESPAWN_APP_START="$(line_count "$APP_LOG")"
CRASH_TRACE_START="$(line_count "$SURFARI_TRACE")"
CRASH_STATE_START="$(line_count "$WEBTUI_TRACE")"
log "crash_trigger_navigate=$CRASH_URL"
send_browser_navigate "$BROWSER_SOCKET" "$BROWSER_TAB_ID" "$CRASH_URL"

wait_for_file_pattern_after "$SURFARI_TRACE" "$CRASH_TRACE_START" "test-renderer-crash tab=${BROWSER_TAB_ID} pane=${PANE_ID} url=${CRASH_URL} ffi=ts_webkit_test_kill_web_content_process" "Surfari gated crash helper invoked"
wait_for_file_pattern_after "$SURFARI_TRACE" "$CRASH_TRACE_START" "renderer-crashed tab=${BROWSER_TAB_ID} pane=${PANE_ID} status=requested code=0 url=${URL} visible=true can_reload=true" "Surfari renderer crash event"
wait_for_file_pattern_after "$WEBTUI_TRACE" "$CRASH_STATE_START" "event=renderer_crashed.*tab_id=${BROWSER_TAB_ID}.*status=requested.*code=0.*url=${URL}.*can_reload=true" "webtui renderer crash state event"
wait_for_file_pattern_after "$WEBTUI_TRACE" "$CRASH_STATE_START" "event=render_state.*loading_bar_active=false.*renderer_crash_active=true.*renderer_crash_tab_id=${BROWSER_TAB_ID}.*renderer_crash_status=requested.*renderer_crash_can_reload=true" "webtui render state shows active reloadable crash without stuck loading"

CRASH_EVENT_LINE="$(
  grep -nE "event=renderer_crashed.*tab_id=${BROWSER_TAB_ID}.*status=requested.*code=0.*url=${URL}.*can_reload=true" "$WEBTUI_TRACE" |
    tail -1 |
    cut -d: -f1
)"
case "$CRASH_EVENT_LINE" in
  ''|*[!0-9]*) fail "could not locate webtui renderer crash event line" ;;
esac
delay 2
require_no_file_pattern_after "$WEBTUI_TRACE" "$CRASH_EVENT_LINE" "event=render_state.*renderer_crash_active=false" "stale post-crash events did not clear crash state before recovery"
require_no_file_pattern_after "$WEBTUI_TRACE" "$CRASH_EVENT_LINE" "event=loading_state.*state=loading" "stale post-crash events did not restart loading before recovery"

RECOVERY_TRACE_START="$(line_count "$SURFARI_TRACE")"
RECOVERY_STATE_START="$(line_count "$WEBTUI_TRACE")"
log "recovery_navigate=$RECOVERY_URL"
send_browser_navigate "$BROWSER_SOCKET" "$BROWSER_TAB_ID" "$RECOVERY_URL"

wait_for_file_pattern_after "$SURFARI_TRACE" "$RECOVERY_TRACE_START" "navigate tab=${BROWSER_TAB_ID} pane=${PANE_ID} url=${RECOVERY_URL} ffi=ts_load_url" "Surfari recovery navigation"
wait_for_file_pattern_after "$WEBTUI_TRACE" "$RECOVERY_STATE_START" "event=url_changed[[:space:]]+url=${RECOVERY_URL}" "webtui recovery URL"
wait_for_file_pattern_after "$WEBTUI_TRACE" "$RECOVERY_STATE_START" "event=title_changed[[:space:]]+title=Issue 756 Crash Recovery" "webtui recovery title"
wait_for_file_pattern_after "$WEBTUI_TRACE" "$RECOVERY_STATE_START" "event=console_message.*message=ISSUE756_EXP30_CRASH_RECOVERY" "webtui recovery console marker"
wait_for_file_pattern_after "$WEBTUI_TRACE" "$RECOVERY_STATE_START" "event=render_state.*title=Issue 756 Crash Recovery.*loading_bar_active=false.*renderer_crash_active=false.*latest_console=ISSUE756_EXP30_CRASH_RECOVERY" "webtui render state cleared crash after recovery"
wait_for_file_pattern_after "$SURFARI_TRACE" "$RECOVERY_TRACE_START" "title-changed tab=${BROWSER_TAB_ID} pane=${PANE_ID} title=Issue 756 Crash Recovery" "Surfari stayed alive through recovery"

if tail -n +"$((NO_RESPAWN_APP_START + 1))" "$APP_LOG" | grep -E "spawned browser path=.*profile=default browser=surfari" >/dev/null 2>&1; then
  fail "recovery spawned a new default Surfari process"
fi

log "PASS: Surfari crash handling real-app harness completed"
log "logs:"
log "  app=$APP_LOG"
log "  surfari_trace=$SURFARI_TRACE"
log "  webtui_trace=$WEBTUI_TRACE"
log "  harness=$HARNESS_LOG"
