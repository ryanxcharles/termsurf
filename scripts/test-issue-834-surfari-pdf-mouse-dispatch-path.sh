#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RUN_ID="$(date +%Y%m%d-%H%M%S)"
LOG_DIR="$ROOT/logs/issue-834-exp57-surfari-pdf-mouse-dispatch-path"
EXP44_LOG_DIR="$ROOT/logs/issue-834-exp44-surfari-pdf-selection-copy"
SUMMARY="$LOG_DIR/surfari-pdf-mouse-dispatch-path-summary.json"
HARNESS_LOG="$LOG_DIR/harness-$RUN_ID.log"
ORACLE_SUMMARY="${TERMSURF_ISSUE834_EXP50_ORACLE_SUMMARY:-$ROOT/logs/issue-834-exp50-separated-token-copy-oracle/separated-token-copy-oracle-summary.json}"
CALIBRATION_SUMMARY="${TERMSURF_ISSUE834_EXP54_CALIBRATION_SUMMARY:-$ROOT/logs/issue-834-exp54-pdf-standalone-geometry-calibration/pdf-standalone-geometry-calibration-summary.json}"
ORIGINAL_CLIPBOARD="$LOG_DIR/original-clipboard-$RUN_ID.txt"
ORIGINAL_RESTORE_STATUS="not-attempted"

EXPECTED_TEXT="LEFT834 MID834 RIGHT834"

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
  local mode="$1"
  local name="$2"
  local start_x="$3"
  local y="$4"
  local end_x="$5"
  local out_summary="$LOG_DIR/$mode-$name-embedded-summary-$RUN_ID.json"
  local geometry_trace="$LOG_DIR/$mode-$name-embedded-geometry-$RUN_ID.log"
  local copy_trace="$LOG_DIR/$mode-$name-embedded-copy-$RUN_ID.log"

  rm -rf "$EXP44_LOG_DIR"
  log "mode=$mode cell=$name ratios=${start_x},${y}-${end_x},${y}"

  if [ "$mode" = "normal-control" ]; then
    if TERMSURF_ISSUE834_PDF_FIXTURE_MODE=separated-tokens \
      TERMSURF_ISSUE834_PDF_EXPECTED_TOKENS="$EXPECTED_TEXT" \
      TERMSURF_ISSUE834_PDF_ACCEPTED_SUBSTRING="RIGHT834" \
      TERMSURF_ISSUE834_PDF_COPY_DRAG_START_X_RATIO="$start_x" \
      TERMSURF_ISSUE834_PDF_COPY_DRAG_END_X_RATIO="$end_x" \
      TERMSURF_ISSUE834_PDF_COPY_DRAG_Y_RATIO="$y" \
      TERMSURF_ISSUE834_PDF_COPY_DELAY_AFTER_DRAG=0.25 \
      TERMSURF_SURFARI_PDF_COPY_TRACE=1 \
      TERMSURF_SURFARI_PDF_COPY_TRACE_FILE="$copy_trace" \
      TERMSURF_SURFARI_PDF_COPY_DIRECT=1 \
      TERMSURF_SURFARI_PDF_VIEW_GEOMETRY_TRACE=1 \
      TERMSURF_SURFARI_PDF_VIEW_GEOMETRY_TRACE_FILE="$geometry_trace" \
      env \
        -u TERMSURF_SURFARI_CACONTEXT_LAYER \
        -u TERMSURF_SURFARI_PDF_MOUSE_DISPATCH_PROBE \
        -u TERMSURF_SURFARI_PDF_MOUSE_DISPATCH_MODE \
        -u TERMSURF_SURFARI_PDF_SELECTION_EDGE_PROBE \
        -u TERMSURF_SURFARI_PDF_SELECTION_EDGE_MODE \
        -u TERMSURF_SURFARI_PDF_SELECTION_EDGE_DELTA_X \
        -u TERMSURF_SURFARI_PDF_RESPONDER_PROBE \
        -u TERMSURF_SURFARI_PDF_RESPONDER_MODE \
        "$ROOT/scripts/test-issue-834-surfari-pdf-selection-copy.sh" >>"$HARNESS_LOG" 2>&1; then
      :
    fi
  else
    local dispatch_mode="$mode"
    if [ "$mode" = "flagged-current" ]; then
      dispatch_mode="current"
    fi
    if TERMSURF_ISSUE834_PDF_FIXTURE_MODE=separated-tokens \
      TERMSURF_ISSUE834_PDF_EXPECTED_TOKENS="$EXPECTED_TEXT" \
      TERMSURF_ISSUE834_PDF_ACCEPTED_SUBSTRING="RIGHT834" \
      TERMSURF_ISSUE834_PDF_COPY_DRAG_START_X_RATIO="$start_x" \
      TERMSURF_ISSUE834_PDF_COPY_DRAG_END_X_RATIO="$end_x" \
      TERMSURF_ISSUE834_PDF_COPY_DRAG_Y_RATIO="$y" \
      TERMSURF_ISSUE834_PDF_COPY_DELAY_AFTER_DRAG=0.25 \
      TERMSURF_SURFARI_PDF_COPY_TRACE=1 \
      TERMSURF_SURFARI_PDF_COPY_TRACE_FILE="$copy_trace" \
      TERMSURF_SURFARI_PDF_COPY_DIRECT=1 \
      TERMSURF_SURFARI_PDF_VIEW_GEOMETRY_TRACE=1 \
      TERMSURF_SURFARI_PDF_VIEW_GEOMETRY_TRACE_FILE="$geometry_trace" \
      TERMSURF_SURFARI_PDF_MOUSE_DISPATCH_PROBE=1 \
      TERMSURF_SURFARI_PDF_MOUSE_DISPATCH_MODE="$dispatch_mode" \
      env \
        -u TERMSURF_SURFARI_CACONTEXT_LAYER \
        -u TERMSURF_SURFARI_PDF_SELECTION_EDGE_PROBE \
        -u TERMSURF_SURFARI_PDF_SELECTION_EDGE_MODE \
        -u TERMSURF_SURFARI_PDF_SELECTION_EDGE_DELTA_X \
        -u TERMSURF_SURFARI_PDF_RESPONDER_PROBE \
        -u TERMSURF_SURFARI_PDF_RESPONDER_MODE \
        "$ROOT/scripts/test-issue-834-surfari-pdf-selection-copy.sh" >>"$HARNESS_LOG" 2>&1; then
      :
    fi
  fi

  if [ -f "$EXP44_LOG_DIR/surfari-pdf-selection-copy-summary.json" ]; then
    python3 - "$EXP44_LOG_DIR/surfari-pdf-selection-copy-summary.json" "$out_summary" "$mode" "$name" "$start_x" "$y" "$end_x" "$geometry_trace" "$copy_trace" <<'PY'
