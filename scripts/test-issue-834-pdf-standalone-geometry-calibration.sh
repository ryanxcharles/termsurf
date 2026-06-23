#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RUN_ID="$(date +%Y%m%d-%H%M%S)"
LOG_DIR="$ROOT/logs/issue-834-exp54-pdf-standalone-geometry-calibration"
RUN_DIR="$(mktemp -d "${TMPDIR:-/tmp}/termsurf-issue834-exp54.XXXXXX")"
SITE_DIR="$RUN_DIR/site"
PDF_PATH="$SITE_DIR/separated-tokens.pdf"
SWIFT_APP="$RUN_DIR/GeometryProbe.swift"
PROBE_APP="$RUN_DIR/geometry-probe"
SUMMARY="$LOG_DIR/pdf-standalone-geometry-calibration-summary.json"
HARNESS_LOG="$LOG_DIR/harness-$RUN_ID.log"
ORACLE_SUMMARY="${TERMSURF_ISSUE834_EXP50_ORACLE_SUMMARY:-$ROOT/logs/issue-834-exp50-separated-token-copy-oracle/separated-token-copy-oracle-summary.json}"
EXP44_LOG_DIR="$ROOT/logs/issue-834-exp44-surfari-pdf-selection-copy"
EMBEDDED_SUMMARY="$LOG_DIR/embedded-summary-$RUN_ID.json"
EMBEDDED_TRACE="$LOG_DIR/embedded-surfari-geometry-$RUN_ID.log"
EMBEDDED_COPY_TRACE="$LOG_DIR/embedded-surfari-copy-$RUN_ID.log"
ORIGINAL_CLIPBOARD="$RUN_DIR/original-clipboard.txt"
ORIGINAL_RESTORE_STATUS="not-attempted"
CURRENT_PID=""

EXPECTED_TEXT="LEFT834 MID834 RIGHT834"
EXPECTED_TOKENS=("LEFT834" "MID834" "RIGHT834")
PDF_TEXT_OPERATORS='BT /F1 24 Tf 72 620 Td (LEFT834) Tj ET | BT /F1 24 Tf 220 620 Td (MID834) Tj ET | BT /F1 24 Tf 360 620 Td (RIGHT834) Tj ET'
PDF_TEXT_BBOXES_JSON='[{"token":"LEFT834","x":72,"y":604,"width":96,"height":32},{"token":"MID834","x":220,"y":604,"width":84,"height":32},{"token":"RIGHT834","x":360,"y":604,"width":108,"height":32}]'

mkdir -p "$LOG_DIR" "$SITE_DIR"

log() {
  printf '%s\n' "$*" | tee -a "$HARNESS_LOG"
}

delay() {
  sleep "${1:-0.5}"
}

restore_original_clipboard() {
  if [ -e "$ORIGINAL_CLIPBOARD" ]; then
    pbcopy <"$ORIGINAL_CLIPBOARD" || return 1
    ORIGINAL_RESTORE_STATUS="restored"
  fi
}

cleanup_current_process() {
  if [ -n "${CURRENT_PID:-}" ] && kill -0 "$CURRENT_PID" >/dev/null 2>&1; then
    kill "$CURRENT_PID" >/dev/null 2>&1 || true
    delay 0.2 || true
    kill -9 "$CURRENT_PID" >/dev/null 2>&1 || true
  fi
  CURRENT_PID=""
}

cleanup() {
  cleanup_current_process || true
  restore_original_clipboard || ORIGINAL_RESTORE_STATUS="restore-failed"
  rm -rf "$RUN_DIR"
}
trap cleanup EXIT

fail() {
  log "FAIL: $*"
  cleanup
  exit 1
}

hash_file() {
  shasum -a 256 "$1" | awk '{print $1}'
}

pasteboard_change_count() {
  swift - <<'SWIFT'
import AppKit
print(NSPasteboard.general.changeCount)
SWIFT
}

