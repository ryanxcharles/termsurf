#!/usr/bin/env python3
"""Probe Roamium PDF find/search through TermSurf keyboard input."""

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
FIND_PROBE = ROOT / "scripts/probe-pdf-find.mjs"
BITCOIN_PDF = ROOT / "test-html/public/bitcoin.pdf"
DEVTOOLS_RE = re.compile(r"DevTools listening on ws://127\.0\.0\.1:(\d+)/")
TARGET_TERM = "Bitcoin"

META_MODIFIER = 8
VKEY_F = 70
VKEY_ENTER = 13


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


class FindPdfHandler(http.server.BaseHTTPRequestHandler):
    log_dir: pathlib.Path
    pdf_bytes: bytes

    def log_message(self, fmt: str, *args: object) -> None:
        with (self.log_dir / "http.log").open("a", encoding="utf-8") as log:
            log.write((fmt % args) + "\n")

    def do_GET(self) -> None:
        if self.path.split("?", 1)[0] != "/pdf-find-fixture.pdf":
            self.send_response(404)
            self.end_headers()
            return
        data = self.pdf_bytes
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
    before_probe_status: str = "not-run"
    after_probe_status: str = "not-run"
    roamium_trace_init: bool = False
    roamium_focus_line: bool = False
    roamium_key_event_line: bool = False
    roamium_key_ffi_line: bool = False
    chromium_key_route_line: bool = False
    chromium_key_target_classification: str = "unknown"
    termsurf_find_command_line: bool = False
    pdf_find_started_line: bool = False
    first_failing_hop: str = "automation-gap"


def start_pdf_server(
    log_dir: pathlib.Path, port: int, pdf_bytes: bytes
) -> socketserver.TCPServer:
    FindPdfHandler.log_dir = log_dir
    FindPdfHandler.pdf_bytes = pdf_bytes
    server = ReusableTcpServer(("127.0.0.1", port), FindPdfHandler)
    host, bound_port = server.server_address
    (log_dir / "http-server.log").write_text(
        f"listening on {host}:{bound_port}\n",
        encoding="utf-8",
    )
    threading.Thread(target=server.serve_forever, daemon=True).start()
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
                        send_message(conn, 3, resize_payload(state.tab_ready_id, width, height))
                        state.resize_sent = True
                        messages.write("sent Resize\n")
                        messages.flush()
                        return
            except socket.timeout:
                pass


def send_focus(conn: socket.socket, state: HarnessState) -> None:
    if not state.tab_ready_id:
        return
    send_message(conn, 10, focus_payload(state.tab_ready_id, True))
    state.focus_sent = True
    time.sleep(0.1)


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


def send_key(
    conn: socket.socket,
    state: HarnessState,
    event_type: str,
    windows_key_code: int,
    utf8: str = "",
    modifiers: int = 0,
) -> None:
    if not state.tab_ready_id:
        return
    send_message(
        conn,
        9,
        key_event_payload(state.tab_ready_id, event_type, windows_key_code, utf8, modifiers),
    )
    state.key_messages_sent.append(
        {
            "index": len(state.key_messages_sent),
            "message": "key_event",
            "type": event_type,
            "windows_key_code": windows_key_code,
            "utf8_len": len(utf8),
            "modifiers": modifiers,
        }
    )
    time.sleep(0.04)


def send_command_shortcut(conn: socket.socket, state: HarnessState, keycode: int) -> None:
    send_key(conn, state, "down", keycode, "", META_MODIFIER)
    send_key(conn, state, "up", keycode, "", META_MODIFIER)


def send_text(conn: socket.socket, state: HarnessState, text: str) -> None:
    for ch in text:
        keycode = ord(ch.upper()) if ch.isalpha() else ord(ch)
        send_key(conn, state, "down", keycode, ch, 0)
        send_key(conn, state, "up", keycode, ch, 0)


def run_find_probe(
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
            str(FIND_PROBE),
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
        out_dir / "pdf-find-devtools-summary.json",
    )


