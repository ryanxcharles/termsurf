#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RUN_ID="$(date +%Y%m%d-%H%M%S)"
LOG_DIR="$ROOT/logs/issue-834-exp36-surfari-visual-compositing"
RUN_DIR="$(mktemp -d "${TMPDIR:-/tmp}/termsurf-issue834-exp36.XXXXXX")"
APP="${TERMSURF_GHOSTBOARD_APP:-$ROOT/ghostboard/macos/build/Debug/TermSurf.app}"
APP_BIN="$APP/Contents/MacOS/termsurf"
WEB="${TERMSURF_WEB:-$ROOT/target/debug/web}"
SURFARI="${TERMSURF_SURFARI:-$ROOT/target/debug/surfari}"
WEBKIT_DEBUG="${TERMSURF_WEBKIT_DEBUG:-$ROOT/webkit/src/WebKitBuild/Debug}"
WEBKIT_DYLIB="$ROOT/surfari/libtermsurf_webkit/build/libtermsurf_webkit.dylib"
WEBKIT_PDF_FIXTURE="$ROOT/webkit/src/WebKitBuild/Debug/TestWebKitAPIResources.bundle/Contents/Resources/multiple-pages-colored.pdf"
SITE_DIR="$RUN_DIR/site"
SERVER_SCRIPT="$RUN_DIR/server.py"
PORT_FILE="$RUN_DIR/server-port.txt"
SUMMARY="$LOG_DIR/surfari-visual-compositing-summary.json"
HARNESS_LOG="$LOG_DIR/harness-$RUN_ID.log"
SERVER_LOG="$LOG_DIR/server-$RUN_ID.log"
SERVER_PID=""
CURRENT_PID=""
SCENARIO_JSONS=()

mkdir -p "$LOG_DIR" "$SITE_DIR"

log() {
  printf '%s\n' "$*" | tee -a "$HARNESS_LOG"
}

delay() {
  sleep "${1:-0.5}"
}

fail() {
  log "FAIL: $*"
  cleanup_current_process || true
  cleanup_server || true
  rm -rf "$RUN_DIR"
  exit 1
}

require_executable() {
  [ -x "$1" ] || fail "missing executable: $1"
}

require_path() {
  [ -e "$1" ] || fail "missing path: $1"
}

cleanup_current_process() {
  if [ -n "${CURRENT_PID:-}" ] && kill -0 "$CURRENT_PID" >/dev/null 2>&1; then
    kill "$CURRENT_PID" >/dev/null 2>&1 || true
    delay 0.5 || true
    kill -9 "$CURRENT_PID" >/dev/null 2>&1 || true
  fi
}

cleanup_server() {
  if [ -n "${SERVER_PID:-}" ] && kill -0 "$SERVER_PID" >/dev/null 2>&1; then
    kill "$SERVER_PID" >/dev/null 2>&1 || true
  fi
}

cleanup() {
  cleanup_current_process || true
  cleanup_server || true
  rm -rf "$RUN_DIR"
}
trap cleanup EXIT

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
  fail "timeout-$label pattern=$pattern file=$file"
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
  fail "timeout-$label pattern=$pattern file=$file"
}

try_wait_for_file_pattern() {
  local file="$1"
  local pattern="$2"
  local label="$3"
  local attempts="${4:-15}"
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

write_fixtures() {
  cat >"$SITE_DIR/index.html" <<'EOF'
<!doctype html>
<meta charset="utf-8">
<title>Issue 834 Surfari HTML Visual Control</title>
<style>
  html,
  body {
    margin: 0;
    background: white;
  }
  .row {
    display: flex;
    gap: 48px;
    padding: 64px;
  }
  .box {
    width: 260px;
    height: 260px;
  }
  .magenta {
    background: #ff00ff;
  }
  .cyan {
    background: #00ffff;
  }
  .yellow {
    background: #ffff00;
  }
</style>
<main class="row" aria-label="ISSUE834_SURFARI_HTML_VISUAL_CONTROL">
  <div class="box magenta"></div>
  <div class="box cyan"></div>
  <div class="box yellow"></div>
</main>
EOF
  cp "$WEBKIT_PDF_FIXTURE" "$SITE_DIR/surfari-render.pdf"
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
  fail "server port missing"
}

capture_screenshots() {
  local presented_line="$1"
  local window_screenshot="$2"
  local full_screenshot="$3"
  local wid
  wid="$(printf '%s\n' "$presented_line" | sed -E 's/.*identity=window_id:([0-9]+).*/\1/')"
  case "$wid" in
    ''|*[!0-9]*) fail "missing window id from presented line: $presented_line" ;;
  esac
  screencapture -x -o -l"$wid" "$window_screenshot"
  [ -s "$window_screenshot" ] || fail "window screenshot missing: $window_screenshot"
  screencapture -x "$full_screenshot"
  [ -s "$full_screenshot" ] || fail "full screenshot missing: $full_screenshot"
}