import json
import sys
from pathlib import Path

source, target, mode, name, start_x, y, end_x, geometry_trace, copy_trace = sys.argv[1:10]
data = json.loads(Path(source).read_text())
data["exp57_cell"] = {
    "mode": mode,
    "name": name,
    "drag_ratios": {
        "start_x": float(start_x),
        "end_x": float(end_x),
        "y": float(y),
    },
    "primary_copy_route": "external-cmd-c",
    "direct_probe_route": "direct-probe",
    "geometry_trace": geometry_trace,
    "copy_trace": copy_trace,
}
Path(target).write_text(json.dumps(data, indent=2, sort_keys=True) + "\n")
PY
  else
    python3 - "$out_summary" "$mode" "$name" "$start_x" "$y" "$end_x" "$geometry_trace" "$copy_trace" <<'PY'
import json
import sys
from pathlib import Path

target, mode, name, start_x, y, end_x, geometry_trace, copy_trace = sys.argv[1:9]
Path(target).write_text(json.dumps({
    "overall_result": "missing",
    "classification": "missing-summary",
    "exp57_cell": {
        "mode": mode,
        "name": name,
        "drag_ratios": {
            "start_x": float(start_x),
            "end_x": float(end_x),
            "y": float(y),
        },
        "primary_copy_route": "external-cmd-c",
        "direct_probe_route": "direct-probe",
        "geometry_trace": geometry_trace,
        "copy_trace": copy_trace,
    },
}, indent=2, sort_keys=True) + "\n")
PY
  fi
}

