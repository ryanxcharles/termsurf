#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RUN_ID="$(date +%Y%m%d-%H%M%S)"
LOG_DIR="$ROOT/logs/issue-834-exp48-surfari-direct-copy-fix"
EXP44_LOG_DIR="$ROOT/logs/issue-834-exp44-surfari-pdf-selection-copy"
SUMMARY="$LOG_DIR/surfari-direct-copy-fix-summary.json"
HARNESS_LOG="$LOG_DIR/harness-$RUN_ID.log"
ORIGINAL_CLIPBOARD="$LOG_DIR/original-clipboard-$RUN_ID.txt"
ORIGINAL_RESTORE_STATUS="not-attempted"

START_X_RATIO="${TERMSURF_ISSUE834_EXP48_START_X_RATIO:-0.58}"
END_X_RATIO="${TERMSURF_ISSUE834_EXP48_END_X_RATIO:-0.99}"
Y_RATIO="${TERMSURF_ISSUE834_EXP48_Y_RATIO:-0.43}"

mkdir -p "$LOG_DIR"

log() {
  printf '%s\n' "$*" | tee -a "$HARNESS_LOG"
}

restore_original_clipboard() {
  if [ -e "$ORIGINAL_CLIPBOARD" ]; then
    pbcopy <"$ORIGINAL_CLIPBOARD" || return 1
    ORIGINAL_RESTORE_STATUS="restored"
  fi
}

cleanup() {
  restore_original_clipboard || ORIGINAL_RESTORE_STATUS="restore-failed"
}
trap cleanup EXIT

hash_file() {
  shasum -a 256 "$1" | awk '{print $1}'
}

run_mode() {
  local name="$1"
  local direct="$2"
  local trace_file="$LOG_DIR/$name-copy-trace-$RUN_ID.log"
  local summary_file="$LOG_DIR/$name-exp44-summary-$RUN_ID.json"

  rm -rf "$EXP44_LOG_DIR"
  log "mode=$name direct=$direct trace=$trace_file drag_ratios=$START_X_RATIO,$Y_RATIO-$END_X_RATIO,$Y_RATIO"
  if [ "$direct" = "yes" ]; then
    if TERMSURF_SURFARI_PDF_COPY_TRACE=1 \
      TERMSURF_SURFARI_PDF_COPY_TRACE_FILE="$trace_file" \
      TERMSURF_SURFARI_PDF_COPY_DIRECT=1 \
      TERMSURF_ISSUE834_PDF_COPY_DRAG_START_X_RATIO="$START_X_RATIO" \
      TERMSURF_ISSUE834_PDF_COPY_DRAG_END_X_RATIO="$END_X_RATIO" \
      TERMSURF_ISSUE834_PDF_COPY_DRAG_Y_RATIO="$Y_RATIO" \
      env -u TERMSURF_SURFARI_CACONTEXT_LAYER \
      "$ROOT/scripts/test-issue-834-surfari-pdf-selection-copy.sh" >>"$HARNESS_LOG" 2>&1; then
      :
    fi
  else
    if TERMSURF_SURFARI_PDF_COPY_TRACE=1 \
      TERMSURF_SURFARI_PDF_COPY_TRACE_FILE="$trace_file" \
      TERMSURF_ISSUE834_PDF_COPY_DRAG_START_X_RATIO="$START_X_RATIO" \
      TERMSURF_ISSUE834_PDF_COPY_DRAG_END_X_RATIO="$END_X_RATIO" \
      TERMSURF_ISSUE834_PDF_COPY_DRAG_Y_RATIO="$Y_RATIO" \
      env -u TERMSURF_SURFARI_CACONTEXT_LAYER \
      "$ROOT/scripts/test-issue-834-surfari-pdf-selection-copy.sh" >>"$HARNESS_LOG" 2>&1; then
      :
    fi
  fi

  if [ -f "$EXP44_LOG_DIR/surfari-pdf-selection-copy-summary.json" ]; then
    cp "$EXP44_LOG_DIR/surfari-pdf-selection-copy-summary.json" "$summary_file"
  else
    python3 - "$summary_file" <<'PY'
import json
import sys
from pathlib import Path
Path(sys.argv[1]).write_text(json.dumps({"overall_result": "missing"}, indent=2) + "\n")
PY
  fi
}

classify() {
  python3 - "$SUMMARY" "$RUN_ID" "$ORIGINAL_RESTORE_STATUS" "$LOG_DIR" "$START_X_RATIO" "$END_X_RATIO" "$Y_RATIO" <<'PY'
import json
import sys
from pathlib import Path

summary = Path(sys.argv[1])
run_id = sys.argv[2]
restore_status = sys.argv[3]
log_dir = Path(sys.argv[4])
start_x, end_x, y = sys.argv[5:8]

def read_json(path):
    return json.loads(path.read_text()) if path.exists() else {"overall_result": "missing"}

baseline = read_json(log_dir / f"baseline-exp44-summary-{run_id}.json")
direct = read_json(log_dir / f"direct-exp44-summary-{run_id}.json")
baseline_clip = baseline.get("clipboard", {})
direct_clip = direct.get("clipboard", {})
baseline_pass = baseline.get("overall_result") == "pass"
direct_pass = direct.get("overall_result") == "pass"
direct_changed = direct_clip.get("after_copy_sample") != direct_clip.get("sentinel")
direct_partial = direct_changed and not direct_clip.get("contains_accepted_substring", False)

if baseline_pass:
    classification = "coordinate-fix-only"
    result = "pass"
elif direct_pass:
    classification = "direct-copy-fix-candidate"
    result = "pass"
elif direct_partial:
    classification = "direct-copy-partial-selection"
    result = "pass"
else:
    classification = "direct-copy-no-effect"
    result = "pass"

if restore_status != "restored":
    result = "fail"
if baseline.get("overall_result") == "missing" or direct.get("overall_result") == "missing":
    result = "fail"

data = {
    "overall_result": result,
    "classification": classification,
    "run_id": run_id,
    "clipboard_restore_status": restore_status,
    "drag_ratios": {
        "start_x": float(start_x),
        "end_x": float(end_x),
        "y": float(y),
    },
    "baseline": baseline,
    "direct": direct,
    "artifacts": {
        "baseline_summary": str(log_dir / f"baseline-exp44-summary-{run_id}.json"),
        "baseline_copy_trace": str(log_dir / f"baseline-copy-trace-{run_id}.log"),
        "direct_summary": str(log_dir / f"direct-exp44-summary-{run_id}.json"),
        "direct_copy_trace": str(log_dir / f"direct-copy-trace-{run_id}.log"),
    },
}
summary.write_text(json.dumps(data, indent=2, sort_keys=True) + "\n")
print(json.dumps({
    "overall_result": result,
    "classification": classification,
}, indent=2, sort_keys=True))
if result == "fail":
    sys.exit(1)
PY
}

pbpaste >"$ORIGINAL_CLIPBOARD" || true
log "run_id=$RUN_ID"
log "original_clipboard_length=$(wc -c <"$ORIGINAL_CLIPBOARD" | tr -d ' ')"
log "original_clipboard_sha256=$(hash_file "$ORIGINAL_CLIPBOARD")"

run_mode baseline no
run_mode direct yes

restore_original_clipboard || ORIGINAL_RESTORE_STATUS="restore-failed"
classify
log "summary=$SUMMARY"
