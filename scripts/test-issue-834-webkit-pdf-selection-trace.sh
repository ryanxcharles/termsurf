#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RUN_ID="$(date +%Y%m%d-%H%M%S)"
LOG_DIR="$ROOT/logs/issue-834-exp58-webkit-pdf-selection-trace"
EXP44_LOG_DIR="$ROOT/logs/issue-834-exp44-surfari-pdf-selection-copy"
SUMMARY="$LOG_DIR/webkit-pdf-selection-trace-summary.json"
HARNESS_LOG="$LOG_DIR/harness-$RUN_ID.log"
WEBKIT_TRACE_CONTROL="/tmp/termsurf-webkit-pdf-selection-trace-file"
ORACLE_SUMMARY="${TERMSURF_ISSUE834_EXP50_ORACLE_SUMMARY:-$ROOT/logs/issue-834-exp50-separated-token-copy-oracle/separated-token-copy-oracle-summary.json}"
CALIBRATION_SUMMARY="${TERMSURF_ISSUE834_EXP54_CALIBRATION_SUMMARY:-$ROOT/logs/issue-834-exp54-pdf-standalone-geometry-calibration/pdf-standalone-geometry-calibration-summary.json}"
SURFARI="${TERMSURF_SURFARI:-$ROOT/target/debug/surfari}"
WEBKIT_DEBUG="${TERMSURF_WEBKIT_DEBUG:-$ROOT/webkit/src/WebKitBuild/Debug}"
EXPECTED_TEXT="LEFT834 MID834 RIGHT834"

mkdir -p "$LOG_DIR"

cleanup() {
  rm -f "$WEBKIT_TRACE_CONTROL"
}
trap cleanup EXIT

log() {
  printf '%s\n' "$*" | tee -a "$HARNESS_LOG"
}

fail() {
  log "FAIL: $*"
  exit 1
}

require_path() {
  [ -e "$1" ] || fail "missing path: $1"
}

require_executable() {
  [ -x "$1" ] || fail "missing executable: $1"
}

run_cell() {
  local name="$1"
  local start_x="$2"
  local y="$3"
  local end_x="$4"
  local out_summary="$LOG_DIR/$name-embedded-summary-$RUN_ID.json"
  local geometry_trace="$LOG_DIR/$name-embedded-geometry-$RUN_ID.log"
  local copy_trace="$LOG_DIR/$name-embedded-copy-$RUN_ID.log"
  local webkit_trace="$LOG_DIR/$name-webkit-selection-$RUN_ID.jsonl"

  rm -rf "$EXP44_LOG_DIR"
  printf '%s\n' "$webkit_trace" >"$WEBKIT_TRACE_CONTROL"
  log "embedded_cell=$name ratios=${start_x},${y}-${end_x},${y} webkit_trace=$webkit_trace"

  if TERMSURF_ISSUE834_PDF_FIXTURE_MODE=separated-tokens \
    TERMSURF_ISSUE834_PDF_EXPECTED_TOKENS="$EXPECTED_TEXT" \
    TERMSURF_ISSUE834_PDF_ACCEPTED_SUBSTRING="RIGHT834" \
    TERMSURF_ISSUE834_PDF_COPY_DRAG_START_X_RATIO="$start_x" \
    TERMSURF_ISSUE834_PDF_COPY_DRAG_END_X_RATIO="$end_x" \
    TERMSURF_ISSUE834_PDF_COPY_DRAG_Y_RATIO="$y" \
    TERMSURF_ISSUE834_PDF_COPY_DELAY_AFTER_DRAG=0.25 \
    TERMSURF_SURFARI="$SURFARI" \
    TERMSURF_WEBKIT_DEBUG="$WEBKIT_DEBUG" \
    TERMSURF_SURFARI_USE_LOCAL_WEBKIT_ENV=1 \
    DYLD_FRAMEWORK_PATH="$WEBKIT_DEBUG" \
    __XPC_DYLD_FRAMEWORK_PATH="$WEBKIT_DEBUG" \
    __XPC_DYLD_LIBRARY_PATH="$WEBKIT_DEBUG" \
    TERMSURF_SURFARI_PDF_COPY_TRACE=1 \
    TERMSURF_SURFARI_PDF_COPY_TRACE_FILE="$copy_trace" \
    TERMSURF_SURFARI_PDF_COPY_DIRECT=1 \
    TERMSURF_SURFARI_PDF_VIEW_GEOMETRY_TRACE=1 \
    TERMSURF_SURFARI_PDF_VIEW_GEOMETRY_TRACE_FILE="$geometry_trace" \
    TERMSURF_WEBKIT_PDF_SELECTION_TRACE=1 \
    TERMSURF_WEBKIT_PDF_SELECTION_TRACE_FILE="$webkit_trace" \
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
  rm -f "$WEBKIT_TRACE_CONTROL"

  if [ -f "$EXP44_LOG_DIR/surfari-pdf-selection-copy-summary.json" ]; then
    python3 - "$EXP44_LOG_DIR/surfari-pdf-selection-copy-summary.json" "$out_summary" "$name" "$start_x" "$y" "$end_x" "$geometry_trace" "$copy_trace" "$webkit_trace" <<'PY'
import json
import sys
from pathlib import Path

source, target, name, start_x, y, end_x, geometry_trace, copy_trace, webkit_trace = sys.argv[1:10]
data = json.loads(Path(source).read_text())
data["exp58_cell"] = {
    "name": name,
    "drag_ratios": {
        "start_x": float(start_x),
        "end_x": float(end_x),
        "y": float(y),
    },
    "copy_route": "external-cmd-c-plus-direct-probe",
    "geometry_trace": geometry_trace,
    "copy_trace": copy_trace,
    "webkit_trace": webkit_trace,
}
Path(target).write_text(json.dumps(data, indent=2, sort_keys=True) + "\n")
PY
  else
    python3 - "$out_summary" "$name" "$start_x" "$y" "$end_x" "$geometry_trace" "$copy_trace" "$webkit_trace" <<'PY'
import json
import sys
from pathlib import Path

target, name, start_x, y, end_x, geometry_trace, copy_trace, webkit_trace = sys.argv[1:9]
Path(target).write_text(json.dumps({
    "overall_result": "missing",
    "classification": "missing-summary",
    "exp58_cell": {
        "name": name,
        "drag_ratios": {
            "start_x": float(start_x),
            "end_x": float(end_x),
            "y": float(y),
        },
        "copy_route": "external-cmd-c-plus-direct-probe",
        "geometry_trace": geometry_trace,
        "copy_trace": copy_trace,
        "webkit_trace": webkit_trace,
    },
}, indent=2, sort_keys=True) + "\n")
PY
  fi
}

