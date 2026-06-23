#!/usr/bin/env python3
"""Probe Roamium PDF keyboard and page-selector navigation."""

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
NAVIGATION_PROBE = ROOT / "scripts/probe-pdf-navigation.mjs"
DEVTOOLS_RE = re.compile(r"DevTools listening on ws://127\.0\.0\.1:(\d+)/")

VKEY_PAGEDOWN = 34
VKEY_SPACE = 32
VKEY_ARROW_DOWN = 40


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


class ReusableTcpServer(socketserver.TCPServer):
    allow_reuse_address = True


class PdfHandler(http.server.BaseHTTPRequestHandler):
    log_dir: pathlib.Path

    def log_message(self, fmt: str, *args: object) -> None:
        with (self.log_dir / "http.log").open("a", encoding="utf-8") as log:
            log.write((fmt % args) + "\n")

    def do_GET(self) -> None:
        if self.path.split("?", 1)[0] != "/bitcoin.pdf":
            self.send_response(404)
            self.end_headers()
            return
        data = BITCOIN_PDF.read_bytes()
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
    mouse_messages_sent: list[dict[str, Any]] = dataclass_field(default_factory=list)
    devtools_port: int | None = None
    key_messages_sent: list[dict[str, Any]] = dataclass_field(default_factory=list)
    before_probe_status: str = "not-run"
    after_probe_status: str = "not-run"
    navigation_probe_status: str = "not-run"
    roamium_trace_init: bool = False
    roamium_focus_line: bool = False
    roamium_focus_ffi_line: bool = False
    roamium_key_event_line: bool = False
    roamium_key_ffi_line: bool = False
    chromium_focus_line: bool = False
    chromium_key_route_line: bool = False
    chromium_key_root_direct_line: bool = False
    chromium_key_target_classification: str = "unknown"
    first_failing_hop: str = "automation-gap"


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


def start_pdf_server(log_dir: pathlib.Path, port: int) -> socketserver.TCPServer:
    PdfHandler.log_dir = log_dir
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
        key_event_payload(
            state.tab_ready_id,
            event_type,
            windows_key_code,
            utf8,
            modifiers,
        ),
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
    time.sleep(0.05)


def send_key_press(
    conn: socket.socket,
    state: HarnessState,
    windows_key_code: int,
    utf8: str = "",
) -> None:
    send_key(conn, state, "down", windows_key_code, utf8)
    send_key(conn, state, "up", windows_key_code, utf8)


