#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RUN_ID="$(date +%Y%m%d-%H%M%S)"
LOG_DIR="$ROOT/logs/issue-834-exp35-surfari-pdf-render"
RUN_DIR="$(mktemp -d "${TMPDIR:-/tmp}/termsurf-issue834-exp35.XXXXXX")"
APP="${TERMSURF_GHOSTBOARD_APP:-$ROOT/ghostboard/macos/build/Debug/TermSurf.app}"
APP_BIN="$APP/Contents/MacOS/termsurf"
WEB="${TERMSURF_WEB:-$ROOT/target/debug/web}"
SURFARI="${TERMSURF_SURFARI:-$ROOT/target/debug/surfari}"
WEBKIT_DEBUG="${TERMSURF_WEBKIT_DEBUG:-$ROOT/webkit/src/WebKitBuild/Debug}"
WEBKIT_DYLIB="$ROOT/surfari/libtermsurf_webkit/build/libtermsurf_webkit.dylib"
CONFIG="$RUN_DIR/config"
COMMAND="$RUN_DIR/run-web-surfari-pdf.sh"
SITE_DIR="$RUN_DIR/site"
SERVER_SCRIPT="$RUN_DIR/server.py"
PORT_FILE="$RUN_DIR/server-port.txt"
PDF_PATH="$SITE_DIR/surfari-render.pdf"
WEBKIT_PDF_FIXTURE="$ROOT/webkit/src/WebKitBuild/Debug/TestWebKitAPIResources.bundle/Contents/Resources/multiple-pages-colored.pdf"
APP_LOG="$LOG_DIR/app-$RUN_ID.log"
HARNESS_LOG="$LOG_DIR/harness-$RUN_ID.log"
SURFARI_TRACE="$LOG_DIR/surfari-trace-$RUN_ID.log"
WEBTUI_TRACE="$LOG_DIR/webtui-$RUN_ID.log"
SERVER_LOG="$LOG_DIR/server-$RUN_ID.log"
SCREENSHOT="$LOG_DIR/screenshot-$RUN_ID.png"
FULL_SCREENSHOT="$LOG_DIR/screenshot-full-$RUN_ID.png"
PIXEL_PROOF="$LOG_DIR/pixel-proof-$RUN_ID.json"
SUMMARY="$LOG_DIR/surfari-pdf-render-summary.json"
PID=""
SERVER_PID=""
URL=""
BROWSER_SOCKET=""
BROWSER_TAB_ID=""
PANE_ID=""
CA_CONTEXT_ID=""
PRESENTED_LINE=""
PIXELS_LINE=""
FIRST_FAILING_HOP="not-run"
CLEANUP_RAN="false"
CLEANUP_APP_STATUS="not-started"
CLEANUP_SERVER_STATUS="not-started"

mkdir -p "$LOG_DIR" "$SITE_DIR"

log() {
  printf '%s\n' "$*" | tee -a "$HARNESS_LOG"
}

delay() {
  sleep "${1:-0.5}"
}

