#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RUN_ID="$(date +%Y%m%d-%H%M%S)"
LOG_DIR="$ROOT/logs/issue-834-exp55-surfari-pdf-calibrated-gesture-copy"
EXP44_LOG_DIR="$ROOT/logs/issue-834-exp44-surfari-pdf-selection-copy"
SUMMARY="$LOG_DIR/surfari-pdf-calibrated-gesture-copy-summary.json"
HARNESS_LOG="$LOG_DIR/harness-$RUN_ID.log"
ORACLE_SUMMARY="${TERMSURF_ISSUE834_EXP50_ORACLE_SUMMARY:-$ROOT/logs/issue-834-exp50-separated-token-copy-oracle/separated-token-copy-oracle-summary.json}"
CALIBRATION_SUMMARY="${TERMSURF_ISSUE834_EXP54_CALIBRATION_SUMMARY:-$ROOT/logs/issue-834-exp54-pdf-standalone-geometry-calibration/pdf-standalone-geometry-calibration-summary.json}"
ORIGINAL_CLIPBOARD="$LOG_DIR/original-clipboard-$RUN_ID.txt"
ORIGINAL_RESTORE_STATUS="not-attempted"

EXPECTED_TEXT="LEFT834 MID834 RIGHT834"
EXPECTED_TOKENS=("LEFT834" "MID834" "RIGHT834")

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
  local start_x="$2"
  local y="$3"
  local end_x="$4"
  local out_summary="$LOG_DIR/$name-embedded-summary-$RUN_ID.json"
  local geometry_trace="$LOG_DIR/$name-embedded-geometry-$RUN_ID.log"
  local copy_trace="$LOG_DIR/$name-embedded-copy-$RUN_ID.log"

  rm -rf "$EXP44_LOG_DIR"
  log "embedded_cell=$name ratios=${start_x},${y}-${end_x},${y}"
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
    env -u TERMSURF_SURFARI_CACONTEXT_LAYER \
    "$ROOT/scripts/test-issue-834-surfari-pdf-selection-copy.sh" >>"$HARNESS_LOG" 2>&1; then
    :
  fi

  if [ -f "$EXP44_LOG_DIR/surfari-pdf-selection-copy-summary.json" ]; then
    python3 - "$EXP44_LOG_DIR/surfari-pdf-selection-copy-summary.json" "$out_summary" "$name" "$start_x" "$y" "$end_x" "$geometry_trace" "$copy_trace" <<'PY'
import json
import sys
from pathlib import Path

source, target, name, start_x, y, end_x, geometry_trace, copy_trace = sys.argv[1:9]
data = json.loads(Path(source).read_text())
data["exp55_cell"] = {
    "name": name,
    "drag_ratios": {
        "start_x": float(start_x),
        "end_x": float(end_x),
        "y": float(y),
    },
    "copy_route": "external-cmd-c-plus-direct-probe",
    "geometry_trace": geometry_trace,
    "copy_trace": copy_trace,
}
Path(target).write_text(json.dumps(data, indent=2, sort_keys=True) + "\n")
PY
  else
    python3 - "$out_summary" "$name" "$start_x" "$y" "$end_x" "$geometry_trace" "$copy_trace" <<'PY'
import json
import sys
from pathlib import Path

