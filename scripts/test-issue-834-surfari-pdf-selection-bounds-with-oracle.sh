#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RUN_ID="$(date +%Y%m%d-%H%M%S)"
LOG_DIR="$ROOT/logs/issue-834-exp51-surfari-pdf-selection-bounds-with-oracle"
EXP49_LOG_DIR="$ROOT/logs/issue-834-exp49-surfari-pdf-selection-bounds"
SUMMARY="$LOG_DIR/surfari-pdf-selection-bounds-with-oracle-summary.json"
HARNESS_LOG="$LOG_DIR/harness-$RUN_ID.log"
ORACLE_SUMMARY="${TERMSURF_ISSUE834_EXP50_ORACLE_SUMMARY:-$ROOT/logs/issue-834-exp50-separated-token-copy-oracle/separated-token-copy-oracle-summary.json}"

mkdir -p "$LOG_DIR"

log() {
  printf '%s\n' "$*" | tee -a "$HARNESS_LOG"
}

run_embedded_matrix() {
  rm -rf "$EXP49_LOG_DIR"
  "$ROOT/scripts/test-issue-834-surfari-pdf-selection-bounds.sh" >>"$HARNESS_LOG" 2>&1
  if [ -f "$EXP49_LOG_DIR/surfari-pdf-selection-bounds-summary.json" ]; then
    cp "$EXP49_LOG_DIR/surfari-pdf-selection-bounds-summary.json" "$LOG_DIR/exp49-summary-$RUN_ID.json"
  else
    python3 - "$LOG_DIR/exp49-summary-$RUN_ID.json" <<'PY'
import json
import sys
from pathlib import Path
Path(sys.argv[1]).write_text(json.dumps({"overall_result": "missing"}, indent=2) + "\n")
PY
  fi
}

classify_with_oracle() {
  python3 - "$SUMMARY" "$RUN_ID" "$ORACLE_SUMMARY" "$LOG_DIR/exp49-summary-$RUN_ID.json" <<'PY'
import json
import sys
from pathlib import Path

summary_path = Path(sys.argv[1])
run_id = sys.argv[2]
oracle_path = Path(sys.argv[3])
embedded_path = Path(sys.argv[4])
expected = ["LEFT834", "MID834", "RIGHT834"]

def load(path):
    if not path.exists():
        return None
    return json.loads(path.read_text())

oracle = load(oracle_path)
embedded = load(embedded_path)

def oracle_open(data):
    if not data:
        return False
    if data.get("classification") != "separated-token-oracle-pass":
        return False
    if data.get("embedded_interpretation_gate") != "open":
        return False
    controls = data.get("controls", {})
    for control in ("pdfkit", "wkpdf"):
        routes = controls.get(control, {})
        if not any(route.get("contains_all_tokens") for route in routes.values()):
            return False
    return True

def embedded_fixture_identity(cell):
    fixture = cell.get("fixture", {})
    bboxes = fixture.get("pdf_text_bboxes")
    operators = fixture.get("pdf_text_operator")
    extracted = fixture.get("text_extracted")
    return {
        "page_geometry": fixture.get("page_geometry"),
        "font": fixture.get("font"),
        "operator_summary": operators,
        "token_boxes": bboxes,
        "extracted_text": extracted,
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
if embedded:
    for cell in embedded.get("cells", []):
        path = Path(cell.get("path", ""))
        detail = load(path)
        combined = dict(cell)
        if detail:
            combined["fixture"] = detail.get("fixture", {})
            combined["coordinate_mapping"] = detail.get("coordinate_mapping", {})
            combined["artifacts"] = detail.get("artifacts", {})
        cells.append(combined)

missing = [cell for cell in cells if cell.get("overall_result") == "missing"]
restore_status = embedded.get("clipboard_restore_status") if embedded else None
gate_open = oracle_open(oracle)
fixture_match, fixture_match_reason = fixture_matches(oracle, cells)

baseline_all = [cell for cell in cells if not cell.get("cell", {}).get("direct_copy") and cell.get("contains_all_tokens")]
direct_all = [cell for cell in cells if cell.get("cell", {}).get("direct_copy") and cell.get("contains_all_tokens")]
direct_with_left_mid = [
    cell for cell in cells
    if cell.get("cell", {}).get("direct_copy")
    and "LEFT834" in cell.get("tokens_present", [])
    and "MID834" in cell.get("tokens_present", [])
]
any_right = any("RIGHT834" in cell.get("tokens_present", []) for cell in cells)
direct_names = {cell.get("cell", {}).get("name") for cell in direct_with_left_mid}
required_names = {"all-ltr", "all-rtl", "all-y-high", "all-y-low", "overwide-delay-025", "overwide-delay-100", "overwide-delay-200"}
right_edge_recurrence = required_names.issubset(direct_names)
targeting_evidence = []
for cell in direct_with_left_mid:
    detail_cell = cell.get("cell", {})
    name = detail_cell.get("name", "")
    if name in required_names:
        coords = cell.get("coordinate_mapping", {})
        end = coords.get("web_drag_end", {})
        start = coords.get("web_drag_start", {})
        if max(float(start.get("x", 0)), float(end.get("x", 0))) >= 900:
            targeting_evidence.append(name)
targeting_complete = required_names.issubset(set(targeting_evidence))

if embedded is None or embedded.get("overall_result") == "missing":
    classification = "harness-insufficient"
    result = "partial"
elif restore_status != "restored" or missing:
    classification = "harness-insufficient"
    result = "fail" if restore_status != "restored" else "partial"
elif not gate_open:
    classification = "oracle-not-open"
    result = "partial"
elif not fixture_match:
    classification = "fixture-identity-gap"
    result = "partial"
elif baseline_all:
    classification = "embedded-geometry-fix-candidate"
    result = "pass"
elif direct_all:
    classification = "embedded-direct-copy-fix-candidate"
    result = "pass"
elif direct_with_left_mid and not any_right and right_edge_recurrence and targeting_complete:
    classification = "embedded-right-edge-selection-gap"
    result = "pass"
elif direct_with_left_mid and not any_right:
    classification = "embedded-right-edge-candidate"
    result = "pass"
else:
    classification = "embedded-copy-routing-gap"
    result = "pass"

summary = {
    "overall_result": result,
    "classification": classification,
    "run_id": run_id,
    "oracle_summary": str(oracle_path),
    "embedded_summary": str(embedded_path),
    "oracle_gate_open": gate_open,
    "fixture_identity_match": fixture_match,
    "fixture_identity_match_reason": fixture_match_reason,
    "clipboard_restore_status": restore_status,
    "expected_tokens": expected,
    "cell_count": len(cells),
    "right_edge_recurrence": right_edge_recurrence,
    "right_edge_required_cells": sorted(required_names),
    "right_edge_cells_with_left_mid": sorted(name for name in direct_names if name),
    "targeting_evidence_cells": sorted(targeting_evidence),
    "targeting_complete": targeting_complete,
    "any_right_token_copied": any_right,
    "cells": cells,
}
summary_path.write_text(json.dumps(summary, indent=2, sort_keys=True) + "\n")
print(json.dumps({
    "overall_result": result,
    "classification": classification,
    "oracle_gate_open": gate_open,
    "fixture_identity_match": fixture_match,
    "right_edge_recurrence": right_edge_recurrence,
    "targeting_complete": targeting_complete,
}, indent=2, sort_keys=True))
if result == "fail":
    sys.exit(1)
PY
}

log "run_id=$RUN_ID"
log "oracle_summary=$ORACLE_SUMMARY"
run_embedded_matrix
classify_with_oracle
log "summary=$SUMMARY"