exact_window_bounds() {
  local window_id="$1"
  swift - "$window_id" <<'SWIFT'
import CoreGraphics
import Foundation

let target = Int(CommandLine.arguments[1])!
guard let info = CGWindowListCopyWindowInfo([.optionAll], kCGNullWindowID) as? [[String: Any]] else {
    exit(1)
}

for window in info {
    guard let id = window[kCGWindowNumber as String] as? Int, id == target else { continue }
    let bounds = (window[kCGWindowBounds as String] as? [String: Any]) ?? [:]
    let x = Int((bounds["X"] as? Double) ?? 0)
    let y = Int((bounds["Y"] as? Double) ?? 0)
    let width = Int((bounds["Width"] as? Double) ?? 0)
    let height = Int((bounds["Height"] as? Double) ?? 0)
    print("\(id)\t\(x)\t\(y)\t\(width)\t\(height)")
    exit(0)
}

exit(1)
SWIFT
}

point_from_window_ratio() {
  local win_line="$1"
  local rx="$2"
  local ry="$3"
  python3 - "$win_line" "$rx" "$ry" <<'PY'
import sys

win_line, rx, ry = sys.argv[1:4]
_, x, y, w, h = win_line.split("\t")
x, y, w, h = map(float, (x, y, w, h))
rx = float(rx)
ry = float(ry)
print(f"{int(x + w * rx + 0.5)} {int(y + h * ry + 0.5)}")
PY
}

activate_pid() {
  local pid="$1"
  swift - "$pid" <<'SWIFT'
import AppKit
import Foundation

let pid = pid_t(Int32(CommandLine.arguments[1])!)
guard let app = NSRunningApplication(processIdentifier: pid) else {
    exit(1)
}
app.activate(options: [.activateAllWindows, .activateIgnoringOtherApps])
SWIFT
}

slow_drag() {
  local sx="$1"
  local sy="$2"
  local ex="$3"
  local ey="$4"
  swift - "$sx" "$sy" "$ex" "$ey" <<'SWIFT'
import CoreGraphics
import Foundation

func point(_ x: String, _ y: String) -> CGPoint {
    CGPoint(x: Double(x) ?? 0, y: Double(y) ?? 0)
}

func event(_ type: CGEventType, _ point: CGPoint) -> CGEvent? {
    CGEvent(mouseEventSource: nil, mouseType: type, mouseCursorPosition: point, mouseButton: .left)
}

let start = point(CommandLine.arguments[1], CommandLine.arguments[2])
let end = point(CommandLine.arguments[3], CommandLine.arguments[4])
event(.mouseMoved, start)?.post(tap: .cghidEventTap)
usleep(50_000)
event(.leftMouseDown, start)?.post(tap: .cghidEventTap)
usleep(100_000)
for step in 1...40 {
    let t = Double(step) / 40.0
    let current = CGPoint(
        x: start.x + (end.x - start.x) * t,
        y: start.y + (end.y - start.y) * t
    )
    event(.leftMouseDragged, current)?.post(tap: .cghidEventTap)
    usleep(15_000)
}
event(.leftMouseUp, end)?.post(tap: .cghidEventTap)
SWIFT
}

write_trigger() {
  local trigger="$1"
  local ack="$2"
  shift 2
  local tmp_trigger="$trigger.tmp"
  printf '%s\n' "$*" >"$tmp_trigger"
  mv "$tmp_trigger" "$trigger"
  for _ in $(seq 1 50); do
    [ -f "$ack" ] && break
    delay 0.1
  done
  [ -f "$ack" ] || fail "standalone trigger did not ack: $*"
  rm -f "$ack"
}