pixel_proof() {
  local window_screenshot="$1"
  local full_screenshot="$2"
  local pixel_json="$3"
  local targets_json="$4"
  local minimum="$5"
  python3 - "$window_screenshot" "$full_screenshot" "$pixel_json" "$targets_json" "$minimum" <<'PY'
from pathlib import Path
import json
import struct
import sys
import zlib

window_path, full_path, out_path, targets_json, minimum = sys.argv[1:6]
targets = json.loads(targets_json)
minimum = int(minimum)
threshold = 40

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
    passed = all(value >= minimum for value in counts.values())
    return {
        "path": path,
        "status": "pass" if passed else "fail",
        "width": width,
        "height": height,
        "targets": counts,
        "total_hits": total_hits,
    }

window = classify(window_path)
full = classify(full_path)
passed = window["status"] == "pass" or full["status"] == "pass"
data = {
    "status": "pass" if passed else "fail",
    "method": "window" if window["status"] == "pass" else "full-screen" if full["status"] == "pass" else "none",
    "window": window,
    "full_screen": full,
    "threshold": threshold,
    "minimum_per_target": minimum,
}
Path(out_path).write_text(json.dumps(data, indent=2, sort_keys=True) + "\n")
if not passed:
    raise SystemExit(json.dumps(data, sort_keys=True))
PY
}