classify() {
  python3 - "$SUMMARY" "$RUN_ID" "$ORACLE_SUMMARY" "$CALIBRATION_SUMMARY" "$LOG_DIR" "$HARNESS_LOG" "$SURFARI" "$WEBKIT_DEBUG" <<'PY'
import json
import sys
from pathlib import Path

summary_path = Path(sys.argv[1])
run_id = sys.argv[2]
oracle_path = Path(sys.argv[3])
calibration_path = Path(sys.argv[4])
log_dir = Path(sys.argv[5])
harness_log = Path(sys.argv[6])
surfari = Path(sys.argv[7])
webkit_debug = Path(sys.argv[8])
root = summary_path.parents[2]
expected = ["LEFT834", "MID834", "RIGHT834"]
cell_names = {"oracle-base", "oracle-x-tight", "oracle-x-wide"}

def load(path):
    path = Path(path)
    return json.loads(path.read_text()) if path.exists() else None

def tokens_in(value):
    return [token for token in expected if token in (value or "")]

def read_records(path):
    path = Path(path)
    records = []
    if not path.exists():
        return records
    for line in path.read_text(errors="replace").splitlines():
        if not line.strip():
            continue
        try:
            records.append(json.loads(line))
        except json.JSONDecodeError as error:
            records.append({"parse_error": str(error), "raw": line[:1000]})
    return records

def selection_strings(value):
    found = []
    if isinstance(value, dict):
        string = value.get("selection_string")
        if isinstance(string, str):
            found.append(string)
        for child in value.values():
            found.extend(selection_strings(child))
    elif isinstance(value, list):
        for child in value:
            found.extend(selection_strings(child))
    return found

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

repo_surfari = root / "target/debug/surfari"
repo_webkit_debug = root / "webkit/src/WebKitBuild/Debug"
repo_stack_proven = surfari == repo_surfari and webkit_debug == repo_webkit_debug and (webkit_debug / "WebKit.framework").exists()

cells = []
all_records = []
missing = []
for path in sorted(log_dir.glob(f"*-embedded-summary-{run_id}.json")):
    data = load(path) or {}
    cell = data.get("exp58_cell", {})
    name = cell.get("name")
    standalone = standalone_by_name.get(name)
    clipboard = data.get("clipboard", {})
    clipboard_sample = clipboard.get("after_copy_sample", "")
    fallback_sample = clipboard.get("fallback_select_all_after_sample", "")
    records = read_records(cell.get("webkit_trace", ""))
    all_records.extend(records)
    strings = [string for record in records for string in selection_strings(record)]
    string_token_sets = [tokens_in(string) for string in strings]
    max_internal_tokens = max(string_token_sets, key=len) if string_token_sets else []
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
        name in cell_names
        and standalone
        and standalone.get("clipboard", {}).get("contains_all_tokens") is True
        and standalone.get("drag_ratios") == cell.get("drag_ratios")
    )
    if data.get("overall_result") == "missing":
        missing.append(name)
    cells.append({
        "path": str(path),
        "name": name,
        "drag_ratios": cell.get("drag_ratios"),
        "matched_standalone_gate": matched,
        "fixture_identity_match": fixture_match,
        "clipboard_tokens": tokens_in(clipboard_sample),
        "fallback_tokens": tokens_in(fallback_sample),
        "contains_all_tokens": all(token in clipboard_sample for token in expected),
        "webkit_trace": cell.get("webkit_trace"),
        "webkit_record_count": len(records),
        "webkit_plugins": sorted({record.get("plugin") for record in records if record.get("plugin")}),
        "webkit_events": sorted({record.get("event") for record in records if record.get("event")}),
        "webkit_selection_strings_sample": strings[:12],
        "webkit_max_internal_tokens": max_internal_tokens,
        "binaries": data.get("binaries", {}),
    })