write_pdf_fixture() {
  python3 - "$PDF_PATH" <<'PY'
from pathlib import Path
import sys

out = Path(sys.argv[1])
objects = []

def esc(value):
    return value.replace("\\", "\\\\").replace("(", "\\(").replace(")", "\\)")

def add(body):
    objects.append(body)

stream_lines = [
    "0 1 1 rg",
    "0 0 612 792 re",
    "f",
    "0 0 0 rg",
]
for token, (x, y) in zip(["LEFT834", "MID834", "RIGHT834"], [(72, 620), (220, 620), (360, 620)]):
    stream_lines.extend([
        "BT",
        "/F1 24 Tf",
        f"{x} {y} Td",
        f"({esc(token)}) Tj",
        "ET",
    ])
stream_lines.append("")
stream = "\n".join(stream_lines)

add("<< /Type /Catalog /Pages 2 0 R >>")
add("<< /Type /Pages /Kids [3 0 R] /Count 1 >>")
add("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>")
add(f"<< /Length {len(stream.encode('ascii'))} >>\nstream\n{stream}endstream")
add("<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>")

data = bytearray(b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n")
offsets = [0]
for index, body in enumerate(objects, start=1):
    offsets.append(len(data))
    data.extend(f"{index} 0 obj\n{body}\nendobj\n".encode("latin1"))
xref = len(data)
data.extend(f"xref\n0 {len(objects) + 1}\n".encode("ascii"))
data.extend(b"0000000000 65535 f \n")
for offset in offsets[1:]:
    data.extend(f"{offset:010d} 00000 n \n".encode("ascii"))
data.extend(
    f"trailer\n<< /Size {len(objects) + 1} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n".encode("ascii")
)
out.write_bytes(data)
PY
}

fixture_identity_json() {
  python3 - "$PDF_PATH" "$PDF_TEXT_BBOXES_JSON" <<'PY'
from pathlib import Path
import hashlib
import json
import re
import sys

path = Path(sys.argv[1])
boxes = json.loads(sys.argv[2])
text = path.read_bytes().decode("latin1", errors="ignore")
operators = []
for match in re.finditer(r"BT\s*/F1 24 Tf\s*([0-9]+) ([0-9]+) Td\s*\(([^()]*)\) Tj\s*ET", text):
    x, y, token = match.groups()
    operators.append({"token": token, "x": int(x), "y": int(y)})
identity = {
    "status": "pass",
    "pdf_path": str(path),
    "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
    "page_geometry": {"media_box": [0, 0, 612, 792]},
    "font": {"name": "Helvetica", "size": 24, "encoding": "WinAnsiEncoding"},
    "operators": operators,
    "operator_summary": "BT /F1 24 Tf 72 620 Td (LEFT834) Tj ET | BT /F1 24 Tf 220 620 Td (MID834) Tj ET | BT /F1 24 Tf 360 620 Td (RIGHT834) Tj ET",
    "token_boxes": boxes,
    "extracted_text": " ".join(operator["token"] for operator in operators),
}
print(json.dumps(identity, sort_keys=True))
PY
}

write_swift_app() {
  cat >"$SWIFT_APP" <<'SWIFT'
import AppKit
import WebKit

let pdfPath = CommandLine.arguments[1]
let tracePath = CommandLine.arguments[2]
let triggerPath = CommandLine.arguments[3]
let ackPath = CommandLine.arguments[4]

func describe(_ value: Any?) -> String {
    guard let value else { return "nil" }
    if let object = value as AnyObject? {
        return "\(type(of: object)):\(Unmanaged.passUnretained(object).toOpaque())"
    }
    return "\(type(of: value)):\(value)"
}

func appendTrace(_ line: String) {
    let url = URL(fileURLWithPath: tracePath)
    try? FileManager.default.createDirectory(at: url.deletingLastPathComponent(), withIntermediateDirectories: true)
    let data = (line + "\n").data(using: .utf8)!
    if FileManager.default.fileExists(atPath: tracePath),
       let handle = try? FileHandle(forWritingTo: url) {
        _ = try? handle.seekToEnd()
        try? handle.write(contentsOf: data)
        try? handle.close()
    } else {
        try? data.write(to: url)
    }
}

func responderChain(_ responder: NSResponder?) -> String {
    var items: [String] = []
    var current = responder
    for _ in 0..<12 {
        guard let item = current else { break }
        items.append(describe(item))
        current = item.nextResponder
    }
    return items.joined(separator: ">")
}

func viewTree(_ view: NSView?, _ depth: Int = 0) -> String {
    guard let view, depth <= 5 else { return "" }
    var items = [
        "\(type(of: view)):\(Unmanaged.passUnretained(view).toOpaque()) frame=\(NSStringFromRect(view.frame)) bounds=\(NSStringFromRect(view.bounds)) hidden=\(view.isHidden) alpha=\(view.alphaValue) layered=\(view.wantsLayer)"
    ]
    for subview in view.subviews {
        let child = viewTree(subview, depth + 1)
        if !child.isEmpty {
            items.append("[\(child)]")
        }
    }
    return items.joined(separator: " ")
}

func scrollViews(_ view: NSView?) -> String {
    guard let view else { return "" }
    var items: [String] = []
    if let scroll = view as? NSScrollView {
        items.append("\(type(of: scroll)):\(Unmanaged.passUnretained(scroll).toOpaque()) frame=\(NSStringFromRect(scroll.frame)) bounds=\(NSStringFromRect(scroll.bounds)) document=\(describe(scroll.documentView)) document_frame=\(NSStringFromRect(scroll.documentView?.frame ?? .zero)) document_bounds=\(NSStringFromRect(scroll.documentView?.bounds ?? .zero)) clip_bounds=\(NSStringFromRect(scroll.contentView.bounds))")
    }
    for subview in view.subviews {
        let child = scrollViews(subview)
        if !child.isEmpty {
            items.append(child)
        }
    }
    return items.joined(separator: " | ")
}

func clipboardSample() -> String {
    let value = NSPasteboard.general.string(forType: .string) ?? ""
    let sample = String(value.prefix(120)).replacingOccurrences(of: "\n", with: " ")
    return "len=\(value.count) change=\(NSPasteboard.general.changeCount) sample=\(sample)"
}

final class Delegate: NSObject, NSApplicationDelegate, WKNavigationDelegate {
    var window: NSWindow!
    var webView: WKWebView!
    var timer: Timer?

    func applicationDidFinishLaunching(_ notification: Notification) {
        installMenus()
        let frame = NSRect(x: 120, y: 120, width: 900, height: 650)
        window = NSWindow(
            contentRect: frame,
            styleMask: [.titled, .closable, .resizable],
            backing: .buffered,
            defer: false
        )
        window.title = "Issue 834 Exp54 WKWebView"
        webView = WKWebView(frame: NSRect(x: 0, y: 0, width: 900, height: 650))
        webView.navigationDelegate = self
        window.contentView = webView
        webView.loadFileURL(
            URL(fileURLWithPath: pdfPath),
            allowingReadAccessTo: URL(fileURLWithPath: pdfPath).deletingLastPathComponent()
        )
        window.makeKeyAndOrderFront(nil)
        window.makeFirstResponder(webView)
        NSApp.activate(ignoringOtherApps: true)
        timer = Timer.scheduledTimer(withTimeInterval: 0.1, repeats: true) { _ in
            self.checkTrigger()
        }
    }

    func installMenus() {
        let mainMenu = NSMenu()
        let appItem = NSMenuItem()
        mainMenu.addItem(appItem)
        let appMenu = NSMenu()
        appMenu.addItem(NSMenuItem(title: "Quit", action: #selector(NSApplication.terminate(_:)), keyEquivalent: "q"))
        appItem.submenu = appMenu

        let editItem = NSMenuItem()
        mainMenu.addItem(editItem)
        let editMenu = NSMenu(title: "Edit")
        let copyItem = NSMenuItem(title: "Copy", action: #selector(NSText.copy(_:)), keyEquivalent: "c")
        copyItem.keyEquivalentModifierMask = [.command]
        editMenu.addItem(copyItem)
        editItem.submenu = editMenu
        NSApp.mainMenu = mainMenu
    }

    func webView(_ webView: WKWebView, didFinish navigation: WKNavigation!) {
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.8) {
            print("READY window_id=\(self.window.windowNumber)")
            fflush(stdout)
        }
    }

    func dump(_ label: String, webX: CGFloat, webY: CGFloat) {
        let windowPoint = webView.convert(NSPoint(x: webX, y: webY), to: nil)
        let hit: NSView = webView.hitTest(windowPoint) ?? webView
        let hitLocal = hit.convert(windowPoint, from: nil)
        let selector = #selector(NSText.copy(_:))
        let targetNil = NSApp.target(forAction: selector, to: nil, from: nil)
        let targetWeb = NSApp.target(forAction: selector, to: nil, from: webView)
        appendTrace("standalone-pdf-calibration-state label=\(label) url=\(webView.url?.absoluteString ?? "") input=\(webX),\(webY) window_point=\(NSStringFromPoint(windowPoint)) web_point=\(NSStringFromPoint(NSPoint(x: webX, y: webY))) hit=\(describe(hit)) hit_local=\(NSStringFromPoint(hitLocal)) window=\(describe(window)) window_frame=\(NSStringFromRect(window.frame)) key_window=\(window.isKeyWindow ? 1 : 0) main_window=\(window.isMainWindow ? 1 : 0) app_key_window=\(describe(NSApp.keyWindow)) app_main_window=\(describe(NSApp.mainWindow)) backing_scale=\(window.screen?.backingScaleFactor ?? NSScreen.main?.backingScaleFactor ?? 1.0) web_view=\(describe(webView)) web_frame=\(NSStringFromRect(webView.frame)) web_bounds=\(NSStringFromRect(webView.bounds)) first_responder=\(describe(window.firstResponder)) responder_chain=\(responderChain(window.firstResponder)) target_nil=\(describe(targetNil)) target_webview=\(describe(targetWeb)) clipboard={\(clipboardSample())}")
        appendTrace("standalone-pdf-calibration-hit-chain label=\(label) chain=\(describe(hit)) point=\(NSStringFromPoint(hitLocal)) frame=\(NSStringFromRect(hit.frame)) bounds=\(NSStringFromRect(hit.bounds))")
        appendTrace("standalone-pdf-calibration-tree label=\(label) tree=\(viewTree(webView))")
        appendTrace("standalone-pdf-calibration-scroll label=\(label) scroll=\(scrollViews(webView))")
    }

    func checkTrigger() {
        guard FileManager.default.fileExists(atPath: triggerPath),
              let text = try? String(contentsOfFile: triggerPath, encoding: .utf8)
        else { return }
        try? FileManager.default.removeItem(atPath: triggerPath)
        let parts = text.split(separator: " ").map(String.init)
        if parts.first == "dump", parts.count >= 4 {
            dump(parts[1], webX: CGFloat(Double(parts[2]) ?? 0), webY: CGFloat(Double(parts[3]) ?? 0))
        }
        try? "ack\n".write(toFile: ackPath, atomically: true, encoding: .utf8)
    }
}

let app = NSApplication.shared
let delegate = Delegate()
app.delegate = delegate
app.setActivationPolicy(.regular)
app.run()
SWIFT
  swiftc "$SWIFT_APP" -o "$PROBE_APP"
}

run_oracle() {
  log "running separated-token oracle"
  "$ROOT/scripts/test-issue-834-separated-token-copy-oracle.sh" >>"$HARNESS_LOG" 2>&1
  [ -f "$ORACLE_SUMMARY" ] || fail "oracle summary missing: $ORACLE_SUMMARY"
}

run_standalone_cell() {
  local name="$1"
  local start_rx="$2"
  local y_ratio="$3"
  local end_rx="$4"
  local trace="$LOG_DIR/$name-standalone-trace-$RUN_ID.log"
  local stdout_log="$LOG_DIR/$name-standalone-stdout-$RUN_ID.log"
  local stderr_log="$LOG_DIR/$name-standalone-stderr-$RUN_ID.log"
  local out_json="$LOG_DIR/$name-standalone-summary-$RUN_ID.json"
  local trigger="$RUN_DIR/$name.trigger"
  local ack="$RUN_DIR/$name.ack"
  local ready window_id win_line sx sy ex ey before_change after_change after_text after_file after_len after_hash sample

  rm -f "$trigger" "$ack"
  log "standalone_cell=$name ratios=${start_rx},${y_ratio}-${end_rx},${y_ratio}"
  "$PROBE_APP" "$PDF_PATH" "$trace" "$trigger" "$ack" >"$stdout_log" 2>"$stderr_log" &
  CURRENT_PID="$!"
  for _ in $(seq 1 80); do
    ready="$(grep -E "^READY window_id=[0-9]+" "$stdout_log" | tail -1 || true)"
    [ -n "$ready" ] && break
    delay 0.25
  done
  [ -n "${ready:-}" ] || fail "$name standalone WKWebView did not become ready"
  window_id="$(printf '%s\n' "$ready" | sed -E 's/.*window_id=([0-9]+).*/\1/')"
  win_line="$(exact_window_bounds "$window_id")" || fail "$name standalone window bounds missing"
  read -r sx sy <<<"$(point_from_window_ratio "$win_line" "$start_rx" "$y_ratio")"
  read -r ex ey <<<"$(point_from_window_ratio "$win_line" "$end_rx" "$y_ratio")"

  activate_pid "$CURRENT_PID" || true
  delay 0.5
  before_change="$(pasteboard_change_count)"
  printf '%s' "ISSUE834_EXP54_${name}_SENTINEL_$RUN_ID" | pbcopy
  write_trigger "$trigger" "$ack" dump before-drag "$(python3 - <<PY
print(round(900 * float("$start_rx"), 1))
PY
)" "$(python3 - <<PY
print(round(650 * float("$y_ratio"), 1))
PY
)"
  swift "$ROOT/scripts/ghostty-app/inject.swift" click "$sx" "$sy" >>"$HARNESS_LOG" 2>&1
  delay 0.25
  slow_drag "$sx" "$sy" "$ex" "$ey" >>"$HARNESS_LOG" 2>&1
  delay 0.75
  write_trigger "$trigger" "$ack" dump after-drag "$(python3 - <<PY
print(round(900 * float("$end_rx"), 1))
PY
)" "$(python3 - <<PY
print(round(650 * float("$y_ratio"), 1))
PY
)"
  write_trigger "$trigger" "$ack" dump before-copy 0 0
  swift "$ROOT/scripts/ghostty-app/inject.swift" key 8 command >>"$HARNESS_LOG" 2>&1
  delay 1
  write_trigger "$trigger" "$ack" dump after-copy 0 0

  after_text="$(pbpaste)"
  after_change="$(pasteboard_change_count)"
  after_file="$RUN_DIR/$name-after-copy.txt"
  printf '%s' "$after_text" >"$after_file"
  after_len="$(wc -c <"$after_file" | tr -d ' ')"
  after_hash="$(hash_file "$after_file")"
  sample="$(printf '%s' "$after_text" | head -c 160 | tr '\n' ' ')"

  python3 - "$out_json" "$name" "$start_rx" "$y_ratio" "$end_rx" "$win_line" "$sx" "$sy" "$ex" "$ey" "$before_change" "$after_change" "$after_len" "$after_hash" "$sample" "$trace" "$stdout_log" "$stderr_log" <<'PY'
import json
import sys
from pathlib import Path

target, name, start_rx, y_ratio, end_rx, win_line, sx, sy, ex, ey, before, after, length, digest, sample, trace, stdout, stderr = sys.argv[1:19]
tokens = ["LEFT834", "MID834", "RIGHT834"]
data = {
    "name": name,
    "copy_route": "cg-event-command-c",
    "drag_ratios": {"start_x": float(start_rx), "end_x": float(end_rx), "y": float(y_ratio)},
    "window_bounds": win_line,
    "drag_global": {"start": [int(sx), int(sy)], "end": [int(ex), int(ey)]},
    "clipboard": {
        "before_change_count": int(before),
        "after_change_count": int(after),
        "after_length": int(length),
        "after_sha256": digest,
        "after_sample": sample,
        "tokens_present": [token for token in tokens if token in sample],
        "contains_all_tokens": all(token in sample for token in tokens),
    },
    "artifacts": {"trace": trace, "stdout": stdout, "stderr": stderr},
}
Path(target).write_text(json.dumps(data, indent=2, sort_keys=True) + "\n")
print(json.dumps({
    "name": name,
    "tokens_present": data["clipboard"]["tokens_present"],
    "contains_all_tokens": data["clipboard"]["contains_all_tokens"],
}, indent=2, sort_keys=True))
PY

  cleanup_current_process
}

run_standalone_matrix() {
  write_swift_app
  run_standalone_cell embedded-ratio 0.58 0.43 0.99
  run_standalone_cell oracle-base 0.18 0.25 0.86
  run_standalone_cell oracle-y-low 0.18 0.21 0.86
  run_standalone_cell oracle-y-high 0.18 0.29 0.86
  run_standalone_cell oracle-x-wide 0.16 0.25 0.90
  run_standalone_cell oracle-x-tight 0.20 0.25 0.82
}

run_embedded() {
  rm -rf "$EXP44_LOG_DIR"
  log "running embedded Surfari geometry trace"
  if TERMSURF_ISSUE834_PDF_FIXTURE_MODE=separated-tokens \
    TERMSURF_ISSUE834_PDF_EXPECTED_TOKENS="$EXPECTED_TEXT" \
    TERMSURF_ISSUE834_PDF_ACCEPTED_SUBSTRING="RIGHT834" \
    TERMSURF_ISSUE834_PDF_COPY_DRAG_START_X_RATIO=0.58 \
    TERMSURF_ISSUE834_PDF_COPY_DRAG_END_X_RATIO=0.99 \
    TERMSURF_ISSUE834_PDF_COPY_DRAG_Y_RATIO=0.43 \
    TERMSURF_ISSUE834_PDF_COPY_DELAY_AFTER_DRAG=0.25 \
    TERMSURF_SURFARI_PDF_COPY_TRACE=1 \
    TERMSURF_SURFARI_PDF_COPY_TRACE_FILE="$EMBEDDED_COPY_TRACE" \
    TERMSURF_SURFARI_PDF_COPY_DIRECT=1 \
    TERMSURF_SURFARI_PDF_VIEW_GEOMETRY_TRACE=1 \
    TERMSURF_SURFARI_PDF_VIEW_GEOMETRY_TRACE_FILE="$EMBEDDED_TRACE" \
    env -u TERMSURF_SURFARI_CACONTEXT_LAYER \
    "$ROOT/scripts/test-issue-834-surfari-pdf-selection-copy.sh" >>"$HARNESS_LOG" 2>&1; then
    :
  fi

  if [ -f "$EXP44_LOG_DIR/surfari-pdf-selection-copy-summary.json" ]; then
    cp "$EXP44_LOG_DIR/surfari-pdf-selection-copy-summary.json" "$EMBEDDED_SUMMARY"
  else
    python3 - "$EMBEDDED_SUMMARY" <<'PY'
from pathlib import Path
import json
import sys

Path(sys.argv[1]).write_text(json.dumps({"overall_result": "missing"}, indent=2) + "\n")
PY
  fi
}

classify() {
  python3 - "$SUMMARY" "$RUN_ID" "$ORIGINAL_RESTORE_STATUS" "$ORACLE_SUMMARY" "$EMBEDDED_SUMMARY" "$LOG_DIR" "$FIXTURE_IDENTITY" "$EMBEDDED_TRACE" "$EMBEDDED_COPY_TRACE" "$HARNESS_LOG" <<'PY'
import json
import re
import sys
from pathlib import Path

summary_path = Path(sys.argv[1])
run_id = sys.argv[2]
restore_status = sys.argv[3]
oracle_path = Path(sys.argv[4])
embedded_summary_path = Path(sys.argv[5])
log_dir = Path(sys.argv[6])
fixture_identity = json.loads(sys.argv[7])
embedded_trace_path = Path(sys.argv[8])
embedded_copy_trace_path = Path(sys.argv[9])
harness_log = Path(sys.argv[10])
expected = ["LEFT834", "MID834", "RIGHT834"]

def load_json(path):
    return json.loads(path.read_text()) if path.exists() else None

def read_text(path):
    return path.read_text(errors="replace") if path.exists() else ""

oracle = load_json(oracle_path)
embedded = load_json(embedded_summary_path)
standalone_cells = []
for path in sorted(log_dir.glob(f"*-standalone-summary-{run_id}.json")):
    cell = load_json(path) or {}
    cell["path"] = str(path)
    trace = read_text(Path(cell.get("artifacts", {}).get("trace", "")))
    cell["trace_complete"] = all(
        marker in trace
        for marker in [
            "standalone-pdf-calibration-state",
            "standalone-pdf-calibration-hit-chain",
            "standalone-pdf-calibration-tree",
            "standalone-pdf-calibration-scroll",
            "target_nil=",
            "target_webview=",
        ]
    )
    standalone_cells.append(cell)

oracle_gate_open = bool(
    oracle
    and oracle.get("classification") == "separated-token-oracle-pass"
    and oracle.get("embedded_interpretation_gate") == "open"
)
embedded_clipboard = (embedded or {}).get("clipboard", {})
embedded_primary = embedded_clipboard.get("after_copy_sample", "")
embedded_tokens = [token for token in expected if token in embedded_primary]
embedded_reproduced = "LEFT834" in embedded_tokens and "MID834" in embedded_tokens and "RIGHT834" not in embedded_tokens

embedded_fixture = (embedded or {}).get("fixture", {})
fixture_identity_match = (
    oracle_gate_open
    and embedded_fixture.get("pdf_text_operator") == fixture_identity.get("operator_summary")
    and embedded_fixture.get("pdf_text_bboxes") == fixture_identity.get("token_boxes")
    and embedded_fixture.get("page_geometry") == fixture_identity.get("page_geometry")
    and embedded_fixture.get("font") == fixture_identity.get("font")
    and embedded_fixture.get("text_extracted") == fixture_identity.get("extracted_text")
)

successes = [cell for cell in standalone_cells if cell.get("clipboard", {}).get("contains_all_tokens")]
embedded_ratio = next((cell for cell in standalone_cells if cell.get("name") == "embedded-ratio"), None)
embedded_ratio_success = bool(embedded_ratio and embedded_ratio.get("clipboard", {}).get("contains_all_tokens"))
standalone_traces_complete = bool(standalone_cells) and all(cell.get("trace_complete") for cell in standalone_cells)
embedded_trace = read_text(embedded_trace_path)
embedded_trace_complete = all(
    marker in embedded_trace
    for marker in [
        "surfari-pdf-view-geometry-state",
        "surfari-pdf-view-geometry-hit-chain",
        "surfari-pdf-view-geometry-tree",
        "surfari-pdf-view-geometry-scroll",
        "target_nil=",
        "target_webview=",
    ]
)

successful_ys = sorted({round(cell.get("drag_ratios", {}).get("y", -1), 3) for cell in successes})
embedded_y = 0.43
embedded_outside_y_band = bool(successful_ys) and (embedded_y < min(successful_ys) or embedded_y > max(successful_ys))

if restore_status != "restored":
    result = "fail"
    classification = "clipboard-restore-failed"
elif not oracle_gate_open or not fixture_identity_match or not embedded_reproduced or not standalone_traces_complete or not embedded_trace_complete:
    result = "partial"
    classification = "harness-insufficient"
elif not successes:
    result = "partial"
    classification = "harness-insufficient"
elif not embedded_ratio_success and embedded_outside_y_band:
    result = "pass"
    classification = "embedded-gesture-outside-standalone-band"
else:
    result = "partial"
    classification = "standalone-calibration-only"

data = {
    "overall_result": result,
    "classification": classification,
    "run_id": run_id,
    "oracle_summary": str(oracle_path),
    "oracle_gate_open": oracle_gate_open,
    "fixture_identity": fixture_identity,
    "fixture_identity_match": fixture_identity_match,
    "standalone_cells": standalone_cells,
    "standalone_success_count": len(successes),
    "standalone_success_names": [cell.get("name") for cell in successes],
    "standalone_success_y_ratios": successful_ys,
    "standalone_embedded_ratio_success": embedded_ratio_success,
    "standalone_traces_complete": standalone_traces_complete,
    "embedded_summary": str(embedded_summary_path),
    "embedded_reproduced_missing_right": embedded_reproduced,
    "embedded_tokens": embedded_tokens,
    "embedded_trace_complete": embedded_trace_complete,
    "embedded_y_ratio": embedded_y,
    "embedded_outside_success_y_band": embedded_outside_y_band,
    "clipboard_restore_status": restore_status,
    "artifacts": {
        "harness_log": str(harness_log),
        "embedded_trace": str(embedded_trace_path),
        "embedded_copy_trace": str(embedded_copy_trace_path),
    },
}
summary_path.write_text(json.dumps(data, indent=2, sort_keys=True) + "\n")
print(json.dumps({
    "overall_result": result,
    "classification": classification,
    "oracle_gate_open": oracle_gate_open,
    "fixture_identity_match": fixture_identity_match,
    "standalone_success_count": len(successes),
    "standalone_success_names": [cell.get("name") for cell in successes],
    "embedded_reproduced_missing_right": embedded_reproduced,
    "embedded_outside_success_y_band": embedded_outside_y_band,
}, indent=2, sort_keys=True))
if result == "fail":
    sys.exit(1)
PY
}

log "run_id=$RUN_ID"
pbpaste >"$ORIGINAL_CLIPBOARD" || true
write_pdf_fixture
FIXTURE_IDENTITY="$(fixture_identity_json)"
log "fixture_identity=$FIXTURE_IDENTITY"
run_oracle
run_standalone_matrix
run_embedded
restore_original_clipboard || ORIGINAL_RESTORE_STATUS="restore-failed"
classify
log "summary=$SUMMARY"
