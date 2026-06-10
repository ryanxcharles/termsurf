#!/usr/bin/env bash
# Issue 802 / Exp 39 — run the first live Ghostty vs Roastty screenshot smoke.
#
# Screenshots are written outside the repo:
#   ${TERMSURF_SHOT_DIR:-$HOME/.cache/termsurf/shots}
#
# Usage:
#   live-ab-smoke.sh [--recipe smoke|ascii-grid] [--max-mismatch-ratio N] [--max-mean-channel-delta N]
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
DIR="$ROOT/scripts/roastty-app"
GHOST_DIR="$ROOT/scripts/ghostty-app"
SHOT_DIR="${TERMSURF_SHOT_DIR:-$HOME/.cache/termsurf/shots}"
GHOSTTY_APP="${GHOSTTY_APP:-$ROOT/vendor/ghostty/macos/build/Debug/Ghostty.app}"
ROASTTY_APP="${ROASTTY_APP:-$ROOT/roastty/macos/build/Debug/Roastty.app}"
SWIFT="$(command -v swift || echo /usr/bin/swift)"

max_mismatch_ratio="0"
max_mean_channel_delta="0"
recipe="smoke"
list_recipes=0
while [ "$#" -gt 0 ]; do
  case "$1" in
    --recipe)
      recipe="${2:?missing value for --recipe}"
      shift 2
      ;;
    --list-recipes)
      list_recipes=1
      shift
      ;;
    --max-mismatch-ratio)
      max_mismatch_ratio="${2:?missing value for --max-mismatch-ratio}"
      shift 2
      ;;
    --max-mean-channel-delta)
      max_mean_channel_delta="${2:?missing value for --max-mean-channel-delta}"
      shift 2
      ;;
    -h|--help)
      echo "usage: $0 [--recipe smoke|ascii-grid] [--max-mismatch-ratio N] [--max-mean-channel-delta N]" >&2
      echo "       $0 --list-recipes" >&2
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      exit 2
      ;;
  esac
done

recipes=(smoke ascii-grid)
if [ "$list_recipes" -eq 1 ]; then
  printf '%s\n' "${recipes[@]}"
  exit 0
fi
case "$recipe" in
  smoke|ascii-grid) ;;
  *)
    echo "unknown recipe: $recipe" >&2
    echo "supported recipes:" >&2
    printf '  %s\n' "${recipes[@]}" >&2
    exit 2
    ;;
esac

mkdir -p "$SHOT_DIR"
stamp="$(date +%Y%m%d-%H%M%S)"
case "$recipe" in
  smoke) marker="ISSUE802_AB_SMOKE_$stamp" ;;
  *) marker="ISSUE802_AB_${recipe//-/_}_$stamp" ;;
esac
ghost_pid=""
roast_pid=""
cleanup_done=0

descendant_pids() {
  local root="$1"
  ps -axo pid=,ppid= | awk -v root="$root" '
    { parent[$1] = $2 }
    END {
      for (pid in parent) {
        current = pid
        while (current in parent && parent[current] != 0) {
          if (parent[current] == root) {
            print pid
            break
          }
          current = parent[current]
        }
      }
    }'
}

kill_launched_tree() {
  local label="$1"
  local pid="$2"
  local expected_path="$3"
  local command descendants
  [ -n "$pid" ] || return
  command="$(ps -p "$pid" -o command= 2>/dev/null || true)"
  if [ -z "$command" ]; then
    echo "$label pid $pid already stopped" >&2
    return
  fi
  if [[ "$command" != *"$expected_path"* ]]; then
    echo "refusing to kill $label pid $pid outside expected app path: $command" >&2
    return
  fi
  descendants="$(descendant_pids "$pid")"
  if [ -n "$descendants" ]; then
    echo "killing $label descendant PIDs: $descendants" >&2
    kill -9 $descendants 2>/dev/null || true
  fi
  echo "killing $label PID: $pid" >&2
  kill -9 "$pid" 2>/dev/null || true
}