run_scenario() {
  local name="$1"
  local url="$2"
  local expected_path="$3"
  local title_pattern="$4"
  local targets_json="$5"
  local minimum="$6"
  local config="$RUN_DIR/config-$name"
  local command="$RUN_DIR/run-$name.sh"
  local app_log="$LOG_DIR/app-$name-$RUN_ID.log"
  local surfari_trace="$LOG_DIR/surfari-$name-$RUN_ID.log"
  local webtui_trace="$LOG_DIR/webtui-$name-$RUN_ID.log"
  local window_screenshot="$LOG_DIR/screenshot-$name-$RUN_ID.png"
  local full_screenshot="$LOG_DIR/screenshot-full-$name-$RUN_ID.png"
  local pixel_json="$LOG_DIR/pixel-proof-$name-$RUN_ID.json"
  local scenario_json="$LOG_DIR/scenario-$name-$RUN_ID.json"
  local browser_ready_line pane_id ca_line ca_context_id presented_line cleanup_status pixel_status

  cat >"$command" <<EOF
#!/usr/bin/env bash
exec "$WEB" --browser surfari "$url"
EOF
  chmod +x "$command"

  cat >"$config" <<EOF
window-save-state = never
initial-command = direct:$command
EOF

  log "scenario=$name"
  log "scenario_url=$url"
  log "scenario_app_log=$app_log"
  log "scenario_surfari_trace=$surfari_trace"
  log "scenario_webtui_trace=$webtui_trace"

  GHOSTTY_CONFIG_PATH="$config" \
  GHOSTTY_LOG=stderr \
  DYLD_FRAMEWORK_PATH="$WEBKIT_DEBUG" \
  TERMSURF_SURFARI_PATH="$SURFARI" \
  TERMSURF_GEOMETRY_TRACE=1 \
  TERMSURF_GEOMETRY_SCENARIO="issue834-exp36-surfari-visual-$name" \
  TERMSURF_WEBTUI_STATE_TRACE_FILE="$webtui_trace" \
  TERMSURF_INPUT_TRACE=1 \
  TERMSURF_PDF_INPUT_TRACE=1 \
  TERMSURF_PDF_INPUT_TRACE_FILE="$surfari_trace" \
    "$APP_BIN" >"$app_log" 2>&1 &
  CURRENT_PID="$!"

  wait_for_file_pattern "$app_log" "TermSurf message decoded type=HelloRequest" "$name web discovered TERMSURF_SOCKET"
  wait_for_file_fixed "$app_log" "browser=surfari url=$url" "$name web requested Surfari overlay"
  wait_for_file_fixed "$app_log" "env=TERMSURF_SURFARI_PATH path=$SURFARI" "$name resolved repo Surfari"
  wait_for_file_pattern "$app_log" "spawned browser path=${SURFARI} .* browser=surfari .*--browser-name=surfari .*--user-data-dir=.*webkit-profiles/default" "$name spawned repo Surfari"
  wait_for_file_pattern "$app_log" "ServerRegister: profile=default browser=surfari" "$name Surfari registered"
  wait_for_file_fixed "$app_log" "url=$url" "$name sent URL"
  wait_for_file_pattern "$app_log" "BrowserReady: pane_id=.* browser=surfari" "$name BrowserReady"
  wait_for_file_pattern "$webtui_trace" "event=render_state.*browser_ready=true.*browser_label=surfari" "$name WebTUI ready"
  wait_for_file_pattern "$surfari_trace" "trace-init pid=" "$name Surfari trace initialized"
  wait_for_file_fixed "$surfari_trace" "url=$url" "$name Surfari trace recorded URL"
  wait_for_file_pattern "$surfari_trace" "ca-context tab=.*context_id=[1-9][0-9]*" "$name nonzero CAContext"
  try_wait_for_file_pattern "$surfari_trace" "loading-state-callback tab=.* state=done" "$name loading done" 20 || true
  try_wait_for_file_pattern "$surfari_trace" "$title_pattern" "$name title evidence" 20 || true
  wait_for_file_fixed "$SERVER_LOG" "\"GET $expected_path HTTP/1.1\" 200" "$name HTTP served fixture"

  browser_ready_line="$(extract_first_match "$app_log" "BrowserReady: pane_id=.* browser=surfari")"
  pane_id="$(printf '%s\n' "$browser_ready_line" | sed -E 's/.*pane_id=([^ ]+) tab_id=.*/\1/')"
  ca_line="$(extract_first_match "$surfari_trace" "ca-context tab=.*context_id=[1-9][0-9]*")"
  ca_context_id="$(printf '%s\n' "$ca_line" | sed -E 's/.*context_id=([0-9]+).*/\1/')"
  wait_for_file_pattern "$app_log" "TermSurf geometry layer=appkit event=presented_pixels .*pane_id:${pane_id} .*context_id=${ca_context_id} .*visible=true" "$name AppKit presented pixels"
  presented_line="$(extract_first_match "$app_log" "TermSurf geometry layer=appkit event=presented .*pane_id:${pane_id} .*context_id=${ca_context_id}")"

  delay 2
  capture_screenshots "$presented_line" "$window_screenshot" "$full_screenshot"
  if pixel_proof "$window_screenshot" "$full_screenshot" "$pixel_json" "$targets_json" "$minimum"; then
    pixel_status="pass"
    log "PASS: $name pixel proof"
  else
    pixel_status="fail"
    log "WARN: $name pixel proof failed"
  fi

  cleanup_current_process || true
  cleanup_status="terminated"
  for _ in $(seq 1 20); do
    if [ -z "$CURRENT_PID" ] || ! kill -0 "$CURRENT_PID" >/dev/null 2>&1; then
      cleanup_status="terminated"
      break
    fi
    delay 0.1
  done
  if [ -n "$CURRENT_PID" ] && kill -0 "$CURRENT_PID" >/dev/null 2>&1; then
    cleanup_status="still-running"
  fi

  SCENARIO_NAME="$name" \
  SCENARIO_URL="$url" \
  SCENARIO_APP_LOG="$app_log" \
  SCENARIO_SURFARI_TRACE="$surfari_trace" \
  SCENARIO_WEBTUI_TRACE="$webtui_trace" \
  SCENARIO_WINDOW_SCREENSHOT="$window_screenshot" \
  SCENARIO_FULL_SCREENSHOT="$full_screenshot" \
  SCENARIO_PIXEL_JSON="$pixel_json" \
  SCENARIO_PIXEL_STATUS="$pixel_status" \
  SCENARIO_PANE_ID="$pane_id" \
  SCENARIO_CA_CONTEXT_ID="$ca_context_id" \
  SCENARIO_CLEANUP_STATUS="$cleanup_status" \
  SCENARIO_OUT="$scenario_json" \
  python3 - <<'PY'
import json
import os
from pathlib import Path

pixel_path = Path(os.environ["SCENARIO_PIXEL_JSON"])
pixel = json.loads(pixel_path.read_text()) if pixel_path.exists() else None
data = {
    "name": os.environ["SCENARIO_NAME"],
    "url": os.environ["SCENARIO_URL"],
    "pane_id": os.environ["SCENARIO_PANE_ID"],
    "ca_context_id": os.environ["SCENARIO_CA_CONTEXT_ID"],
    "pixel_status": os.environ["SCENARIO_PIXEL_STATUS"],
    "pixel_proof": pixel,
    "cleanup_status": os.environ["SCENARIO_CLEANUP_STATUS"],
    "artifacts": {
        "app_log": os.environ["SCENARIO_APP_LOG"],
        "surfari_trace": os.environ["SCENARIO_SURFARI_TRACE"],
        "webtui_trace": os.environ["SCENARIO_WEBTUI_TRACE"],
        "window_screenshot": os.environ["SCENARIO_WINDOW_SCREENSHOT"],
        "full_screenshot": os.environ["SCENARIO_FULL_SCREENSHOT"],
        "pixel_proof": os.environ["SCENARIO_PIXEL_JSON"],
    },
}
Path(os.environ["SCENARIO_OUT"]).write_text(json.dumps(data, indent=2, sort_keys=True) + "\n")
PY
  SCENARIO_JSONS+=("$scenario_json")
  CURRENT_PID=""
}