def run_navigation_probe(
    log_dir: pathlib.Path,
    devtools_port: int,
    url_contains: str,
    out_name: str,
    probe: str,
    timeout_seconds: int,
    settle_seconds: int,
) -> tuple[str, str, pathlib.Path]:
    out_dir = log_dir / out_name
    out_dir.mkdir(parents=True, exist_ok=True)
    proc = subprocess.run(
        [
            "node",
            str(NAVIGATION_PROBE),
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
        out_dir / "pdf-navigation-devtools-summary.json",
    )


def load_json(path: pathlib.Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def trace_flags(log_dir: pathlib.Path, state: HarnessState) -> None:
    trace = read_text(log_dir / "pdf-input.log")
    stderr = read_text(log_dir / "roamium.stderr")
    state.roamium_trace_init = "trace-init" in trace
    state.roamium_focus_line = "focus-event" in trace
    state.roamium_focus_ffi_line = "ffi=ts_set_focus" in trace
    state.roamium_key_event_line = "key-event" in trace
    state.roamium_key_ffi_line = "ffi=ts_forward_key_event" in trace
    state.chromium_focus_line = "[termsurf-pdf-input] focus" in stderr
    state.chromium_key_route_line = "[termsurf-pdf-input] key-route" in stderr
    state.chromium_key_root_direct_line = "key-route route_mode=root-direct" in stderr
    target_match = re.search(r"\[termsurf-pdf-input\] key-target .*classification=([^ ]+)", stderr)
    if target_match:
        state.chromium_key_target_classification = target_match.group(1)


def state_changed(before: dict[str, Any] | None, after: dict[str, Any] | None) -> bool:
    if not before or not after:
        return False
    before_state = before.get("before", before).get("state", {}).get("value", {})
    after_state = after.get("after", after).get("state", {}).get("value", {})
    return (
        before_state.get("pageSelectorValue") != after_state.get("pageSelectorValue")
        or before_state.get("documentScroll") != after_state.get("documentScroll")
        or before_state.get("viewerProps") != after_state.get("viewerProps")
        or before_state.get("controllerProps") != after_state.get("controllerProps")
    )


def screenshot_changed(before: dict[str, Any] | None, after: dict[str, Any] | None) -> bool:
    if not before or not after:
        return False
    before_sha = before.get("before", before).get("screenshot", {}).get("sha256")
    after_sha = after.get("after", after).get("screenshot", {}).get("sha256")
    return bool(before_sha and after_sha and before_sha != after_sha)


def plugin_center(summary: dict[str, Any] | None) -> dict[str, float] | None:
    rect = (
        ((summary or {}).get("before") or {})
        .get("state", {})
        .get("value", {})
        .get("pluginRect")
    )
    try:
        x = float(rect["x"]) + float(rect["width"]) / 2
        y = float(rect["y"]) + float(rect["height"]) / 2
        return {"x": x, "y": y, "width": float(rect["width"]), "height": float(rect["height"])}
    except (TypeError, KeyError, ValueError):
        return None


def classify(
    args: argparse.Namespace,
    state: HarnessState,
    before_summary: dict[str, Any] | None,
    after_summary: dict[str, Any] | None,
    navigation_summary: dict[str, Any] | None,
) -> None:
    if not state.server_register_received:
        state.first_failing_hop = "roamium-not-registered"
    elif not state.tab_ready_id:
        state.first_failing_hop = "tab-not-ready"
    elif not state.resize_sent:
        state.first_failing_hop = "resize-not-sent"
    elif not state.devtools_port:
        state.first_failing_hop = "devtools-missing"
    elif args.probe == "keyboard-page-scroll" and not state.focus_sent:
        state.first_failing_hop = "focus-not-sent"
    elif args.probe == "keyboard-page-scroll" and not state.mouse_messages_sent:
        state.first_failing_hop = "protocol-plugin-click-not-sent"
    elif args.probe == "keyboard-page-scroll" and not state.key_messages_sent:
        state.first_failing_hop = "protocol-key-not-sent"
    elif args.probe == "keyboard-page-scroll" and not state.roamium_key_event_line:
        state.first_failing_hop = "roamium-key-receive-missing"
    elif args.probe == "keyboard-page-scroll" and not state.roamium_key_ffi_line:
        state.first_failing_hop = "roamium-key-ffi-missing"
    elif args.probe == "keyboard-page-scroll" and not state.chromium_key_route_line:
        state.first_failing_hop = "chromium-key-route-missing"
    elif args.probe == "keyboard-page-scroll" and not (
        state_changed(before_summary, after_summary)
        or screenshot_changed(before_summary, after_summary)
    ):
        state.first_failing_hop = "pdf-keyboard-navigation-no-change"
    elif args.probe == "toolbar-page-selector" and not navigation_summary:
        state.first_failing_hop = "navigation-summary-missing"
    elif args.probe == "toolbar-page-selector" and navigation_summary.get("status") != "pass":
        state.first_failing_hop = navigation_summary.get(
            "firstFailingHop",
            "page-selector-navigation-failed",
        )
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
        "focus_sent": state.focus_sent,
        "devtools_port": state.devtools_port,
        "protocol_mouse_messages_sent": len(state.mouse_messages_sent),
        "protocol_mouse_messages": state.mouse_messages_sent,
        "protocol_key_messages_sent": len(state.key_messages_sent),
        "protocol_key_messages": state.key_messages_sent,
        "before_probe_status": state.before_probe_status,
        "after_probe_status": state.after_probe_status,
        "navigation_probe_status": state.navigation_probe_status,
        "roamium_trace_init": state.roamium_trace_init,
        "roamium_focus_line": state.roamium_focus_line,
        "roamium_focus_ffi_line": state.roamium_focus_ffi_line,
        "roamium_key_event_line": state.roamium_key_event_line,
        "roamium_key_ffi_line": state.roamium_key_ffi_line,
        "chromium_focus_line": state.chromium_focus_line,
        "chromium_key_route_line": state.chromium_key_route_line,
        "chromium_key_root_direct_line": state.chromium_key_root_direct_line,
        "chromium_key_target_classification": state.chromium_key_target_classification,
        "first_failing_hop": state.first_failing_hop,
    }
    data.update(extra)
    (log_dir / "pdf-navigation-summary.json").write_text(
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
        choices=["keyboard-page-scroll", "toolbar-page-selector"],
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
    if not NAVIGATION_PROBE.exists():
        raise SystemExit(f"missing navigation probe: {NAVIGATION_PROBE}")
    if args.serve_bitcoin_pdf and not BITCOIN_PDF.exists():
        raise SystemExit(f"missing Bitcoin PDF fixture: {BITCOIN_PDF}")

    url = args.url
    pdf_server = None
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
        navigation_summary = None

        if state.devtools_port and args.probe == "keyboard-page-scroll":
            state.before_probe_status, before_error, before_path = run_navigation_probe(
                log_dir,
                state.devtools_port,
                args.url_contains,
                "before",
                "snapshot",
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
                time.sleep(0.15)
            send_key_press(conn, state, VKEY_PAGEDOWN)
            send_key_press(conn, state, VKEY_SPACE, " ")
            send_key_press(conn, state, VKEY_ARROW_DOWN)
            time.sleep(0.8)

            state.after_probe_status, after_error, after_path = run_navigation_probe(
                log_dir,
                state.devtools_port,
                args.url_contains,
                "after",
                "snapshot",
                args.capture_timeout_seconds,
                1,
            )
            extra["after_probe_error"] = after_error
            after_summary = load_json(after_path) if after_path.exists() else None
            extra["after_summary"] = after_summary
            extra["state_changed"] = state_changed(before_summary, after_summary)
            extra["screenshot_changed"] = screenshot_changed(before_summary, after_summary)
        elif state.devtools_port and args.probe == "toolbar-page-selector":
            state.navigation_probe_status, probe_error, probe_path = run_navigation_probe(
                log_dir,
                state.devtools_port,
                args.url_contains,
                "toolbar-page-selector",
                "toolbar-page-selector",
                args.capture_timeout_seconds,
                args.settle_seconds,
            )
            extra["navigation_probe_error"] = probe_error
            navigation_summary = load_json(probe_path) if probe_path.exists() else None
            extra["navigation_summary"] = navigation_summary

        trace_flags(log_dir, state)
        classify(args, state, before_summary, after_summary, navigation_summary)
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
        if pdf_server:
            pdf_server.shutdown()


if __name__ == "__main__":
    sys.exit(main())
