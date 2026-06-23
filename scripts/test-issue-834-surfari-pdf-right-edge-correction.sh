#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RUN_ID="$(date +%Y%m%d-%H%M%S)"
LOG_DIR="$ROOT/logs/issue-834-exp52-surfari-pdf-right-edge-correction"
EXP44_LOG_DIR="$ROOT/logs/issue-834-exp44-surfari-pdf-selection-copy"
SUMMARY="$LOG_DIR/surfari-pdf-right-edge-correction-summary.json"
HARNESS_LOG="$LOG_DIR/harness-$RUN_ID.log"
ORACLE_SUMMARY="${TERMSURF_ISSUE834_EXP50_ORACLE_SUMMARY:-$ROOT/logs/issue-834-exp50-separated-token-copy-oracle/separated-token-copy-oracle-summary.json}"
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

run_cell() {
  local name="$1"
  local mode="$2"
  local delta="$3"
  local start_x="$4"
  local y="$5"
  local end_x="$6"
  local delay_after_drag="$7"
  local out_summary="$LOG_DIR/$name-summary-$RUN_ID.json"
  local out_trace="$LOG_DIR/$name-copy-trace-$RUN_ID.log"

  rm -rf "$EXP44_LOG_DIR"
  log "cell=$name mode=$mode delta=$delta drag=${start_x},${y}-${end_x},${y} delay=$delay_after_drag"

  if [ "$mode" = "none" ]; then
    if TERMSURF_ISSUE834_PDF_FIXTURE_MODE=separated-tokens \
      TERMSURF_ISSUE834_PDF_EXPECTED_TOKENS="LEFT834 MID834 RIGHT834" \
      TERMSURF_ISSUE834_PDF_ACCEPTED_SUBSTRING="RIGHT834" \
      TERMSURF_ISSUE834_PDF_COPY_DRAG_START_X_RATIO="$start_x" \
      TERMSURF_ISSUE834_PDF_COPY_DRAG_END_X_RATIO="$end_x" \
      TERMSURF_ISSUE834_PDF_COPY_DRAG_Y_RATIO="$y" \
      TERMSURF_ISSUE834_PDF_COPY_DELAY_AFTER_DRAG="$delay_after_drag" \
      TERMSURF_SURFARI_PDF_COPY_TRACE=1 \
      TERMSURF_SURFARI_PDF_COPY_TRACE_FILE="$out_trace" \
      TERMSURF_SURFARI_PDF_COPY_DIRECT=1 \
      env -u TERMSURF_SURFARI_CACONTEXT_LAYER \
      "$ROOT/scripts/test-issue-834-surfari-pdf-selection-copy.sh" >>"$HARNESS_LOG" 2>&1; then
      :
    fi
  else
    if TERMSURF_ISSUE834_PDF_FIXTURE_MODE=separated-tokens \
      TERMSURF_ISSUE834_PDF_EXPECTED_TOKENS="LEFT834 MID834 RIGHT834" \
      TERMSURF_ISSUE834_PDF_ACCEPTED_SUBSTRING="RIGHT834" \
      TERMSURF_ISSUE834_PDF_COPY_DRAG_START_X_RATIO="$start_x" \
      TERMSURF_ISSUE834_PDF_COPY_DRAG_END_X_RATIO="$end_x" \
      TERMSURF_ISSUE834_PDF_COPY_DRAG_Y_RATIO="$y" \
      TERMSURF_ISSUE834_PDF_COPY_DELAY_AFTER_DRAG="$delay_after_drag" \
      TERMSURF_SURFARI_PDF_COPY_TRACE=1 \
      TERMSURF_SURFARI_PDF_COPY_TRACE_FILE="$out_trace" \
      TERMSURF_SURFARI_PDF_COPY_DIRECT=1 \
      TERMSURF_SURFARI_PDF_SELECTION_EDGE_PROBE=1 \
      TERMSURF_SURFARI_PDF_SELECTION_EDGE_MODE="$mode" \
      TERMSURF_SURFARI_PDF_SELECTION_EDGE_DELTA_X="$delta" \
      env -u TERMSURF_SURFARI_CACONTEXT_LAYER \
      "$ROOT/scripts/test-issue-834-surfari-pdf-selection-copy.sh" >>"$HARNESS_LOG" 2>&1; then
      :
    fi
  fi

  if [ -f "$EXP44_LOG_DIR/surfari-pdf-selection-copy-summary.json" ]; then
    python3 - "$EXP44_LOG_DIR/surfari-pdf-selection-copy-summary.json" "$out_summary" "$name" "$mode" "$delta" "$start_x" "$y" "$end_x" "$delay_after_drag" "$out_trace" <<'PY'
import json
import sys
from pathlib import Path

source, target, name, mode, delta, start_x, y, end_x, delay, trace = sys.argv[1:11]
data = json.loads(Path(source).read_text())
data["exp52_cell"] = {
    "name": name,
    "correction_mode": mode,
    "delta_x": float(delta),
    "drag_ratios": {
        "start_x": float(start_x),
        "end_x": float(end_x),
        "y": float(y),
    },
    "copy_delay_after_drag_seconds": float(delay),
    "copy_trace": trace,
}
Path(target).write_text(json.dumps(data, indent=2, sort_keys=True) + "\n")
PY
  else
    python3 - "$out_summary" "$name" "$mode" "$delta" "$start_x" "$y" "$end_x" "$delay_after_drag" "$out_trace" <<'PY'
import json
import sys
from pathlib import Path

target, name, mode, delta, start_x, y, end_x, delay, trace = sys.argv[1:10]
Path(target).write_text(json.dumps({
    "overall_result": "missing",
    "classification": "missing-summary",
    "exp52_cell": {
        "name": name,
        "correction_mode": mode,
        "delta_x": float(delta),
        "drag_ratios": {
            "start_x": float(start_x),
            "end_x": float(end_x),
            "y": float(y),
        },
        "copy_delay_after_drag_seconds": float(delay),
        "copy_trace": trace,
    },
}, indent=2, sort_keys=True) + "\n")
PY
  fi
}