write_summary() {
  local native_snapshot="not-found"
  if rg -n "takeSnapshot|WKSnapshot|snapshot" "$ROOT/surfari" >/dev/null 2>&1; then
    native_snapshot="candidate-found"
  fi
  python3 - "$SUMMARY" "$RUN_ID" "$HARNESS_LOG" "$SERVER_LOG" "$native_snapshot" "${SCENARIO_JSONS[@]}" <<'PY'
from pathlib import Path
import json
import sys

summary_path = Path(sys.argv[1])
run_id = sys.argv[2]
harness_log = sys.argv[3]
server_log = sys.argv[4]
native_snapshot = sys.argv[5]
scenarios = [json.loads(Path(path).read_text()) for path in sys.argv[6:]]
by_name = {scenario["name"]: scenario for scenario in scenarios}
html = by_name.get("html", {})
pdf = by_name.get("pdf", {})
html_pass = html.get("pixel_status") == "pass"
pdf_pass = pdf.get("pixel_status") == "pass"

if html_pass and not pdf_pass:
    classification = "pdf-only-render-gap"
    overall = "pass"
elif not html_pass and not pdf_pass:
    classification = "generic-surfari-render-gap"
    overall = "pass"
elif html_pass and pdf_pass:
    classification = "no-visual-gap-detected"
    overall = "pass"
else:
    classification = "inconclusive"
    overall = "partial"

data = {
    "overall_result": overall,
    "classification": classification,
    "run_id": run_id,
    "native_snapshot_hook": native_snapshot,
    "artifacts": {
        "harness_log": harness_log,
        "server_log": server_log,
    },
    "html": html,
    "pdf": pdf,
    "cleanup": {
        "server_status": "terminated",
        "scenario_statuses": {
            scenario["name"]: scenario.get("cleanup_status") for scenario in scenarios
        },
    },
}
summary_path.write_text(json.dumps(data, indent=2, sort_keys=True) + "\n")
print(json.dumps({
    "overall_result": overall,
    "classification": classification,
    "native_snapshot_hook": native_snapshot,
    "html_pixel": html.get("pixel_status"),
    "pdf_pixel": pdf.get("pixel_status"),
}, indent=2, sort_keys=True))
PY
}

require_executable "$APP_BIN"
require_executable "$WEB"
require_executable "$SURFARI"
require_path "$WEBKIT_DEBUG/WebKit.framework"
require_path "$WEBKIT_DYLIB"
require_path "$WEBKIT_PDF_FIXTURE"

write_fixtures
start_server
PORT="$(cat "$PORT_FILE")"

log "run_id=$RUN_ID"
log "app_bin=$APP_BIN"
log "web=$WEB"
log "surfari=$SURFARI"
log "webkit_debug=$WEBKIT_DEBUG"
log "server_log=$SERVER_LOG"
log "summary=$SUMMARY"

run_scenario \
  "html" \
  "http://127.0.0.1:$PORT/index.html" \
  "/index.html" \
  "title-changed tab=.*title=Issue 834 Surfari HTML Visual Control" \
  '{"magenta":[255,0,255],"cyan":[0,255,255],"yellow":[255,255,0]}' \
  5000

run_scenario \
  "pdf" \
  "http://127.0.0.1:$PORT/surfari-render.pdf" \
  "/surfari-render.pdf" \
  "title-changed tab=.*title=.*" \
  '{"webkit_green":[0,128,0]}' \
  5000

cleanup_server || true
write_summary
rm -rf "$RUN_DIR"
trap - EXIT

log "PASS: issue 834 experiment 36 Surfari visual compositing diagnostics"
log "summary=$SUMMARY"
