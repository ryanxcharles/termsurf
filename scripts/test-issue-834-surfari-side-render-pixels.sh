#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RUN_ID="$(date +%Y%m%d-%H%M%S)"
LOG_DIR="$ROOT/logs/issue-834-exp37-surfari-side-render-pixels"
EXP36_LOG_DIR="$ROOT/logs/issue-834-exp36-surfari-visual-compositing"
HARNESS_LOG="$LOG_DIR/harness-$RUN_ID.log"
RENDER_PROOF_TRACE="$LOG_DIR/surfari-render-proof-$RUN_ID.log"
SUMMARY="$LOG_DIR/surfari-side-render-pixels-summary.json"

mkdir -p "$LOG_DIR"

log() {
  printf '%s\n' "$*" | tee -a "$HARNESS_LOG"
}

fail() {
  log "FAIL: $*"
  exit 1
}

require_executable() {
  [ -x "$1" ] || fail "missing executable: $1"
}

require_path() {
  [ -e "$1" ] || fail "missing path: $1"
}

require_executable "$ROOT/scripts/test-issue-834-surfari-visual-compositing.sh"
require_executable "${TERMSURF_GHOSTBOARD_APP:-$ROOT/ghostboard/macos/build/Debug/TermSurf.app}/Contents/MacOS/termsurf"
require_executable "${TERMSURF_WEB:-$ROOT/target/debug/web}"
require_executable "${TERMSURF_SURFARI:-$ROOT/target/debug/surfari}"
require_path "${TERMSURF_WEBKIT_DEBUG:-$ROOT/webkit/src/WebKitBuild/Debug}/WebKit.framework"
require_path "$ROOT/surfari/libtermsurf_webkit/build/libtermsurf_webkit.dylib"

log "run_id=$RUN_ID"
log "render_proof_trace=$RENDER_PROOF_TRACE"
log "summary=$SUMMARY"

rm -rf "$EXP36_LOG_DIR"
TERMSURF_SURFARI_RENDER_PROOF_TRACE_FILE="$RENDER_PROOF_TRACE" \
  "$ROOT/scripts/test-issue-834-surfari-visual-compositing.sh" 2>&1 | tee -a "$HARNESS_LOG"

EXP36_SUMMARY="$EXP36_LOG_DIR/surfari-visual-compositing-summary.json"
[ -s "$EXP36_SUMMARY" ] || fail "missing Experiment 36 summary: $EXP36_SUMMARY"
[ -s "$RENDER_PROOF_TRACE" ] || fail "missing Surfari render proof trace: $RENDER_PROOF_TRACE"

python3 - "$SUMMARY" "$RUN_ID" "$HARNESS_LOG" "$RENDER_PROOF_TRACE" "$EXP36_SUMMARY" <<'PY'
from pathlib import Path
import json
import re
import sys

summary_path = Path(sys.argv[1])
run_id = sys.argv[2]
harness_log = sys.argv[3]
render_trace = Path(sys.argv[4])
exp36_summary_path = Path(sys.argv[5])
exp36 = json.loads(exp36_summary_path.read_text())

line_re = re.compile(
    r"render-proof tab=(?P<tab>\d+) pane=(?P<pane>\S+) url=(?P<url>\S+) "
    r"method=(?P<method>\S+) status=(?P<status>\S+) width=(?P<width>\d+) "
    r"height=(?P<height>\d+) magenta=(?P<magenta>\d+) cyan=(?P<cyan>\d+) "
    r"yellow=(?P<yellow>\d+) webkit_green=(?P<webkit_green>\d+) error=(?P<error>.*)$"
)

proofs = []
for line in render_trace.read_text(encoding="utf-8", errors="replace").splitlines():
    match = line_re.search(line)
    if not match:
        continue
    row = match.groupdict()
    for key in ("tab", "width", "height", "magenta", "cyan", "yellow", "webkit_green"):
        row[key] = int(row[key])
    proofs.append(row)

def pick(name):
    scenario = exp36.get(name, {})
    url = scenario.get("url", "")
    matches = [proof for proof in proofs if proof.get("url") == url]
    proof = matches[-1] if matches else None
    return {
        **scenario,
        "internal_render": proof,
    }

html = pick("html")
pdf = pick("pdf")
html_internal = html.get("internal_render")
pdf_internal = pdf.get("internal_render")

def proof_pass(proof, names):
    if not proof or proof.get("status") != "pass":
        return False
    return any(proof.get(name, 0) >= 5000 for name in names)

html_pass = proof_pass(html_internal, ("magenta", "cyan", "yellow"))
pdf_pass = proof_pass(pdf_internal, ("webkit_green",))
ghostboard_blank = (
    exp36.get("html", {}).get("pixel_status") == "fail"
    and exp36.get("pdf", {}).get("pixel_status") == "fail"
)

if html_pass and pdf_pass and ghostboard_blank:
    overall = "pass"
    classification = "ghostboard-compositing-gap"
elif html_pass and not pdf_pass:
    overall = "pass"
    classification = "webkit-pdf-render-gap"
elif not html_internal or not pdf_internal:
    overall = "partial"
    classification = "capture-api-unsupported"
elif html_internal.get("status") in {"capture-failed", "unsupported"} or pdf_internal.get("status") in {"capture-failed", "unsupported"}:
    overall = "partial"
    classification = "capture-api-unsupported"
elif not html_pass and not pdf_pass:
    overall = "partial"
    classification = "unvalidated-blank-internal-capture"
else:
    overall = "partial"
    classification = "inconclusive"

data = {
    "overall_result": overall,
    "classification": classification,
    "run_id": run_id,
    "artifacts": {
        "harness_log": harness_log,
        "render_proof_trace": str(render_trace),
        "exp36_summary": str(exp36_summary_path),
    },
    "html": html,
    "pdf": pdf,
    "ghostboard_visible": {
        "html_pixel_status": exp36.get("html", {}).get("pixel_status"),
        "pdf_pixel_status": exp36.get("pdf", {}).get("pixel_status"),
    },
    "cleanup": exp36.get("cleanup", {}),
}
summary_path.write_text(json.dumps(data, indent=2, sort_keys=True) + "\n")
print(json.dumps({
    "overall_result": overall,
    "classification": classification,
    "html_internal": html_internal,
    "pdf_internal": pdf_internal,
    "ghostboard_visible": data["ghostboard_visible"],
}, indent=2, sort_keys=True))
PY

log "PASS: issue 834 experiment 37 Surfari-side render diagnostics"
log "summary=$SUMMARY"