classify() {
  python3 - "$SUMMARY" "$RUN_ID" "$ORIGINAL_RESTORE_STATUS" "$LOG_DIR" "$ORACLE_SUMMARY" <<'PY'
import json
import sys
from pathlib import Path

summary = Path(sys.argv[1])
run_id = sys.argv[2]
restore_status = sys.argv[3]
log_dir = Path(sys.argv[4])
oracle_path = Path(sys.argv[5])
expected = ["LEFT834", "MID834", "RIGHT834"]

def load(path):
    return json.loads(path.read_text()) if path.exists() else None

oracle = load(oracle_path)
oracle_gate_open = bool(
    oracle
    and oracle.get("classification") == "separated-token-oracle-pass"
    and oracle.get("embedded_interpretation_gate") == "open"
)

def embedded_fixture_identity(cell):
    fixture = cell.get("fixture", {})
    return {
        "page_geometry": fixture.get("page_geometry"),
        "font": fixture.get("font"),
        "operator_summary": fixture.get("pdf_text_operator"),
        "token_boxes": fixture.get("pdf_text_bboxes"),
        "extracted_text": fixture.get("text_extracted"),
    }

def fixture_matches(oracle_data, cells):
    if not oracle_data or not cells:
        return False, "missing-oracle-or-cells"
    oracle_identity = oracle_data.get("fixture_identity", {})
    expected_operator = oracle_identity.get("operator_summary")
    expected_boxes = oracle_identity.get("token_boxes")
    expected_font = oracle_identity.get("font")
    expected_geometry = oracle_identity.get("page_geometry")
    for cell in cells:
        identity = embedded_fixture_identity(cell)
        if identity["operator_summary"] != expected_operator:
            return False, f"operator-mismatch:{cell.get('path')}"
        if identity["token_boxes"] != expected_boxes:
            return False, f"token-box-mismatch:{cell.get('path')}"
        if identity["font"] != expected_font:
            return False, f"font-mismatch:{cell.get('path')}"
        if identity["page_geometry"] != expected_geometry:
            return False, f"geometry-mismatch:{cell.get('path')}"
        if identity["extracted_text"] != "LEFT834 MID834 RIGHT834":
            return False, f"extracted-text-mismatch:{cell.get('path')}"
    return True, "match"

cells = []
for path in sorted(log_dir.glob(f"*-summary-{run_id}.json")):
    data = load(path) or {}
    clipboard = data.get("clipboard", {})
    primary = clipboard.get("after_copy_sample", "")
    fallback = clipboard.get("fallback_select_all_after_sample", "")
    trace_path = data.get("exp52_cell", {}).get("copy_trace")
    trace = Path(trace_path).read_text(errors="replace") if trace_path and Path(trace_path).exists() else ""
    primary_tokens = [token for token in expected if token in primary]
    fallback_tokens = [token for token in expected if token in fallback]
    adjusted_trace_present = (
        data.get("exp52_cell", {}).get("correction_mode") == "none"
        or "edge_mode=" in trace
        or "surfari-pdf-selection-edge" in trace
    )
    cells.append({
        "path": str(path),
        "cell": data.get("exp52_cell", {}),
        "overall_result": data.get("overall_result"),
        "fixture": data.get("fixture", {}),
        "coordinate_mapping": data.get("coordinate_mapping", {}),
        "primary_sample": primary,
        "fallback_sample": fallback,
        "primary_tokens": primary_tokens,
        "fallback_tokens": fallback_tokens,
        "primary_contains_all_tokens": all(token in primary for token in expected),
        "fallback_contains_all_tokens": all(token in fallback for token in expected),
        "primary_contains_rightmost": "RIGHT834" in primary,
        "adjusted_trace_present": adjusted_trace_present,
    })

missing = [cell for cell in cells if cell.get("overall_result") == "missing"]
fixture_identity_match, fixture_identity_match_reason = fixture_matches(oracle, cells)
baseline = [cell for cell in cells if cell["cell"].get("correction_mode") == "none"]
corrections = [cell for cell in cells if cell["cell"].get("correction_mode") != "none"]
baseline_reproduced = any(
    "LEFT834" in cell["primary_tokens"]
    and "MID834" in cell["primary_tokens"]
    and "RIGHT834" not in cell["primary_tokens"]
    for cell in baseline
)
trace_ok = all(cell["adjusted_trace_present"] for cell in cells)
primary_winners = [cell for cell in corrections if cell["primary_contains_all_tokens"]]
fallback_only = [
    cell for cell in corrections
    if not cell["primary_contains_all_tokens"] and cell["fallback_contains_all_tokens"]
]

if restore_status != "restored" or missing:
    result = "fail" if restore_status != "restored" else "partial"
    classification = "harness-insufficient"
elif not oracle_gate_open or not fixture_identity_match or not baseline_reproduced or not trace_ok:
    result = "partial"
    classification = "harness-insufficient"
elif primary_winners:
    first = primary_winners[0]
    mode = first["cell"].get("correction_mode")
    if mode == "delta":
        classification = "edge-delta-fix-candidate"
    elif mode == "extra-drag":
        classification = "extra-drag-fix-candidate"
    elif mode == "target":
        classification = "target-routing-fix-candidate"
    else:
        classification = "harness-insufficient"
    result = "pass"
elif fallback_only:
    result = "pass"
    classification = "fallback-only-copy"
else:
    result = "pass"
    classification = "right-edge-persists"

summary_data = {
    "overall_result": result,
    "classification": classification,
    "run_id": run_id,
    "oracle_summary": str(oracle_path),
    "oracle_gate_open": oracle_gate_open,
    "fixture_identity_match": fixture_identity_match,
    "fixture_identity_match_reason": fixture_identity_match_reason,
    "clipboard_restore_status": restore_status,
    "baseline_reproduced": baseline_reproduced,
    "correction_trace_ok": trace_ok,
    "expected_tokens": expected,
    "cell_count": len(cells),
    "primary_winners": primary_winners,
    "fallback_only": fallback_only,
    "cells": cells,
}
summary.write_text(json.dumps(summary_data, indent=2, sort_keys=True) + "\n")
print(json.dumps({
    "overall_result": result,
    "classification": classification,
    "baseline_reproduced": baseline_reproduced,
    "fixture_identity_match": fixture_identity_match,
    "primary_winner_count": len(primary_winners),
    "fallback_only_count": len(fallback_only),
}, indent=2, sort_keys=True))
if result == "fail":
    sys.exit(1)
PY
}

pbpaste >"$ORIGINAL_CLIPBOARD" || true
log "run_id=$RUN_ID"
log "oracle_summary=$ORACLE_SUMMARY"

run_cell baseline-none none 0 0.58 0.43 0.99 0.25
run_cell delta-8 delta 8 0.58 0.43 0.99 0.25
run_cell delta-16 delta 16 0.58 0.43 0.99 0.25
run_cell delta-32 delta 32 0.58 0.43 0.99 0.25
run_cell delta-64 delta 64 0.58 0.43 0.99 0.25
run_cell extra-drag-32 extra-drag 32 0.58 0.43 0.99 0.25
run_cell target-0 target 0 0.58 0.43 0.99 0.25

restore_original_clipboard || ORIGINAL_RESTORE_STATUS="restore-failed"
classify
log "summary=$SUMMARY"
