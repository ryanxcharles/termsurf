#!/usr/bin/env python3
"""Probe Roamium PDF internal and external links through TermSurf mouse input."""

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
LINK_PROBE = ROOT / "scripts/probe-pdf-links.mjs"
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


class LinkPdfHandler(http.server.BaseHTTPRequestHandler):
    log_dir: pathlib.Path
    pdf_bytes: bytes

    def log_message(self, fmt: str, *args: object) -> None:
        with (self.log_dir / "http.log").open("a", encoding="utf-8") as log:
            log.write((fmt % args) + "\n")

    def do_GET(self) -> None:
        request_path = self.path.split("?", 1)[0]
        if request_path == "/pdf-link-fixture.pdf":
            data = self.pdf_bytes
            self.send_response(200)
            self.send_header("Content-Type", "application/pdf")
            self.send_header("Content-Length", str(len(data)))
            self.end_headers()
            self.wfile.write(data)
            return
        if request_path == "/pdf-link-target.html":
            data = (
                "<!doctype html><meta charset='utf-8'>"
                "<title>PDF Link Target</title>"
                "<h1 id='target'>PDF external link target reached</h1>"
            ).encode("utf-8")
            self.send_response(200)
            self.send_header("Content-Type", "text/html; charset=utf-8")
            self.send_header("Content-Length", str(len(data)))
            self.end_headers()
            self.wfile.write(data)
            return
        self.send_response(404)
        self.end_headers()


@dataclass
class HarnessState:
    server_register_received: bool = False
    create_tab_sent: bool = False
    tab_ready_id: int | None = None
    resize_sent: bool = False
    devtools_port: int | None = None
    mouse_messages_sent: list[dict[str, Any]] = dataclass_field(default_factory=list)
    before_probe_status: str = "not-run"
    after_probe_status: str = "not-run"
    roamium_trace_init: bool = False
    roamium_mouse_event_line: bool = False
    roamium_ffi_line: bool = False
    chromium_route_line: bool = False
    chromium_input_router_line: bool = False
    pdf_plugin_input_line: bool = False
    pdfium_mousedown_line: bool = False
    first_failing_hop: str = "automation-gap"


def pdf_string(value: str) -> str:
    return value.replace("\\", "\\\\").replace("(", "\\(").replace(")", "\\)")


def generate_link_pdf(external_url: str) -> bytes:
    content1 = (
        "BT\n"
        "/F1 24 Tf\n"
        "72 700 Td (INTERNAL LINK AREA - click top half to page 2) Tj\n"
        "72 320 Td (EXTERNAL LINK AREA - click bottom half to target page) Tj\n"
        "ET\n"
    ).encode("ascii")
    content2 = b"BT\n/F1 28 Tf\n72 700 Td (PAGE 2 INTERNAL LINK TARGET) Tj\nET\n"
    objects = [
        "<< /Type /Catalog /Pages 2 0 R >>",
        "<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] "
        "/Resources << /Font << /F1 9 0 R >> >> /Contents 5 0 R "
        "/Annots [7 0 R 8 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] "
        "/Resources << /Font << /F1 9 0 R >> >> /Contents 6 0 R >>",
        f"<< /Length {len(content1)} >>\nstream\n{content1.decode('ascii')}endstream",
        f"<< /Length {len(content2)} >>\nstream\n{content2.decode('ascii')}endstream",
        "<< /Type /Annot /Subtype /Link /Rect [0 396 612 792] "
        "/Border [0 0 0] /Dest [4 0 R /XYZ 0 792 0] >>",
        "<< /Type /Annot /Subtype /Link /Rect [0 0 612 396] "
        f"/Border [0 0 0] /A << /S /URI /URI ({pdf_string(external_url)}) >> >>",
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
    ]
    data = bytearray(b"%PDF-1.4\n")
    offsets = [0]
    for index, obj in enumerate(objects, start=1):
        offsets.append(len(data))
        data.extend(f"{index} 0 obj\n".encode("ascii"))
        data.extend(obj.encode("ascii"))
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
    return bytes(data)


