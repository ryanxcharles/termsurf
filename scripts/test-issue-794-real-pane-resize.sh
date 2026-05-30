#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LOG_DIR="${LOG_DIR:-"$ROOT/logs/issue-794-exp9-real-pane-resize-$(date +%Y%m%d-%H%M%S)"}"
case "$LOG_DIR" in
  /*) ;;
  *) LOG_DIR="$ROOT/$LOG_DIR" ;;
esac
PORT="${PORT:-9794}"
CLASS="${CLASS:-com.termsurf.issue-794-exp9-$$}"

WEZBOARD="$ROOT/wezboard/target/debug/wezboard-gui"
WEZBOARD_CLI="$ROOT/wezboard/target/debug/wezboard"
WEB="$ROOT/webtui/target/debug/web"
ROAMIUM="$ROOT/chromium/src/out/Default/roamium"
PDF_DIR="$ROOT/test-html/public"
PDF_URL="http://127.0.0.1:$PORT/bitcoin.pdf"
TRACE_FILE="$LOG_DIR/pdf-input.log"
RUNTIME_DIR="${XDG_RUNTIME_DIR:-"$HOME/.local/share"}/termsurf/wezboard"

mkdir -p "$LOG_DIR"

for bin in "$WEZBOARD" "$WEZBOARD_CLI" "$WEB" "$ROAMIUM"; do
  if [[ ! -x "$bin" ]]; then
    echo "missing executable: $bin" >&2
    exit 1
  fi
done

if [[ ! -f "$PDF_DIR/bitcoin.pdf" ]]; then
  echo "missing fixture: $PDF_DIR/bitcoin.pdf" >&2
  exit 1
fi

cat >"$LOG_DIR/commands.txt" <<EOF
LOG_DIR=$LOG_DIR
PORT=$PORT
CLASS=$CLASS
TRACE_FILE=$TRACE_FILE
WEZBOARD=$WEZBOARD
WEZBOARD_CLI=$WEZBOARD_CLI
WEB=$WEB
ROAMIUM=$ROAMIUM
PDF_URL=$PDF_URL
RUNTIME_DIR=$RUNTIME_DIR

Run this inside the debug Wezboard window:

TERMSURF_PDF_INPUT_TRACE=1 \\
TERMSURF_PDF_INPUT_TRACE_FILE="$TRACE_FILE" \\
"$WEB" \\
  --browser "$ROAMIUM" \\
  "$PDF_URL"

The runner starts an isolated debug Wezboard instance first, waits for its
TermSurf socket, then spawns this command automatically in that instance using:

WEZBOARD_UNIX_SOCKET="\$CLI_SOCKET" \\
  "$WEZBOARD_CLI" cli --no-auto-start spawn --window-id 0 -- \\
  /usr/bin/env \\
    TERMSURF_SOCKET="\$TERMSURF_SOCKET_PATH" \\
    TERMSURF_PDF_INPUT_TRACE=1 \\
    TERMSURF_PDF_INPUT_TRACE_FILE="$TRACE_FILE" \\
    "$WEB" --browser "$ROAMIUM" "$PDF_URL"

It then runs:

WEZBOARD_UNIX_SOCKET="\$CLI_SOCKET" "$WEZBOARD_CLI" cli --no-auto-start split-pane --bottom --percent 50 -- "$SHELL" -lc "sleep 30"
EOF

(
  cd "$PDF_DIR"
  python3 -m http.server "$PORT" --bind 127.0.0.1
) >"$LOG_DIR/fixture-server.log" 2>&1 &
SERVER_PID=$!

cleanup() {
  kill "$SERVER_PID" >/dev/null 2>&1 || true
  if [[ -n "${WEZBOARD_PID:-}" ]]; then
    kill "$WEZBOARD_PID" >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT

echo "fixture server pid=$SERVER_PID port=$PORT" | tee "$LOG_DIR/runner.log"
echo "wezboard class=$CLASS" | tee -a "$LOG_DIR/runner.log"
echo "trace file: $TRACE_FILE" | tee -a "$LOG_DIR/runner.log"
echo "commands: $LOG_DIR/commands.txt" | tee -a "$LOG_DIR/runner.log"

env -u TERMSURF_SOCKET -u TERMSURF_PANE_ID -u WEZBOARD_UNIX_SOCKET -u WEZBOARD_PANE \
  TERMSURF_PDF_INPUT_TRACE=1 \
  TERMSURF_PDF_INPUT_TRACE_FILE="$TRACE_FILE" \
"$WEZBOARD" start --always-new-process --class "$CLASS" -- \
  "$SHELL" -lc "sleep 60" \
  >"$LOG_DIR/wezboard.stdout" 2>"$LOG_DIR/wezboard.stderr" &
WEZBOARD_PID=$!
CLI_SOCKET="$RUNTIME_DIR/gui-sock-$WEZBOARD_PID"
TERMSURF_SOCKET_PATH="$(
  python3 - <<'PY' "$WEZBOARD_PID"
import os
import sys
import tempfile

print(os.path.join(tempfile.gettempdir(), "termsurf", f"wezboard-{sys.argv[1]}.sock"))
PY
)"

echo "wezboard pid=$WEZBOARD_PID" | tee -a "$LOG_DIR/runner.log"
echo "wezboard cli socket=$CLI_SOCKET" | tee -a "$LOG_DIR/runner.log"
echo "termsurf socket=$TERMSURF_SOCKET_PATH" | tee -a "$LOG_DIR/runner.log"
for _ in $(seq 1 60); do
  if [[ -S "$CLI_SOCKET" ]]; then
    break
  fi
  sleep 0.25
done

if [[ ! -S "$CLI_SOCKET" ]]; then
  echo "failed to discover debug wezboard CLI socket: $CLI_SOCKET" | tee -a "$LOG_DIR/runner.log"
  exit 1
fi

for _ in $(seq 1 60); do
  if [[ -S "$TERMSURF_SOCKET_PATH" ]]; then
    break
  fi
  sleep 0.25
done

if [[ ! -S "$TERMSURF_SOCKET_PATH" ]]; then
  echo "failed to discover debug TermSurf socket: $TERMSURF_SOCKET_PATH" | tee -a "$LOG_DIR/runner.log"
  exit 1
fi

echo "spawning web pane..." | tee -a "$LOG_DIR/runner.log"
PANE_ID="$(WEZBOARD_UNIX_SOCKET="$CLI_SOCKET" "$WEZBOARD_CLI" cli --no-auto-start spawn --window-id 0 -- \
  /usr/bin/env \
    TERMSURF_SOCKET="$TERMSURF_SOCKET_PATH" \
    TERMSURF_PDF_INPUT_TRACE=1 \
    TERMSURF_PDF_INPUT_TRACE_FILE="$TRACE_FILE" \
    "$WEB" --browser "$ROAMIUM" "$PDF_URL" \
  2>"$LOG_DIR/spawn-web.stderr" | tr -d '[:space:]')"

if [[ -z "$PANE_ID" || ! "$PANE_ID" =~ ^[0-9]+$ ]]; then
  echo "failed to spawn web pane; output='$PANE_ID'" | tee -a "$LOG_DIR/runner.log"
  exit 1
fi

echo "web pane id=$PANE_ID" | tee -a "$LOG_DIR/runner.log"
sleep 5

echo "capturing initial pane list..." | tee -a "$LOG_DIR/runner.log"
for _ in $(seq 1 60); do
  if WEZBOARD_UNIX_SOCKET="$CLI_SOCKET" "$WEZBOARD_CLI" cli --no-auto-start list --format json >"$LOG_DIR/cli-list-before.json" 2>"$LOG_DIR/cli-list-before.stderr"; then
    break
  fi
  sleep 0.5
done

if [[ ! -s "$LOG_DIR/cli-list-before.json" ]]; then
  echo "failed to capture initial pane list" | tee -a "$LOG_DIR/runner.log"
  exit 1
fi

echo "splitting pane..." | tee -a "$LOG_DIR/runner.log"
WEZBOARD_UNIX_SOCKET="$CLI_SOCKET" "$WEZBOARD_CLI" cli --no-auto-start split-pane --pane-id "$PANE_ID" --bottom --percent 50 -- \
  "$SHELL" -lc "sleep 30" \
  >"$LOG_DIR/split-pane.stdout" 2>"$LOG_DIR/split-pane.stderr"

cat "$LOG_DIR/split-pane.stdout" | tee -a "$LOG_DIR/runner.log"
sleep 5

WEZBOARD_UNIX_SOCKET="$CLI_SOCKET" "$WEZBOARD_CLI" cli --no-auto-start list --format json >"$LOG_DIR/cli-list-after.json" 2>"$LOG_DIR/cli-list-after.stderr" || true

echo "done. log dir: $LOG_DIR" | tee -a "$LOG_DIR/runner.log"
echo "stopping wezboard..." | tee -a "$LOG_DIR/runner.log"
kill "$WEZBOARD_PID" >/dev/null 2>&1 || true

wait "$WEZBOARD_PID" || true