def load_json(path: pathlib.Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def plugin_center(summary: dict[str, Any] | None) -> dict[str, float] | None:
    rect = (((summary or {}).get("snapshot") or {}).get("state", {}).get("value", {}).get("pluginRect"))
    try:
        return {
            "x": float(rect["x"]) + float(rect["width"]) / 2,
            "y": float(rect["y"]) + float(rect["height"]) / 2,
            "width": float(rect["width"]),
            "height": float(rect["height"]),
        }
    except (TypeError, KeyError, ValueError):
        return None


def page_value(summary: dict[str, Any] | None) -> str:
    return str(
        (((summary or {}).get("snapshot") or {}).get("state", {}).get("value", {})).get(
            "pageSelectorValue", ""
        )
    )


def screenshot_hash(summary: dict[str, Any] | None) -> str:
    return str((((summary or {}).get("snapshot") or {}).get("screenshot", {})).get("sha256", ""))


def screenshot_path(log_dir: pathlib.Path, out_name: str, summary: dict[str, Any] | None) -> pathlib.Path | None:
    relative = (((summary or {}).get("snapshot") or {}).get("screenshot", {})).get("relativePath")
    if not relative:
        return None
    return log_dir / out_name / str(relative)


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


def plugin_region_diff(
    log_dir: pathlib.Path,
    before_summary: dict[str, Any] | None,
    after_summary: dict[str, Any] | None,
) -> dict[str, Any]:
    try:
        before_path = screenshot_path(log_dir, "before", before_summary)
        after_path = screenshot_path(log_dir, "after", after_summary)
        if not before_path or not after_path:
            return {"ok": False, "reason": "screenshot-path-missing"}
        before_width, before_height, before_pixels = png_rgba(before_path)
        after_width, after_height, after_pixels = png_rgba(after_path)
        if (before_width, before_height) != (after_width, after_height):
            return {"ok": False, "reason": "screenshot-size-mismatch"}
        value = (((before_summary or {}).get("snapshot") or {}).get("state", {})).get("value", {})
        rect = value.get("pluginRect") or {}
        dpr = float((value.get("viewport") or {}).get("devicePixelRatio") or 1)
        x0 = max(0, int(float(rect["x"]) * dpr))
        y0 = max(0, int(float(rect["y"]) * dpr))
        x1 = min(before_width, int((float(rect["x"]) + float(rect["width"])) * dpr))
        y1 = min(before_height, int((float(rect["y"]) + float(rect["height"])) * dpr))
        changed = 0
        total = max(0, x1 - x0) * max(0, y1 - y0)
        for y in range(y0, y1):
            row = y * before_width * 4
            for x in range(x0, x1):
                pos = row + x * 4
                if before_pixels[pos : pos + 4] != after_pixels[pos : pos + 4]:
                    changed += 1
        return {
            "ok": True,
            "changed_pixels": changed,
            "total_pixels": total,
            "changed_ratio": changed / total if total else 0,
            "rect": {"x0": x0, "y0": y0, "x1": x1, "y1": y1},
        }
    except Exception as error:
        return {"ok": False, "reason": str(error)}


def trace_flags(log_dir: pathlib.Path, state: HarnessState) -> None:
    trace = read_text(log_dir / "pdf-input.log")
    stderr = read_text(log_dir / "roamium.stderr")
    state.roamium_trace_init = "trace-init" in trace
    state.roamium_focus_line = "focus-event" in trace
    state.roamium_key_event_line = "key-event" in trace
    state.roamium_key_ffi_line = "ffi=ts_forward_key_event" in trace
    state.chromium_key_route_line = "[termsurf-pdf-input] key-route" in stderr
    state.termsurf_find_command_line = "[termsurf-pdf-input] find-command" in stderr
    state.pdf_find_started_line = "startedFindInPage" in stderr or "StartFind" in stderr
    target_match = re.search(r"\[termsurf-pdf-input\] key-target .*classification=([^ ]+)", stderr)
    if target_match:
        state.chromium_key_target_classification = target_match.group(1)


def classify(
    state: HarnessState,
    before_summary: dict[str, Any] | None,
    after_summary: dict[str, Any] | None,
    image_diff: dict[str, Any] | None,
) -> None:
    before_page = page_value(before_summary)
    after_page = page_value(after_summary)
    before_hash = screenshot_hash(before_summary)
    after_hash = screenshot_hash(after_summary)
    if not state.server_register_received:
        state.first_failing_hop = "roamium-not-registered"
    elif not state.tab_ready_id:
        state.first_failing_hop = "tab-not-ready"
    elif not state.resize_sent:
        state.first_failing_hop = "resize-not-sent"
    elif not state.devtools_port:
        state.first_failing_hop = "devtools-missing"
    elif not before_summary:
        state.first_failing_hop = "before-summary-missing"
    elif not state.focus_sent:
        state.first_failing_hop = "focus-not-sent"
    elif not state.mouse_messages_sent:
        state.first_failing_hop = "protocol-plugin-click-not-sent"
    elif not state.key_messages_sent:
        state.first_failing_hop = "protocol-key-not-sent"
    elif not state.roamium_key_event_line:
        state.first_failing_hop = "roamium-key-receive-missing"
    elif not state.roamium_key_ffi_line:
        state.first_failing_hop = "roamium-key-ffi-missing"
    elif not state.chromium_key_route_line:
        state.first_failing_hop = "chromium-key-route-missing"
    elif not after_summary:
        state.first_failing_hop = "after-summary-missing"
    elif after_page == "2" and before_page != after_page:
        state.first_failing_hop = "no-failure-observed"
    elif (
        before_hash
        and after_hash
        and before_hash != after_hash
        and state.termsurf_find_command_line
        and bool((image_diff or {}).get("ok"))
        and int((image_diff or {}).get("changed_pixels") or 0) > 0
    ):
        state.first_failing_hop = "no-failure-observed"
    else:
        state.first_failing_hop = "pdf-find-search-no-match-observed"


def write_summary(
    log_dir: pathlib.Path,
    args: argparse.Namespace,
    state: HarnessState,
    extra: dict[str, Any],
) -> None:
    data = {
        "probe": args.probe,
        "target_term": TARGET_TERM,
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
        "before_probe_status": state.before_probe_status,
        "after_probe_status": state.after_probe_status,
        "roamium_trace_init": state.roamium_trace_init,
        "roamium_focus_line": state.roamium_focus_line,
        "roamium_key_event_line": state.roamium_key_event_line,
        "roamium_key_ffi_line": state.roamium_key_ffi_line,
        "chromium_key_route_line": state.chromium_key_route_line,
        "chromium_key_target_classification": state.chromium_key_target_classification,
        "termsurf_find_command_line": state.termsurf_find_command_line,
        "pdf_find_started_line": state.pdf_find_started_line,
        "first_failing_hop": state.first_failing_hop,
    }
    data.update(extra)
    (log_dir / "pdf-find-summary.json").write_text(
        json.dumps(data, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--log-dir", required=True)
    parser.add_argument("--width", type=int, default=1200)
    parser.add_argument("--height", type=int, default=900)
    parser.add_argument("--pdf-port", type=int, default=9799)
    parser.add_argument("--url-contains", default="pdf-find-fixture.pdf")
    parser.add_argument("--setup-timeout", type=float, default=30)
    parser.add_argument("--capture-timeout-seconds", type=int, default=30)
    parser.add_argument("--settle-seconds", type=int, default=4)
    parser.add_argument("--probe", choices=["positive-search"], required=True)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    log_dir = pathlib.Path(args.log_dir).resolve()
    args.log_dir = str(log_dir)
    log_dir.mkdir(parents=True, exist_ok=True)
    if not ROAMIUM.exists():
        raise SystemExit(f"missing Roamium binary: {ROAMIUM}")
    if not FIND_PROBE.exists():
        raise SystemExit(f"missing find probe: {FIND_PROBE}")
    if not BITCOIN_PDF.exists():
        raise SystemExit(f"missing Bitcoin PDF fixture: {BITCOIN_PDF}")

    pdf_bytes = BITCOIN_PDF.read_bytes()
    (log_dir / "pdf-find-fixture.pdf").write_bytes(pdf_bytes)
    pdf_server = start_pdf_server(log_dir, args.pdf_port, pdf_bytes)
    url = f"http://127.0.0.1:{args.pdf_port}/pdf-find-fixture.pdf"

    socket_path = log_dir / "gui.sock"
    try:
        socket_path.unlink()
    except FileNotFoundError:
        pass

    state = HarnessState()
    extra: dict[str, Any] = {
        "url": url,
        "fixture": str(BITCOIN_PDF.relative_to(ROOT)),
        "fixture_bytes": len(pdf_bytes),
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
        before_summary = None
        after_summary = None
        image_diff = None

        if state.devtools_port:
            state.before_probe_status, before_error, before_path = run_find_probe(
                log_dir,
                state.devtools_port,
                args.url_contains,
                "before",
                args.capture_timeout_seconds,
                args.settle_seconds,
            )
            extra["before_probe_error"] = before_error
            before_summary = load_json(before_path) if before_path.exists() else None
            extra["before_summary"] = before_summary

            send_focus(conn, state)
            center = plugin_center(before_summary)
            extra["plugin_click_point"] = center
            if center:
                send_click(conn, state, center["x"], center["y"])
                time.sleep(0.2)
            send_command_shortcut(conn, state, VKEY_F)
            time.sleep(0.2)
            send_text(conn, state, TARGET_TERM)
            send_key(conn, state, "down", VKEY_ENTER)
            send_key(conn, state, "up", VKEY_ENTER)
            time.sleep(5.0)

            state.after_probe_status, after_error, after_path = run_find_probe(
                log_dir,
                state.devtools_port,
                args.url_contains,
                "after",
                args.capture_timeout_seconds,
                1,
            )
            extra["after_probe_error"] = after_error
            after_summary = load_json(after_path) if after_path.exists() else None
            extra["after_summary"] = after_summary
            extra["before_page"] = page_value(before_summary)
            extra["after_page"] = page_value(after_summary)
            extra["screenshot_changed"] = screenshot_hash(before_summary) != screenshot_hash(
                after_summary
            )
            image_diff = plugin_region_diff(log_dir, before_summary, after_summary)
            extra["plugin_region_diff"] = image_diff

        trace_flags(log_dir, state)
        classify(state, before_summary, after_summary, image_diff)
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
