#!/usr/bin/env python3
"""Inventory advanced Roamium PDF surfaces through TermSurf."""

from __future__ import annotations

import argparse
import http.server
import json
import os
import pathlib
import re
import socket
import socketserver
import struct
import subprocess
import sys
import threading
import time
from dataclasses import dataclass, field as dataclass_field
from typing import Any

from termsurf_pdf_protocol_harness import (
    bool_field,
    create_tab_payload,
    double_field,
    inner_payload,
    send_message,
    string_field,
    tab_ready_id,
    varint_field,
)


ROOT = pathlib.Path(__file__).resolve().parents[1]
ROAMIUM = ROOT / "chromium/src/out/Default/roamium"
BITCOIN_PDF = ROOT / "test-html/public/bitcoin.pdf"
ADVANCED_PROBE = ROOT / "scripts/probe-pdf-advanced.mjs"
DEVTOOLS_RE = re.compile(r"DevTools listening on ws://127\.0\.0\.1:(\d+)/")


def resize_payload(tab_id: int, width: int, height: int) -> bytes:
    return (
        varint_field(1, tab_id)
        + varint_field(2, width)
        + varint_field(3, height)
        + double_field(4, 0.0)
        + double_field(5, 0.0)
        + double_field(6, float(width))
        + double_field(7, float(height))
        + double_field(8, 1.0)
    )


def focus_payload(tab_id: int, focused: bool) -> bytes:
    return varint_field(1, tab_id) + bool_field(2, focused)


def key_event_payload(
    tab_id: int,
    event_type: str,
    windows_key_code: int,
    utf8: str = "",
    modifiers: int = 0,
) -> bytes:
    return (
        varint_field(1, tab_id)
        + string_field(2, event_type)
        + varint_field(3, windows_key_code)
        + string_field(4, utf8)
        + varint_field(5, modifiers)
    )


def mouse_event_payload(
    tab_id: int,
    event_type: str,
    button: str,
    x: float,
    y: float,
    click_count: int,
    modifiers: int,
) -> bytes:
    return (
        varint_field(1, tab_id)
        + string_field(2, event_type)
        + string_field(3, button)
        + double_field(4, x)
        + double_field(5, y)
        + varint_field(6, click_count)
        + varint_field(7, modifiers)
    )


class ReusableTcpServer(socketserver.TCPServer):
    allow_reuse_address = True


class AdvancedPdfHandler(http.server.BaseHTTPRequestHandler):
    log_dir: pathlib.Path
    fixtures: dict[str, dict[str, Any]]
    requests: list[dict[str, Any]]

    def log_message(self, fmt: str, *args: object) -> None:
        with (self.log_dir / "http.log").open("a", encoding="utf-8") as log:
            log.write((fmt % args) + "\n")

    def do_GET(self) -> None:
        request_path = self.path.split("?", 1)[0]
        fixture = self.fixtures.get(request_path)
        if not fixture:
            self.requests.append({"path": request_path, "status": 404})
            self.send_response(404)
            self.end_headers()
            return
        data = pathlib.Path(fixture["path"]).read_bytes()
        self.requests.append(
            {
                "path": request_path,
                "status": 200,
                "content_type": "application/pdf",
                "bytes": len(data),
                "kind": fixture["kind"],
            }
        )
        self.send_response(200)
        self.send_header("Content-Type", "application/pdf")
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)


@dataclass
class HarnessState:
    server_register_received: bool = False
    create_tab_sent: bool = False
    tab_ready_id: int | None = None
    resize_sent: bool = False
    focus_sent: bool = False
    devtools_port: int | None = None
    mouse_messages_sent: list[dict[str, Any]] = dataclass_field(default_factory=list)
    key_messages_sent: list[dict[str, Any]] = dataclass_field(default_factory=list)
    probe_status: str = "not-run"
    roamium_trace_init: bool = False
    roamium_mouse_event_line: bool = False
    roamium_key_event_line: bool = False
    first_failing_hop: str = "automation-gap"


def read_text(path: pathlib.Path) -> str:
    try:
        return path.read_text(encoding="utf-8", errors="replace")
    except FileNotFoundError:
        return ""