classify() {
  python3 - "$SUMMARY" "$RUN_ID" "$ORIGINAL_RESTORE_STATUS" "$ORACLE_SUMMARY" "$CALIBRATION_SUMMARY" "$LOG_DIR" "$HARNESS_LOG" <<'PY'
import json
import sys
from collections import defaultdict
from pathlib import Path

summary_path = Path(sys.argv[1])
run_id = sys.argv[2]
restore_status = sys.argv[3]
oracle_path = Path(sys.argv[4])
calibration_path = Path(sys.argv[5])
log_dir = Path(sys.argv[6])
harness_log = Path(sys.argv[7])
expected = ["LEFT834", "MID834", "RIGHT834"]
calibrated_names = {"oracle-base", "oracle-y-low", "oracle-y-high", "oracle-x-wide", "oracle-x-tight"}
expected_modes = [
    "normal-control",
    "flagged-current",
    "window-send-event",
    "webview-direct",
    "flipped-view-direct",
    "pdf-hud-direct",
]
control_modes = {"normal-control", "flagged-current"}
descendant_modes = {"flipped-view-direct", "pdf-hud-direct"}

def load(path):
    return json.loads(path.read_text()) if path.exists() else None

def read(path):
    return Path(path).read_text(errors="replace") if path and Path(path).exists() else ""

def tokens_in(value):
    return [token for token in expected if token in (value or "")]

def sample_lines(trace, marker):
    return [line[:800] for line in trace.splitlines() if marker in line]

def dispatch_lines(trace):
    return sample_lines(trace, "surfari-pdf-mouse-dispatch")

def dispatch_event_phases(lines):
    phases = set()
    for line in lines:
        for phase in ["mouse-down", "mouse-drag", "mouse-up"]:
            if f"phase={phase} " in line:
                phases.add(phase)
    return sorted(phases)

def target_unavailable(lines):
    return any("target_exists=0" in line for line in lines)

oracle = load(oracle_path)
calibration = load(calibration_path)
oracle_gate_open = bool(
    oracle
    and oracle.get("classification") == "separated-token-oracle-pass"
    and oracle.get("embedded_interpretation_gate") == "open"
)
calibration_gate_open = bool(
    calibration
    and calibration.get("classification") == "embedded-gesture-outside-standalone-band"
    and calibration.get("standalone_success_count", 0) > 0
    and calibration.get("fixture_identity_match") is True
)
standalone_by_name = {cell.get("name"): cell for cell in (calibration or {}).get("standalone_cells", [])}

cells = []
missing = []
for path in sorted(log_dir.glob(f"*-embedded-summary-{run_id}.json")):
    data = load(path) or {}
    cell = data.get("exp57_cell", {})
    mode = cell.get("mode")
    name = cell.get("name")
    standalone = standalone_by_name.get(name)
    clipboard = data.get("clipboard", {})
    primary = clipboard.get("after_copy_sample", "")
    fallback = clipboard.get("fallback_select_all_after_sample", "")
    copy_trace = read(cell.get("copy_trace"))
    geometry_trace = read(cell.get("geometry_trace"))
    direct_lines = sample_lines(copy_trace, "surfari-pdf-copy-direct")
    dispatch = dispatch_lines(copy_trace)
    direct_tokens = sorted({token for line in direct_lines for token in tokens_in(line)})
    fixture = data.get("fixture", {})
    fixture_identity = (calibration or {}).get("fixture_identity", {})
    fixture_match = (
        fixture.get("pdf_text_operator") == fixture_identity.get("operator_summary")
        and fixture.get("pdf_text_bboxes") == fixture_identity.get("token_boxes")
        and fixture.get("page_geometry") == fixture_identity.get("page_geometry")
        and fixture.get("font") == fixture_identity.get("font")
        and fixture.get("text_extracted") == fixture_identity.get("extracted_text")
    )
    matched = bool(
        standalone
        and standalone.get("clipboard", {}).get("contains_all_tokens") is True
        and standalone.get("drag_ratios") == cell.get("drag_ratios")
        and standalone.get("copy_route")
        and standalone.get("artifacts", {}).get("trace")
    )
    trace_complete = all(
        marker in geometry_trace
        for marker in [
            "surfari-pdf-view-geometry-state",
            "surfari-pdf-view-geometry-hit-chain",
            "surfari-pdf-view-geometry-tree",
            "surfari-pdf-view-geometry-scroll",
        ]
    ) and bool(copy_trace)
    dispatch_complete = mode == "normal-control" or set(dispatch_event_phases(dispatch)) >= {"mouse-down", "mouse-drag", "mouse-up"}
    if data.get("overall_result") == "missing":
        missing.append(f"{mode}:{name}")
    cells.append({
        "path": str(path),
        "mode": mode,
        "name": name,
        "drag_ratios": cell.get("drag_ratios"),
        "matched_standalone_gate": matched,
        "matched_standalone": {
            "present": standalone is not None,
            "name": standalone.get("name") if standalone else None,
            "drag_ratios": standalone.get("drag_ratios") if standalone else None,
            "copy_route": standalone.get("copy_route") if standalone else None,
            "trace": standalone.get("artifacts", {}).get("trace") if standalone else None,
            "contains_all_tokens": standalone.get("clipboard", {}).get("contains_all_tokens") if standalone else None,
        },
        "fixture_identity_match": fixture_match,
        "trace_complete": trace_complete,
        "dispatch_trace_complete": dispatch_complete,
        "dispatch_event_phases": dispatch_event_phases(dispatch),
        "dispatch_target_unavailable": target_unavailable(dispatch),
        "dispatch_lines": dispatch,
        "primary_route": cell.get("primary_copy_route"),
        "primary_sample": primary,
        "primary_tokens": tokens_in(primary),
        "primary_contains_all_tokens": all(token in primary for token in expected),
        "fallback_sample": fallback,
        "fallback_tokens": tokens_in(fallback),
        "fallback_contains_all_tokens": all(token in fallback for token in expected),
        "direct_probe_route": cell.get("direct_probe_route"),
        "direct_probe_lines": direct_lines,
        "direct_probe_tokens": direct_tokens,
        "direct_probe_contains_all_tokens": all(token in direct_tokens for token in expected),
        "artifacts": {
            "geometry_trace": cell.get("geometry_trace"),
            "copy_trace": cell.get("copy_trace"),
        },
    })

cells_by_mode = defaultdict(list)
for cell in cells:
    cells_by_mode[cell["mode"]].append(cell)

mode_names_complete = all({cell["name"] for cell in cells_by_mode[mode]} == calibrated_names for mode in expected_modes)
matched_all = bool(cells) and all(cell["matched_standalone_gate"] for cell in cells)
fixture_all = bool(cells) and all(cell["fixture_identity_match"] for cell in cells)
trace_all = bool(cells) and all(cell["trace_complete"] for cell in cells)
dispatch_trace_all = all(cell["dispatch_trace_complete"] for cell in cells if cell["mode"] != "normal-control")

def mode_reproduces_baseline(mode):
    mode_cells = cells_by_mode[mode]
    return (
        len(mode_cells) == len(calibrated_names)
        and all(not cell["primary_contains_all_tokens"] for cell in mode_cells)
        and all("LEFT834" in cell["primary_tokens"] for cell in mode_cells)
        and all("RIGHT834" not in cell["primary_tokens"] for cell in mode_cells)
    )

normal_baseline_reproduced = mode_reproduces_baseline("normal-control")
flagged_baseline_reproduced = mode_reproduces_baseline("flagged-current")

non_control_modes = [mode for mode in expected_modes if mode not in control_modes]
matrix_winners = [
    mode for mode in non_control_modes
    if len(cells_by_mode[mode]) == len(calibrated_names)
    and all(cell["primary_contains_all_tokens"] for cell in cells_by_mode[mode])
]
single_winners = [
    {"mode": cell["mode"], "name": cell["name"]}
    for mode in non_control_modes
    for cell in cells_by_mode[mode]
    if cell["primary_contains_all_tokens"]
]
baseline_tokens = {cell["name"]: tuple(cell["primary_tokens"]) for cell in cells_by_mode["flagged-current"]}
changed_selection = [
    {"mode": cell["mode"], "name": cell["name"], "tokens": cell["primary_tokens"], "baseline_tokens": list(baseline_tokens.get(cell["name"], ()))}
    for mode in non_control_modes
    for cell in cells_by_mode[mode]
    if tuple(cell["primary_tokens"]) != baseline_tokens.get(cell["name"], ())
]
target_unavailable_cells = [
    {"mode": cell["mode"], "name": cell["name"]}
    for mode in descendant_modes
    for cell in cells_by_mode[mode]
    if cell["dispatch_target_unavailable"]
]
comparable_non_control_cells = [
    cell for mode in non_control_modes for cell in cells_by_mode[mode]
    if not cell["dispatch_target_unavailable"]
]

if restore_status != "restored":
    result = "fail"
    classification = "clipboard-restore-failed"
elif (
    not oracle_gate_open
    or not calibration_gate_open
    or missing
    or not mode_names_complete
    or not matched_all
    or not fixture_all
    or not trace_all
    or not dispatch_trace_all
    or not normal_baseline_reproduced
    or not flagged_baseline_reproduced
):
    result = "partial"
    classification = "harness-insufficient"
elif matrix_winners:
    result = "pass"
    classification = "mouse-dispatch-matrix-candidate"
elif single_winners:
    result = "pass"
    classification = "mouse-dispatch-fix-candidate"
elif changed_selection:
    result = "pass"
    classification = "dispatch-changes-selection-only"
elif target_unavailable_cells and not comparable_non_control_cells:
    result = "pass"
    classification = "dispatch-target-unavailable"
else:
    result = "pass"
    classification = "dispatch-path-unchanged"

data = {
    "overall_result": result,
    "classification": classification,
    "run_id": run_id,
    "oracle_summary": str(oracle_path),
    "oracle_gate_open": oracle_gate_open,
    "calibration_summary": str(calibration_path),
    "calibration_gate_open": calibration_gate_open,
    "clipboard_restore_status": restore_status,
    "mode_names_complete": mode_names_complete,
    "matched_calibrated_cells": matched_all,
    "fixture_identity_match": fixture_all,
    "traces_complete": trace_all,
    "dispatch_traces_complete": dispatch_trace_all,
    "normal_baseline_reproduced": normal_baseline_reproduced,
    "flagged_baseline_reproduced": flagged_baseline_reproduced,
    "matrix_winners": matrix_winners,
    "single_winners": single_winners,
    "changed_selection": changed_selection,
    "target_unavailable_cells": target_unavailable_cells,
    "missing_cells": missing,
    "cells": cells,
    "artifacts": {
        "harness_log": str(harness_log),
    },
}
summary_path.write_text(json.dumps(data, indent=2, sort_keys=True) + "\n")
print(json.dumps({
    "overall_result": result,
    "classification": classification,
    "oracle_gate_open": oracle_gate_open,
    "calibration_gate_open": calibration_gate_open,
    "normal_baseline_reproduced": normal_baseline_reproduced,
    "flagged_baseline_reproduced": flagged_baseline_reproduced,
    "matrix_winners": matrix_winners,
    "single_winner_count": len(single_winners),
    "changed_selection_count": len(changed_selection),
    "target_unavailable_count": len(target_unavailable_cells),
}, indent=2, sort_keys=True))
if result == "fail":
    sys.exit(1)
PY
}

pbpaste >"$ORIGINAL_CLIPBOARD" || true
log "run_id=$RUN_ID"
log "oracle_summary=$ORACLE_SUMMARY"
log "calibration_summary=$CALIBRATION_SUMMARY"

for mode in normal-control flagged-current window-send-event webview-direct flipped-view-direct pdf-hud-direct; do
  run_cell "$mode" oracle-base 0.18 0.25 0.86
  run_cell "$mode" oracle-y-low 0.18 0.21 0.86
  run_cell "$mode" oracle-y-high 0.18 0.29 0.86
  run_cell "$mode" oracle-x-wide 0.16 0.25 0.90
  run_cell "$mode" oracle-x-tight 0.20 0.25 0.82
done

restore_original_clipboard || ORIGINAL_RESTORE_STATUS="restore-failed"
classify
log "summary=$SUMMARY"