json_summary() {
  local result="$1"
  local hop="$2"
  SUMMARY_RESULT="$result" \
  SUMMARY_HOP="$hop" \
  SUMMARY_RUN_ID="$RUN_ID" \
  SUMMARY_URL="$URL" \
  SUMMARY_PDF_PATH="$PDF_PATH" \
  SUMMARY_APP_LOG="$APP_LOG" \
  SUMMARY_HARNESS_LOG="$HARNESS_LOG" \
  SUMMARY_SURFARI_TRACE="$SURFARI_TRACE" \
  SUMMARY_WEBTUI_TRACE="$WEBTUI_TRACE" \
  SUMMARY_SERVER_LOG="$SERVER_LOG" \
  SUMMARY_SCREENSHOT="$SCREENSHOT" \
  SUMMARY_FULL_SCREENSHOT="$FULL_SCREENSHOT" \
  SUMMARY_PIXEL_PROOF="$PIXEL_PROOF" \
  SUMMARY_CA_CONTEXT_ID="$CA_CONTEXT_ID" \
  SUMMARY_PANE_ID="$PANE_ID" \
  SUMMARY_BROWSER_TAB_ID="$BROWSER_TAB_ID" \
  SUMMARY_BROWSER_SOCKET="$BROWSER_SOCKET" \
  SUMMARY="$SUMMARY" \
  python3 - <<'PY'
import json
import os
from pathlib import Path

def exists(path):
    return bool(path) and Path(path).exists()

pixel_path = os.environ["SUMMARY_PIXEL_PROOF"]
pixel_proof = None
if exists(pixel_path):
    pixel_proof = json.loads(Path(pixel_path).read_text())

data = {
    "overall_result": os.environ["SUMMARY_RESULT"],
    "first_failing_hop": os.environ["SUMMARY_HOP"],
    "run_id": os.environ["SUMMARY_RUN_ID"],
    "fixture": {
        "url": os.environ["SUMMARY_URL"],
        "path": os.environ["SUMMARY_PDF_PATH"],
        "exists": exists(os.environ["SUMMARY_PDF_PATH"]),
    },
    "artifacts": {
        "app_log": os.environ["SUMMARY_APP_LOG"],
        "harness_log": os.environ["SUMMARY_HARNESS_LOG"],
        "surfari_trace": os.environ["SUMMARY_SURFARI_TRACE"],
        "webtui_trace": os.environ["SUMMARY_WEBTUI_TRACE"],
        "server_log": os.environ["SUMMARY_SERVER_LOG"],
        "screenshot": os.environ["SUMMARY_SCREENSHOT"],
        "full_screenshot": os.environ["SUMMARY_FULL_SCREENSHOT"],
        "pixel_proof": pixel_path,
    },
    "ca_context": {
        "context_id": os.environ["SUMMARY_CA_CONTEXT_ID"],
        "nonzero": os.environ["SUMMARY_CA_CONTEXT_ID"].isdigit()
        and int(os.environ["SUMMARY_CA_CONTEXT_ID"]) > 0,
    },
    "pane_id": os.environ["SUMMARY_PANE_ID"],
    "browser": {
        "tab_id": os.environ["SUMMARY_BROWSER_TAB_ID"],
        "socket": os.environ["SUMMARY_BROWSER_SOCKET"],
    },
    "pixel_proof": pixel_proof,
    "cleanup": {
        "app_pid": os.environ.get("SUMMARY_APP_PID", ""),
        "server_pid": os.environ.get("SUMMARY_SERVER_PID", ""),
        "ran": os.environ.get("SUMMARY_CLEANUP_RAN", ""),
        "app_status": os.environ.get("SUMMARY_CLEANUP_APP_STATUS", ""),
        "server_status": os.environ.get("SUMMARY_CLEANUP_SERVER_STATUS", ""),
    },
}
Path(os.environ["SUMMARY"]).write_text(
    json.dumps(data, indent=2, sort_keys=True) + "\n",
    encoding="utf-8",
)
PY
}

fail() {
  FIRST_FAILING_HOP="$1"
  shift || true
  log "FAIL: $FIRST_FAILING_HOP${1:+: $*}"
  cleanup_processes || true
  export SUMMARY_CLEANUP_RAN="$CLEANUP_RAN"
  export SUMMARY_CLEANUP_APP_STATUS="$CLEANUP_APP_STATUS"
  export SUMMARY_CLEANUP_SERVER_STATUS="$CLEANUP_SERVER_STATUS"
  json_summary "fail" "$FIRST_FAILING_HOP" || true
  cleanup_files || true
  trap - EXIT
  exit 1
}

