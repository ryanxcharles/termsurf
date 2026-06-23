#!/usr/bin/env python3
"""Probe Roamium PDF form input with calibrated field coordinates."""

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
import tempfile
import threading
import time
import zlib
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
FORMS_PROBE = ROOT / "scripts/probe-pdf-forms.mjs"
DEVTOOLS_RE = re.compile(r"DevTools listening on ws://127\.0\.0\.1:(\d+)/")
TEXT_VALUE = "TermSurf834"
VKEY_ESCAPE = 27
PAGE_WIDTH = 612.0
PAGE_HEIGHT = 792.0
FIELD_RECTS = {
    "name": {"type": "text", "rect": [160.0, 650.0, 380.0, 675.0]},
    "agree": {"type": "checkbox", "rect": [160.0, 585.0, 210.0, 635.0]},
}


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


class FormPdfHandler(http.server.BaseHTTPRequestHandler):
    log_dir: pathlib.Path
    fixture_path: pathlib.Path
    requests: list[dict[str, Any]]

    def log_message(self, fmt: str, *args: object) -> None:
        with (self.log_dir / "http.log").open("a", encoding="utf-8") as log:
            log.write((fmt % args) + "\n")

    def do_GET(self) -> None:
        request_path = self.path.split("?", 1)[0]
        if request_path != "/form.pdf":
            self.requests.append({"path": request_path, "status": 404})
            self.send_response(404)
            self.end_headers()
            return
        data = self.fixture_path.read_bytes()
        self.requests.append(
            {
                "path": request_path,
                "status": 200,
                "content_type": "application/pdf",
                "bytes": len(data),
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
    devtools_actions_sent: list[dict[str, Any]] = dataclass_field(default_factory=list)
    roamium_trace_init: bool = False
    roamium_mouse_event_line: bool = False
    roamium_key_event_line: bool = False
    first_failing_hop: str = "automation-gap"


def pdf_string(value: str) -> bytes:
    escaped = value.replace("\\", "\\\\").replace("(", "\\(").replace(")", "\\)")
    return f"({escaped})".encode("ascii")


def write_acroform_pdf(path: pathlib.Path) -> None:
    content = (
        b"BT /F1 18 Tf 72 720 Td (TermSurf PDF Forms Probe) Tj "
        b"72 660 Td (Name:) Tj 0 -40 Td (Agree:) Tj ET"
    )
    off_stream = b"q 1 1 1 rg 0 0 50 50 re f 0 0 0 RG 2 w 1 1 48 48 re S Q"
    yes_stream = (
        b"q 1 1 1 rg 0 0 50 50 re f 0 0 0 RG 2 w 1 1 48 48 re S "
        b"5 w 10 27 m 22 12 l 42 39 l S Q"
    )
    objects: list[bytes] = [
        b"<< /Type /Catalog /Pages 3 0 R /AcroForm 2 0 R >>",
        b"<< /Fields [6 0 R 7 0 R] /NeedAppearances true "
        b"/DR << /Font << /Helv 5 0 R >> >> /DA (/Helv 12 Tf 0 g) >>",
        b"<< /Type /Pages /Kids [4 0 R] /Count 1 >>",
        b"<< /Type /Page /Parent 3 0 R /MediaBox [0 0 612 792] "
        b"/Resources << /Font << /F1 5 0 R /Helv 5 0 R >> >> "
        b"/Contents 8 0 R /Annots [6 0 R 7 0 R] >>",
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
        b"<< /Type /Annot /Subtype /Widget /FT /Tx /T "
        + pdf_string("name")
        + b" /V () /Rect [160 650 380 675] /DA (/Helv 12 Tf 0 g) /F 4 /P 4 0 R >>",
        b"<< /Type /Annot /Subtype /Widget /FT /Btn /T "
        + pdf_string("agree")
        + b" /V /Off /AS /Off /Rect [160 585 210 635] /F 4 /P 4 0 R "
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


def prepare_fixture(log_dir: pathlib.Path) -> dict[str, Any]:
    fixtures_dir = log_dir / "fixtures"
    fixtures_dir.mkdir(parents=True, exist_ok=True)
    fixture_path = fixtures_dir / "form.pdf"
    write_acroform_pdf(fixture_path)
    metadata = {
        "kind": "form",
        "path": str(fixture_path),
        "bytes": fixture_path.stat().st_size,
        "generation": "deterministic-acroform",
        "page": {"width": PAGE_WIDTH, "height": PAGE_HEIGHT},
        "fields": FIELD_RECTS,
    }
    qpdf = shutil_which("qpdf")
    if qpdf:
        proc = subprocess.run(
            [qpdf, "--check", str(fixture_path)],
            cwd=str(ROOT),
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            check=False,
        )
        metadata["qpdf_check"] = {
            "cmd": [qpdf, "--check", str(fixture_path)],
            "returncode": proc.returncode,
            "stdout": proc.stdout,
            "stderr": proc.stderr,
            "ok": proc.returncode == 0,
        }
    else:
        metadata["qpdf_check"] = {"ok": None, "reason": "qpdf-not-found"}
    (log_dir / "fixture.json").write_text(
        json.dumps(metadata, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    return metadata


def shutil_which(name: str) -> str | None:
    for part in os.environ.get("PATH", "").split(os.pathsep):
        candidate = pathlib.Path(part) / name
        if candidate.exists() and os.access(candidate, os.X_OK):
            return str(candidate)
    return None


def read_text(path: pathlib.Path) -> str:
    try:
        return path.read_text(encoding="utf-8", errors="replace")
    except FileNotFoundError:
        return ""


def start_pdf_server(
    log_dir: pathlib.Path,
    port: int,
    fixture_path: pathlib.Path,
) -> socketserver.TCPServer:
    FormPdfHandler.log_dir = log_dir
    FormPdfHandler.fixture_path = fixture_path
    FormPdfHandler.requests = []
    server = ReusableTcpServer(("127.0.0.1", port), FormPdfHandler)
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


def send_click(conn: socket.socket, state: HarnessState, x: float, y: float, name: str) -> None:
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
                "name": name,
                "type": event_type,
                "button": "left",
                "x": x,
                "y": y,
                "click_count": 1,
                "modifiers": 0,
            }
        )
        time.sleep(0.05)


def send_key(
    conn: socket.socket,
    state: HarnessState,
    event_type: str,
    windows_key_code: int,
    utf8: str = "",
) -> None:
    if not state.tab_ready_id:
        return
    send_message(conn, 9, key_event_payload(state.tab_ready_id, event_type, windows_key_code, utf8, 0))
    state.key_messages_sent.append(
        {
            "index": len(state.key_messages_sent),
            "type": event_type,
            "windows_key_code": windows_key_code,
            "utf8_len": len(utf8),
        }
    )
    time.sleep(0.04)


def send_text(conn: socket.socket, state: HarnessState, text: str) -> None:
    for ch in text:
        keycode = ord(ch.upper()) if ch.isalpha() else ord(ch)
        send_key(conn, state, "down", keycode, ch)
        send_key(conn, state, "up", keycode, "")


def send_escape(conn: socket.socket, state: HarnessState) -> None:
    send_key(conn, state, "down", VKEY_ESCAPE, "")
    send_key(conn, state, "up", VKEY_ESCAPE, "")


def run_snapshot(
    log_dir: pathlib.Path,
    devtools_port: int,
    url_contains: str,
    name: str,
    timeout_seconds: int,
    settle_seconds: int,
    action: str | None = None,
    action_x: float | None = None,
    action_y: float | None = None,
    action_text: str | None = None,
) -> dict[str, Any] | None:
    out_dir = log_dir / "devtools"
    out_dir.mkdir(parents=True, exist_ok=True)
    cmd = [
            "node",
            str(FORMS_PROBE),
            "--devtools-port",
            str(devtools_port),
            "--url-contains",
            url_contains,
            "--out-dir",
            str(out_dir),
            "--name",
            name,
            "--timeout-seconds",
            str(timeout_seconds),
            "--settle-seconds",
            str(settle_seconds),
    ]
    if action:
        cmd += ["--action", action]
    if action_x is not None:
        cmd += ["--action-x", str(action_x)]
    if action_y is not None:
        cmd += ["--action-y", str(action_y)]
    if action_text is not None:
        cmd += ["--action-text", action_text]
    proc = subprocess.run(
        cmd,
        cwd=str(ROOT),
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    (out_dir / f"{name}.stdout").write_text(proc.stdout, encoding="utf-8")
    (out_dir / f"{name}.stderr").write_text(proc.stderr, encoding="utf-8")
    path = out_dir / f"{name}.json"
    if not path.exists():
        return None
    return json.loads(path.read_text(encoding="utf-8"))


def record_devtools_action(state: HarnessState, name: str, summary: dict[str, Any] | None) -> None:
    if not summary or "actionResult" not in summary:
        return
    results = summary.get("actionResult") or []
    state.devtools_actions_sent.append(
        {
            "index": len(state.devtools_actions_sent),
            "name": name,
            "count": len(results),
            "ok": bool(results) and all(bool(item.get("ok")) for item in results),
            "results": results,
        }
    )


def pdf_value(summary: dict[str, Any] | None) -> dict[str, Any]:
    values = (summary or {}).get("values") or []
    for item in values:
        value = item.get("value", {})
        if value.get("viewerPresent") or value.get("pluginPresent"):
            return value
    return values[0].get("value", {}) if values else {}


def plugin_loaded(summary: dict[str, Any] | None) -> bool:
    return bool(summary and summary.get("pluginLoaded"))


def compute_field_geometry(summary: dict[str, Any], fixture: dict[str, Any]) -> dict[str, Any]:
    value = pdf_value(summary)
    plugin = value.get("pluginRect") or {}
    if not plugin or plugin.get("width", 0) <= 0 or plugin.get("height", 0) <= 0:
        return {"ok": False, "reason": "plugin-rect-missing"}
    page = fixture["page"]
    scale = min(float(plugin["width"]) / float(page["width"]), float(plugin["height"]) / float(page["height"]))
    rendered_width = float(page["width"]) * scale
    rendered_height = float(page["height"]) * scale
    offset_x = float(plugin["x"]) + (float(plugin["width"]) - rendered_width) / 2
    offset_y = float(plugin["y"]) + (float(plugin["height"]) - rendered_height) / 2
    fields = {}
    for name, field in fixture["fields"].items():
        x0, y0, x1, y1 = [float(v) for v in field["rect"]]
        rect = {
            "x": offset_x + x0 * scale,
            "y": offset_y + (float(page["height"]) - y1) * scale,
            "width": (x1 - x0) * scale,
            "height": (y1 - y0) * scale,
        }
        fields[name] = {
            "type": field["type"],
            "pdf_rect": field["rect"],
            "screen_rect": rect,
            "click": {"x": rect["x"] + rect["width"] / 2, "y": rect["y"] + rect["height"] / 2},
        }
    return {
        "ok": True,
        "plugin_rect": plugin,
        "page_rect": {"x": offset_x, "y": offset_y, "width": rendered_width, "height": rendered_height},
        "scale": scale,
        "fields": fields,
    }


def screenshot_path(log_dir: pathlib.Path, summary: dict[str, Any] | None) -> pathlib.Path | None:
    relative = ((summary or {}).get("screenshot") or {}).get("relativePath")
    if not relative:
        return None
    return log_dir / "devtools" / str(relative)


def png_rgba(path: pathlib.Path) -> tuple[int, int, bytes]:
    data = path.read_bytes()
    if not data.startswith(b"\x89PNG\r\n\x1a\n"):
        raise ValueError(f"not a PNG: {path}")
    offset = 8
    width = height = color_type = bit_depth = None
    compressed = bytearray()
    while offset < len(data):
        length = struct.unpack(">I", data[offset : offset + 4])[0]
        chunk_type = data[offset + 4 : offset + 8]
        chunk = data[offset + 8 : offset + 8 + length]
        offset += 12 + length
        if chunk_type == b"IHDR":
            width, height, bit_depth, color_type = struct.unpack(">IIBB", chunk[:10])
        elif chunk_type == b"IDAT":
            compressed.extend(chunk)
        elif chunk_type == b"IEND":
            break
    if width is None or height is None or bit_depth != 8 or color_type not in (2, 6):
        raise ValueError(f"unsupported PNG format: {path}")
    channels = 4 if color_type == 6 else 3
    stride = width * channels
    raw = zlib.decompress(bytes(compressed))
    out = bytearray(width * height * 4)
    prev = bytearray(stride)
    src = 0
    dst = 0
    for _ in range(height):
        filter_type = raw[src]
        src += 1
        row = bytearray(raw[src : src + stride])
        src += stride
        for i in range(stride):
            left = row[i - channels] if i >= channels else 0
            up = prev[i]
            up_left = prev[i - channels] if i >= channels else 0
            if filter_type == 1:
                row[i] = (row[i] + left) & 0xFF
            elif filter_type == 2:
                row[i] = (row[i] + up) & 0xFF
            elif filter_type == 3:
                row[i] = (row[i] + ((left + up) // 2)) & 0xFF
            elif filter_type == 4:
                p = left + up - up_left
                pa = abs(p - left)
                pb = abs(p - up)
                pc = abs(p - up_left)
                predictor = left if pa <= pb and pa <= pc else up if pb <= pc else up_left
                row[i] = (row[i] + predictor) & 0xFF
            elif filter_type != 0:
                raise ValueError(f"unsupported PNG filter {filter_type}: {path}")
        for x in range(width):
            out[dst : dst + 3] = row[x * channels : x * channels + 3]
            out[dst + 3] = row[x * channels + 3] if channels == 4 else 255
            dst += 4
        prev = row
    return width, height, bytes(out)


def screenshot_diff(
    log_dir: pathlib.Path,
    before: dict[str, Any] | None,
    after: dict[str, Any] | None,
    rect: dict[str, float],
    dpr: float,
) -> dict[str, Any]:
    try:
        before_path = screenshot_path(log_dir, before)
        after_path = screenshot_path(log_dir, after)
        if not before_path or not after_path:
            return {"ok": False, "reason": "screenshot-path-missing"}
        before_width, before_height, before_pixels = png_rgba(before_path)
        after_width, after_height, after_pixels = png_rgba(after_path)
        if (before_width, before_height) != (after_width, after_height):
            return {"ok": False, "reason": "screenshot-size-mismatch"}
        margin = 4
        x0 = max(0, int((float(rect["x"]) - margin) * dpr))
        y0 = max(0, int((float(rect["y"]) - margin) * dpr))
        x1 = min(before_width, int((float(rect["x"]) + float(rect["width"]) + margin) * dpr))
        y1 = min(before_height, int((float(rect["y"]) + float(rect["height"]) + margin) * dpr))
        in_changed = 0
        out_changed = 0
        in_total = max(0, x1 - x0) * max(0, y1 - y0)
        out_total = before_width * before_height - in_total
        for y in range(before_height):
            row = y * before_width * 4
            for x in range(before_width):
                pos = row + x * 4
                if before_pixels[pos : pos + 4] == after_pixels[pos : pos + 4]:
                    continue
                if x0 <= x < x1 and y0 <= y < y1:
                    in_changed += 1
                else:
                    out_changed += 1
        return {
            "ok": True,
            "inside_changed_pixels": in_changed,
            "outside_changed_pixels": out_changed,
            "inside_total_pixels": in_total,
            "outside_total_pixels": out_total,
            "inside_changed_ratio": in_changed / in_total if in_total else 0,
            "outside_changed_ratio": out_changed / out_total if out_total else 0,
            "rect_pixels": {"x0": x0, "y0": y0, "x1": x1, "y1": y1},
        }
    except Exception as error:
        return {"ok": False, "reason": str(error)}


def trace_flags(log_dir: pathlib.Path, state: HarnessState) -> None:
    trace = read_text(log_dir / "pdf-input.log")
    state.roamium_trace_init = "trace-init" in trace
    state.roamium_mouse_event_line = "mouse-event" in trace
    state.roamium_key_event_line = "key-event" in trace


def localized(diff: dict[str, Any]) -> bool:
    return bool(
        diff.get("ok")
        and diff.get("inside_changed_pixels", 0) > 8
        and diff.get("inside_changed_ratio", 0) > diff.get("outside_changed_ratio", 1) * 20
    )


def scenario_needs_text(scenario: str) -> bool:
    return scenario in (
        "text",
        "text-then-checkbox",
        "checkbox-then-text",
        "text-bg-checkbox",
        "text-escape-checkbox",
        "text-double-checkbox",
        "checkbox-bg-text",
        "checkbox-escape-text",
        "checkbox-double-text",
    )


def scenario_needs_checkbox(scenario: str) -> bool:
    return scenario in (
        "checkbox",
        "text-then-checkbox",
        "checkbox-then-text",
        "text-bg-checkbox",
        "text-escape-checkbox",
        "text-double-checkbox",
        "checkbox-bg-text",
        "checkbox-escape-text",
        "checkbox-double-text",
    )


def classify(
    input_path: str,
    scenario: str,
    state: HarnessState,
    before: dict[str, Any] | None,
    geometry: dict[str, Any],
    text_diff: dict[str, Any],
    checkbox_diff: dict[str, Any],
) -> None:
    if not state.server_register_received:
        state.first_failing_hop = "roamium-not-registered"
    elif not state.tab_ready_id:
        state.first_failing_hop = "tab-not-ready"
    elif not state.devtools_port:
        state.first_failing_hop = "devtools-target-discovery-failed"
    elif not before:
        state.first_failing_hop = "devtools-target-discovery-failed"
    elif not plugin_loaded(before):
        state.first_failing_hop = "pdf-load-failed"
    elif not geometry.get("ok"):
        state.first_failing_hop = "form-geometry-observable-missing"
    elif input_path == "devtools" and not state.devtools_actions_sent:
        state.first_failing_hop = "devtools-input-not-sent"
    elif input_path == "devtools" and not all(item.get("ok") for item in state.devtools_actions_sent):
        state.first_failing_hop = "devtools-input-state-missing"
    elif input_path == "termsurf" and (
        not state.mouse_messages_sent or (scenario_needs_text(scenario) and not state.key_messages_sent)
    ):
        state.first_failing_hop = "protocol-input-not-sent"
    elif input_path == "termsurf" and (
        not state.roamium_mouse_event_line or (scenario_needs_text(scenario) and not state.roamium_key_event_line)
    ):
        state.first_failing_hop = "roamium-input-trace-missing"
    elif scenario_needs_text(scenario) and not localized(text_diff):
        state.first_failing_hop = "form-text-value-missing"
    elif scenario_needs_checkbox(scenario) and not localized(checkbox_diff):
        state.first_failing_hop = "form-checkbox-state-missing"
    elif not scenario_needs_text(scenario) and not scenario_needs_checkbox(scenario):
        state.first_failing_hop = "automation-gap"
    else:
        state.first_failing_hop = "no-failure-observed"


def write_summary(log_dir: pathlib.Path, state: HarnessState, extra: dict[str, Any]) -> None:
    data = {
        "server_register_received": state.server_register_received,
        "create_tab_sent": state.create_tab_sent,
        "tab_ready_id": state.tab_ready_id,
        "resize_sent": state.resize_sent,
        "focus_sent": state.focus_sent,
        "devtools_port": state.devtools_port,
        "protocol_mouse_messages_sent": len(state.mouse_messages_sent),
        "protocol_mouse_messages": state.mouse_messages_sent,
        "protocol_key_messages_sent": len(state.key_messages_sent),
        "protocol_key_messages": state.key_messages_sent,
        "devtools_actions_sent": len(state.devtools_actions_sent),
        "devtools_actions": state.devtools_actions_sent,
        "roamium_trace_init": state.roamium_trace_init,
        "roamium_mouse_event_line": state.roamium_mouse_event_line,
        "roamium_key_event_line": state.roamium_key_event_line,
        "first_failing_hop": state.first_failing_hop,
    }
    data.update(extra)
    (log_dir / "pdf-forms-summary.json").write_text(
        json.dumps(data, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--log-dir", required=True)
    parser.add_argument(
        "--input-path",
        choices=["termsurf", "devtools", "compare"],
        default="termsurf",
    )
    parser.add_argument(
        "--scenario",
        choices=[
            "combined",
            "text",
            "checkbox",
            "text-then-checkbox",
            "checkbox-then-text",
            "text-bg-checkbox",
            "text-escape-checkbox",
            "text-double-checkbox",
            "checkbox-bg-text",
            "checkbox-escape-text",
            "checkbox-double-text",
        ],
        default="combined",
    )
    parser.add_argument("--width", type=int, default=1200)
    parser.add_argument("--height", type=int, default=900)
    parser.add_argument("--pdf-port", type=int, default=0)
    parser.add_argument("--setup-timeout", type=float, default=30)
    parser.add_argument("--capture-timeout-seconds", type=int, default=30)
    parser.add_argument("--settle-seconds", type=int, default=1)
    return parser.parse_args()


def run_combined(args: argparse.Namespace) -> int:
    log_dir = pathlib.Path(args.log_dir).resolve()
    log_dir.mkdir(parents=True, exist_ok=True)
    if args.input_path == "compare":
        path_runs: dict[str, dict[str, Any]] = {}
        path_commands: dict[str, dict[str, Any]] = {}
        for input_path in ("termsurf", "devtools"):
            path_dir = log_dir / input_path
            cmd = [
                sys.executable,
                str(pathlib.Path(__file__).resolve()),
                "--log-dir",
                str(path_dir),
                "--scenario",
                "combined",
                "--input-path",
                input_path,
                "--width",
                str(args.width),
                "--height",
                str(args.height),
                "--setup-timeout",
                str(args.setup_timeout),
                "--capture-timeout-seconds",
                str(args.capture_timeout_seconds),
                "--settle-seconds",
                str(args.settle_seconds),
            ]
            proc = subprocess.run(
                cmd,
                cwd=str(ROOT),
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                check=False,
            )
            path_commands[input_path] = {
                "cmd": cmd,
                "returncode": proc.returncode,
                "stdout": proc.stdout,
                "stderr": proc.stderr,
            }
            summary_path = path_dir / "pdf-forms-summary.json"
            path_runs[input_path] = (
                json.loads(summary_path.read_text(encoding="utf-8")) if summary_path.exists() else {}
            )

        def scenario_hop(path_summary: dict[str, Any], name: str) -> str | None:
            top_level = {
                "text": "text_scenario",
                "checkbox": "checkbox_scenario",
                "text-then-checkbox": "text_then_checkbox_scenario",
                "checkbox-then-text": "checkbox_then_text_scenario",
            }
            if name in top_level:
                return (path_summary.get(top_level[name]) or {}).get("first_failing_hop")
            return ((path_summary.get("focus_reset_scenarios") or {}).get(name) or {}).get(
                "first_failing_hop"
            )

        comparable_scenarios = (
            "text",
            "checkbox",
            "text-then-checkbox",
            "checkbox-then-text",
            "text-bg-checkbox",
            "checkbox-bg-text",
        )
        divergences = {}
        for name in comparable_scenarios:
            termsurf_hop = scenario_hop(path_runs.get("termsurf", {}), name)
            devtools_hop = scenario_hop(path_runs.get("devtools", {}), name)
            if termsurf_hop != devtools_hop:
                divergences[name] = {"termsurf": termsurf_hop, "devtools": devtools_hop}
        termsurf_hop = path_runs.get("termsurf", {}).get("first_failing_hop")
        devtools_hop = path_runs.get("devtools", {}).get("first_failing_hop")
        child_failure = any(item.get("returncode") != 0 for item in path_commands.values())
        if child_failure:
            first_failing_hop = "automation-gap"
        elif divergences:
            first_failing_hop = "termsurf-devtools-divergence"
        elif termsurf_hop == devtools_hop == "form-sequence-workaround-required":
            first_failing_hop = "chromium-pdf-focus-semantics"
        elif termsurf_hop == devtools_hop == "no-failure-observed":
            first_failing_hop = "no-failure-observed"
        else:
            first_failing_hop = termsurf_hop or devtools_hop or "devtools-input-state-missing"
        data = {
            "scenario": "combined",
            "input_path": "compare",
            "first_failing_hop": first_failing_hop,
            "termsurf_results": path_runs.get("termsurf"),
            "devtools_results": path_runs.get("devtools"),
            "input_path_divergences": divergences,
            "path_commands": path_commands,
        }
        (log_dir / "pdf-forms-summary.json").write_text(
            json.dumps(data, indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )
        return 0 if first_failing_hop in ("no-failure-observed", "chromium-pdf-focus-semantics") else 1

    runs: dict[str, dict[str, Any]] = {}
    commands: dict[str, dict[str, Any]] = {}
    scenarios = (
        "text",
        "checkbox",
        "text-then-checkbox",
        "checkbox-then-text",
        "text-bg-checkbox",
        "text-escape-checkbox",
        "text-double-checkbox",
        "checkbox-bg-text",
        "checkbox-escape-text",
        "checkbox-double-text",
    )
    scenario_dirs = {
        "text": "t",
        "checkbox": "c",
        "text-then-checkbox": "ttc",
        "checkbox-then-text": "ctt",
        "text-bg-checkbox": "tbc",
        "text-escape-checkbox": "tec",
        "text-double-checkbox": "tdc",
        "checkbox-bg-text": "cbt",
        "checkbox-escape-text": "cet",
        "checkbox-double-text": "cdt",
    }
    for scenario in scenarios:
        scenario_dir = log_dir / scenario_dirs[scenario]
        cmd = [
            sys.executable,
            str(pathlib.Path(__file__).resolve()),
            "--log-dir",
            str(scenario_dir),
            "--scenario",
            scenario,
            "--input-path",
            args.input_path,
            "--width",
            str(args.width),
            "--height",
            str(args.height),
            "--setup-timeout",
            str(args.setup_timeout),
            "--capture-timeout-seconds",
            str(args.capture_timeout_seconds),
            "--settle-seconds",
            str(args.settle_seconds),
        ]
        proc = subprocess.run(
            cmd,
            cwd=str(ROOT),
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            check=False,
        )
        commands[scenario] = {
            "cmd": cmd,
            "returncode": proc.returncode,
            "stdout": proc.stdout,
            "stderr": proc.stderr,
        }
        summary_path = scenario_dir / "pdf-forms-summary.json"
        runs[scenario] = json.loads(summary_path.read_text(encoding="utf-8")) if summary_path.exists() else {}
    text_ok = runs.get("text", {}).get("first_failing_hop") == "no-failure-observed"
    checkbox_ok = runs.get("checkbox", {}).get("first_failing_hop") == "no-failure-observed"
    sequence_failures = {
        name: runs.get(name, {}).get("first_failing_hop")
        for name in ("text-then-checkbox", "checkbox-then-text")
        if runs.get(name, {}).get("first_failing_hop") != "no-failure-observed"
    }
    reset_variants = (
        "text-bg-checkbox",
        "text-escape-checkbox",
        "text-double-checkbox",
        "checkbox-bg-text",
        "checkbox-escape-text",
        "checkbox-double-text",
    )
    focus_reset_results = {name: runs.get(name, {}).get("first_failing_hop") for name in reset_variants}
    focus_reset_successes = [
        name for name, result in focus_reset_results.items() if result == "no-failure-observed"
    ]
    if text_ok and checkbox_ok and not sequence_failures:
        first_failing_hop = "no-failure-observed"
    elif not text_ok:
        first_failing_hop = runs.get("text", {}).get("first_failing_hop") or "text-scenario-missing"
    elif not checkbox_ok:
        first_failing_hop = runs.get("checkbox", {}).get("first_failing_hop") or "checkbox-scenario-missing"
    elif focus_reset_successes:
        first_failing_hop = "form-sequence-workaround-required"
    else:
        first_failing_hop = next(iter(sequence_failures.values()), "sequence-scenario-missing")
    data = {
        "scenario": "combined",
        "input_path": args.input_path,
        "first_failing_hop": first_failing_hop,
        "text_scenario": runs.get("text"),
        "checkbox_scenario": runs.get("checkbox"),
        "text_then_checkbox_scenario": runs.get("text-then-checkbox"),
        "checkbox_then_text_scenario": runs.get("checkbox-then-text"),
        "sequence_failures": sequence_failures,
        "focus_reset_results": focus_reset_results,
        "focus_reset_successes": focus_reset_successes,
        "focus_reset_scenarios": {name: runs.get(name) for name in reset_variants},
        "scenario_commands": commands,
        "protocol_mouse_messages_sent": sum(
            int((runs.get(name) or {}).get("protocol_mouse_messages_sent") or 0)
            for name in scenarios
        ),
        "protocol_key_messages_sent": sum(
            int((runs.get(name) or {}).get("protocol_key_messages_sent") or 0)
            for name in scenarios
        ),
        "roamium_mouse_event_line": all(
            bool((runs.get(name) or {}).get("roamium_mouse_event_line"))
            for name in scenarios
        ),
        "roamium_key_event_line": all(
            bool((runs.get(name) or {}).get("roamium_key_event_line"))
            for name in ("text", "text-then-checkbox", "checkbox-then-text")
        ),
    }
    (log_dir / "pdf-forms-summary.json").write_text(
        json.dumps(data, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    return 0 if first_failing_hop != "automation-gap" else 1


def main() -> int:
    args = parse_args()
    if args.scenario == "combined":
        return run_combined(args)
    log_dir = pathlib.Path(args.log_dir).resolve()
    log_dir.mkdir(parents=True, exist_ok=True)
    if not ROAMIUM.exists():
        raise SystemExit(f"missing Roamium binary: {ROAMIUM}")
    if not FORMS_PROBE.exists():
        raise SystemExit(f"missing forms probe: {FORMS_PROBE}")
    fixture = prepare_fixture(log_dir)
    pdf_server = start_pdf_server(log_dir, args.pdf_port, pathlib.Path(fixture["path"]))
    host, port = pdf_server.server_address
    url = f"http://{host}:{port}/form.pdf"
    socket_tmp = tempfile.TemporaryDirectory(prefix="ts834-pdf-")
    socket_path = pathlib.Path(socket_tmp.name) / "gui.sock"

    state = HarnessState()
    extra: dict[str, Any] = {
        "scenario": args.scenario,
        "input_path": args.input_path,
        "url": url,
        "fixture": fixture,
        "http_server": {"host": host, "port": port},
        "typed_text": TEXT_VALUE,
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
        state.devtools_port = wait_for_devtools_port(log_dir, args.setup_timeout)
        before = None
        after_text = None
        after_checkbox = None
        after_reset = None
        geometry: dict[str, Any] = {"ok": False, "reason": "not-run"}
        text_diff: dict[str, Any] = {"ok": False, "reason": "not-run"}
        checkbox_diff: dict[str, Any] = {"ok": False, "reason": "not-run"}
        if state.devtools_port:
            before = run_snapshot(
                log_dir,
                state.devtools_port,
                "form.pdf",
                "before",
                args.capture_timeout_seconds,
                args.settle_seconds,
            )
            if before:
                geometry = compute_field_geometry(before, fixture)
                extra["geometry"] = geometry
                if geometry.get("ok"):
                    name_click = geometry["fields"]["name"]["click"]
                    checkbox_click = geometry["fields"]["agree"]["click"]
                    page_rect = geometry["page_rect"]
                    background_click = {
                        "x": page_rect["x"] + page_rect["width"] * 0.72,
                        "y": page_rect["y"] + page_rect["height"] * 0.62,
                    }

                    if args.scenario in (
                        "checkbox",
                        "checkbox-then-text",
                        "checkbox-bg-text",
                        "checkbox-escape-text",
                        "checkbox-double-text",
                    ):
                        if args.input_path == "devtools":
                            after_checkbox = run_snapshot(
                                log_dir,
                                state.devtools_port,
                                "form.pdf",
                                "after-checkbox",
                                args.capture_timeout_seconds,
                                args.settle_seconds,
                                action="click",
                                action_x=checkbox_click["x"],
                                action_y=checkbox_click["y"],
                            )
                            record_devtools_action(state, "agree", after_checkbox)
                        else:
                            send_click(conn, state, checkbox_click["x"], checkbox_click["y"], "agree")
                            time.sleep(0.5)
                            after_checkbox = run_snapshot(
                                log_dir,
                                state.devtools_port,
                                "form.pdf",
                                "after-checkbox",
                                args.capture_timeout_seconds,
                                args.settle_seconds,
                            )
                        if args.scenario == "checkbox-bg-text":
                            if args.input_path == "devtools":
                                after_reset = run_snapshot(
                                    log_dir,
                                    state.devtools_port,
                                    "form.pdf",
                                    "after-reset",
                                    args.capture_timeout_seconds,
                                    args.settle_seconds,
                                    action="click",
                                    action_x=background_click["x"],
                                    action_y=background_click["y"],
                                )
                                record_devtools_action(state, "background", after_reset)
                            else:
                                send_click(conn, state, background_click["x"], background_click["y"], "background")
                                time.sleep(0.2)
                                after_reset = run_snapshot(
                                    log_dir,
                                    state.devtools_port,
                                    "form.pdf",
                                    "after-reset",
                                    args.capture_timeout_seconds,
                                    args.settle_seconds,
                                )
                        elif args.scenario == "checkbox-escape-text":
                            if args.input_path == "devtools":
                                after_reset = run_snapshot(
                                    log_dir,
                                    state.devtools_port,
                                    "form.pdf",
                                    "after-reset",
                                    args.capture_timeout_seconds,
                                    args.settle_seconds,
                                    action="escape",
                                )
                                record_devtools_action(state, "escape", after_reset)
                            else:
                                send_escape(conn, state)
                                time.sleep(0.2)
                                after_reset = run_snapshot(
                                    log_dir,
                                    state.devtools_port,
                                    "form.pdf",
                                    "after-reset",
                                    args.capture_timeout_seconds,
                                    args.settle_seconds,
                                )
                    if args.scenario in (
                        "text",
                        "text-then-checkbox",
                        "checkbox-then-text",
                        "text-bg-checkbox",
                        "text-escape-checkbox",
                        "text-double-checkbox",
                        "checkbox-bg-text",
                        "checkbox-escape-text",
                        "checkbox-double-text",
                    ):
                        if args.input_path == "devtools":
                            after_text = run_snapshot(
                                log_dir,
                                state.devtools_port,
                                "form.pdf",
                                "after-name-click",
                                args.capture_timeout_seconds,
                                args.settle_seconds,
                                action="click",
                                action_x=name_click["x"],
                                action_y=name_click["y"],
                            )
                            record_devtools_action(state, "name", after_text)
                            if args.scenario == "checkbox-double-text":
                                after_text = run_snapshot(
                                    log_dir,
                                    state.devtools_port,
                                    "form.pdf",
                                    "after-name-double",
                                    args.capture_timeout_seconds,
                                    args.settle_seconds,
                                    action="click",
                                    action_x=name_click["x"],
                                    action_y=name_click["y"],
                                )
                                record_devtools_action(state, "name-double", after_text)
                            after_text = run_snapshot(
                                log_dir,
                                state.devtools_port,
                                "form.pdf",
                                "after-text",
                                args.capture_timeout_seconds,
                                args.settle_seconds,
                                action="text",
                                action_text=TEXT_VALUE,
                            )
                            record_devtools_action(state, "text", after_text)
                        else:
                            send_click(conn, state, name_click["x"], name_click["y"], "name")
                            if args.scenario == "checkbox-double-text":
                                send_click(conn, state, name_click["x"], name_click["y"], "name-double")
                            send_text(conn, state, TEXT_VALUE)
                            time.sleep(0.5)
                            after_text = run_snapshot(
                                log_dir,
                                state.devtools_port,
                                "form.pdf",
                                "after-text",
                                args.capture_timeout_seconds,
                                args.settle_seconds,
                            )
                    if args.scenario in (
                        "text-then-checkbox",
                        "text-bg-checkbox",
                        "text-escape-checkbox",
                        "text-double-checkbox",
                    ):
                        if args.scenario == "text-bg-checkbox":
                            if args.input_path == "devtools":
                                after_reset = run_snapshot(
                                    log_dir,
                                    state.devtools_port,
                                    "form.pdf",
                                    "after-reset",
                                    args.capture_timeout_seconds,
                                    args.settle_seconds,
                                    action="click",
                                    action_x=background_click["x"],
                                    action_y=background_click["y"],
                                )
                                record_devtools_action(state, "background", after_reset)
                            else:
                                send_click(conn, state, background_click["x"], background_click["y"], "background")
                                time.sleep(0.2)
                                after_reset = run_snapshot(
                                    log_dir,
                                    state.devtools_port,
                                    "form.pdf",
                                    "after-reset",
                                    args.capture_timeout_seconds,
                                    args.settle_seconds,
                                )
                        elif args.scenario == "text-escape-checkbox":
                            if args.input_path == "devtools":
                                after_reset = run_snapshot(
                                    log_dir,
                                    state.devtools_port,
                                    "form.pdf",
                                    "after-reset",
                                    args.capture_timeout_seconds,
                                    args.settle_seconds,
                                    action="escape",
                                )
                                record_devtools_action(state, "escape", after_reset)
                            else:
                                send_escape(conn, state)
                                time.sleep(0.2)
                                after_reset = run_snapshot(
                                    log_dir,
                                    state.devtools_port,
                                    "form.pdf",
                                    "after-reset",
                                    args.capture_timeout_seconds,
                                    args.settle_seconds,
                                )
                        if args.input_path == "devtools":
                            after_checkbox = run_snapshot(
                                log_dir,
                                state.devtools_port,
                                "form.pdf",
                                "after-checkbox",
                                args.capture_timeout_seconds,
                                args.settle_seconds,
                                action="click",
                                action_x=checkbox_click["x"],
                                action_y=checkbox_click["y"],
                            )
                            record_devtools_action(state, "agree", after_checkbox)
                            if args.scenario == "text-double-checkbox":
                                after_checkbox = run_snapshot(
                                    log_dir,
                                    state.devtools_port,
                                    "form.pdf",
                                    "after-checkbox-double",
                                    args.capture_timeout_seconds,
                                    args.settle_seconds,
                                    action="click",
                                    action_x=checkbox_click["x"],
                                    action_y=checkbox_click["y"],
                                )
                                record_devtools_action(state, "agree-double", after_checkbox)
                        else:
                            send_click(conn, state, checkbox_click["x"], checkbox_click["y"], "agree")
                            if args.scenario == "text-double-checkbox":
                                send_click(conn, state, checkbox_click["x"], checkbox_click["y"], "agree-double")
                            time.sleep(0.5)
                            after_checkbox = run_snapshot(
                                log_dir,
                                state.devtools_port,
                                "form.pdf",
                                "after-checkbox",
                                args.capture_timeout_seconds,
                                args.settle_seconds,
                            )
                    dpr = float((pdf_value(before).get("viewport") or {}).get("devicePixelRatio") or 1)
                    if after_text:
                        text_before = before
                        if args.scenario in ("checkbox-then-text", "checkbox-double-text"):
                            text_before = after_checkbox
                        elif args.scenario in ("checkbox-bg-text", "checkbox-escape-text"):
                            text_before = after_reset or after_checkbox
                        text_diff = screenshot_diff(
                            log_dir,
                            text_before,
                            after_text,
                            geometry["fields"]["name"]["screen_rect"],
                            dpr,
                        )
                    if after_checkbox:
                        checkbox_before = before
                        if args.scenario in ("text-then-checkbox", "text-double-checkbox"):
                            checkbox_before = after_text
                        elif args.scenario in ("text-bg-checkbox", "text-escape-checkbox"):
                            checkbox_before = after_reset or after_text
                        checkbox_diff = screenshot_diff(
                            log_dir,
                            checkbox_before,
                            after_checkbox,
                            geometry["fields"]["agree"]["screen_rect"],
                            dpr,
                        )
        trace_flags(log_dir, state)
        extra.update(
            {
                "before": before,
                "after_text": after_text,
                "after_checkbox": after_checkbox,
                "after_reset": after_reset,
                "text_diff": text_diff,
                "checkbox_diff": checkbox_diff,
                "http_requests": FormPdfHandler.requests,
            }
        )
        classify(args.input_path, args.scenario, state, before, geometry, text_diff, checkbox_diff)
        write_summary(log_dir, state, extra)
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
        socket_tmp.cleanup()


if __name__ == "__main__":
    sys.exit(main())
