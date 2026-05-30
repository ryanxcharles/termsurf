#!/usr/bin/env python3
"""Probe PDF toolbar controls through Roamium and DevTools."""

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
from dataclasses import dataclass
from typing import Any


ROOT = pathlib.Path(__file__).resolve().parents[1]
ROAMIUM = ROOT / "chromium/src/out/Default/roamium"
BITCOIN_PDF = ROOT / "test-html/public/bitcoin.pdf"
TOOLBAR_PROBE = ROOT / "scripts/probe-pdf-toolbar.mjs"
TOOLBAR_EVENT_PROBE = ROOT / "scripts/probe-pdf-toolbar-events.mjs"
SAVE_PRINT_TITLE_LOCAL_PROBE = ROOT / "scripts/probe-pdf-save-print-title-local.mjs"
DEVTOOLS_RE = re.compile(r"DevTools listening on ws://127\.0\.0\.1:(\d+)/")


def varint(value: int) -> bytes:
    out = bytearray()
    while value >= 0x80:
        out.append((value & 0x7F) | 0x80)
        value >>= 7
    out.append(value)
    return bytes(out)


def read_varint(buf: bytes, index: int) -> tuple[int, int]:
    shift = 0
    value = 0
    while index < len(buf):
        byte = buf[index]
        index += 1
        value |= (byte & 0x7F) << shift
        if not byte & 0x80:
            return value, index
        shift += 7
    return 0, index


def field(number: int, wire_type: int) -> bytes:
    return varint((number << 3) | wire_type)


def string_field(number: int, value: str) -> bytes:
    data = value.encode("utf-8")
    return field(number, 2) + varint(len(data)) + data


def varint_field(number: int, value: int) -> bytes:
    return field(number, 0) + varint(value)


def bool_field(number: int, value: bool) -> bytes:
    return field(number, 0) + varint(1 if value else 0)


def double_field(number: int, value: float) -> bytes:
    return field(number, 1) + struct.pack("<d", value)


def wrap(inner_field: int, payload: bytes) -> bytes:
    return field(inner_field, 2) + varint(len(payload)) + payload


def send_message(conn: socket.socket, inner_field: int, payload: bytes) -> None:
    message = wrap(inner_field, payload)
    conn.sendall(struct.pack("<I", len(message)) + message)


def inner_payload(payload: bytes) -> tuple[int, bytes]:
    key, index = read_varint(payload, 0)
    length, index = read_varint(payload, index)
    return key >> 3, payload[index : index + length]


def tab_ready_id(payload: bytes) -> int | None:
    index = 0
    while index < len(payload):
        key, index = read_varint(payload, index)
        field_number = key >> 3
        wire_type = key & 7
        if wire_type == 0:
            value, index = read_varint(payload, index)
            if field_number == 2:
                return value
        elif wire_type == 2:
            length, index = read_varint(payload, index)
            index += length
        else:
            return None
    return None


def create_tab_payload(url: str, width: int, height: int) -> bytes:
    return (
        string_field(1, url)
        + string_field(2, "fake-pane")
        + varint_field(3, width)
        + varint_field(4, height)
        + bool_field(5, False)
    )


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


class ReusableTcpServer(socketserver.TCPServer):
    allow_reuse_address = True