def start_pdf_server(
    log_dir: pathlib.Path,
    port: int,
    pdf_bytes: bytes,
) -> socketserver.TCPServer:
    LinkPdfHandler.log_dir = log_dir
    LinkPdfHandler.pdf_bytes = pdf_bytes
    server = ReusableTcpServer(("127.0.0.1", port), LinkPdfHandler)
    host, bound_port = server.server_address
    (log_dir / "http-server.log").write_text(
        f"listening on {host}:{bound_port}\n",
        encoding="utf-8",
    )
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    return server


def read_text(path: pathlib.Path) -> str:
    try:
        return path.read_text(encoding="utf-8", errors="replace")
    except FileNotFoundError:
        return ""


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
                        state.resize_sent = True
                        messages.write("sent Resize\n")
                        messages.flush()
                        return
            except socket.timeout:
                pass


def send_click(conn: socket.socket, state: HarnessState, x: float, y: float) -> None:
    if not state.tab_ready_id:
        return
    for event_type in ("down", "up"):
        send_message(
            conn,
            6,
            mouse_event_payload(state.tab_ready_id, event_type, "left", x, y, 1, 0),
        )
        state.mouse_messages_sent.append(
            {
                "index": len(state.mouse_messages_sent),
                "message": "mouse_event",
                "type": event_type,
                "button": "left",
                "x": x,
                "y": y,
                "click_count": 1,
                "modifiers": 0,
            }
        )
        time.sleep(0.05)


