#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RUN_ID="$(date +%Y%m%d-%H%M%S)"
LOG_DIR="$ROOT/logs/issue-756-exp26-surfari-pane-split"
RUN_DIR="$(mktemp -d "${TMPDIR:-/tmp}/termsurf-issue756-exp26.XXXXXX")"
APP="${TERMSURF_GHOSTBOARD_APP:-$ROOT/ghostboard/macos/build/Debug/TermSurf.app}"
APP_BIN="$APP/Contents/MacOS/termsurf"
WEB="${TERMSURF_WEB:-$ROOT/target/debug/web}"
SURFARI="${TERMSURF_SURFARI:-$ROOT/target/debug/surfari}"
WEBKIT_DEBUG="${TERMSURF_WEBKIT_DEBUG:-$ROOT/webkit/src/WebKitBuild/Debug}"
SITE_DIR="$RUN_DIR/site"
WINDOW_BOUNDS="$RUN_DIR/window-bounds.swift"
HARNESS_LOG="$LOG_DIR/harness-$RUN_ID.log"
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

require_log_after() {
  local app_log="$1"
  local start_line="$2"
  local pattern="$3"
  local label="$4"
  if tail -n +"$((start_line + 1))" "$app_log" | grep -E "$pattern" >/dev/null 2>&1; then
    log "PASS: $label"
  else
    fail "missing $label"
  fi
}

require_trace_after() {
  local trace="$1"
  local start_line="$2"
  local needle="$3"
  local label="$4"
  if tail -n +"$((start_line + 1))" "$trace" | grep -F "$needle" >/dev/null 2>&1; then
    log "PASS: $label"
  else
    fail "missing $label"
  fi
}

extract_window_id() {
  printf '%s\n' "$1" | sed -E 's/.*identity=window_id:([0-9]+).*/\1/'
}

extract_context_id() {
  printf '%s\n' "$1" | sed -E 's/.*context_id=([^ ]+).*/\1/'
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

extract_top_point() {
  printf '%s\n' "$1" | sed -E 's/.*top_point=\{([^,]+), ([^}]+)\}.*/\1,\2/'
}

point_y() {
  printf '%s\n' "$1" | awk -F, '{print $2}'
}

extract_appkit_pixel() {
  printf '%s\n' "$1" | sed -E 's/.*appkit_pixel=([^ ]+).*/\1/'
}

pair_width() {
  printf '%s\n' "$1" | awk -Fx '{print $1}'
}

pair_height() {
  printf '%s\n' "$1" | awk -Fx '{print $2}'
}

compare_split_right_pair() {
  local pair="$1"
  local ref="$2"
  local tolerance="$3"
  awk -v pair="$pair" -v ref="$ref" -v tolerance="$tolerance" 'BEGIN {
    split(pair, p, "x"); split(ref, r, "x")
    delta = p[2] - r[2]; if (delta < 0) delta = -delta
    exit !((p[1] < r[1]) && (delta <= tolerance))
  }'
}

compare_split_down_pair() {
  local pair="$1"
  local ref="$2"
  local tolerance="$3"
  awk -v pair="$pair" -v ref="$ref" -v tolerance="$tolerance" 'BEGIN {
    split(pair, p, "x"); split(ref, r, "x")
    delta = p[1] - r[1]; if (delta < 0) delta = -delta
    exit !((p[2] < r[2]) && (delta <= tolerance))
  }'
}

compare_split_right_resize_pair() {
  local pair="$1"
  local ref="$2"
  local tolerance="$3"
  awk -v pair="$pair" -v ref="$ref" -v tolerance="$tolerance" 'BEGIN {
    split(pair, p, "x"); split(ref, r, "x")
    delta = p[2] - r[2]; if (delta < 0) delta = -delta
    exit !((p[1] > r[1]) && (delta <= tolerance))
  }'
}

wait_for_split_frame_after() {
  local app_log="$1"
  local mode="$2"
  local start_line="$3"
  local pane_id="$4"
  local context_id="$5"
  local ref_frame="$6"
  local label="$7"
  local line frame_size
  for _ in $(seq 1 45); do
    while IFS= read -r line; do
      frame_size="$(extract_frame_size "$line")"
      case "$mode" in
        right)
          compare_split_right_pair "$frame_size" "$ref_frame" 8 && {
            printf '%s\n' "$line"
            return 0
          }
          ;;
        down)
          compare_split_down_pair "$frame_size" "$ref_frame" 8 && {
            printf '%s\n' "$line"
            return 0
          }
          ;;
        right-resize)
          compare_split_right_resize_pair "$frame_size" "$ref_frame" 8 && {
            printf '%s\n' "$line"
            return 0
          }
          ;;
      esac
    done < <(tail -n +"$((start_line + 1))" "$app_log" | grep -E "TermSurf geometry layer=appkit event=presented .*pane_id:${pane_id} .*context_id=${context_id}" || true)
    delay 1
  done
  fail "timed out waiting for $label"
}