cleanup() {
  if [ "$cleanup_done" -eq 1 ]; then
    return
  fi
  cleanup_done=1
  kill_launched_tree "Ghostty" "$ghost_pid" "$GHOSTTY_APP/Contents/MacOS/ghostty"
  kill_launched_tree "Roastty" "$roast_pid" "$ROASTTY_APP/Contents/MacOS/roastty"
}
trap cleanup EXIT INT TERM

delay() {
  osascript -e "delay ${1:-0.35}" >/dev/null 2>&1
}

activate() {
  local app="$1"
  osascript -e "tell application \"$app\" to activate" >/dev/null
  delay 0.8
}

set_window_bounds() {
  local app="$1"
  local process_name
  process_name="$(basename "$app" .app)"
  osascript >/dev/null <<OSA
tell application "System Events"
  tell process "$process_name"
    set position of front window to {40, 80}
    set size of front window to {800, 632}
  end tell
end tell
OSA
  delay 0.4
}

type_shell_command() {
  local app="$1"
  local command="$2"
  local tmp="/tmp/termsurf-ab-smoke-command-$$.txt"
  printf '%s' "$command" >"$tmp"
  activate "$app"
  osascript -e 'tell application "System Events" to key code 53' >/dev/null
  delay 0.2
  osascript -e "tell application \"System Events\" to keystroke (read POSIX file \"$tmp\")" >/dev/null
  osascript -e 'tell application "System Events" to key code 36' >/dev/null
  rm -f "$tmp"
  delay 1.0
}

recipe_command() {
  case "$recipe" in
    smoke)
      printf 'clear; echo %s' "$marker"
      ;;
    ascii-grid)
      printf "printf '\\033[2J\\033[H%s\\nABCDEFGHIJKLMNOPQRSTUVWXYZ\\nabcdefghijklmnopqrstuvwxyz\\n0123456789\\n@#$%%^&*()_+-=[]{};:,.<>/?\\n'; sleep 2" "$marker"
      ;;
  esac
}

image_dim() {
  local png="$1"
  local key="$2"
  sips -g "$key" "$png" 2>/dev/null | awk -v k="$key" '$1 == k || $1 == k ":" { print $2 }'
}

roastty_window_bounds() {
  local pid="$1"
  "$SWIFT" "$DIR/list-windows.swift" "$pid" |
    awk '/ name="👻"/ { print; found=1; exit } !found && /layer=0/ { candidate=$0 } END { if (!found && candidate != "") print candidate }'
}

crop_roastty_window() {
  local pid="$1"
  local full_png="$2"
  local out_png="$3"
  local target_w="${4:-}"
  local target_h="${5:-}"
  local line x y w h scale
  line="$(roastty_window_bounds "$pid")"
  [ -n "$line" ] || { echo "no Roastty window bounds found for pid $pid" >&2; return 1; }

  read -r x y w h < <(printf '%s\n' "$line" |
    sed -E 's/.*bounds=\(([0-9.-]+),([0-9.-]+) ([0-9.-]+)x([0-9.-]+)\).*/\1 \2 \3 \4/')
  for value in "$x" "$y" "$w" "$h"; do
    [[ "$value" =~ ^-?[0-9]+([.][0-9]+)?$ ]] || {
      echo "could not parse Roastty bounds: $line" >&2
      return 1
    }
  done

  scale="${TERMSURF_SCREEN_SCALE:-2}"
  read -r px py pw ph < <(awk \
    -v x="$x" -v y="$y" -v w="$w" -v h="$h" -v s="$scale" \
    -v tw="$target_w" -v th="$target_h" \
    'BEGIN {
      maxw = int(w*s); maxh = int(h*s);
      cw = tw == "" ? maxw : int(tw);
      ch = th == "" ? maxh : int(th);
      if (cw > maxw) cw = maxw;
      if (ch > maxh) ch = maxh;
      printf "%d %d %d %d\n", x*s, y*s, cw, ch
    }')
  "$SWIFT" "$DIR/crop.swift" "$full_png" "$out_png" "$px" "$py" "$pw" "$ph" >&2
}