cleanup_processes() {
  CLEANUP_RAN="true"
  if [ -n "${PID:-}" ] && kill -0 "$PID" >/dev/null 2>&1; then
    kill "$PID" >/dev/null 2>&1 || true
    delay 0.5 || true
    kill -9 "$PID" >/dev/null 2>&1 || true
    CLEANUP_APP_STATUS="terminated"
  elif [ -n "${PID:-}" ]; then
    CLEANUP_APP_STATUS="not-running"
  fi
  if [ -n "${SERVER_PID:-}" ] && kill -0 "$SERVER_PID" >/dev/null 2>&1; then
    kill "$SERVER_PID" >/dev/null 2>&1 || true
    CLEANUP_SERVER_STATUS="terminated"
  elif [ -n "${SERVER_PID:-}" ]; then
    CLEANUP_SERVER_STATUS="not-running"
  fi
}

cleanup_files() {
  rm -rf "$RUN_DIR"
}

cleanup() {
  cleanup_processes || true
  cleanup_files || true
}
trap cleanup EXIT

require_executable() {
  [ -x "$1" ] || fail "missing-executable" "$1"
}

require_path() {
  [ -e "$1" ] || fail "missing-path" "$1"
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
  fail "timeout-$label" "pattern=$pattern file=$file"
}

wait_for_file_fixed() {
  local file="$1"
  local pattern="$2"
  local label="$3"
  local attempts="${4:-60}"
  for _ in $(seq 1 "$attempts"); do
    if grep -F "$pattern" "$file" >/dev/null 2>&1; then
      log "PASS: $label"
      return 0
    fi
    delay 1
  done
  fail "timeout-$label" "pattern=$pattern file=$file"
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

write_pdf_fixture() {
  cp "$WEBKIT_PDF_FIXTURE" "$PDF_PATH"
  [ -s "$PDF_PATH" ] || fail "pdf-fixture-missing" "$PDF_PATH"
}

start_server() {
  cat >"$SERVER_SCRIPT" <<'PY'
import http.server
import mimetypes
import pathlib
import socketserver
import sys

site = pathlib.Path(sys.argv[1])
port_file = pathlib.Path(sys.argv[2])

class Handler(http.server.SimpleHTTPRequestHandler):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, directory=str(site), **kwargs)

    def guess_type(self, path):
        if path.endswith(".pdf"):
            return "application/pdf"
        return mimetypes.guess_type(path)[0] or "application/octet-stream"

    def log_message(self, fmt, *args):
        print(fmt % args, flush=True)

with socketserver.TCPServer(("127.0.0.1", 0), Handler) as httpd:
    port_file.write_text(str(httpd.server_address[1]), encoding="utf-8")
    httpd.serve_forever()
PY
  python3 "$SERVER_SCRIPT" "$SITE_DIR" "$PORT_FILE" >"$SERVER_LOG" 2>&1 &
  SERVER_PID="$!"
  for _ in $(seq 1 50); do
    [ -s "$PORT_FILE" ] && return 0
    delay 0.1
  done
  fail "server-port-missing"
}

capture_window() {
  local line="$1"
  local wid
  wid="$(printf '%s\n' "$line" | sed -E 's/.*identity=window_id:([0-9]+).*/\1/')"
  case "$wid" in
    ''|*[!0-9]*) fail "window-id-missing" "$line" ;;
  esac
  screencapture -x -o -l"$wid" "$SCREENSHOT"
  [ -s "$SCREENSHOT" ] || fail "screenshot-missing" "$SCREENSHOT"
  log "PASS: screenshot=$SCREENSHOT"
  screencapture -x "$FULL_SCREENSHOT"
  [ -s "$FULL_SCREENSHOT" ] || fail "full-screenshot-missing" "$FULL_SCREENSHOT"
  log "PASS: full_screenshot=$FULL_SCREENSHOT"
}

