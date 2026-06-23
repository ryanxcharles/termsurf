#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RUN_ID="$(date +%Y%m%d-%H%M%S)"
LOG_DIR="$ROOT/logs/issue-834-exp60-surfari-pdf-action-path-compare"
EXP44_LOG_DIR="$ROOT/logs/issue-834-exp44-surfari-pdf-selection-copy"
CALIBRATION_LOG_DIR="$ROOT/logs/issue-834-exp54-pdf-standalone-geometry-calibration"
CALIBRATION_SUMMARY="$CALIBRATION_LOG_DIR/pdf-standalone-geometry-calibration-summary.json"
ORACLE_SUMMARY="${TERMSURF_ISSUE834_EXP50_ORACLE_SUMMARY:-$ROOT/logs/issue-834-exp50-separated-token-copy-oracle/separated-token-copy-oracle-summary.json}"
SUMMARY="$LOG_DIR/surfari-pdf-action-path-compare-summary.json"
HARNESS_LOG="$LOG_DIR/harness-$RUN_ID.log"

EXPECTED_TEXT="LEFT834 MID834 RIGHT834"

mkdir -p "$LOG_DIR"

log() {
  printf '%s\n' "$*" | tee -a "$HARNESS_LOG"
}

fail() {
  log "FAIL: $*"
  exit 1
}

copy_exp44_summary() {
  local target="$1"
  if [ -f "$EXP44_LOG_DIR/surfari-pdf-selection-copy-summary.json" ]; then
    cp "$EXP44_LOG_DIR/surfari-pdf-selection-copy-summary.json" "$target"
  else
    fail "missing Exp44 summary: $EXP44_LOG_DIR/surfari-pdf-selection-copy-summary.json"
  fi
}

run_embedded_cell() {
  local name="$1"
  local start_x="$2"
  local y="$3"
  local end_x="$4"
  local accepted="${5:-RIGHT834}"
  local copy_trace="$LOG_DIR/$name-embedded-copy-$RUN_ID.log"
  local geometry_trace="$LOG_DIR/$name-embedded-geometry-$RUN_ID.log"
  local summary="$LOG_DIR/$name-embedded-summary-$RUN_ID.json"

  log "running embedded cell name=$name start_x=$start_x y=$y end_x=$end_x accepted=$accepted"
  rm -f "$EXP44_LOG_DIR/surfari-pdf-selection-copy-summary.json"
  TERMSURF_ISSUE834_PDF_FIXTURE_MODE=separated-tokens \
    TERMSURF_ISSUE834_PDF_EXPECTED_TOKENS="$EXPECTED_TEXT" \
    TERMSURF_ISSUE834_PDF_ACCEPTED_SUBSTRING="$accepted" \
    TERMSURF_ISSUE834_PDF_COPY_DRAG_START_X_RATIO="$start_x" \
    TERMSURF_ISSUE834_PDF_COPY_DRAG_END_X_RATIO="$end_x" \
    TERMSURF_ISSUE834_PDF_COPY_DRAG_Y_RATIO="$y" \
    TERMSURF_ISSUE834_PDF_COPY_DELAY_AFTER_DRAG=0.25 \
    TERMSURF_SURFARI_PDF_COPY_TRACE=1 \
    TERMSURF_SURFARI_PDF_COPY_TRACE_FILE="$copy_trace" \
    TERMSURF_SURFARI_PDF_COPY_DIRECT=1 \
    TERMSURF_SURFARI_PDF_VIEW_GEOMETRY_TRACE=1 \
    TERMSURF_SURFARI_PDF_VIEW_GEOMETRY_TRACE_FILE="$geometry_trace" \
    env -u TERMSURF_SURFARI_PDF_MOUSE_DISPATCH_PROBE \
      -u TERMSURF_SURFARI_PDF_MOUSE_DISPATCH_MODE \
      -u TERMSURF_SURFARI_PDF_SELECTION_EDGE_PROBE \
      -u TERMSURF_SURFARI_PDF_SELECTION_EDGE_MODE \
      -u TERMSURF_SURFARI_PDF_SELECTION_EDGE_DELTA_X \
      -u TERMSURF_SURFARI_PDF_RESPONDER_PROBE \
      -u TERMSURF_SURFARI_PDF_RESPONDER_MODE \
      "$ROOT/scripts/test-issue-834-surfari-pdf-selection-copy.sh" >>"$HARNESS_LOG" 2>&1
  copy_exp44_summary "$summary"
}

log "run_id=$RUN_ID"
log "oracle_summary=$ORACLE_SUMMARY"
log "calibration_summary=$CALIBRATION_SUMMARY"

[ -f "$ORACLE_SUMMARY" ] || fail "missing oracle summary: $ORACLE_SUMMARY"

log "refreshing standalone calibration gate"
rm -rf "$CALIBRATION_LOG_DIR"
"$ROOT/scripts/test-issue-834-pdf-standalone-geometry-calibration.sh" >>"$HARNESS_LOG" 2>&1
[ -f "$CALIBRATION_SUMMARY" ] || fail "missing refreshed calibration summary"