echo "starting Ghostty and Roastty" >&2
ghost_pid="$("$GHOST_DIR/start-app.sh")"
roast_pid="$("$DIR/start-app.sh")"
echo "Ghostty pid=$ghost_pid Roastty pid=$roast_pid recipe=$recipe marker=$marker" >&2

activate "$GHOSTTY_APP"
set_window_bounds "$GHOSTTY_APP"
activate "$ROASTTY_APP"
set_window_bounds "$ROASTTY_APP"

command="$(recipe_command)"
type_shell_command "$GHOSTTY_APP" "$command"
type_shell_command "$ROASTTY_APP" "$command"

ghost_png="$("$GHOST_DIR/screenshot.sh" "$ghost_pid" "ghostty-ab-$stamp")"
ghost_w="$(image_dim "$ghost_png" pixelWidth)"
ghost_h="$(image_dim "$ghost_png" pixelHeight)"
[ -n "$ghost_w" ] && [ -n "$ghost_h" ] || {
  echo "could not read Ghostty capture dimensions: $ghost_png" >&2
  exit 1
}

activate "$ROASTTY_APP"
roast_full="$SHOT_DIR/roastty-ab-full-$stamp.png"
roast_crop="$SHOT_DIR/roastty-ab-crop-$stamp.png"
screencapture -x "$roast_full"
crop_roastty_window "$roast_pid" "$roast_full" "$roast_crop" "$ghost_w" "$ghost_h"

for png in "$ghost_png" "$roast_crop"; do
  [ -s "$png" ] || { echo "missing or empty capture: $png" >&2; exit 1; }
done

diff_args=(
  "$ghost_png"
  "$roast_crop"
  --max-mismatch-ratio "$max_mismatch_ratio"
  --max-mean-channel-delta "$max_mean_channel_delta"
)
diff_status=0
diff_json="$("$SWIFT" "$DIR/pngdiff.swift" "${diff_args[@]}")" || diff_status=$?

roast_w="$(image_dim "$roast_crop" pixelWidth)"
roast_h="$(image_dim "$roast_crop" pixelHeight)"
harness_verdict="FAIL"
[ "$diff_status" -eq 0 ] && harness_verdict="PASS"

python3 - "$harness_verdict" "$diff_status" "$ghost_pid" "$roast_pid" "$recipe" "$marker" \
  "$ghost_png" "$roast_full" "$roast_crop" "$ghost_w" "$ghost_h" "$roast_w" "$roast_h" \
  "$max_mismatch_ratio" "$max_mean_channel_delta" "$diff_json" <<'PY'
import json
import sys

(
    verdict,
    diff_status,
    ghost_pid,
    roast_pid,
    recipe,
    marker,
    ghost_png,
    roast_full,
    roast_crop,
    ghost_w,
    ghost_h,
    roast_w,
    roast_h,
    max_mismatch_ratio,
    max_mean_channel_delta,
    diff_json,
) = sys.argv[1:]

try:
    diff = json.loads(diff_json)
except json.JSONDecodeError:
    diff = {"error": "invalid_diff_json", "raw": diff_json}

summary = {
    "verdict": verdict,
    "recipe": recipe,
    "marker": marker,
    "ghostty_pid": int(ghost_pid),
    "roastty_pid": int(roast_pid),
    "ghostty_png": ghost_png,
    "roastty_full_png": roast_full,
    "roastty_crop_png": roast_crop,
    "ghostty_size": {"width": int(ghost_w or 0), "height": int(ghost_h or 0)},
    "roastty_crop_size": {"width": int(roast_w or 0), "height": int(roast_h or 0)},
    "max_mismatch_ratio": float(max_mismatch_ratio),
    "max_mean_channel_delta": float(max_mean_channel_delta),
    "diff_exit_status": int(diff_status),
    "diff": diff,
}
print(json.dumps(summary, sort_keys=True, separators=(",", ":")))
PY

exit "$diff_status"