pixel_proof() {
  python3 - "$SCREENSHOT" "$FULL_SCREENSHOT" "$PIXEL_PROOF" <<'PY'
from pathlib import Path
import json
import struct
import sys
import zlib

targets = {
    "webkit_green": (0, 128, 0),
}
threshold = 32

def read_png(path):
    png = Path(path).read_bytes()
    if png[:8] != b"\x89PNG\r\n\x1a\n":
        raise SystemExit(f"not png: {path}")
    pos = 8
    width = height = bit_depth = color_type = None
    chunks = []
    while pos < len(png):
        length = struct.unpack(">I", png[pos:pos + 4])[0]
        ctype = png[pos + 4:pos + 8]
        data = png[pos + 8:pos + 8 + length]
        pos += 12 + length
        if ctype == b"IHDR":
            width, height, bit_depth, color_type, _, _, _ = struct.unpack(">IIBBBBB", data)
        elif ctype == b"IDAT":
            chunks.append(data)
        elif ctype == b"IEND":
            break
    if bit_depth != 8 or color_type not in (2, 6):
        raise SystemExit(
            f"unsupported png path={path} bit_depth={bit_depth} color_type={color_type}"
        )
    bpp = 3 if color_type == 2 else 4
    raw = zlib.decompress(b"".join(chunks))
    stride = width * bpp
    rows = []
    i = 0
    prev = bytearray(stride)
    for _ in range(height):
        filt = raw[i]
        i += 1
        row = bytearray(raw[i:i + stride])
        i += stride
        for x in range(stride):
            left = row[x - bpp] if x >= bpp else 0
            up = prev[x]
            up_left = prev[x - bpp] if x >= bpp else 0
            if filt == 1:
                row[x] = (row[x] + left) & 0xFF
            elif filt == 2:
                row[x] = (row[x] + up) & 0xFF
            elif filt == 3:
                row[x] = (row[x] + ((left + up) // 2)) & 0xFF
            elif filt == 4:
                p = left + up - up_left
                pa = abs(p - left)
                pb = abs(p - up)
                pc = abs(p - up_left)
                predictor = left if pa <= pb and pa <= pc else up if pb <= pc else up_left
                row[x] = (row[x] + predictor) & 0xFF
            elif filt != 0:
                raise SystemExit(f"unsupported png filter={filt}")
        rows.append(row)
        prev = row
    return width, height, bpp, stride, rows

def classify(path):
    width, height, bpp, stride, rows = read_png(path)
    counts = {name: 0 for name in targets}
    for row in rows:
        for x in range(0, stride, bpp):
            rgb = tuple(row[x:x + 3])
            for name, target in targets.items():
                if sum(abs(rgb[channel] - target[channel]) for channel in range(3)) <= threshold:
                    counts[name] += 1
    total_hits = sum(counts.values())
    passed = all(value >= 5000 for value in counts.values()) and total_hits >= 5000
    return {
        "path": path,
        "status": "pass" if passed else "fail",
        "width": width,
        "height": height,
        "targets": counts,
        "total_hits": total_hits,
    }

window = classify(sys.argv[1])
full = classify(sys.argv[2])
passed = window["status"] == "pass" or full["status"] == "pass"
data = {
    "status": "pass" if passed else "fail",
    "method": "window" if window["status"] == "pass" else "full-screen" if full["status"] == "pass" else "none",
    "window": window,
    "full_screen": full,
    "threshold": threshold,
    "minimum_per_target": 5000,
    "minimum_total": 5000,
}
Path(sys.argv[3]).write_text(json.dumps(data, indent=2, sort_keys=True) + "\n")
if not passed:
    raise SystemExit(json.dumps(data, sort_keys=True))
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

require_executable "$APP_BIN"
require_executable "$WEB"
require_executable "$SURFARI"
require_path "$WEBKIT_DEBUG/WebKit.framework"
require_path "$WEBKIT_DYLIB"
require_path "$WEBKIT_PDF_FIXTURE"

write_pdf_fixture
start_server
PORT="$(cat "$PORT_FILE")"
URL="http://127.0.0.1:$PORT/surfari-render.pdf"

cat >"$COMMAND" <<EOF
#!/usr/bin/env bash
exec "$WEB" --browser surfari "$URL"
EOF
chmod +x "$COMMAND"

cat >"$CONFIG" <<EOF
window-save-state = never
initial-command = direct:$COMMAND
EOF

log "run_id=$RUN_ID"
log "app=$APP"
log "app_bin=$APP_BIN"
log "web=$WEB"
log "surfari=$SURFARI"
log "webkit_debug=$WEBKIT_DEBUG"
log "webkit_dylib=$WEBKIT_DYLIB"
log "config=$CONFIG"
log "command=$COMMAND"
log "url=$URL"
log "pdf=$PDF_PATH"
log "app_log=$APP_LOG"
log "surfari_trace=$SURFARI_TRACE"
log "webtui_trace=$WEBTUI_TRACE"
log "server_log=$SERVER_LOG"
log "screenshot=$SCREENSHOT"
log "full_screenshot=$FULL_SCREENSHOT"
log "summary=$SUMMARY"

GHOSTTY_CONFIG_PATH="$CONFIG" \
GHOSTTY_LOG=stderr \
DYLD_FRAMEWORK_PATH="$WEBKIT_DEBUG" \
TERMSURF_SURFARI_PATH="$SURFARI" \
TERMSURF_GEOMETRY_TRACE=1 \
TERMSURF_GEOMETRY_SCENARIO=issue834-exp35-surfari-pdf-render \
TERMSURF_WEBTUI_STATE_TRACE_FILE="$WEBTUI_TRACE" \
TERMSURF_INPUT_TRACE=1 \
TERMSURF_PDF_INPUT_TRACE=1 \
TERMSURF_PDF_INPUT_TRACE_FILE="$SURFARI_TRACE" \
  "$APP_BIN" >"$APP_LOG" 2>&1 &
PID="$!"
export SUMMARY_APP_PID="$PID"
export SUMMARY_SERVER_PID="$SERVER_PID"
log "pid=$PID"
log "server_pid=$SERVER_PID"

wait_for_file_pattern "$APP_LOG" "TermSurf message decoded type=HelloRequest" "web discovered TERMSURF_SOCKET"
wait_for_file_fixed "$APP_LOG" "browser=surfari url=$URL" "web requested Surfari PDF overlay"
wait_for_file_fixed "$APP_LOG" "env=TERMSURF_SURFARI_PATH path=$SURFARI" "Ghostboard resolved repo Surfari"
wait_for_file_pattern "$APP_LOG" "spawned browser path=${SURFARI} .* browser=surfari .*--browser-name=surfari .*--user-data-dir=.*webkit-profiles/default" "Ghostboard spawned repo Surfari"
wait_for_file_pattern "$APP_LOG" "ServerRegister: profile=default browser=surfari" "Surfari registered browser identity"
wait_for_file_fixed "$APP_LOG" "url=$URL" "Ghostboard sent PDF URL"
wait_for_file_pattern "$APP_LOG" "BrowserReady: pane_id=.* browser=surfari" "Ghostboard emitted Surfari BrowserReady"
wait_for_file_pattern "$APP_LOG" "TermSurf geometry layer=appkit event=presented " "AppKit presented Surfari overlay"
wait_for_file_pattern "$WEBTUI_TRACE" "event=render_state.*browser_ready=true.*browser_label=surfari" "WebTUI rendered Surfari ready state"
wait_for_file_pattern "$SURFARI_TRACE" "trace-init pid=" "Surfari trace initialized"
wait_for_file_fixed "$SURFARI_TRACE" "url=$URL" "Surfari trace recorded PDF URL"
wait_for_file_pattern "$SURFARI_TRACE" "ca-context tab=.*context_id=[1-9][0-9]*" "Surfari emitted nonzero CAContext"
try_wait_for_file_pattern "$SURFARI_TRACE" "loading-state-callback tab=.* state=done" "Surfari emitted PDF loading done" 20 || true
try_wait_for_file_pattern "$SURFARI_TRACE" "title-changed tab=.*(surfari-render|127\\.0\\.0\\.1|http)" "Surfari emitted PDF title" 15 || true
wait_for_file_fixed "$SERVER_LOG" '"GET /surfari-render.pdf HTTP/1.1" 200' "HTTP server served PDF"

BROWSER_READY_LINE="$(extract_first_match "$APP_LOG" "BrowserReady: pane_id=.* browser=surfari")"
PANE_ID="$(printf '%s\n' "$BROWSER_READY_LINE" | sed -E 's/.*pane_id=([^ ]+) tab_id=.*/\1/')"
BROWSER_SOCKET="$(printf '%s\n' "$BROWSER_READY_LINE" | sed -E 's/.*socket=([^ ]+) browser=surfari.*/\1/')"
BROWSER_TAB_ID="$(printf '%s\n' "$BROWSER_READY_LINE" | sed -E 's/.*tab_id=([0-9]+) socket=.*/\1/')"
case "$PANE_ID" in
  "") fail "pane-id-missing" "$BROWSER_READY_LINE" ;;
esac
case "$BROWSER_SOCKET" in
  "") fail "browser-socket-missing" "$BROWSER_READY_LINE" ;;
esac
case "$BROWSER_TAB_ID" in
  ''|*[!0-9]*) fail "browser-tab-id-missing" "$BROWSER_READY_LINE" ;;