class PdfHandler(http.server.BaseHTTPRequestHandler):
    log_dir: pathlib.Path
    extensionless_path: pathlib.Path | None = None
    untitled_path: pathlib.Path | None = None

    def log_message(self, fmt: str, *args: object) -> None:
        with (self.log_dir / "http.log").open("a", encoding="utf-8") as log:
            log.write((fmt % args) + "\n")

    def do_GET(self) -> None:
        request_path = self.path.split("?", 1)[0]
        if request_path == "/bitcoin.pdf":
            data = BITCOIN_PDF.read_bytes()
        elif request_path == "/embedded-pdf.html":
            data = (
                "<!doctype html><meta charset='utf-8'>"
                "<title>Embedded PDF Host</title>"
                "<style>html,body{margin:0;width:100%;height:100%;}"
                "embed{width:100%;height:100%;display:block;}</style>"
                "<embed src='/bitcoin.pdf' type='application/pdf'>"
            ).encode("utf-8")
            self.send_response(200)
            self.send_header("Content-Type", "text/html; charset=utf-8")
            self.send_header("Content-Length", str(len(data)))
            self.end_headers()
            self.wfile.write(data)
            return
        elif request_path == "/bitcoin-extensionless" and self.extensionless_path:
            data = self.extensionless_path.read_bytes()
        elif request_path == "/untitled.pdf" and self.untitled_path:
            data = self.untitled_path.read_bytes()
        else:
            self.send_response(404)
            self.end_headers()
            return

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
    devtools_port: int | None = None
    toolbar_probe_status: str = "not-run"
    first_failing_hop: str = "automation-gap"


def start_pdf_server(log_dir: pathlib.Path, port: int) -> socketserver.TCPServer:
    PdfHandler.log_dir = log_dir
    PdfHandler.extensionless_path = log_dir / "fixtures" / "bitcoin-extensionless"
    PdfHandler.untitled_path = log_dir / "fixtures" / "untitled.pdf"
    server = ReusableTcpServer(("127.0.0.1", port), PdfHandler)
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