def run_checked(cmd: list[str], log_path: pathlib.Path) -> dict[str, Any]:
    proc = subprocess.run(
        cmd,
        cwd=str(ROOT),
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    log_path.write_text(
        json.dumps(
            {
                "cmd": cmd,
                "returncode": proc.returncode,
                "stdout": proc.stdout,
                "stderr": proc.stderr,
            },
            indent=2,
        )
        + "\n",
        encoding="utf-8",
    )
    return {
        "cmd": cmd,
        "returncode": proc.returncode,
        "stdout": proc.stdout,
        "stderr": proc.stderr,
    }


def write_minimal_pdf(path: pathlib.Path, title: str) -> None:
    content = f"BT /F1 24 Tf 72 720 Td ({title}) Tj ET".encode("ascii")
    objects: list[bytes] = [
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] "
        b"/Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>",
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
        b"<< /Length %d >>\nstream\n%s\nendstream" % (len(content), content),
    ]
    data = bytearray(b"%PDF-1.4\n")
    offsets = [0]
    for index, obj in enumerate(objects, start=1):
        offsets.append(len(data))
        data.extend(f"{index} 0 obj\n".encode("ascii"))
        data.extend(obj)
        data.extend(b"\nendobj\n")
    xref_offset = len(data)
    data.extend(f"xref\n0 {len(objects) + 1}\n".encode("ascii"))
    data.extend(b"0000000000 65535 f \n")
    for offset in offsets[1:]:
        data.extend(f"{offset:010d} 00000 n \n".encode("ascii"))
    data.extend(
        (
            f"trailer\n<< /Size {len(objects) + 1} /Root 1 0 R >>\n"
            f"startxref\n{xref_offset}\n%%EOF\n"
        ).encode("ascii")
    )
    path.write_bytes(bytes(data))


def write_acroform_pdf(path: pathlib.Path) -> None:
    content = (
        b"BT /F1 18 Tf 72 720 Td (TermSurf AcroForm Probe) Tj "
        b"72 660 Td (Name:) Tj 0 -40 Td (Agree:) Tj ET"
    )
    off_stream = b"q 1 1 1 rg 0 0 15 15 re f Q"
    yes_stream = b"q 1 1 1 rg 0 0 15 15 re f 0 0 0 RG 2 w 3 8 m 7 3 l 13 13 l S Q"
    objects: list[bytes] = [
        b"<< /Type /Catalog /Pages 3 0 R /AcroForm 2 0 R >>",
        b"<< /Fields [6 0 R 7 0 R] /NeedAppearances true "
        b"/DR << /Font << /Helv 5 0 R >> >> /DA (/Helv 12 Tf 0 g) >>",
        b"<< /Type /Pages /Kids [4 0 R] /Count 1 >>",
        b"<< /Type /Page /Parent 3 0 R /MediaBox [0 0 612 792] "
        b"/Resources << /Font << /F1 5 0 R /Helv 5 0 R >> >> "
        b"/Contents 8 0 R /Annots [6 0 R 7 0 R] >>",
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
        b"<< /Type /Annot /Subtype /Widget /FT /Tx /T (name) /V () "
        b"/Rect [160 650 380 675] /DA (/Helv 12 Tf 0 g) /F 4 /P 4 0 R >>",
        b"<< /Type /Annot /Subtype /Widget /FT /Btn /T (agree) /V /Off /AS /Off "
        b"/Rect [160 610 175 625] /F 4 /P 4 0 R "
        b"/AP << /N << /Off 9 0 R /Yes 10 0 R >> >> >>",
        b"<< /Length %d >>\nstream\n%s\nendstream" % (len(content), content),
        b"<< /Type /XObject /Subtype /Form /BBox [0 0 15 15] /Length %d >>\nstream\n%s\nendstream"
        % (len(off_stream), off_stream),
        b"<< /Type /XObject /Subtype /Form /BBox [0 0 15 15] /Length %d >>\nstream\n%s\nendstream"
        % (len(yes_stream), yes_stream),
    ]
    data = bytearray(b"%PDF-1.7\n")
    offsets = [0]
    for index, obj in enumerate(objects, start=1):
        offsets.append(len(data))
        data.extend(f"{index} 0 obj\n".encode("ascii"))
        data.extend(obj)
        data.extend(b"\nendobj\n")
    xref_offset = len(data)
    data.extend(f"xref\n0 {len(objects) + 1}\n".encode("ascii"))
    data.extend(b"0000000000 65535 f \n")
    for offset in offsets[1:]:
        data.extend(f"{offset:010d} 00000 n \n".encode("ascii"))
    data.extend(
        (
            f"trailer\n<< /Size {len(objects) + 1} /Root 1 0 R >>\n"
            f"startxref\n{xref_offset}\n%%EOF\n"
        ).encode("ascii")
    )
    path.write_bytes(bytes(data))