esac
CA_LINE="$(extract_first_match "$SURFARI_TRACE" "ca-context tab=.*context_id=[1-9][0-9]*")"
CA_CONTEXT_ID="$(printf '%s\n' "$CA_LINE" | sed -E 's/.*context_id=([0-9]+).*/\1/')"
case "$CA_CONTEXT_ID" in
  ''|0|*[!0-9]*) fail "ca-context-invalid" "$CA_LINE" ;;
esac
wait_for_file_fixed "$APP_LOG" "pane_id:$PANE_ID" "AppKit identified Surfari pane"
wait_for_file_pattern "$APP_LOG" "TermSurf geometry layer=appkit event=presented_pixels .*pane_id:${PANE_ID} .*context_id=${CA_CONTEXT_ID} .*visible=true" "AppKit presented nonzero pixels"
PRESENTED_LINE="$(extract_first_match "$APP_LOG" "TermSurf geometry layer=appkit event=presented .*pane_id:${PANE_ID} .*context_id=${CA_CONTEXT_ID}")"
PIXELS_LINE="$(extract_first_match "$APP_LOG" "TermSurf geometry layer=appkit event=presented_pixels .*pane_id:${PANE_ID} .*context_id=${CA_CONTEXT_ID}")"

delay 2
capture_window "$PRESENTED_LINE"
if pixel_proof; then
  log "PASS: pixel_proof=$PIXEL_PROOF"