wait_for_split_pixels_after() {
  local app_log="$1"
  local mode="$2"
  local start_line="$3"
  local pane_id="$4"
  local context_id="$5"
  local ref_pixel="$6"
  local label="$7"
  local line pixel
  for _ in $(seq 1 45); do
    while IFS= read -r line; do
      pixel="$(extract_appkit_pixel "$line")"
      case "$mode" in
        right)
          compare_split_right_pair "$pixel" "$ref_pixel" 16 && {
            printf '%s\n' "$line"
            return 0
          }
          ;;
        down)
          compare_split_down_pair "$pixel" "$ref_pixel" 16 && {
            printf '%s\n' "$line"
            return 0
          }
          ;;
        right-resize)
          compare_split_right_resize_pair "$pixel" "$ref_pixel" 16 && {
            printf '%s\n' "$line"
            return 0
          }
          ;;
      esac
    done < <(tail -n +"$((start_line + 1))" "$app_log" | grep -E "TermSurf geometry layer=appkit event=presented_pixels .*pane_id:${pane_id} .*context_id=${context_id}" || true)
    delay 1
  done
  fail "timed out waiting for $label"
}

wait_for_hit_after() {
  local app_log="$1"
  local start_line="$2"
  local context_id="$3"
  local label="$4"
  local line
  for _ in $(seq 1 30); do
    line="$(tail -n +"$((start_line + 1))" "$app_log" | grep -E "TermSurf geometry layer=appkit event=hit_test .*context_id=${context_id} .*hit=true .*web_point=\\{" | tail -1 || true)"
    if [ -n "$line" ]; then
      printf '%s\n' "$line"
      return 0
    fi
    delay 1
  done
  fail "timed out waiting for $label"
}

wait_for_negative_hit_after() {
  local app_log="$1"
  local start_line="$2"
  local context_id="$3"
  local label="$4"
  for _ in $(seq 1 10); do
    if tail -n +"$((start_line + 1))" "$app_log" | grep -E "TermSurf geometry layer=appkit event=hit_test .*context_id=${context_id} .*hit=true" >/dev/null 2>&1; then
      fail "$label routed to original Surfari context"
    fi
    if tail -n +"$((start_line + 1))" "$app_log" | grep -E "TermSurf geometry layer=appkit event=hit_test .*context_id=${context_id} .*hit=false" >/dev/null 2>&1; then
      log "PASS: observed $label with explicit hit=false"
      return 0
    fi
    delay 1
  done
  log "PASS: $label did not route to original Surfari context"
}

window_bounds() {
  swift "$WINDOW_BOUNDS" "$1"
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

click_negative_global_point() {
  local app_log="$1"
  local x="$2"
  local y="$3"
  local label="$4"
  log "${label}_input_point=${x},${y}"
  pointer_move "$x" "$y"
  delay 0.75
  NEGATIVE_HIT_START_LINE="$(line_count "$app_log")"
  pointer_click "$x" "$y"
}

assert_hit_inside_frame() {
  local app_log="$1"
  local window_id="$2"
  local present_line="$3"
  local context_id="$4"
  local label="$5"
  local win_line wx wy ww wh root_size root_height content_y frame_x frame_y frame_size frame_width frame_height click_x click_y hit_start hit_line

  win_line="$(window_bounds "$window_id")" || fail "failed to resolve window bounds for $label window_id=$window_id"
  IFS=$'\t' read -r _wid wx wy ww wh <<<"$win_line"
  root_size="$(extract_root_frame_size "$present_line")"
  root_height="$(pair_height "$root_size")"
  content_y="$(awk -v wh="$wh" -v root_h="$root_height" 'BEGIN { print int(wh - root_h) }')"
  frame_x="$(extract_frame_x "$present_line")"
  frame_y="$(extract_frame_y "$present_line")"
  frame_size="$(extract_frame_size "$present_line")"
  frame_width="$(pair_width "$frame_size")"
  frame_height="$(pair_height "$frame_size")"
  click_x="$(awk -v wx="$wx" -v fx="$frame_x" -v fw="$frame_width" 'BEGIN { print int(wx + fx + (fw / 2) + 0.5) }')"
  click_y="$(awk -v wy="$wy" -v cy="$content_y" -v fy="$frame_y" -v fh="$frame_height" 'BEGIN { print int(wy + cy + fy + (fh / 2) + 0.5) }')"
  hit_start="$(line_count "$app_log")"
  click_global_point "$click_x" "$click_y" "$label"
  hit_line="$(wait_for_hit_after "$app_log" "$hit_start" "$context_id" "$label AppKit hit-test")"
  case "$hit_line" in
    *"overlay_frame=$(extract_overlay_frame "$present_line")"*|*"web_point={"*) ;;
    *) fail "$label hit-test did not include overlay frame or web point: $hit_line" ;;
  esac
  log "PASS: $label hit-test reached Surfari context"
}