run_embedded_cell oracle-base 0.18 0.25 0.86 RIGHT834
run_embedded_cell oracle-y-low 0.18 0.21 0.86 RIGHT834
run_embedded_cell no-selection 0.18 0.25 0.18 RIGHT834

python3 - "$SUMMARY" "$RUN_ID" "$ORACLE_SUMMARY" "$CALIBRATION_SUMMARY" "$LOG_DIR" <<'PY'
from pathlib import Path
import json
import re
import sys

summary_path, run_id, oracle_path, calibration_path, log_dir = sys.argv[1:6]
log_dir = Path(log_dir)

EXPECTED = ["LEFT834", "MID834", "RIGHT834"]

def load_json(path):
    path = Path(path)
    if not path.exists():
        return None
    return json.loads(path.read_text())

def read_text(path):
    path = Path(path)
    return path.read_text(errors="replace") if path.exists() else ""

def contains_all_tokens(sample):
    return all(token in (sample or "") for token in EXPECTED)

def last_line(text, marker):
    lines = [line for line in text.splitlines() if marker in line]
    return lines[-1] if lines else ""

def action_evidence(trace):
    state = (
        last_line(trace, "pdf-copy-state")
        or last_line(trace, "standalone-pdf-calibration-state")
        or last_line(trace, "pdf-view-geometry-state")
    )
    geometry = (
        last_line(trace, "pdf-view-geometry-state")
        or last_line(trace, "standalone-pdf-calibration-state")
    )
    js = last_line(trace, "pdf-copy-js")
    direct = [line for line in trace.splitlines() if "explicit-copy-target" in line or "direct-copy" in line]
    return {
        "has_state": bool(state),
        "has_js_selection_trace": bool(js),
        "has_direct_copy_trace": bool(direct),
        "state_line": state[:1200],
        "geometry_line": geometry[:1200],
        "js_line": js[:1200],
        "direct_copy_lines": direct[-6:],
        "target_nil": re.search(r"target_nil=([^ ]+)", state).group(1) if re.search(r"target_nil=([^ ]+)", state) else "",
        "target_webview": re.search(r"target_webview=([^ ]+)", state).group(1) if re.search(r"target_webview=([^ ]+)", state) else "",
        "hit_target": re.search(r"hit=([^ ]+)", geometry).group(1) if re.search(r"hit=([^ ]+)", geometry) else "",
        "first_responder": re.search(r"first_responder=([^ ]+)", state).group(1) if re.search(r"first_responder=([^ ]+)", state) else "",
        "key_window": re.search(r"key_window=([01])", state).group(1) if re.search(r"key_window=([01])", state) else "",
        "main_window": re.search(r"main_window=([01])", state).group(1) if re.search(r"main_window=([01])", state) else "",
    }

oracle = load_json(oracle_path)
calibration = load_json(calibration_path)

oracle_gate_open = bool(oracle and oracle.get("overall_result") == "pass")
standalone_success_count = int((calibration or {}).get("standalone_success_count") or 0)
calibration_gate_open = bool(
    calibration
    and calibration.get("oracle_gate_open") is True
    and standalone_success_count >= 2
)
calibration_fixture = (calibration or {}).get("fixture_identity", {})
calibration_fixture_match = bool(
    calibration_fixture.get("extracted_text") == "LEFT834 MID834 RIGHT834"
    and "RIGHT834" in calibration_fixture.get("operator_summary", "")
)

standalone_by_name = {
    cell.get("name"): cell
    for cell in (calibration or {}).get("standalone_cells", [])
}
selected_names = ["oracle-base", "oracle-y-low"]

cells = []
for name in selected_names:
    standalone = standalone_by_name.get(name) or {}
    embedded_summary_path = log_dir / f"{name}-embedded-summary-{run_id}.json"
    embedded_copy_trace = log_dir / f"{name}-embedded-copy-{run_id}.log"
    embedded_geometry_trace = log_dir / f"{name}-embedded-geometry-{run_id}.log"
    standalone_trace = Path(standalone.get("artifacts", {}).get("trace", ""))
    embedded = load_json(embedded_summary_path)
    standalone_sample = standalone.get("clipboard", {}).get("after_sample", "")
    embedded_sample = (embedded or {}).get("clipboard", {}).get("after_copy_sample", "")
    embedded_fallback = (embedded or {}).get("clipboard", {}).get("fallback_select_all_after_sample", "")
    cells.append({
        "name": name,
        "drag_ratios": standalone.get("drag_ratios"),
        "standalone": {
            "summary": standalone.get("path"),
            "trace": str(standalone_trace),
            "contains_all_tokens": contains_all_tokens(standalone_sample),
            "after_sample": standalone_sample,
            "trace_complete": standalone.get("trace_complete"),
            "action": action_evidence(read_text(standalone_trace)),
        },
        "embedded": {
            "summary": str(embedded_summary_path),
            "copy_trace": str(embedded_copy_trace),
            "geometry_trace": str(embedded_geometry_trace),
            "overall_result": (embedded or {}).get("overall_result"),
            "classification": (embedded or {}).get("classification"),
            "after_copy_sample": embedded_sample,
            "fallback_select_all_after_sample": embedded_fallback,
            "contains_all_tokens": contains_all_tokens(embedded_sample),
            "contains_right": "RIGHT834" in embedded_sample or "RIGHT834" in embedded_fallback,
            "clipboard_restore_status": (embedded or {}).get("clipboard", {}).get("restore_status"),
            "fixture": (embedded or {}).get("fixture", {}),
            "action": action_evidence(read_text(embedded_copy_trace) + "\n" + read_text(embedded_geometry_trace)),
        },
    })