plugins = sorted({record.get("plugin") for record in all_records if record.get("plugin")})
events = {record.get("event") for record in all_records if record.get("event")}
active_path_records = [
    record for record in all_records
    if record.get("source") == "termsurf-webkit-pdf-selection"
    and record.get("plugin") in {"unified", "legacy"}
]
selection_records = [
    record for record in all_records
    if selection_strings(record)
]
internal_strings = [string for record in all_records for string in selection_strings(record)]
internal_token_sets = [tokens_in(string) for string in internal_strings]
max_internal_tokens = max(internal_token_sets, key=len) if internal_token_sets else []
clipboard_token_sets = [cell["clipboard_tokens"] for cell in cells]
max_clipboard_tokens = max(clipboard_token_sets, key=len) if clipboard_token_sets else []

matched_cells = len(cells) == 3 and all(cell["matched_standalone_gate"] for cell in cells)
fixture_identity_match = bool(cells) and all(cell["fixture_identity_match"] for cell in cells)
trace_nonempty = bool(active_path_records)
tracking_trace = bool(
    events
    & {
        "beginTrackingSelection-enter",
        "continueTrackingSelection-enter",
        "continueTrackingSelection-after",
        "setCurrentSelection",
        "performCopyEditingOperation-before",
        "performCopyEditingOperation-after",
    }
)

if missing or not oracle_gate_open or not calibration_gate_open or not matched_cells or not fixture_identity_match or not repo_stack_proven or not trace_nonempty:
    overall_result = "fail"
    classification = "harness-insufficient"
elif len(max_internal_tokens) == 3 and len(max_clipboard_tokens) < 3:
    overall_result = "pass"
    classification = "webkit-copy-routing-gap"
elif internal_strings and len(max_internal_tokens) < 3:
    overall_result = "pass"
    classification = "webkit-selection-left-only"
elif tracking_trace:
    overall_result = "partial"
    classification = "webkit-plugin-path-identified"
else:
    overall_result = "partial"
    classification = "webkit-plugin-path-identified"

summary = {
    "overall_result": overall_result,
    "classification": classification,
    "run_id": run_id,
    "oracle_gate_open": oracle_gate_open,
    "calibration_gate_open": calibration_gate_open,
    "matched_cells": matched_cells,
    "fixture_identity_match": fixture_identity_match,
    "repo_stack_proven": repo_stack_proven,
    "repo_stack": {
        "surfari": str(surfari),
        "expected_surfari": str(repo_surfari),
        "webkit_debug": str(webkit_debug),
        "expected_webkit_debug": str(repo_webkit_debug),
        "webkit_framework_exists": (webkit_debug / "WebKit.framework").exists(),
    },
    "trace_nonempty": trace_nonempty,
    "trace_record_count": len(active_path_records),
    "trace_plugins": plugins,
    "trace_events": sorted(events),
    "tracking_trace": tracking_trace,
    "selection_record_count": len(selection_records),
    "max_internal_tokens": max_internal_tokens,
    "max_clipboard_tokens": max_clipboard_tokens,
    "missing_cells": missing,
    "cells": cells,
    "artifacts": {
        "harness_log": str(harness_log),
        "oracle_summary": str(oracle_path),
        "calibration_summary": str(calibration_path),
    },
}
summary_path.write_text(json.dumps(summary, indent=2, sort_keys=True) + "\n")
print(json.dumps(summary, indent=2, sort_keys=True))
PY
}

require_executable "$SURFARI"
require_path "$WEBKIT_DEBUG/WebKit.framework"
require_path "$ORACLE_SUMMARY"
require_path "$CALIBRATION_SUMMARY"

log "run_id=$RUN_ID"
log "surfari=$SURFARI"
log "webkit_debug=$WEBKIT_DEBUG"
log "oracle_summary=$ORACLE_SUMMARY"
log "calibration_summary=$CALIBRATION_SUMMARY"

run_cell "oracle-base" "0.18" "0.25" "0.86"
run_cell "oracle-x-tight" "0.20" "0.25" "0.82"
run_cell "oracle-x-wide" "0.16" "0.25" "0.90"
classify

log "summary=$SUMMARY"