assert_split_right_negative() {
  local app_log="$1"
  local window_id="$2"
  local present_line="$3"
  local baseline_frame_size="$4"
  local context_id="$5"
  local label="$6"
  local win_line wx wy ww wh frame_x frame_size frame_width initial_width click_x click_y

  win_line="$(window_bounds "$window_id")" || fail "failed to resolve window bounds for $label"
  IFS=$'\t' read -r _wid wx wy ww wh <<<"$win_line"
  frame_x="$(extract_frame_x "$present_line")"
  frame_size="$(extract_frame_size "$present_line")"
  frame_width="$(pair_width "$frame_size")"
  initial_width="$(pair_width "$baseline_frame_size")"
  click_x="$(awk -v wx="$wx" -v fx="$frame_x" -v fw="$frame_width" -v iw="$initial_width" 'BEGIN { print int(wx + fx + fw + ((iw - fw) / 2) + 0.5) }')"
  click_y=$((wy + wh / 2))
  click_negative_global_point "$app_log" "$click_x" "$click_y" "$label"
  wait_for_negative_hit_after "$app_log" "$NEGATIVE_HIT_START_LINE" "$context_id" "$label sibling-pane negative hit-test"
}

assert_split_down_negative() {
  local app_log="$1"
  local window_id="$2"
  local present_line="$3"
  local context_id="$4"
  local label="$5"
  local win_line wx wy ww wh frame_x frame_size frame_width inside_x inside_y hit_start hit_line top_point top_y root_size root_height top_global_offset negative_y

  win_line="$(window_bounds "$window_id")" || fail "failed to resolve window bounds for $label"
  IFS=$'\t' read -r _wid wx wy ww wh <<<"$win_line"
  frame_x="$(extract_frame_x "$present_line")"
  frame_size="$(extract_frame_size "$present_line")"
  frame_width="$(pair_width "$frame_size")"
  inside_x="$(awk -v wx="$wx" -v fx="$frame_x" -v fw="$frame_width" 'BEGIN { print int(wx + fx + (fw / 2) + 0.5) }')"
  inside_y=$((wy + 150))
  hit_start="$(line_count "$app_log")"
  click_global_point "$inside_x" "$inside_y" "${label}_inside_probe"
  hit_line="$(wait_for_hit_after "$app_log" "$hit_start" "$context_id" "$label inside probe")"
  log "PASS: $label inside probe reached Surfari context"
  top_point="$(extract_top_point "$hit_line")"
  top_y="$(point_y "$top_point")"
  [ -n "$top_y" ] && [ "$top_y" != "$top_point" ] || fail "$label hit-test missing top_point y"
  root_size="$(extract_root_frame_size "$present_line")"
  root_height="$(pair_height "$root_size")"
  top_global_offset="$(awk -v global_y="$inside_y" -v top_y="$top_y" 'BEGIN { print int(global_y - top_y) }')"
  negative_y="$(awk -v offset="$top_global_offset" -v root_h="$root_height" 'BEGIN { print int(offset + root_h + 24) }')"
  click_negative_global_point "$app_log" "$inside_x" "$negative_y" "$label"
  wait_for_negative_hit_after "$app_log" "$NEGATIVE_HIT_START_LINE" "$context_id" "$label sibling-pane negative hit-test"
}

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

cat >"$SITE_DIR/index.html" <<'EOF'
<!doctype html>
<meta charset="utf-8">
<title>Issue 756 Surfari Split Geometry</title>
<main style="min-height:1600px;font:18px system-ui,sans-serif">
  <h1>ISSUE756_EXP26_SURFARI_SPLIT_GEOMETRY</h1>
  <input id="field" value="ready">
  <script>
    document.getElementById("field").focus();
    console.log("ISSUE756_EXP26_READY");
  </script>