def run_toolbar_probe(args: argparse.Namespace, state: HarnessState) -> tuple[str, str]:
    if args.probe == "events":
        script = TOOLBAR_EVENT_PROBE
        out_name = "toolbar-events"
    elif args.probe == "save-print-title-local":
        script = SAVE_PRINT_TITLE_LOCAL_PROBE
        out_name = "save-print-title-local"
    else:
        script = TOOLBAR_PROBE
        out_name = "toolbar"
    out_dir = pathlib.Path(args.log_dir).resolve() / out_name
    out_dir.mkdir(parents=True, exist_ok=True)
    cmd = [
        "node",
        str(script),
        "--devtools-port",
        str(state.devtools_port),
        "--url-contains",
        args.url_contains,
        "--out-dir",
        str(out_dir),
        "--timeout-seconds",
        str(args.capture_timeout_seconds),
        "--settle-seconds",
        str(args.settle_seconds),
    ]
    if args.probe == "save-print-title-local":
        downloads_dir = out_dir / "downloads"
        downloads_dir.mkdir(parents=True, exist_ok=True)
        cmd.extend(["--downloads-dir", str(downloads_dir)])
        fixture_dir = pathlib.Path(args.log_dir).resolve() / "fixtures"
        cmd.extend(
            [
                "--http-pdf-url",
                f"http://127.0.0.1:{args.pdf_port}/bitcoin.pdf",
                "--file-pdf-url",
                (BITCOIN_PDF.resolve().as_uri()),
                "--http-extensionless-url",
                f"http://127.0.0.1:{args.pdf_port}/bitcoin-extensionless",
                "--file-extensionless-url",
                (fixture_dir / "bitcoin-extensionless").resolve().as_uri(),
                "--http-untitled-url",
                f"http://127.0.0.1:{args.pdf_port}/untitled.pdf",
                "--file-untitled-url",
                (fixture_dir / "untitled.pdf").resolve().as_uri(),
                "--embedded-html-url",
                f"http://127.0.0.1:{args.pdf_port}/embedded-pdf.html",
                "--trace-file",
                str(pathlib.Path(args.log_dir).resolve() / "pdf-input.log"),
                "--roamium-stderr",
                str(pathlib.Path(args.log_dir).resolve() / "roamium.stderr"),
            ]
        )
        if args.enable_pdf_print_intercept:
            bridge_trace_file = pathlib.Path(args.log_dir).resolve() / "pdf-print-bridge.log"
            print_intercept_file = pathlib.Path(args.log_dir).resolve() / "pdf-print.log"
            bridge_trace_file.write_text("", encoding="utf-8")
            print_intercept_file.write_text("", encoding="utf-8")
            cmd.extend(
                [
                    "--print-intercept-file",
                    str(print_intercept_file),
                    "--print-bridge-trace-file",
                    str(bridge_trace_file),
                ]
            )
    proc = subprocess.run(
        cmd,
        cwd=str(ROOT),
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    (out_dir / "probe.stdout").write_text(proc.stdout, encoding="utf-8")
    (out_dir / "probe.stderr").write_text(proc.stderr, encoding="utf-8")
    return ("ok" if proc.returncode == 0 else "error", proc.stderr.strip())


def classify(state: HarnessState, toolbar_summary: dict[str, Any] | None) -> None:
    if not state.server_register_received:
        state.first_failing_hop = "roamium-not-registered"
    elif not state.tab_ready_id:
        state.first_failing_hop = "tab-not-ready"
    elif not state.resize_sent:
        state.first_failing_hop = "resize-not-sent"
    elif not state.devtools_port:
        state.first_failing_hop = "devtools-missing"
    elif state.toolbar_probe_status != "ok":
        state.first_failing_hop = "toolbar-probe-error"
    elif not toolbar_summary:
        state.first_failing_hop = "toolbar-summary-missing"
    elif toolbar_summary.get("status") == "pass":
        state.first_failing_hop = "no-failure-observed"
    else:
        state.first_failing_hop = "toolbar-control-partial"


def write_summary(
    log_dir: pathlib.Path,
    state: HarnessState,
    extra: dict[str, Any],
) -> None:
    data = {
        "server_register_received": state.server_register_received,
        "create_tab_sent": state.create_tab_sent,
        "tab_ready_id": state.tab_ready_id,
        "resize_sent": state.resize_sent,
        "devtools_port": state.devtools_port,
        "toolbar_probe_status": state.toolbar_probe_status,
        "first_failing_hop": state.first_failing_hop,
    }
    data.update(extra)
    (log_dir / "pdf-toolbar-summary.json").write_text(
        json.dumps(data, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("url", nargs="?")
    parser.add_argument("--log-dir", required=True)
    parser.add_argument("--width", type=int, default=1200)
    parser.add_argument("--height", type=int, default=900)
    parser.add_argument("--serve-bitcoin-pdf", action="store_true")
    parser.add_argument("--pdf-port", type=int, default=9799)
    parser.add_argument("--url-contains", default="bitcoin.pdf")
    parser.add_argument("--setup-timeout", type=float, default=30)
    parser.add_argument("--capture-timeout-seconds", type=int, default=30)
    parser.add_argument("--settle-seconds", type=int, default=8)
    parser.add_argument(
        "--probe",
        choices=["toolbar", "events", "save-print-title-local"],
        default="toolbar",
    )
    parser.add_argument("--enable-pdf-print-intercept", action="store_true")
    return parser.parse_args()


def write_untitled_pdf(path: pathlib.Path) -> None:
    content = b"BT /F1 24 Tf 72 720 Td (Untitled PDF Fixture) Tj ET"
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


def prepare_exp13_fixtures(log_dir: pathlib.Path) -> dict[str, pathlib.Path]:
    fixtures_dir = log_dir / "fixtures"
    fixtures_dir.mkdir(parents=True, exist_ok=True)
    extensionless = fixtures_dir / "bitcoin-extensionless"
    extensionless.write_bytes(BITCOIN_PDF.read_bytes())
    untitled = fixtures_dir / "untitled.pdf"
    write_untitled_pdf(untitled)
    untitled_extensionless = fixtures_dir / "untitled-extensionless"
    untitled_extensionless.write_bytes(untitled.read_bytes())
    return {
        "extensionless": extensionless,
        "untitled": untitled,
        "untitled_extensionless": untitled_extensionless,
    }


def main() -> int:
    args = parse_args()
    log_dir = pathlib.Path(args.log_dir).resolve()
    args.log_dir = str(log_dir)
    log_dir.mkdir(parents=True, exist_ok=True)

    if not ROAMIUM.exists():
        raise SystemExit(f"missing Roamium binary: {ROAMIUM}")
    if not TOOLBAR_PROBE.exists():
        raise SystemExit(f"missing toolbar probe: {TOOLBAR_PROBE}")
    if args.probe == "events" and not TOOLBAR_EVENT_PROBE.exists():
        raise SystemExit(f"missing toolbar event probe: {TOOLBAR_EVENT_PROBE}")
    if args.probe == "save-print-title-local" and not SAVE_PRINT_TITLE_LOCAL_PROBE.exists():
        raise SystemExit(
            f"missing save/print/title/local probe: {SAVE_PRINT_TITLE_LOCAL_PROBE}"
        )
    if args.serve_bitcoin_pdf and not BITCOIN_PDF.exists():
        raise SystemExit(f"missing Bitcoin PDF fixture: {BITCOIN_PDF}")

    url = args.url
    pdf_server = None
    fixtures = {}
    if args.probe == "save-print-title-local":
        fixtures = prepare_exp13_fixtures(log_dir)
        extra_fixtures_path = log_dir / "fixtures.json"
        extra_fixtures_path.write_text(
            json.dumps({key: str(value) for key, value in fixtures.items()}, indent=2)
            + "\n",
            encoding="utf-8",
        )
    if args.serve_bitcoin_pdf:
        pdf_server = start_pdf_server(log_dir, args.pdf_port)
        url = url or f"http://127.0.0.1:{args.pdf_port}/bitcoin.pdf"
    if not url:
        raise SystemExit("url is required unless --serve-bitcoin-pdf is used")

    socket_path = log_dir / "gui.sock"
    try:
        socket_path.unlink()
    except FileNotFoundError:
        pass

    state = HarnessState()
    extra: dict[str, Any] = {"url": url}
    listener = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    listener.bind(str(socket_path))
    listener.listen(1)
    listener.settimeout(args.setup_timeout)

    stdout = (log_dir / "roamium.stdout").open("wb")
    stderr = (log_dir / "roamium.stderr").open("wb")
    env = os.environ.copy()
    env["TERMSURF_PDF_INPUT_TRACE"] = "1"
    env["TERMSURF_PDF_INPUT_TRACE_FILE"] = str(log_dir / "pdf-input.log")
    if args.enable_pdf_print_intercept:
        env["TERMSURF_PDF_PRINT_INTERCEPT"] = "1"
        env["TERMSURF_PDF_PRINT_INTERCEPT_FILE"] = str(log_dir / "pdf-print.log")
        env["TERMSURF_PDF_PRINT_BRIDGE_TRACE"] = "1"
        env["TERMSURF_PDF_PRINT_BRIDGE_TRACE_FILE"] = str(
            log_dir / "pdf-print-bridge.log"
        )
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
        if state.devtools_port:
            state.toolbar_probe_status, probe_error = run_toolbar_probe(args, state)
            extra["toolbar_probe_error"] = probe_error

        if args.probe == "events":
            toolbar_summary_path = log_dir / "toolbar-events" / "toolbar-events-summary.json"
        elif args.probe == "save-print-title-local":
            toolbar_summary_path = (
                log_dir
                / "save-print-title-local"
                / "save-print-title-local-summary.json"
            )
        else:
            toolbar_summary_path = log_dir / "toolbar" / "toolbar-summary.json"
        toolbar_summary = None
        if toolbar_summary_path.exists():
            toolbar_summary = json.loads(toolbar_summary_path.read_text(encoding="utf-8"))
            extra["toolbar_summary"] = toolbar_summary

        classify(state, toolbar_summary)
        write_summary(log_dir, state, extra)
        return 0 if state.toolbar_probe_status == "ok" else 1
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
        if pdf_server:
            pdf_server.shutdown()


if __name__ == "__main__":
    sys.exit(main())
