#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RUN_ID="$(date +%Y%m%d-%H%M%S)"
LOG_DIR="$ROOT/logs/issue-834-exp47-surfari-pdf-copy-target"
EXP44_LOG_DIR="$ROOT/logs/issue-834-exp44-surfari-pdf-selection-copy"
SUMMARY="$LOG_DIR/surfari-pdf-copy-target-summary.json"
HARNESS_LOG="$LOG_DIR/harness-$RUN_ID.log"
ORIGINAL_CLIPBOARD="$LOG_DIR/original-clipboard-$RUN_ID.txt"
ORIGINAL_RESTORE_STATUS="not-attempted"

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

run_probe() {
  local name="$1"
  local inprocess="$2"
  local trace_file="$LOG_DIR/$name-copy-trace-$RUN_ID.log"
  local summary_file="$LOG_DIR/$name-exp44-summary-$RUN_ID.json"

  rm -rf "$EXP44_LOG_DIR"
  log "probe=$name inprocess=$inprocess trace=$trace_file"
  if [ "$inprocess" = "yes" ]; then
    if TERMSURF_SURFARI_PDF_COPY_TRACE=1 \
      TERMSURF_SURFARI_PDF_COPY_TRACE_FILE="$trace_file" \
      TERMSURF_SURFARI_PDF_COPY_INPROCESS=1 \
      env -u TERMSURF_SURFARI_CACONTEXT_LAYER \
      "$ROOT/scripts/test-issue-834-surfari-pdf-selection-copy.sh" >>"$HARNESS_LOG" 2>&1; then
      :
    fi
  else
    if TERMSURF_SURFARI_PDF_COPY_TRACE=1 \
      TERMSURF_SURFARI_PDF_COPY_TRACE_FILE="$trace_file" \
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
  python3 - "$SUMMARY" "$RUN_ID" "$ORIGINAL_RESTORE_STATUS" "$LOG_DIR" <<'PY'
import json
import sys
from pathlib import Path

summary = Path(sys.argv[1])
run_id = sys.argv[2]
restore_status = sys.argv[3]
log_dir = Path(sys.argv[4])

def read_json(path):
    return json.loads(path.read_text()) if path.exists() else {"overall_result": "missing"}

baseline = read_json(log_dir / f"baseline-exp44-summary-{run_id}.json")
inprocess = read_json(log_dir / f"inprocess-exp44-summary-{run_id}.json")
baseline_trace = log_dir / f"baseline-copy-trace-{run_id}.log"
inprocess_trace = log_dir / f"inprocess-copy-trace-{run_id}.log"
baseline_trace_text = baseline_trace.read_text(errors="replace") if baseline_trace.exists() else ""
inprocess_trace_text = inprocess_trace.read_text(errors="replace") if inprocess_trace.exists() else ""

baseline_pass = baseline.get("overall_result") == "pass"
inprocess_pass = inprocess.get("overall_result") == "pass"
has_target = "target_nil=" in baseline_trace_text and "target_webview=" in baseline_trace_text
has_inprocess = "surfari-pdf-copy-inprocess" in inprocess_trace_text
has_marker_after_inprocess = bool(
    inprocess
    .get("clipboard", {})
    .get("contains_accepted_substring", False)
)
has_marker_in_inprocess_trace_clipboard = "clipboard={len=16" in inprocess_trace_text and "sample=TS834PDFCOPYQXJZ" in inprocess_trace_text

if baseline_pass:
    classification = "external-copy-baseline-pass"
    result = "pass"
elif inprocess_pass or has_marker_after_inprocess or has_marker_in_inprocess_trace_clipboard:
    classification = "inprocess-copy-succeeds"
    result = "pass"
elif has_target and has_inprocess:
    classification = "trace-insufficient"
    result = "partial"
else:
    classification = "trace-insufficient"
    result = "partial"

if restore_status != "restored":
    result = "fail"
if baseline.get("overall_result") == "missing":
    result = "fail"

data = {
    "overall_result": result,
    "classification": classification,
    "run_id": run_id,
    "clipboard_restore_status": restore_status,
    "baseline": baseline,
    "inprocess": inprocess,
    "artifacts": {
        "baseline_summary": str(log_dir / f"baseline-exp44-summary-{run_id}.json"),
        "baseline_copy_trace": str(baseline_trace),
        "inprocess_summary": str(log_dir / f"inprocess-exp44-summary-{run_id}.json"),
        "inprocess_copy_trace": str(inprocess_trace),
    },
    "trace_evidence": {
        "baseline_has_copy_target": has_target,
        "inprocess_has_probe": has_inprocess,
        "inprocess_summary_clipboard_contains_marker": has_marker_after_inprocess,
        "inprocess_trace_clipboard_contains_marker": has_marker_in_inprocess_trace_clipboard,
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

run_probe baseline no

baseline_result="$(python3 - "$LOG_DIR/baseline-exp44-summary-$RUN_ID.json" <<'PY'
import json
import sys
from pathlib import Path
data = json.loads(Path(sys.argv[1]).read_text())
print(data.get("overall_result", "missing"))
PY
)"

if [ "$baseline_result" != "pass" ]; then
  run_probe inprocess yes
else
  python3 - "$LOG_DIR/inprocess-exp44-summary-$RUN_ID.json" <<'PY'
import json
import sys
from pathlib import Path
Path(sys.argv[1]).write_text(json.dumps({"overall_result": "skipped"}, indent=2) + "\n")
PY
  : >"$LOG_DIR/inprocess-copy-trace-$RUN_ID.log"
fi

restore_original_clipboard || ORIGINAL_RESTORE_STATUS="restore-failed"
classify
log "summary=$SUMMARY"