</main>
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
log "webkit_debug=$WEBKIT_DEBUG"

run_scenario() {
  local scenario="$1"
  local config="$RUN_DIR/config-$scenario"
  local command="$RUN_DIR/run-$scenario.sh"
  local app_log="$LOG_DIR/app-$scenario-$RUN_ID.log"
  local surfari_trace="$LOG_DIR/surfari-$scenario-$RUN_ID.log"
  local webtui_trace="$LOG_DIR/webtui-$scenario-$RUN_ID.log"
  local url="file://$SITE_DIR/index.html"
  local app_start trace_start presented_line pixels_line browser_ready_line pane_id browser_tab_id window_id context_id baseline_frame baseline_pixel

  cat >"$command" <<EOF
#!/usr/bin/env bash
exec "$WEB" --browser surfari "$url"
EOF
  chmod +x "$command"

  cat >"$config" <<EOF
window-save-state = never
initial-command = direct:$command
keybind = ctrl+d=new_split:right
keybind = ctrl+j=new_split:down
keybind = ctrl+l=resize_split:right,20
EOF

  log "scenario=$scenario"
  log "scenario_app_log=$app_log"
  log "scenario_surfari_trace=$surfari_trace"

  GHOSTTY_CONFIG_PATH="$config" \
  GHOSTTY_LOG=stderr \
  DYLD_FRAMEWORK_PATH="$WEBKIT_DEBUG" \
  TERMSURF_SURFARI_PATH="$SURFARI" \
  TERMSURF_GEOMETRY_TRACE=1 \
  TERMSURF_GEOMETRY_SCENARIO="issue756-exp26-$scenario" \
  TERMSURF_WEBTUI_STATE_TRACE_FILE="$webtui_trace" \
  TERMSURF_INPUT_TRACE=1 \
  TERMSURF_PDF_INPUT_TRACE=1 \
  TERMSURF_PDF_INPUT_TRACE_FILE="$surfari_trace" \
    "$APP_BIN" >"$app_log" 2>&1 &
  PID="$!"
  log "scenario_pid=$PID"

  app_start="$(line_count "$app_log")"
  trace_start="$(line_count "$surfari_trace")"
  wait_for_file_pattern_after "$app_log" "$app_start" "BrowserReady: pane_id=.* browser=surfari" "$scenario BrowserReady"
  wait_for_file_pattern_after "$app_log" "$app_start" "TermSurf geometry layer=appkit event=presented " "$scenario AppKit presented overlay"
  wait_for_file_pattern_after "$surfari_trace" "$trace_start" "create-tab pane=.* url=${url}" "$scenario Surfari created tab"
  wait_for_file_pattern_after "$surfari_trace" "$trace_start" "title-changed tab=.*title=Issue 756 Surfari Split Geometry" "$scenario Surfari loaded title"

  browser_ready_line="$(tail -n +"$((app_start + 1))" "$app_log" | grep -E "BrowserReady: pane_id=.* browser=surfari" | tail -1)"
  pane_id="$(printf '%s\n' "$browser_ready_line" | sed -E 's/.*pane_id=([^ ]+) tab_id=.*/\1/')"
  browser_tab_id="$(printf '%s\n' "$browser_ready_line" | sed -E 's/.*tab_id=([0-9]+) socket=.*/\1/')"
  presented_line="$(wait_for_line_after "$app_log" "$app_start" "TermSurf geometry layer=appkit event=presented .*pane_id:${pane_id}" "$scenario initial AppKit line")"
  pixels_line="$(wait_for_line_after "$app_log" "$app_start" "TermSurf geometry layer=appkit event=presented_pixels .*pane_id:${pane_id}" "$scenario initial AppKit pixels")"
  window_id="$(extract_window_id "$presented_line")"
  context_id="$(extract_context_id "$presented_line")"
  baseline_frame="$(extract_frame_size "$presented_line")"
  baseline_pixel="$(extract_appkit_pixel "$pixels_line")"
  log "${scenario}_pane_id=$pane_id"
  log "${scenario}_browser_tab_id=$browser_tab_id"
  log "${scenario}_window_id=$window_id"
  log "${scenario}_context_id=$context_id"
  log "${scenario}_baseline_frame=$baseline_frame"
  log "${scenario}_baseline_pixel=$baseline_pixel"

  case "$scenario" in
    split-right)
      run_split_right_assertions "$app_log" "$surfari_trace" "$pane_id" "$browser_tab_id" "$window_id" "$context_id" "$baseline_frame" "$baseline_pixel"
      ;;
    split-down)
      run_split_down_assertions "$app_log" "$surfari_trace" "$pane_id" "$browser_tab_id" "$window_id" "$context_id" "$baseline_frame" "$baseline_pixel"
      ;;
    split-right-resize)
      run_split_right_resize_assertions "$app_log" "$surfari_trace" "$pane_id" "$browser_tab_id" "$window_id" "$context_id" "$baseline_frame" "$baseline_pixel"
      ;;
    *)
      fail "unsupported scenario: $scenario"
      ;;
  esac

  kill "$PID" >/dev/null 2>&1 || true
  delay 0.5
  kill -9 "$PID" >/dev/null 2>&1 || true
  PID=""
  log "PASS: scenario $scenario"
}