def prepare_fixtures(log_dir: pathlib.Path) -> dict[str, Any]:
    fixtures_dir = log_dir / "fixtures"
    fixtures_dir.mkdir(parents=True, exist_ok=True)
    valid = fixtures_dir / "valid.pdf"
    valid.write_bytes(BITCOIN_PDF.read_bytes())
    form = fixtures_dir / "form.pdf"
    annotation = fixtures_dir / "annotation.pdf"
    write_acroform_pdf(form)
    write_minimal_pdf(annotation, "TermSurf Annotation Probe")
    fixture_info: dict[str, Any] = {
        "/valid.pdf": {
            "kind": "valid-control",
            "path": str(valid),
            "bytes": valid.stat().st_size,
            "generation": "copied-bitcoin-pdf",
        },
        "/form.pdf": {
            "kind": "form",
            "path": str(form),
            "bytes": form.stat().st_size,
            "generation": "deterministic-acroform",
            "fields": ["name text field", "agree checkbox"],
        },
        "/annotation.pdf": {
            "kind": "annotation",
            "path": str(annotation),
            "bytes": annotation.stat().st_size,
            "generation": "minimal-pdf-placeholder",
            "note": "Placeholder until a deterministic annotation fixture generator is added.",
        },
    }
    qpdf = shutil_which("qpdf")
    if qpdf:
        fixture_info["qpdf_version"] = run_checked([qpdf, "--version"], log_dir / "qpdf-version.json")
    (log_dir / "fixtures.json").write_text(
        json.dumps(fixture_info, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    return fixture_info


def shutil_which(name: str) -> str | None:
    for part in os.environ.get("PATH", "").split(os.pathsep):
        candidate = pathlib.Path(part) / name
        if candidate.exists() and os.access(candidate, os.X_OK):
            return str(candidate)
    return None


def start_pdf_server(
    log_dir: pathlib.Path,
    port: int,
    fixtures: dict[str, dict[str, Any]],
) -> socketserver.TCPServer:
    AdvancedPdfHandler.log_dir = log_dir
    AdvancedPdfHandler.fixtures = fixtures
    AdvancedPdfHandler.requests = []
    server = ReusableTcpServer(("127.0.0.1", port), AdvancedPdfHandler)
    host, bound_port = server.server_address
    (log_dir / "http-server.log").write_text(
        f"listening on {host}:{bound_port}\n",
        encoding="utf-8",
    )
    threading.Thread(target=server.serve_forever, daemon=True).start()
    return server


def wait_for_devtools_port(log_dir: pathlib.Path, timeout: float) -> int | None:
    deadline = time.time() + timeout
    while time.time() < deadline:
        text = read_text(log_dir / "roamium.stdout") + "\n" + read_text(
            log_dir / "roamium.stderr"
        )
        if match := DEVTOOLS_RE.search(text):
            return int(match.group(1))
        time.sleep(0.1)
    return None


def wait_for_tab_ready(
    conn: socket.socket,
    log_dir: pathlib.Path,
    url: str,
    width: int,
    height: int,
    timeout: float,
    state: HarnessState,
) -> None:
    start = time.time()
    with (log_dir / "messages.log").open("w", encoding="utf-8") as messages:
        while time.time() - start < timeout:
            try:
                header = conn.recv(4)
                if not header:
                    break
                size = struct.unpack("<I", header)[0]
                payload = bytearray()
                while len(payload) < size:
                    payload.extend(conn.recv(size - len(payload)))
                top, body = inner_payload(bytes(payload))
                messages.write(f"t={time.time() - start:.3f} top_field={top}\n")
                messages.flush()
                if top == 12 and not state.create_tab_sent:
                    state.server_register_received = True
                    send_message(conn, 1, create_tab_payload(url, width, height))
                    state.create_tab_sent = True
                    messages.write("sent CreateTab\n")
                    messages.flush()
                elif top == 13:
                    state.tab_ready_id = tab_ready_id(body)
                    messages.write(f"tab_ready id={state.tab_ready_id}\n")
                    if state.tab_ready_id:
                        send_message(
                            conn,
                            3,
                            resize_payload(state.tab_ready_id, width, height),
                        )
                        send_message(conn, 10, focus_payload(state.tab_ready_id, True))
                        state.resize_sent = True
                        state.focus_sent = True
                        messages.write("sent Resize and Focus\n")
                        messages.flush()
                        return
            except socket.timeout:
                pass


def send_mouse(
    conn: socket.socket,
    state: HarnessState,
    event_type: str,
    button: str,
    x: float,
    y: float,
) -> None:
    if not state.tab_ready_id:
        return
    send_message(conn, 6, mouse_event_payload(state.tab_ready_id, event_type, button, x, y, 1, 0))
    state.mouse_messages_sent.append(
        {
            "type": event_type,
            "button": button,
            "x": x,
            "y": y,
        }
    )
    time.sleep(0.05)


def send_key(
    conn: socket.socket,
    state: HarnessState,
    event_type: str,
    key_code: int,
    utf8: str = "",
) -> None:
    if not state.tab_ready_id:
        return
    send_message(conn, 9, key_event_payload(state.tab_ready_id, event_type, key_code, utf8, 0))
    state.key_messages_sent.append({"type": event_type, "windows_key_code": key_code, "utf8_len": len(utf8)})
    time.sleep(0.04)


def trace_flags(log_dir: pathlib.Path, state: HarnessState) -> None:
    trace = read_text(log_dir / "pdf-input.log")
    state.roamium_trace_init = "trace-init" in trace
    state.roamium_mouse_event_line = "mouse-event" in trace
    state.roamium_key_event_line = "key-event" in trace


def run_devtools_probe(
    log_dir: pathlib.Path,
    devtools_port: int,
    url_contains: str,
    probe: str,
    timeout_seconds: int,
    settle_seconds: int,
) -> tuple[str, str, pathlib.Path]:
    out_dir = log_dir / "devtools"
    out_dir.mkdir(parents=True, exist_ok=True)
    proc = subprocess.run(
        [
            "node",
            str(ADVANCED_PROBE),
            "--devtools-port",
            str(devtools_port),
            "--url-contains",
            url_contains,
            "--out-dir",
            str(out_dir),
            "--probe",
            probe,
            "--timeout-seconds",
            str(timeout_seconds),
            "--settle-seconds",
            str(settle_seconds),
        ],
        cwd=str(ROOT),
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    (out_dir / "probe.stdout").write_text(proc.stdout, encoding="utf-8")
    (out_dir / "probe.stderr").write_text(proc.stderr, encoding="utf-8")
    return (
        "ok" if proc.returncode == 0 else "error",
        proc.stderr.strip(),
        out_dir / "pdf-advanced-devtools-summary.json",
    )


def load_json(path: pathlib.Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


SOURCE_AUDIT = {
    "forms": [
        "chrome/browser/resources/pdf/pdf_internal_plugin_wrapper.ts relays formFocusChange and tracks isFormFieldFocused.",
        "chrome/browser/resources/pdf/pdf_viewer.ts handles formFocusChange and updates formFieldFocus_.",
        "chrome/browser/resources/pdf/elements/viewer_toolbar.ts disables toolbar shortcuts while form fields are focused.",
        "pdf/pdfium/pdfium_page.cc exposes AccessibilityTextFieldInfo for form-like fields.",
    ],
    "annotations": [
        "chrome/browser/resources/pdf/pdf_viewer.ts uses pdfInk2Enabled_, annotationMode_, and finish/start ink stroke plugin messages.",
        "chrome/browser/resources/pdf/elements/viewer_toolbar.html exposes annotation controls when pdfInk2Enabled and annotationAvailable are true.",
        "chrome/browser/resources/pdf/ink2_manager.ts manages text annotations and finishTextAnnotation messages.",
        "pdf/pdf_ink_module.cc handles annotationRedo, annotationUndo, and ink/text annotation messages.",
    ],
    "context_menu": [
        "components/pdf/browser/pdf_document_helper.h exposes RunContextMenu().",
        "chrome/browser/resources/pdf/gesture_detector.ts handles contextmenu suppression for touch gestures but not right click.",
        "PDF plugin context menus are native/menu-observation sensitive and require an external watcher before right-click probing.",
    ],
    "accessibility_searchify": [
        "pdf/pdf_view_web_plugin.h implements EnableAccessibility, LoadOrReloadAccessibility, and Searchify callbacks.",
        "chrome/browser/resources/pdf/pdf_viewer.ts handles showSearchifyInProgress and setHasSearchifyText messages.",
        "components/pdf/browser/pdf_document_helper.h tracks SearchifyStarted().",
        "pdf/pdfium/pdfium_page.cc tracks searchified text in accessibility text runs.",
    ],
}


def plugin_loaded(summary: dict[str, Any] | None) -> bool:
    return bool(summary and summary.get("pluginLoaded"))


def visible_controls(controls: list[dict[str, Any]]) -> list[dict[str, Any]]:
    visible = []
    for control in controls:
        rect = control.get("rect") or {}
        if (
            not control.get("hidden")
            and not control.get("disabled")
            and rect.get("width", 0) > 0
            and rect.get("height", 0) > 0
        ):
            visible.append(control)
    return visible


def pdf_value(summary: dict[str, Any] | None) -> dict[str, Any]:
    values = (summary or {}).get("values") or []
    for item in values:
        value = item.get("value", {})
        if value.get("viewerPresent") or value.get("pluginPresent"):
            return value
    return values[0].get("value", {}) if values else {}


def classify(args: argparse.Namespace, state: HarnessState, summary: dict[str, Any] | None, fixtures: dict[str, Any]) -> None:
    if not state.server_register_received:
        state.first_failing_hop = "roamium-not-registered"
    elif not state.tab_ready_id:
        state.first_failing_hop = "tab-not-ready"
    elif not state.devtools_port:
        state.first_failing_hop = "devtools-target-discovery-failed"
    elif state.probe_status != "ok" or not summary:
        state.first_failing_hop = "devtools-target-discovery-failed"
    elif not plugin_loaded(summary):
        state.first_failing_hop = "pdf-load-failed"
    elif args.probe == "forms":
        if fixtures.get("/form.pdf", {}).get("generation") != "deterministic-acroform":
            state.first_failing_hop = "fixture-generation-gap"
        elif not state.mouse_messages_sent:
            state.first_failing_hop = "protocol-input-not-sent"
        elif not state.roamium_mouse_event_line:
            state.first_failing_hop = "roamium-input-trace-missing"
        else:
            state.first_failing_hop = "form-value-observable-missing"
    elif args.probe == "annotations":
        value = pdf_value(summary)
        flags = value.get("loadTimeFlags", {})
        controls = value.get("annotationControls") or []
        if not controls:
            state.first_failing_hop = "annotation-ui-missing"
        elif not visible_controls(controls):
            state.first_failing_hop = "annotation-state-observable-missing"
        elif flags.get("pdfInk2Enabled") is not True:
            state.first_failing_hop = "annotation-ui-disabled-by-flags"
        else:
            state.first_failing_hop = "annotation-state-observable-missing"
    elif args.probe == "context-menu":
        state.first_failing_hop = "context-menu-native-watcher-missing"
    elif args.probe == "accessibility-searchify":
        value = pdf_value(summary)
        flags = value.get("loadTimeFlags", {})
        searchify = value.get("searchifyProgress")
        if not searchify:
            state.first_failing_hop = "accessibility-searchify-observable-missing"
        elif searchify.get("hidden"):
            state.first_failing_hop = "accessibility-searchify-source-only"
        elif flags.get("pdfSearchifySaveEnabled") is not True:
            state.first_failing_hop = "accessibility-searchify-disabled-by-flags"
        else:
            state.first_failing_hop = "accessibility-searchify-source-only"


def write_summary(log_dir: pathlib.Path, args: argparse.Namespace, state: HarnessState, extra: dict[str, Any]) -> None:
    data = {
        "probe": args.probe,
        "server_register_received": state.server_register_received,
        "create_tab_sent": state.create_tab_sent,
        "tab_ready_id": state.tab_ready_id,
        "resize_sent": state.resize_sent,
        "focus_sent": state.focus_sent,
        "devtools_port": state.devtools_port,
        "probe_status": state.probe_status,
        "protocol_mouse_messages_sent": len(state.mouse_messages_sent),
        "protocol_mouse_messages": state.mouse_messages_sent,
        "protocol_key_messages_sent": len(state.key_messages_sent),
        "protocol_key_messages": state.key_messages_sent,
        "roamium_trace_init": state.roamium_trace_init,
        "roamium_mouse_event_line": state.roamium_mouse_event_line,
        "roamium_key_event_line": state.roamium_key_event_line,
        "first_failing_hop": state.first_failing_hop,
    }
    data.update(extra)
    (log_dir / "pdf-advanced-summary.json").write_text(
        json.dumps(data, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--log-dir", required=True)
    parser.add_argument(
        "--probe",
        choices=["forms", "annotations", "context-menu", "accessibility-searchify"],
        required=True,
    )
    parser.add_argument("--width", type=int, default=1200)
    parser.add_argument("--height", type=int, default=900)
    parser.add_argument("--pdf-port", type=int, default=0)
    parser.add_argument("--setup-timeout", type=float, default=30)
    parser.add_argument("--capture-timeout-seconds", type=int, default=30)
    parser.add_argument("--settle-seconds", type=int, default=3)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    log_dir = pathlib.Path(args.log_dir).resolve()
    log_dir.mkdir(parents=True, exist_ok=True)
    if not ROAMIUM.exists():
        raise SystemExit(f"missing Roamium binary: {ROAMIUM}")
    if not BITCOIN_PDF.exists():
        raise SystemExit(f"missing Bitcoin PDF fixture: {BITCOIN_PDF}")
    if not ADVANCED_PROBE.exists():
        raise SystemExit(f"missing advanced probe: {ADVANCED_PROBE}")
    fixtures = prepare_fixtures(log_dir)
    pdf_server = start_pdf_server(log_dir, args.pdf_port, fixtures)
    host, port = pdf_server.server_address
    request_path = {
        "forms": "/form.pdf",
        "annotations": "/annotation.pdf",
        "context-menu": "/valid.pdf",
        "accessibility-searchify": "/valid.pdf",
    }[args.probe]
    url = f"http://{host}:{port}{request_path}"
    socket_path = log_dir / "gui.sock"
    try:
        socket_path.unlink()
    except FileNotFoundError:
        pass

    state = HarnessState()
    extra: dict[str, Any] = {
        "url": url,
        "fixtures": fixtures,
        "source_audit": SOURCE_AUDIT,
        "http_server": {"host": host, "port": port},
    }
    listener = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    listener.bind(str(socket_path))
    listener.listen(1)
    listener.settimeout(args.setup_timeout)
    stdout = (log_dir / "roamium.stdout").open("wb")
    stderr = (log_dir / "roamium.stderr").open("wb")
    env = os.environ.copy()
    env["TERMSURF_PDF_INPUT_TRACE"] = "1"
    env["TERMSURF_PDF_INPUT_TRACE_FILE"] = str(log_dir / "pdf-input.log")
    proc = subprocess.Popen(
        [
            str(ROAMIUM),
            f"--ipc-socket={socket_path}",
            f"--user-data-dir={log_dir / 'profile'}",
            "--no-sandbox",
        ],
        cwd=str(ROOT / "chromium/src"),
        stdout=stdout,
        stderr=stderr,
        env=env,
    )
    conn: socket.socket | None = None
    try:
        conn, _ = listener.accept()
        conn.settimeout(0.2)
        wait_for_tab_ready(conn, log_dir, url, args.width, args.height, args.setup_timeout, state)
        if args.probe == "forms":
            send_mouse(conn, state, "down", "left", 220, 240)
            send_mouse(conn, state, "up", "left", 220, 240)
            for ch in "ABC":
                send_key(conn, state, "down", ord(ch), ch)
                send_key(conn, state, "up", ord(ch), "")
        elif args.probe == "context-menu":
            extra["context_menu_watcher"] = {
                "ready": False,
                "right_click_sent": False,
                "reason": "native menu watcher not available in this VM session",
            }
        state.devtools_port = wait_for_devtools_port(log_dir, args.setup_timeout)
        devtools_summary = None
        if state.devtools_port:
            state.probe_status, probe_error, probe_path = run_devtools_probe(
                log_dir,
                state.devtools_port,
                pathlib.Path(request_path).name,
                args.probe,
                args.capture_timeout_seconds,
                args.settle_seconds,
            )
            extra["probe_error"] = probe_error
            devtools_summary = load_json(probe_path) if probe_path.exists() else None
            extra["devtools_summary"] = devtools_summary
        trace_flags(log_dir, state)
        extra["http_requests"] = AdvancedPdfHandler.requests
        classify(args, state, devtools_summary, fixtures)
        write_summary(log_dir, args, state, extra)
        return 0 if state.first_failing_hop != "automation-gap" else 1
    finally:
        if conn:
            conn.close()
        proc.terminate()
        try:
            proc.wait(timeout=5)
        except subprocess.TimeoutExpired:
            proc.kill()
        stdout.close()
        stderr.close()
        listener.close()
        pdf_server.shutdown()


if __name__ == "__main__":
    sys.exit(main())
