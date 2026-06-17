#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SCENARIO="${1:-initial-open}"
TS="$(date +%Y%m%d-%H%M%S)"
LOG_DIR="$ROOT/logs"
RUN_DIR="$(mktemp -d "${TMPDIR:-/tmp}/termsurf-ghostboard-geometry-${SCENARIO}.XXXXXX")"
APP="${TERMSURF_GHOSTBOARD_APP:-$ROOT/ghostboard/macos/build/Debug/TermSurf.app}"
APP_BIN="$APP/Contents/MacOS/termsurf"
WEB="${TERMSURF_WEB:-$ROOT/target/debug/web}"
ROAMIUM="${TERMSURF_ROAMIUM:-$ROOT/chromium/src/out/Default/roamium}"
URL="${TERMSURF_GEOMETRY_URL:-https://example.com}"
URL_B="${TERMSURF_GEOMETRY_SECOND_URL:-https://example.org}"
URL_C="${TERMSURF_GEOMETRY_THIRD_URL:-https://example.net}"
APP_LOG="$LOG_DIR/ghostboard-geometry-${SCENARIO}-app-${TS}.log"
HARNESS_LOG="$LOG_DIR/ghostboard-geometry-${SCENARIO}-harness-${TS}.log"
SCREENSHOT="$LOG_DIR/ghostboard-geometry-${SCENARIO}-screenshot-${TS}.png"
SCREENSHOT_GROW="$LOG_DIR/ghostboard-geometry-${SCENARIO}-grow-screenshot-${TS}.png"
SCREENSHOT_SHRINK="$LOG_DIR/ghostboard-geometry-${SCENARIO}-shrink-screenshot-${TS}.png"
SCREENSHOT_SPLIT="$LOG_DIR/ghostboard-geometry-${SCENARIO}-split-screenshot-${TS}.png"
SCREENSHOT_ZOOM="$LOG_DIR/ghostboard-geometry-${SCENARIO}-zoom-screenshot-${TS}.png"
SCREENSHOT_UNZOOM="$LOG_DIR/ghostboard-geometry-${SCENARIO}-unzoom-screenshot-${TS}.png"
SCREENSHOT_CLOSE="$LOG_DIR/ghostboard-geometry-${SCENARIO}-close-screenshot-${TS}.png"
SCREENSHOT_TAB_NEW="$LOG_DIR/ghostboard-geometry-${SCENARIO}-new-tab-screenshot-${TS}.png"
SCREENSHOT_TAB_BACK="$LOG_DIR/ghostboard-geometry-${SCENARIO}-back-tab-screenshot-${TS}.png"
SCREENSHOT_TAB_BROWSER_B="$LOG_DIR/ghostboard-geometry-${SCENARIO}-browser-b-screenshot-${TS}.png"
SCREENSHOT_TAB_BROWSER_A_RESTORED="$LOG_DIR/ghostboard-geometry-${SCENARIO}-browser-a-restored-screenshot-${TS}.png"
SCREENSHOT_TAB_BROWSER_B_RESTORED="$LOG_DIR/ghostboard-geometry-${SCENARIO}-browser-b-restored-screenshot-${TS}.png"
SCREENSHOT_TAB_AFTER_CLOSE="$LOG_DIR/ghostboard-geometry-${SCENARIO}-after-close-screenshot-${TS}.png"
SCREENSHOT_WINDOW_B="$LOG_DIR/ghostboard-geometry-${SCENARIO}-window-b-screenshot-${TS}.png"
SCREENSHOT_WINDOW_C="$LOG_DIR/ghostboard-geometry-${SCENARIO}-window-c-screenshot-${TS}.png"
SCREENSHOT_WINDOW_A_RESTORED="$LOG_DIR/ghostboard-geometry-${SCENARIO}-window-a-restored-screenshot-${TS}.png"
SCREENSHOT_WINDOW_B_RESTORED="$LOG_DIR/ghostboard-geometry-${SCENARIO}-window-b-restored-screenshot-${TS}.png"
SCREENSHOT_DISPLAY_MOVED="$LOG_DIR/ghostboard-geometry-${SCENARIO}-moved-screenshot-${TS}.png"
SCREENSHOT_DISPLAY_RETURNED="$LOG_DIR/ghostboard-geometry-${SCENARIO}-returned-screenshot-${TS}.png"
SCREENSHOT_FULLSCREEN="$LOG_DIR/ghostboard-geometry-${SCENARIO}-fullscreen-screenshot-${TS}.png"
SCREENSHOT_UNFULLSCREEN="$LOG_DIR/ghostboard-geometry-${SCENARIO}-unfullscreen-screenshot-${TS}.png"
SCREENSHOT_MINIMIZE_RESTORED="$LOG_DIR/ghostboard-geometry-${SCENARIO}-minimize-restored-screenshot-${TS}.png"
SCREENSHOT_HIDE_RESTORED="$LOG_DIR/ghostboard-geometry-${SCENARIO}-hide-restored-screenshot-${TS}.png"
SCREENSHOT_FONT_INCREASE="$LOG_DIR/ghostboard-geometry-${SCENARIO}-font-increase-screenshot-${TS}.png"
SCREENSHOT_FONT_DECREASE="$LOG_DIR/ghostboard-geometry-${SCENARIO}-font-decrease-screenshot-${TS}.png"
SCREENSHOT_TUI_SHRINK="$LOG_DIR/ghostboard-geometry-${SCENARIO}-tui-shrink-screenshot-${TS}.png"
SCREENSHOT_TUI_RESET="$LOG_DIR/ghostboard-geometry-${SCENARIO}-tui-reset-screenshot-${TS}.png"
ROAMIUM_TRACE="$LOG_DIR/ghostboard-geometry-${SCENARIO}-roamium-${TS}.log"
SIBLING_ALIVE_COMMAND="$RUN_DIR/sibling-alive-command.txt"
SIBLING_FOCUS_COMMAND="$RUN_DIR/sibling-focus-command.txt"
BROWSER_FOCUS_COMMAND="$RUN_DIR/browser-focus-command.txt"
TUI_VIEWPORT_SHRINK_COMMAND="$RUN_DIR/tui-viewport-shrink-command.txt"
TUI_VIEWPORT_RESET_COMMAND="$RUN_DIR/tui-viewport-reset-command.txt"
NEW_TAB_COMMAND_LOG="$RUN_DIR/new-tab-command.log"
NEW_TAB_MARKER_COMMAND="$RUN_DIR/new-tab-marker-command.txt"
SECOND_BROWSER_COMMAND="$RUN_DIR/second-browser-command.txt"
THIRD_BROWSER_COMMAND="$RUN_DIR/third-browser-command.txt"
PID=""

mkdir -p "$LOG_DIR"

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

require_file() {
  [ -x "$1" ] || fail "missing executable: $1"
}

require_readable() {
  [ -r "$1" ] || fail "missing readable file: $1"
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

wait_for_log() {
  local pattern="$1"
  local label="$2"
  local attempts="${3:-30}"
  for _ in $(seq 1 "$attempts"); do
    if grep -E "$pattern" "$APP_LOG" >/dev/null 2>&1; then
      log "PASS: observed $label"
      return 0
    fi
    delay 1
  done
  fail "timed out waiting for $label"
}

require_log() {
  local pattern="$1"
  local label="$2"
  if grep -E "$pattern" "$APP_LOG" >/dev/null 2>&1; then
    log "PASS: $label"
  else
    fail "missing $label"
  fi
}

require_log_after() {
  local start_line="$1"
  local pattern="$2"
  local label="$3"
  if tail -n +"$((start_line + 1))" "$APP_LOG" | grep -E "$pattern" >/dev/null 2>&1; then
    log "PASS: $label"
  else
    fail "missing $label"
  fi
}

wait_for_log_after() {
  local start_line="$1"
  local pattern="$2"
  local label="$3"
  local attempts="${4:-30}"
  for _ in $(seq 1 "$attempts"); do
    if tail -n +"$((start_line + 1))" "$APP_LOG" | grep -E "$pattern" >/dev/null 2>&1; then
      log "PASS: $label"
      return 0
    fi
    delay 1
  done
  fail "timed out waiting for $label"
}

require_trace() {
  local needle="$1"
  local label="$2"
  if grep -F "$needle" "$ROAMIUM_TRACE" >/dev/null 2>&1; then
    log "PASS: $label"
  else
    fail "missing $label"
  fi
}

trace_line_count() {
  if [ -r "$ROAMIUM_TRACE" ]; then
    wc -l <"$ROAMIUM_TRACE" | tr -d ' '
  else
    printf '0\n'
  fi
}

require_trace_after() {
  local start_line="$1"
  local needle="$2"
  local label="$3"
  if tail -n +"$((start_line + 1))" "$ROAMIUM_TRACE" | grep -F "$needle" >/dev/null 2>&1; then
    log "PASS: $label"
  else
    fail "missing $label"
  fi
}

require_no_trace_after() {
  local start_line="$1"
  local needle="$2"
  local label="$3"
  if tail -n +"$((start_line + 1))" "$ROAMIUM_TRACE" | grep -F "$needle" >/dev/null 2>&1; then
    fail "$label"
  fi
  log "PASS: $label"
}

require_text() {
  local haystack="$1"
  local needle="$2"
  local label="$3"
  case "$haystack" in
    *"$needle"*) log "PASS: $label" ;;
    *) fail "missing $label" ;;
  esac
}

log_line_count() {
  wc -l <"$APP_LOG" | tr -d ' '
}

extract_appkit_pixel() {
  printf '%s\n' "$1" | sed -E 's/.*appkit_pixel=([^ ]+).*/\1/'
}

extract_selected_tab_id() {
  printf '%s\n' "$1" | sed -E 's/.*selected_tab_id:([^ ]+) .*/\1/'
}

extract_pane_id() {
  printf '%s\n' "$1" | sed -E 's/.*pane_id[:=]([^ ]+).*/\1/'
}

extract_surface_id() {
  printf '%s\n' "$1" | sed -E 's/.*surface_id:([^ ]+) .*/\1/'
}

extract_browser_tab_id() {
  printf '%s\n' "$1" | sed -E 's/.*browser_tab_id:([^ ]+) .*/\1/'
}

extract_context_id() {
  printf '%s\n' "$1" | sed -E 's/.*context_id=([^ ]+).*/\1/'
}

extract_grid() {
  printf '%s\n' "$1" | sed -E 's/.*grid=([^ ]+).*/\1/'
}

extract_grid_width() {
  printf '%s\n' "$1" | sed -E 's/^([0-9]+)x[0-9]+.*/\1/'
}

extract_grid_height() {
  printf '%s\n' "$1" | sed -E 's/^[0-9]+x([0-9]+).*/\1/'
}

extract_cell_size() {
  printf '%s\n' "$1" | sed -E 's/.*cell=([^ ]+).*/\1/'
}

extract_overlay_frame() {
  printf '%s\n' "$1" | sed -E 's/.*overlay_frame=(\{\{[^}]+\}, \{[^}]+\}\}).*/\1/'
}

extract_frame_size() {
  printf '%s\n' "$1" | sed -E 's/.*overlay_frame=\{\{[^}]+\}, \{([^,]+), ([^}]+)\}\}.*/\1x\2/'
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

extract_backing_scale() {
  printf '%s\n' "$1" | sed -E 's/.*backing_scale=([^ ]+).*/\1/'
}

appkit_pixel_from_geometry_line() {
  local line="$1"
  local frame_size
  local width
  local height
  local scale
  frame_size="$(extract_frame_size "$line")"
  width="$(pair_width "$frame_size")"
  height="$(pair_height "$frame_size")"
  scale="$(extract_backing_scale "$line")"
  awk -v width="$width" -v height="$height" -v scale="$scale" \
    'BEGIN { printf "%dx%d\n", int((width * scale) + 0.5), int((height * scale) + 0.5) }'
}

pair_width() {
  printf '%s\n' "$1" | awk -Fx '{print $1}'
}

pair_height() {
  printf '%s\n' "$1" | awk -Fx '{print $2}'
}

compare_pair() {
  local pair="$1"
  local ref="$2"
  local mode="$3"
  local width height ref_width ref_height
  width="$(pair_width "$pair")"
  height="$(pair_height "$pair")"
  ref_width="$(pair_width "$ref")"
  ref_height="$(pair_height "$ref")"
  awk \
    -v width="$width" \
    -v height="$height" \
    -v ref_width="$ref_width" \
    -v ref_height="$ref_height" \
    -v mode="$mode" \
    'BEGIN {
      if (mode == "gt") { exit !((width > ref_width) && (height > ref_height)) }
      if (mode == "lt") { exit !((width < ref_width) && (height < ref_height)) }
      exit 1
    }'
}

compare_split_right_pair() {
  local pair="$1"
  local ref="$2"
  local tolerance="$3"
  local width height ref_width ref_height
  width="$(pair_width "$pair")"
  height="$(pair_height "$pair")"
  ref_width="$(pair_width "$ref")"
  ref_height="$(pair_height "$ref")"
  awk \
    -v width="$width" \
    -v height="$height" \
    -v ref_width="$ref_width" \
    -v ref_height="$ref_height" \
    -v tolerance="$tolerance" \
    'BEGIN {
      delta = height - ref_height
      if (delta < 0) delta = -delta
      exit !((width < ref_width) && (delta <= tolerance))
    }'
}

compare_split_down_pair() {
  local pair="$1"
  local ref="$2"
  local tolerance="$3"
  local width height ref_width ref_height
  width="$(pair_width "$pair")"
  height="$(pair_height "$pair")"
  ref_width="$(pair_width "$ref")"
  ref_height="$(pair_height "$ref")"
  awk \
    -v width="$width" \
    -v height="$height" \
    -v ref_width="$ref_width" \
    -v ref_height="$ref_height" \
    -v tolerance="$tolerance" \
    'BEGIN {
      delta = width - ref_width
      if (delta < 0) delta = -delta
      exit !((height < ref_height) && (delta <= tolerance))
    }'
}

compare_split_right_resize_pair() {
  local pair="$1"
  local ref="$2"
  local tolerance="$3"
  local width height ref_width ref_height
  width="$(pair_width "$pair")"
  height="$(pair_height "$pair")"
  ref_width="$(pair_width "$ref")"
  ref_height="$(pair_height "$ref")"
  awk \
    -v width="$width" \
    -v height="$height" \
    -v ref_width="$ref_width" \
    -v ref_height="$ref_height" \
    -v tolerance="$tolerance" \
    'BEGIN {
      delta = height - ref_height
      if (delta < 0) delta = -delta
      exit !((width > ref_width) && (delta <= tolerance))
    }'
}

compare_split_right_equalize_pair() {
  local pair="$1"
  local target="$2"
  local unequal="$3"
  local tolerance="$4"
  local width height target_width target_height unequal_width
  width="$(pair_width "$pair")"
  height="$(pair_height "$pair")"
  target_width="$(pair_width "$target")"
  target_height="$(pair_height "$target")"
  unequal_width="$(pair_width "$unequal")"
  awk \
    -v width="$width" \
    -v height="$height" \
    -v target_width="$target_width" \
    -v target_height="$target_height" \
    -v unequal_width="$unequal_width" \
    -v tolerance="$tolerance" \
    'BEGIN {
      width_delta = width - target_width
      if (width_delta < 0) width_delta = -width_delta
      height_delta = height - target_height
      if (height_delta < 0) height_delta = -height_delta
      exit !((width < unequal_width) && (width_delta <= tolerance) && (height_delta <= tolerance))
    }'
}

compare_split_right_zoom_pair() {
  local pair="$1"
  local target="$2"
  local split="$3"
  local tolerance="$4"
  local width height target_width target_height split_width
  width="$(pair_width "$pair")"
  height="$(pair_height "$pair")"
  target_width="$(pair_width "$target")"
  target_height="$(pair_height "$target")"
  split_width="$(pair_width "$split")"
  awk \
    -v width="$width" \
    -v height="$height" \
    -v target_width="$target_width" \
    -v target_height="$target_height" \
    -v split_width="$split_width" \
    -v tolerance="$tolerance" \
    'BEGIN {
      width_delta = width - target_width
      if (width_delta < 0) width_delta = -width_delta
      height_delta = height - target_height
      if (height_delta < 0) height_delta = -height_delta
      exit !((width > split_width) && (width_delta <= tolerance) && (height_delta <= tolerance))
    }'
}

wait_for_appkit_pixels_after() {
  local start_line="$1"
  local ref_pixel="$2"
  local mode="$3"
  local label="$4"
  local attempts="${5:-30}"
  local line pixel
  for _ in $(seq 1 "$attempts"); do
    while IFS= read -r line; do
      pixel="$(extract_appkit_pixel "$line")"
      if [ -n "$pixel" ] && compare_pair "$pixel" "$ref_pixel" "$mode"; then
        printf '%s\n' "$line"
        return 0
      fi
    done < <(tail -n +"$((start_line + 1))" "$APP_LOG" | grep -E 'TermSurf geometry layer=appkit event=presented_pixels' || true)
    delay 1
  done
  fail "timed out waiting for $label"
}

wait_for_appkit_frame_after() {
  local start_line="$1"
  local ref_frame="$2"
  local mode="$3"
  local label="$4"
  local attempts="${5:-30}"
  local line frame_size
  for _ in $(seq 1 "$attempts"); do
    while IFS= read -r line; do
      frame_size="$(extract_frame_size "$line")"
      if [ -n "$frame_size" ] && [ "$frame_size" != "$line" ] && compare_pair "$frame_size" "$ref_frame" "$mode"; then
        printf '%s\n' "$line"
        return 0
      fi
    done < <(tail -n +"$((start_line + 1))" "$APP_LOG" | grep -E 'TermSurf geometry layer=appkit event=presented ' || true)
    delay 1
  done
  fail "timed out waiting for $label"
}

wait_for_split_right_frame_after() {
  local start_line="$1"
  local pane_id="$2"
  local context_id="$3"
  local ref_frame="$4"
  local label="$5"
  local attempts="${6:-30}"
  local line frame_size
  for _ in $(seq 1 "$attempts"); do
    while IFS= read -r line; do
      frame_size="$(extract_frame_size "$line")"
      if [ -n "$frame_size" ] && [ "$frame_size" != "$line" ] && compare_split_right_pair "$frame_size" "$ref_frame" 8; then
        printf '%s\n' "$line"
        return 0
      fi
    done < <(tail -n +"$((start_line + 1))" "$APP_LOG" | grep -E "TermSurf geometry layer=appkit event=presented .*pane_id:${pane_id} .*context_id=${context_id}" || true)
    delay 1
  done
  fail "timed out waiting for $label"
}

wait_for_split_right_pixels_after() {
  local start_line="$1"
  local pane_id="$2"
  local context_id="$3"
  local ref_pixel="$4"
  local label="$5"
  local attempts="${6:-30}"
  local line pixel
  for _ in $(seq 1 "$attempts"); do
    while IFS= read -r line; do
      pixel="$(extract_appkit_pixel "$line")"
      if [ -n "$pixel" ] && compare_split_right_pair "$pixel" "$ref_pixel" 16; then
        printf '%s\n' "$line"
        return 0
      fi
    done < <(tail -n +"$((start_line + 1))" "$APP_LOG" | grep -E "TermSurf geometry layer=appkit event=presented_pixels .*pane_id:${pane_id} .*context_id=${context_id}" || true)
    delay 1
  done
  fail "timed out waiting for $label"
}

wait_for_split_down_frame_after() {
  local start_line="$1"
  local pane_id="$2"
  local context_id="$3"
  local ref_frame="$4"
  local label="$5"
  local attempts="${6:-30}"
  local line frame_size
  for _ in $(seq 1 "$attempts"); do
    while IFS= read -r line; do
      frame_size="$(extract_frame_size "$line")"
      if [ -n "$frame_size" ] && [ "$frame_size" != "$line" ] && compare_split_down_pair "$frame_size" "$ref_frame" 8; then
        printf '%s\n' "$line"
        return 0
      fi
    done < <(tail -n +"$((start_line + 1))" "$APP_LOG" | grep -E "TermSurf geometry layer=appkit event=presented .*pane_id:${pane_id} .*context_id=${context_id}" || true)
    delay 1
  done
  fail "timed out waiting for $label"
}

wait_for_split_down_pixels_after() {
  local start_line="$1"
  local pane_id="$2"
  local context_id="$3"
  local ref_pixel="$4"
  local label="$5"
  local attempts="${6:-30}"
  local line pixel
  for _ in $(seq 1 "$attempts"); do
    while IFS= read -r line; do
      pixel="$(extract_appkit_pixel "$line")"
      if [ -n "$pixel" ] && compare_split_down_pair "$pixel" "$ref_pixel" 16; then
        printf '%s\n' "$line"
        return 0
      fi
    done < <(tail -n +"$((start_line + 1))" "$APP_LOG" | grep -E "TermSurf geometry layer=appkit event=presented_pixels .*pane_id:${pane_id} .*context_id=${context_id}" || true)
    delay 1
  done
  fail "timed out waiting for $label"
}

wait_for_split_right_resize_frame_after() {
  local start_line="$1"
  local pane_id="$2"
  local context_id="$3"
  local ref_frame="$4"
  local label="$5"
  local attempts="${6:-30}"
  local line frame_size
  for _ in $(seq 1 "$attempts"); do
    while IFS= read -r line; do
      frame_size="$(extract_frame_size "$line")"
      if [ -n "$frame_size" ] && [ "$frame_size" != "$line" ] && compare_split_right_resize_pair "$frame_size" "$ref_frame" 8; then
        printf '%s\n' "$line"
        return 0
      fi
    done < <(tail -n +"$((start_line + 1))" "$APP_LOG" | grep -E "TermSurf geometry layer=appkit event=presented .*pane_id:${pane_id} .*context_id=${context_id}" || true)
    delay 1
  done
  fail "timed out waiting for $label"
}

wait_for_split_right_resize_pixels_after() {
  local start_line="$1"
  local pane_id="$2"
  local context_id="$3"
  local ref_pixel="$4"
  local label="$5"
  local attempts="${6:-30}"
  local line pixel
  for _ in $(seq 1 "$attempts"); do
    while IFS= read -r line; do
      pixel="$(extract_appkit_pixel "$line")"
      if [ -n "$pixel" ] && compare_split_right_resize_pair "$pixel" "$ref_pixel" 16; then
        printf '%s\n' "$line"
        return 0
      fi
    done < <(tail -n +"$((start_line + 1))" "$APP_LOG" | grep -E "TermSurf geometry layer=appkit event=presented_pixels .*pane_id:${pane_id} .*context_id=${context_id}" || true)
    delay 1
  done
  fail "timed out waiting for $label"
}

wait_for_split_right_equalize_frame_after() {
  local start_line="$1"
  local pane_id="$2"
  local context_id="$3"
  local target_frame="$4"
  local unequal_frame="$5"
  local label="$6"
  local attempts="${7:-30}"
  local line frame_size
  for _ in $(seq 1 "$attempts"); do
    while IFS= read -r line; do
      frame_size="$(extract_frame_size "$line")"
      if [ -n "$frame_size" ] && [ "$frame_size" != "$line" ] && compare_split_right_equalize_pair "$frame_size" "$target_frame" "$unequal_frame" 8; then
        printf '%s\n' "$line"
        return 0
      fi
    done < <(tail -n +"$((start_line + 1))" "$APP_LOG" | grep -E "TermSurf geometry layer=appkit event=presented .*pane_id:${pane_id} .*context_id=${context_id}" || true)
    delay 1
  done
  fail "timed out waiting for $label"
}

wait_for_split_right_equalize_pixels_after() {
  local start_line="$1"
  local pane_id="$2"
  local context_id="$3"
  local target_pixel="$4"
  local unequal_pixel="$5"
  local label="$6"
  local attempts="${7:-30}"
  local line pixel
  for _ in $(seq 1 "$attempts"); do
    while IFS= read -r line; do
      pixel="$(extract_appkit_pixel "$line")"
      if [ -n "$pixel" ] && compare_split_right_equalize_pair "$pixel" "$target_pixel" "$unequal_pixel" 16; then
        printf '%s\n' "$line"
        return 0
      fi
    done < <(tail -n +"$((start_line + 1))" "$APP_LOG" | grep -E "TermSurf geometry layer=appkit event=presented_pixels .*pane_id:${pane_id} .*context_id=${context_id}" || true)
    delay 1
  done
  fail "timed out waiting for $label"
}

wait_for_split_right_zoom_frame_after() {
  local start_line="$1"
  local pane_id="$2"
  local context_id="$3"
  local target_frame="$4"
  local split_frame="$5"
  local label="$6"
  local attempts="${7:-30}"
  local line frame_size
  for _ in $(seq 1 "$attempts"); do
    while IFS= read -r line; do
      frame_size="$(extract_frame_size "$line")"
      if [ -n "$frame_size" ] && [ "$frame_size" != "$line" ] && compare_split_right_zoom_pair "$frame_size" "$target_frame" "$split_frame" 8; then
        printf '%s\n' "$line"
        return 0
      fi
    done < <(tail -n +"$((start_line + 1))" "$APP_LOG" | grep -E "TermSurf geometry layer=appkit event=presented .*pane_id:${pane_id} .*context_id=${context_id}" || true)
    delay 1
  done
  fail "timed out waiting for $label"
}

wait_for_split_right_zoom_pixels_after() {
  local start_line="$1"
  local pane_id="$2"
  local context_id="$3"
  local target_pixel="$4"
  local split_pixel="$5"
  local label="$6"
  local attempts="${7:-30}"
  local line pixel
  for _ in $(seq 1 "$attempts"); do
    while IFS= read -r line; do
      pixel="$(extract_appkit_pixel "$line")"
      if [ -n "$pixel" ] && compare_split_right_zoom_pair "$pixel" "$target_pixel" "$split_pixel" 16; then
        printf '%s\n' "$line"
        return 0
      fi
    done < <(tail -n +"$((start_line + 1))" "$APP_LOG" | grep -E "TermSurf geometry layer=appkit event=presented_pixels .*pane_id:${pane_id} .*context_id=${context_id}" || true)
    delay 1
  done
  fail "timed out waiting for $label"
}

wait_for_hit_after() {
  local start_line="$1"
  local context_id="$2"
  local label="$3"
  local attempts="${4:-30}"
  local line
  for _ in $(seq 1 "$attempts"); do
    line="$(tail -n +"$((start_line + 1))" "$APP_LOG" | grep -E "TermSurf geometry layer=appkit event=hit_test .*context_id=${context_id} .*hit=true .*web_point=\\{" | tail -1 || true)"
    if [ -n "$line" ]; then
      printf '%s\n' "$line"
      return 0
    fi
    delay 1
  done
  fail "timed out waiting for $label"
}

wait_for_negative_hit_after() {
  local start_line="$1"
  local context_id="$2"
  local label="$3"
  local mode="${4:-require-hit-false}"
  local attempts="${5:-30}"
  local line
  for _ in $(seq 1 "$attempts"); do
    if tail -n +"$((start_line + 1))" "$APP_LOG" | grep -E "TermSurf geometry layer=appkit event=hit_test .*context_id=${context_id} .*hit=true" >/dev/null 2>&1; then
      fail "$label routed to original browser context"
    fi
    line="$(tail -n +"$((start_line + 1))" "$APP_LOG" | grep -E "TermSurf geometry layer=appkit event=hit_test .*context_id=${context_id} .*hit=false" | tail -1 || true)"
    if [ -n "$line" ] && printf '%s\n' "$line" | grep -F 'overlay_frame={' >/dev/null 2>&1; then
      printf '%s\n' "$line"
      log "PASS: observed $label with explicit hit=false"
      return 0
    fi
    delay 1
  done
  if [ "$mode" = "allow-absent" ]; then
    log "PASS: $label did not route to original browser context"
  else
    fail "timed out waiting for $label explicit hit=false"
  fi
}

require_no_different_appkit_frame_after() {
  local start_line="$1"
  local pane_id="$2"
  local context_id="$3"
  local expected_frame="$4"
  local label="$5"
  local line
  line="$(tail -n +"$((start_line + 1))" "$APP_LOG" |
    grep -E "TermSurf geometry layer=appkit event=presented .*pane_id:${pane_id} .*context_id=${context_id}" |
    grep -Fv "overlay_frame=$expected_frame" |
    tail -1 || true)"
  if [ -n "$line" ]; then
    fail "$label changed frame: $line"
  fi
  log "PASS: $label"
}

require_no_different_appkit_pixels_after() {
  local start_line="$1"
  local pane_id="$2"
  local context_id="$3"
  local expected_pixels="$4"
  local label="$5"
  local line
  line="$(tail -n +"$((start_line + 1))" "$APP_LOG" |
    grep -E "TermSurf geometry layer=appkit event=presented_pixels .*pane_id:${pane_id} .*context_id=${context_id}" |
    grep -Fv "appkit_pixel=$expected_pixels" |
    tail -1 || true)"
  if [ -n "$line" ]; then
    fail "$label changed pixels: $line"
  fi
  log "PASS: $label"
}

wait_for_line_after() {
  local start_line="$1"
  local pattern="$2"
  local label="$3"
  local attempts="${4:-30}"
  local line
  for _ in $(seq 1 "$attempts"); do
    line="$(tail -n +"$((start_line + 1))" "$APP_LOG" |
      grep -E "$pattern" |
      tail -1 || true)"
    if [ -n "$line" ]; then
      printf '%s\n' "$line"
      return 0
    fi
    delay 1
  done
  fail "timed out waiting for $label"
}

wait_for_different_zig_event_after() {
  local start_line="$1"
  local event="$2"
  local old_pane_id="$3"
  local label="$4"
  local attempts="${5:-30}"
  local line
  for _ in $(seq 1 "$attempts"); do
    line="$(tail -n +"$((start_line + 1))" "$APP_LOG" |
      grep -E "TermSurf geometry layer=zig event=${event} " |
      grep -Fv "pane_id:${old_pane_id}" |
      tail -1 || true)"
    if [ -n "$line" ]; then
      printf '%s\n' "$line"
      return 0
    fi
    delay 1
  done
  fail "timed out waiting for $label"
}

wait_for_exact_appkit_frame_after() {
  local start_line="$1"
  local pane_id="$2"
  local context_id="$3"
  local expected_frame="$4"
  local label="$5"
  local attempts="${6:-30}"
  local line
  for _ in $(seq 1 "$attempts"); do
    line="$(tail -n +"$((start_line + 1))" "$APP_LOG" |
      grep -E "TermSurf geometry layer=appkit event=presented .*pane_id:${pane_id} .*context_id=${context_id}" |
      grep -F "overlay_frame=$expected_frame" |
      tail -1 || true)"
    if [ -n "$line" ]; then
      printf '%s\n' "$line"
      return 0
    fi
    delay 1
  done
  fail "timed out waiting for $label"
}

wait_for_exact_appkit_pixels_after() {
  local start_line="$1"
  local pane_id="$2"
  local context_id="$3"
  local expected_pixel="$4"
  local label="$5"
  local attempts="${6:-30}"
  local line
  for _ in $(seq 1 "$attempts"); do
    line="$(tail -n +"$((start_line + 1))" "$APP_LOG" |
      grep -E "TermSurf geometry layer=appkit event=presented_pixels .*pane_id:${pane_id} .*context_id=${context_id}" |
      grep -F "appkit_pixel=$expected_pixel" |
      tail -1 || true)"
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
  local expected_frame="$4"
  local label="$5"
  local attempts="${6:-30}"
  local line frame
  for _ in $(seq 1 "$attempts"); do
    while IFS= read -r line; do
      frame="$(extract_overlay_frame "$line")"
      if [ -n "$frame" ] && [ "$frame" != "$line" ] && [ "$frame" != "$expected_frame" ]; then
        printf '%s\n' "$line"
        return 0
      fi
    done < <(tail -n +"$((start_line + 1))" "$APP_LOG" |
      grep -E "TermSurf geometry layer=appkit event=presented .*pane_id:${pane_id} .*context_id=${context_id}" || true)
    delay 1
  done
  fail "timed out waiting for $label"
}

wait_for_changed_appkit_pixels_after() {
  local start_line="$1"
  local pane_id="$2"
  local context_id="$3"
  local expected_pixel="$4"
  local label="$5"
  local attempts="${6:-30}"
  local line pixel
  for _ in $(seq 1 "$attempts"); do
    while IFS= read -r line; do
      pixel="$(extract_appkit_pixel "$line")"
      if [ -n "$pixel" ] && [ "$pixel" != "$line" ] && [ "$pixel" != "$expected_pixel" ]; then
        printf '%s\n' "$line"
        return 0
      fi
    done < <(tail -n +"$((start_line + 1))" "$APP_LOG" |
      grep -E "TermSurf geometry layer=appkit event=presented_pixels .*pane_id:${pane_id} .*context_id=${context_id}" || true)
    delay 1
  done
  fail "timed out waiting for $label"
}

wait_for_selected_tab_change_after() {
  local start_line="$1"
  local selected_tab_id="$2"
  local label="$3"
  local attempts="${4:-30}"
  local line changed_id
  for _ in $(seq 1 "$attempts"); do
    while IFS= read -r line; do
      changed_id="$(extract_selected_tab_id "$line")"
      case "$changed_id" in
        ""|"$line"|"$selected_tab_id"|unknown:*|-1) ;;
        *)
          printf '%s\n' "$line"
          return 0
          ;;
      esac
    done < <(tail -n +"$((start_line + 1))" "$APP_LOG" |
      grep -E "TermSurf geometry layer=appkit event=.*selected_tab_id:" || true)
    delay 1
  done
  fail "timed out waiting for $label"
}

wait_for_selected_tab_id_after() {
  local start_line="$1"
  local selected_tab_id="$2"
  local label="$3"
  local attempts="${4:-30}"
  local line
  for _ in $(seq 1 "$attempts"); do
    line="$(tail -n +"$((start_line + 1))" "$APP_LOG" |
      grep -E "TermSurf geometry layer=appkit event=.*selected_tab_id:${selected_tab_id}" |
      tail -1 || true)"
    if [ -n "$line" ]; then
      printf '%s\n' "$line"
      return 0
    fi
    delay 1
  done
  fail "timed out waiting for $label"
}

window_bounds() {
  swift "$WINDOW_BOUNDS" "$WID"
}

window_bounds_for() {
  swift "$WINDOW_BOUNDS" "$1"
}

app_windows() {
  swift "$APP_WINDOWS" "$PID"
}

focus_window_bounds() {
  local bounds="$1"
  local bid bx by bw bh
  IFS=$'\t' read -r bid bx by bw bh <<<"$bounds"
  swift "$FOCUS_WINDOW" "$PID" "$bx" "$by" "$bw" "$bh"
}

set_window_size() {
  local width="$1"
  local height="$2"
  swift "$RESIZE_WINDOW" "$PID" 40 80 "$width" "$height"
}

click_window_center() {
  local bounds="$1"
  local label="$2"
  local bid bx by bw bh click_x click_y
  IFS=$'\t' read -r bid bx by bw bh <<<"$bounds"
  click_x=$((bx + bw / 2))
  click_y=$((by + bh / 2))
  log "${label}_input_point=${click_x},${click_y}"
  swift "$ROOT/scripts/ghostty-app/inject.swift" move "$click_x" "$click_y" >>"$HARNESS_LOG" 2>&1
  delay 0.25
  swift "$ROOT/scripts/ghostty-app/inject.swift" click "$click_x" "$click_y" left 1 >>"$HARNESS_LOG" 2>&1
}

click_global_point() {
  local x="$1"
  local y="$2"
  local label="$3"
  log "${label}_input_point=${x},${y}"
  swift "$ROOT/scripts/ghostty-app/inject.swift" move "$x" "$y" >>"$HARNESS_LOG" 2>&1
  delay 0.25
  swift "$ROOT/scripts/ghostty-app/inject.swift" click "$x" "$y" left 1 >>"$HARNESS_LOG" 2>&1
}

click_negative_global_point() {
  local x="$1"
  local y="$2"
  local label="$3"
  log "${label}_input_point=${x},${y}"
  swift "$ROOT/scripts/ghostty-app/inject.swift" move "$x" "$y" >>"$HARNESS_LOG" 2>&1
  delay 0.75
  NEGATIVE_HIT_START_LINE="$(log_line_count)"
  swift "$ROOT/scripts/ghostty-app/inject.swift" click "$x" "$y" left 1 >>"$HARNESS_LOG" 2>&1
}

case "$SCENARIO" in
  initial-open|window-resize|split-right|split-down|split-right-resize|split-right-equalize|split-right-zoom|split-right-close-sibling|split-right-close-browser-pane|split-right-focus-switch|new-terminal-tab-visibility|open-browser-in-new-tab|close-browser-tab|open-browser-in-new-window|multiple-windows-with-browsers|display-move-backing-scale|fullscreen-unfullscreen|minimize-hide-restore|font-size-cell-metrics|tui-overlay-resize-command) ;;
  *)
    fail "unsupported scenario: $SCENARIO"
    ;;
esac

require_file "$APP_BIN"
require_file "$WEB"
require_file "$ROAMIUM"
require_readable "$ROOT/scripts/ghostty-app/inject.swift"
require_readable "$ROOT/scripts/ghostty-app/winid.swift"

COMMAND="$RUN_DIR/run-web.sh"
CONFIG="$RUN_DIR/config"
WINDOW_BOUNDS="$RUN_DIR/window-bounds.swift"
APP_WINDOWS="$RUN_DIR/app-windows.swift"
DISPLAY_INVENTORY="$RUN_DIR/display-inventory.swift"
ACTIVATE_APP="$RUN_DIR/activate-app.swift"
FOCUS_WINDOW="$RUN_DIR/focus-window.swift"
FULLSCREEN_WINDOW="$RUN_DIR/fullscreen-window.swift"
MINIMIZE_WINDOW="$RUN_DIR/minimize-window.swift"
HIDE_APP="$RUN_DIR/hide-app.swift"
RESIZE_WINDOW="$RUN_DIR/resize-window.swift"
cat >"$COMMAND" <<EOF
#!/usr/bin/env bash
exec "$WEB" --browser "$ROAMIUM" "$URL"
EOF
chmod +x "$COMMAND"

if [ "$SCENARIO" = "new-terminal-tab-visibility" ] || [ "$SCENARIO" = "open-browser-in-new-tab" ] || [ "$SCENARIO" = "close-browser-tab" ] || [ "$SCENARIO" = "open-browser-in-new-window" ] || [ "$SCENARIO" = "multiple-windows-with-browsers" ]; then
  FIRST_RUN_MARKER="$RUN_DIR/first-web-ran"
  cat >"$COMMAND" <<EOF
#!/usr/bin/env bash
set -euo pipefail
if mkdir "$FIRST_RUN_MARKER" 2>/dev/null; then
  exec "$WEB" --browser "$ROAMIUM" "$URL"
fi
printf 'new-tab-command invocation pid=%s\\n' "\$\$" >>"$NEW_TAB_COMMAND_LOG"
exec /bin/zsh -f -c 'printf "ISSUE809_EXP12_NEW_TAB_READY\\n"; while :; do sleep 60; done'
EOF
  chmod +x "$COMMAND"
fi

cat >"$CONFIG" <<EOF
window-save-state = never
initial-command = direct:$COMMAND
EOF

if [ "$SCENARIO" = "new-terminal-tab-visibility" ] || [ "$SCENARIO" = "open-browser-in-new-tab" ] || [ "$SCENARIO" = "close-browser-tab" ]; then
  cat >>"$CONFIG" <<'EOF'
keybind = ctrl+t=new_tab
keybind = ctrl+1=goto_tab:1
keybind = ctrl+2=goto_tab:2
keybind = ctrl+p=previous_tab
keybind = ctrl+n=next_tab
EOF
fi

if [ "$SCENARIO" = "close-browser-tab" ]; then
  cat >>"$CONFIG" <<'EOF'
confirm-close-surface = false
keybind = ctrl+w=close_tab
EOF
fi

if [ "$SCENARIO" = "open-browser-in-new-window" ] || [ "$SCENARIO" = "multiple-windows-with-browsers" ]; then
  cat >>"$CONFIG" <<'EOF'
keybind = ctrl+b=new_window
EOF
fi

if [ "$SCENARIO" = "font-size-cell-metrics" ]; then
  cat >>"$CONFIG" <<'EOF'
keybind = ctrl+u=increase_font_size:2
keybind = ctrl+y=decrease_font_size:2
EOF
fi

if [ "$SCENARIO" = "split-right" ] || [ "$SCENARIO" = "split-right-resize" ] || [ "$SCENARIO" = "split-right-equalize" ] || [ "$SCENARIO" = "split-right-zoom" ] || [ "$SCENARIO" = "split-right-focus-switch" ]; then
  cat >>"$CONFIG" <<'EOF'
keybind = ctrl+d=new_split:right
EOF
fi

if [ "$SCENARIO" = "split-right-close-sibling" ]; then
  cat >>"$CONFIG" <<'EOF'
confirm-close-surface = false
keybind = ctrl+d=new_split:right
keybind = ctrl+k=close_surface
EOF
fi

if [ "$SCENARIO" = "split-right-close-browser-pane" ]; then
  cat >>"$CONFIG" <<'EOF'
confirm-close-surface = false
keybind = ctrl+d=new_split:right
keybind = ctrl+k=close_surface
EOF
fi

if [ "$SCENARIO" = "split-right-resize" ] || [ "$SCENARIO" = "split-right-equalize" ]; then
  cat >>"$CONFIG" <<'EOF'
keybind = ctrl+l=resize_split:right,20
EOF
fi

if [ "$SCENARIO" = "split-right-equalize" ]; then
  cat >>"$CONFIG" <<'EOF'
keybind = ctrl+e=equalize_splits
EOF
fi

if [ "$SCENARIO" = "split-right-zoom" ]; then
  cat >>"$CONFIG" <<'EOF'
keybind = ctrl+z=toggle_split_zoom
EOF
fi

if [ "$SCENARIO" = "split-down" ]; then
  cat >>"$CONFIG" <<'EOF'
keybind = ctrl+j=new_split:down
EOF
fi

cat >"$WINDOW_BOUNDS" <<'EOF'
import CoreGraphics
import Foundation

guard CommandLine.arguments.count == 2,
      let target = Int(CommandLine.arguments[1]),
      let info = CGWindowListCopyWindowInfo([.optionAll], kCGNullWindowID) as? [[String: Any]]
else {
    exit(2)
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
EOF

cat >"$APP_WINDOWS" <<'EOF'
import CoreGraphics
import Foundation

guard CommandLine.arguments.count == 2,
      let targetPID = Int(CommandLine.arguments[1]),
      let info = CGWindowListCopyWindowInfo([.optionAll], kCGNullWindowID) as? [[String: Any]]
else {
    exit(2)
}

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

cat >"$DISPLAY_INVENTORY" <<'EOF'
import AppKit
import Foundation

for screen in NSScreen.screens {
    let id = (screen.deviceDescription[NSDeviceDescriptionKey("NSScreenNumber")] as? NSNumber)?.intValue ?? -1
    let frame = screen.frame
    let visible = screen.visibleFrame
    let scale = screen.backingScaleFactor
    let isMain = screen == NSScreen.main
    print("\(id)\t\(Int(frame.origin.x))\t\(Int(frame.origin.y))\t\(Int(frame.size.width))\t\(Int(frame.size.height))\t\(Int(visible.origin.x))\t\(Int(visible.origin.y))\t\(Int(visible.size.width))\t\(Int(visible.size.height))\t\(scale)\t\(isMain ? "main" : "secondary")")
}
EOF

cat >"$ACTIVATE_APP" <<'EOF'
import AppKit
import Foundation

guard CommandLine.arguments.count == 2,
      let rawPID = Int32(CommandLine.arguments[1]),
      let app = NSRunningApplication(processIdentifier: pid_t(rawPID))
else {
    exit(2)
}

app.activate(options: [.activateAllWindows, .activateIgnoringOtherApps])
Thread.sleep(forTimeInterval: 0.5)
EOF

cat >"$FOCUS_WINDOW" <<'EOF'
import ApplicationServices
import Foundation

guard CommandLine.arguments.count == 6,
      let rawPID = Int32(CommandLine.arguments[1]),
      let targetX = Double(CommandLine.arguments[2]),
      let targetY = Double(CommandLine.arguments[3]),
      let targetWidth = Double(CommandLine.arguments[4]),
      let targetHeight = Double(CommandLine.arguments[5])
else {
    fputs("usage: focus-window.swift <pid> <x> <y> <width> <height>\n", stderr)
    exit(2)
}

guard AXIsProcessTrusted() else {
    fputs("accessibility permission is not trusted for window focus automation\n", stderr)
    exit(3)
}

func point(_ value: CFTypeRef?) -> CGPoint? {
    guard let value, CFGetTypeID(value) == AXValueGetTypeID() else { return nil }
    var result = CGPoint.zero
    guard AXValueGetValue(value as! AXValue, .cgPoint, &result) else { return nil }
    return result
}

func size(_ value: CFTypeRef?) -> CGSize? {
    guard let value, CFGetTypeID(value) == AXValueGetTypeID() else { return nil }
    var result = CGSize.zero
    guard AXValueGetValue(value as! AXValue, .cgSize, &result) else { return nil }
    return result
}

let app = AXUIElementCreateApplication(pid_t(rawPID))
var windowsValue: CFTypeRef?
let windowsResult = AXUIElementCopyAttributeValue(
    app,
    kAXWindowsAttribute as CFString,
    &windowsValue
)

guard windowsResult == .success,
      let windows = windowsValue as? [AXUIElement]
else {
    fputs("could not read target app windows: \(windowsResult.rawValue)\n", stderr)
    exit(4)
}

for window in windows {
    var positionValue: CFTypeRef?
    var sizeValue: CFTypeRef?
    guard AXUIElementCopyAttributeValue(window, kAXPositionAttribute as CFString, &positionValue) == .success,
          AXUIElementCopyAttributeValue(window, kAXSizeAttribute as CFString, &sizeValue) == .success,
          let position = point(positionValue),
          let windowSize = size(sizeValue)
    else { continue }

    let positionMatches = abs(Double(position.x) - targetX) <= 4 &&
        abs(Double(position.y) - targetY) <= 4
    let sizeMatches = abs(Double(windowSize.width) - targetWidth) <= 8 &&
        abs(Double(windowSize.height) - targetHeight) <= 8
    guard positionMatches && sizeMatches else { continue }

    _ = AXUIElementPerformAction(window, kAXRaiseAction as CFString)
    _ = AXUIElementSetAttributeValue(app, kAXFocusedWindowAttribute as CFString, window)
    _ = AXUIElementSetAttributeValue(window, kAXMainAttribute as CFString, kCFBooleanTrue)
    _ = AXUIElementSetAttributeValue(window, kAXFocusedAttribute as CFString, kCFBooleanTrue)
    Thread.sleep(forTimeInterval: 0.5)
    exit(0)
}

fputs("no matching accessibility window for bounds \(targetX),\(targetY) \(targetWidth)x\(targetHeight)\n", stderr)
exit(5)
EOF

cat >"$FULLSCREEN_WINDOW" <<'EOF'
import ApplicationServices
import Foundation

guard CommandLine.arguments.count == 7,
      let rawPID = Int32(CommandLine.arguments[1]),
      let targetX = Double(CommandLine.arguments[2]),
      let targetY = Double(CommandLine.arguments[3]),
      let targetWidth = Double(CommandLine.arguments[4]),
      let targetHeight = Double(CommandLine.arguments[5])
else {
    fputs("usage: fullscreen-window.swift <pid> <x> <y> <width> <height> <enter|exit|state>\n", stderr)
    exit(2)
}

let mode = CommandLine.arguments[6]
guard mode == "enter" || mode == "exit" || mode == "state" else {
    fputs("mode must be enter, exit, or state\n", stderr)
    exit(2)
}

guard AXIsProcessTrusted() else {
    fputs("accessibility permission is not trusted for fullscreen automation\n", stderr)
    exit(3)
}

func point(_ value: CFTypeRef?) -> CGPoint? {
    guard let value, CFGetTypeID(value) == AXValueGetTypeID() else { return nil }
    var result = CGPoint.zero
    guard AXValueGetValue(value as! AXValue, .cgPoint, &result) else { return nil }
    return result
}

func size(_ value: CFTypeRef?) -> CGSize? {
    guard let value, CFGetTypeID(value) == AXValueGetTypeID() else { return nil }
    var result = CGSize.zero
    guard AXValueGetValue(value as! AXValue, .cgSize, &result) else { return nil }
    return result
}

let app = AXUIElementCreateApplication(pid_t(rawPID))
var windowsValue: CFTypeRef?
let windowsResult = AXUIElementCopyAttributeValue(
    app,
    kAXWindowsAttribute as CFString,
    &windowsValue
)

guard windowsResult == .success,
      let windows = windowsValue as? [AXUIElement]
else {
    fputs("could not read target app windows: \(windowsResult.rawValue)\n", stderr)
    exit(4)
}

for window in windows {
    var positionValue: CFTypeRef?
    var sizeValue: CFTypeRef?
    guard AXUIElementCopyAttributeValue(window, kAXPositionAttribute as CFString, &positionValue) == .success,
          AXUIElementCopyAttributeValue(window, kAXSizeAttribute as CFString, &sizeValue) == .success,
          let position = point(positionValue),
          let windowSize = size(sizeValue)
    else { continue }

    let positionMatches = abs(Double(position.x) - targetX) <= 12 &&
        abs(Double(position.y) - targetY) <= 12
    let sizeMatches = abs(Double(windowSize.width) - targetWidth) <= 16 &&
        abs(Double(windowSize.height) - targetHeight) <= 16
    guard positionMatches && sizeMatches else { continue }

    var fullScreenValue: CFTypeRef?
    let stateResult = AXUIElementCopyAttributeValue(window, "AXFullScreen" as CFString, &fullScreenValue)
    let isFullscreen = stateResult == .success && ((fullScreenValue as? Bool) ?? false)
    if mode == "state" {
        print(isFullscreen ? "true" : "false")
        exit(0)
    }

    let target = mode == "enter"
    if isFullscreen != target {
        let setResult = AXUIElementSetAttributeValue(
            window,
            "AXFullScreen" as CFString,
            target ? kCFBooleanTrue : kCFBooleanFalse
        )
        guard setResult == .success else {
            fputs("failed to set AXFullScreen: \(setResult.rawValue)\n", stderr)
            exit(5)
        }
    }

    Thread.sleep(forTimeInterval: 1.5)
    var afterValue: CFTypeRef?
    let afterResult = AXUIElementCopyAttributeValue(window, "AXFullScreen" as CFString, &afterValue)
    guard afterResult == .success else {
        fputs("failed to read AXFullScreen after set: \(afterResult.rawValue)\n", stderr)
        exit(6)
    }
    print(((afterValue as? Bool) ?? false) ? "true" : "false")
    exit(0)
}

fputs("no matching accessibility window for bounds \(targetX),\(targetY) \(targetWidth)x\(targetHeight)\n", stderr)
exit(7)
EOF

cat >"$MINIMIZE_WINDOW" <<'EOF'
import ApplicationServices
import Foundation

guard CommandLine.arguments.count == 7,
      let rawPID = Int32(CommandLine.arguments[1]),
      let targetX = Double(CommandLine.arguments[2]),
      let targetY = Double(CommandLine.arguments[3]),
      let targetWidth = Double(CommandLine.arguments[4]),
      let targetHeight = Double(CommandLine.arguments[5])
else {
    fputs("usage: minimize-window.swift <pid> <x> <y> <width> <height> <minimize|restore|state>\n", stderr)
    exit(2)
}

let mode = CommandLine.arguments[6]
guard mode == "minimize" || mode == "restore" || mode == "state" else {
    fputs("mode must be minimize, restore, or state\n", stderr)
    exit(2)
}

guard AXIsProcessTrusted() else {
    fputs("accessibility permission is not trusted for minimize automation\n", stderr)
    exit(3)
}

func point(_ value: CFTypeRef?) -> CGPoint? {
    guard let value, CFGetTypeID(value) == AXValueGetTypeID() else { return nil }
    var result = CGPoint.zero
    guard AXValueGetValue(value as! AXValue, .cgPoint, &result) else { return nil }
    return result
}

func size(_ value: CFTypeRef?) -> CGSize? {
    guard let value, CFGetTypeID(value) == AXValueGetTypeID() else { return nil }
    var result = CGSize.zero
    guard AXValueGetValue(value as! AXValue, .cgSize, &result) else { return nil }
    return result
}

let app = AXUIElementCreateApplication(pid_t(rawPID))
var windowsValue: CFTypeRef?
let windowsResult = AXUIElementCopyAttributeValue(
    app,
    kAXWindowsAttribute as CFString,
    &windowsValue
)

guard windowsResult == .success,
      let windows = windowsValue as? [AXUIElement]
else {
    fputs("could not read target app windows: \(windowsResult.rawValue)\n", stderr)
    exit(4)
}

for window in windows {
    var positionValue: CFTypeRef?
    var sizeValue: CFTypeRef?
    guard AXUIElementCopyAttributeValue(window, kAXPositionAttribute as CFString, &positionValue) == .success,
          AXUIElementCopyAttributeValue(window, kAXSizeAttribute as CFString, &sizeValue) == .success,
          let position = point(positionValue),
          let windowSize = size(sizeValue)
    else { continue }

    let positionMatches = abs(Double(position.x) - targetX) <= 16 &&
        abs(Double(position.y) - targetY) <= 16
    let sizeMatches = abs(Double(windowSize.width) - targetWidth) <= 16 &&
        abs(Double(windowSize.height) - targetHeight) <= 16
    guard positionMatches && sizeMatches else { continue }

    var minimizedValue: CFTypeRef?
    let stateResult = AXUIElementCopyAttributeValue(window, kAXMinimizedAttribute as CFString, &minimizedValue)
    let isMinimized = stateResult == .success && ((minimizedValue as? Bool) ?? false)
    if mode == "state" {
        print(isMinimized ? "true" : "false")
        exit(0)
    }

    let target = mode == "minimize"
    if isMinimized != target {
        let setResult = AXUIElementSetAttributeValue(
            window,
            kAXMinimizedAttribute as CFString,
            target ? kCFBooleanTrue : kCFBooleanFalse
        )
        guard setResult == .success else {
            fputs("failed to set AXMinimized: \(setResult.rawValue)\n", stderr)
            exit(5)
        }
    }

    Thread.sleep(forTimeInterval: 1.0)
    var afterValue: CFTypeRef?
    let afterResult = AXUIElementCopyAttributeValue(window, kAXMinimizedAttribute as CFString, &afterValue)
    guard afterResult == .success else {
        fputs("failed to read AXMinimized after set: \(afterResult.rawValue)\n", stderr)
        exit(6)
    }
    print(((afterValue as? Bool) ?? false) ? "true" : "false")
    exit(0)
}

fputs("no matching accessibility window for bounds \(targetX),\(targetY) \(targetWidth)x\(targetHeight)\n", stderr)
exit(7)
EOF

cat >"$HIDE_APP" <<'EOF'
import AppKit
import Foundation

guard CommandLine.arguments.count == 3,
      let rawPID = Int32(CommandLine.arguments[1]),
      let app = NSRunningApplication(processIdentifier: pid_t(rawPID))
else {
    fputs("usage: hide-app.swift <pid> <hide|show>\n", stderr)
    exit(2)
}

let mode = CommandLine.arguments[2]
func setVisibleWithSystemEvents(_ visible: Bool) -> Bool {
    let process = Process()
    process.executableURL = URL(fileURLWithPath: "/usr/bin/osascript")
    let state = visible ? "true" : "false"
    process.arguments = [
        "-e",
        "tell application \"System Events\" to set visible of (first application process whose unix id is \(rawPID)) to \(state)"
    ]
    do {
        try process.run()
        process.waitUntilExit()
        return process.terminationStatus == 0
    } catch {
        return false
    }
}

if mode == "hide" {
    app.activate(options: [.activateAllWindows, .activateIgnoringOtherApps])
    Thread.sleep(forTimeInterval: 0.5)
    guard app.hide() || setVisibleWithSystemEvents(false) else {
        fputs("failed to hide app through AppKit and System Events\n", stderr)
        exit(3)
    }
    Thread.sleep(forTimeInterval: 1.0)
    print("hidden")
} else if mode == "show" {
    guard app.unhide() || setVisibleWithSystemEvents(true) else {
        fputs("failed to unhide app through AppKit and System Events\n", stderr)
        exit(4)
    }
    app.activate(options: [.activateAllWindows, .activateIgnoringOtherApps])
    Thread.sleep(forTimeInterval: 1.0)
    print("visible")
} else {
    fputs("mode must be hide or show\n", stderr)
    exit(2)
}
EOF

cat >"$RESIZE_WINDOW" <<'EOF'
import ApplicationServices
import Foundation

guard CommandLine.arguments.count == 6,
      let rawPID = Int32(CommandLine.arguments[1]),
      let x = Double(CommandLine.arguments[2]),
      let y = Double(CommandLine.arguments[3]),
      let width = Double(CommandLine.arguments[4]),
      let height = Double(CommandLine.arguments[5])
else {
    fputs("usage: resize-window.swift <pid> <x> <y> <width> <height>\n", stderr)
    exit(2)
}

guard AXIsProcessTrusted() else {
    fputs("accessibility permission is not trusted for window resize automation\n", stderr)
    exit(3)
}

let app = AXUIElementCreateApplication(pid_t(rawPID))
var windowsValue: CFTypeRef?
let windowsResult = AXUIElementCopyAttributeValue(
    app,
    kAXWindowsAttribute as CFString,
    &windowsValue
)

guard windowsResult == .success,
      let windows = windowsValue as? [AXUIElement],
      let window = windows.first
else {
    fputs("could not read target app windows: \(windowsResult.rawValue)\n", stderr)
    exit(4)
}

var position = CGPoint(x: x, y: y)
var size = CGSize(width: width, height: height)

guard let positionValue = AXValueCreate(.cgPoint, &position),
      let sizeValue = AXValueCreate(.cgSize, &size)
else {
    fputs("could not create accessibility values\n", stderr)
    exit(5)
}

let positionResult = AXUIElementSetAttributeValue(
    window,
    kAXPositionAttribute as CFString,
    positionValue
)
let sizeResult = AXUIElementSetAttributeValue(
    window,
    kAXSizeAttribute as CFString,
    sizeValue
)

guard positionResult == .success, sizeResult == .success else {
    fputs("resize failed position=\(positionResult.rawValue) size=\(sizeResult.rawValue)\n", stderr)
    exit(6)
}

Thread.sleep(forTimeInterval: 0.5)
EOF

log "scenario=$SCENARIO"
log "run_dir=$RUN_DIR"
log "app=$APP"
log "web=$WEB"
log "roamium=$ROAMIUM"
log "url=$URL"
log "app_log=$APP_LOG"
log "roamium_trace=$ROAMIUM_TRACE"
log "screenshot=$SCREENSHOT"
if [ "$SCENARIO" = "window-resize" ]; then
  log "grow_screenshot=$SCREENSHOT_GROW"
  log "shrink_screenshot=$SCREENSHOT_SHRINK"
fi
if [ "$SCENARIO" = "split-right" ] || [ "$SCENARIO" = "split-down" ] || [ "$SCENARIO" = "split-right-resize" ] || [ "$SCENARIO" = "split-right-equalize" ]; then
  log "split_screenshot=$SCREENSHOT_SPLIT"
fi
if [ "$SCENARIO" = "split-right-zoom" ]; then
  log "zoom_screenshot=$SCREENSHOT_ZOOM"
  log "unzoom_screenshot=$SCREENSHOT_UNZOOM"
fi
if [ "$SCENARIO" = "split-right-close-sibling" ]; then
  log "close_screenshot=$SCREENSHOT_CLOSE"
fi
if [ "$SCENARIO" = "split-right-close-browser-pane" ]; then
  log "close_screenshot=$SCREENSHOT_CLOSE"
  log "sibling_alive_command=$SIBLING_ALIVE_COMMAND"
fi
if [ "$SCENARIO" = "split-right-focus-switch" ]; then
  log "split_screenshot=$SCREENSHOT_SPLIT"
  log "sibling_focus_command=$SIBLING_FOCUS_COMMAND"
  log "browser_focus_command=$BROWSER_FOCUS_COMMAND"
fi
if [ "$SCENARIO" = "new-terminal-tab-visibility" ]; then
  log "new_tab_screenshot=$SCREENSHOT_TAB_NEW"
  log "back_tab_screenshot=$SCREENSHOT_TAB_BACK"
  log "new_tab_command_log=$NEW_TAB_COMMAND_LOG"
  log "new_tab_marker_command=$NEW_TAB_MARKER_COMMAND"
fi
if [ "$SCENARIO" = "open-browser-in-new-tab" ] || [ "$SCENARIO" = "close-browser-tab" ]; then
  log "new_tab_screenshot=$SCREENSHOT_TAB_NEW"
  log "browser_b_screenshot=$SCREENSHOT_TAB_BROWSER_B"
  log "browser_a_restored_screenshot=$SCREENSHOT_TAB_BROWSER_A_RESTORED"
  log "browser_b_restored_screenshot=$SCREENSHOT_TAB_BROWSER_B_RESTORED"
  log "after_close_screenshot=$SCREENSHOT_TAB_AFTER_CLOSE"
  log "second_browser_command=$SECOND_BROWSER_COMMAND"
fi
if [ "$SCENARIO" = "open-browser-in-new-window" ]; then
  log "window_b_screenshot=$SCREENSHOT_WINDOW_B"
  log "window_a_restored_screenshot=$SCREENSHOT_WINDOW_A_RESTORED"
  log "window_b_command=$SECOND_BROWSER_COMMAND"
  log "first_run_wrapper_command_log=$NEW_TAB_COMMAND_LOG"
fi
if [ "$SCENARIO" = "multiple-windows-with-browsers" ]; then
  log "window_b_screenshot=$SCREENSHOT_WINDOW_B"
  log "window_c_screenshot=$SCREENSHOT_WINDOW_C"
  log "window_a_restored_screenshot=$SCREENSHOT_WINDOW_A_RESTORED"
  log "window_b_restored_screenshot=$SCREENSHOT_WINDOW_B_RESTORED"
  log "window_b_command=$SECOND_BROWSER_COMMAND"
  log "window_c_command=$THIRD_BROWSER_COMMAND"
  log "first_run_wrapper_command_log=$NEW_TAB_COMMAND_LOG"
fi
if [ "$SCENARIO" = "display-move-backing-scale" ]; then
  log "display_moved_screenshot=$SCREENSHOT_DISPLAY_MOVED"
  log "display_returned_screenshot=$SCREENSHOT_DISPLAY_RETURNED"
fi
if [ "$SCENARIO" = "fullscreen-unfullscreen" ]; then
  log "fullscreen_screenshot=$SCREENSHOT_FULLSCREEN"
  log "unfullscreen_screenshot=$SCREENSHOT_UNFULLSCREEN"
fi
if [ "$SCENARIO" = "minimize-hide-restore" ]; then
  log "minimize_restored_screenshot=$SCREENSHOT_MINIMIZE_RESTORED"
  log "hide_restored_screenshot=$SCREENSHOT_HIDE_RESTORED"
fi
if [ "$SCENARIO" = "font-size-cell-metrics" ]; then
  log "font_increase_screenshot=$SCREENSHOT_FONT_INCREASE"
  log "font_decrease_screenshot=$SCREENSHOT_FONT_DECREASE"
fi
if [ "$SCENARIO" = "tui-overlay-resize-command" ]; then
  log "tui_shrink_screenshot=$SCREENSHOT_TUI_SHRINK"
  log "tui_reset_screenshot=$SCREENSHOT_TUI_RESET"
  log "tui_viewport_shrink_command=$TUI_VIEWPORT_SHRINK_COMMAND"
  log "tui_viewport_reset_command=$TUI_VIEWPORT_RESET_COMMAND"
fi

GHOSTTY_CONFIG_PATH="$CONFIG" \
GHOSTTY_LOG=stderr \
TERMSURF_GEOMETRY_TRACE=1 \
TERMSURF_GEOMETRY_SCENARIO="$SCENARIO" \
TERMSURF_INPUT_TRACE=1 \
TERMSURF_PDF_INPUT_TRACE=1 \
TERMSURF_PDF_INPUT_TRACE_FILE="$ROAMIUM_TRACE" \
  "$APP_BIN" >"$APP_LOG" 2>&1 &
PID="$!"
log "pid=$PID"

wait_for_log 'TermSurf geometry layer=appkit event=presented ' "AppKit overlay presentation"

PRESENTED_LINE="$(grep -E 'TermSurf geometry layer=appkit event=presented ' "$APP_LOG" | tail -1)"
WID="$(printf '%s\n' "$PRESENTED_LINE" | sed -E 's/.*identity=window_id:([^ ]+) .*/\1/')"
case "$WID" in
  ''|*[!0-9]*) fail "failed to extract numeric AppKit window id from presented geometry: $PRESENTED_LINE" ;;
esac
log "presented_window_id=$WID"

swift "$ACTIVATE_APP" "$PID" >>"$HARNESS_LOG" 2>&1 || fail "failed to activate pid=$PID"
delay 0.5

WIN_LINE="$(window_bounds)" || fail "failed to resolve bounds for window id=$WID"
IFS=$'\t' read -r WID WX WY WW WH <<<"$WIN_LINE"
log "window=$WIN_LINE"

screencapture -x -o -l"$WID" "$SCREENSHOT"
log "screenshot_exit=$?"

click_window_center "$WIN_LINE" "initial"
delay 1

require_log 'TermSurf geometry layer=zig' "Zig geometry record"
require_log 'TermSurf geometry layer=bridge' "bridge geometry record"
require_log 'TermSurf geometry layer=appkit event=presented ' "AppKit presented geometry record"
require_log 'TermSurf geometry layer=appkit event=hit_test .*hit=true' "AppKit hit-test geometry record"
require_log "scenario=${SCENARIO}" "scenario id in geometry records"

CA_CONTEXT_LINE="$(grep -E 'TermSurf geometry layer=zig event=ca_context' "$APP_LOG" | tail -1)"
ZIG_PRESENT_LINE="$(grep -E 'TermSurf geometry layer=zig event=present_overlay_call' "$APP_LOG" | tail -1)"
BRIDGE_PRESENT_LINE="$(grep -E 'TermSurf geometry layer=bridge event=present_target_found' "$APP_LOG" | tail -1)"
APPKIT_PRESENT_LINE="$(grep -E 'TermSurf geometry layer=appkit event=presented ' "$APP_LOG" | tail -1)"
APPKIT_PIXELS_LINE="$(grep -E 'TermSurf geometry layer=appkit event=presented_pixels' "$APP_LOG" | tail -1)"
HIT_TEST_LINE="$(grep -E 'TermSurf geometry layer=appkit event=hit_test .*hit=true' "$APP_LOG" | tail -1)"

[ -n "$CA_CONTEXT_LINE" ] || fail "missing Zig ca_context geometry line"
[ -n "$ZIG_PRESENT_LINE" ] || fail "missing Zig present_overlay_call geometry line"
[ -n "$BRIDGE_PRESENT_LINE" ] || fail "missing bridge present_target_found geometry line"
[ -n "$APPKIT_PRESENT_LINE" ] || fail "missing AppKit presented geometry line"
[ -n "$APPKIT_PIXELS_LINE" ] || fail "missing AppKit presented-pixels geometry line"
[ -n "$HIT_TEST_LINE" ] || fail "missing AppKit hit-test geometry line"

PANE_ID="$(printf '%s\n' "$CA_CONTEXT_LINE" | sed -E 's/.*pane_id:([^ ]+).*/\1/')"
[ -n "$PANE_ID" ] || fail "could not extract pane id"
BROWSER_TAB_ID="$(printf '%s\n' "$CA_CONTEXT_LINE" | sed -E 's/.*browser_tab_id:([^ ]+).*/\1/')"
case "$BROWSER_TAB_ID" in
  ''|unknown:*) fail "could not extract concrete browser tab id from Zig ca_context" ;;
esac
TAB_READY_LINE="$(grep -E "TermSurf geometry layer=zig event=tab_ready .*pane_id:${PANE_ID} .*browser_tab_id:${BROWSER_TAB_ID}" "$APP_LOG" | tail -1 || true)"
if [ -z "$TAB_READY_LINE" ] && grep -E "TabReady: pane_id=${PANE_ID} tab_id=${BROWSER_TAB_ID}" "$APP_LOG" >/dev/null 2>&1; then
  TAB_READY_LINE="pane_id:${PANE_ID} browser_tab_id:${BROWSER_TAB_ID} note=tab-ready-log-fallback"
fi
[ -n "$TAB_READY_LINE" ] || fail "missing Zig tab_ready geometry line for pane id and browser tab id"
CONTEXT_ID="$(printf '%s\n' "$ZIG_PRESENT_LINE" | sed -E 's/.*context_id=([0-9]+).*/\1/')"
[ -n "$CONTEXT_ID" ] || fail "could not extract context id"
GRID="$(printf '%s\n' "$ZIG_PRESENT_LINE" | sed -E 's/.*grid=([^ ]+).*/\1/')"
[ -n "$GRID" ] || fail "could not extract Zig overlay grid"
BROWSER_PIXEL="$(printf '%s\n' "$ZIG_PRESENT_LINE" | sed -E 's/.*browser_pixel=([^ ]+).*/\1/')"
[ -n "$BROWSER_PIXEL" ] || fail "could not extract Zig browser pixel size"
OVERLAY_FRAME="$(printf '%s\n' "$APPKIT_PRESENT_LINE" | sed -E 's/.*overlay_frame=([^ ]+ [^ ]+ [^ ]+ [^ ]+) root_frame=.*/\1/')"
[ -n "$OVERLAY_FRAME" ] && [ "$OVERLAY_FRAME" != "none" ] || fail "could not extract AppKit overlay frame"
OVERLAY_FRAME_SIZE="$(extract_frame_size "$APPKIT_PRESENT_LINE")"
[ -n "$OVERLAY_FRAME_SIZE" ] && [ "$OVERLAY_FRAME_SIZE" != "$APPKIT_PRESENT_LINE" ] || fail "could not extract AppKit overlay frame size"
OVERLAY_FRAME_X="$(extract_frame_x "$APPKIT_PRESENT_LINE")"
[ -n "$OVERLAY_FRAME_X" ] && [ "$OVERLAY_FRAME_X" != "$APPKIT_PRESENT_LINE" ] || fail "could not extract AppKit overlay frame x"
OVERLAY_FRAME_Y="$(extract_frame_y "$APPKIT_PRESENT_LINE")"
[ -n "$OVERLAY_FRAME_Y" ] && [ "$OVERLAY_FRAME_Y" != "$APPKIT_PRESENT_LINE" ] || fail "could not extract AppKit overlay frame y"
APPKIT_PIXEL="$(printf '%s\n' "$APPKIT_PIXELS_LINE" | sed -E 's/.*appkit_pixel=([^ ]+).*/\1/')"
[ -n "$APPKIT_PIXEL" ] || fail "could not extract AppKit presented pixel size"
APPKIT_PIXEL_WIDTH="${APPKIT_PIXEL%x*}"
APPKIT_PIXEL_HEIGHT="${APPKIT_PIXEL#*x}"

log "correlation_pane_id=$PANE_ID"
log "correlation_browser_tab_id=$BROWSER_TAB_ID"
log "correlation_context_id=$CONTEXT_ID"
log "correlation_grid=$GRID"
log "correlation_browser_pixel=$BROWSER_PIXEL"
log "correlation_overlay_frame=$OVERLAY_FRAME"
log "correlation_overlay_frame_size=$OVERLAY_FRAME_SIZE"
log "correlation_overlay_frame_x=$OVERLAY_FRAME_X"
log "correlation_overlay_frame_y=$OVERLAY_FRAME_Y"
log "correlation_appkit_pixel=$APPKIT_PIXEL"
log "correlation_scenario=$SCENARIO"
log "correlation_timestamp=$TS"
log "correlation_app_log=$APP_LOG"
log "correlation_harness_log=$HARNESS_LOG"
log "correlation_screenshot=$SCREENSHOT"

require_text "$TAB_READY_LINE" "pane_id:${PANE_ID}" "Zig tab_ready shares pane id"
require_text "$TAB_READY_LINE" "browser_tab_id:${BROWSER_TAB_ID}" "Zig tab_ready shares browser tab id"
require_text "$CA_CONTEXT_LINE" "pane_id:${PANE_ID}" "Zig ca_context shares pane id"
require_text "$CA_CONTEXT_LINE" "browser_tab_id:${BROWSER_TAB_ID}" "Zig ca_context shares browser tab id"
require_text "$CA_CONTEXT_LINE" "grid=${GRID}" "Zig ca_context shares grid"
require_text "$CA_CONTEXT_LINE" "browser_pixel=${BROWSER_PIXEL}" "Zig ca_context shares browser pixel"
require_text "$CA_CONTEXT_LINE" "context_id=${CONTEXT_ID}" "Zig ca_context shares context"
require_log "TermSurf geometry layer=bridge .*pane_id:${PANE_ID}" "bridge shares pane id"
require_log "TermSurf geometry layer=appkit .*pane_id:${PANE_ID}" "AppKit shares pane id"
require_text "$BRIDGE_PRESENT_LINE" "grid=${GRID}" "bridge shares grid"
require_text "$BRIDGE_PRESENT_LINE" "browser_pixel=${BROWSER_PIXEL}" "bridge shares browser pixel"
require_text "$BRIDGE_PRESENT_LINE" "context_id=${CONTEXT_ID}" "bridge shares context"
require_text "$APPKIT_PRESENT_LINE" "grid=${GRID}" "AppKit shares grid"
require_text "$APPKIT_PRESENT_LINE" "browser_pixel=${BROWSER_PIXEL}" "AppKit shares browser pixel"
require_text "$APPKIT_PRESENT_LINE" "context_id=${CONTEXT_ID}" "AppKit shares context"
require_text "$APPKIT_PIXELS_LINE" "appkit_pixel=${APPKIT_PIXEL}" "AppKit reports presented pixel size"
require_log "TermSurf geometry layer=zig event=appkit_presented_pixels .*pane_id:${PANE_ID} .*appkit_pixel=${APPKIT_PIXEL}" "Zig records AppKit presented pixel size"
require_log "TermSurf geometry layer=zig event=appkit_corrective_resize .*pane_id:${PANE_ID} .*appkit_pixel=${APPKIT_PIXEL}" "Zig sends corrective resize for AppKit pixel size"
require_log "TermSurf geometry layer=appkit .*context_id=${CONTEXT_ID}" "AppKit shares context id"
require_text "$HIT_TEST_LINE" "context_id=${CONTEXT_ID}" "hit-test shares context"
require_text "$HIT_TEST_LINE" "hit=true" "hit-test is inside overlay"
require_text "$HIT_TEST_LINE" "web_point={" "hit-test includes webview-relative point"
require_log "TermSurf geometry .*scenario=${SCENARIO}" "timestamped run contains scenario id"
require_log 'window_id:[^ ]+ surface_id:[^ ]+ selected_tab_id:[^ ]+ pane_id:[^ ]+ browser_tab_id:[^ ]+' "canonical identity tuple fields"
require_readable "$ROAMIUM_TRACE"
require_trace "resize tab_id=${BROWSER_TAB_ID} pane_id=${PANE_ID} pixel_width=${APPKIT_PIXEL_WIDTH} pixel_height=${APPKIT_PIXEL_HEIGHT} screen_x=0 screen_y=0 screen_width=0 screen_height=0 screen_scale=0 ffi=ts_set_view_size" "Roamium applied resize to AppKit pixel size via ts_set_view_size"

if [ "$SCENARIO" = "display-move-backing-scale" ]; then
  A_WINDOW_ID="$WID"
  A_SURFACE_ID="$(extract_surface_id "$APPKIT_PRESENT_LINE")"
  A_SELECTED_TAB_ID="$(extract_selected_tab_id "$APPKIT_PRESENT_LINE")"
  A_PANE_ID="$PANE_ID"
  A_BROWSER_TAB_ID="$BROWSER_TAB_ID"
  A_CONTEXT_ID="$CONTEXT_ID"
  A_FRAME="$OVERLAY_FRAME"
  A_FRAME_SIZE="$OVERLAY_FRAME_SIZE"
  A_FRAME_X="$OVERLAY_FRAME_X"
  A_FRAME_Y="$OVERLAY_FRAME_Y"
  A_ROOT_FRAME_SIZE="$(extract_root_frame_size "$APPKIT_PRESENT_LINE")"
  A_PIXEL="$APPKIT_PIXEL"
  A_BACKING_SCALE="$(extract_backing_scale "$APPKIT_PRESENT_LINE")"
  log "browser_a_window_id=$A_WINDOW_ID"
  log "browser_a_surface_id=$A_SURFACE_ID"
  log "browser_a_selected_tab_id=$A_SELECTED_TAB_ID"
  log "browser_a_pane_id=$A_PANE_ID"
  log "browser_a_browser_tab_id=$A_BROWSER_TAB_ID"
  log "browser_a_context_id=$A_CONTEXT_ID"
  log "browser_a_backing_scale=$A_BACKING_SCALE"

  DISPLAY_LINES="$(swift "$DISPLAY_INVENTORY")"
  DISPLAY_COUNT="$(printf '%s\n' "$DISPLAY_LINES" | awk 'NF { count++ } END { print count + 0 }')"
  log "display_count=$DISPLAY_COUNT"
  while IFS= read -r display_line; do
    [ -n "$display_line" ] && log "display=$display_line"
  done <<EOF
$DISPLAY_LINES
EOF

  if [ "$DISPLAY_COUNT" -lt 2 ]; then
    log "PARTIAL: only one display is available; cross-display move cannot run in this VM"
    screencapture -x -o -l"$A_WINDOW_ID" "$SCREENSHOT_DISPLAY_MOVED"
    log "single_display_screenshot_exit=$?"

    SINGLE_HIT_START_LINE="$(log_line_count)"
    click_window_center "$WIN_LINE" "single_display_browser_area"
    SINGLE_HIT_LINE="$(wait_for_hit_after "$SINGLE_HIT_START_LINE" "$A_CONTEXT_ID" "single-display browser hit-test")"
    require_text "$SINGLE_HIT_LINE" "window_id:${A_WINDOW_ID}" "single-display hit-test has window id"
    require_text "$SINGLE_HIT_LINE" "surface_id:${A_SURFACE_ID}" "single-display hit-test has surface id"
    require_text "$SINGLE_HIT_LINE" "selected_tab_id:${A_SELECTED_TAB_ID}" "single-display hit-test has selected tab id"
    require_text "$SINGLE_HIT_LINE" "overlay_frame=${A_FRAME}" "single-display hit-test uses AppKit frame"
    require_text "$SINGLE_HIT_LINE" "web_point={" "single-display hit-test includes webview-relative point"

    SINGLE_MODE_START_LINE="$(log_line_count)"
    SINGLE_MODE_TRACE_START_LINE="$(trace_line_count)"
    log "single_display_mode_key=enter=Mode::Browse"
    swift "$ROOT/scripts/ghostty-app/inject.swift" key 36 >>"$HARNESS_LOG" 2>&1
    wait_for_log_after "$SINGLE_MODE_START_LINE" "ModeChanged: pane_id=${A_PANE_ID} browsing=true" "single-display webtui entered browse mode"
    require_trace_after "$SINGLE_MODE_TRACE_START_LINE" "focus-changed tab=${A_BROWSER_TAB_ID} pane=${A_PANE_ID} ffi=ts_set_focus focused=true" "Roamium observed focus=true on single display"

    SINGLE_KEY_START_LINE="$(trace_line_count)"
    printf 'ISSUE809_EXP17_SINGLE_DISPLAY\n' >"$BROWSER_FOCUS_COMMAND"
    swift "$ROOT/scripts/ghostty-app/inject.swift" type "$BROWSER_FOCUS_COMMAND" >>"$HARNESS_LOG" 2>&1
    require_trace_after "$SINGLE_KEY_START_LINE" "key-event tab=${A_BROWSER_TAB_ID} pane=${A_PANE_ID}" "single-display keyboard marker reached browser"
    log "PASS: scenario display-move-backing-scale partial-single-display"
    exit 0
  fi

  ORIGINAL_WIN_LINE="$WIN_LINE"
  IFS=$'\t' read -r _ORIG_WID ORIG_WX ORIG_WY ORIG_WW ORIG_WH <<<"$ORIGINAL_WIN_LINE"
  ORIG_CENTER_X="$(awk -v x="$ORIG_WX" -v w="$ORIG_WW" 'BEGIN { print int(x + (w / 2)) }')"
  ORIG_CENTER_Y="$(awk -v y="$ORIG_WY" -v h="$ORIG_WH" 'BEGIN { print int(y + (h / 2)) }')"
  SOURCE_DISPLAY_LINE="$(printf '%s\n' "$DISPLAY_LINES" | awk -F '\t' -v x="$ORIG_CENTER_X" -v y="$ORIG_CENTER_Y" '$2 <= x && x < ($2 + $4) && $3 <= y && y < ($3 + $5) { print; exit }')"
  [ -n "$SOURCE_DISPLAY_LINE" ] || fail "could not identify source display for window center"
  SOURCE_DISPLAY_ID="$(printf '%s\n' "$SOURCE_DISPLAY_LINE" | awk -F '\t' '{print $1}')"
  SOURCE_SCALE="$(printf '%s\n' "$SOURCE_DISPLAY_LINE" | awk -F '\t' '{print $10}')"
  DEST_DISPLAY_LINE="$(printf '%s\n' "$DISPLAY_LINES" | awk -F '\t' -v source="$SOURCE_DISPLAY_ID" '$1 != source { print; exit }')"
  [ -n "$DEST_DISPLAY_LINE" ] || fail "could not identify destination display"
  IFS=$'\t' read -r DEST_DISPLAY_ID DEST_X DEST_Y DEST_W DEST_H DEST_VX DEST_VY DEST_VW DEST_VH DEST_SCALE _DEST_KIND <<<"$DEST_DISPLAY_LINE"
  log "source_display=$SOURCE_DISPLAY_LINE"
  log "destination_display=$DEST_DISPLAY_LINE"

  MOVE_START_LINE="$(log_line_count)"
  MOVE_TRACE_START_LINE="$(trace_line_count)"
  DEST_WINDOW_X="$(awk -v x="$DEST_VX" 'BEGIN { print int(x + 40) }')"
  DEST_WINDOW_Y="$(awk -v y="$DEST_VY" 'BEGIN { print int(y + 40) }')"
  swift "$RESIZE_WINDOW" "$PID" "$DEST_WINDOW_X" "$DEST_WINDOW_Y" "$ORIG_WW" "$ORIG_WH" >>"$HARNESS_LOG" 2>&1
  delay 2
  MOVED_WIN_LINE="$(window_bounds_for "$A_WINDOW_ID")" || fail "failed to resolve moved window bounds"
  log "moved_window=$MOVED_WIN_LINE"
  IFS=$'\t' read -r _MOVED_WID MOVED_WX MOVED_WY MOVED_WW MOVED_WH <<<"$MOVED_WIN_LINE"
  MOVED_CENTER_X="$(awk -v x="$MOVED_WX" -v w="$MOVED_WW" 'BEGIN { print int(x + (w / 2)) }')"
  MOVED_CENTER_Y="$(awk -v y="$MOVED_WY" -v h="$MOVED_WH" 'BEGIN { print int(y + (h / 2)) }')"
  awk -F '\t' -v x="$MOVED_CENTER_X" -v y="$MOVED_CENTER_Y" '{
    exit !(($2 <= x) && (x < ($2 + $4)) && ($3 <= y) && (y < ($3 + $5)))
  }' <<EOF || fail "moved window center is not inside destination display"
$DEST_DISPLAY_LINE
EOF
  log "PASS: moved window center is inside destination display"
  screencapture -x -o -l"$A_WINDOW_ID" "$SCREENSHOT_DISPLAY_MOVED"
  log "display_moved_screenshot_exit=$?"
  MOVED_PRESENT_LINE="$(wait_for_line_after "$MOVE_START_LINE" "TermSurf geometry layer=appkit event=presented .*pane_id:${A_PANE_ID} .*context_id=${A_CONTEXT_ID}" "display-moved AppKit presentation")"
  MOVED_PIXELS_LINE="$(wait_for_line_after "$MOVE_START_LINE" "TermSurf geometry layer=appkit event=presented_pixels .*pane_id:${A_PANE_ID} .*context_id=${A_CONTEXT_ID}" "display-moved AppKit pixels")"
  MOVED_FRAME="$(extract_overlay_frame "$MOVED_PRESENT_LINE")"
  MOVED_FRAME_SIZE="$(extract_frame_size "$MOVED_PRESENT_LINE")"
  MOVED_SCALE="$(extract_backing_scale "$MOVED_PRESENT_LINE")"
  MOVED_PIXEL="$(extract_appkit_pixel "$MOVED_PIXELS_LINE")"
  [ "$MOVED_FRAME" = "$A_FRAME" ] || fail "browser AppKit frame changed after display move: expected=$A_FRAME actual=$MOVED_FRAME"
  [ "$MOVED_SCALE" = "$DEST_SCALE" ] || fail "display-moved backing scale mismatch: expected=$DEST_SCALE actual=$MOVED_SCALE"
  MOVED_EXPECTED_PIXEL="$(awk -v size="$MOVED_FRAME_SIZE" -v scale="$MOVED_SCALE" 'BEGIN { split(size, parts, "x"); printf "%dx%d", int((parts[1] * scale) + 0.5), int((parts[2] * scale) + 0.5) }')"
  [ "$MOVED_PIXEL" = "$MOVED_EXPECTED_PIXEL" ] || fail "display-moved AppKit pixel mismatch: expected=$MOVED_EXPECTED_PIXEL actual=$MOVED_PIXEL"
  log "PASS: display-moved AppKit frame, pixels, and backing scale match destination display"
  if [ "$MOVED_PIXEL" != "$A_PIXEL" ]; then
    MOVED_PIXEL_WIDTH="${MOVED_PIXEL%x*}"
    MOVED_PIXEL_HEIGHT="${MOVED_PIXEL#*x}"
    require_trace_after "$MOVE_TRACE_START_LINE" "resize tab_id=${A_BROWSER_TAB_ID} pane_id=${A_PANE_ID} pixel_width=${MOVED_PIXEL_WIDTH} pixel_height=${MOVED_PIXEL_HEIGHT} screen_x=0 screen_y=0 screen_width=0 screen_height=0 screen_scale=0 ffi=ts_set_view_size" "Roamium applied display-moved resize to AppKit pixel size"
  else
    log "PASS: display-moved AppKit pixels unchanged; no Roamium resize required"
  fi

  MOVED_ROOT_HEIGHT="$(pair_height "$A_ROOT_FRAME_SIZE")"
  MOVED_CONTENT_Y_OFFSET="$(awk -v wh="$MOVED_WH" -v root_h="$MOVED_ROOT_HEIGHT" 'BEGIN { print int(wh - root_h) }')"
  MOVED_X="$(awk -v wx="$MOVED_WX" -v frame_x="$A_FRAME_X" 'BEGIN { print int(wx + frame_x + 4) }')"
  MOVED_Y="$(awk -v wy="$MOVED_WY" -v content_y="$MOVED_CONTENT_Y_OFFSET" -v frame_y="$A_FRAME_Y" 'BEGIN { print int(wy + content_y + frame_y + 4) }')"
  MOVED_HIT_START_LINE="$(log_line_count)"
  click_global_point "$MOVED_X" "$MOVED_Y" "display_moved_browser_area"
  MOVED_HIT_LINE="$(wait_for_hit_after "$MOVED_HIT_START_LINE" "$A_CONTEXT_ID" "display-moved browser hit-test")"
  require_text "$MOVED_HIT_LINE" "window_id:${A_WINDOW_ID}" "display-moved hit-test has window id"
  require_text "$MOVED_HIT_LINE" "surface_id:${A_SURFACE_ID}" "display-moved hit-test has surface id"
  require_text "$MOVED_HIT_LINE" "selected_tab_id:${A_SELECTED_TAB_ID}" "display-moved hit-test has selected tab id"
  require_text "$MOVED_HIT_LINE" "overlay_frame=${A_FRAME}" "display-moved hit-test uses AppKit frame"
  require_text "$MOVED_HIT_LINE" "web_point={" "display-moved hit-test includes webview-relative point"

  MOVED_MODE_START_LINE="$(log_line_count)"
  MOVED_MODE_TRACE_START_LINE="$(trace_line_count)"
  log "display_moved_mode_key=enter=Mode::Browse"
  swift "$ROOT/scripts/ghostty-app/inject.swift" key 36 >>"$HARNESS_LOG" 2>&1
  wait_for_log_after "$MOVED_MODE_START_LINE" "ModeChanged: pane_id=${A_PANE_ID} browsing=true" "display-moved webtui entered browse mode"
  require_trace_after "$MOVED_MODE_TRACE_START_LINE" "focus-changed tab=${A_BROWSER_TAB_ID} pane=${A_PANE_ID} ffi=ts_set_focus focused=true" "Roamium observed focus=true after display move"
  MOVED_KEY_START_LINE="$(trace_line_count)"
  printf 'ISSUE809_EXP17_MOVED_DISPLAY\n' >"$BROWSER_FOCUS_COMMAND"
  swift "$ROOT/scripts/ghostty-app/inject.swift" type "$BROWSER_FOCUS_COMMAND" >>"$HARNESS_LOG" 2>&1
  require_trace_after "$MOVED_KEY_START_LINE" "key-event tab=${A_BROWSER_TAB_ID} pane=${A_PANE_ID}" "display-moved keyboard marker reached browser"

  RETURN_CONTROL_START_LINE="$(log_line_count)"
  RETURN_CONTROL_TRACE_START_LINE="$(trace_line_count)"
  log "display_moved_control_key=escape=Mode::Control"
  swift "$ROOT/scripts/ghostty-app/inject.swift" key 53 >>"$HARNESS_LOG" 2>&1
  wait_for_log_after "$RETURN_CONTROL_START_LINE" "ModeChanged: pane_id=${A_PANE_ID} browsing=false" "display-moved webtui returned to control mode"
  require_trace_after "$RETURN_CONTROL_TRACE_START_LINE" "focus-changed tab=${A_BROWSER_TAB_ID} pane=${A_PANE_ID} ffi=ts_set_focus focused=false" "Roamium observed focus=false before return move"

  RETURN_START_LINE="$(log_line_count)"
  RETURN_TRACE_START_LINE="$(trace_line_count)"
  swift "$RESIZE_WINDOW" "$PID" "$ORIG_WX" "$ORIG_WY" "$ORIG_WW" "$ORIG_WH" >>"$HARNESS_LOG" 2>&1
  delay 2
  RETURNED_WIN_LINE="$(window_bounds_for "$A_WINDOW_ID")" || fail "failed to resolve returned window bounds"
  log "returned_window=$RETURNED_WIN_LINE"
  IFS=$'\t' read -r _RETURNED_WID RETURNED_WX RETURNED_WY RETURNED_WW RETURNED_WH <<<"$RETURNED_WIN_LINE"
  RETURNED_CENTER_X="$(awk -v x="$RETURNED_WX" -v w="$RETURNED_WW" 'BEGIN { print int(x + (w / 2)) }')"
  RETURNED_CENTER_Y="$(awk -v y="$RETURNED_WY" -v h="$RETURNED_WH" 'BEGIN { print int(y + (h / 2)) }')"
  awk -F '\t' -v x="$RETURNED_CENTER_X" -v y="$RETURNED_CENTER_Y" '{
    exit !(($2 <= x) && (x < ($2 + $4)) && ($3 <= y) && (y < ($3 + $5)))
  }' <<EOF || fail "returned window center is not inside source display"
$SOURCE_DISPLAY_LINE
EOF
  log "PASS: returned window center is inside source display"
  screencapture -x -o -l"$A_WINDOW_ID" "$SCREENSHOT_DISPLAY_RETURNED"
  log "display_returned_screenshot_exit=$?"
  RETURNED_PRESENT_LINE="$(wait_for_line_after "$RETURN_START_LINE" "TermSurf geometry layer=appkit event=presented .*pane_id:${A_PANE_ID} .*context_id=${A_CONTEXT_ID}" "display-returned AppKit presentation")"
  RETURNED_PIXELS_LINE="$(wait_for_line_after "$RETURN_START_LINE" "TermSurf geometry layer=appkit event=presented_pixels .*pane_id:${A_PANE_ID} .*context_id=${A_CONTEXT_ID}" "display-returned AppKit pixels")"
  RETURNED_FRAME="$(extract_overlay_frame "$RETURNED_PRESENT_LINE")"
  RETURNED_FRAME_SIZE="$(extract_frame_size "$RETURNED_PRESENT_LINE")"
  RETURNED_SCALE="$(extract_backing_scale "$RETURNED_PRESENT_LINE")"
  RETURNED_PIXEL="$(extract_appkit_pixel "$RETURNED_PIXELS_LINE")"
  [ "$RETURNED_FRAME" = "$A_FRAME" ] || fail "browser AppKit frame changed after display return: expected=$A_FRAME actual=$RETURNED_FRAME"
  [ "$RETURNED_SCALE" = "$SOURCE_SCALE" ] || fail "display-returned backing scale mismatch: expected=$SOURCE_SCALE actual=$RETURNED_SCALE"
  RETURNED_EXPECTED_PIXEL="$(awk -v size="$RETURNED_FRAME_SIZE" -v scale="$RETURNED_SCALE" 'BEGIN { split(size, parts, "x"); printf "%dx%d", int((parts[1] * scale) + 0.5), int((parts[2] * scale) + 0.5) }')"
  [ "$RETURNED_PIXEL" = "$RETURNED_EXPECTED_PIXEL" ] || fail "display-returned AppKit pixel mismatch: expected=$RETURNED_EXPECTED_PIXEL actual=$RETURNED_PIXEL"
  log "PASS: display-returned AppKit frame, pixels, and backing scale match source display"
  if [ "$RETURNED_PIXEL" != "$MOVED_PIXEL" ]; then
    RETURNED_PIXEL_WIDTH="${RETURNED_PIXEL%x*}"
    RETURNED_PIXEL_HEIGHT="${RETURNED_PIXEL#*x}"
    require_trace_after "$RETURN_TRACE_START_LINE" "resize tab_id=${A_BROWSER_TAB_ID} pane_id=${A_PANE_ID} pixel_width=${RETURNED_PIXEL_WIDTH} pixel_height=${RETURNED_PIXEL_HEIGHT} screen_x=0 screen_y=0 screen_width=0 screen_height=0 screen_scale=0 ffi=ts_set_view_size" "Roamium applied display-returned resize to AppKit pixel size"
  else
    log "PASS: display-returned AppKit pixels unchanged; no Roamium resize required"
  fi

  RETURN_HIT_START_LINE="$(log_line_count)"
  click_window_center "$RETURNED_WIN_LINE" "display_returned_browser_area"
  RETURN_HIT_LINE="$(wait_for_hit_after "$RETURN_HIT_START_LINE" "$A_CONTEXT_ID" "display-returned browser hit-test")"
  require_text "$RETURN_HIT_LINE" "window_id:${A_WINDOW_ID}" "display-returned hit-test has window id"
  require_text "$RETURN_HIT_LINE" "surface_id:${A_SURFACE_ID}" "display-returned hit-test has surface id"
  require_text "$RETURN_HIT_LINE" "selected_tab_id:${A_SELECTED_TAB_ID}" "display-returned hit-test has selected tab id"
  require_text "$RETURN_HIT_LINE" "overlay_frame=${A_FRAME}" "display-returned hit-test uses AppKit frame"
  require_text "$RETURN_HIT_LINE" "web_point={" "display-returned hit-test includes webview-relative point"

  [ "$MOVE_TRACE_START_LINE" -lt "$MOVED_MODE_TRACE_START_LINE" ] || fail "trace boundaries for display move were not monotonic"
  [ "$MOVED_MODE_TRACE_START_LINE" -lt "$RETURN_TRACE_START_LINE" ] || fail "trace boundaries for display return were not monotonic"
fi

if [ "$SCENARIO" = "fullscreen-unfullscreen" ]; then
  A_WINDOW_ID="$WID"
  A_SURFACE_ID="$(extract_surface_id "$APPKIT_PRESENT_LINE")"
  A_SELECTED_TAB_ID="$(extract_selected_tab_id "$APPKIT_PRESENT_LINE")"
  A_PANE_ID="$PANE_ID"
  A_BROWSER_TAB_ID="$BROWSER_TAB_ID"
  A_CONTEXT_ID="$CONTEXT_ID"
  A_FRAME="$OVERLAY_FRAME"
  A_FRAME_SIZE="$OVERLAY_FRAME_SIZE"
  A_FRAME_X="$OVERLAY_FRAME_X"
  A_FRAME_Y="$OVERLAY_FRAME_Y"
  A_PIXEL="$APPKIT_PIXEL"
  A_BACKING_SCALE="$(extract_backing_scale "$APPKIT_PRESENT_LINE")"
  log "browser_a_window_id=$A_WINDOW_ID"
  log "browser_a_surface_id=$A_SURFACE_ID"
  log "browser_a_selected_tab_id=$A_SELECTED_TAB_ID"
  log "browser_a_pane_id=$A_PANE_ID"
  log "browser_a_browser_tab_id=$A_BROWSER_TAB_ID"
  log "browser_a_context_id=$A_CONTEXT_ID"
  log "browser_a_backing_scale=$A_BACKING_SCALE"

  FULLSCREEN_START_LINE="$(log_line_count)"
  FULLSCREEN_TRACE_START_LINE="$(trace_line_count)"
  FULLSCREEN_STATE="$(swift "$FULLSCREEN_WINDOW" "$PID" "$WX" "$WY" "$WW" "$WH" enter >>"$HARNESS_LOG" 2>&1; tail -1 "$HARNESS_LOG")"
  [ "$FULLSCREEN_STATE" = "true" ] || fail "AXFullScreen did not become true after entering fullscreen: $FULLSCREEN_STATE"
  log "PASS: AXFullScreen true after enter"
  delay 4

  FULLSCREEN_WIN_LINE="$(app_windows | awk -F '\t' -v old="$A_WINDOW_ID" '$1 == old { print; found=1 } END { if (!found) exit 1 }' || true)"
  if [ -z "$FULLSCREEN_WIN_LINE" ]; then
    FULLSCREEN_WIN_LINE="$(app_windows | head -1 || true)"
  fi
  [ -n "$FULLSCREEN_WIN_LINE" ] || fail "could not resolve fullscreen Ghostboard window"
  IFS=$'\t' read -r FULLSCREEN_WINDOW_ID FULLSCREEN_WX FULLSCREEN_WY FULLSCREEN_WW FULLSCREEN_WH <<<"$FULLSCREEN_WIN_LINE"
  log "fullscreen_window=$FULLSCREEN_WIN_LINE"

  FULLSCREEN_PRESENT_LINE="$(wait_for_line_after "$FULLSCREEN_START_LINE" "TermSurf geometry layer=appkit event=presented .*pane_id:${A_PANE_ID} .*context_id=${A_CONTEXT_ID}" "fullscreen AppKit presentation" 45)"
  FULLSCREEN_PIXELS_LINE="$(wait_for_line_after "$FULLSCREEN_START_LINE" "TermSurf geometry layer=appkit event=presented_pixels .*pane_id:${A_PANE_ID} .*context_id=${A_CONTEXT_ID}" "fullscreen AppKit pixels" 45)"
  FULLSCREEN_PRESENT_WINDOW_ID="$(printf '%s\n' "$FULLSCREEN_PRESENT_LINE" | sed -E 's/.*window_id:([^ ]+) .*/\1/')"
  FULLSCREEN_SURFACE_ID="$(extract_surface_id "$FULLSCREEN_PRESENT_LINE")"
  FULLSCREEN_SELECTED_TAB_ID="$(extract_selected_tab_id "$FULLSCREEN_PRESENT_LINE")"
  FULLSCREEN_FRAME="$(extract_overlay_frame "$FULLSCREEN_PRESENT_LINE")"
  FULLSCREEN_FRAME_SIZE="$(extract_frame_size "$FULLSCREEN_PRESENT_LINE")"
  FULLSCREEN_FRAME_X="$(extract_frame_x "$FULLSCREEN_PRESENT_LINE")"
  FULLSCREEN_FRAME_Y="$(extract_frame_y "$FULLSCREEN_PRESENT_LINE")"
  FULLSCREEN_SCALE="$(extract_backing_scale "$FULLSCREEN_PRESENT_LINE")"
  FULLSCREEN_PIXEL="$(extract_appkit_pixel "$FULLSCREEN_PIXELS_LINE")"
  [ "$FULLSCREEN_PRESENT_WINDOW_ID" = "$FULLSCREEN_WINDOW_ID" ] || fail "fullscreen AppKit window id mismatch: current=$FULLSCREEN_WINDOW_ID presented=$FULLSCREEN_PRESENT_WINDOW_ID"
  [ "$FULLSCREEN_SURFACE_ID" = "$A_SURFACE_ID" ] || fail "fullscreen surface id changed"
  [ "$FULLSCREEN_SELECTED_TAB_ID" = "$A_SELECTED_TAB_ID" ] || fail "fullscreen selected tab id changed"
  [ "$FULLSCREEN_PIXEL" != "$A_PIXEL" ] || fail "fullscreen AppKit pixels did not change from baseline"
  compare_pair "$FULLSCREEN_FRAME_SIZE" "$A_FRAME_SIZE" gt || fail "fullscreen frame did not grow from baseline: baseline=$A_FRAME_SIZE fullscreen=$FULLSCREEN_FRAME_SIZE"
  log "fullscreen_window_id=$FULLSCREEN_PRESENT_WINDOW_ID"
  log "fullscreen_overlay_frame=$FULLSCREEN_FRAME"
  log "fullscreen_overlay_frame_size=$FULLSCREEN_FRAME_SIZE"
  log "fullscreen_appkit_pixel=$FULLSCREEN_PIXEL"
  log "fullscreen_backing_scale=$FULLSCREEN_SCALE"

  FULLSCREEN_PIXEL_WIDTH="${FULLSCREEN_PIXEL%x*}"
  FULLSCREEN_PIXEL_HEIGHT="${FULLSCREEN_PIXEL#*x}"
  require_trace_after "$FULLSCREEN_TRACE_START_LINE" "resize tab_id=${A_BROWSER_TAB_ID} pane_id=${A_PANE_ID} pixel_width=${FULLSCREEN_PIXEL_WIDTH} pixel_height=${FULLSCREEN_PIXEL_HEIGHT} screen_x=0 screen_y=0 screen_width=0 screen_height=0 screen_scale=0 ffi=ts_set_view_size" "Roamium applied fullscreen resize to AppKit pixel size"

  screencapture -x -o -l"$FULLSCREEN_WINDOW_ID" "$SCREENSHOT_FULLSCREEN"
  log "fullscreen_screenshot_exit=$?"

  FULLSCREEN_CLICK_X="$(awk -v wx="$FULLSCREEN_WX" -v frame_x="$FULLSCREEN_FRAME_X" -v frame_size="$FULLSCREEN_FRAME_SIZE" 'BEGIN { split(frame_size, parts, "x"); print int(wx + frame_x + (parts[1] / 2) + 0.5) }')"
  FULLSCREEN_CLICK_Y="$(awk -v wy="$FULLSCREEN_WY" -v frame_y="$FULLSCREEN_FRAME_Y" -v frame_size="$FULLSCREEN_FRAME_SIZE" 'BEGIN { split(frame_size, parts, "x"); print int(wy + frame_y + (parts[2] / 2) + 0.5) }')"
  FULLSCREEN_HIT_START_LINE="$(log_line_count)"
  click_global_point "$FULLSCREEN_CLICK_X" "$FULLSCREEN_CLICK_Y" "fullscreen_browser_area"
  FULLSCREEN_HIT_LINE="$(wait_for_hit_after "$FULLSCREEN_HIT_START_LINE" "$A_CONTEXT_ID" "fullscreen browser hit-test")"
  require_text "$FULLSCREEN_HIT_LINE" "window_id:${FULLSCREEN_WINDOW_ID}" "fullscreen hit-test has current window id"
  require_text "$FULLSCREEN_HIT_LINE" "surface_id:${A_SURFACE_ID}" "fullscreen hit-test has surface id"
  require_text "$FULLSCREEN_HIT_LINE" "selected_tab_id:${A_SELECTED_TAB_ID}" "fullscreen hit-test has selected tab id"
  require_text "$FULLSCREEN_HIT_LINE" "overlay_frame=${FULLSCREEN_FRAME}" "fullscreen hit-test uses fullscreen frame"
  require_text "$FULLSCREEN_HIT_LINE" "web_point={" "fullscreen hit-test includes webview-relative point"

  FULLSCREEN_MODE_START_LINE="$(log_line_count)"
  FULLSCREEN_MODE_TRACE_START_LINE="$(trace_line_count)"
  log "fullscreen_mode_key=enter=Mode::Browse"
  swift "$ROOT/scripts/ghostty-app/inject.swift" key 36 >>"$HARNESS_LOG" 2>&1
  wait_for_log_after "$FULLSCREEN_MODE_START_LINE" "ModeChanged: pane_id=${A_PANE_ID} browsing=true" "fullscreen webtui entered browse mode"
  require_trace_after "$FULLSCREEN_MODE_TRACE_START_LINE" "focus-changed tab=${A_BROWSER_TAB_ID} pane=${A_PANE_ID} ffi=ts_set_focus focused=true" "Roamium observed focus=true after fullscreen"

  FULLSCREEN_KEY_START_LINE="$(trace_line_count)"
  printf 'ISSUE809_EXP18_FULLSCREEN\n' >"$BROWSER_FOCUS_COMMAND"
  swift "$ROOT/scripts/ghostty-app/inject.swift" type "$BROWSER_FOCUS_COMMAND" >>"$HARNESS_LOG" 2>&1
  require_trace_after "$FULLSCREEN_KEY_START_LINE" "key-event tab=${A_BROWSER_TAB_ID} pane=${A_PANE_ID}" "fullscreen keyboard marker reached browser"

  FULLSCREEN_CONTROL_START_LINE="$(log_line_count)"
  FULLSCREEN_CONTROL_TRACE_START_LINE="$(trace_line_count)"
  log "fullscreen_control_key=escape=Mode::Control"
  swift "$ROOT/scripts/ghostty-app/inject.swift" key 53 >>"$HARNESS_LOG" 2>&1
  wait_for_log_after "$FULLSCREEN_CONTROL_START_LINE" "ModeChanged: pane_id=${A_PANE_ID} browsing=false" "fullscreen webtui returned to control mode"
  require_trace_after "$FULLSCREEN_CONTROL_TRACE_START_LINE" "focus-changed tab=${A_BROWSER_TAB_ID} pane=${A_PANE_ID} ffi=ts_set_focus focused=false" "Roamium observed focus=false before unfullscreen"

  UNFULLSCREEN_START_LINE="$(log_line_count)"
  UNFULLSCREEN_TRACE_START_LINE="$(trace_line_count)"
  UNFULLSCREEN_STATE="$(swift "$FULLSCREEN_WINDOW" "$PID" "$FULLSCREEN_WX" "$FULLSCREEN_WY" "$FULLSCREEN_WW" "$FULLSCREEN_WH" exit >>"$HARNESS_LOG" 2>&1; tail -1 "$HARNESS_LOG")"
  [ "$UNFULLSCREEN_STATE" = "false" ] || fail "AXFullScreen did not become false after exiting fullscreen: $UNFULLSCREEN_STATE"
  log "PASS: AXFullScreen false after exit"
  delay 4

  UNFULLSCREEN_WIN_LINE="$(app_windows | awk -F '\t' -v old="$A_WINDOW_ID" '$1 == old { print; found=1 } END { if (!found) exit 1 }' || true)"
  if [ -z "$UNFULLSCREEN_WIN_LINE" ]; then
    UNFULLSCREEN_WIN_LINE="$(app_windows | head -1 || true)"
  fi
  [ -n "$UNFULLSCREEN_WIN_LINE" ] || fail "could not resolve unfullscreen Ghostboard window"
  IFS=$'\t' read -r UNFULLSCREEN_WINDOW_ID UNFULLSCREEN_WX UNFULLSCREEN_WY UNFULLSCREEN_WW UNFULLSCREEN_WH <<<"$UNFULLSCREEN_WIN_LINE"
  log "unfullscreen_window=$UNFULLSCREEN_WIN_LINE"

  UNFULLSCREEN_PRESENT_LINE="$(wait_for_line_after "$UNFULLSCREEN_START_LINE" "TermSurf geometry layer=appkit event=presented .*pane_id:${A_PANE_ID} .*context_id=${A_CONTEXT_ID}" "unfullscreen AppKit presentation" 45)"
  UNFULLSCREEN_PIXELS_LINE="$(wait_for_line_after "$UNFULLSCREEN_START_LINE" "TermSurf geometry layer=appkit event=presented_pixels .*pane_id:${A_PANE_ID} .*context_id=${A_CONTEXT_ID}" "unfullscreen AppKit pixels" 45)"
  UNFULLSCREEN_PRESENT_WINDOW_ID="$(printf '%s\n' "$UNFULLSCREEN_PRESENT_LINE" | sed -E 's/.*window_id:([^ ]+) .*/\1/')"
  UNFULLSCREEN_SURFACE_ID="$(extract_surface_id "$UNFULLSCREEN_PRESENT_LINE")"
  UNFULLSCREEN_SELECTED_TAB_ID="$(extract_selected_tab_id "$UNFULLSCREEN_PRESENT_LINE")"
  UNFULLSCREEN_FRAME="$(extract_overlay_frame "$UNFULLSCREEN_PRESENT_LINE")"
  UNFULLSCREEN_FRAME_SIZE="$(extract_frame_size "$UNFULLSCREEN_PRESENT_LINE")"
  UNFULLSCREEN_SCALE="$(extract_backing_scale "$UNFULLSCREEN_PRESENT_LINE")"
  UNFULLSCREEN_PIXEL="$(extract_appkit_pixel "$UNFULLSCREEN_PIXELS_LINE")"
  [ "$UNFULLSCREEN_PRESENT_WINDOW_ID" = "$UNFULLSCREEN_WINDOW_ID" ] || fail "unfullscreen AppKit window id mismatch: current=$UNFULLSCREEN_WINDOW_ID presented=$UNFULLSCREEN_PRESENT_WINDOW_ID"
  [ "$UNFULLSCREEN_SURFACE_ID" = "$A_SURFACE_ID" ] || fail "unfullscreen surface id changed"
  [ "$UNFULLSCREEN_SELECTED_TAB_ID" = "$A_SELECTED_TAB_ID" ] || fail "unfullscreen selected tab id changed"
  [ "$UNFULLSCREEN_PIXEL" != "$FULLSCREEN_PIXEL" ] || fail "unfullscreen AppKit pixels did not change from fullscreen"
  log "unfullscreen_window_id=$UNFULLSCREEN_PRESENT_WINDOW_ID"
  log "unfullscreen_overlay_frame=$UNFULLSCREEN_FRAME"
  log "unfullscreen_overlay_frame_size=$UNFULLSCREEN_FRAME_SIZE"
  log "unfullscreen_appkit_pixel=$UNFULLSCREEN_PIXEL"
  log "unfullscreen_backing_scale=$UNFULLSCREEN_SCALE"

  UNFULLSCREEN_PIXEL_WIDTH="${UNFULLSCREEN_PIXEL%x*}"
  UNFULLSCREEN_PIXEL_HEIGHT="${UNFULLSCREEN_PIXEL#*x}"
  require_trace_after "$UNFULLSCREEN_TRACE_START_LINE" "resize tab_id=${A_BROWSER_TAB_ID} pane_id=${A_PANE_ID} pixel_width=${UNFULLSCREEN_PIXEL_WIDTH} pixel_height=${UNFULLSCREEN_PIXEL_HEIGHT} screen_x=0 screen_y=0 screen_width=0 screen_height=0 screen_scale=0 ffi=ts_set_view_size" "Roamium applied unfullscreen resize to AppKit pixel size"

  screencapture -x -o -l"$UNFULLSCREEN_WINDOW_ID" "$SCREENSHOT_UNFULLSCREEN"
  log "unfullscreen_screenshot_exit=$?"

  UNFULLSCREEN_HIT_START_LINE="$(log_line_count)"
  click_window_center "$UNFULLSCREEN_WIN_LINE" "unfullscreen_browser_area"
  UNFULLSCREEN_HIT_LINE="$(wait_for_hit_after "$UNFULLSCREEN_HIT_START_LINE" "$A_CONTEXT_ID" "unfullscreen browser hit-test")"
  require_text "$UNFULLSCREEN_HIT_LINE" "window_id:${UNFULLSCREEN_WINDOW_ID}" "unfullscreen hit-test has current window id"
  require_text "$UNFULLSCREEN_HIT_LINE" "surface_id:${A_SURFACE_ID}" "unfullscreen hit-test has surface id"
  require_text "$UNFULLSCREEN_HIT_LINE" "selected_tab_id:${A_SELECTED_TAB_ID}" "unfullscreen hit-test has selected tab id"
  require_text "$UNFULLSCREEN_HIT_LINE" "overlay_frame=${UNFULLSCREEN_FRAME}" "unfullscreen hit-test uses unfullscreen frame"
  require_text "$UNFULLSCREEN_HIT_LINE" "web_point={" "unfullscreen hit-test includes webview-relative point"

  UNFULLSCREEN_MODE_START_LINE="$(log_line_count)"
  UNFULLSCREEN_MODE_TRACE_START_LINE="$(trace_line_count)"
  log "unfullscreen_mode_key=enter=Mode::Browse"
  swift "$ROOT/scripts/ghostty-app/inject.swift" key 36 >>"$HARNESS_LOG" 2>&1
  wait_for_log_after "$UNFULLSCREEN_MODE_START_LINE" "ModeChanged: pane_id=${A_PANE_ID} browsing=true" "unfullscreen webtui entered browse mode"
  require_trace_after "$UNFULLSCREEN_MODE_TRACE_START_LINE" "focus-changed tab=${A_BROWSER_TAB_ID} pane=${A_PANE_ID} ffi=ts_set_focus focused=true" "Roamium observed focus=true after unfullscreen"

  UNFULLSCREEN_KEY_START_LINE="$(trace_line_count)"
  printf 'ISSUE809_EXP18_UNFULLSCREEN\n' >"$BROWSER_FOCUS_COMMAND"
  swift "$ROOT/scripts/ghostty-app/inject.swift" type "$BROWSER_FOCUS_COMMAND" >>"$HARNESS_LOG" 2>&1
  require_trace_after "$UNFULLSCREEN_KEY_START_LINE" "key-event tab=${A_BROWSER_TAB_ID} pane=${A_PANE_ID}" "unfullscreen keyboard marker reached browser"

  [ "$FULLSCREEN_TRACE_START_LINE" -lt "$FULLSCREEN_MODE_TRACE_START_LINE" ] || fail "trace boundaries for fullscreen were not monotonic"
  [ "$FULLSCREEN_MODE_TRACE_START_LINE" -lt "$UNFULLSCREEN_TRACE_START_LINE" ] || fail "trace boundaries for unfullscreen were not monotonic"
fi

if [ "$SCENARIO" = "minimize-hide-restore" ]; then
  A_WINDOW_ID="$WID"
  A_SURFACE_ID="$(extract_surface_id "$APPKIT_PRESENT_LINE")"
  A_SELECTED_TAB_ID="$(extract_selected_tab_id "$APPKIT_PRESENT_LINE")"
  A_PANE_ID="$PANE_ID"
  A_BROWSER_TAB_ID="$BROWSER_TAB_ID"
  A_CONTEXT_ID="$CONTEXT_ID"
  A_FRAME="$OVERLAY_FRAME"
  A_FRAME_SIZE="$OVERLAY_FRAME_SIZE"
  A_FRAME_X="$OVERLAY_FRAME_X"
  A_FRAME_Y="$OVERLAY_FRAME_Y"
  A_ROOT_FRAME_SIZE="$(extract_root_frame_size "$APPKIT_PRESENT_LINE")"
  A_PIXEL="$APPKIT_PIXEL"
  A_BACKING_SCALE="$(extract_backing_scale "$APPKIT_PRESENT_LINE")"
  log "browser_a_window_id=$A_WINDOW_ID"
  log "browser_a_surface_id=$A_SURFACE_ID"
  log "browser_a_selected_tab_id=$A_SELECTED_TAB_ID"
  log "browser_a_pane_id=$A_PANE_ID"
  log "browser_a_browser_tab_id=$A_BROWSER_TAB_ID"
  log "browser_a_context_id=$A_CONTEXT_ID"
  log "browser_a_backing_scale=$A_BACKING_SCALE"

  FORMER_CLICK_X="$(awk -v wx="$WX" -v frame_x="$A_FRAME_X" -v frame_size="$A_FRAME_SIZE" 'BEGIN { split(frame_size, parts, "x"); print int(wx + frame_x + (parts[1] / 2) + 0.5) }')"
  FORMER_CLICK_Y="$(awk -v wy="$WY" -v frame_y="$A_FRAME_Y" -v frame_size="$A_FRAME_SIZE" 'BEGIN { split(frame_size, parts, "x"); print int(wy + frame_y + (parts[2] / 2) + 0.5) }')"

  MINIMIZE_START_LINE="$(log_line_count)"
  MINIMIZE_STATE="$(swift "$MINIMIZE_WINDOW" "$PID" "$WX" "$WY" "$WW" "$WH" minimize >>"$HARNESS_LOG" 2>&1; tail -1 "$HARNESS_LOG")"
  [ "$MINIMIZE_STATE" = "true" ] || fail "AXMinimized did not become true after minimize: $MINIMIZE_STATE"
  log "PASS: AXMinimized true after minimize"
  delay 2
  if app_windows | awk -F '\t' -v wid="$A_WINDOW_ID" '$1 == wid { found=1 } END { exit found ? 0 : 1 }'; then
    fail "minimized window remained visible in onscreen CG window list"
  fi
  log "PASS: minimized window absent from onscreen CG window list"

  MINIMIZED_HIT_START_LINE="$(log_line_count)"
  click_global_point "$FORMER_CLICK_X" "$FORMER_CLICK_Y" "minimized_former_browser_area"
  wait_for_negative_hit_after "$MINIMIZED_HIT_START_LINE" "$A_CONTEXT_ID" "minimized former browser area hit-test" allow-absent 3

  RESTORE_START_LINE="$(log_line_count)"
  RESTORE_STATE="$(swift "$MINIMIZE_WINDOW" "$PID" "$WX" "$WY" "$WW" "$WH" restore >>"$HARNESS_LOG" 2>&1; tail -1 "$HARNESS_LOG")"
  [ "$RESTORE_STATE" = "false" ] || fail "AXMinimized did not become false after restore: $RESTORE_STATE"
  log "PASS: AXMinimized false after restore"
  delay 2
  RESTORE_WIN_LINE="$(window_bounds_for "$A_WINDOW_ID")" || fail "failed to resolve restored window bounds"
  log "minimize_restored_window=$RESTORE_WIN_LINE"
  IFS=$'\t' read -r RESTORE_WINDOW_ID RESTORE_WX RESTORE_WY RESTORE_WW RESTORE_WH <<<"$RESTORE_WIN_LINE"
  [ "$RESTORE_WINDOW_ID" = "$A_WINDOW_ID" ] || fail "minimize restore window id changed unexpectedly: expected=$A_WINDOW_ID actual=$RESTORE_WINDOW_ID"
  RESTORE_BACKING_LINE="$(wait_for_line_after "$RESTORE_START_LINE" "TermSurf geometry layer=appkit event=backing_properties_changed .*pane_id:${A_PANE_ID} .*context_id=${A_CONTEXT_ID}" "minimize-restored AppKit backing properties" 45)"
  RESTORE_PRESENT_WINDOW_ID="$(printf '%s\n' "$RESTORE_BACKING_LINE" | sed -E 's/.*window_id:([^ ]+) .*/\1/')"
  RESTORE_SURFACE_ID="$(extract_surface_id "$RESTORE_BACKING_LINE")"
  RESTORE_SELECTED_TAB_ID="$(extract_selected_tab_id "$RESTORE_BACKING_LINE")"
  RESTORE_FRAME="$(extract_overlay_frame "$RESTORE_BACKING_LINE")"
  RESTORE_SCALE="$(extract_backing_scale "$RESTORE_BACKING_LINE")"
  RESTORE_PIXEL="$(appkit_pixel_from_geometry_line "$RESTORE_BACKING_LINE")"
  [ "$RESTORE_PRESENT_WINDOW_ID" = "$A_WINDOW_ID" ] || fail "minimize-restored AppKit window id mismatch"
  [ "$RESTORE_SURFACE_ID" = "$A_SURFACE_ID" ] || fail "minimize-restored surface id changed"
  [ "$RESTORE_SELECTED_TAB_ID" = "$A_SELECTED_TAB_ID" ] || fail "minimize-restored selected tab id changed"
  [ "$RESTORE_FRAME" = "$A_FRAME" ] || fail "minimize-restored frame mismatch: expected=$A_FRAME actual=$RESTORE_FRAME"
  [ "$RESTORE_SCALE" = "$A_BACKING_SCALE" ] || fail "minimize-restored backing scale mismatch: expected=$A_BACKING_SCALE actual=$RESTORE_SCALE"
  [ "$RESTORE_PIXEL" = "$A_PIXEL" ] || fail "minimize-restored current AppKit pixel mismatch: expected=$A_PIXEL actual=$RESTORE_PIXEL"
  log "PASS: minimize-restored current AppKit pixel matched baseline"
  require_no_different_appkit_frame_after "$RESTORE_START_LINE" "$A_PANE_ID" "$A_CONTEXT_ID" "$A_FRAME" "minimize-restored AppKit frame stayed stable"
  require_no_different_appkit_pixels_after "$RESTORE_START_LINE" "$A_PANE_ID" "$A_CONTEXT_ID" "$A_PIXEL" "minimize-restored AppKit pixels stayed stable"

  screencapture -x -o -l"$A_WINDOW_ID" "$SCREENSHOT_MINIMIZE_RESTORED"
  log "minimize_restored_screenshot_exit=$?"
  RESTORE_HIT_START_LINE="$(log_line_count)"
  click_window_center "$RESTORE_WIN_LINE" "minimize_restored_browser_area"
  RESTORE_HIT_LINE="$(wait_for_hit_after "$RESTORE_HIT_START_LINE" "$A_CONTEXT_ID" "minimize-restored browser hit-test")"
  require_text "$RESTORE_HIT_LINE" "window_id:${A_WINDOW_ID}" "minimize-restored hit-test has window id"
  require_text "$RESTORE_HIT_LINE" "surface_id:${A_SURFACE_ID}" "minimize-restored hit-test has surface id"
  require_text "$RESTORE_HIT_LINE" "selected_tab_id:${A_SELECTED_TAB_ID}" "minimize-restored hit-test has selected tab id"
  require_text "$RESTORE_HIT_LINE" "overlay_frame=${A_FRAME}" "minimize-restored hit-test uses AppKit frame"
  require_text "$RESTORE_HIT_LINE" "web_point={" "minimize-restored hit-test includes webview-relative point"

  RESTORE_MODE_START_LINE="$(log_line_count)"
  RESTORE_MODE_TRACE_START_LINE="$(trace_line_count)"
  log "minimize_restored_mode_key=enter=Mode::Browse"
  swift "$ROOT/scripts/ghostty-app/inject.swift" key 36 >>"$HARNESS_LOG" 2>&1
  wait_for_log_after "$RESTORE_MODE_START_LINE" "ModeChanged: pane_id=${A_PANE_ID} browsing=true" "minimize-restored webtui entered browse mode"
  require_trace_after "$RESTORE_MODE_TRACE_START_LINE" "focus-changed tab=${A_BROWSER_TAB_ID} pane=${A_PANE_ID} ffi=ts_set_focus focused=true" "Roamium observed focus=true after minimize restore"
  RESTORE_KEY_START_LINE="$(trace_line_count)"
  printf 'ISSUE809_EXP19_MINIMIZE_RESTORE\n' >"$BROWSER_FOCUS_COMMAND"
  swift "$ROOT/scripts/ghostty-app/inject.swift" type "$BROWSER_FOCUS_COMMAND" >>"$HARNESS_LOG" 2>&1
  require_trace_after "$RESTORE_KEY_START_LINE" "key-event tab=${A_BROWSER_TAB_ID} pane=${A_PANE_ID}" "minimize-restored keyboard marker reached browser"

  RESTORE_CONTROL_START_LINE="$(log_line_count)"
  RESTORE_CONTROL_TRACE_START_LINE="$(trace_line_count)"
  log "minimize_restored_control_key=escape=Mode::Control"
  swift "$ROOT/scripts/ghostty-app/inject.swift" key 53 >>"$HARNESS_LOG" 2>&1
  wait_for_log_after "$RESTORE_CONTROL_START_LINE" "ModeChanged: pane_id=${A_PANE_ID} browsing=false" "minimize-restored webtui returned to control mode"
  require_trace_after "$RESTORE_CONTROL_TRACE_START_LINE" "focus-changed tab=${A_BROWSER_TAB_ID} pane=${A_PANE_ID} ffi=ts_set_focus focused=false" "Roamium observed focus=false before hide"

  HIDE_START_LINE="$(log_line_count)"
  HIDE_RESULT="$(swift "$HIDE_APP" "$PID" hide >>"$HARNESS_LOG" 2>&1; tail -1 "$HARNESS_LOG")"
  [ "$HIDE_RESULT" = "hidden" ] || fail "app hide did not report hidden: $HIDE_RESULT"
  log "PASS: app hide reported hidden"
  delay 2
  if app_windows | awk -F '\t' -v wid="$A_WINDOW_ID" '$1 == wid { found=1 } END { exit found ? 0 : 1 }'; then
    fail "hidden window remained visible in onscreen CG window list"
  fi
  log "PASS: hidden window absent from onscreen CG window list"

  HIDDEN_HIT_START_LINE="$(log_line_count)"
  click_global_point "$FORMER_CLICK_X" "$FORMER_CLICK_Y" "hidden_former_browser_area"
  wait_for_negative_hit_after "$HIDDEN_HIT_START_LINE" "$A_CONTEXT_ID" "hidden former browser area hit-test" allow-absent 3

  SHOW_START_LINE="$(log_line_count)"
  SHOW_RESULT="$(swift "$HIDE_APP" "$PID" show >>"$HARNESS_LOG" 2>&1; tail -1 "$HARNESS_LOG")"
  [ "$SHOW_RESULT" = "visible" ] || fail "app show did not report visible: $SHOW_RESULT"
  log "PASS: app show reported visible"
  delay 2
  SHOW_WIN_LINE="$(window_bounds_for "$A_WINDOW_ID")" || fail "failed to resolve unhidden window bounds"
  log "hide_restored_window=$SHOW_WIN_LINE"
  IFS=$'\t' read -r SHOW_WINDOW_ID SHOW_WX SHOW_WY SHOW_WW SHOW_WH <<<"$SHOW_WIN_LINE"
  [ "$SHOW_WINDOW_ID" = "$A_WINDOW_ID" ] || fail "hide restore window id changed unexpectedly: expected=$A_WINDOW_ID actual=$SHOW_WINDOW_ID"
  require_no_different_appkit_frame_after "$SHOW_START_LINE" "$A_PANE_ID" "$A_CONTEXT_ID" "$A_FRAME" "hide-restored AppKit frame stayed stable"
  require_no_different_appkit_pixels_after "$SHOW_START_LINE" "$A_PANE_ID" "$A_CONTEXT_ID" "$A_PIXEL" "hide-restored AppKit pixels stayed stable"

  screencapture -x -o -l"$A_WINDOW_ID" "$SCREENSHOT_HIDE_RESTORED"
  log "hide_restored_screenshot_exit=$?"
  SHOW_HIT_START_LINE="$(log_line_count)"
  click_window_center "$SHOW_WIN_LINE" "hide_restored_browser_area"
  SHOW_HIT_LINE="$(wait_for_hit_after "$SHOW_HIT_START_LINE" "$A_CONTEXT_ID" "hide-restored browser hit-test")"
  require_text "$SHOW_HIT_LINE" "window_id:${A_WINDOW_ID}" "hide-restored hit-test has window id"
  require_text "$SHOW_HIT_LINE" "surface_id:${A_SURFACE_ID}" "hide-restored hit-test has surface id"
  require_text "$SHOW_HIT_LINE" "selected_tab_id:${A_SELECTED_TAB_ID}" "hide-restored hit-test has selected tab id"
  require_text "$SHOW_HIT_LINE" "overlay_frame=${A_FRAME}" "hide-restored hit-test uses AppKit frame"
  require_text "$SHOW_HIT_LINE" "backing_scale=${A_BACKING_SCALE}" "hide-restored hit-test has backing scale"
  require_text "$SHOW_HIT_LINE" "web_point={" "hide-restored hit-test includes webview-relative point"
  SHOW_PIXEL="$(appkit_pixel_from_geometry_line "$SHOW_HIT_LINE")"
  [ "$SHOW_PIXEL" = "$A_PIXEL" ] || fail "hide-restored current AppKit pixel mismatch: expected=$A_PIXEL actual=$SHOW_PIXEL"
  log "PASS: hide-restored current AppKit pixel matched baseline"

  SHOW_MODE_START_LINE="$(log_line_count)"
  SHOW_MODE_TRACE_START_LINE="$(trace_line_count)"
  log "hide_restored_mode_key=enter=Mode::Browse"
  swift "$ROOT/scripts/ghostty-app/inject.swift" key 36 >>"$HARNESS_LOG" 2>&1
  wait_for_log_after "$SHOW_MODE_START_LINE" "ModeChanged: pane_id=${A_PANE_ID} browsing=true" "hide-restored webtui entered browse mode"
  require_trace_after "$SHOW_MODE_TRACE_START_LINE" "focus-changed tab=${A_BROWSER_TAB_ID} pane=${A_PANE_ID} ffi=ts_set_focus focused=true" "Roamium observed focus=true after hide restore"
  SHOW_KEY_START_LINE="$(trace_line_count)"
  printf 'ISSUE809_EXP19_HIDE_RESTORE\n' >"$BROWSER_FOCUS_COMMAND"
  swift "$ROOT/scripts/ghostty-app/inject.swift" type "$BROWSER_FOCUS_COMMAND" >>"$HARNESS_LOG" 2>&1
  require_trace_after "$SHOW_KEY_START_LINE" "key-event tab=${A_BROWSER_TAB_ID} pane=${A_PANE_ID}" "hide-restored keyboard marker reached browser"
fi

if [ "$SCENARIO" = "open-browser-in-new-window" ]; then
  A_WINDOW_ID="$WID"
  A_SELECTED_TAB_ID="$(extract_selected_tab_id "$APPKIT_PRESENT_LINE")"
  A_PANE_ID="$PANE_ID"
  A_BROWSER_TAB_ID="$BROWSER_TAB_ID"
  A_CONTEXT_ID="$CONTEXT_ID"
  A_FRAME="$OVERLAY_FRAME"
  A_FRAME_SIZE="$OVERLAY_FRAME_SIZE"
  A_FRAME_X="$OVERLAY_FRAME_X"
  A_FRAME_Y="$OVERLAY_FRAME_Y"
  A_ROOT_FRAME_SIZE="$(extract_root_frame_size "$APPKIT_PRESENT_LINE")"
  A_PIXEL="$APPKIT_PIXEL"
  log "browser_a_window_id=$A_WINDOW_ID"
  log "browser_a_selected_tab_id=$A_SELECTED_TAB_ID"
  log "browser_a_pane_id=$A_PANE_ID"
  log "browser_a_browser_tab_id=$A_BROWSER_TAB_ID"
  log "browser_a_context_id=$A_CONTEXT_ID"

  NEW_WINDOW_START_LINE="$(log_line_count)"
  NEW_WINDOW_TRACE_START_LINE="$(trace_line_count)"
  log "new_window_keybind=ctrl+b=new_window"
  swift "$ROOT/scripts/ghostty-app/inject.swift" key 11 control >>"$HARNESS_LOG" 2>&1
  delay 2

  require_log_after "$NEW_WINDOW_START_LINE" "dispatching action target=surface action=.new_window" "new window action dispatched"
  require_log_after "$NEW_WINDOW_START_LINE" 'starting command command=`/usr/bin/login`' "new window started plain login shell"
  if [ -s "$NEW_TAB_COMMAND_LOG" ]; then
    fail "new window unexpectedly inherited and ran the first-run web wrapper"
  fi
  log "PASS: new window did not inherit the first-run web wrapper"

  B_WIN_LINE=""
  for _ in $(seq 1 30); do
    B_WIN_LINE="$(app_windows | awk -F '\t' -v old="$A_WINDOW_ID" '$1 != old { print; exit }' || true)"
    if [ -n "$B_WIN_LINE" ]; then
      break
    fi
    delay 1
  done
  [ -n "$B_WIN_LINE" ] || fail "timed out waiting for second Ghostboard window"
  IFS=$'\t' read -r B_WINDOW_ID B_WX B_WY B_WW B_WH <<<"$B_WIN_LINE"
  [ "$B_WINDOW_ID" != "$A_WINDOW_ID" ] || fail "new window reused browser A window id"
  log "window_b=$B_WIN_LINE"

  click_window_center "$B_WIN_LINE" "window_b_shell"
  delay 1

  BROWSER_B_START_LINE="$(log_line_count)"
  BROWSER_B_TRACE_START_LINE="$(trace_line_count)"
  printf '"%s" --browser "%s" "%s"' "$WEB" "$ROAMIUM" "$URL_B" >"$SECOND_BROWSER_COMMAND"
  log "browser_b_command=$(cat "$SECOND_BROWSER_COMMAND")"
  swift "$ROOT/scripts/ghostty-app/inject.swift" type "$SECOND_BROWSER_COMMAND" >>"$HARNESS_LOG" 2>&1
  swift "$ROOT/scripts/ghostty-app/inject.swift" key 36 >>"$HARNESS_LOG" 2>&1

  B_CA_CONTEXT_LINE="$(wait_for_different_zig_event_after "$BROWSER_B_START_LINE" "ca_context" "$A_PANE_ID" "browser B Zig ca_context")"
  B_PANE_ID="$(extract_pane_id "$B_CA_CONTEXT_LINE")"
  B_BROWSER_TAB_ID="$(extract_browser_tab_id "$B_CA_CONTEXT_LINE")"
  B_CONTEXT_ID="$(extract_context_id "$B_CA_CONTEXT_LINE")"
  [ -n "$B_PANE_ID" ] || fail "could not extract browser B pane id"
  [ -n "$B_BROWSER_TAB_ID" ] || fail "could not extract browser B tab id"
  [ -n "$B_CONTEXT_ID" ] || fail "could not extract browser B context id"
  [ "$B_PANE_ID" != "$A_PANE_ID" ] || fail "browser B reused browser A pane id"
  [ "$B_BROWSER_TAB_ID" != "$A_BROWSER_TAB_ID" ] || fail "browser B reused browser A tab id"
  [ "$B_CONTEXT_ID" != "$A_CONTEXT_ID" ] || fail "browser B reused browser A CA/context id"
  log "browser_b_pane_id=$B_PANE_ID"
  log "browser_b_browser_tab_id=$B_BROWSER_TAB_ID"
  log "browser_b_context_id=$B_CONTEXT_ID"

  B_APPKIT_PRESENT_LINE="$(wait_for_line_after "$BROWSER_B_START_LINE" "TermSurf geometry layer=appkit event=presented .*pane_id:${B_PANE_ID} .*context_id=${B_CONTEXT_ID}" "browser B AppKit presentation")"
  B_APPKIT_PIXELS_LINE="$(wait_for_line_after "$BROWSER_B_START_LINE" "TermSurf geometry layer=appkit event=presented_pixels .*pane_id:${B_PANE_ID} .*context_id=${B_CONTEXT_ID}" "browser B AppKit pixels")"
  B_PRESENT_WINDOW_ID="$(printf '%s\n' "$B_APPKIT_PRESENT_LINE" | sed -E 's/.*window_id:([^ ]+) .*/\1/')"
  B_SELECTED_TAB_ID="$(extract_selected_tab_id "$B_APPKIT_PRESENT_LINE")"
  B_FRAME="$(extract_overlay_frame "$B_APPKIT_PRESENT_LINE")"
  B_FRAME_SIZE="$(extract_frame_size "$B_APPKIT_PRESENT_LINE")"
  B_FRAME_X="$(extract_frame_x "$B_APPKIT_PRESENT_LINE")"
  B_FRAME_Y="$(extract_frame_y "$B_APPKIT_PRESENT_LINE")"
  B_PIXEL="$(extract_appkit_pixel "$B_APPKIT_PIXELS_LINE")"
  [ "$B_PRESENT_WINDOW_ID" = "$B_WINDOW_ID" ] || fail "browser B AppKit window id mismatch: expected=$B_WINDOW_ID actual=$B_PRESENT_WINDOW_ID"
  [ "$B_SELECTED_TAB_ID" = "$B_WINDOW_ID" ] || fail "browser B selected tab id mismatch: expected=$B_WINDOW_ID actual=$B_SELECTED_TAB_ID"
  log "browser_b_window_id=$B_PRESENT_WINDOW_ID"
  log "browser_b_selected_tab_id=$B_SELECTED_TAB_ID"
  log "browser_b_overlay_frame=$B_FRAME"
  log "browser_b_overlay_frame_size=$B_FRAME_SIZE"
  log "browser_b_appkit_pixel=$B_PIXEL"

  B_PIXEL_WIDTH="${B_PIXEL%x*}"
  B_PIXEL_HEIGHT="${B_PIXEL#*x}"
  require_trace_after "$BROWSER_B_TRACE_START_LINE" "resize tab_id=${B_BROWSER_TAB_ID} pane_id=${B_PANE_ID} pixel_width=${B_PIXEL_WIDTH} pixel_height=${B_PIXEL_HEIGHT} screen_x=0 screen_y=0 screen_width=0 screen_height=0 screen_scale=0 ffi=ts_set_view_size" "Roamium applied browser B resize to AppKit pixel size"

  if tail -n +"$((BROWSER_B_START_LINE + 1))" "$APP_LOG" |
    grep -E "TermSurf geometry layer=appkit event=presented .*window_id:${B_WINDOW_ID} .*pane_id:${A_PANE_ID} .*context_id=${A_CONTEXT_ID} .*visible=true" >/dev/null 2>&1; then
    fail "browser A overlay was presented as visible in window B"
  fi
  log "PASS: browser A was not presented as visible in window B"

  screencapture -x -o -l"$B_WINDOW_ID" "$SCREENSHOT_WINDOW_B"
  log "window_b_screenshot_exit=$?"

  B_CLICK_X="$(awk -v wx="$B_WX" -v frame_x="$B_FRAME_X" -v frame_size="$B_FRAME_SIZE" 'BEGIN { split(frame_size, parts, "x"); print int(wx + frame_x + (parts[1] / 2) + 0.5) }')"
  B_CLICK_Y="$(awk -v wy="$B_WY" -v frame_y="$B_FRAME_Y" -v frame_size="$B_FRAME_SIZE" 'BEGIN { split(frame_size, parts, "x"); print int(wy + frame_y + (parts[2] / 2) + 0.5) }')"
  B_HIT_START_LINE="$(log_line_count)"
  click_global_point "$B_CLICK_X" "$B_CLICK_Y" "browser_b_window_area"
  B_HIT_LINE="$(wait_for_hit_after "$B_HIT_START_LINE" "$B_CONTEXT_ID" "browser B hit-test in window B")"
  require_text "$B_HIT_LINE" "window_id:${B_WINDOW_ID}" "browser B hit-test has window B id"
  require_text "$B_HIT_LINE" "selected_tab_id:${B_SELECTED_TAB_ID}" "browser B hit-test has window B selected tab id"
  require_text "$B_HIT_LINE" "overlay_frame=${B_FRAME}" "browser B hit-test uses browser B frame"
  require_text "$B_HIT_LINE" "web_point={" "browser B hit-test includes webview-relative point"

  B_MODE_START_LINE="$(log_line_count)"
  B_MODE_TRACE_START_LINE="$(trace_line_count)"
  log "browser_b_mode_key=enter=Mode::Browse"
  swift "$ROOT/scripts/ghostty-app/inject.swift" key 36 >>"$HARNESS_LOG" 2>&1
  wait_for_log_after "$B_MODE_START_LINE" "ModeChanged: pane_id=${B_PANE_ID} browsing=true" "browser B webtui entered browse mode"
  require_trace_after "$B_MODE_TRACE_START_LINE" "focus-changed tab=${B_BROWSER_TAB_ID} pane=${B_PANE_ID} ffi=ts_set_focus focused=true" "Roamium observed browser B focus=true after browse mode"

  B_KEY_START_LINE="$(trace_line_count)"
  printf 'ISSUE809_EXP15_BROWSER_B_WINDOW\n' >"$BROWSER_FOCUS_COMMAND"
  swift "$ROOT/scripts/ghostty-app/inject.swift" type "$BROWSER_FOCUS_COMMAND" >>"$HARNESS_LOG" 2>&1
  require_trace_after "$B_KEY_START_LINE" "key-event tab=${B_BROWSER_TAB_ID} pane=${B_PANE_ID}" "browser B keyboard marker reached browser B"
  require_no_trace_after "$B_KEY_START_LINE" "key-event tab=${A_BROWSER_TAB_ID} pane=${A_PANE_ID}" "browser B keyboard marker did not reach browser A"

  B_CONTROL_START_LINE="$(log_line_count)"
  B_CONTROL_TRACE_START_LINE="$(trace_line_count)"
  log "browser_b_control_key=escape=Mode::Control"
  swift "$ROOT/scripts/ghostty-app/inject.swift" key 53 >>"$HARNESS_LOG" 2>&1
  wait_for_log_after "$B_CONTROL_START_LINE" "ModeChanged: pane_id=${B_PANE_ID} browsing=false" "browser B webtui returned to control mode"
  require_trace_after "$B_CONTROL_TRACE_START_LINE" "focus-changed tab=${B_BROWSER_TAB_ID} pane=${B_PANE_ID} ffi=ts_set_focus focused=false" "Roamium observed browser B focus=false after control mode"

  A_RESTORE_START_LINE="$(log_line_count)"
  A_RESTORE_TRACE_START_LINE="$(trace_line_count)"
  A_WIN_LINE="$(window_bounds_for "$A_WINDOW_ID")" || fail "failed to resolve browser A window bounds for window id=$A_WINDOW_ID"
  focus_window_bounds "$A_WIN_LINE" >>"$HARNESS_LOG" 2>&1 || fail "failed to focus browser A window via accessibility"
  IFS=$'\t' read -r _A_WID A_WX A_WY A_WW A_WH <<<"$A_WIN_LINE"
  A_ROOT_HEIGHT="$(pair_height "$A_ROOT_FRAME_SIZE")"
  A_CONTENT_Y_OFFSET="$(awk -v wh="$A_WH" -v root_h="$A_ROOT_HEIGHT" 'BEGIN { print int(wh - root_h) }')"
  A_VISIBLE_X="$(awk -v wx="$A_WX" -v frame_x="$A_FRAME_X" 'BEGIN { print int(wx + frame_x + 4) }')"
  A_VISIBLE_Y="$(awk -v wy="$A_WY" -v content_y="$A_CONTENT_Y_OFFSET" -v frame_y="$A_FRAME_Y" 'BEGIN { print int(wy + content_y + frame_y + 4) }')"
  A_HIT_START_LINE="$(log_line_count)"
  click_global_point "$A_VISIBLE_X" "$A_VISIBLE_Y" "window_a_visible_edge_restore"
  A_HIT_LINE="$(wait_for_hit_after "$A_HIT_START_LINE" "$A_CONTEXT_ID" "browser A restored visible hit-test in window A")"
  require_no_different_appkit_frame_after "$A_RESTORE_START_LINE" "$A_PANE_ID" "$A_CONTEXT_ID" "$A_FRAME" "browser A kept AppKit frame after returning to window A"
  require_no_different_appkit_pixels_after "$A_RESTORE_START_LINE" "$A_PANE_ID" "$A_CONTEXT_ID" "$A_PIXEL" "browser A kept AppKit pixels after returning to window A"

  require_text "$A_HIT_LINE" "window_id:${A_WINDOW_ID}" "browser A restored hit-test has window A id"
  require_text "$A_HIT_LINE" "selected_tab_id:${A_SELECTED_TAB_ID}" "browser A restored hit-test has original selected tab id"
  require_text "$A_HIT_LINE" "overlay_frame=${A_FRAME}" "browser A restored hit-test uses browser A frame"
  require_text "$A_HIT_LINE" "web_point={" "browser A restored hit-test includes webview-relative point"

  A_MODE_START_LINE="$(log_line_count)"
  A_MODE_TRACE_START_LINE="$(trace_line_count)"
  log "browser_a_mode_key=enter=Mode::Browse"
  swift "$ROOT/scripts/ghostty-app/inject.swift" key 36 >>"$HARNESS_LOG" 2>&1
  wait_for_log_after "$A_MODE_START_LINE" "ModeChanged: pane_id=${A_PANE_ID} browsing=true" "browser A webtui entered browse mode after window restore"
  require_trace_after "$A_MODE_TRACE_START_LINE" "focus-changed tab=${A_BROWSER_TAB_ID} pane=${A_PANE_ID} ffi=ts_set_focus focused=true" "Roamium observed browser A focus=true after window restore"

  A_KEY_START_LINE="$(trace_line_count)"
  printf 'ISSUE809_EXP15_BROWSER_A_WINDOW\n' >"$BROWSER_FOCUS_COMMAND"
  swift "$ROOT/scripts/ghostty-app/inject.swift" type "$BROWSER_FOCUS_COMMAND" >>"$HARNESS_LOG" 2>&1
  require_trace_after "$A_KEY_START_LINE" "key-event tab=${A_BROWSER_TAB_ID} pane=${A_PANE_ID}" "browser A keyboard marker reached browser A"
  require_no_trace_after "$A_KEY_START_LINE" "key-event tab=${B_BROWSER_TAB_ID} pane=${B_PANE_ID}" "browser A keyboard marker did not reach browser B"

  screencapture -x -o -l"$A_WINDOW_ID" "$SCREENSHOT_WINDOW_A_RESTORED"
  log "window_a_restored_screenshot_exit=$?"

  [ "$NEW_WINDOW_TRACE_START_LINE" -lt "$BROWSER_B_TRACE_START_LINE" ] || fail "trace boundaries for browser B window open were not monotonic"
  [ "$BROWSER_B_TRACE_START_LINE" -lt "$A_RESTORE_TRACE_START_LINE" ] || fail "trace boundaries for browser A window restore were not monotonic"
fi

if [ "$SCENARIO" = "multiple-windows-with-browsers" ]; then
  A_WINDOW_ID="$WID"
  A_SURFACE_ID="$(extract_surface_id "$APPKIT_PRESENT_LINE")"
  A_SELECTED_TAB_ID="$(extract_selected_tab_id "$APPKIT_PRESENT_LINE")"
  A_PANE_ID="$PANE_ID"
  A_BROWSER_TAB_ID="$BROWSER_TAB_ID"
  A_CONTEXT_ID="$CONTEXT_ID"
  A_FRAME="$OVERLAY_FRAME"
  A_FRAME_SIZE="$OVERLAY_FRAME_SIZE"
  A_FRAME_X="$OVERLAY_FRAME_X"
  A_FRAME_Y="$OVERLAY_FRAME_Y"
  A_ROOT_FRAME_SIZE="$(extract_root_frame_size "$APPKIT_PRESENT_LINE")"
  A_PIXEL="$APPKIT_PIXEL"
  log "browser_a_window_id=$A_WINDOW_ID"
  log "browser_a_surface_id=$A_SURFACE_ID"
  log "browser_a_selected_tab_id=$A_SELECTED_TAB_ID"
  log "browser_a_pane_id=$A_PANE_ID"
  log "browser_a_browser_tab_id=$A_BROWSER_TAB_ID"
  log "browser_a_context_id=$A_CONTEXT_ID"

  NEW_WINDOW_B_START_LINE="$(log_line_count)"
  NEW_WINDOW_B_TRACE_START_LINE="$(trace_line_count)"
  log "new_window_b_keybind=ctrl+b=new_window"
  swift "$ROOT/scripts/ghostty-app/inject.swift" key 11 control >>"$HARNESS_LOG" 2>&1
  delay 2

  require_log_after "$NEW_WINDOW_B_START_LINE" "dispatching action target=surface action=.new_window" "window B new-window action dispatched"
  require_log_after "$NEW_WINDOW_B_START_LINE" 'starting command command=`/usr/bin/login`' "window B started plain login shell"
  if [ -s "$NEW_TAB_COMMAND_LOG" ]; then
    fail "window B unexpectedly inherited and ran the first-run web wrapper"
  fi
  log "PASS: window B did not inherit the first-run web wrapper"

  B_WIN_LINE=""
  for _ in $(seq 1 30); do
    B_WIN_LINE="$(app_windows | awk -F '\t' -v old="$A_WINDOW_ID" '$1 != old { print; exit }' || true)"
    if [ -n "$B_WIN_LINE" ]; then
      break
    fi
    delay 1
  done
  [ -n "$B_WIN_LINE" ] || fail "timed out waiting for second Ghostboard window"
  IFS=$'\t' read -r B_WINDOW_ID B_WX B_WY B_WW B_WH <<<"$B_WIN_LINE"
  [ "$B_WINDOW_ID" != "$A_WINDOW_ID" ] || fail "window B reused browser A window id"
  log "window_b=$B_WIN_LINE"

  click_window_center "$B_WIN_LINE" "window_b_shell"
  delay 1

  BROWSER_B_START_LINE="$(log_line_count)"
  BROWSER_B_TRACE_START_LINE="$(trace_line_count)"
  printf '"%s" --browser "%s" "%s"' "$WEB" "$ROAMIUM" "$URL_B" >"$SECOND_BROWSER_COMMAND"
  log "browser_b_command=$(cat "$SECOND_BROWSER_COMMAND")"
  swift "$ROOT/scripts/ghostty-app/inject.swift" type "$SECOND_BROWSER_COMMAND" >>"$HARNESS_LOG" 2>&1
  swift "$ROOT/scripts/ghostty-app/inject.swift" key 36 >>"$HARNESS_LOG" 2>&1

  B_CA_CONTEXT_LINE="$(wait_for_different_zig_event_after "$BROWSER_B_START_LINE" "ca_context" "$A_PANE_ID" "browser B Zig ca_context")"
  B_PANE_ID="$(extract_pane_id "$B_CA_CONTEXT_LINE")"
  B_BROWSER_TAB_ID="$(extract_browser_tab_id "$B_CA_CONTEXT_LINE")"
  B_CONTEXT_ID="$(extract_context_id "$B_CA_CONTEXT_LINE")"
  [ -n "$B_PANE_ID" ] || fail "could not extract browser B pane id"
  [ -n "$B_BROWSER_TAB_ID" ] || fail "could not extract browser B tab id"
  [ -n "$B_CONTEXT_ID" ] || fail "could not extract browser B context id"
  [ "$B_PANE_ID" != "$A_PANE_ID" ] || fail "browser B reused browser A pane id"
  [ "$B_BROWSER_TAB_ID" != "$A_BROWSER_TAB_ID" ] || fail "browser B reused browser A tab id"
  [ "$B_CONTEXT_ID" != "$A_CONTEXT_ID" ] || fail "browser B reused browser A CA/context id"
  log "browser_b_pane_id=$B_PANE_ID"
  log "browser_b_browser_tab_id=$B_BROWSER_TAB_ID"
  log "browser_b_context_id=$B_CONTEXT_ID"

  B_APPKIT_PRESENT_LINE="$(wait_for_line_after "$BROWSER_B_START_LINE" "TermSurf geometry layer=appkit event=presented .*pane_id:${B_PANE_ID} .*context_id=${B_CONTEXT_ID}" "browser B AppKit presentation")"
  B_APPKIT_PIXELS_LINE="$(wait_for_line_after "$BROWSER_B_START_LINE" "TermSurf geometry layer=appkit event=presented_pixels .*pane_id:${B_PANE_ID} .*context_id=${B_CONTEXT_ID}" "browser B AppKit pixels")"
  B_PRESENT_WINDOW_ID="$(printf '%s\n' "$B_APPKIT_PRESENT_LINE" | sed -E 's/.*window_id:([^ ]+) .*/\1/')"
  B_SURFACE_ID="$(extract_surface_id "$B_APPKIT_PRESENT_LINE")"
  B_SELECTED_TAB_ID="$(extract_selected_tab_id "$B_APPKIT_PRESENT_LINE")"
  B_FRAME="$(extract_overlay_frame "$B_APPKIT_PRESENT_LINE")"
  B_FRAME_SIZE="$(extract_frame_size "$B_APPKIT_PRESENT_LINE")"
  B_FRAME_X="$(extract_frame_x "$B_APPKIT_PRESENT_LINE")"
  B_FRAME_Y="$(extract_frame_y "$B_APPKIT_PRESENT_LINE")"
  B_ROOT_FRAME_SIZE="$(extract_root_frame_size "$B_APPKIT_PRESENT_LINE")"
  B_PIXEL="$(extract_appkit_pixel "$B_APPKIT_PIXELS_LINE")"
  [ "$B_PRESENT_WINDOW_ID" = "$B_WINDOW_ID" ] || fail "browser B AppKit window id mismatch: expected=$B_WINDOW_ID actual=$B_PRESENT_WINDOW_ID"
  [ -n "$B_SURFACE_ID" ] || fail "could not extract browser B AppKit surface id"
  [ "$B_SURFACE_ID" != "$A_SURFACE_ID" ] || fail "browser B reused browser A surface id"
  [ "$B_SELECTED_TAB_ID" = "$B_WINDOW_ID" ] || fail "browser B selected tab id mismatch: expected=$B_WINDOW_ID actual=$B_SELECTED_TAB_ID"
  log "browser_b_window_id=$B_PRESENT_WINDOW_ID"
  log "browser_b_surface_id=$B_SURFACE_ID"
  log "browser_b_selected_tab_id=$B_SELECTED_TAB_ID"
  log "browser_b_overlay_frame=$B_FRAME"
  log "browser_b_appkit_pixel=$B_PIXEL"

  B_PIXEL_WIDTH="${B_PIXEL%x*}"
  B_PIXEL_HEIGHT="${B_PIXEL#*x}"
  require_trace_after "$BROWSER_B_TRACE_START_LINE" "resize tab_id=${B_BROWSER_TAB_ID} pane_id=${B_PANE_ID} pixel_width=${B_PIXEL_WIDTH} pixel_height=${B_PIXEL_HEIGHT} screen_x=0 screen_y=0 screen_width=0 screen_height=0 screen_scale=0 ffi=ts_set_view_size" "Roamium applied browser B resize to AppKit pixel size"

  NEW_WINDOW_C_START_LINE="$(log_line_count)"
  NEW_WINDOW_C_TRACE_START_LINE="$(trace_line_count)"
  log "new_window_c_keybind=ctrl+b=new_window"
  swift "$ROOT/scripts/ghostty-app/inject.swift" key 11 control >>"$HARNESS_LOG" 2>&1
  delay 2

  require_log_after "$NEW_WINDOW_C_START_LINE" "dispatching action target=surface action=.new_window" "window C new-window action dispatched"
  require_log_after "$NEW_WINDOW_C_START_LINE" 'starting command command=`/usr/bin/login`' "window C started plain login shell"
  if [ -s "$NEW_TAB_COMMAND_LOG" ]; then
    fail "window C unexpectedly inherited and ran the first-run web wrapper"
  fi
  log "PASS: window C did not inherit the first-run web wrapper"

  C_WIN_LINE=""
  for _ in $(seq 1 30); do
    C_WIN_LINE="$(app_windows | awk -F '\t' -v a="$A_WINDOW_ID" -v b="$B_WINDOW_ID" '$1 != a && $1 != b { print; exit }' || true)"
    if [ -n "$C_WIN_LINE" ]; then
      break
    fi
    delay 1
  done
  [ -n "$C_WIN_LINE" ] || fail "timed out waiting for third Ghostboard window"
  IFS=$'\t' read -r C_WINDOW_ID C_WX C_WY C_WW C_WH <<<"$C_WIN_LINE"
  [ "$C_WINDOW_ID" != "$A_WINDOW_ID" ] || fail "window C reused browser A window id"
  [ "$C_WINDOW_ID" != "$B_WINDOW_ID" ] || fail "window C reused browser B window id"
  log "window_c=$C_WIN_LINE"

  click_window_center "$C_WIN_LINE" "window_c_shell"
  delay 1

  BROWSER_C_START_LINE="$(log_line_count)"
  BROWSER_C_TRACE_START_LINE="$(trace_line_count)"
  printf '"%s" --browser "%s" "%s"' "$WEB" "$ROAMIUM" "$URL_C" >"$THIRD_BROWSER_COMMAND"
  log "browser_c_command=$(cat "$THIRD_BROWSER_COMMAND")"
  swift "$ROOT/scripts/ghostty-app/inject.swift" type "$THIRD_BROWSER_COMMAND" >>"$HARNESS_LOG" 2>&1
  swift "$ROOT/scripts/ghostty-app/inject.swift" key 36 >>"$HARNESS_LOG" 2>&1

  C_CA_CONTEXT_LINE="$(wait_for_different_zig_event_after "$BROWSER_C_START_LINE" "ca_context" "$B_PANE_ID" "browser C Zig ca_context")"
  C_PANE_ID="$(extract_pane_id "$C_CA_CONTEXT_LINE")"
  C_BROWSER_TAB_ID="$(extract_browser_tab_id "$C_CA_CONTEXT_LINE")"
  C_CONTEXT_ID="$(extract_context_id "$C_CA_CONTEXT_LINE")"
  [ -n "$C_PANE_ID" ] || fail "could not extract browser C pane id"
  [ -n "$C_BROWSER_TAB_ID" ] || fail "could not extract browser C tab id"
  [ -n "$C_CONTEXT_ID" ] || fail "could not extract browser C context id"
  [ "$C_PANE_ID" != "$A_PANE_ID" ] || fail "browser C reused browser A pane id"
  [ "$C_PANE_ID" != "$B_PANE_ID" ] || fail "browser C reused browser B pane id"
  [ "$C_BROWSER_TAB_ID" != "$A_BROWSER_TAB_ID" ] || fail "browser C reused browser A tab id"
  [ "$C_BROWSER_TAB_ID" != "$B_BROWSER_TAB_ID" ] || fail "browser C reused browser B tab id"
  [ "$C_CONTEXT_ID" != "$A_CONTEXT_ID" ] || fail "browser C reused browser A CA/context id"
  [ "$C_CONTEXT_ID" != "$B_CONTEXT_ID" ] || fail "browser C reused browser B CA/context id"
  log "browser_c_pane_id=$C_PANE_ID"
  log "browser_c_browser_tab_id=$C_BROWSER_TAB_ID"
  log "browser_c_context_id=$C_CONTEXT_ID"

  C_APPKIT_PRESENT_LINE="$(wait_for_line_after "$BROWSER_C_START_LINE" "TermSurf geometry layer=appkit event=presented .*pane_id:${C_PANE_ID} .*context_id=${C_CONTEXT_ID}" "browser C AppKit presentation")"
  C_APPKIT_PIXELS_LINE="$(wait_for_line_after "$BROWSER_C_START_LINE" "TermSurf geometry layer=appkit event=presented_pixels .*pane_id:${C_PANE_ID} .*context_id=${C_CONTEXT_ID}" "browser C AppKit pixels")"
  C_PRESENT_WINDOW_ID="$(printf '%s\n' "$C_APPKIT_PRESENT_LINE" | sed -E 's/.*window_id:([^ ]+) .*/\1/')"
  C_SURFACE_ID="$(extract_surface_id "$C_APPKIT_PRESENT_LINE")"
  C_SELECTED_TAB_ID="$(extract_selected_tab_id "$C_APPKIT_PRESENT_LINE")"
  C_FRAME="$(extract_overlay_frame "$C_APPKIT_PRESENT_LINE")"
  C_FRAME_SIZE="$(extract_frame_size "$C_APPKIT_PRESENT_LINE")"
  C_FRAME_X="$(extract_frame_x "$C_APPKIT_PRESENT_LINE")"
  C_FRAME_Y="$(extract_frame_y "$C_APPKIT_PRESENT_LINE")"
  C_ROOT_FRAME_SIZE="$(extract_root_frame_size "$C_APPKIT_PRESENT_LINE")"
  C_PIXEL="$(extract_appkit_pixel "$C_APPKIT_PIXELS_LINE")"
  [ "$C_PRESENT_WINDOW_ID" = "$C_WINDOW_ID" ] || fail "browser C AppKit window id mismatch: expected=$C_WINDOW_ID actual=$C_PRESENT_WINDOW_ID"
  [ -n "$C_SURFACE_ID" ] || fail "could not extract browser C AppKit surface id"
  [ "$C_SURFACE_ID" != "$A_SURFACE_ID" ] || fail "browser C reused browser A surface id"
  [ "$C_SURFACE_ID" != "$B_SURFACE_ID" ] || fail "browser C reused browser B surface id"
  [ "$C_SELECTED_TAB_ID" = "$C_WINDOW_ID" ] || fail "browser C selected tab id mismatch: expected=$C_WINDOW_ID actual=$C_SELECTED_TAB_ID"
  log "browser_c_window_id=$C_PRESENT_WINDOW_ID"
  log "browser_c_surface_id=$C_SURFACE_ID"
  log "browser_c_selected_tab_id=$C_SELECTED_TAB_ID"
  log "browser_c_overlay_frame=$C_FRAME"
  log "browser_c_appkit_pixel=$C_PIXEL"

  C_PIXEL_WIDTH="${C_PIXEL%x*}"
  C_PIXEL_HEIGHT="${C_PIXEL#*x}"
  require_trace_after "$BROWSER_C_TRACE_START_LINE" "resize tab_id=${C_BROWSER_TAB_ID} pane_id=${C_PANE_ID} pixel_width=${C_PIXEL_WIDTH} pixel_height=${C_PIXEL_HEIGHT} screen_x=0 screen_y=0 screen_width=0 screen_height=0 screen_scale=0 ffi=ts_set_view_size" "Roamium applied browser C resize to AppKit pixel size"

  if tail -n +"$((BROWSER_C_START_LINE + 1))" "$APP_LOG" |
    grep -E "TermSurf geometry layer=appkit event=presented .*window_id:${C_WINDOW_ID} .*pane_id:${A_PANE_ID} .*context_id=${A_CONTEXT_ID} .*visible=true" >/dev/null 2>&1; then
    fail "browser A overlay was presented as visible in window C"
  fi
  log "PASS: browser A was not presented as visible in window C"
  if tail -n +"$((BROWSER_C_START_LINE + 1))" "$APP_LOG" |
    grep -E "TermSurf geometry layer=appkit event=presented .*window_id:${C_WINDOW_ID} .*pane_id:${B_PANE_ID} .*context_id=${B_CONTEXT_ID} .*visible=true" >/dev/null 2>&1; then
    fail "browser B overlay was presented as visible in window C"
  fi
  log "PASS: browser B was not presented as visible in window C"

  screencapture -x -o -l"$A_WINDOW_ID" "$SCREENSHOT_WINDOW_A_RESTORED"
  log "window_a_alive_screenshot_exit=$?"
  screencapture -x -o -l"$B_WINDOW_ID" "$SCREENSHOT_WINDOW_B"
  log "window_b_screenshot_exit=$?"
  screencapture -x -o -l"$C_WINDOW_ID" "$SCREENSHOT_WINDOW_C"
  log "window_c_screenshot_exit=$?"

  C_CLICK_X="$(awk -v wx="$C_WX" -v frame_x="$C_FRAME_X" -v frame_size="$C_FRAME_SIZE" 'BEGIN { split(frame_size, parts, "x"); print int(wx + frame_x + (parts[1] / 2) + 0.5) }')"
  C_CLICK_Y="$(awk -v wy="$C_WY" -v frame_y="$C_FRAME_Y" -v frame_size="$C_FRAME_SIZE" 'BEGIN { split(frame_size, parts, "x"); print int(wy + frame_y + (parts[2] / 2) + 0.5) }')"
  C_HIT_START_LINE="$(log_line_count)"
  click_global_point "$C_CLICK_X" "$C_CLICK_Y" "browser_c_window_area"
  C_HIT_LINE="$(wait_for_hit_after "$C_HIT_START_LINE" "$C_CONTEXT_ID" "browser C hit-test in window C")"
  require_text "$C_HIT_LINE" "window_id:${C_WINDOW_ID}" "browser C hit-test has window C id"
  require_text "$C_HIT_LINE" "surface_id:${C_SURFACE_ID}" "browser C hit-test has surface C id"
  require_text "$C_HIT_LINE" "selected_tab_id:${C_SELECTED_TAB_ID}" "browser C hit-test has window C selected tab id"
  require_text "$C_HIT_LINE" "overlay_frame=${C_FRAME}" "browser C hit-test uses browser C frame"
  require_text "$C_HIT_LINE" "web_point={" "browser C hit-test includes webview-relative point"

  C_MODE_START_LINE="$(log_line_count)"
  C_MODE_TRACE_START_LINE="$(trace_line_count)"
  log "browser_c_mode_key=enter=Mode::Browse"
  swift "$ROOT/scripts/ghostty-app/inject.swift" key 36 >>"$HARNESS_LOG" 2>&1
  wait_for_log_after "$C_MODE_START_LINE" "ModeChanged: pane_id=${C_PANE_ID} browsing=true" "browser C webtui entered browse mode"
  require_trace_after "$C_MODE_TRACE_START_LINE" "focus-changed tab=${C_BROWSER_TAB_ID} pane=${C_PANE_ID} ffi=ts_set_focus focused=true" "Roamium observed browser C focus=true after browse mode"

  C_KEY_START_LINE="$(trace_line_count)"
  printf 'ISSUE809_EXP16_BROWSER_C_WINDOW\n' >"$BROWSER_FOCUS_COMMAND"
  swift "$ROOT/scripts/ghostty-app/inject.swift" type "$BROWSER_FOCUS_COMMAND" >>"$HARNESS_LOG" 2>&1
  require_trace_after "$C_KEY_START_LINE" "key-event tab=${C_BROWSER_TAB_ID} pane=${C_PANE_ID}" "browser C keyboard marker reached browser C"
  require_no_trace_after "$C_KEY_START_LINE" "key-event tab=${A_BROWSER_TAB_ID} pane=${A_PANE_ID}" "browser C keyboard marker did not reach browser A"
  require_no_trace_after "$C_KEY_START_LINE" "key-event tab=${B_BROWSER_TAB_ID} pane=${B_PANE_ID}" "browser C keyboard marker did not reach browser B"

  C_CONTROL_START_LINE="$(log_line_count)"
  C_CONTROL_TRACE_START_LINE="$(trace_line_count)"
  log "browser_c_control_key=escape=Mode::Control"
  swift "$ROOT/scripts/ghostty-app/inject.swift" key 53 >>"$HARNESS_LOG" 2>&1
  wait_for_log_after "$C_CONTROL_START_LINE" "ModeChanged: pane_id=${C_PANE_ID} browsing=false" "browser C webtui returned to control mode"
  require_trace_after "$C_CONTROL_TRACE_START_LINE" "focus-changed tab=${C_BROWSER_TAB_ID} pane=${C_PANE_ID} ffi=ts_set_focus focused=false" "Roamium observed browser C focus=false after control mode"

  B_RESTORE_START_LINE="$(log_line_count)"
  B_RESTORE_TRACE_START_LINE="$(trace_line_count)"
  B_WIN_LINE="$(window_bounds_for "$B_WINDOW_ID")" || fail "failed to resolve browser B window bounds for window id=$B_WINDOW_ID"
  focus_window_bounds "$B_WIN_LINE" >>"$HARNESS_LOG" 2>&1 || fail "failed to focus browser B window via accessibility"
  IFS=$'\t' read -r _B_WID B_WX B_WY B_WW B_WH <<<"$B_WIN_LINE"
  B_ROOT_HEIGHT="$(pair_height "$B_ROOT_FRAME_SIZE")"
  B_CONTENT_Y_OFFSET="$(awk -v wh="$B_WH" -v root_h="$B_ROOT_HEIGHT" 'BEGIN { print int(wh - root_h) }')"
  B_VISIBLE_X="$(awk -v wx="$B_WX" -v frame_x="$B_FRAME_X" 'BEGIN { print int(wx + frame_x + 4) }')"
  B_VISIBLE_Y="$(awk -v wy="$B_WY" -v content_y="$B_CONTENT_Y_OFFSET" -v frame_y="$B_FRAME_Y" 'BEGIN { print int(wy + content_y + frame_y + 4) }')"
  B_HIT_START_LINE="$(log_line_count)"
  click_global_point "$B_VISIBLE_X" "$B_VISIBLE_Y" "window_b_visible_edge_restore"
  B_HIT_LINE="$(wait_for_hit_after "$B_HIT_START_LINE" "$B_CONTEXT_ID" "browser B restored visible hit-test in window B")"
  require_no_different_appkit_frame_after "$B_RESTORE_START_LINE" "$B_PANE_ID" "$B_CONTEXT_ID" "$B_FRAME" "browser B kept AppKit frame after returning to window B"
  require_no_different_appkit_pixels_after "$B_RESTORE_START_LINE" "$B_PANE_ID" "$B_CONTEXT_ID" "$B_PIXEL" "browser B kept AppKit pixels after returning to window B"
  require_text "$B_HIT_LINE" "window_id:${B_WINDOW_ID}" "browser B restored hit-test has window B id"
  require_text "$B_HIT_LINE" "surface_id:${B_SURFACE_ID}" "browser B restored hit-test has surface B id"
  require_text "$B_HIT_LINE" "selected_tab_id:${B_SELECTED_TAB_ID}" "browser B restored hit-test has selected tab B id"
  require_text "$B_HIT_LINE" "overlay_frame=${B_FRAME}" "browser B restored hit-test uses browser B frame"
  require_text "$B_HIT_LINE" "web_point={" "browser B restored hit-test includes webview-relative point"

  B_MODE_START_LINE="$(log_line_count)"
  B_MODE_TRACE_START_LINE="$(trace_line_count)"
  log "browser_b_mode_key=enter=Mode::Browse"
  swift "$ROOT/scripts/ghostty-app/inject.swift" key 36 >>"$HARNESS_LOG" 2>&1
  wait_for_log_after "$B_MODE_START_LINE" "ModeChanged: pane_id=${B_PANE_ID} browsing=true" "browser B webtui entered browse mode after window restore"
  require_trace_after "$B_MODE_TRACE_START_LINE" "focus-changed tab=${B_BROWSER_TAB_ID} pane=${B_PANE_ID} ffi=ts_set_focus focused=true" "Roamium observed browser B focus=true after window restore"

  B_KEY_START_LINE="$(trace_line_count)"
  printf 'ISSUE809_EXP16_BROWSER_B_WINDOW\n' >"$BROWSER_FOCUS_COMMAND"
  swift "$ROOT/scripts/ghostty-app/inject.swift" type "$BROWSER_FOCUS_COMMAND" >>"$HARNESS_LOG" 2>&1
  require_trace_after "$B_KEY_START_LINE" "key-event tab=${B_BROWSER_TAB_ID} pane=${B_PANE_ID}" "browser B keyboard marker reached browser B"
  require_no_trace_after "$B_KEY_START_LINE" "key-event tab=${A_BROWSER_TAB_ID} pane=${A_PANE_ID}" "browser B keyboard marker did not reach browser A"
  require_no_trace_after "$B_KEY_START_LINE" "key-event tab=${C_BROWSER_TAB_ID} pane=${C_PANE_ID}" "browser B keyboard marker did not reach browser C"

  B_CONTROL_START_LINE="$(log_line_count)"
  B_CONTROL_TRACE_START_LINE="$(trace_line_count)"
  log "browser_b_control_key=escape=Mode::Control"
  swift "$ROOT/scripts/ghostty-app/inject.swift" key 53 >>"$HARNESS_LOG" 2>&1
  wait_for_log_after "$B_CONTROL_START_LINE" "ModeChanged: pane_id=${B_PANE_ID} browsing=false" "browser B webtui returned to control mode"
  require_trace_after "$B_CONTROL_TRACE_START_LINE" "focus-changed tab=${B_BROWSER_TAB_ID} pane=${B_PANE_ID} ffi=ts_set_focus focused=false" "Roamium observed browser B focus=false after control mode"
  screencapture -x -o -l"$B_WINDOW_ID" "$SCREENSHOT_WINDOW_B_RESTORED"
  log "window_b_restored_screenshot_exit=$?"

  A_RESTORE_START_LINE="$(log_line_count)"
  A_RESTORE_TRACE_START_LINE="$(trace_line_count)"
  A_WIN_LINE="$(window_bounds_for "$A_WINDOW_ID")" || fail "failed to resolve browser A window bounds for window id=$A_WINDOW_ID"
  focus_window_bounds "$A_WIN_LINE" >>"$HARNESS_LOG" 2>&1 || fail "failed to focus browser A window via accessibility"
  IFS=$'\t' read -r _A_WID A_WX A_WY A_WW A_WH <<<"$A_WIN_LINE"
  A_ROOT_HEIGHT="$(pair_height "$A_ROOT_FRAME_SIZE")"
  A_CONTENT_Y_OFFSET="$(awk -v wh="$A_WH" -v root_h="$A_ROOT_HEIGHT" 'BEGIN { print int(wh - root_h) }')"
  A_VISIBLE_X="$(awk -v wx="$A_WX" -v frame_x="$A_FRAME_X" 'BEGIN { print int(wx + frame_x + 4) }')"
  A_VISIBLE_Y="$(awk -v wy="$A_WY" -v content_y="$A_CONTENT_Y_OFFSET" -v frame_y="$A_FRAME_Y" 'BEGIN { print int(wy + content_y + frame_y + 4) }')"
  A_HIT_START_LINE="$(log_line_count)"
  click_global_point "$A_VISIBLE_X" "$A_VISIBLE_Y" "window_a_visible_edge_restore"
  A_HIT_LINE="$(wait_for_hit_after "$A_HIT_START_LINE" "$A_CONTEXT_ID" "browser A restored visible hit-test in window A")"
  require_no_different_appkit_frame_after "$A_RESTORE_START_LINE" "$A_PANE_ID" "$A_CONTEXT_ID" "$A_FRAME" "browser A kept AppKit frame after returning to window A"
  require_no_different_appkit_pixels_after "$A_RESTORE_START_LINE" "$A_PANE_ID" "$A_CONTEXT_ID" "$A_PIXEL" "browser A kept AppKit pixels after returning to window A"
  require_text "$A_HIT_LINE" "window_id:${A_WINDOW_ID}" "browser A restored hit-test has window A id"
  require_text "$A_HIT_LINE" "surface_id:${A_SURFACE_ID}" "browser A restored hit-test has surface A id"
  require_text "$A_HIT_LINE" "selected_tab_id:${A_SELECTED_TAB_ID}" "browser A restored hit-test has selected tab A id"
  require_text "$A_HIT_LINE" "overlay_frame=${A_FRAME}" "browser A restored hit-test uses browser A frame"
  require_text "$A_HIT_LINE" "web_point={" "browser A restored hit-test includes webview-relative point"

  A_MODE_START_LINE="$(log_line_count)"
  A_MODE_TRACE_START_LINE="$(trace_line_count)"
  log "browser_a_mode_key=enter=Mode::Browse"
  swift "$ROOT/scripts/ghostty-app/inject.swift" key 36 >>"$HARNESS_LOG" 2>&1
  wait_for_log_after "$A_MODE_START_LINE" "ModeChanged: pane_id=${A_PANE_ID} browsing=true" "browser A webtui entered browse mode after window restore"
  require_trace_after "$A_MODE_TRACE_START_LINE" "focus-changed tab=${A_BROWSER_TAB_ID} pane=${A_PANE_ID} ffi=ts_set_focus focused=true" "Roamium observed browser A focus=true after window restore"

  A_KEY_START_LINE="$(trace_line_count)"
  printf 'ISSUE809_EXP16_BROWSER_A_WINDOW\n' >"$BROWSER_FOCUS_COMMAND"
  swift "$ROOT/scripts/ghostty-app/inject.swift" type "$BROWSER_FOCUS_COMMAND" >>"$HARNESS_LOG" 2>&1
  require_trace_after "$A_KEY_START_LINE" "key-event tab=${A_BROWSER_TAB_ID} pane=${A_PANE_ID}" "browser A keyboard marker reached browser A"
  require_no_trace_after "$A_KEY_START_LINE" "key-event tab=${B_BROWSER_TAB_ID} pane=${B_PANE_ID}" "browser A keyboard marker did not reach browser B"
  require_no_trace_after "$A_KEY_START_LINE" "key-event tab=${C_BROWSER_TAB_ID} pane=${C_PANE_ID}" "browser A keyboard marker did not reach browser C"

  screencapture -x -o -l"$A_WINDOW_ID" "$SCREENSHOT_WINDOW_A_RESTORED"
  log "window_a_restored_screenshot_exit=$?"

  [ "$NEW_WINDOW_B_TRACE_START_LINE" -lt "$BROWSER_B_TRACE_START_LINE" ] || fail "trace boundaries for browser B window open were not monotonic"
  [ "$BROWSER_B_TRACE_START_LINE" -lt "$NEW_WINDOW_C_TRACE_START_LINE" ] || fail "trace boundaries for window C open were not monotonic"
  [ "$NEW_WINDOW_C_TRACE_START_LINE" -lt "$BROWSER_C_TRACE_START_LINE" ] || fail "trace boundaries for browser C open were not monotonic"
  [ "$BROWSER_C_TRACE_START_LINE" -lt "$B_RESTORE_TRACE_START_LINE" ] || fail "trace boundaries for browser B window restore were not monotonic"
  [ "$B_RESTORE_TRACE_START_LINE" -lt "$A_RESTORE_TRACE_START_LINE" ] || fail "trace boundaries for browser A window restore were not monotonic"
fi

if [ "$SCENARIO" = "new-terminal-tab-visibility" ]; then
  BASE_SELECTED_TAB_ID="$(printf '%s\n' "$APPKIT_PRESENT_LINE" | sed -E 's/.*selected_tab_id:([^ ]+) .*/\1/')"
  [ -n "$BASE_SELECTED_TAB_ID" ] || fail "could not extract baseline selected tab id"
  BASE_FRAME="$OVERLAY_FRAME"
  BASE_FRAME_SIZE="$OVERLAY_FRAME_SIZE"
  BASE_FRAME_X="$OVERLAY_FRAME_X"
  BASE_FRAME_Y="$OVERLAY_FRAME_Y"
  BASE_PIXEL="$APPKIT_PIXEL"
  log "baseline_selected_tab_id=$BASE_SELECTED_TAB_ID"

  NEW_TAB_START_LINE="$(log_line_count)"
  NEW_TAB_TRACE_START_LINE="$(trace_line_count)"
  log "new_tab_keybind=ctrl+t=new_tab"
  swift "$ROOT/scripts/ghostty-app/inject.swift" key 17 control >>"$HARNESS_LOG" 2>&1
  delay 2

  require_log_after "$NEW_TAB_START_LINE" "dispatching action target=surface action=.new_tab" "new terminal tab action dispatched"
  require_log_after "$NEW_TAB_START_LINE" 'starting command command=`/usr/bin/login`' "new terminal tab started plain login shell"
  if [ -s "$NEW_TAB_COMMAND_LOG" ]; then
    fail "new terminal tab unexpectedly inherited and ran the first-run web wrapper"
  fi
  log "PASS: new terminal tab did not inherit the first-run web wrapper"

  TABBED_PRESENT_LINE="$(wait_for_changed_appkit_frame_after "$NEW_TAB_START_LINE" "$PANE_ID" "$CONTEXT_ID" "$BASE_FRAME" "browser tab geometry adjusted for native tab bar")"
  TABBED_PIXELS_LINE="$(wait_for_changed_appkit_pixels_after "$NEW_TAB_START_LINE" "$PANE_ID" "$CONTEXT_ID" "$BASE_PIXEL" "browser tab AppKit pixels adjusted for native tab bar")"
  TABBED_FRAME="$(extract_overlay_frame "$TABBED_PRESENT_LINE")"
  TABBED_FRAME_SIZE="$(extract_frame_size "$TABBED_PRESENT_LINE")"
  TABBED_FRAME_X="$(extract_frame_x "$TABBED_PRESENT_LINE")"
  TABBED_FRAME_Y="$(extract_frame_y "$TABBED_PRESENT_LINE")"
  TABBED_PIXEL="$(extract_appkit_pixel "$TABBED_PIXELS_LINE")"
  log "tabbed_overlay_frame=$TABBED_FRAME"
  log "tabbed_overlay_frame_size=$TABBED_FRAME_SIZE"
  log "tabbed_appkit_pixel=$TABBED_PIXEL"

  NEW_TAB_SELECT_START_LINE="$(log_line_count)"
  log "select_new_tab_keybind=ctrl+2=goto_tab:2"
  swift "$ROOT/scripts/ghostty-app/inject.swift" key 19 control >>"$HARNESS_LOG" 2>&1
  delay 1

  NEW_SELECTED_TAB_LINE="$(wait_for_selected_tab_change_after "$NEW_TAB_SELECT_START_LINE" "$BASE_SELECTED_TAB_ID" "new terminal tab selected")"
  NEW_SELECTED_TAB_ID="$(extract_selected_tab_id "$NEW_SELECTED_TAB_LINE")"
  [ -n "$NEW_SELECTED_TAB_ID" ] || fail "could not extract selected tab id for new terminal tab"
  log "PASS: new terminal tab selected"
  log "new_selected_tab_id=$NEW_SELECTED_TAB_ID"

  if tail -n +"$((NEW_TAB_START_LINE + 1))" "$APP_LOG" |
    grep -E "TermSurf geometry layer=zig event=(tab_ready|ca_context) " |
    grep -Fv "pane_id:${PANE_ID}" >/dev/null 2>&1; then
    fail "new terminal tab created a second browser pane/context"
  fi
  log "PASS: new terminal tab did not create a second browser pane/context"
  if tail -n +"$((NEW_TAB_TRACE_START_LINE + 1))" "$ROAMIUM_TRACE" |
    grep -E "resize tab_id=|title-changed tab=|key-event tab=|mouse-event tab=|mouse-move tab=" |
    grep -Fv "pane_id=${PANE_ID}" |
    grep -Fv "pane=${PANE_ID}" >/dev/null 2>&1; then
    fail "Roamium trace shows activity for a second browser context after new tab"
  fi
  log "PASS: Roamium trace shows no second browser context after new tab"

  screencapture -x -o -l"$NEW_SELECTED_TAB_ID" "$SCREENSHOT_TAB_NEW"
  log "new_tab_screenshot_exit=$?"

  if tail -n +"$((NEW_TAB_SELECT_START_LINE + 1))" "$APP_LOG" |
    grep -E "TermSurf geometry layer=appkit event=presented .*pane_id:${PANE_ID} .*context_id=${CONTEXT_ID} .*visible=true .*selected_tab_id:${NEW_SELECTED_TAB_ID}" >/dev/null 2>&1; then
    fail "original browser overlay was presented as visible in the selected new tab"
  fi
  log "PASS: original browser overlay was not freshly presented as visible in the selected new tab"

  TAB_WIN_LINE="$(window_bounds_for "$NEW_SELECTED_TAB_ID")" || fail "failed to resolve new-tab window bounds for window id=$NEW_SELECTED_TAB_ID"
  IFS=$'\t' read -r _TAB_WID TAB_WX TAB_WY TAB_WW TAB_WH <<<"$TAB_WIN_LINE"
  TAB_BROWSER_X="$(awk -v wx="$TAB_WX" -v frame_x="$TABBED_FRAME_X" -v frame_size="$TABBED_FRAME_SIZE" 'BEGIN { split(frame_size, parts, "x"); print int(wx + frame_x + (parts[1] / 2) + 0.5) }')"
  TAB_BROWSER_Y="$(awk -v wy="$TAB_WY" -v frame_y="$TABBED_FRAME_Y" -v frame_size="$TABBED_FRAME_SIZE" 'BEGIN { split(frame_size, parts, "x"); print int(wy + frame_y + (parts[2] / 2) + 0.5) }')"

  click_negative_global_point "$TAB_BROWSER_X" "$TAB_BROWSER_Y" "new_tab_former_browser_area"
  wait_for_negative_hit_after "$NEGATIVE_HIT_START_LINE" "$CONTEXT_ID" "new terminal tab former browser area negative hit-test" allow-absent

  NEW_TAB_KEY_START_LINE="$(trace_line_count)"
  printf 'ISSUE809_EXP12_NEW_TAB_TERMINAL\n' >"$NEW_TAB_MARKER_COMMAND"
  swift "$ROOT/scripts/ghostty-app/inject.swift" type "$NEW_TAB_MARKER_COMMAND" >>"$HARNESS_LOG" 2>&1
  delay 1
  require_no_trace_after "$NEW_TAB_KEY_START_LINE" "key-event tab=${BROWSER_TAB_ID} pane=${PANE_ID}" "new terminal tab keyboard marker did not reach original browser context"

  SWITCH_BACK_START_LINE="$(log_line_count)"
  SWITCH_BACK_TRACE_START_LINE="$(trace_line_count)"
  log "switch_back_keybind=ctrl+p=previous_tab"
  swift "$ROOT/scripts/ghostty-app/inject.swift" key 35 control >>"$HARNESS_LOG" 2>&1
  delay 1

  wait_for_log_after "$SWITCH_BACK_START_LINE" "Pane focus changed: pane_id=${PANE_ID} focused=true" "original browser pane focused again after tab switch"
  require_no_different_appkit_frame_after "$SWITCH_BACK_START_LINE" "$PANE_ID" "$CONTEXT_ID" "$TABBED_FRAME" "browser tab kept tab-bar-adjusted AppKit frame after switch back"
  require_no_different_appkit_pixels_after "$SWITCH_BACK_START_LINE" "$PANE_ID" "$CONTEXT_ID" "$TABBED_PIXEL" "browser tab kept tab-bar-adjusted AppKit pixels after switch back"

  BROWSER_RESTORE_HIT_START_LINE="$(log_line_count)"
  TAB_WIN_LINE="$(window_bounds_for "$BASE_SELECTED_TAB_ID")" || fail "failed to resolve restored browser-tab window bounds for window id=$BASE_SELECTED_TAB_ID"
  IFS=$'\t' read -r _TAB_WID TAB_WX TAB_WY TAB_WW TAB_WH <<<"$TAB_WIN_LINE"
  TAB_BROWSER_X="$(awk -v wx="$TAB_WX" -v frame_x="$TABBED_FRAME_X" -v frame_size="$TABBED_FRAME_SIZE" 'BEGIN { split(frame_size, parts, "x"); print int(wx + frame_x + (parts[1] / 2) + 0.5) }')"
  TAB_BROWSER_Y="$(awk -v wy="$TAB_WY" -v frame_y="$TABBED_FRAME_Y" -v frame_size="$TABBED_FRAME_SIZE" 'BEGIN { split(frame_size, parts, "x"); print int(wy + frame_y + (parts[2] / 2) + 0.5) }')"
  click_global_point "$TAB_BROWSER_X" "$TAB_BROWSER_Y" "restored_browser_area"
  RESTORE_HIT_LINE="$(wait_for_hit_after "$BROWSER_RESTORE_HIT_START_LINE" "$CONTEXT_ID" "restored browser tab hit-test")"
  RESTORE_HIT_FRAME_SIZE="$(extract_frame_size "$RESTORE_HIT_LINE")"
  RESTORE_HIT_FRAME_X="$(extract_frame_x "$RESTORE_HIT_LINE")"
  RESTORE_HIT_FRAME_Y="$(extract_frame_y "$RESTORE_HIT_LINE")"
  require_text "$RESTORE_HIT_LINE" "selected_tab_id:${BASE_SELECTED_TAB_ID}" "restored browser hit-test has original selected tab id"
  [ "$RESTORE_HIT_FRAME_SIZE" = "$TABBED_FRAME_SIZE" ] || fail "restored hit-test frame size changed: expected=$TABBED_FRAME_SIZE actual=$RESTORE_HIT_FRAME_SIZE"
  [ "$RESTORE_HIT_FRAME_X" = "$TABBED_FRAME_X" ] || fail "restored hit-test frame x changed: expected=$TABBED_FRAME_X actual=$RESTORE_HIT_FRAME_X"
  [ "$RESTORE_HIT_FRAME_Y" = "$TABBED_FRAME_Y" ] || fail "restored hit-test frame y changed: expected=$TABBED_FRAME_Y actual=$RESTORE_HIT_FRAME_Y"
  require_text "$RESTORE_HIT_LINE" "web_point={" "restored browser hit-test includes webview-relative point"
  log "PASS: restored browser hit-test uses tab-bar-adjusted overlay frame"

  BROWSER_MODE_START_LINE="$(log_line_count)"
  BROWSER_MODE_TRACE_START_LINE="$(trace_line_count)"
  log "restored_browser_mode_key=enter=Mode::Browse"
  swift "$ROOT/scripts/ghostty-app/inject.swift" key 36 >>"$HARNESS_LOG" 2>&1
  wait_for_log_after "$BROWSER_MODE_START_LINE" "ModeChanged: pane_id=${PANE_ID} browsing=true" "webtui entered browse mode after tab restore"
  require_trace_after "$BROWSER_MODE_TRACE_START_LINE" "focus-changed tab=${BROWSER_TAB_ID} pane=${PANE_ID} ffi=ts_set_focus focused=true" "Roamium observed restored browser pane focus=true after browse mode"

  BROWSER_KEY_START_LINE="$(trace_line_count)"
  printf 'ISSUE809_EXP12_BROWSER_RESTORED\n' >"$BROWSER_FOCUS_COMMAND"
  swift "$ROOT/scripts/ghostty-app/inject.swift" type "$BROWSER_FOCUS_COMMAND" >>"$HARNESS_LOG" 2>&1
  BROWSER_KEY_SEEN=""
  for _ in $(seq 1 10); do
    if tail -n +"$((BROWSER_KEY_START_LINE + 1))" "$ROAMIUM_TRACE" | grep -F "key-event tab=${BROWSER_TAB_ID} pane=${PANE_ID}" >/dev/null 2>&1; then
      BROWSER_KEY_SEEN="1"
      break
    fi
    delay 1
  done
  [ -n "$BROWSER_KEY_SEEN" ] || fail "restored browser tab keyboard marker did not reach original browser context"
  log "PASS: restored browser tab keyboard marker reached original browser context"

  screencapture -x -o -l"$WID" "$SCREENSHOT_TAB_BACK"
  log "back_tab_screenshot_exit=$?"
fi

if [ "$SCENARIO" = "open-browser-in-new-tab" ] || [ "$SCENARIO" = "close-browser-tab" ]; then
  A_SELECTED_TAB_ID="$(extract_selected_tab_id "$APPKIT_PRESENT_LINE")"
  [ -n "$A_SELECTED_TAB_ID" ] || fail "could not extract browser A selected tab id"
  A_PANE_ID="$PANE_ID"
  A_BROWSER_TAB_ID="$BROWSER_TAB_ID"
  A_CONTEXT_ID="$CONTEXT_ID"
  A_FRAME="$OVERLAY_FRAME"
  A_FRAME_SIZE="$OVERLAY_FRAME_SIZE"
  A_FRAME_X="$OVERLAY_FRAME_X"
  A_FRAME_Y="$OVERLAY_FRAME_Y"
  A_PIXEL="$APPKIT_PIXEL"
  log "browser_a_selected_tab_id=$A_SELECTED_TAB_ID"
  log "browser_a_pane_id=$A_PANE_ID"
  log "browser_a_browser_tab_id=$A_BROWSER_TAB_ID"
  log "browser_a_context_id=$A_CONTEXT_ID"

  NEW_TAB_START_LINE="$(log_line_count)"
  NEW_TAB_TRACE_START_LINE="$(trace_line_count)"
  log "new_tab_keybind=ctrl+t=new_tab"
  swift "$ROOT/scripts/ghostty-app/inject.swift" key 17 control >>"$HARNESS_LOG" 2>&1
  delay 2

  require_log_after "$NEW_TAB_START_LINE" "dispatching action target=surface action=.new_tab" "new terminal tab action dispatched"
  require_log_after "$NEW_TAB_START_LINE" 'starting command command=`/usr/bin/login`' "new terminal tab started plain login shell"
  if [ -s "$NEW_TAB_COMMAND_LOG" ]; then
    fail "new terminal tab unexpectedly inherited and ran the first-run web wrapper"
  fi
  log "PASS: new terminal tab did not inherit the first-run web wrapper"

  A_TABBED_PRESENT_LINE="$(wait_for_changed_appkit_frame_after "$NEW_TAB_START_LINE" "$A_PANE_ID" "$A_CONTEXT_ID" "$A_FRAME" "browser A geometry adjusted for native tab bar")"
  A_TABBED_PIXELS_LINE="$(wait_for_changed_appkit_pixels_after "$NEW_TAB_START_LINE" "$A_PANE_ID" "$A_CONTEXT_ID" "$A_PIXEL" "browser A AppKit pixels adjusted for native tab bar")"
  A_TABBED_FRAME="$(extract_overlay_frame "$A_TABBED_PRESENT_LINE")"
  A_TABBED_FRAME_SIZE="$(extract_frame_size "$A_TABBED_PRESENT_LINE")"
  A_TABBED_FRAME_X="$(extract_frame_x "$A_TABBED_PRESENT_LINE")"
  A_TABBED_FRAME_Y="$(extract_frame_y "$A_TABBED_PRESENT_LINE")"
  A_TABBED_PIXEL="$(extract_appkit_pixel "$A_TABBED_PIXELS_LINE")"
  log "browser_a_tabbed_overlay_frame=$A_TABBED_FRAME"
  log "browser_a_tabbed_overlay_frame_size=$A_TABBED_FRAME_SIZE"
  log "browser_a_tabbed_appkit_pixel=$A_TABBED_PIXEL"

  TAB2_SELECT_START_LINE="$(log_line_count)"
  log "select_tab2_keybind=ctrl+2=goto_tab:2"
  swift "$ROOT/scripts/ghostty-app/inject.swift" key 19 control >>"$HARNESS_LOG" 2>&1
  delay 1

  TAB2_SELECTED_LINE="$(wait_for_selected_tab_change_after "$TAB2_SELECT_START_LINE" "$A_SELECTED_TAB_ID" "tab 2 selected")"
  TAB2_SELECTED_TAB_ID="$(extract_selected_tab_id "$TAB2_SELECTED_LINE")"
  [ -n "$TAB2_SELECTED_TAB_ID" ] || fail "could not extract tab 2 selected tab id"
  log "PASS: tab 2 selected"
  log "tab2_selected_tab_id=$TAB2_SELECTED_TAB_ID"

  screencapture -x -o -l"$TAB2_SELECTED_TAB_ID" "$SCREENSHOT_TAB_NEW"
  log "new_tab_screenshot_exit=$?"

  if tail -n +"$((TAB2_SELECT_START_LINE + 1))" "$APP_LOG" |
    grep -E "TermSurf geometry layer=appkit event=presented .*pane_id:${A_PANE_ID} .*context_id=${A_CONTEXT_ID} .*visible=true .*selected_tab_id:${TAB2_SELECTED_TAB_ID}" >/dev/null 2>&1; then
    fail "browser A overlay was presented as visible in selected tab 2"
  fi
  log "PASS: browser A was not freshly presented as visible in selected tab 2"

  TAB2_WIN_LINE="$(window_bounds_for "$TAB2_SELECTED_TAB_ID")" || fail "failed to resolve tab 2 window bounds for window id=$TAB2_SELECTED_TAB_ID"
  IFS=$'\t' read -r _TAB2_WID TAB2_WX TAB2_WY TAB2_WW TAB2_WH <<<"$TAB2_WIN_LINE"
  TAB2_A_X="$(awk -v wx="$TAB2_WX" -v frame_x="$A_TABBED_FRAME_X" -v frame_size="$A_TABBED_FRAME_SIZE" 'BEGIN { split(frame_size, parts, "x"); print int(wx + frame_x + (parts[1] / 2) + 0.5) }')"
  TAB2_A_Y="$(awk -v wy="$TAB2_WY" -v frame_y="$A_TABBED_FRAME_Y" -v frame_size="$A_TABBED_FRAME_SIZE" 'BEGIN { split(frame_size, parts, "x"); print int(wy + frame_y + (parts[2] / 2) + 0.5) }')"
  click_negative_global_point "$TAB2_A_X" "$TAB2_A_Y" "tab2_former_browser_a_area"
  wait_for_negative_hit_after "$NEGATIVE_HIT_START_LINE" "$A_CONTEXT_ID" "tab 2 former browser A area negative hit-test" allow-absent

  TAB2_KEY_START_LINE="$(trace_line_count)"
  printf 'ISSUE809_EXP13_TAB2_TERMINAL\n' >"$NEW_TAB_MARKER_COMMAND"
  swift "$ROOT/scripts/ghostty-app/inject.swift" type "$NEW_TAB_MARKER_COMMAND" >>"$HARNESS_LOG" 2>&1
  swift "$ROOT/scripts/ghostty-app/inject.swift" key 36 >>"$HARNESS_LOG" 2>&1
  delay 1
  require_no_trace_after "$TAB2_KEY_START_LINE" "key-event tab=${A_BROWSER_TAB_ID} pane=${A_PANE_ID}" "tab 2 terminal keyboard marker did not reach browser A"

  BROWSER_B_START_LINE="$(log_line_count)"
  BROWSER_B_TRACE_START_LINE="$(trace_line_count)"
  printf '"%s" --browser "%s" "%s"' "$WEB" "$ROAMIUM" "$URL_B" >"$SECOND_BROWSER_COMMAND"
  log "browser_b_command=$(cat "$SECOND_BROWSER_COMMAND")"
  swift "$ROOT/scripts/ghostty-app/inject.swift" type "$SECOND_BROWSER_COMMAND" >>"$HARNESS_LOG" 2>&1
  swift "$ROOT/scripts/ghostty-app/inject.swift" key 36 >>"$HARNESS_LOG" 2>&1

  B_CA_CONTEXT_LINE="$(wait_for_different_zig_event_after "$BROWSER_B_START_LINE" "ca_context" "$A_PANE_ID" "browser B Zig ca_context")"
  B_PANE_ID="$(extract_pane_id "$B_CA_CONTEXT_LINE")"
  B_BROWSER_TAB_ID="$(extract_browser_tab_id "$B_CA_CONTEXT_LINE")"
  B_CONTEXT_ID="$(extract_context_id "$B_CA_CONTEXT_LINE")"
  [ -n "$B_PANE_ID" ] || fail "could not extract browser B pane id"
  [ -n "$B_BROWSER_TAB_ID" ] || fail "could not extract browser B tab id"
  [ -n "$B_CONTEXT_ID" ] || fail "could not extract browser B context id"
  [ "$B_PANE_ID" != "$A_PANE_ID" ] || fail "browser B reused browser A pane id"
  [ "$B_BROWSER_TAB_ID" != "$A_BROWSER_TAB_ID" ] || fail "browser B reused browser A tab id"
  [ "$B_CONTEXT_ID" != "$A_CONTEXT_ID" ] || fail "browser B reused browser A CA/context id"
  log "browser_b_pane_id=$B_PANE_ID"
  log "browser_b_browser_tab_id=$B_BROWSER_TAB_ID"
  log "browser_b_context_id=$B_CONTEXT_ID"

  B_APPKIT_PRESENT_LINE="$(wait_for_line_after "$BROWSER_B_START_LINE" "TermSurf geometry layer=appkit event=presented .*pane_id:${B_PANE_ID} .*context_id=${B_CONTEXT_ID}" "browser B AppKit presentation")"
  B_APPKIT_PIXELS_LINE="$(wait_for_line_after "$BROWSER_B_START_LINE" "TermSurf geometry layer=appkit event=presented_pixels .*pane_id:${B_PANE_ID} .*context_id=${B_CONTEXT_ID}" "browser B AppKit pixels")"
  B_SELECTED_TAB_ID="$(extract_selected_tab_id "$B_APPKIT_PRESENT_LINE")"
  B_FRAME="$(extract_overlay_frame "$B_APPKIT_PRESENT_LINE")"
  B_FRAME_SIZE="$(extract_frame_size "$B_APPKIT_PRESENT_LINE")"
  B_FRAME_X="$(extract_frame_x "$B_APPKIT_PRESENT_LINE")"
  B_FRAME_Y="$(extract_frame_y "$B_APPKIT_PRESENT_LINE")"
  B_PIXEL="$(extract_appkit_pixel "$B_APPKIT_PIXELS_LINE")"
  [ "$B_SELECTED_TAB_ID" = "$TAB2_SELECTED_TAB_ID" ] || fail "browser B selected tab mismatch: expected=$TAB2_SELECTED_TAB_ID actual=$B_SELECTED_TAB_ID"
  log "browser_b_selected_tab_id=$B_SELECTED_TAB_ID"
  log "browser_b_overlay_frame=$B_FRAME"
  log "browser_b_overlay_frame_size=$B_FRAME_SIZE"
  log "browser_b_appkit_pixel=$B_PIXEL"

  B_PIXEL_WIDTH="${B_PIXEL%x*}"
  B_PIXEL_HEIGHT="${B_PIXEL#*x}"
  require_trace_after "$BROWSER_B_TRACE_START_LINE" "resize tab_id=${B_BROWSER_TAB_ID} pane_id=${B_PANE_ID} pixel_width=${B_PIXEL_WIDTH} pixel_height=${B_PIXEL_HEIGHT} screen_x=0 screen_y=0 screen_width=0 screen_height=0 screen_scale=0 ffi=ts_set_view_size" "Roamium applied browser B resize to AppKit pixel size"

  if tail -n +"$((BROWSER_B_START_LINE + 1))" "$APP_LOG" |
    grep -E "TermSurf geometry layer=appkit event=presented .*pane_id:${A_PANE_ID} .*context_id=${A_CONTEXT_ID} .*visible=true .*selected_tab_id:${TAB2_SELECTED_TAB_ID}" >/dev/null 2>&1; then
    fail "browser A overlay was presented as visible in tab 2 after browser B opened"
  fi
  log "PASS: browser A stayed hidden after browser B opened in tab 2"

  screencapture -x -o -l"$TAB2_SELECTED_TAB_ID" "$SCREENSHOT_TAB_BROWSER_B"
  log "browser_b_screenshot_exit=$?"

  B_CLICK_X="$(awk -v wx="$TAB2_WX" -v frame_x="$B_FRAME_X" -v frame_size="$B_FRAME_SIZE" 'BEGIN { split(frame_size, parts, "x"); print int(wx + frame_x + (parts[1] / 2) + 0.5) }')"
  B_CLICK_Y="$(awk -v wy="$TAB2_WY" -v frame_y="$B_FRAME_Y" -v frame_size="$B_FRAME_SIZE" 'BEGIN { split(frame_size, parts, "x"); print int(wy + frame_y + (parts[2] / 2) + 0.5) }')"
  B_HIT_START_LINE="$(log_line_count)"
  click_global_point "$B_CLICK_X" "$B_CLICK_Y" "browser_b_area"
  B_HIT_LINE="$(wait_for_hit_after "$B_HIT_START_LINE" "$B_CONTEXT_ID" "browser B hit-test")"
  require_text "$B_HIT_LINE" "selected_tab_id:${TAB2_SELECTED_TAB_ID}" "browser B hit-test has tab 2 selected tab id"
  require_text "$B_HIT_LINE" "overlay_frame=${B_FRAME}" "browser B hit-test uses browser B frame"
  require_text "$B_HIT_LINE" "web_point={" "browser B hit-test includes webview-relative point"

  B_MODE_START_LINE="$(log_line_count)"
  B_MODE_TRACE_START_LINE="$(trace_line_count)"
  log "browser_b_mode_key=enter=Mode::Browse"
  swift "$ROOT/scripts/ghostty-app/inject.swift" key 36 >>"$HARNESS_LOG" 2>&1
  wait_for_log_after "$B_MODE_START_LINE" "ModeChanged: pane_id=${B_PANE_ID} browsing=true" "browser B webtui entered browse mode"
  require_trace_after "$B_MODE_TRACE_START_LINE" "focus-changed tab=${B_BROWSER_TAB_ID} pane=${B_PANE_ID} ffi=ts_set_focus focused=true" "Roamium observed browser B focus=true after browse mode"

  B_KEY_START_LINE="$(trace_line_count)"
  printf 'ISSUE809_EXP13_BROWSER_B\n' >"$BROWSER_FOCUS_COMMAND"
  swift "$ROOT/scripts/ghostty-app/inject.swift" type "$BROWSER_FOCUS_COMMAND" >>"$HARNESS_LOG" 2>&1
  require_trace_after "$B_KEY_START_LINE" "key-event tab=${B_BROWSER_TAB_ID} pane=${B_PANE_ID}" "browser B keyboard marker reached browser B"
  require_no_trace_after "$B_KEY_START_LINE" "key-event tab=${A_BROWSER_TAB_ID} pane=${A_PANE_ID}" "browser B keyboard marker did not reach browser A"

  B_CONTROL_START_LINE="$(log_line_count)"
  B_CONTROL_TRACE_START_LINE="$(trace_line_count)"
  log "browser_b_control_key=escape=Mode::Control"
  swift "$ROOT/scripts/ghostty-app/inject.swift" key 53 >>"$HARNESS_LOG" 2>&1
  wait_for_log_after "$B_CONTROL_START_LINE" "ModeChanged: pane_id=${B_PANE_ID} browsing=false" "browser B webtui returned to control mode"
  require_trace_after "$B_CONTROL_TRACE_START_LINE" "focus-changed tab=${B_BROWSER_TAB_ID} pane=${B_PANE_ID} ffi=ts_set_focus focused=false" "Roamium observed browser B focus=false after control mode"

  if [ "$SCENARIO" = "close-browser-tab" ]; then
    CLOSE_TAB_START_LINE="$(log_line_count)"
    CLOSE_TAB_TRACE_START_LINE="$(trace_line_count)"
    log "close_tab_keybind=ctrl+w=close_tab"
    swift "$ROOT/scripts/ghostty-app/inject.swift" key 13 control >>"$HARNESS_LOG" 2>&1
    delay 1

    CLOSE_SELECTED_LINE="$(wait_for_selected_tab_change_after "$CLOSE_TAB_START_LINE" "$TAB2_SELECTED_TAB_ID" "browser B native tab closed/selection changed")"
    CLOSE_SELECTED_TAB_ID="$(extract_selected_tab_id "$CLOSE_SELECTED_LINE")"
    [ "$CLOSE_SELECTED_TAB_ID" = "$A_SELECTED_TAB_ID" ] || fail "closing browser B tab selected unexpected tab: expected=$A_SELECTED_TAB_ID actual=$CLOSE_SELECTED_TAB_ID"
    log "PASS: closing browser B tab selected browser A tab"
    log "close_selected_tab_id=$CLOSE_SELECTED_TAB_ID"

    A_AFTER_CLOSE_PRESENT_LINE="$(wait_for_changed_appkit_frame_after "$CLOSE_TAB_START_LINE" "$A_PANE_ID" "$A_CONTEXT_ID" "$A_TABBED_FRAME" "browser A geometry restored after browser B tab close")"
    A_AFTER_CLOSE_PIXELS_LINE="$(wait_for_changed_appkit_pixels_after "$CLOSE_TAB_START_LINE" "$A_PANE_ID" "$A_CONTEXT_ID" "$A_TABBED_PIXEL" "browser A AppKit pixels restored after browser B tab close")"
    A_AFTER_CLOSE_FRAME="$(extract_overlay_frame "$A_AFTER_CLOSE_PRESENT_LINE")"
    A_AFTER_CLOSE_FRAME_SIZE="$(extract_frame_size "$A_AFTER_CLOSE_PRESENT_LINE")"
    A_AFTER_CLOSE_FRAME_X="$(extract_frame_x "$A_AFTER_CLOSE_PRESENT_LINE")"
    A_AFTER_CLOSE_FRAME_Y="$(extract_frame_y "$A_AFTER_CLOSE_PRESENT_LINE")"
    A_AFTER_CLOSE_PIXEL="$(extract_appkit_pixel "$A_AFTER_CLOSE_PIXELS_LINE")"
    A_AFTER_CLOSE_PIXEL_WIDTH="${A_AFTER_CLOSE_PIXEL%x*}"
    A_AFTER_CLOSE_PIXEL_HEIGHT="${A_AFTER_CLOSE_PIXEL#*x}"
    log "browser_a_after_close_overlay_frame=$A_AFTER_CLOSE_FRAME"
    log "browser_a_after_close_overlay_frame_size=$A_AFTER_CLOSE_FRAME_SIZE"
    log "browser_a_after_close_appkit_pixel=$A_AFTER_CLOSE_PIXEL"
    require_trace_after "$CLOSE_TAB_TRACE_START_LINE" "resize tab_id=${A_BROWSER_TAB_ID} pane_id=${A_PANE_ID} pixel_width=${A_AFTER_CLOSE_PIXEL_WIDTH} pixel_height=${A_AFTER_CLOSE_PIXEL_HEIGHT} screen_x=0 screen_y=0 screen_width=0 screen_height=0 screen_scale=0 ffi=ts_set_view_size" "Roamium resized browser A after browser B tab close"

    CLEAR_OVERLAY_SEEN=""
    for _ in $(seq 1 30); do
      if tail -n +"$((CLOSE_TAB_START_LINE + 1))" "$APP_LOG" | grep -E "TermSurf geometry layer=zig event=clear_overlay_call .*pane_id:${B_PANE_ID}" >/dev/null 2>&1; then
        CLEAR_OVERLAY_SEEN="1"
        break
      fi
      if tail -n +"$((CLOSE_TAB_TRACE_START_LINE + 1))" "$ROAMIUM_TRACE" | grep -F "key-event tab=${B_BROWSER_TAB_ID} pane=${B_PANE_ID}" >/dev/null 2>&1; then
        fail "Control-W was forwarded to browser B input before close_tab cleanup"
      fi
      delay 1
    done
    [ -n "$CLEAR_OVERLAY_SEEN" ] || fail "timed out waiting for Zig records browser B clear_overlay_call after tab close"
    log "PASS: Zig records browser B clear_overlay_call after tab close"

    wait_for_log_after "$CLOSE_TAB_START_LINE" "TermSurf geometry layer=bridge event=clear_request .*pane_id:${B_PANE_ID}" "Swift bridge records browser B clear_request after tab close"

    CLEAR_RESULT=""
    for _ in $(seq 1 30); do
      if tail -n +"$((CLOSE_TAB_START_LINE + 1))" "$APP_LOG" | grep -E "TermSurf geometry layer=bridge event=clear_target_found .*pane_id:${B_PANE_ID}" >/dev/null 2>&1 &&
        tail -n +"$((CLOSE_TAB_START_LINE + 1))" "$APP_LOG" | grep -E "TermSurf geometry layer=appkit event=clear .*pane_id:${B_PANE_ID}" >/dev/null 2>&1; then
        CLEAR_RESULT="target-found"
        break
      fi
      if tail -n +"$((CLOSE_TAB_START_LINE + 1))" "$APP_LOG" | grep -E "TermSurf geometry layer=bridge event=clear_rejected .*pane_id:${B_PANE_ID} .*note=no-surface" >/dev/null 2>&1; then
        CLEAR_RESULT="surface-already-gone"
        break
      fi
      delay 1
    done
    [ -n "$CLEAR_RESULT" ] || fail "missing AppKit clear or bridge no-surface cleanup evidence after browser B tab close"
    log "PASS: observed browser B tab-close clear result clear_result=$CLEAR_RESULT"

    require_log_after "$CLOSE_TAB_START_LINE" "CloseTab: pane_id=${B_PANE_ID} tab_id=${B_BROWSER_TAB_ID}" "Zig records CloseTab for browser B after tab close"
    require_trace_after "$CLOSE_TAB_TRACE_START_LINE" "close-tab tab_id=${B_BROWSER_TAB_ID} pane_id=${B_PANE_ID} result=destroying ffi=ts_destroy_web_contents" "Roamium received CloseTab and destroyed browser B"
    require_trace_after "$CLOSE_TAB_TRACE_START_LINE" "close-tab tab_id=${B_BROWSER_TAB_ID} result=removed" "Roamium removed closed browser B tab"

    SELECT_CLOSED_START_LINE="$(log_line_count)"
    log "select_closed_tab_keybind=ctrl+2=goto_tab:2"
    swift "$ROOT/scripts/ghostty-app/inject.swift" key 19 control >>"$HARNESS_LOG" 2>&1
    delay 1
    if tail -n +"$((SELECT_CLOSED_START_LINE + 1))" "$APP_LOG" |
      grep -E "TermSurf geometry layer=appkit event=.*selected_tab_id:${TAB2_SELECTED_TAB_ID}" >/dev/null 2>&1; then
      fail "closed browser B native tab was selectable after close"
    fi
    log "PASS: closed browser B native tab was not selectable by ctrl+2"

    require_no_trace_after "$CLOSE_TAB_TRACE_START_LINE" "key-event tab=${B_BROWSER_TAB_ID} pane=${B_PANE_ID}" "close-tab and closed-tab probes did not reach browser B input"

    TAB1_WIN_LINE="$(window_bounds_for "$A_SELECTED_TAB_ID")" || fail "failed to resolve browser A window bounds after tab close for window id=$A_SELECTED_TAB_ID"
    IFS=$'\t' read -r _TAB1_WID TAB1_WX TAB1_WY TAB1_WW TAB1_WH <<<"$TAB1_WIN_LINE"
    A_CLICK_X="$(awk -v wx="$TAB1_WX" -v frame_x="$A_AFTER_CLOSE_FRAME_X" -v frame_size="$A_AFTER_CLOSE_FRAME_SIZE" 'BEGIN { split(frame_size, parts, "x"); print int(wx + frame_x + (parts[1] / 2) + 0.5) }')"
    A_CLICK_Y="$(awk -v wy="$TAB1_WY" -v frame_y="$A_AFTER_CLOSE_FRAME_Y" -v frame_size="$A_AFTER_CLOSE_FRAME_SIZE" 'BEGIN { split(frame_size, parts, "x"); print int(wy + frame_y + (parts[2] / 2) + 0.5) }')"
    A_AFTER_CLOSE_HIT_START_LINE="$(log_line_count)"
    click_global_point "$A_CLICK_X" "$A_CLICK_Y" "browser_a_after_tab_close_area"
    A_AFTER_CLOSE_HIT_LINE="$(wait_for_hit_after "$A_AFTER_CLOSE_HIT_START_LINE" "$A_CONTEXT_ID" "browser A after tab-close hit-test")"
    require_text "$A_AFTER_CLOSE_HIT_LINE" "selected_tab_id:${A_SELECTED_TAB_ID}" "browser A after tab-close hit-test has tab 1 selected tab id"
    require_text "$A_AFTER_CLOSE_HIT_LINE" "overlay_frame=${A_AFTER_CLOSE_FRAME}" "browser A after tab-close hit-test uses browser A post-close frame"
    require_text "$A_AFTER_CLOSE_HIT_LINE" "web_point={" "browser A after tab-close hit-test includes webview-relative point"

    A_AFTER_CLOSE_MODE_START_LINE="$(log_line_count)"
    A_AFTER_CLOSE_MODE_TRACE_START_LINE="$(trace_line_count)"
    log "browser_a_after_tab_close_mode_key=enter=Mode::Browse"
    swift "$ROOT/scripts/ghostty-app/inject.swift" key 36 >>"$HARNESS_LOG" 2>&1
    wait_for_log_after "$A_AFTER_CLOSE_MODE_START_LINE" "ModeChanged: pane_id=${A_PANE_ID} browsing=true" "browser A entered browse mode after browser B tab close"
    require_trace_after "$A_AFTER_CLOSE_MODE_TRACE_START_LINE" "focus-changed tab=${A_BROWSER_TAB_ID} pane=${A_PANE_ID} ffi=ts_set_focus focused=true" "Roamium observed browser A focus=true after browser B tab close"

    A_AFTER_CLOSE_KEY_START_LINE="$(trace_line_count)"
    printf 'ISSUE809_EXP14_BROWSER_A_AFTER_CLOSE\n' >"$BROWSER_FOCUS_COMMAND"
    swift "$ROOT/scripts/ghostty-app/inject.swift" type "$BROWSER_FOCUS_COMMAND" >>"$HARNESS_LOG" 2>&1
    require_trace_after "$A_AFTER_CLOSE_KEY_START_LINE" "key-event tab=${A_BROWSER_TAB_ID} pane=${A_PANE_ID}" "browser A keyboard marker reached browser A after browser B tab close"
    require_no_trace_after "$A_AFTER_CLOSE_KEY_START_LINE" "key-event tab=${B_BROWSER_TAB_ID} pane=${B_PANE_ID}" "browser A keyboard marker did not reach closed browser B"

    screencapture -x -o -l"$A_SELECTED_TAB_ID" "$SCREENSHOT_TAB_AFTER_CLOSE"
    log "after_close_screenshot_exit=$?"

    [ "$NEW_TAB_TRACE_START_LINE" -lt "$BROWSER_B_TRACE_START_LINE" ] || fail "trace boundaries for browser B open were not monotonic"
    [ "$BROWSER_B_TRACE_START_LINE" -lt "$CLOSE_TAB_TRACE_START_LINE" ] || fail "trace boundaries for browser B close were not monotonic"
  fi

  if [ "$SCENARIO" = "open-browser-in-new-tab" ]; then
  SWITCH_A_START_LINE="$(log_line_count)"
  SWITCH_A_TRACE_START_LINE="$(trace_line_count)"
  log "switch_to_browser_a_keybind=ctrl+p=previous_tab"
  swift "$ROOT/scripts/ghostty-app/inject.swift" key 35 control >>"$HARNESS_LOG" 2>&1
  delay 1
  wait_for_log_after "$SWITCH_A_START_LINE" "Pane focus changed: pane_id=${A_PANE_ID} focused=true" "browser A pane focused again"
  require_no_different_appkit_frame_after "$SWITCH_A_START_LINE" "$A_PANE_ID" "$A_CONTEXT_ID" "$A_TABBED_FRAME" "browser A kept tab-bar-adjusted AppKit frame after tab restore"
  require_no_different_appkit_pixels_after "$SWITCH_A_START_LINE" "$A_PANE_ID" "$A_CONTEXT_ID" "$A_TABBED_PIXEL" "browser A kept tab-bar-adjusted AppKit pixels after tab restore"

  TAB1_WIN_LINE="$(window_bounds_for "$A_SELECTED_TAB_ID")" || fail "failed to resolve tab 1 window bounds for window id=$A_SELECTED_TAB_ID"
  IFS=$'\t' read -r _TAB1_WID TAB1_WX TAB1_WY TAB1_WW TAB1_WH <<<"$TAB1_WIN_LINE"
  A_CLICK_X="$(awk -v wx="$TAB1_WX" -v frame_x="$A_TABBED_FRAME_X" -v frame_size="$A_TABBED_FRAME_SIZE" 'BEGIN { split(frame_size, parts, "x"); print int(wx + frame_x + (parts[1] / 2) + 0.5) }')"
  A_CLICK_Y="$(awk -v wy="$TAB1_WY" -v frame_y="$A_TABBED_FRAME_Y" -v frame_size="$A_TABBED_FRAME_SIZE" 'BEGIN { split(frame_size, parts, "x"); print int(wy + frame_y + (parts[2] / 2) + 0.5) }')"
  A_RESTORE_HIT_START_LINE="$(log_line_count)"
  click_global_point "$A_CLICK_X" "$A_CLICK_Y" "browser_a_restored_area"
  A_RESTORE_HIT_LINE="$(wait_for_hit_after "$A_RESTORE_HIT_START_LINE" "$A_CONTEXT_ID" "browser A restored hit-test")"
  require_text "$A_RESTORE_HIT_LINE" "selected_tab_id:${A_SELECTED_TAB_ID}" "browser A restored hit-test has tab 1 selected tab id"
  require_text "$A_RESTORE_HIT_LINE" "overlay_frame=${A_TABBED_FRAME}" "browser A restored hit-test uses browser A frame"
  require_text "$A_RESTORE_HIT_LINE" "web_point={" "browser A restored hit-test includes webview-relative point"

  A_MODE_START_LINE="$(log_line_count)"
  A_MODE_TRACE_START_LINE="$(trace_line_count)"
  log "browser_a_mode_key=enter=Mode::Browse"
  swift "$ROOT/scripts/ghostty-app/inject.swift" key 36 >>"$HARNESS_LOG" 2>&1
  wait_for_log_after "$A_MODE_START_LINE" "ModeChanged: pane_id=${A_PANE_ID} browsing=true" "browser A webtui entered browse mode"
  require_trace_after "$A_MODE_TRACE_START_LINE" "focus-changed tab=${A_BROWSER_TAB_ID} pane=${A_PANE_ID} ffi=ts_set_focus focused=true" "Roamium observed browser A focus=true after browse mode"

  A_KEY_START_LINE="$(trace_line_count)"
  printf 'ISSUE809_EXP13_BROWSER_A\n' >"$BROWSER_FOCUS_COMMAND"
  swift "$ROOT/scripts/ghostty-app/inject.swift" type "$BROWSER_FOCUS_COMMAND" >>"$HARNESS_LOG" 2>&1
  require_trace_after "$A_KEY_START_LINE" "key-event tab=${A_BROWSER_TAB_ID} pane=${A_PANE_ID}" "browser A keyboard marker reached browser A"
  require_no_trace_after "$A_KEY_START_LINE" "key-event tab=${B_BROWSER_TAB_ID} pane=${B_PANE_ID}" "browser A keyboard marker did not reach browser B"
  screencapture -x -o -l"$A_SELECTED_TAB_ID" "$SCREENSHOT_TAB_BROWSER_A_RESTORED"
  log "browser_a_restored_screenshot_exit=$?"

  A_CONTROL_START_LINE="$(log_line_count)"
  A_CONTROL_TRACE_START_LINE="$(trace_line_count)"
  log "browser_a_control_key=escape=Mode::Control"
  swift "$ROOT/scripts/ghostty-app/inject.swift" key 53 >>"$HARNESS_LOG" 2>&1
  wait_for_log_after "$A_CONTROL_START_LINE" "ModeChanged: pane_id=${A_PANE_ID} browsing=false" "browser A webtui returned to control mode"
  require_trace_after "$A_CONTROL_TRACE_START_LINE" "focus-changed tab=${A_BROWSER_TAB_ID} pane=${A_PANE_ID} ffi=ts_set_focus focused=false" "Roamium observed browser A focus=false after control mode"

  SWITCH_B_START_LINE="$(log_line_count)"
  SWITCH_B_TRACE_START_LINE="$(trace_line_count)"
  log "switch_to_browser_b_keybind=ctrl+n=next_tab"
  swift "$ROOT/scripts/ghostty-app/inject.swift" key 45 control >>"$HARNESS_LOG" 2>&1
  delay 1
  wait_for_log_after "$SWITCH_B_START_LINE" "Pane focus changed: pane_id=${B_PANE_ID} focused=true" "browser B pane focused again"
  require_no_different_appkit_frame_after "$SWITCH_B_START_LINE" "$B_PANE_ID" "$B_CONTEXT_ID" "$B_FRAME" "browser B kept AppKit frame after tab restore"
  require_no_different_appkit_pixels_after "$SWITCH_B_START_LINE" "$B_PANE_ID" "$B_CONTEXT_ID" "$B_PIXEL" "browser B kept AppKit pixels after tab restore"

  TAB2_WIN_LINE="$(window_bounds_for "$TAB2_SELECTED_TAB_ID")" || fail "failed to resolve restored tab 2 window bounds for window id=$TAB2_SELECTED_TAB_ID"
  IFS=$'\t' read -r _TAB2_WID TAB2_WX TAB2_WY TAB2_WW TAB2_WH <<<"$TAB2_WIN_LINE"
  B_CLICK_X="$(awk -v wx="$TAB2_WX" -v frame_x="$B_FRAME_X" -v frame_size="$B_FRAME_SIZE" 'BEGIN { split(frame_size, parts, "x"); print int(wx + frame_x + (parts[1] / 2) + 0.5) }')"
  B_CLICK_Y="$(awk -v wy="$TAB2_WY" -v frame_y="$B_FRAME_Y" -v frame_size="$B_FRAME_SIZE" 'BEGIN { split(frame_size, parts, "x"); print int(wy + frame_y + (parts[2] / 2) + 0.5) }')"
  B_RESTORE_HIT_START_LINE="$(log_line_count)"
  click_global_point "$B_CLICK_X" "$B_CLICK_Y" "browser_b_restored_area"
  B_RESTORE_HIT_LINE="$(wait_for_hit_after "$B_RESTORE_HIT_START_LINE" "$B_CONTEXT_ID" "browser B restored hit-test")"
  require_text "$B_RESTORE_HIT_LINE" "selected_tab_id:${TAB2_SELECTED_TAB_ID}" "browser B restored hit-test has tab 2 selected tab id"
  require_text "$B_RESTORE_HIT_LINE" "overlay_frame=${B_FRAME}" "browser B restored hit-test uses browser B frame"
  require_text "$B_RESTORE_HIT_LINE" "web_point={" "browser B restored hit-test includes webview-relative point"

  B_RESTORE_MODE_START_LINE="$(log_line_count)"
  B_RESTORE_MODE_TRACE_START_LINE="$(trace_line_count)"
  log "browser_b_restored_mode_key=enter=Mode::Browse"
  swift "$ROOT/scripts/ghostty-app/inject.swift" key 36 >>"$HARNESS_LOG" 2>&1
  wait_for_log_after "$B_RESTORE_MODE_START_LINE" "ModeChanged: pane_id=${B_PANE_ID} browsing=true" "browser B restored webtui entered browse mode"
  require_trace_after "$B_RESTORE_MODE_TRACE_START_LINE" "focus-changed tab=${B_BROWSER_TAB_ID} pane=${B_PANE_ID} ffi=ts_set_focus focused=true" "Roamium observed browser B restored focus=true after browse mode"

  B_RESTORE_KEY_START_LINE="$(trace_line_count)"
  printf 'ISSUE809_EXP13_BROWSER_B_RESTORED\n' >"$BROWSER_FOCUS_COMMAND"
  swift "$ROOT/scripts/ghostty-app/inject.swift" type "$BROWSER_FOCUS_COMMAND" >>"$HARNESS_LOG" 2>&1
  require_trace_after "$B_RESTORE_KEY_START_LINE" "key-event tab=${B_BROWSER_TAB_ID} pane=${B_PANE_ID}" "browser B restored keyboard marker reached browser B"
  require_no_trace_after "$B_RESTORE_KEY_START_LINE" "key-event tab=${A_BROWSER_TAB_ID} pane=${A_PANE_ID}" "browser B restored keyboard marker did not reach browser A"
  screencapture -x -o -l"$TAB2_SELECTED_TAB_ID" "$SCREENSHOT_TAB_BROWSER_B_RESTORED"
  log "browser_b_restored_screenshot_exit=$?"

  [ "$BROWSER_B_TRACE_START_LINE" -lt "$SWITCH_B_TRACE_START_LINE" ] || fail "trace boundaries for browser B restore were not monotonic"
  [ "$NEW_TAB_TRACE_START_LINE" -lt "$BROWSER_B_TRACE_START_LINE" ] || fail "trace boundaries for browser B open were not monotonic"
  fi
fi

if [ "$SCENARIO" = "window-resize" ]; then
  INITIAL_PIXEL="$APPKIT_PIXEL"
  INITIAL_FRAME_SIZE="$OVERLAY_FRAME_SIZE"
  INITIAL_WW="$WW"
  INITIAL_WH="$WH"

  GROW_WIDTH=$((INITIAL_WW + 320))
  GROW_HEIGHT=$((INITIAL_WH + 220))
  log "resize_grow_target=${GROW_WIDTH}x${GROW_HEIGHT}"
  GROW_START_LINE="$(log_line_count)"
  set_window_size "$GROW_WIDTH" "$GROW_HEIGHT" >>"$HARNESS_LOG" 2>&1 || fail "failed to grow window via System Events"
  delay 1
  GROW_WIN_LINE="$(window_bounds)" || fail "failed to resolve grown window bounds for window id=$WID"
  log "grow_window=$GROW_WIN_LINE"
  GROW_PRESENT_LINE="$(wait_for_appkit_frame_after "$GROW_START_LINE" "$INITIAL_FRAME_SIZE" gt "grown AppKit overlay frame")"
  GROW_PIXELS_LINE="$(wait_for_appkit_pixels_after "$GROW_START_LINE" "$INITIAL_PIXEL" gt "grown AppKit presented pixels")"
  GROW_FRAME_SIZE="$(extract_frame_size "$GROW_PRESENT_LINE")"
  GROW_PIXEL="$(extract_appkit_pixel "$GROW_PIXELS_LINE")"
  GROW_PIXEL_WIDTH="${GROW_PIXEL%x*}"
  GROW_PIXEL_HEIGHT="${GROW_PIXEL#*x}"
  log "PASS: observed grown AppKit overlay frame overlay_frame_size=$GROW_FRAME_SIZE"
  log "PASS: observed grown AppKit presented pixels appkit_pixel=$GROW_PIXEL"
  log "grow_overlay_frame_size=$GROW_FRAME_SIZE"
  log "grow_appkit_pixel=$GROW_PIXEL"
  require_log_after "$GROW_START_LINE" "TermSurf geometry layer=zig event=appkit_presented_pixels .*pane_id:${PANE_ID} .*appkit_pixel=${GROW_PIXEL}" "Zig records grown AppKit presented pixel size"
  require_trace "resize tab_id=${BROWSER_TAB_ID} pane_id=${PANE_ID} pixel_width=${GROW_PIXEL_WIDTH} pixel_height=${GROW_PIXEL_HEIGHT} screen_x=0 screen_y=0 screen_width=0 screen_height=0 screen_scale=0 ffi=ts_set_view_size" "Roamium applied grow resize to AppKit pixel size via ts_set_view_size"
  screencapture -x -o -l"$WID" "$SCREENSHOT_GROW"
  log "grow_screenshot_exit=$?"
  GROW_HIT_START_LINE="$(log_line_count)"
  click_window_center "$GROW_WIN_LINE" "grow"
  GROW_HIT_LINE="$(wait_for_hit_after "$GROW_HIT_START_LINE" "$CONTEXT_ID" "grown AppKit hit-test")"
  log "PASS: observed grown AppKit hit-test"
  require_text "$GROW_HIT_LINE" "overlay_frame=" "grown hit-test includes current overlay frame"
  require_text "$GROW_HIT_LINE" "web_point={" "grown hit-test includes webview-relative point"

  SHRINK_WIDTH=$((INITIAL_WW + 80))
  SHRINK_HEIGHT=$((INITIAL_WH + 60))
  log "resize_shrink_target=${SHRINK_WIDTH}x${SHRINK_HEIGHT}"
  SHRINK_START_LINE="$(log_line_count)"
  set_window_size "$SHRINK_WIDTH" "$SHRINK_HEIGHT" >>"$HARNESS_LOG" 2>&1 || fail "failed to shrink window via System Events"
  delay 1
  SHRINK_WIN_LINE="$(window_bounds)" || fail "failed to resolve shrunken window bounds for window id=$WID"
  log "shrink_window=$SHRINK_WIN_LINE"
  SHRINK_PRESENT_LINE="$(wait_for_appkit_frame_after "$SHRINK_START_LINE" "$GROW_FRAME_SIZE" lt "shrunken AppKit overlay frame")"
  SHRINK_PIXELS_LINE="$(wait_for_appkit_pixels_after "$SHRINK_START_LINE" "$GROW_PIXEL" lt "shrunken AppKit presented pixels")"
  SHRINK_FRAME_SIZE="$(extract_frame_size "$SHRINK_PRESENT_LINE")"
  SHRINK_PIXEL="$(extract_appkit_pixel "$SHRINK_PIXELS_LINE")"
  SHRINK_PIXEL_WIDTH="${SHRINK_PIXEL%x*}"
  SHRINK_PIXEL_HEIGHT="${SHRINK_PIXEL#*x}"
  log "PASS: observed shrunken AppKit overlay frame overlay_frame_size=$SHRINK_FRAME_SIZE"
  log "PASS: observed shrunken AppKit presented pixels appkit_pixel=$SHRINK_PIXEL"
  log "shrink_overlay_frame_size=$SHRINK_FRAME_SIZE"
  log "shrink_appkit_pixel=$SHRINK_PIXEL"
  require_log_after "$SHRINK_START_LINE" "TermSurf geometry layer=zig event=appkit_presented_pixels .*pane_id:${PANE_ID} .*appkit_pixel=${SHRINK_PIXEL}" "Zig records shrunken AppKit presented pixel size"
  require_trace "resize tab_id=${BROWSER_TAB_ID} pane_id=${PANE_ID} pixel_width=${SHRINK_PIXEL_WIDTH} pixel_height=${SHRINK_PIXEL_HEIGHT} screen_x=0 screen_y=0 screen_width=0 screen_height=0 screen_scale=0 ffi=ts_set_view_size" "Roamium applied shrink resize to AppKit pixel size via ts_set_view_size"
  screencapture -x -o -l"$WID" "$SCREENSHOT_SHRINK"
  log "shrink_screenshot_exit=$?"
  SHRINK_HIT_START_LINE="$(log_line_count)"
  click_window_center "$SHRINK_WIN_LINE" "shrink"
  SHRINK_HIT_LINE="$(wait_for_hit_after "$SHRINK_HIT_START_LINE" "$CONTEXT_ID" "shrunken AppKit hit-test")"
  log "PASS: observed shrunken AppKit hit-test"
  require_text "$SHRINK_HIT_LINE" "overlay_frame=" "shrunken hit-test includes current overlay frame"
  require_text "$SHRINK_HIT_LINE" "web_point={" "shrunken hit-test includes webview-relative point"
fi

if [ "$SCENARIO" = "font-size-cell-metrics" ]; then
  A_WINDOW_ID="$WID"
  A_SURFACE_ID="$(extract_surface_id "$APPKIT_PRESENT_LINE")"
  A_SELECTED_TAB_ID="$(extract_selected_tab_id "$APPKIT_PRESENT_LINE")"
  A_PANE_ID="$PANE_ID"
  A_BROWSER_TAB_ID="$BROWSER_TAB_ID"
  A_CONTEXT_ID="$CONTEXT_ID"
  A_GRID="$(extract_grid "$APPKIT_PRESENT_LINE")"
  A_CELL="$(extract_cell_size "$APPKIT_PRESENT_LINE")"
  A_FRAME="$OVERLAY_FRAME"
  A_FRAME_SIZE="$OVERLAY_FRAME_SIZE"
  A_PIXEL="$APPKIT_PIXEL"
  A_BACKING_SCALE="$(extract_backing_scale "$APPKIT_PRESENT_LINE")"
  log "font_baseline_window_id=$A_WINDOW_ID"
  log "font_baseline_surface_id=$A_SURFACE_ID"
  log "font_baseline_selected_tab_id=$A_SELECTED_TAB_ID"
  log "font_baseline_pane_id=$A_PANE_ID"
  log "font_baseline_browser_tab_id=$A_BROWSER_TAB_ID"
  log "font_baseline_context_id=$A_CONTEXT_ID"
  log "font_baseline_grid=$A_GRID"
  log "font_baseline_cell=$A_CELL"
  log "font_baseline_frame=$A_FRAME"
  log "font_baseline_appkit_pixel=$A_PIXEL"
  log "font_baseline_backing_scale=$A_BACKING_SCALE"

  INCREASE_START_LINE="$(log_line_count)"
  INCREASE_TRACE_START_LINE="$(trace_line_count)"
  log "font_increase_keybind=ctrl+u=increase_font_size:2"
  swift "$ROOT/scripts/ghostty-app/inject.swift" key 32 control >>"$HARNESS_LOG" 2>&1
  delay 1
  INCREASE_PRESENT_LINE="$(wait_for_changed_appkit_frame_after "$INCREASE_START_LINE" "$A_PANE_ID" "$A_CONTEXT_ID" "$A_FRAME" "font-increased AppKit overlay frame" 45)"
  INCREASE_PIXELS_LINE="$(wait_for_changed_appkit_pixels_after "$INCREASE_START_LINE" "$A_PANE_ID" "$A_CONTEXT_ID" "$A_PIXEL" "font-increased AppKit pixels" 45)"
  INCREASE_WINDOW_ID="$(printf '%s\n' "$INCREASE_PRESENT_LINE" | sed -E 's/.*window_id:([^ ]+) .*/\1/')"
  INCREASE_SURFACE_ID="$(extract_surface_id "$INCREASE_PRESENT_LINE")"
  INCREASE_SELECTED_TAB_ID="$(extract_selected_tab_id "$INCREASE_PRESENT_LINE")"
  INCREASE_GRID="$(extract_grid "$INCREASE_PRESENT_LINE")"
  INCREASE_CELL="$(extract_cell_size "$INCREASE_PRESENT_LINE")"
  INCREASE_FRAME="$(extract_overlay_frame "$INCREASE_PRESENT_LINE")"
  INCREASE_PIXEL="$(extract_appkit_pixel "$INCREASE_PIXELS_LINE")"
  INCREASE_SCALE="$(extract_backing_scale "$INCREASE_PRESENT_LINE")"
  INCREASE_CURRENT_PIXEL="$(appkit_pixel_from_geometry_line "$INCREASE_PRESENT_LINE")"
  [ "$INCREASE_WINDOW_ID" = "$A_WINDOW_ID" ] || fail "font-increase window id changed unexpectedly: expected=$A_WINDOW_ID actual=$INCREASE_WINDOW_ID"
  [ "$INCREASE_SURFACE_ID" = "$A_SURFACE_ID" ] || fail "font-increase surface id changed"
  [ "$INCREASE_SELECTED_TAB_ID" = "$A_SELECTED_TAB_ID" ] || fail "font-increase selected tab id changed"
  [ "$INCREASE_GRID" != "$A_GRID" ] || fail "font-increase grid did not change: $INCREASE_GRID"
  [ "$INCREASE_CELL" != "$A_CELL" ] || fail "font-increase cell size did not change: $INCREASE_CELL"
  [ "$INCREASE_SCALE" = "$A_BACKING_SCALE" ] || fail "font-increase backing scale mismatch: expected=$A_BACKING_SCALE actual=$INCREASE_SCALE"
  [ "$INCREASE_CURRENT_PIXEL" = "$INCREASE_PIXEL" ] || fail "font-increase current frame-derived pixel mismatch: expected=$INCREASE_PIXEL actual=$INCREASE_CURRENT_PIXEL"
  log "font_increase_grid=$INCREASE_GRID"
  log "font_increase_cell=$INCREASE_CELL"
  log "font_increase_frame=$INCREASE_FRAME"
  log "font_increase_appkit_pixel=$INCREASE_PIXEL"
  log "PASS: font-increase current AppKit pixel matched presented pixels"
  INCREASE_PIXEL_WIDTH="${INCREASE_PIXEL%x*}"
  INCREASE_PIXEL_HEIGHT="${INCREASE_PIXEL#*x}"
  require_log_after "$INCREASE_START_LINE" "TermSurf geometry layer=zig event=appkit_presented_pixels .*pane_id:${A_PANE_ID} .*appkit_pixel=${INCREASE_PIXEL}" "Zig records font-increased AppKit presented pixel size"
  require_trace_after "$INCREASE_TRACE_START_LINE" "resize tab_id=${A_BROWSER_TAB_ID} pane_id=${A_PANE_ID} pixel_width=${INCREASE_PIXEL_WIDTH} pixel_height=${INCREASE_PIXEL_HEIGHT} screen_x=0 screen_y=0 screen_width=0 screen_height=0 screen_scale=0 ffi=ts_set_view_size" "Roamium applied font-increase resize to AppKit pixel size"

  screencapture -x -o -l"$A_WINDOW_ID" "$SCREENSHOT_FONT_INCREASE"
  log "font_increase_screenshot_exit=$?"
  INCREASE_WIN_LINE="$(window_bounds_for "$A_WINDOW_ID")" || fail "failed to resolve font-increased window bounds"
  INCREASE_HIT_START_LINE="$(log_line_count)"
  click_window_center "$INCREASE_WIN_LINE" "font_increase_browser_area"
  INCREASE_HIT_LINE="$(wait_for_hit_after "$INCREASE_HIT_START_LINE" "$A_CONTEXT_ID" "font-increased browser hit-test")"
  require_text "$INCREASE_HIT_LINE" "window_id:${A_WINDOW_ID}" "font-increased hit-test has window id"
  require_text "$INCREASE_HIT_LINE" "surface_id:${A_SURFACE_ID}" "font-increased hit-test has surface id"
  require_text "$INCREASE_HIT_LINE" "selected_tab_id:${A_SELECTED_TAB_ID}" "font-increased hit-test has selected tab id"
  require_text "$INCREASE_HIT_LINE" "overlay_frame=${INCREASE_FRAME}" "font-increased hit-test uses current AppKit frame"
  require_text "$INCREASE_HIT_LINE" "web_point={" "font-increased hit-test includes webview-relative point"

  INCREASE_MODE_START_LINE="$(log_line_count)"
  INCREASE_MODE_TRACE_START_LINE="$(trace_line_count)"
  log "font_increase_mode_key=enter=Mode::Browse"
  swift "$ROOT/scripts/ghostty-app/inject.swift" key 36 >>"$HARNESS_LOG" 2>&1
  wait_for_log_after "$INCREASE_MODE_START_LINE" "ModeChanged: pane_id=${A_PANE_ID} browsing=true" "font-increased webtui entered browse mode"
  require_trace_after "$INCREASE_MODE_TRACE_START_LINE" "focus-changed tab=${A_BROWSER_TAB_ID} pane=${A_PANE_ID} ffi=ts_set_focus focused=true" "Roamium observed focus=true after font increase"
  INCREASE_KEY_START_LINE="$(trace_line_count)"
  printf 'ISSUE809_EXP20_FONT_INCREASE\n' >"$BROWSER_FOCUS_COMMAND"
  swift "$ROOT/scripts/ghostty-app/inject.swift" type "$BROWSER_FOCUS_COMMAND" >>"$HARNESS_LOG" 2>&1
  require_trace_after "$INCREASE_KEY_START_LINE" "key-event tab=${A_BROWSER_TAB_ID} pane=${A_PANE_ID}" "font-increased keyboard marker reached browser"

  INCREASE_CONTROL_START_LINE="$(log_line_count)"
  INCREASE_CONTROL_TRACE_START_LINE="$(trace_line_count)"
  log "font_increase_control_key=escape=Mode::Control"
  swift "$ROOT/scripts/ghostty-app/inject.swift" key 53 >>"$HARNESS_LOG" 2>&1
  wait_for_log_after "$INCREASE_CONTROL_START_LINE" "ModeChanged: pane_id=${A_PANE_ID} browsing=false" "font-increased webtui returned to control mode"
  require_trace_after "$INCREASE_CONTROL_TRACE_START_LINE" "focus-changed tab=${A_BROWSER_TAB_ID} pane=${A_PANE_ID} ffi=ts_set_focus focused=false" "Roamium observed focus=false before font decrease"

  DECREASE_START_LINE="$(log_line_count)"
  DECREASE_TRACE_START_LINE="$(trace_line_count)"
  log "font_decrease_keybind=ctrl+y=decrease_font_size:2"
  swift "$ROOT/scripts/ghostty-app/inject.swift" key 16 control >>"$HARNESS_LOG" 2>&1
  delay 1
  DECREASE_PRESENT_LINE="$(wait_for_exact_appkit_frame_after "$DECREASE_START_LINE" "$A_PANE_ID" "$A_CONTEXT_ID" "$A_FRAME" "font-decreased AppKit overlay frame" 45)"
  DECREASE_PIXELS_LINE="$(wait_for_exact_appkit_pixels_after "$DECREASE_START_LINE" "$A_PANE_ID" "$A_CONTEXT_ID" "$A_PIXEL" "font-decreased AppKit pixels" 45)"
  DECREASE_WINDOW_ID="$(printf '%s\n' "$DECREASE_PRESENT_LINE" | sed -E 's/.*window_id:([^ ]+) .*/\1/')"
  DECREASE_SURFACE_ID="$(extract_surface_id "$DECREASE_PRESENT_LINE")"
  DECREASE_SELECTED_TAB_ID="$(extract_selected_tab_id "$DECREASE_PRESENT_LINE")"
  DECREASE_GRID="$(extract_grid "$DECREASE_PRESENT_LINE")"
  DECREASE_CELL="$(extract_cell_size "$DECREASE_PRESENT_LINE")"
  DECREASE_FRAME="$(extract_overlay_frame "$DECREASE_PRESENT_LINE")"
  DECREASE_PIXEL="$(extract_appkit_pixel "$DECREASE_PIXELS_LINE")"
  DECREASE_SCALE="$(extract_backing_scale "$DECREASE_PRESENT_LINE")"
  DECREASE_CURRENT_PIXEL="$(appkit_pixel_from_geometry_line "$DECREASE_PRESENT_LINE")"
  [ "$DECREASE_WINDOW_ID" = "$A_WINDOW_ID" ] || fail "font-decrease window id changed unexpectedly: expected=$A_WINDOW_ID actual=$DECREASE_WINDOW_ID"
  [ "$DECREASE_SURFACE_ID" = "$A_SURFACE_ID" ] || fail "font-decrease surface id changed"
  [ "$DECREASE_SELECTED_TAB_ID" = "$A_SELECTED_TAB_ID" ] || fail "font-decrease selected tab id changed"
  [ "$DECREASE_GRID" = "$A_GRID" ] || fail "font-decrease grid did not return to baseline: expected=$A_GRID actual=$DECREASE_GRID"
  [ "$DECREASE_CELL" = "$A_CELL" ] || fail "font-decrease cell did not return to baseline: expected=$A_CELL actual=$DECREASE_CELL"
  [ "$DECREASE_SCALE" = "$A_BACKING_SCALE" ] || fail "font-decrease backing scale mismatch: expected=$A_BACKING_SCALE actual=$DECREASE_SCALE"
  [ "$DECREASE_CURRENT_PIXEL" = "$DECREASE_PIXEL" ] || fail "font-decrease current frame-derived pixel mismatch: expected=$DECREASE_PIXEL actual=$DECREASE_CURRENT_PIXEL"
  log "font_decrease_grid=$DECREASE_GRID"
  log "font_decrease_cell=$DECREASE_CELL"
  log "font_decrease_frame=$DECREASE_FRAME"
  log "font_decrease_appkit_pixel=$DECREASE_PIXEL"
  log "PASS: font-decrease current AppKit pixel matched presented pixels"
  DECREASE_PIXEL_WIDTH="${DECREASE_PIXEL%x*}"
  DECREASE_PIXEL_HEIGHT="${DECREASE_PIXEL#*x}"
  require_log_after "$DECREASE_START_LINE" "TermSurf geometry layer=zig event=appkit_presented_pixels .*pane_id:${A_PANE_ID} .*appkit_pixel=${DECREASE_PIXEL}" "Zig records font-decreased AppKit presented pixel size"
  require_trace_after "$DECREASE_TRACE_START_LINE" "resize tab_id=${A_BROWSER_TAB_ID} pane_id=${A_PANE_ID} pixel_width=${DECREASE_PIXEL_WIDTH} pixel_height=${DECREASE_PIXEL_HEIGHT} screen_x=0 screen_y=0 screen_width=0 screen_height=0 screen_scale=0 ffi=ts_set_view_size" "Roamium applied font-decrease resize to AppKit pixel size"

  screencapture -x -o -l"$A_WINDOW_ID" "$SCREENSHOT_FONT_DECREASE"
  log "font_decrease_screenshot_exit=$?"
  DECREASE_WIN_LINE="$(window_bounds_for "$A_WINDOW_ID")" || fail "failed to resolve font-decreased window bounds"
  DECREASE_HIT_START_LINE="$(log_line_count)"
  click_window_center "$DECREASE_WIN_LINE" "font_decrease_browser_area"
  DECREASE_HIT_LINE="$(wait_for_hit_after "$DECREASE_HIT_START_LINE" "$A_CONTEXT_ID" "font-decreased browser hit-test")"
  require_text "$DECREASE_HIT_LINE" "window_id:${A_WINDOW_ID}" "font-decreased hit-test has window id"
  require_text "$DECREASE_HIT_LINE" "surface_id:${A_SURFACE_ID}" "font-decreased hit-test has surface id"
  require_text "$DECREASE_HIT_LINE" "selected_tab_id:${A_SELECTED_TAB_ID}" "font-decreased hit-test has selected tab id"
  require_text "$DECREASE_HIT_LINE" "overlay_frame=${DECREASE_FRAME}" "font-decreased hit-test uses current AppKit frame"
  require_text "$DECREASE_HIT_LINE" "web_point={" "font-decreased hit-test includes webview-relative point"

  DECREASE_MODE_START_LINE="$(log_line_count)"
  DECREASE_MODE_TRACE_START_LINE="$(trace_line_count)"
  log "font_decrease_mode_key=enter=Mode::Browse"
  swift "$ROOT/scripts/ghostty-app/inject.swift" key 36 >>"$HARNESS_LOG" 2>&1
  wait_for_log_after "$DECREASE_MODE_START_LINE" "ModeChanged: pane_id=${A_PANE_ID} browsing=true" "font-decreased webtui entered browse mode"
  require_trace_after "$DECREASE_MODE_TRACE_START_LINE" "focus-changed tab=${A_BROWSER_TAB_ID} pane=${A_PANE_ID} ffi=ts_set_focus focused=true" "Roamium observed focus=true after font decrease"
  DECREASE_KEY_START_LINE="$(trace_line_count)"
  printf 'ISSUE809_EXP20_FONT_DECREASE\n' >"$BROWSER_FOCUS_COMMAND"
  swift "$ROOT/scripts/ghostty-app/inject.swift" type "$BROWSER_FOCUS_COMMAND" >>"$HARNESS_LOG" 2>&1
  require_trace_after "$DECREASE_KEY_START_LINE" "key-event tab=${A_BROWSER_TAB_ID} pane=${A_PANE_ID}" "font-decreased keyboard marker reached browser"
fi

if [ "$SCENARIO" = "tui-overlay-resize-command" ]; then
  A_WINDOW_ID="$WID"
  A_SURFACE_ID="$(extract_surface_id "$APPKIT_PRESENT_LINE")"
  A_SELECTED_TAB_ID="$(extract_selected_tab_id "$APPKIT_PRESENT_LINE")"
  A_PANE_ID="$PANE_ID"
  A_BROWSER_TAB_ID="$BROWSER_TAB_ID"
  A_CONTEXT_ID="$CONTEXT_ID"
  A_GRID="$(extract_grid "$APPKIT_PRESENT_LINE")"
  A_GRID_WIDTH="$(extract_grid_width "$A_GRID")"
  A_GRID_HEIGHT="$(extract_grid_height "$A_GRID")"
  A_CELL="$(extract_cell_size "$APPKIT_PRESENT_LINE")"
  A_FRAME="$OVERLAY_FRAME"
  A_FRAME_SIZE="$OVERLAY_FRAME_SIZE"
  A_FRAME_X="$OVERLAY_FRAME_X"
  A_FRAME_Y="$OVERLAY_FRAME_Y"
  A_FRAME_WIDTH="$(pair_width "$A_FRAME_SIZE")"
  A_FRAME_HEIGHT="$(pair_height "$A_FRAME_SIZE")"
  A_ROOT_FRAME_SIZE="$(extract_root_frame_size "$APPKIT_PRESENT_LINE")"
  A_PIXEL="$APPKIT_PIXEL"
  A_PIXEL_HEIGHT="${A_PIXEL#*x}"
  A_BACKING_SCALE="$(extract_backing_scale "$APPKIT_PRESENT_LINE")"
  log "tui_baseline_window_id=$A_WINDOW_ID"
  log "tui_baseline_surface_id=$A_SURFACE_ID"
  log "tui_baseline_selected_tab_id=$A_SELECTED_TAB_ID"
  log "tui_baseline_pane_id=$A_PANE_ID"
  log "tui_baseline_browser_tab_id=$A_BROWSER_TAB_ID"
  log "tui_baseline_context_id=$A_CONTEXT_ID"
  log "tui_baseline_grid=$A_GRID"
  log "tui_baseline_cell=$A_CELL"
  log "tui_baseline_frame=$A_FRAME"
  log "tui_baseline_appkit_pixel=$A_PIXEL"
  log "tui_baseline_backing_scale=$A_BACKING_SCALE"

  SHRINK_ROWS=12
  [ "$A_GRID_HEIGHT" -gt "$SHRINK_ROWS" ] || fail "baseline grid height is too small for TUI shrink test: baseline=$A_GRID"
  SHRINK_START_LINE="$(log_line_count)"
  SHRINK_TRACE_START_LINE="$(trace_line_count)"
  printf ':viewport height %s' "$SHRINK_ROWS" >"$TUI_VIEWPORT_SHRINK_COMMAND"
  log "tui_viewport_shrink_command_text=$(cat "$TUI_VIEWPORT_SHRINK_COMMAND")"
  swift "$ROOT/scripts/ghostty-app/inject.swift" type "$TUI_VIEWPORT_SHRINK_COMMAND" >>"$HARNESS_LOG" 2>&1
  swift "$ROOT/scripts/ghostty-app/inject.swift" key 36 >>"$HARNESS_LOG" 2>&1
  delay 1

  SHRINK_SET_OVERLAY_LINE="$(wait_for_line_after "$SHRINK_START_LINE" "TermSurf geometry layer=zig event=set_overlay_update .*pane_id:${A_PANE_ID} .*browser_tab_id:${A_BROWSER_TAB_ID} .*grid=${A_GRID_WIDTH}x${SHRINK_ROWS}\\+1\\+1 .*context_id=${A_CONTEXT_ID}" "TUI shrink SetOverlay update" 45)"
  SHRINK_PRESENT_LINE="$(wait_for_changed_appkit_frame_after "$SHRINK_START_LINE" "$A_PANE_ID" "$A_CONTEXT_ID" "$A_FRAME" "TUI-shrunken AppKit overlay frame" 45)"
  SHRINK_PIXELS_LINE="$(wait_for_changed_appkit_pixels_after "$SHRINK_START_LINE" "$A_PANE_ID" "$A_CONTEXT_ID" "$A_PIXEL" "TUI-shrunken AppKit pixels" 45)"
  SHRINK_WINDOW_ID="$(printf '%s\n' "$SHRINK_PRESENT_LINE" | sed -E 's/.*window_id:([^ ]+) .*/\1/')"
  SHRINK_SURFACE_ID="$(extract_surface_id "$SHRINK_PRESENT_LINE")"
  SHRINK_SELECTED_TAB_ID="$(extract_selected_tab_id "$SHRINK_PRESENT_LINE")"
  SHRINK_GRID="$(extract_grid "$SHRINK_PRESENT_LINE")"
  SHRINK_GRID_WIDTH="$(extract_grid_width "$SHRINK_GRID")"
  SHRINK_GRID_HEIGHT="$(extract_grid_height "$SHRINK_GRID")"
  SHRINK_FRAME="$(extract_overlay_frame "$SHRINK_PRESENT_LINE")"
  SHRINK_FRAME_SIZE="$(extract_frame_size "$SHRINK_PRESENT_LINE")"
  SHRINK_FRAME_X="$(extract_frame_x "$SHRINK_PRESENT_LINE")"
  SHRINK_FRAME_Y="$(extract_frame_y "$SHRINK_PRESENT_LINE")"
  SHRINK_FRAME_WIDTH="$(pair_width "$SHRINK_FRAME_SIZE")"
  SHRINK_FRAME_HEIGHT="$(pair_height "$SHRINK_FRAME_SIZE")"
  SHRINK_PIXEL="$(extract_appkit_pixel "$SHRINK_PIXELS_LINE")"
  SHRINK_PIXEL_WIDTH="${SHRINK_PIXEL%x*}"
  SHRINK_PIXEL_HEIGHT="${SHRINK_PIXEL#*x}"
  SHRINK_SCALE="$(extract_backing_scale "$SHRINK_PRESENT_LINE")"
  SHRINK_CURRENT_PIXEL="$(appkit_pixel_from_geometry_line "$SHRINK_PRESENT_LINE")"
  [ "$SHRINK_WINDOW_ID" = "$A_WINDOW_ID" ] || fail "TUI shrink window id changed: expected=$A_WINDOW_ID actual=$SHRINK_WINDOW_ID"
  [ "$SHRINK_SURFACE_ID" = "$A_SURFACE_ID" ] || fail "TUI shrink surface id changed"
  [ "$SHRINK_SELECTED_TAB_ID" = "$A_SELECTED_TAB_ID" ] || fail "TUI shrink selected tab id changed"
  [ "$SHRINK_GRID_WIDTH" = "$A_GRID_WIDTH" ] || fail "TUI shrink grid width changed unexpectedly: expected=$A_GRID_WIDTH actual=$SHRINK_GRID_WIDTH"
  [ "$SHRINK_GRID_HEIGHT" = "$SHRINK_ROWS" ] || fail "TUI shrink grid height mismatch: expected=$SHRINK_ROWS actual=$SHRINK_GRID_HEIGHT"
  [ "$SHRINK_SCALE" = "$A_BACKING_SCALE" ] || fail "TUI shrink backing scale mismatch: expected=$A_BACKING_SCALE actual=$SHRINK_SCALE"
  [ "$SHRINK_CURRENT_PIXEL" = "$SHRINK_PIXEL" ] || fail "TUI shrink current frame-derived pixel mismatch: expected=$SHRINK_PIXEL actual=$SHRINK_CURRENT_PIXEL"
  [ "$SHRINK_FRAME_HEIGHT" -lt "$A_FRAME_HEIGHT" ] || fail "TUI shrink frame height did not shrink: baseline=$A_FRAME_SIZE shrink=$SHRINK_FRAME_SIZE"
  [ "$SHRINK_PIXEL_HEIGHT" -lt "$A_PIXEL_HEIGHT" ] || fail "TUI shrink AppKit pixel height did not shrink: baseline=$A_PIXEL shrink=$SHRINK_PIXEL"
  log "tui_shrink_set_overlay=$SHRINK_SET_OVERLAY_LINE"
  log "tui_shrink_grid=$SHRINK_GRID"
  log "tui_shrink_frame=$SHRINK_FRAME"
  log "tui_shrink_appkit_pixel=$SHRINK_PIXEL"
  log "PASS: TUI command shrank SetOverlay, AppKit frame, and AppKit pixels"
  require_log_after "$SHRINK_START_LINE" "TermSurf geometry layer=zig event=appkit_presented_pixels .*pane_id:${A_PANE_ID} .*appkit_pixel=${SHRINK_PIXEL}" "Zig records TUI-shrunken AppKit presented pixel size"
  require_trace_after "$SHRINK_TRACE_START_LINE" "resize tab_id=${A_BROWSER_TAB_ID} pane_id=${A_PANE_ID} pixel_width=${SHRINK_PIXEL_WIDTH} pixel_height=${SHRINK_PIXEL_HEIGHT} screen_x=0 screen_y=0 screen_width=0 screen_height=0 screen_scale=0 ffi=ts_set_view_size" "Roamium applied TUI shrink resize to AppKit pixel size"
  SHRINK_WIN_LINE="$(window_bounds_for "$A_WINDOW_ID")" || fail "failed to resolve TUI-shrunken window bounds"
  [ "$SHRINK_WIN_LINE" = "$WIN_LINE" ] || fail "TUI shrink changed window bounds: baseline=$WIN_LINE shrink=$SHRINK_WIN_LINE"
  screencapture -x -o -l"$A_WINDOW_ID" "$SCREENSHOT_TUI_SHRINK"
  log "tui_shrink_screenshot_exit=$?"

  IFS=$'\t' read -r _SHRINK_WID SHRINK_WX SHRINK_WY SHRINK_WW SHRINK_WH <<<"$SHRINK_WIN_LINE"
  SHRINK_ROOT_HEIGHT="$(pair_height "$A_ROOT_FRAME_SIZE")"
  SHRINK_CONTENT_Y_OFFSET="$(awk -v wh="$SHRINK_WH" -v root_h="$SHRINK_ROOT_HEIGHT" 'BEGIN { print int(wh - root_h) }')"
  SHRINK_INSIDE_X="$(awk -v wx="$SHRINK_WX" -v frame_x="$SHRINK_FRAME_X" -v frame_w="$SHRINK_FRAME_WIDTH" 'BEGIN { print int(wx + frame_x + (frame_w / 2) + 0.5) }')"
  SHRINK_INSIDE_Y="$(awk -v wy="$SHRINK_WY" -v content_y="$SHRINK_CONTENT_Y_OFFSET" -v frame_y="$SHRINK_FRAME_Y" -v frame_h="$SHRINK_FRAME_HEIGHT" 'BEGIN { print int(wy + content_y + frame_y + (frame_h / 2) + 0.5) }')"
  SHRINK_HIT_START_LINE="$(log_line_count)"
  click_global_point "$SHRINK_INSIDE_X" "$SHRINK_INSIDE_Y" "tui_shrink_inside"
  SHRINK_HIT_LINE="$(wait_for_hit_after "$SHRINK_HIT_START_LINE" "$A_CONTEXT_ID" "TUI-shrunken browser hit-test")"
  require_text "$SHRINK_HIT_LINE" "window_id:${A_WINDOW_ID}" "TUI-shrunken hit-test has window id"
  require_text "$SHRINK_HIT_LINE" "surface_id:${A_SURFACE_ID}" "TUI-shrunken hit-test has surface id"
  require_text "$SHRINK_HIT_LINE" "selected_tab_id:${A_SELECTED_TAB_ID}" "TUI-shrunken hit-test has selected tab id"
  require_text "$SHRINK_HIT_LINE" "overlay_frame=${SHRINK_FRAME}" "TUI-shrunken hit-test uses current AppKit frame"
  require_text "$SHRINK_HIT_LINE" "web_point={" "TUI-shrunken hit-test includes webview-relative point"

  FORMER_LOWER_X="$SHRINK_INSIDE_X"
  FORMER_LOWER_Y="$(awk -v wy="$SHRINK_WY" -v content_y="$SHRINK_CONTENT_Y_OFFSET" -v frame_y="$A_FRAME_Y" -v shrink_h="$SHRINK_FRAME_HEIGHT" -v baseline_h="$A_FRAME_HEIGHT" 'BEGIN { print int(wy + content_y + frame_y + shrink_h + ((baseline_h - shrink_h) / 2) + 0.5) }')"
  click_negative_global_point "$FORMER_LOWER_X" "$FORMER_LOWER_Y" "tui_shrink_former_lower_browser_area"
  wait_for_negative_hit_after "$NEGATIVE_HIT_START_LINE" "$A_CONTEXT_ID" "TUI-shrunken former lower browser area hit-test" allow-absent

  SHRINK_MODE_START_LINE="$(log_line_count)"
  SHRINK_MODE_TRACE_START_LINE="$(trace_line_count)"
  log "tui_shrink_mode_key=enter=Mode::Browse"
  swift "$ROOT/scripts/ghostty-app/inject.swift" key 36 >>"$HARNESS_LOG" 2>&1
  wait_for_log_after "$SHRINK_MODE_START_LINE" "ModeChanged: pane_id=${A_PANE_ID} browsing=true" "TUI-shrunken webtui entered browse mode"
  require_trace_after "$SHRINK_MODE_TRACE_START_LINE" "focus-changed tab=${A_BROWSER_TAB_ID} pane=${A_PANE_ID} ffi=ts_set_focus focused=true" "Roamium observed focus=true after TUI shrink"
  SHRINK_KEY_START_LINE="$(trace_line_count)"
  printf 'ISSUE809_EXP21_TUI_SHRINK\n' >"$BROWSER_FOCUS_COMMAND"
  swift "$ROOT/scripts/ghostty-app/inject.swift" type "$BROWSER_FOCUS_COMMAND" >>"$HARNESS_LOG" 2>&1
  require_trace_after "$SHRINK_KEY_START_LINE" "key-event tab=${A_BROWSER_TAB_ID} pane=${A_PANE_ID}" "TUI-shrunken keyboard marker reached browser"

  SHRINK_CONTROL_START_LINE="$(log_line_count)"
  SHRINK_CONTROL_TRACE_START_LINE="$(trace_line_count)"
  log "tui_shrink_control_key=escape=Mode::Control"
  swift "$ROOT/scripts/ghostty-app/inject.swift" key 53 >>"$HARNESS_LOG" 2>&1
  wait_for_log_after "$SHRINK_CONTROL_START_LINE" "ModeChanged: pane_id=${A_PANE_ID} browsing=false" "TUI-shrunken webtui returned to control mode"
  require_trace_after "$SHRINK_CONTROL_TRACE_START_LINE" "focus-changed tab=${A_BROWSER_TAB_ID} pane=${A_PANE_ID} ffi=ts_set_focus focused=false" "Roamium observed focus=false before TUI reset"

  RESET_START_LINE="$(log_line_count)"
  RESET_TRACE_START_LINE="$(trace_line_count)"
  printf ':viewport reset' >"$TUI_VIEWPORT_RESET_COMMAND"
  log "tui_viewport_reset_command_text=$(cat "$TUI_VIEWPORT_RESET_COMMAND")"
  swift "$ROOT/scripts/ghostty-app/inject.swift" type "$TUI_VIEWPORT_RESET_COMMAND" >>"$HARNESS_LOG" 2>&1
  swift "$ROOT/scripts/ghostty-app/inject.swift" key 36 >>"$HARNESS_LOG" 2>&1
  delay 1

  RESET_SET_OVERLAY_LINE="$(wait_for_line_after "$RESET_START_LINE" "TermSurf geometry layer=zig event=set_overlay_update .*pane_id:${A_PANE_ID} .*browser_tab_id:${A_BROWSER_TAB_ID} .*grid=${A_GRID_WIDTH}x${A_GRID_HEIGHT}\\+1\\+1 .*context_id=${A_CONTEXT_ID}" "TUI reset SetOverlay update" 45)"
  RESET_PRESENT_LINE="$(wait_for_exact_appkit_frame_after "$RESET_START_LINE" "$A_PANE_ID" "$A_CONTEXT_ID" "$A_FRAME" "TUI-reset AppKit overlay frame" 45)"
  RESET_PIXELS_LINE="$(wait_for_exact_appkit_pixels_after "$RESET_START_LINE" "$A_PANE_ID" "$A_CONTEXT_ID" "$A_PIXEL" "TUI-reset AppKit pixels" 45)"
  RESET_WINDOW_ID="$(printf '%s\n' "$RESET_PRESENT_LINE" | sed -E 's/.*window_id:([^ ]+) .*/\1/')"
  RESET_SURFACE_ID="$(extract_surface_id "$RESET_PRESENT_LINE")"
  RESET_SELECTED_TAB_ID="$(extract_selected_tab_id "$RESET_PRESENT_LINE")"
  RESET_GRID="$(extract_grid "$RESET_PRESENT_LINE")"
  RESET_FRAME="$(extract_overlay_frame "$RESET_PRESENT_LINE")"
  RESET_PIXEL="$(extract_appkit_pixel "$RESET_PIXELS_LINE")"
  RESET_PIXEL_WIDTH="${RESET_PIXEL%x*}"
  RESET_PIXEL_HEIGHT="${RESET_PIXEL#*x}"
  RESET_SCALE="$(extract_backing_scale "$RESET_PRESENT_LINE")"
  RESET_CURRENT_PIXEL="$(appkit_pixel_from_geometry_line "$RESET_PRESENT_LINE")"
  [ "$RESET_WINDOW_ID" = "$A_WINDOW_ID" ] || fail "TUI reset window id changed: expected=$A_WINDOW_ID actual=$RESET_WINDOW_ID"
  [ "$RESET_SURFACE_ID" = "$A_SURFACE_ID" ] || fail "TUI reset surface id changed"
  [ "$RESET_SELECTED_TAB_ID" = "$A_SELECTED_TAB_ID" ] || fail "TUI reset selected tab id changed"
  [ "$RESET_GRID" = "$A_GRID" ] || fail "TUI reset grid mismatch: expected=$A_GRID actual=$RESET_GRID"
  [ "$RESET_PIXEL" = "$A_PIXEL" ] || fail "TUI reset AppKit pixels did not return to baseline: expected=$A_PIXEL actual=$RESET_PIXEL"
  [ "$RESET_SCALE" = "$A_BACKING_SCALE" ] || fail "TUI reset backing scale mismatch: expected=$A_BACKING_SCALE actual=$RESET_SCALE"
  [ "$RESET_CURRENT_PIXEL" = "$RESET_PIXEL" ] || fail "TUI reset current frame-derived pixel mismatch: expected=$RESET_PIXEL actual=$RESET_CURRENT_PIXEL"
  log "tui_reset_set_overlay=$RESET_SET_OVERLAY_LINE"
  log "tui_reset_grid=$RESET_GRID"
  log "tui_reset_frame=$RESET_FRAME"
  log "tui_reset_appkit_pixel=$RESET_PIXEL"
  log "PASS: TUI reset returned SetOverlay, AppKit frame, and AppKit pixels to baseline"
  require_trace_after "$RESET_TRACE_START_LINE" "resize tab_id=${A_BROWSER_TAB_ID} pane_id=${A_PANE_ID} pixel_width=${RESET_PIXEL_WIDTH} pixel_height=${RESET_PIXEL_HEIGHT} screen_x=0 screen_y=0 screen_width=0 screen_height=0 screen_scale=0 ffi=ts_set_view_size" "Roamium applied TUI reset resize to AppKit pixel size"
  RESET_WIN_LINE="$(window_bounds_for "$A_WINDOW_ID")" || fail "failed to resolve TUI-reset window bounds"
  [ "$RESET_WIN_LINE" = "$WIN_LINE" ] || fail "TUI reset changed window bounds: baseline=$WIN_LINE reset=$RESET_WIN_LINE"
  screencapture -x -o -l"$A_WINDOW_ID" "$SCREENSHOT_TUI_RESET"
  log "tui_reset_screenshot_exit=$?"

  RESET_HIT_START_LINE="$(log_line_count)"
  click_window_center "$RESET_WIN_LINE" "tui_reset_browser_area"
  RESET_HIT_LINE="$(wait_for_hit_after "$RESET_HIT_START_LINE" "$A_CONTEXT_ID" "TUI-reset browser hit-test")"
  require_text "$RESET_HIT_LINE" "window_id:${A_WINDOW_ID}" "TUI-reset hit-test has window id"
  require_text "$RESET_HIT_LINE" "surface_id:${A_SURFACE_ID}" "TUI-reset hit-test has surface id"
  require_text "$RESET_HIT_LINE" "selected_tab_id:${A_SELECTED_TAB_ID}" "TUI-reset hit-test has selected tab id"
  require_text "$RESET_HIT_LINE" "overlay_frame=${A_FRAME}" "TUI-reset hit-test uses baseline AppKit frame"
  require_text "$RESET_HIT_LINE" "web_point={" "TUI-reset hit-test includes webview-relative point"

  RESET_MODE_START_LINE="$(log_line_count)"
  RESET_MODE_TRACE_START_LINE="$(trace_line_count)"
  log "tui_reset_mode_key=enter=Mode::Browse"
  swift "$ROOT/scripts/ghostty-app/inject.swift" key 36 >>"$HARNESS_LOG" 2>&1
  wait_for_log_after "$RESET_MODE_START_LINE" "ModeChanged: pane_id=${A_PANE_ID} browsing=true" "TUI-reset webtui entered browse mode"
  require_trace_after "$RESET_MODE_TRACE_START_LINE" "focus-changed tab=${A_BROWSER_TAB_ID} pane=${A_PANE_ID} ffi=ts_set_focus focused=true" "Roamium observed focus=true after TUI reset"
  RESET_KEY_START_LINE="$(trace_line_count)"
  printf 'ISSUE809_EXP21_TUI_RESET\n' >"$BROWSER_FOCUS_COMMAND"
  swift "$ROOT/scripts/ghostty-app/inject.swift" type "$BROWSER_FOCUS_COMMAND" >>"$HARNESS_LOG" 2>&1
  require_trace_after "$RESET_KEY_START_LINE" "key-event tab=${A_BROWSER_TAB_ID} pane=${A_PANE_ID}" "TUI-reset keyboard marker reached browser"

  [ "$SHRINK_TRACE_START_LINE" -lt "$SHRINK_MODE_TRACE_START_LINE" ] || fail "trace boundaries for TUI shrink were not monotonic"
  [ "$SHRINK_MODE_TRACE_START_LINE" -lt "$RESET_TRACE_START_LINE" ] || fail "trace boundaries for TUI reset were not monotonic"
fi

if [ "$SCENARIO" = "split-right" ]; then
  SPLIT_START_LINE="$(log_line_count)"
  SPLIT_TRACE_START_LINE="$(trace_line_count)"
  log "split_keybind=ctrl+d=new_split:right"
  swift "$ROOT/scripts/ghostty-app/inject.swift" key 2 control >>"$HARNESS_LOG" 2>&1
  delay 1

  SPLIT_PRESENT_LINE="$(wait_for_split_right_frame_after "$SPLIT_START_LINE" "$PANE_ID" "$CONTEXT_ID" "$OVERLAY_FRAME_SIZE" "split-right AppKit overlay frame")"
  SPLIT_PIXELS_LINE="$(wait_for_split_right_pixels_after "$SPLIT_START_LINE" "$PANE_ID" "$CONTEXT_ID" "$APPKIT_PIXEL" "split-right AppKit presented pixels")"
  SPLIT_FRAME_SIZE="$(extract_frame_size "$SPLIT_PRESENT_LINE")"
  SPLIT_FRAME_X="$(extract_frame_x "$SPLIT_PRESENT_LINE")"
  SPLIT_FRAME_WIDTH="$(pair_width "$SPLIT_FRAME_SIZE")"
  SPLIT_PIXEL="$(extract_appkit_pixel "$SPLIT_PIXELS_LINE")"
  SPLIT_PIXEL_WIDTH="${SPLIT_PIXEL%x*}"
  SPLIT_PIXEL_HEIGHT="${SPLIT_PIXEL#*x}"
  log "PASS: observed split-right AppKit overlay frame overlay_frame_size=$SPLIT_FRAME_SIZE"
  log "PASS: observed split-right AppKit presented pixels appkit_pixel=$SPLIT_PIXEL"
  log "split_overlay_frame_size=$SPLIT_FRAME_SIZE"
  log "split_overlay_frame_x=$SPLIT_FRAME_X"
  log "split_appkit_pixel=$SPLIT_PIXEL"
  require_log_after "$SPLIT_START_LINE" "TermSurf geometry layer=zig event=appkit_presented_pixels .*pane_id:${PANE_ID} .*appkit_pixel=${SPLIT_PIXEL}" "Zig records split-right AppKit presented pixel size"
  require_trace_after "$SPLIT_TRACE_START_LINE" "resize tab_id=${BROWSER_TAB_ID} pane_id=${PANE_ID} pixel_width=${SPLIT_PIXEL_WIDTH} pixel_height=${SPLIT_PIXEL_HEIGHT} screen_x=0 screen_y=0 screen_width=0 screen_height=0 screen_scale=0 ffi=ts_set_view_size" "Roamium applied split-right resize to AppKit pixel size via ts_set_view_size"
  screencapture -x -o -l"$WID" "$SCREENSHOT_SPLIT"
  log "split_screenshot_exit=$?"

  SPLIT_WIN_LINE="$(window_bounds)" || fail "failed to resolve split window bounds for window id=$WID"
  IFS=$'\t' read -r _SPLIT_WID SPLIT_WX SPLIT_WY SPLIT_WW SPLIT_WH <<<"$SPLIT_WIN_LINE"
  SPLIT_FRAME_WIDTH="$(pair_width "$SPLIT_FRAME_SIZE")"
  INITIAL_FRAME_WIDTH="$(pair_width "$OVERLAY_FRAME_SIZE")"
  SPLIT_INSIDE_X="$(awk -v wx="$SPLIT_WX" -v frame_x="$SPLIT_FRAME_X" -v frame_w="$SPLIT_FRAME_WIDTH" 'BEGIN { print int(wx + frame_x + (frame_w / 2) + 0.5) }')"
  SPLIT_INSIDE_Y=$((SPLIT_WY + SPLIT_WH / 2))
  SPLIT_HIT_START_LINE="$(log_line_count)"
  click_global_point "$SPLIT_INSIDE_X" "$SPLIT_INSIDE_Y" "split_inside"
  SPLIT_HIT_LINE="$(wait_for_hit_after "$SPLIT_HIT_START_LINE" "$CONTEXT_ID" "split-right AppKit hit-test")"
  log "PASS: observed split-right AppKit hit-test"
  require_text "$SPLIT_HIT_LINE" "overlay_frame=" "split-right hit-test includes current overlay frame"
  require_text "$SPLIT_HIT_LINE" "web_point={" "split-right hit-test includes webview-relative point"

  SPLIT_NEGATIVE_X="$(awk -v wx="$SPLIT_WX" -v frame_x="$SPLIT_FRAME_X" -v frame_w="$SPLIT_FRAME_WIDTH" -v old_w="$INITIAL_FRAME_WIDTH" 'BEGIN { print int(wx + frame_x + frame_w + ((old_w - frame_w) / 2) + 0.5) }')"
  SPLIT_NEGATIVE_Y="$SPLIT_INSIDE_Y"
  click_negative_global_point "$SPLIT_NEGATIVE_X" "$SPLIT_NEGATIVE_Y" "split_sibling_negative"
  wait_for_negative_hit_after "$NEGATIVE_HIT_START_LINE" "$CONTEXT_ID" "split-right sibling-pane negative hit-test" allow-absent
fi

if [ "$SCENARIO" = "split-right-focus-switch" ]; then
  SPLIT_START_LINE="$(log_line_count)"
  SPLIT_TRACE_START_LINE="$(trace_line_count)"
  log "split_keybind=ctrl+d=new_split:right"
  swift "$ROOT/scripts/ghostty-app/inject.swift" key 2 control >>"$HARNESS_LOG" 2>&1
  delay 1

  SPLIT_PRESENT_LINE="$(wait_for_split_right_frame_after "$SPLIT_START_LINE" "$PANE_ID" "$CONTEXT_ID" "$OVERLAY_FRAME_SIZE" "split-right AppKit overlay frame")"
  SPLIT_PIXELS_LINE="$(wait_for_split_right_pixels_after "$SPLIT_START_LINE" "$PANE_ID" "$CONTEXT_ID" "$APPKIT_PIXEL" "split-right AppKit presented pixels")"
  SPLIT_FRAME="$(printf '%s\n' "$SPLIT_PRESENT_LINE" | sed -E 's/.*overlay_frame=(\{\{[^}]+\}, \{[^}]+\}\}).*/\1/')"
  SPLIT_FRAME_SIZE="$(extract_frame_size "$SPLIT_PRESENT_LINE")"
  SPLIT_FRAME_X="$(extract_frame_x "$SPLIT_PRESENT_LINE")"
  SPLIT_FRAME_Y="$(extract_frame_y "$SPLIT_PRESENT_LINE")"
  SPLIT_FRAME_WIDTH="$(pair_width "$SPLIT_FRAME_SIZE")"
  SPLIT_PIXEL="$(extract_appkit_pixel "$SPLIT_PIXELS_LINE")"
  SPLIT_PIXEL_WIDTH="${SPLIT_PIXEL%x*}"
  SPLIT_PIXEL_HEIGHT="${SPLIT_PIXEL#*x}"
  log "PASS: observed split-right AppKit overlay frame overlay_frame_size=$SPLIT_FRAME_SIZE"
  log "PASS: observed split-right AppKit presented pixels appkit_pixel=$SPLIT_PIXEL"
  log "split_overlay_frame=$SPLIT_FRAME"
  log "split_overlay_frame_size=$SPLIT_FRAME_SIZE"
  log "split_overlay_frame_x=$SPLIT_FRAME_X"
  log "split_overlay_frame_y=$SPLIT_FRAME_Y"
  log "split_appkit_pixel=$SPLIT_PIXEL"
  require_log_after "$SPLIT_START_LINE" "TermSurf geometry layer=zig event=appkit_presented_pixels .*pane_id:${PANE_ID} .*appkit_pixel=${SPLIT_PIXEL}" "Zig records split-right AppKit presented pixel size"
  require_trace_after "$SPLIT_TRACE_START_LINE" "resize tab_id=${BROWSER_TAB_ID} pane_id=${PANE_ID} pixel_width=${SPLIT_PIXEL_WIDTH} pixel_height=${SPLIT_PIXEL_HEIGHT} screen_x=0 screen_y=0 screen_width=0 screen_height=0 screen_scale=0 ffi=ts_set_view_size" "Roamium applied split-right resize to AppKit pixel size via ts_set_view_size"

  SPLIT_WIN_LINE="$(window_bounds)" || fail "failed to resolve split window bounds for window id=$WID"
  IFS=$'\t' read -r _SPLIT_WID SPLIT_WX SPLIT_WY SPLIT_WW SPLIT_WH <<<"$SPLIT_WIN_LINE"
  INITIAL_FRAME_WIDTH="$(pair_width "$OVERLAY_FRAME_SIZE")"
  BROWSER_FOCUS_X="$(awk -v wx="$SPLIT_WX" -v frame_x="$SPLIT_FRAME_X" -v frame_w="$SPLIT_FRAME_WIDTH" 'BEGIN { print int(wx + frame_x + (frame_w / 2) + 0.5) }')"
  BROWSER_FOCUS_Y=$((SPLIT_WY + SPLIT_WH / 2))
  BROWSER_PANE_FOCUS_X="$BROWSER_FOCUS_X"
  BROWSER_PANE_FOCUS_Y=$((SPLIT_WY + SPLIT_WH - 40))
  SIBLING_FOCUS_X="$(awk -v wx="$SPLIT_WX" -v frame_x="$SPLIT_FRAME_X" -v split_w="$SPLIT_FRAME_WIDTH" -v initial_w="$INITIAL_FRAME_WIDTH" 'BEGIN { print int(wx + frame_x + split_w + ((initial_w - split_w) / 2) + 0.5) }')"
  SIBLING_FOCUS_Y="$BROWSER_FOCUS_Y"

  BROWSER_PRIME_TRACE_START_LINE="$(trace_line_count)"
  click_global_point "$BROWSER_PANE_FOCUS_X" "$BROWSER_PANE_FOCUS_Y" "browser_prime_terminal_focus"

  BROWSER_PRIME_HIT_START_LINE="$(log_line_count)"
  click_global_point "$BROWSER_FOCUS_X" "$BROWSER_FOCUS_Y" "browser_prime_focus"
  BROWSER_PRIME_HIT_LINE="$(wait_for_hit_after "$BROWSER_PRIME_HIT_START_LINE" "$CONTEXT_ID" "same-tab browser prime focus hit-test")"
  require_text "$BROWSER_PRIME_HIT_LINE" "web_point={" "browser prime focus hit-test includes webview-relative point"
  require_no_trace_after "$BROWSER_PRIME_TRACE_START_LINE" "focus-changed tab=${BROWSER_TAB_ID} pane=${PANE_ID} ffi=ts_set_focus focused=true" "browser pane focus in Control mode did not focus Roamium before Browse mode"

  SIBLING_FOCUS_TRACE_START_LINE="$(trace_line_count)"
  click_negative_global_point "$SIBLING_FOCUS_X" "$SIBLING_FOCUS_Y" "sibling_focus"
  wait_for_negative_hit_after "$NEGATIVE_HIT_START_LINE" "$CONTEXT_ID" "same-tab sibling focus negative hit-test" allow-absent

  SIBLING_KEY_START_LINE="$(log_line_count)"
  printf 'ISSUE809_EXP11_SIBLING_FOCUS\n' >"$SIBLING_FOCUS_COMMAND"
  swift "$ROOT/scripts/ghostty-app/inject.swift" type "$SIBLING_FOCUS_COMMAND" >>"$HARNESS_LOG" 2>&1
  SIBLING_KEY_SEEN=""
  for _ in $(seq 1 10); do
    if tail -n +"$((SIBLING_KEY_START_LINE + 1))" "$APP_LOG" | grep -E "TermSurf geometry layer=appkit event=key_down .*overlay_frame=none .*visible=false .*focused=true" >/dev/null 2>&1; then
      SIBLING_KEY_SEEN="1"
      break
    fi
    delay 1
  done
  [ -n "$SIBLING_KEY_SEEN" ] || fail "sibling terminal pane did not receive keyboard events after focus switch"
  log "PASS: sibling terminal pane received keyboard events after focus switch"
  require_no_trace_after "$SIBLING_FOCUS_TRACE_START_LINE" "key-event tab=${BROWSER_TAB_ID} pane=${PANE_ID}" "sibling terminal marker did not reach original browser context"
  require_trace_after "$SIBLING_FOCUS_TRACE_START_LINE" "focus-changed tab=${BROWSER_TAB_ID} pane=${PANE_ID} ffi=ts_set_focus focused=false" "Roamium observed original browser pane focus=false after sibling focus"
  require_no_different_appkit_frame_after "$SIBLING_KEY_START_LINE" "$PANE_ID" "$CONTEXT_ID" "$SPLIT_FRAME" "sibling focus kept original browser AppKit frame unchanged"
  require_no_different_appkit_pixels_after "$SIBLING_KEY_START_LINE" "$PANE_ID" "$CONTEXT_ID" "$SPLIT_PIXEL" "sibling focus kept original browser AppKit pixels unchanged"

  BROWSER_REFOCUS_TRACE_START_LINE="$(trace_line_count)"
  click_global_point "$BROWSER_PANE_FOCUS_X" "$BROWSER_PANE_FOCUS_Y" "browser_refocus_terminal_focus"

  BROWSER_REFOCUS_HIT_START_LINE="$(log_line_count)"
  click_global_point "$BROWSER_FOCUS_X" "$BROWSER_FOCUS_Y" "browser_refocus"
  BROWSER_REFOCUS_HIT_LINE="$(wait_for_hit_after "$BROWSER_REFOCUS_HIT_START_LINE" "$CONTEXT_ID" "same-tab browser refocus hit-test")"
  BROWSER_REFOCUS_FRAME_SIZE="$(extract_frame_size "$BROWSER_REFOCUS_HIT_LINE")"
  BROWSER_REFOCUS_FRAME_X="$(extract_frame_x "$BROWSER_REFOCUS_HIT_LINE")"
  BROWSER_REFOCUS_FRAME_Y="$(extract_frame_y "$BROWSER_REFOCUS_HIT_LINE")"
  [ "$BROWSER_REFOCUS_FRAME_SIZE" = "$SPLIT_FRAME_SIZE" ] || fail "browser refocus hit-test frame size changed: expected=$SPLIT_FRAME_SIZE actual=$BROWSER_REFOCUS_FRAME_SIZE"
  [ "$BROWSER_REFOCUS_FRAME_X" = "$SPLIT_FRAME_X" ] || fail "browser refocus hit-test frame x changed: expected=$SPLIT_FRAME_X actual=$BROWSER_REFOCUS_FRAME_X"
  [ "$BROWSER_REFOCUS_FRAME_Y" = "$SPLIT_FRAME_Y" ] || fail "browser refocus hit-test frame y changed: expected=$SPLIT_FRAME_Y actual=$BROWSER_REFOCUS_FRAME_Y"
  log "PASS: browser refocus hit-test uses split baseline overlay frame"
  require_text "$BROWSER_REFOCUS_HIT_LINE" "web_point={" "browser refocus hit-test includes webview-relative point"
  require_no_different_appkit_frame_after "$BROWSER_REFOCUS_HIT_START_LINE" "$PANE_ID" "$CONTEXT_ID" "$SPLIT_FRAME" "browser refocus kept original browser AppKit frame unchanged"
  require_no_different_appkit_pixels_after "$BROWSER_REFOCUS_HIT_START_LINE" "$PANE_ID" "$CONTEXT_ID" "$SPLIT_PIXEL" "browser refocus kept original browser AppKit pixels unchanged"
  require_no_trace_after "$BROWSER_REFOCUS_TRACE_START_LINE" "focus-changed tab=${BROWSER_TAB_ID} pane=${PANE_ID} ffi=ts_set_focus focused=true" "browser refocus in Control mode did not focus Roamium before Browse mode"

  BROWSER_MODE_START_LINE="$(log_line_count)"
  BROWSER_MODE_TRACE_START_LINE="$(trace_line_count)"
  log "browser_refocus_mode_key=enter=Mode::Browse"
  swift "$ROOT/scripts/ghostty-app/inject.swift" key 36 >>"$HARNESS_LOG" 2>&1
  wait_for_log_after "$BROWSER_MODE_START_LINE" "ModeChanged: pane_id=${PANE_ID} browsing=true" "webtui entered browse mode after browser refocus"
  require_trace_after "$BROWSER_MODE_TRACE_START_LINE" "focus-changed tab=${BROWSER_TAB_ID} pane=${PANE_ID} ffi=ts_set_focus focused=true" "Roamium observed original browser pane focus=true after browse mode"

  BROWSER_KEY_START_LINE="$(trace_line_count)"
  printf 'ISSUE809_EXP11_BROWSER_REFOCUS\n' >"$BROWSER_FOCUS_COMMAND"
  swift "$ROOT/scripts/ghostty-app/inject.swift" type "$BROWSER_FOCUS_COMMAND" >>"$HARNESS_LOG" 2>&1
  BROWSER_KEY_SEEN=""
  for _ in $(seq 1 10); do
    if tail -n +"$((BROWSER_KEY_START_LINE + 1))" "$ROAMIUM_TRACE" | grep -F "key-event tab=${BROWSER_TAB_ID} pane=${PANE_ID}" >/dev/null 2>&1; then
      BROWSER_KEY_SEEN="1"
      break
    fi
    delay 1
  done
  [ -n "$BROWSER_KEY_SEEN" ] || fail "browser refocus keyboard marker did not reach original browser context"
  log "PASS: browser refocus keyboard marker reached original browser context"

  screencapture -x -o -l"$WID" "$SCREENSHOT_SPLIT"
  log "focus_switch_screenshot_exit=$?"
fi

if [ "$SCENARIO" = "split-right-resize" ]; then
  SPLIT_START_LINE="$(log_line_count)"
  SPLIT_TRACE_START_LINE="$(trace_line_count)"
  log "split_keybind=ctrl+d=new_split:right"
  swift "$ROOT/scripts/ghostty-app/inject.swift" key 2 control >>"$HARNESS_LOG" 2>&1
  delay 1

  SPLIT_PRESENT_LINE="$(wait_for_split_right_frame_after "$SPLIT_START_LINE" "$PANE_ID" "$CONTEXT_ID" "$OVERLAY_FRAME_SIZE" "split-right AppKit overlay frame")"
  SPLIT_PIXELS_LINE="$(wait_for_split_right_pixels_after "$SPLIT_START_LINE" "$PANE_ID" "$CONTEXT_ID" "$APPKIT_PIXEL" "split-right AppKit presented pixels")"
  SPLIT_FRAME_SIZE="$(extract_frame_size "$SPLIT_PRESENT_LINE")"
  SPLIT_FRAME_X="$(extract_frame_x "$SPLIT_PRESENT_LINE")"
  SPLIT_FRAME_WIDTH="$(pair_width "$SPLIT_FRAME_SIZE")"
  SPLIT_PIXEL="$(extract_appkit_pixel "$SPLIT_PIXELS_LINE")"
  SPLIT_PIXEL_WIDTH="${SPLIT_PIXEL%x*}"
  SPLIT_PIXEL_HEIGHT="${SPLIT_PIXEL#*x}"
  log "PASS: observed split-right AppKit overlay frame overlay_frame_size=$SPLIT_FRAME_SIZE"
  log "PASS: observed split-right AppKit presented pixels appkit_pixel=$SPLIT_PIXEL"
  log "split_overlay_frame_size=$SPLIT_FRAME_SIZE"
  log "split_overlay_frame_x=$SPLIT_FRAME_X"
  log "split_appkit_pixel=$SPLIT_PIXEL"
  require_log_after "$SPLIT_START_LINE" "TermSurf geometry layer=zig event=appkit_presented_pixels .*pane_id:${PANE_ID} .*appkit_pixel=${SPLIT_PIXEL}" "Zig records split-right AppKit presented pixel size"
  require_trace_after "$SPLIT_TRACE_START_LINE" "resize tab_id=${BROWSER_TAB_ID} pane_id=${PANE_ID} pixel_width=${SPLIT_PIXEL_WIDTH} pixel_height=${SPLIT_PIXEL_HEIGHT} screen_x=0 screen_y=0 screen_width=0 screen_height=0 screen_scale=0 ffi=ts_set_view_size" "Roamium applied split-right resize to AppKit pixel size via ts_set_view_size"

  DIVIDER_START_LINE="$(log_line_count)"
  DIVIDER_TRACE_START_LINE="$(trace_line_count)"
  log "resize_split_keybind=ctrl+l=resize_split:right,20"
  swift "$ROOT/scripts/ghostty-app/inject.swift" key 37 control >>"$HARNESS_LOG" 2>&1
  delay 1

  DIVIDER_PRESENT_LINE="$(wait_for_split_right_resize_frame_after "$DIVIDER_START_LINE" "$PANE_ID" "$CONTEXT_ID" "$SPLIT_FRAME_SIZE" "split-right divider-resized AppKit overlay frame")"
  DIVIDER_PIXELS_LINE="$(wait_for_split_right_resize_pixels_after "$DIVIDER_START_LINE" "$PANE_ID" "$CONTEXT_ID" "$SPLIT_PIXEL" "split-right divider-resized AppKit presented pixels")"
  DIVIDER_FRAME_SIZE="$(extract_frame_size "$DIVIDER_PRESENT_LINE")"
  DIVIDER_FRAME_X="$(extract_frame_x "$DIVIDER_PRESENT_LINE")"
  DIVIDER_PIXEL="$(extract_appkit_pixel "$DIVIDER_PIXELS_LINE")"
  DIVIDER_PIXEL_WIDTH="${DIVIDER_PIXEL%x*}"
  DIVIDER_PIXEL_HEIGHT="${DIVIDER_PIXEL#*x}"
  log "PASS: observed split-right divider-resized AppKit overlay frame overlay_frame_size=$DIVIDER_FRAME_SIZE"
  log "PASS: observed split-right divider-resized AppKit presented pixels appkit_pixel=$DIVIDER_PIXEL"
  log "divider_overlay_frame_size=$DIVIDER_FRAME_SIZE"
  log "divider_overlay_frame_x=$DIVIDER_FRAME_X"
  log "divider_appkit_pixel=$DIVIDER_PIXEL"
  require_log_after "$DIVIDER_START_LINE" "TermSurf geometry layer=zig event=appkit_presented_pixels .*pane_id:${PANE_ID} .*appkit_pixel=${DIVIDER_PIXEL}" "Zig records split-right divider-resized AppKit presented pixel size"
  require_trace_after "$DIVIDER_TRACE_START_LINE" "resize tab_id=${BROWSER_TAB_ID} pane_id=${PANE_ID} pixel_width=${DIVIDER_PIXEL_WIDTH} pixel_height=${DIVIDER_PIXEL_HEIGHT} screen_x=0 screen_y=0 screen_width=0 screen_height=0 screen_scale=0 ffi=ts_set_view_size" "Roamium applied split-right divider resize to AppKit pixel size via ts_set_view_size"
  screencapture -x -o -l"$WID" "$SCREENSHOT_SPLIT"
  log "divider_screenshot_exit=$?"

  DIVIDER_WIN_LINE="$(window_bounds)" || fail "failed to resolve divider-resized window bounds for window id=$WID"
  IFS=$'\t' read -r _DIVIDER_WID DIVIDER_WX DIVIDER_WY DIVIDER_WW DIVIDER_WH <<<"$DIVIDER_WIN_LINE"
  DIVIDER_FRAME_WIDTH="$(pair_width "$DIVIDER_FRAME_SIZE")"
  DIVIDER_INSIDE_X="$(awk -v wx="$DIVIDER_WX" -v frame_x="$DIVIDER_FRAME_X" -v frame_w="$DIVIDER_FRAME_WIDTH" 'BEGIN { print int(wx + frame_x + (frame_w / 2) + 0.5) }')"
  DIVIDER_INSIDE_Y=$((DIVIDER_WY + DIVIDER_WH / 2))
  DIVIDER_HIT_START_LINE="$(log_line_count)"
  click_global_point "$DIVIDER_INSIDE_X" "$DIVIDER_INSIDE_Y" "divider_inside"
  DIVIDER_HIT_LINE="$(wait_for_hit_after "$DIVIDER_HIT_START_LINE" "$CONTEXT_ID" "split-right divider-resized AppKit hit-test")"
  log "PASS: observed split-right divider-resized AppKit hit-test"
  require_text "$DIVIDER_HIT_LINE" "overlay_frame=" "split-right divider-resized hit-test includes current overlay frame"
  require_text "$DIVIDER_HIT_LINE" "web_point={" "split-right divider-resized hit-test includes webview-relative point"

  DIVIDER_NEGATIVE_X="$(awk -v wx="$DIVIDER_WX" -v frame_x="$DIVIDER_FRAME_X" -v frame_w="$DIVIDER_FRAME_WIDTH" -v ww="$DIVIDER_WW" 'BEGIN { print int(wx + frame_x + frame_w + ((ww - frame_x - frame_w) / 2) + 0.5) }')"
  DIVIDER_NEGATIVE_Y="$DIVIDER_INSIDE_Y"
  click_negative_global_point "$DIVIDER_NEGATIVE_X" "$DIVIDER_NEGATIVE_Y" "divider_sibling_negative"
  wait_for_negative_hit_after "$NEGATIVE_HIT_START_LINE" "$CONTEXT_ID" "split-right divider-resized sibling-pane negative hit-test" allow-absent
fi

if [ "$SCENARIO" = "split-right-equalize" ]; then
  SPLIT_START_LINE="$(log_line_count)"
  SPLIT_TRACE_START_LINE="$(trace_line_count)"
  log "split_keybind=ctrl+d=new_split:right"
  swift "$ROOT/scripts/ghostty-app/inject.swift" key 2 control >>"$HARNESS_LOG" 2>&1
  delay 1

  SPLIT_PRESENT_LINE="$(wait_for_split_right_frame_after "$SPLIT_START_LINE" "$PANE_ID" "$CONTEXT_ID" "$OVERLAY_FRAME_SIZE" "split-right AppKit overlay frame")"
  SPLIT_PIXELS_LINE="$(wait_for_split_right_pixels_after "$SPLIT_START_LINE" "$PANE_ID" "$CONTEXT_ID" "$APPKIT_PIXEL" "split-right AppKit presented pixels")"
  SPLIT_FRAME_SIZE="$(extract_frame_size "$SPLIT_PRESENT_LINE")"
  SPLIT_FRAME_X="$(extract_frame_x "$SPLIT_PRESENT_LINE")"
  SPLIT_FRAME_WIDTH="$(pair_width "$SPLIT_FRAME_SIZE")"
  SPLIT_PIXEL="$(extract_appkit_pixel "$SPLIT_PIXELS_LINE")"
  SPLIT_PIXEL_WIDTH="${SPLIT_PIXEL%x*}"
  SPLIT_PIXEL_HEIGHT="${SPLIT_PIXEL#*x}"
  log "PASS: observed split-right AppKit overlay frame overlay_frame_size=$SPLIT_FRAME_SIZE"
  log "PASS: observed split-right AppKit presented pixels appkit_pixel=$SPLIT_PIXEL"
  log "split_overlay_frame_size=$SPLIT_FRAME_SIZE"
  log "split_overlay_frame_x=$SPLIT_FRAME_X"
  log "split_appkit_pixel=$SPLIT_PIXEL"
  require_log_after "$SPLIT_START_LINE" "TermSurf geometry layer=zig event=appkit_presented_pixels .*pane_id:${PANE_ID} .*appkit_pixel=${SPLIT_PIXEL}" "Zig records split-right AppKit presented pixel size"
  require_trace_after "$SPLIT_TRACE_START_LINE" "resize tab_id=${BROWSER_TAB_ID} pane_id=${PANE_ID} pixel_width=${SPLIT_PIXEL_WIDTH} pixel_height=${SPLIT_PIXEL_HEIGHT} screen_x=0 screen_y=0 screen_width=0 screen_height=0 screen_scale=0 ffi=ts_set_view_size" "Roamium applied split-right resize to AppKit pixel size via ts_set_view_size"

  DIVIDER_START_LINE="$(log_line_count)"
  DIVIDER_TRACE_START_LINE="$(trace_line_count)"
  log "resize_split_keybind=ctrl+l=resize_split:right,20"
  swift "$ROOT/scripts/ghostty-app/inject.swift" key 37 control >>"$HARNESS_LOG" 2>&1
  delay 1

  DIVIDER_PRESENT_LINE="$(wait_for_split_right_resize_frame_after "$DIVIDER_START_LINE" "$PANE_ID" "$CONTEXT_ID" "$SPLIT_FRAME_SIZE" "split-right divider-resized AppKit overlay frame")"
  DIVIDER_PIXELS_LINE="$(wait_for_split_right_resize_pixels_after "$DIVIDER_START_LINE" "$PANE_ID" "$CONTEXT_ID" "$SPLIT_PIXEL" "split-right divider-resized AppKit presented pixels")"
  DIVIDER_FRAME_SIZE="$(extract_frame_size "$DIVIDER_PRESENT_LINE")"
  DIVIDER_PIXEL="$(extract_appkit_pixel "$DIVIDER_PIXELS_LINE")"
  DIVIDER_PIXEL_WIDTH="${DIVIDER_PIXEL%x*}"
  DIVIDER_PIXEL_HEIGHT="${DIVIDER_PIXEL#*x}"
  log "PASS: observed split-right divider-resized AppKit overlay frame overlay_frame_size=$DIVIDER_FRAME_SIZE"
  log "PASS: observed split-right divider-resized AppKit presented pixels appkit_pixel=$DIVIDER_PIXEL"
  log "divider_overlay_frame_size=$DIVIDER_FRAME_SIZE"
  log "divider_appkit_pixel=$DIVIDER_PIXEL"
  require_log_after "$DIVIDER_START_LINE" "TermSurf geometry layer=zig event=appkit_presented_pixels .*pane_id:${PANE_ID} .*appkit_pixel=${DIVIDER_PIXEL}" "Zig records split-right divider-resized AppKit presented pixel size"
  require_trace_after "$DIVIDER_TRACE_START_LINE" "resize tab_id=${BROWSER_TAB_ID} pane_id=${PANE_ID} pixel_width=${DIVIDER_PIXEL_WIDTH} pixel_height=${DIVIDER_PIXEL_HEIGHT} screen_x=0 screen_y=0 screen_width=0 screen_height=0 screen_scale=0 ffi=ts_set_view_size" "Roamium applied split-right divider resize to AppKit pixel size via ts_set_view_size"

  EQUALIZE_START_LINE="$(log_line_count)"
  EQUALIZE_TRACE_START_LINE="$(trace_line_count)"
  log "equalize_keybind=ctrl+e=equalize_splits"
  swift "$ROOT/scripts/ghostty-app/inject.swift" key 14 control >>"$HARNESS_LOG" 2>&1
  delay 1

  EQUALIZE_PRESENT_LINE="$(wait_for_split_right_equalize_frame_after "$EQUALIZE_START_LINE" "$PANE_ID" "$CONTEXT_ID" "$SPLIT_FRAME_SIZE" "$DIVIDER_FRAME_SIZE" "split-right equalized AppKit overlay frame")"
  EQUALIZE_PIXELS_LINE="$(wait_for_split_right_equalize_pixels_after "$EQUALIZE_START_LINE" "$PANE_ID" "$CONTEXT_ID" "$SPLIT_PIXEL" "$DIVIDER_PIXEL" "split-right equalized AppKit presented pixels")"
  EQUALIZE_FRAME_SIZE="$(extract_frame_size "$EQUALIZE_PRESENT_LINE")"
  EQUALIZE_FRAME_X="$(extract_frame_x "$EQUALIZE_PRESENT_LINE")"
  EQUALIZE_PIXEL="$(extract_appkit_pixel "$EQUALIZE_PIXELS_LINE")"
  EQUALIZE_PIXEL_WIDTH="${EQUALIZE_PIXEL%x*}"
  EQUALIZE_PIXEL_HEIGHT="${EQUALIZE_PIXEL#*x}"
  log "PASS: observed split-right equalized AppKit overlay frame overlay_frame_size=$EQUALIZE_FRAME_SIZE"
  log "PASS: observed split-right equalized AppKit presented pixels appkit_pixel=$EQUALIZE_PIXEL"
  log "equalize_overlay_frame_size=$EQUALIZE_FRAME_SIZE"
  log "equalize_overlay_frame_x=$EQUALIZE_FRAME_X"
  log "equalize_appkit_pixel=$EQUALIZE_PIXEL"
  require_log_after "$EQUALIZE_START_LINE" "TermSurf geometry layer=zig event=appkit_presented_pixels .*pane_id:${PANE_ID} .*appkit_pixel=${EQUALIZE_PIXEL}" "Zig records split-right equalized AppKit presented pixel size"
  require_trace_after "$EQUALIZE_TRACE_START_LINE" "resize tab_id=${BROWSER_TAB_ID} pane_id=${PANE_ID} pixel_width=${EQUALIZE_PIXEL_WIDTH} pixel_height=${EQUALIZE_PIXEL_HEIGHT} screen_x=0 screen_y=0 screen_width=0 screen_height=0 screen_scale=0 ffi=ts_set_view_size" "Roamium applied split-right equalized resize to AppKit pixel size via ts_set_view_size"
  screencapture -x -o -l"$WID" "$SCREENSHOT_SPLIT"
  log "equalize_screenshot_exit=$?"

  EQUALIZE_WIN_LINE="$(window_bounds)" || fail "failed to resolve equalized window bounds for window id=$WID"
  IFS=$'\t' read -r _EQUALIZE_WID EQUALIZE_WX EQUALIZE_WY EQUALIZE_WW EQUALIZE_WH <<<"$EQUALIZE_WIN_LINE"
  EQUALIZE_FRAME_WIDTH="$(pair_width "$EQUALIZE_FRAME_SIZE")"
  EQUALIZE_INSIDE_X="$(awk -v wx="$EQUALIZE_WX" -v frame_x="$EQUALIZE_FRAME_X" -v frame_w="$EQUALIZE_FRAME_WIDTH" 'BEGIN { print int(wx + frame_x + (frame_w / 2) + 0.5) }')"
  EQUALIZE_INSIDE_Y=$((EQUALIZE_WY + EQUALIZE_WH / 2))
  EQUALIZE_HIT_START_LINE="$(log_line_count)"
  click_global_point "$EQUALIZE_INSIDE_X" "$EQUALIZE_INSIDE_Y" "equalize_inside"
  EQUALIZE_HIT_LINE="$(wait_for_hit_after "$EQUALIZE_HIT_START_LINE" "$CONTEXT_ID" "split-right equalized AppKit hit-test")"
  log "PASS: observed split-right equalized AppKit hit-test"
  require_text "$EQUALIZE_HIT_LINE" "overlay_frame=" "split-right equalized hit-test includes current overlay frame"
  require_text "$EQUALIZE_HIT_LINE" "web_point={" "split-right equalized hit-test includes webview-relative point"

  EQUALIZE_NEGATIVE_X="$(awk -v wx="$EQUALIZE_WX" -v frame_x="$EQUALIZE_FRAME_X" -v frame_w="$EQUALIZE_FRAME_WIDTH" -v ww="$EQUALIZE_WW" 'BEGIN { print int(wx + frame_x + frame_w + ((ww - frame_x - frame_w) / 2) + 0.5) }')"
  EQUALIZE_NEGATIVE_Y="$EQUALIZE_INSIDE_Y"
  click_negative_global_point "$EQUALIZE_NEGATIVE_X" "$EQUALIZE_NEGATIVE_Y" "equalize_sibling_negative"
  wait_for_negative_hit_after "$NEGATIVE_HIT_START_LINE" "$CONTEXT_ID" "split-right equalized sibling-pane negative hit-test" allow-absent
fi

if [ "$SCENARIO" = "split-right-zoom" ]; then
  SPLIT_START_LINE="$(log_line_count)"
  SPLIT_TRACE_START_LINE="$(trace_line_count)"
  log "split_keybind=ctrl+d=new_split:right"
  swift "$ROOT/scripts/ghostty-app/inject.swift" key 2 control >>"$HARNESS_LOG" 2>&1
  delay 1

  SPLIT_PRESENT_LINE="$(wait_for_split_right_frame_after "$SPLIT_START_LINE" "$PANE_ID" "$CONTEXT_ID" "$OVERLAY_FRAME_SIZE" "split-right AppKit overlay frame")"
  SPLIT_PIXELS_LINE="$(wait_for_split_right_pixels_after "$SPLIT_START_LINE" "$PANE_ID" "$CONTEXT_ID" "$APPKIT_PIXEL" "split-right AppKit presented pixels")"
  SPLIT_FRAME_SIZE="$(extract_frame_size "$SPLIT_PRESENT_LINE")"
  SPLIT_FRAME_X="$(extract_frame_x "$SPLIT_PRESENT_LINE")"
  SPLIT_PIXEL="$(extract_appkit_pixel "$SPLIT_PIXELS_LINE")"
  SPLIT_PIXEL_WIDTH="${SPLIT_PIXEL%x*}"
  SPLIT_PIXEL_HEIGHT="${SPLIT_PIXEL#*x}"
  log "PASS: observed split-right AppKit overlay frame overlay_frame_size=$SPLIT_FRAME_SIZE"
  log "PASS: observed split-right AppKit presented pixels appkit_pixel=$SPLIT_PIXEL"
  log "split_overlay_frame_size=$SPLIT_FRAME_SIZE"
  log "split_overlay_frame_x=$SPLIT_FRAME_X"
  log "split_appkit_pixel=$SPLIT_PIXEL"
  require_log_after "$SPLIT_START_LINE" "TermSurf geometry layer=zig event=appkit_presented_pixels .*pane_id:${PANE_ID} .*appkit_pixel=${SPLIT_PIXEL}" "Zig records split-right AppKit presented pixel size"
  require_trace_after "$SPLIT_TRACE_START_LINE" "resize tab_id=${BROWSER_TAB_ID} pane_id=${PANE_ID} pixel_width=${SPLIT_PIXEL_WIDTH} pixel_height=${SPLIT_PIXEL_HEIGHT} screen_x=0 screen_y=0 screen_width=0 screen_height=0 screen_scale=0 ffi=ts_set_view_size" "Roamium applied split-right resize to AppKit pixel size via ts_set_view_size"

  SPLIT_FOCUS_WIN_LINE="$(window_bounds)" || fail "failed to resolve split window bounds for window id=$WID"
  IFS=$'\t' read -r _SPLIT_FOCUS_WID SPLIT_FOCUS_WX SPLIT_FOCUS_WY SPLIT_FOCUS_WW SPLIT_FOCUS_WH <<<"$SPLIT_FOCUS_WIN_LINE"
  SPLIT_FRAME_WIDTH="$(pair_width "$SPLIT_FRAME_SIZE")"
  SPLIT_FOCUS_X="$(awk -v wx="$SPLIT_FOCUS_WX" -v frame_x="$SPLIT_FRAME_X" -v frame_w="$SPLIT_FRAME_WIDTH" 'BEGIN { print int(wx + frame_x + (frame_w / 2) + 0.5) }')"
  SPLIT_FOCUS_Y=$((SPLIT_FOCUS_WY + SPLIT_FOCUS_WH / 2))
  SPLIT_FOCUS_HIT_START_LINE="$(log_line_count)"
  click_global_point "$SPLIT_FOCUS_X" "$SPLIT_FOCUS_Y" "split_focus_browser"
  SPLIT_FOCUS_HIT_LINE="$(wait_for_hit_after "$SPLIT_FOCUS_HIT_START_LINE" "$CONTEXT_ID" "split-right browser-pane focus hit-test")"
  log "PASS: focused split-right browser pane before zoom"
  require_text "$SPLIT_FOCUS_HIT_LINE" "overlay_frame=" "split-right browser-pane focus hit-test includes current overlay frame"
  require_text "$SPLIT_FOCUS_HIT_LINE" "web_point={" "split-right browser-pane focus hit-test includes webview-relative point"

  ZOOM_START_LINE="$(log_line_count)"
  ZOOM_TRACE_START_LINE="$(trace_line_count)"
  log "zoom_keybind=ctrl+z=toggle_split_zoom"
  swift "$ROOT/scripts/ghostty-app/inject.swift" key 6 control >>"$HARNESS_LOG" 2>&1
  delay 1

  ZOOM_PRESENT_LINE="$(wait_for_split_right_zoom_frame_after "$ZOOM_START_LINE" "$PANE_ID" "$CONTEXT_ID" "$OVERLAY_FRAME_SIZE" "$SPLIT_FRAME_SIZE" "split-right zoomed AppKit overlay frame")"
  ZOOM_PIXELS_LINE="$(wait_for_split_right_zoom_pixels_after "$ZOOM_START_LINE" "$PANE_ID" "$CONTEXT_ID" "$APPKIT_PIXEL" "$SPLIT_PIXEL" "split-right zoomed AppKit presented pixels")"
  ZOOM_FRAME_SIZE="$(extract_frame_size "$ZOOM_PRESENT_LINE")"
  ZOOM_FRAME_X="$(extract_frame_x "$ZOOM_PRESENT_LINE")"
  ZOOM_PIXEL="$(extract_appkit_pixel "$ZOOM_PIXELS_LINE")"
  ZOOM_PIXEL_WIDTH="${ZOOM_PIXEL%x*}"
  ZOOM_PIXEL_HEIGHT="${ZOOM_PIXEL#*x}"
  log "PASS: observed split-right zoomed AppKit overlay frame overlay_frame_size=$ZOOM_FRAME_SIZE"
  log "PASS: observed split-right zoomed AppKit presented pixels appkit_pixel=$ZOOM_PIXEL"
  log "zoom_overlay_frame_size=$ZOOM_FRAME_SIZE"
  log "zoom_overlay_frame_x=$ZOOM_FRAME_X"
  log "zoom_appkit_pixel=$ZOOM_PIXEL"
  require_log_after "$ZOOM_START_LINE" "TermSurf geometry layer=zig event=appkit_presented_pixels .*pane_id:${PANE_ID} .*appkit_pixel=${ZOOM_PIXEL}" "Zig records split-right zoomed AppKit presented pixel size"
  require_trace_after "$ZOOM_TRACE_START_LINE" "resize tab_id=${BROWSER_TAB_ID} pane_id=${PANE_ID} pixel_width=${ZOOM_PIXEL_WIDTH} pixel_height=${ZOOM_PIXEL_HEIGHT} screen_x=0 screen_y=0 screen_width=0 screen_height=0 screen_scale=0 ffi=ts_set_view_size" "Roamium applied split-right zoomed resize to AppKit pixel size via ts_set_view_size"
  screencapture -x -o -l"$WID" "$SCREENSHOT_ZOOM"
  log "zoom_screenshot_exit=$?"

  ZOOM_WIN_LINE="$(window_bounds)" || fail "failed to resolve zoomed window bounds for window id=$WID"
  IFS=$'\t' read -r _ZOOM_WID ZOOM_WX ZOOM_WY ZOOM_WW ZOOM_WH <<<"$ZOOM_WIN_LINE"
  ZOOM_FRAME_WIDTH="$(pair_width "$ZOOM_FRAME_SIZE")"
  ZOOM_INSIDE_X="$(awk -v wx="$ZOOM_WX" -v frame_x="$ZOOM_FRAME_X" -v frame_w="$ZOOM_FRAME_WIDTH" 'BEGIN { print int(wx + frame_x + (frame_w / 2) + 0.5) }')"
  ZOOM_INSIDE_Y=$((ZOOM_WY + ZOOM_WH / 2))
  ZOOM_HIT_START_LINE="$(log_line_count)"
  click_global_point "$ZOOM_INSIDE_X" "$ZOOM_INSIDE_Y" "zoom_inside"
  ZOOM_HIT_LINE="$(wait_for_hit_after "$ZOOM_HIT_START_LINE" "$CONTEXT_ID" "split-right zoomed AppKit hit-test")"
  log "PASS: observed split-right zoomed AppKit hit-test"
  require_text "$ZOOM_HIT_LINE" "overlay_frame=" "split-right zoomed hit-test includes current overlay frame"
  require_text "$ZOOM_HIT_LINE" "web_point={" "split-right zoomed hit-test includes webview-relative point"

  UNZOOM_START_LINE="$(log_line_count)"
  UNZOOM_TRACE_START_LINE="$(trace_line_count)"
  log "unzoom_keybind=ctrl+z=toggle_split_zoom"
  swift "$ROOT/scripts/ghostty-app/inject.swift" key 6 control >>"$HARNESS_LOG" 2>&1
  delay 1

  UNZOOM_PRESENT_LINE="$(wait_for_split_right_equalize_frame_after "$UNZOOM_START_LINE" "$PANE_ID" "$CONTEXT_ID" "$SPLIT_FRAME_SIZE" "$ZOOM_FRAME_SIZE" "split-right unzoomed AppKit overlay frame")"
  UNZOOM_PIXELS_LINE="$(wait_for_split_right_equalize_pixels_after "$UNZOOM_START_LINE" "$PANE_ID" "$CONTEXT_ID" "$SPLIT_PIXEL" "$ZOOM_PIXEL" "split-right unzoomed AppKit presented pixels")"
  UNZOOM_FRAME_SIZE="$(extract_frame_size "$UNZOOM_PRESENT_LINE")"
  UNZOOM_FRAME_X="$(extract_frame_x "$UNZOOM_PRESENT_LINE")"
  UNZOOM_PIXEL="$(extract_appkit_pixel "$UNZOOM_PIXELS_LINE")"
  UNZOOM_PIXEL_WIDTH="${UNZOOM_PIXEL%x*}"
  UNZOOM_PIXEL_HEIGHT="${UNZOOM_PIXEL#*x}"
  log "PASS: observed split-right unzoomed AppKit overlay frame overlay_frame_size=$UNZOOM_FRAME_SIZE"
  log "PASS: observed split-right unzoomed AppKit presented pixels appkit_pixel=$UNZOOM_PIXEL"
  log "unzoom_overlay_frame_size=$UNZOOM_FRAME_SIZE"
  log "unzoom_overlay_frame_x=$UNZOOM_FRAME_X"
  log "unzoom_appkit_pixel=$UNZOOM_PIXEL"
  require_log_after "$UNZOOM_START_LINE" "TermSurf geometry layer=zig event=appkit_presented_pixels .*pane_id:${PANE_ID} .*appkit_pixel=${UNZOOM_PIXEL}" "Zig records split-right unzoomed AppKit presented pixel size"
  require_trace_after "$UNZOOM_TRACE_START_LINE" "resize tab_id=${BROWSER_TAB_ID} pane_id=${PANE_ID} pixel_width=${UNZOOM_PIXEL_WIDTH} pixel_height=${UNZOOM_PIXEL_HEIGHT} screen_x=0 screen_y=0 screen_width=0 screen_height=0 screen_scale=0 ffi=ts_set_view_size" "Roamium applied split-right unzoomed resize to AppKit pixel size via ts_set_view_size"
  screencapture -x -o -l"$WID" "$SCREENSHOT_UNZOOM"
  log "unzoom_screenshot_exit=$?"

  UNZOOM_WIN_LINE="$(window_bounds)" || fail "failed to resolve unzoomed window bounds for window id=$WID"
  IFS=$'\t' read -r _UNZOOM_WID UNZOOM_WX UNZOOM_WY UNZOOM_WW UNZOOM_WH <<<"$UNZOOM_WIN_LINE"
  UNZOOM_FRAME_WIDTH="$(pair_width "$UNZOOM_FRAME_SIZE")"
  UNZOOM_INSIDE_X="$(awk -v wx="$UNZOOM_WX" -v frame_x="$UNZOOM_FRAME_X" -v frame_w="$UNZOOM_FRAME_WIDTH" 'BEGIN { print int(wx + frame_x + (frame_w / 2) + 0.5) }')"
  UNZOOM_INSIDE_Y=$((UNZOOM_WY + UNZOOM_WH / 2))
  UNZOOM_HIT_START_LINE="$(log_line_count)"
  click_global_point "$UNZOOM_INSIDE_X" "$UNZOOM_INSIDE_Y" "unzoom_inside"
  UNZOOM_HIT_LINE="$(wait_for_hit_after "$UNZOOM_HIT_START_LINE" "$CONTEXT_ID" "split-right unzoomed AppKit hit-test")"
  log "PASS: observed split-right unzoomed AppKit hit-test"
  require_text "$UNZOOM_HIT_LINE" "overlay_frame=" "split-right unzoomed hit-test includes current overlay frame"
  require_text "$UNZOOM_HIT_LINE" "web_point={" "split-right unzoomed hit-test includes webview-relative point"

  UNZOOM_NEGATIVE_X="$(awk -v wx="$UNZOOM_WX" -v frame_x="$UNZOOM_FRAME_X" -v frame_w="$UNZOOM_FRAME_WIDTH" -v ww="$UNZOOM_WW" 'BEGIN { print int(wx + frame_x + frame_w + ((ww - frame_x - frame_w) / 2) + 0.5) }')"
  UNZOOM_NEGATIVE_Y="$UNZOOM_INSIDE_Y"
  click_negative_global_point "$UNZOOM_NEGATIVE_X" "$UNZOOM_NEGATIVE_Y" "unzoom_sibling_negative"
  wait_for_negative_hit_after "$NEGATIVE_HIT_START_LINE" "$CONTEXT_ID" "split-right unzoomed sibling-pane negative hit-test" allow-absent
fi

if [ "$SCENARIO" = "split-right-close-sibling" ]; then
  log "confirm_close_surface=false"
  SPLIT_START_LINE="$(log_line_count)"
  SPLIT_TRACE_START_LINE="$(trace_line_count)"
  log "split_keybind=ctrl+d=new_split:right"
  swift "$ROOT/scripts/ghostty-app/inject.swift" key 2 control >>"$HARNESS_LOG" 2>&1
  delay 1

  SPLIT_PRESENT_LINE="$(wait_for_split_right_frame_after "$SPLIT_START_LINE" "$PANE_ID" "$CONTEXT_ID" "$OVERLAY_FRAME_SIZE" "split-right AppKit overlay frame")"
  SPLIT_PIXELS_LINE="$(wait_for_split_right_pixels_after "$SPLIT_START_LINE" "$PANE_ID" "$CONTEXT_ID" "$APPKIT_PIXEL" "split-right AppKit presented pixels")"
  SPLIT_FRAME_SIZE="$(extract_frame_size "$SPLIT_PRESENT_LINE")"
  SPLIT_FRAME_X="$(extract_frame_x "$SPLIT_PRESENT_LINE")"
  SPLIT_PIXEL="$(extract_appkit_pixel "$SPLIT_PIXELS_LINE")"
  SPLIT_PIXEL_WIDTH="${SPLIT_PIXEL%x*}"
  SPLIT_PIXEL_HEIGHT="${SPLIT_PIXEL#*x}"
  log "PASS: observed split-right AppKit overlay frame overlay_frame_size=$SPLIT_FRAME_SIZE"
  log "PASS: observed split-right AppKit presented pixels appkit_pixel=$SPLIT_PIXEL"
  log "split_overlay_frame_size=$SPLIT_FRAME_SIZE"
  log "split_overlay_frame_x=$SPLIT_FRAME_X"
  log "split_appkit_pixel=$SPLIT_PIXEL"
  require_log_after "$SPLIT_START_LINE" "TermSurf geometry layer=zig event=appkit_presented_pixels .*pane_id:${PANE_ID} .*appkit_pixel=${SPLIT_PIXEL}" "Zig records split-right AppKit presented pixel size"
  require_trace_after "$SPLIT_TRACE_START_LINE" "resize tab_id=${BROWSER_TAB_ID} pane_id=${PANE_ID} pixel_width=${SPLIT_PIXEL_WIDTH} pixel_height=${SPLIT_PIXEL_HEIGHT} screen_x=0 screen_y=0 screen_width=0 screen_height=0 screen_scale=0 ffi=ts_set_view_size" "Roamium applied split-right resize to AppKit pixel size via ts_set_view_size"

  SPLIT_FRAME_WIDTH="$(pair_width "$SPLIT_FRAME_SIZE")"
  CLOSE_START_LINE="$(log_line_count)"
  CLOSE_TRACE_START_LINE="$(trace_line_count)"
  log "close_keybind=ctrl+k=close_surface"
  swift "$ROOT/scripts/ghostty-app/inject.swift" key 40 control >>"$HARNESS_LOG" 2>&1
  delay 1

  CLOSE_PRESENT_LINE="$(wait_for_split_right_zoom_frame_after "$CLOSE_START_LINE" "$PANE_ID" "$CONTEXT_ID" "$OVERLAY_FRAME_SIZE" "$SPLIT_FRAME_SIZE" "split-right sibling-closed AppKit overlay frame")"
  CLOSE_PIXELS_LINE="$(wait_for_split_right_zoom_pixels_after "$CLOSE_START_LINE" "$PANE_ID" "$CONTEXT_ID" "$APPKIT_PIXEL" "$SPLIT_PIXEL" "split-right sibling-closed AppKit presented pixels")"
  CLOSE_FRAME_SIZE="$(extract_frame_size "$CLOSE_PRESENT_LINE")"
  CLOSE_FRAME_X="$(extract_frame_x "$CLOSE_PRESENT_LINE")"
  CLOSE_PIXEL="$(extract_appkit_pixel "$CLOSE_PIXELS_LINE")"
  CLOSE_PIXEL_WIDTH="${CLOSE_PIXEL%x*}"
  CLOSE_PIXEL_HEIGHT="${CLOSE_PIXEL#*x}"
  log "PASS: observed split-right sibling-closed AppKit overlay frame overlay_frame_size=$CLOSE_FRAME_SIZE"
  log "PASS: observed split-right sibling-closed AppKit presented pixels appkit_pixel=$CLOSE_PIXEL"
  log "close_overlay_frame_size=$CLOSE_FRAME_SIZE"
  log "close_overlay_frame_x=$CLOSE_FRAME_X"
  log "close_appkit_pixel=$CLOSE_PIXEL"
  require_log_after "$CLOSE_START_LINE" "TermSurf geometry layer=zig event=appkit_presented_pixels .*pane_id:${PANE_ID} .*appkit_pixel=${CLOSE_PIXEL}" "Zig records split-right sibling-closed AppKit presented pixel size"
  require_trace_after "$CLOSE_TRACE_START_LINE" "resize tab_id=${BROWSER_TAB_ID} pane_id=${PANE_ID} pixel_width=${CLOSE_PIXEL_WIDTH} pixel_height=${CLOSE_PIXEL_HEIGHT} screen_x=0 screen_y=0 screen_width=0 screen_height=0 screen_scale=0 ffi=ts_set_view_size" "Roamium applied split-right sibling-closed resize to AppKit pixel size via ts_set_view_size"
  screencapture -x -o -l"$WID" "$SCREENSHOT_CLOSE"
  log "close_screenshot_exit=$?"

  CLOSE_WIN_LINE="$(window_bounds)" || fail "failed to resolve sibling-closed window bounds for window id=$WID"
  IFS=$'\t' read -r _CLOSE_WID CLOSE_WX CLOSE_WY CLOSE_WW CLOSE_WH <<<"$CLOSE_WIN_LINE"
  CLOSE_FRAME_WIDTH="$(pair_width "$CLOSE_FRAME_SIZE")"
  CLOSE_INSIDE_X="$(awk -v wx="$CLOSE_WX" -v frame_x="$CLOSE_FRAME_X" -v frame_w="$CLOSE_FRAME_WIDTH" 'BEGIN { print int(wx + frame_x + (frame_w / 2) + 0.5) }')"
  CLOSE_INSIDE_Y=$((CLOSE_WY + CLOSE_WH / 2))
  CLOSE_HIT_START_LINE="$(log_line_count)"
  click_global_point "$CLOSE_INSIDE_X" "$CLOSE_INSIDE_Y" "close_inside"
  CLOSE_HIT_LINE="$(wait_for_hit_after "$CLOSE_HIT_START_LINE" "$CONTEXT_ID" "split-right sibling-closed AppKit hit-test")"
  log "PASS: observed split-right sibling-closed AppKit hit-test"
  require_text "$CLOSE_HIT_LINE" "overlay_frame=" "split-right sibling-closed hit-test includes current overlay frame"
  require_text "$CLOSE_HIT_LINE" "web_point={" "split-right sibling-closed hit-test includes webview-relative point"

  FORMER_SIBLING_X="$(awk -v wx="$CLOSE_WX" -v frame_x="$CLOSE_FRAME_X" -v split_w="$SPLIT_FRAME_WIDTH" -v close_w="$CLOSE_FRAME_WIDTH" 'BEGIN { print int(wx + frame_x + split_w + ((close_w - split_w) / 2) + 0.5) }')"
  FORMER_SIBLING_Y="$CLOSE_INSIDE_Y"
  FORMER_SIBLING_HIT_START_LINE="$(log_line_count)"
  click_global_point "$FORMER_SIBLING_X" "$FORMER_SIBLING_Y" "former_sibling_inside"
  FORMER_SIBLING_HIT_LINE="$(wait_for_hit_after "$FORMER_SIBLING_HIT_START_LINE" "$CONTEXT_ID" "former sibling area AppKit hit-test after close")"
  log "PASS: observed former sibling area AppKit hit-test after close"
  require_text "$FORMER_SIBLING_HIT_LINE" "overlay_frame=" "former sibling area hit-test includes current overlay frame"
  require_text "$FORMER_SIBLING_HIT_LINE" "web_point={" "former sibling area hit-test includes webview-relative point"
fi

if [ "$SCENARIO" = "split-right-close-browser-pane" ]; then
  log "confirm_close_surface=false"
  SPLIT_START_LINE="$(log_line_count)"
  SPLIT_TRACE_START_LINE="$(trace_line_count)"
  log "split_keybind=ctrl+d=new_split:right"
  swift "$ROOT/scripts/ghostty-app/inject.swift" key 2 control >>"$HARNESS_LOG" 2>&1
  delay 1

  SPLIT_PRESENT_LINE="$(wait_for_split_right_frame_after "$SPLIT_START_LINE" "$PANE_ID" "$CONTEXT_ID" "$OVERLAY_FRAME_SIZE" "split-right AppKit overlay frame")"
  SPLIT_PIXELS_LINE="$(wait_for_split_right_pixels_after "$SPLIT_START_LINE" "$PANE_ID" "$CONTEXT_ID" "$APPKIT_PIXEL" "split-right AppKit presented pixels")"
  SPLIT_FRAME_SIZE="$(extract_frame_size "$SPLIT_PRESENT_LINE")"
  SPLIT_FRAME_X="$(extract_frame_x "$SPLIT_PRESENT_LINE")"
  SPLIT_PIXEL="$(extract_appkit_pixel "$SPLIT_PIXELS_LINE")"
  SPLIT_PIXEL_WIDTH="${SPLIT_PIXEL%x*}"
  SPLIT_PIXEL_HEIGHT="${SPLIT_PIXEL#*x}"
  SPLIT_FRAME_WIDTH="$(pair_width "$SPLIT_FRAME_SIZE")"
  INITIAL_FRAME_WIDTH="$(pair_width "$OVERLAY_FRAME_SIZE")"
  log "PASS: observed split-right AppKit overlay frame overlay_frame_size=$SPLIT_FRAME_SIZE"
  log "PASS: observed split-right AppKit presented pixels appkit_pixel=$SPLIT_PIXEL"
  log "split_overlay_frame_size=$SPLIT_FRAME_SIZE"
  log "split_overlay_frame_x=$SPLIT_FRAME_X"
  log "split_appkit_pixel=$SPLIT_PIXEL"
  require_log_after "$SPLIT_START_LINE" "TermSurf geometry layer=zig event=appkit_presented_pixels .*pane_id:${PANE_ID} .*appkit_pixel=${SPLIT_PIXEL}" "Zig records split-right AppKit presented pixel size"
  require_trace_after "$SPLIT_TRACE_START_LINE" "resize tab_id=${BROWSER_TAB_ID} pane_id=${PANE_ID} pixel_width=${SPLIT_PIXEL_WIDTH} pixel_height=${SPLIT_PIXEL_HEIGHT} screen_x=0 screen_y=0 screen_width=0 screen_height=0 screen_scale=0 ffi=ts_set_view_size" "Roamium applied split-right resize to AppKit pixel size via ts_set_view_size"

  SPLIT_WIN_LINE="$(window_bounds)" || fail "failed to resolve split window bounds for window id=$WID"
  IFS=$'\t' read -r _SPLIT_WID SPLIT_WX SPLIT_WY SPLIT_WW SPLIT_WH <<<"$SPLIT_WIN_LINE"
  BROWSER_FOCUS_X="$(awk -v wx="$SPLIT_WX" -v frame_x="$SPLIT_FRAME_X" -v frame_w="$SPLIT_FRAME_WIDTH" 'BEGIN { print int(wx + frame_x + (frame_w / 2) + 0.5) }')"
  BROWSER_FOCUS_Y=$((SPLIT_WY + SPLIT_WH / 2))
  BROWSER_FOCUS_HIT_START_LINE="$(log_line_count)"
  click_global_point "$BROWSER_FOCUS_X" "$BROWSER_FOCUS_Y" "close_browser_focus"
  BROWSER_FOCUS_HIT_LINE="$(wait_for_hit_after "$BROWSER_FOCUS_HIT_START_LINE" "$CONTEXT_ID" "split-right browser-pane focus hit-test")"
  log "PASS: focused split-right browser pane before close"
  require_text "$BROWSER_FOCUS_HIT_LINE" "overlay_frame=" "split-right browser-pane focus hit-test includes current overlay frame"
  require_text "$BROWSER_FOCUS_HIT_LINE" "web_point={" "split-right browser-pane focus hit-test includes webview-relative point"

  CLOSE_START_LINE="$(log_line_count)"
  CLOSE_TRACE_START_LINE="$(trace_line_count)"
  log "close_keybind=ctrl+k=close_surface"
  swift "$ROOT/scripts/ghostty-app/inject.swift" key 40 control >>"$HARNESS_LOG" 2>&1
  delay 1

  CLEAR_OVERLAY_SEEN=""
  for _ in $(seq 1 30); do
    if tail -n +"$((CLOSE_START_LINE + 1))" "$APP_LOG" | grep -E "TermSurf geometry layer=zig event=clear_overlay_call .*pane_id:${PANE_ID}" >/dev/null 2>&1; then
      CLEAR_OVERLAY_SEEN="1"
      break
    fi
    if tail -n +"$((CLOSE_TRACE_START_LINE + 1))" "$ROAMIUM_TRACE" | grep -F "key-event tab=${BROWSER_TAB_ID} pane=${PANE_ID}" >/dev/null 2>&1; then
      fail "Control-K was forwarded to Roamium browser input before close_surface cleanup"
    fi
    delay 1
  done
  [ -n "$CLEAR_OVERLAY_SEEN" ] || fail "timed out waiting for Zig records browser-pane clear_overlay_call after close"
  log "PASS: Zig records browser-pane clear_overlay_call after close"

  wait_for_log_after "$CLOSE_START_LINE" "TermSurf geometry layer=bridge event=clear_request .*pane_id:${PANE_ID}" "Swift bridge records browser-pane clear_request after close"

  CLEAR_RESULT=""
  for _ in $(seq 1 30); do
    if tail -n +"$((CLOSE_START_LINE + 1))" "$APP_LOG" | grep -E "TermSurf geometry layer=bridge event=clear_target_found .*pane_id:${PANE_ID}" >/dev/null 2>&1 &&
      tail -n +"$((CLOSE_START_LINE + 1))" "$APP_LOG" | grep -E "TermSurf geometry layer=appkit event=clear .*pane_id:${PANE_ID}" >/dev/null 2>&1; then
      CLEAR_RESULT="target-found"
      break
    fi
    if tail -n +"$((CLOSE_START_LINE + 1))" "$APP_LOG" | grep -E "TermSurf geometry layer=bridge event=clear_rejected .*pane_id:${PANE_ID} .*note=no-surface" >/dev/null 2>&1; then
      CLEAR_RESULT="surface-already-gone"
      break
    fi
    delay 1
  done
  [ -n "$CLEAR_RESULT" ] || fail "missing AppKit clear or bridge no-surface cleanup evidence after browser-pane close"
  log "PASS: observed browser-pane clear result clear_result=$CLEAR_RESULT"

  require_log_after "$CLOSE_START_LINE" "CloseTab: pane_id=${PANE_ID} tab_id=${BROWSER_TAB_ID}" "Zig records CloseTab for browser pane after close"
  require_trace_after "$CLOSE_TRACE_START_LINE" "close-tab tab_id=${BROWSER_TAB_ID} pane_id=${PANE_ID} result=destroying ffi=ts_destroy_web_contents" "Roamium received CloseTab and destroyed browser tab"
  require_trace_after "$CLOSE_TRACE_START_LINE" "close-tab tab_id=${BROWSER_TAB_ID} result=removed" "Roamium removed closed browser tab"

  screencapture -x -o -l"$WID" "$SCREENSHOT_CLOSE"
  log "close_screenshot_exit=$?"

  FORMER_BROWSER_X="$BROWSER_FOCUS_X"
  FORMER_BROWSER_Y="$BROWSER_FOCUS_Y"
  click_negative_global_point "$FORMER_BROWSER_X" "$FORMER_BROWSER_Y" "former_browser_after_close"
  wait_for_negative_hit_after "$NEGATIVE_HIT_START_LINE" "$CONTEXT_ID" "former browser-pane area after browser close" allow-absent

  REMAINING_SIBLING_X="$(awk -v wx="$SPLIT_WX" -v frame_x="$SPLIT_FRAME_X" -v split_w="$SPLIT_FRAME_WIDTH" -v initial_w="$INITIAL_FRAME_WIDTH" 'BEGIN { print int(wx + frame_x + split_w + ((initial_w - split_w) / 2) + 0.5) }')"
  REMAINING_SIBLING_Y="$BROWSER_FOCUS_Y"
  click_negative_global_point "$REMAINING_SIBLING_X" "$REMAINING_SIBLING_Y" "remaining_sibling_after_close"
  wait_for_negative_hit_after "$NEGATIVE_HIT_START_LINE" "$CONTEXT_ID" "remaining sibling area after browser close" allow-absent

  SIBLING_KEY_START_LINE="$(log_line_count)"
  printf 'ISSUE809_EXP10_SIBLING_ALIVE\n' >"$SIBLING_ALIVE_COMMAND"
  swift "$ROOT/scripts/ghostty-app/inject.swift" type "$SIBLING_ALIVE_COMMAND" >>"$HARNESS_LOG" 2>&1
  SIBLING_KEY_SEEN=""
  for _ in $(seq 1 10); do
    if tail -n +"$((SIBLING_KEY_START_LINE + 1))" "$APP_LOG" | grep -E "TermSurf geometry layer=appkit event=key_down .*overlay_frame=none .*visible=false .*focused=true" >/dev/null 2>&1; then
      SIBLING_KEY_SEEN="1"
      break
    fi
    delay 1
  done
  [ -n "$SIBLING_KEY_SEEN" ] || fail "remaining sibling pane did not receive post-close keyboard events"
  log "PASS: remaining sibling pane received post-close keyboard events"
fi

if [ "$SCENARIO" = "split-down" ]; then
  SPLIT_START_LINE="$(log_line_count)"
  SPLIT_TRACE_START_LINE="$(trace_line_count)"
  log "split_keybind=ctrl+j=new_split:down"
  swift "$ROOT/scripts/ghostty-app/inject.swift" key 38 control >>"$HARNESS_LOG" 2>&1
  delay 1

  SPLIT_PRESENT_LINE="$(wait_for_split_down_frame_after "$SPLIT_START_LINE" "$PANE_ID" "$CONTEXT_ID" "$OVERLAY_FRAME_SIZE" "split-down AppKit overlay frame")"
  SPLIT_PIXELS_LINE="$(wait_for_split_down_pixels_after "$SPLIT_START_LINE" "$PANE_ID" "$CONTEXT_ID" "$APPKIT_PIXEL" "split-down AppKit presented pixels")"
  SPLIT_FRAME_SIZE="$(extract_frame_size "$SPLIT_PRESENT_LINE")"
  SPLIT_FRAME_X="$(extract_frame_x "$SPLIT_PRESENT_LINE")"
  SPLIT_FRAME_Y="$(extract_frame_y "$SPLIT_PRESENT_LINE")"
  SPLIT_PIXEL="$(extract_appkit_pixel "$SPLIT_PIXELS_LINE")"
  SPLIT_PIXEL_WIDTH="${SPLIT_PIXEL%x*}"
  SPLIT_PIXEL_HEIGHT="${SPLIT_PIXEL#*x}"
  log "PASS: observed split-down AppKit overlay frame overlay_frame_size=$SPLIT_FRAME_SIZE"
  log "PASS: observed split-down AppKit presented pixels appkit_pixel=$SPLIT_PIXEL"
  log "split_overlay_frame_size=$SPLIT_FRAME_SIZE"
  log "split_overlay_frame_x=$SPLIT_FRAME_X"
  log "split_overlay_frame_y=$SPLIT_FRAME_Y"
  log "split_appkit_pixel=$SPLIT_PIXEL"
  require_log_after "$SPLIT_START_LINE" "TermSurf geometry layer=zig event=appkit_presented_pixels .*pane_id:${PANE_ID} .*appkit_pixel=${SPLIT_PIXEL}" "Zig records split-down AppKit presented pixel size"
  require_trace_after "$SPLIT_TRACE_START_LINE" "resize tab_id=${BROWSER_TAB_ID} pane_id=${PANE_ID} pixel_width=${SPLIT_PIXEL_WIDTH} pixel_height=${SPLIT_PIXEL_HEIGHT} screen_x=0 screen_y=0 screen_width=0 screen_height=0 screen_scale=0 ffi=ts_set_view_size" "Roamium applied split-down resize to AppKit pixel size via ts_set_view_size"
  screencapture -x -o -l"$WID" "$SCREENSHOT_SPLIT"
  log "split_screenshot_exit=$?"

  SPLIT_WIN_LINE="$(window_bounds)" || fail "failed to resolve split window bounds for window id=$WID"
  IFS=$'\t' read -r _SPLIT_WID SPLIT_WX SPLIT_WY SPLIT_WW SPLIT_WH <<<"$SPLIT_WIN_LINE"
  SPLIT_FRAME_WIDTH="$(pair_width "$SPLIT_FRAME_SIZE")"
  SPLIT_FRAME_HEIGHT="$(pair_height "$SPLIT_FRAME_SIZE")"
  INITIAL_FRAME_HEIGHT="$(pair_height "$OVERLAY_FRAME_SIZE")"
  SPLIT_INSIDE_X="$(awk -v wx="$SPLIT_WX" -v frame_x="$SPLIT_FRAME_X" -v frame_w="$SPLIT_FRAME_WIDTH" 'BEGIN { print int(wx + frame_x + (frame_w / 2) + 0.5) }')"
  SPLIT_INSIDE_Y=$((SPLIT_WY + 150))
  SPLIT_HIT_START_LINE="$(log_line_count)"
  click_global_point "$SPLIT_INSIDE_X" "$SPLIT_INSIDE_Y" "split_inside"
  SPLIT_HIT_LINE="$(wait_for_hit_after "$SPLIT_HIT_START_LINE" "$CONTEXT_ID" "split-down AppKit hit-test")"
  log "PASS: observed split-down AppKit hit-test"
  require_text "$SPLIT_HIT_LINE" "overlay_frame=" "split-down hit-test includes current overlay frame"
  require_text "$SPLIT_HIT_LINE" "web_point={" "split-down hit-test includes webview-relative point"

  SPLIT_NEGATIVE_X="$SPLIT_INSIDE_X"
  SPLIT_NEGATIVE_Y=$((SPLIT_WY + 285))
  click_negative_global_point "$SPLIT_NEGATIVE_X" "$SPLIT_NEGATIVE_Y" "split_sibling_negative"
  wait_for_negative_hit_after "$NEGATIVE_HIT_START_LINE" "$CONTEXT_ID" "split-down sibling-pane negative hit-test"
fi

log "PASS: scenario $SCENARIO"