target, name, start_x, y, end_x, geometry_trace, copy_trace = sys.argv[1:8]
Path(target).write_text(json.dumps({
    "overall_result": "missing",
    "classification": "missing-summary",
    "exp55_cell": {
        "name": name,
        "drag_ratios": {
            "start_x": float(start_x),
            "end_x": float(end_x),
            "y": float(y),
        },
        "copy_route": "external-cmd-c-plus-direct-probe",
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
import re
import sys
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

def load(path):
    return json.loads(path.read_text()) if path.exists() else None

def read(path):
    return Path(path).read_text(errors="replace") if path and Path(path).exists() else ""

def tokens_in(value):
    return [token for token in expected if token in (value or "")]

def trace_lines_with_samples(trace):
    lines = []
    for line in trace.splitlines():
        if "surfari-pdf-copy-direct" in line or "surfari-pdf-copy-inprocess" in line:
            lines.append(line[:600])
    return lines

def parse_state_line(line):
    fields = {}
    for key in [
        "key_window",
        "main_window",
        "app_key_window",
        "app_main_window",
        "first_responder",
        "responder_chain",
        "target_nil",
        "target_webview",
    ]:
        match = re.search(rf"(?:^| ){key}=([^ ]+)", line)
        fields[key] = match.group(1) if match else ""
    return fields

def select_state(trace, marker, preferred_labels):
    lines = [line for line in trace.splitlines() if marker in line]
    for label in preferred_labels:
        for line in reversed(lines):
            if f"label={label} " in line:
                return parse_state_line(line), line[:800]
    if lines:
        return parse_state_line(lines[-1]), lines[-1][:800]
    return {}, ""

def class_name(value):
    if not value:
        return ""
    return value.split(":", 1)[0]

def responder_comparison(embedded_trace, embedded_copy_trace, standalone_trace):
    embedded_state, embedded_line = select_state(
        embedded_trace + "\n" + embedded_copy_trace,
        "surfari-pdf-view-geometry-state",
        ["after-direct-copy", "after-external-copy", "before-direct-copy", "mouse-up"],
    )
    if not embedded_state:
        embedded_state, embedded_line = select_state(
            embedded_copy_trace,
            "surfari-pdf-copy-state",
            ["after-direct-copy", "after-external-copy", "before-direct-copy", "after-mouse-up"],
        )
    standalone_state, standalone_line = select_state(
        standalone_trace,
        "standalone-pdf-calibration-state",
        ["after-copy", "before-copy", "after-drag"],
    )
    required = ["key_window", "main_window", "first_responder", "responder_chain", "target_nil", "target_webview"]
    complete = all(embedded_state.get(key) for key in required) and all(standalone_state.get(key) for key in required)
    differences = {}
    for key in ["key_window", "main_window"]:
        differences[key] = {
            "embedded": embedded_state.get(key, ""),
            "standalone": standalone_state.get(key, ""),
            "different": embedded_state.get(key, "") != standalone_state.get(key, ""),
        }
    for key in ["first_responder", "target_nil", "target_webview"]:
        differences[key] = {
            "embedded": embedded_state.get(key, ""),
            "standalone": standalone_state.get(key, ""),
            "embedded_class": class_name(embedded_state.get(key, "")),
            "standalone_class": class_name(standalone_state.get(key, "")),
            "different": class_name(embedded_state.get(key, "")) != class_name(standalone_state.get(key, "")),
        }
    differences["responder_chain"] = {
        "embedded": embedded_state.get("responder_chain", ""),
        "standalone": standalone_state.get("responder_chain", ""),
        "different": embedded_state.get("responder_chain", "") != standalone_state.get("responder_chain", ""),
    }
    material_difference = complete and any(item["different"] for item in differences.values())
    return {
        "complete": complete,
        "material_difference": material_difference,
        "embedded_state": embedded_state,
        "standalone_state": standalone_state,
        "differences": differences,
        "embedded_line": embedded_line,
        "standalone_line": standalone_line,
    }

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
standalone_by_name = {}
for cell in (calibration or {}).get("standalone_cells", []):
    standalone_by_name[cell.get("name")] = cell

cells = []
missing = []
for path in sorted(log_dir.glob(f"*-embedded-summary-{run_id}.json")):
    data = load(path) or {}
    cell = data.get("exp55_cell", {})
    name = cell.get("name")
    standalone = standalone_by_name.get(name)
    clipboard = data.get("clipboard", {})
    primary = clipboard.get("after_copy_sample", "")
    fallback = clipboard.get("fallback_select_all_after_sample", "")
    copy_trace = read(cell.get("copy_trace"))
    geometry_trace = read(cell.get("geometry_trace"))
    standalone_trace = read(standalone.get("artifacts", {}).get("trace") if standalone else "")
    responder = responder_comparison(geometry_trace, copy_trace, standalone_trace)
    primary_tokens = tokens_in(primary)
    fallback_tokens = tokens_in(fallback)
    direct_lines = trace_lines_with_samples(copy_trace)
    direct_tokens = sorted({token for line in direct_lines for token in tokens_in(line)})
    matched = bool(
        name == "embedded-ratio"
        or (
            standalone
            and standalone.get("clipboard", {}).get("contains_all_tokens") is True
            and standalone.get("drag_ratios") == cell.get("drag_ratios")
            and standalone.get("copy_route")
            and standalone.get("artifacts", {}).get("trace")
        )
    )
    trace_complete = all(
        marker in geometry_trace
        for marker in [
            "surfari-pdf-view-geometry-state",
            "surfari-pdf-view-geometry-hit-chain",
            "surfari-pdf-view-geometry-tree",
            "surfari-pdf-view-geometry-scroll",
            "target_nil=",
            "target_webview=",
        ]
    ) and bool(copy_trace) and bool(standalone_trace) and responder["complete"]
    fixture = data.get("fixture", {})
    fixture_identity = (calibration or {}).get("fixture_identity", {})
    fixture_match = (
        fixture.get("pdf_text_operator") == fixture_identity.get("operator_summary")
        and fixture.get("pdf_text_bboxes") == fixture_identity.get("token_boxes")
        and fixture.get("page_geometry") == fixture_identity.get("page_geometry")
        and fixture.get("font") == fixture_identity.get("font")
        and fixture.get("text_extracted") == fixture_identity.get("extracted_text")
    )
    if data.get("overall_result") == "missing":
        missing.append(name)
    cells.append({
        "path": str(path),
        "name": name,
        "is_calibrated": name in calibrated_names,
        "drag_ratios": cell.get("drag_ratios"),
        "copy_route": cell.get("copy_route"),
        "matched_standalone": {
            "present": standalone is not None,
            "name": standalone.get("name") if standalone else None,
            "drag_ratios": standalone.get("drag_ratios") if standalone else None,
            "copy_route": standalone.get("copy_route") if standalone else None,
            "trace": standalone.get("artifacts", {}).get("trace") if standalone else None,
            "contains_all_tokens": standalone.get("clipboard", {}).get("contains_all_tokens") if standalone else None,
        },
        "matched_standalone_gate": matched,
        "fixture_identity_match": fixture_match,
        "trace_complete": trace_complete,
        "responder_comparison": responder,
        "primary_sample": primary,
        "primary_tokens": primary_tokens,
        "primary_contains_all_tokens": all(token in primary for token in expected),
        "fallback_sample": fallback,
        "fallback_tokens": fallback_tokens,
        "fallback_contains_all_tokens": all(token in fallback for token in expected),
        "direct_probe_lines": direct_lines,
        "direct_probe_tokens": direct_tokens,
        "direct_probe_contains_all_tokens": all(token in direct_tokens for token in expected),
        "artifacts": {
            "geometry_trace": cell.get("geometry_trace"),
            "copy_trace": cell.get("copy_trace"),
        },
    })

calibrated_cells = [cell for cell in cells if cell["is_calibrated"]]
comparison_cells = [cell for cell in cells if not cell["is_calibrated"]]
matched_all = bool(calibrated_cells) and all(cell["matched_standalone_gate"] for cell in calibrated_cells)
fixture_all = bool(cells) and all(cell["fixture_identity_match"] for cell in cells)
trace_all = bool(cells) and all(cell["trace_complete"] for cell in cells)
primary_passes = [cell for cell in calibrated_cells if cell["primary_contains_all_tokens"]]
primary_left_mid_only = [
    cell for cell in calibrated_cells
    if "LEFT834" in cell["primary_tokens"]
    and "MID834" in cell["primary_tokens"]
    and "RIGHT834" not in cell["primary_tokens"]
]
primary_left_only = [
    cell for cell in calibrated_cells
    if cell["primary_tokens"] == ["LEFT834"]
]
fallback_or_direct_all = [
    cell for cell in calibrated_cells
    if not cell["primary_contains_all_tokens"]
    and (cell["fallback_contains_all_tokens"] or cell["direct_probe_contains_all_tokens"])
]
responder_gap_cells = [
    cell for cell in calibrated_cells
    if not cell["primary_contains_all_tokens"]
    and cell.get("responder_comparison", {}).get("material_difference")
]

if restore_status != "restored":
    result = "fail"
    classification = "clipboard-restore-failed"
elif (
    not oracle_gate_open
    or not calibration_gate_open
    or missing
    or not matched_all
    or not fixture_all
    or not trace_all
):
    result = "partial"
    classification = "harness-insufficient"
elif len(primary_passes) == len(calibrated_cells):
    result = "pass"
    classification = "embedded-calibrated-matrix-pass"
elif primary_passes:
    result = "pass"
    classification = "embedded-calibrated-single-cell-pass"
elif fallback_or_direct_all:
    result = "pass"
    classification = "embedded-calibrated-copy-routing-gap"
elif responder_gap_cells:
    result = "pass"
    classification = "responder-gap-candidate"
elif primary_left_only:
    result = "pass"
    classification = "embedded-calibrated-coordinate-selection-gap"
elif primary_left_mid_only:
    result = "pass"
    classification = "embedded-calibrated-right-edge-gap"
else:
    result = "partial"
    classification = "harness-insufficient"

data = {
    "overall_result": result,
    "classification": classification,
    "run_id": run_id,
    "oracle_summary": str(oracle_path),
    "oracle_gate_open": oracle_gate_open,
    "calibration_summary": str(calibration_path),
    "calibration_gate_open": calibration_gate_open,
    "clipboard_restore_status": restore_status,
    "matched_calibrated_cells": matched_all,
    "fixture_identity_match": fixture_all,
    "traces_complete": trace_all,
    "missing_cells": missing,
    "primary_pass_count": len(primary_passes),
    "primary_pass_names": [cell["name"] for cell in primary_passes],
    "primary_left_only_names": [cell["name"] for cell in primary_left_only],
    "primary_left_mid_only_names": [cell["name"] for cell in primary_left_mid_only],
    "fallback_or_direct_all_names": [cell["name"] for cell in fallback_or_direct_all],
    "responder_gap_names": [cell["name"] for cell in responder_gap_cells],
    "cells": cells,
    "comparison_cells": comparison_cells,
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
    "matched_calibrated_cells": matched_all,
    "fixture_identity_match": fixture_all,
    "traces_complete": trace_all,
    "primary_pass_count": len(primary_passes),
    "primary_pass_names": [cell["name"] for cell in primary_passes],
    "primary_left_only_names": [cell["name"] for cell in primary_left_only],
    "responder_gap_names": [cell["name"] for cell in responder_gap_cells],
}, indent=2, sort_keys=True))
if result == "fail":
    sys.exit(1)
PY
}

pbpaste >"$ORIGINAL_CLIPBOARD" || true
log "run_id=$RUN_ID"
log "oracle_summary=$ORACLE_SUMMARY"
log "calibration_summary=$CALIBRATION_SUMMARY"

run_cell embedded-ratio 0.58 0.43 0.99
run_cell oracle-base 0.18 0.25 0.86
run_cell oracle-y-low 0.18 0.21 0.86
run_cell oracle-y-high 0.18 0.29 0.86
run_cell oracle-x-wide 0.16 0.25 0.90
run_cell oracle-x-tight 0.20 0.25 0.82

restore_original_clipboard || ORIGINAL_RESTORE_STATUS="restore-failed"
classify
log "summary=$SUMMARY"