no_selection_summary = load_json(log_dir / f"no-selection-embedded-summary-{run_id}.json")
no_selection_sample = (no_selection_summary or {}).get("clipboard", {}).get("after_copy_sample", "")
no_selection_sentinel = (no_selection_summary or {}).get("clipboard", {}).get("sentinel", "")
no_selection_control = {
    "summary": str(log_dir / f"no-selection-embedded-summary-{run_id}.json"),
    "overall_result": (no_selection_summary or {}).get("overall_result"),
    "after_copy_sample": no_selection_sample,
    "sentinel": no_selection_sentinel,
    "sentinel_unchanged": bool(no_selection_sentinel and no_selection_sample == no_selection_sentinel),
    "clipboard_restore_status": (no_selection_summary or {}).get("clipboard", {}).get("restore_status"),
}

standalone_success = bool(cells) and all(cell["standalone"]["contains_all_tokens"] for cell in cells)
embedded_failure = bool(cells) and all(not cell["embedded"]["contains_all_tokens"] for cell in cells)
embedded_traces = bool(cells) and all(cell["embedded"]["action"]["has_state"] for cell in cells)
standalone_traces = bool(cells) and all(cell["standalone"]["action"]["has_state"] for cell in cells)
clipboard_restored = (
    all(cell["embedded"]["clipboard_restore_status"] == "restored" for cell in cells)
    and no_selection_control["clipboard_restore_status"] == "restored"
)
no_selection_ok = no_selection_control["sentinel_unchanged"]
fixture_match = calibration_fixture_match and all(
    cell["embedded"].get("fixture", {}).get("mode") == "separated-tokens"
    and "RIGHT834" in " ".join(cell["embedded"].get("fixture", {}).get("expected_tokens", []))
    and cell["embedded"].get("fixture", {}).get("text_extracted") == "LEFT834 MID834 RIGHT834"
    and Path(cell["embedded"]["summary"]).exists()
    for cell in cells
)

copy_target_gap = any(
    (cell["standalone"]["action"]["target_webview"] and cell["standalone"]["action"]["target_webview"] != "nil")
    and (not cell["embedded"]["action"]["target_webview"] or cell["embedded"]["action"]["target_webview"] == "nil")
    for cell in cells
)
direct_copy_all = any(
    any(contains_all_tokens(line) for line in cell["embedded"]["action"]["direct_copy_lines"])
    for cell in cells
)
selection_state_gap = any(
    cell["embedded"]["action"]["has_js_selection_trace"]
    and '"length":0' in cell["embedded"]["action"]["js_line"]
    for cell in cells
)

if not (
    oracle_gate_open
    and calibration_gate_open
    and fixture_match
    and standalone_success
    and embedded_failure
    and standalone_traces
    and embedded_traces
    and clipboard_restored
    and no_selection_ok
):
    overall = "partial"
    classification = "harness-insufficient"
elif direct_copy_all:
    overall = "pass"
    classification = "direct-copy-candidate"
elif copy_target_gap:
    overall = "pass"
    classification = "copy-target-gap"
elif selection_state_gap:
    overall = "pass"
    classification = "selection-state-gap"
else:
    overall = "pass"
    classification = "action-path-equivalent-selection-missing"

data = {
    "overall_result": overall,
    "classification": classification,
    "run_id": run_id,
    "oracle_summary": oracle_path,
    "calibration_summary": calibration_path,
    "oracle_gate_open": oracle_gate_open,
    "calibration_gate_open": calibration_gate_open,
    "standalone_success_count": standalone_success_count,
    "fixture_identity_match": fixture_match,
    "standalone_success": standalone_success,
    "embedded_failure_reproduced": embedded_failure,
    "standalone_traces_complete": standalone_traces,
    "embedded_traces_complete": embedded_traces,
    "clipboard_restored": clipboard_restored,
    "no_selection_control": no_selection_control,
    "copy_target_gap": copy_target_gap,
    "direct_copy_all_tokens": direct_copy_all,
    "selection_state_gap": selection_state_gap,
    "cells": cells,
    "artifacts": {
        "harness_log": str(log_dir / f"harness-{run_id}.log"),
    },
}
Path(summary_path).write_text(json.dumps(data, indent=2, sort_keys=True) + "\n")
print(json.dumps({"overall_result": overall, "classification": classification}, indent=2, sort_keys=True))
PY

log "summary=$SUMMARY"