run_split_right_assertions() {
  local app_log="$1" trace="$2" pane_id="$3" tab_id="$4" window_id="$5" context_id="$6" baseline_frame="$7" baseline_pixel="$8"
  local split_start split_trace_start split_present split_pixels split_frame split_pixel split_width split_height
  split_start="$(line_count "$app_log")"
  split_trace_start="$(line_count "$trace")"
  log "split_keybind=ctrl+d=new_split:right"
  swift "$ROOT/scripts/ghostty-app/inject.swift" key 2 control >>"$HARNESS_LOG" 2>&1
  delay 1
  require_log_after "$app_log" "$split_start" "dispatching action target=surface action=.new_split" "split-right action dispatched"
  split_present="$(wait_for_split_frame_after "$app_log" right "$split_start" "$pane_id" "$context_id" "$baseline_frame" "split-right AppKit overlay frame")"
  split_pixels="$(wait_for_split_pixels_after "$app_log" right "$split_start" "$pane_id" "$context_id" "$baseline_pixel" "split-right AppKit pixels")"
  split_frame="$(extract_frame_size "$split_present")"
  split_pixel="$(extract_appkit_pixel "$split_pixels")"
  split_width="${split_pixel%x*}"
  split_height="${split_pixel#*x}"
  log "split_right_frame=$split_frame"
  log "split_right_pixel=$split_pixel"
  require_log_after "$app_log" "$split_start" "TermSurf geometry layer=zig event=appkit_presented_pixels .*pane_id:${pane_id} .*appkit_pixel=${split_pixel}" "Zig records split-right AppKit pixels"
  require_trace_after "$trace" "$split_trace_start" "resize tab_id=${tab_id} pane_id=${pane_id} pixel_width=${split_width} pixel_height=${split_height} screen_x=0 screen_y=0 screen_width=0 screen_height=0 screen_scale=0 ffi=ts_set_view_size" "Surfari applied split-right resize"
  assert_hit_inside_frame "$app_log" "$window_id" "$split_present" "$context_id" "split_right_inside"
  assert_split_right_negative "$app_log" "$window_id" "$split_present" "$baseline_frame" "$context_id" "split_right_sibling"
}

run_split_down_assertions() {
  local app_log="$1" trace="$2" pane_id="$3" tab_id="$4" window_id="$5" context_id="$6" baseline_frame="$7" baseline_pixel="$8"
  local split_start split_trace_start split_present split_pixels split_frame split_pixel split_width split_height
  split_start="$(line_count "$app_log")"
  split_trace_start="$(line_count "$trace")"
  log "split_keybind=ctrl+j=new_split:down"
  swift "$ROOT/scripts/ghostty-app/inject.swift" key 38 control >>"$HARNESS_LOG" 2>&1
  delay 1
  require_log_after "$app_log" "$split_start" "dispatching action target=surface action=.new_split" "split-down action dispatched"
  split_present="$(wait_for_split_frame_after "$app_log" down "$split_start" "$pane_id" "$context_id" "$baseline_frame" "split-down AppKit overlay frame")"
  split_pixels="$(wait_for_split_pixels_after "$app_log" down "$split_start" "$pane_id" "$context_id" "$baseline_pixel" "split-down AppKit pixels")"
  split_frame="$(extract_frame_size "$split_present")"
  split_pixel="$(extract_appkit_pixel "$split_pixels")"
  split_width="${split_pixel%x*}"
  split_height="${split_pixel#*x}"
  log "split_down_frame=$split_frame"
  log "split_down_pixel=$split_pixel"
  require_log_after "$app_log" "$split_start" "TermSurf geometry layer=zig event=appkit_presented_pixels .*pane_id:${pane_id} .*appkit_pixel=${split_pixel}" "Zig records split-down AppKit pixels"
  require_trace_after "$trace" "$split_trace_start" "resize tab_id=${tab_id} pane_id=${pane_id} pixel_width=${split_width} pixel_height=${split_height} screen_x=0 screen_y=0 screen_width=0 screen_height=0 screen_scale=0 ffi=ts_set_view_size" "Surfari applied split-down resize"
  assert_split_down_negative "$app_log" "$window_id" "$split_present" "$context_id" "split_down_sibling"
}