def run_link_probe(
    log_dir: pathlib.Path,
    devtools_port: int,
    url_contains: str,
    out_name: str,
    timeout_seconds: int,
    settle_seconds: int,
) -> tuple[str, str, pathlib.Path]:
    out_dir = log_dir / out_name
    out_dir.mkdir(parents=True, exist_ok=True)
    proc = subprocess.run(
        [
            "node",
            str(LINK_PROBE),
            "--devtools-port",
            str(devtools_port),
            "--url-contains",
            url_contains,
            "--out-dir",
            str(out_dir),
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
        out_dir / "pdf-links-devtools-summary.json",
    )


def load_json(path: pathlib.Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def plugin_click_point(
    before_summary: dict[str, Any] | None,
    probe: str,
) -> dict[str, float] | None:
    rect = (
        ((before_summary or {}).get("state") or {})
        .get("value", {})
        .get("pluginRect")
    )
    try:
        x = float(rect["x"]) + float(rect["width"]) / 2
        if probe == "internal-link":
            y = float(rect["y"]) + float(rect["height"]) * 0.25
        else:
            y = float(rect["y"]) + float(rect["height"]) * 0.75
        return {
            "x": x,
            "y": y,
            "width": float(rect["width"]),
            "height": float(rect["height"]),
            "source": "plugin-rect-top-half" if probe == "internal-link" else "plugin-rect-bottom-half",
        }
    except (TypeError, KeyError, ValueError):
        return None


def trace_flags(log_dir: pathlib.Path, state: HarnessState) -> None:
    trace = read_text(log_dir / "pdf-input.log")
    stderr = read_text(log_dir / "roamium.stderr")
    state.roamium_trace_init = "trace-init" in trace
    state.roamium_mouse_event_line = "mouse-event" in trace
    state.roamium_ffi_line = "ffi=ts_forward_mouse_event" in trace
    state.chromium_route_line = "[termsurf-pdf-input] mouse-route" in stderr
    state.chromium_input_router_line = "input-router" in stderr
    state.pdf_plugin_input_line = "[termsurf-pdf-input] pdf-plugin" in stderr
    state.pdfium_mousedown_line = "[termsurf-pdf-input] pdfium-mouse event=mouse-down" in stderr


def viewer_page(summary: dict[str, Any] | None) -> Any:
    value = ((summary or {}).get("state") or {}).get("value", {})
    return (
        value.get("viewerProps", {}).get("pageNo_", {}).get("value")
        or value.get("controllerProps", {}).get("pageNo_", {}).get("value")
        or value.get("pageSelectorValue")
    )


def screenshot_sha(summary: dict[str, Any] | None) -> str | None:
    return ((summary or {}).get("screenshot") or {}).get("sha256")


def state_url(summary: dict[str, Any] | None) -> str:
    return str((((summary or {}).get("state") or {}).get("value") or {}).get("url") or "")


def classify(
    args: argparse.Namespace,
    state: HarnessState,
    before_summary: dict[str, Any] | None,
    after_summary: dict[str, Any] | None,
) -> None:
    before_page = viewer_page(before_summary)
    after_page = viewer_page(after_summary)
    before_sha = screenshot_sha(before_summary)
    after_sha = screenshot_sha(after_summary)
    before_url = state_url(before_summary)
    after_url = state_url(after_summary)
    if not state.server_register_received:
        state.first_failing_hop = "roamium-not-registered"
    elif not state.tab_ready_id:
        state.first_failing_hop = "tab-not-ready"
    elif not state.resize_sent:
        state.first_failing_hop = "resize-not-sent"
    elif not state.devtools_port:
        state.first_failing_hop = "devtools-missing"
    elif state.before_probe_status != "ok":
        state.first_failing_hop = "before-state-capture-failed"
    elif not state.mouse_messages_sent:
        state.first_failing_hop = "protocol-mouse-not-sent"
    elif not state.roamium_mouse_event_line:
        state.first_failing_hop = "roamium-mouse-receive-missing"
    elif not state.roamium_ffi_line:
        state.first_failing_hop = "roamium-mouse-ffi-missing"
    elif not state.chromium_route_line:
        state.first_failing_hop = "chromium-mouse-route-missing"
    elif args.probe == "internal-link" and state.after_probe_status != "ok":
        state.first_failing_hop = "after-state-capture-failed"
    elif args.probe == "external-link" and state.after_probe_status != "ok":
        state.first_failing_hop = "external-navigation-target-missing"
    elif args.probe == "internal-link" and not (
        before_page != after_page or (before_sha and after_sha and before_sha != after_sha)
    ):
        state.first_failing_hop = "internal-link-no-navigation"
    elif args.probe == "external-link" and "pdf-link-target.html" not in after_url:
        state.first_failing_hop = "external-link-no-navigation"
    else:
        state.first_failing_hop = "no-failure-observed"


def write_summary(
    log_dir: pathlib.Path,
    args: argparse.Namespace,
    state: HarnessState,
    extra: dict[str, Any],
) -> None:
    data = {
        "probe": args.probe,
        "server_register_received": state.server_register_received,
        "create_tab_sent": state.create_tab_sent,
        "tab_ready_id": state.tab_ready_id,
        "resize_sent": state.resize_sent,
        "devtools_port": state.devtools_port,
        "protocol_mouse_messages_sent": len(state.mouse_messages_sent),
        "protocol_mouse_messages": state.mouse_messages_sent,
        "before_probe_status": state.before_probe_status,
        "after_probe_status": state.after_probe_status,
        "roamium_trace_init": state.roamium_trace_init,
        "roamium_mouse_event_line": state.roamium_mouse_event_line,
        "roamium_ffi_line": state.roamium_ffi_line,
        "chromium_route_line": state.chromium_route_line,
        "chromium_input_router_line": state.chromium_input_router_line,
        "pdf_plugin_input_line": state.pdf_plugin_input_line,
        "pdfium_mousedown_line": state.pdfium_mousedown_line,
        "first_failing_hop": state.first_failing_hop,
    }
    data.update(extra)
    (log_dir / "pdf-links-summary.json").write_text(
        json.dumps(data, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--log-dir", required=True)
    parser.add_argument("--width", type=int, default=1200)
    parser.add_argument("--height", type=int, default=900)
    parser.add_argument("--pdf-port", type=int, default=9799)
    parser.add_argument("--setup-timeout", type=float, default=30)
    parser.add_argument("--capture-timeout-seconds", type=int, default=30)
    parser.add_argument("--settle-seconds", type=int, default=8)
    parser.add_argument(
        "--probe",
        choices=["internal-link", "external-link"],
        required=True,
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    log_dir = pathlib.Path(args.log_dir).resolve()
    args.log_dir = str(log_dir)
    log_dir.mkdir(parents=True, exist_ok=True)

    if not ROAMIUM.exists():
        raise SystemExit(f"missing Roamium binary: {ROAMIUM}")
    if not LINK_PROBE.exists():
        raise SystemExit(f"missing link probe: {LINK_PROBE}")

    external_url = f"http://127.0.0.1:{args.pdf_port}/pdf-link-target.html"
    pdf_bytes = generate_link_pdf(external_url)
    fixture_path = log_dir / "pdf-link-fixture.pdf"
    fixture_path.write_bytes(pdf_bytes)
    (log_dir / "fixture.json").write_text(
        json.dumps(
            {
                "fixture": str(fixture_path),
                "bytes": len(pdf_bytes),
                "pdf_url": f"http://127.0.0.1:{args.pdf_port}/pdf-link-fixture.pdf",
                "external_url": external_url,
                "internal_link_rect_pdf_points": [0, 396, 612, 792],
                "external_link_rect_pdf_points": [0, 0, 612, 396],
            },
            indent=2,
            sort_keys=True,
        )
        + "\n",
        encoding="utf-8",
    )
    pdf_server = start_pdf_server(log_dir, args.pdf_port, pdf_bytes)
    url = f"http://127.0.0.1:{args.pdf_port}/pdf-link-fixture.pdf"

    socket_path = log_dir / "gui.sock"
    try:
        socket_path.unlink()
    except FileNotFoundError:
        pass

    state = HarnessState()
    extra: dict[str, Any] = {"url": url, "external_url": external_url}
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
        wait_for_tab_ready(
            conn,
            log_dir,
            url,
            args.width,
            args.height,
            args.setup_timeout,
            state,
        )
        state.devtools_port = wait_for_devtools_port(log_dir, args.setup_timeout)
        before_summary = None
        after_summary = None
        if state.devtools_port:
            state.before_probe_status, before_error, before_path = run_link_probe(
                log_dir,
                state.devtools_port,
                "pdf-link-fixture.pdf",
                "before",
                args.capture_timeout_seconds,
                args.settle_seconds,
            )
            extra["before_probe_error"] = before_error
            before_summary = load_json(before_path) if before_path.exists() else None
            extra["before_summary"] = before_summary
            click_point = plugin_click_point(before_summary, args.probe)
            extra["click_point"] = click_point
            if click_point:
                send_click(conn, state, click_point["x"], click_point["y"])
                time.sleep(1.0)

            after_url_contains = (
                "pdf-link-target.html"
                if args.probe == "external-link"
                else "pdf-link-fixture.pdf"
            )
            state.after_probe_status, after_error, after_path = run_link_probe(
                log_dir,
                state.devtools_port,
                after_url_contains,
                "after",
                args.capture_timeout_seconds,
                1,
            )
            extra["after_probe_error"] = after_error
            after_summary = load_json(after_path) if after_path.exists() else None
            extra["after_summary"] = after_summary
            extra["before_page"] = viewer_page(before_summary)
            extra["after_page"] = viewer_page(after_summary)
            extra["before_url"] = state_url(before_summary)
            extra["after_url"] = state_url(after_summary)
            extra["screenshot_changed"] = (
                screenshot_sha(before_summary) != screenshot_sha(after_summary)
            )

        trace_flags(log_dir, state)
        classify(args, state, before_summary, after_summary)
        write_summary(log_dir, args, state, extra)
        return 0 if state.first_failing_hop == "no-failure-observed" else 1
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