else
  fail "pdf-pixel-proof-failed" "$PIXEL_PROOF"
fi

send_browser_close "$BROWSER_SOCKET" "$BROWSER_TAB_ID"
wait_for_file_pattern "$SURFARI_TRACE" "close-tab tab_id=${BROWSER_TAB_ID} result=removed" "Surfari accepted CloseTab" 20
try_wait_for_file_pattern "$SURFARI_TRACE" "close-tab result=no-tabs-remaining" "Surfari began no-tabs shutdown" 10 || true

FIRST_FAILING_HOP="no-failure-observed"
cleanup_processes || true
export SUMMARY_CLEANUP_RAN="$CLEANUP_RAN"
export SUMMARY_CLEANUP_APP_STATUS="$CLEANUP_APP_STATUS"
export SUMMARY_CLEANUP_SERVER_STATUS="$CLEANUP_SERVER_STATUS"
json_summary "pass" "$FIRST_FAILING_HOP"
cleanup_files || true
trap - EXIT
log "PASS: issue 834 experiment 35 Surfari PDF render"
log "logs:"
log "  harness=$HARNESS_LOG"
log "  app_log=$APP_LOG"
log "  surfari_trace=$SURFARI_TRACE"
log "  webtui_trace=$WEBTUI_TRACE"
log "  server_log=$SERVER_LOG"
log "  screenshot=$SCREENSHOT"
log "  pixel_proof=$PIXEL_PROOF"
log "  summary=$SUMMARY"
