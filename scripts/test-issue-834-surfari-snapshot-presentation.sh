#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RUN_ID="$(date +%Y%m%d-%H%M%S)"
LOG_DIR="$ROOT/logs/issue-834-exp39-surfari-snapshot-presentation"
EXP37_LOG_DIR="$ROOT/logs/issue-834-exp37-surfari-side-render-pixels"
EXP36_LOG_DIR="$ROOT/logs/issue-834-exp36-surfari-visual-compositing"
HARNESS_LOG="$LOG_DIR/harness-$RUN_ID.log"
SUMMARY="$LOG_DIR/surfari-snapshot-presentation-summary.json"
APP_BIN="${TERMSURF_GHOSTBOARD_APP:-$ROOT/ghostboard/macos/build/Debug/TermSurf.app}/Contents/MacOS/termsurf"
WEB="${TERMSURF_WEB:-$ROOT/target/debug/web}"
SURFARI="${TERMSURF_SURFARI:-$ROOT/target/debug/surfari}"
WEBKIT_DEBUG="${TERMSURF_WEBKIT_DEBUG:-$ROOT/webkit/src/WebKitBuild/Debug}"

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

require_executable "$ROOT/scripts/test-issue-834-surfari-side-render-pixels.sh"
require_executable "$APP_BIN"
require_executable "$WEB"
require_executable "$SURFARI"

log "run_id=$RUN_ID"
log "ghostboard_app_bin=$APP_BIN"
log "web=$WEB"
log "surfari=$SURFARI"
log "webkit_debug=$WEBKIT_DEBUG"
log "termsurf_surfari_cacontext_layer=${TERMSURF_SURFARI_CACONTEXT_LAYER-__unset__}"
log "summary=$SUMMARY"

if [ "${TERMSURF_SURFARI_CACONTEXT_LAYER+x}" = "x" ]; then
  fail "TERMSURF_SURFARI_CACONTEXT_LAYER must be unset for default presentation proof"
fi

rm -rf "$EXP36_LOG_DIR" "$EXP37_LOG_DIR"
env -u TERMSURF_SURFARI_CACONTEXT_LAYER \
  "$ROOT/scripts/test-issue-834-surfari-side-render-pixels.sh" 2>&1 | tee -a "$HARNESS_LOG"

EXP37_SUMMARY="$EXP37_LOG_DIR/surfari-side-render-pixels-summary.json"
[ -s "$EXP37_SUMMARY" ] || fail "missing Experiment 37 summary: $EXP37_SUMMARY"

python3 - "$SUMMARY" "$RUN_ID" "$HARNESS_LOG" "$EXP37_SUMMARY" "$APP_BIN" "$WEB" "$SURFARI" "$WEBKIT_DEBUG" <<'PY'
from pathlib import Path
import json
import sys

summary_path = Path(sys.argv[1])
run_id = sys.argv[2]
harness_log = sys.argv[3]
exp37_summary_path = Path(sys.argv[4])
app_bin = sys.argv[5]
web = sys.argv[6]
surfari = sys.argv[7]
webkit_debug = sys.argv[8]
exp37 = json.loads(exp37_summary_path.read_text())

def window_pass(scenario, targets):
    proof = scenario.get("pixel_proof", {}).get("window", {})
    counts = proof.get("targets", {})
    return all(counts.get(name, 0) >= 5000 for name in targets)

def refresh_reasons(scenario):
    app_log = scenario.get("artifacts", {}).get("app_log")
    if not app_log or not Path(app_log).exists():
        return []
    reasons = []
    for line in Path(app_log).read_text(errors="replace").splitlines():
        marker = "snapshot-layer-refresh reason="
        if marker in line:
            reasons.append(line.split(marker, 1)[1].split()[0])
    return reasons

html = exp37.get("html", {})
pdf = exp37.get("pdf", {})
html_visible = window_pass(html, ("cyan", "yellow"))
pdf_visible = window_pass(pdf, ("webkit_green",))
html_internal_ok = html.get("internal_render", {}).get("status") == "pass"
pdf_internal_ok = pdf.get("internal_render", {}).get("status") == "pass"
html_refresh = refresh_reasons(html)
pdf_refresh = refresh_reasons(pdf)
refresh_seen = bool(html_refresh or pdf_refresh)

if html_visible and pdf_visible and html_internal_ok and pdf_internal_ok and refresh_seen:
    overall = "partial"
    classification = "default-snapshot-visible-refresh-deltas-unproven"
elif html_visible and pdf_visible:
    overall = "partial"
    classification = "visible-without-refresh-trace"
else:
    overall = "fail"
    classification = "default-snapshot-not-visible"

data = {
    "overall_result": overall,
    "classification": classification,
    "run_id": run_id,
    "termsurf_surfari_cacontext_layer": "unset",
    "default_export_method": "snapshot-backed",
    "artifacts": {
        "harness_log": harness_log,
        "exp37_summary": str(exp37_summary_path),
    },
    "binaries": {
        "ghostboard_app_bin": app_bin,
        "web": web,
        "surfari": surfari,
        "webkit_debug": webkit_debug,
    },
    "html": {
        **html,
        "visible_pixel_proof": html.get("pixel_proof", {}).get("window"),
        "visible_window_bounded": True,
        "source_window_excluded": True,
        "snapshot_refresh_reasons": html_refresh,
    },
    "pdf": {
        **pdf,
        "visible_pixel_proof": pdf.get("pixel_proof", {}).get("window"),
        "visible_window_bounded": True,
        "source_window_excluded": True,
        "snapshot_refresh_reasons": pdf_refresh,
    },
    "cleanup": exp37.get("cleanup", {}),
}
summary_path.write_text(json.dumps(data, indent=2, sort_keys=True) + "\n")
print(json.dumps({
    "overall_result": overall,
    "classification": classification,
    "env": "unset",
    "default_export_method": "snapshot-backed",
    "html_visible": html_visible,
    "pdf_visible": pdf_visible,
    "html_internal": html_internal_ok,
    "pdf_internal": pdf_internal_ok,
    "html_refresh_reasons": html_refresh,
    "pdf_refresh_reasons": pdf_refresh,
}, indent=2, sort_keys=True))
PY

log "PASS: issue 834 experiment 39 Surfari snapshot presentation diagnostics"
log "summary=$SUMMARY"