run_split_right_resize_assertions() {
  local app_log="$1" trace="$2" pane_id="$3" tab_id="$4" window_id="$5" context_id="$6" baseline_frame="$7" baseline_pixel="$8"
  local split_start split_trace_start split_present split_pixels split_frame split_pixel split_width split_height divider_start divider_trace_start divider_present divider_pixels divider_frame divider_pixel divider_width divider_height
  split_start="$(line_count "$app_log")"
  split_trace_start="$(line_count "$trace")"
  log "split_keybind=ctrl+d=new_split:right"
  swift "$ROOT/scripts/ghostty-app/inject.swift" key 2 control >>"$HARNESS_LOG" 2>&1
  delay 1
  require_log_after "$app_log" "$split_start" "dispatching action target=surface action=.new_split" "split-right resize initial split action dispatched"
  split_present="$(wait_for_split_frame_after "$app_log" right "$split_start" "$pane_id" "$context_id" "$baseline_frame" "split-right resize initial AppKit overlay frame")"
  split_pixels="$(wait_for_split_pixels_after "$app_log" right "$split_start" "$pane_id" "$context_id" "$baseline_pixel" "split-right resize initial AppKit pixels")"
  split_frame="$(extract_frame_size "$split_present")"
  split_pixel="$(extract_appkit_pixel "$split_pixels")"
  split_width="${split_pixel%x*}"
  split_height="${split_pixel#*x}"
  require_trace_after "$trace" "$split_trace_start" "resize tab_id=${tab_id} pane_id=${pane_id} pixel_width=${split_width} pixel_height=${split_height} screen_x=0 screen_y=0 screen_width=0 screen_height=0 screen_scale=0 ffi=ts_set_view_size" "Surfari applied initial split-right resize"

  divider_start="$(line_count "$app_log")"
  divider_trace_start="$(line_count "$trace")"
  log "resize_split_keybind=ctrl+l=resize_split:right,20"
  swift "$ROOT/scripts/ghostty-app/inject.swift" key 37 control >>"$HARNESS_LOG" 2>&1
  delay 1
  require_log_after "$app_log" "$divider_start" "dispatching action target=surface action=.resize_split" "split-right divider resize action dispatched"
  divider_present="$(wait_for_split_frame_after "$app_log" right-resize "$divider_start" "$pane_id" "$context_id" "$split_frame" "split-right divider-resized AppKit overlay frame")"
  divider_pixels="$(wait_for_split_pixels_after "$app_log" right-resize "$divider_start" "$pane_id" "$context_id" "$split_pixel" "split-right divider-resized AppKit pixels")"
  divider_frame="$(extract_frame_size "$divider_present")"
  divider_pixel="$(extract_appkit_pixel "$divider_pixels")"
  divider_width="${divider_pixel%x*}"
  divider_height="${divider_pixel#*x}"
  log "split_right_resized_frame=$divider_frame"
  log "split_right_resized_pixel=$divider_pixel"
  require_log_after "$app_log" "$divider_start" "TermSurf geometry layer=zig event=appkit_presented_pixels .*pane_id:${pane_id} .*appkit_pixel=${divider_pixel}" "Zig records split-right divider AppKit pixels"
  require_trace_after "$trace" "$divider_trace_start" "resize tab_id=${tab_id} pane_id=${pane_id} pixel_width=${divider_width} pixel_height=${divider_height} screen_x=0 screen_y=0 screen_width=0 screen_height=0 screen_scale=0 ffi=ts_set_view_size" "Surfari applied split-right divider resize"
  assert_hit_inside_frame "$app_log" "$window_id" "$divider_present" "$context_id" "split_right_resized_inside"
  assert_split_right_negative "$app_log" "$window_id" "$divider_present" "$baseline_frame" "$context_id" "split_right_resized_sibling"
}

run_scenario split-right
run_scenario split-down
run_scenario split-right-resize

log "PASS: issue 756 Surfari pane/split geometry"
