#!/usr/bin/env python3
"""Drive PDF scrolling through the TermSurf protocol.

This harness runs Roamium against a minimal fake GUI socket, creates a PDF tab,
captures before/after DevTools probe artifacts, sends TermSurf ScrollEvent
messages directly to Roamium, and writes a hop-by-hop JSON summary.
"""

from __future__ import annotations

import argparse
import hashlib
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
CAPTURE_SCRIPT = ROOT / "scripts/capture-pdf-interactions.mjs"
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


def scroll_payload(
    tab_id: int,
    x: float,
    y: float,
    delta_x: float,
    delta_y: float,
    phase: int,
    momentum_phase: int,
    precise: bool,
    modifiers: int,
) -> bytes:
    return (
        varint_field(1, tab_id)
        + double_field(2, x)
        + double_field(3, y)
        + double_field(4, delta_x)
        + double_field(5, delta_y)
        + varint_field(6, phase)
        + varint_field(7, momentum_phase)
        + bool_field(8, precise)
        + varint_field(9, modifiers)
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
    devtools_port: int | None = None
    scroll_events_sent: int = 0
    scroll_events: list[dict[str, Any]] = dataclass_field(default_factory=list)
    coordinate_source: str = "not-selected"
    coordinates: dict[str, float] | None = None
    before_capture_status: str = "not-run"
    after_capture_status: str = "not-run"
    before_after_state_changed: bool = False
    before_after_screenshot_changed: bool = False
    roamium_trace_init: bool = False
    roamium_scroll_event_line: bool = False
    roamium_ffi_line: bool = False
    first_failing_hop: str = "automation-gap"


def start_pdf_server(log_dir: pathlib.Path, port: int) -> socketserver.TCPServer:
    PdfHandler.log_dir = log_dir
    try:
        server = ReusableTcpServer(("127.0.0.1", port), PdfHandler)
    except OSError as err:
        (log_dir / "http-server.log").write_text(
            f"failed to bind 127.0.0.1:{port}: {err}\n",
            encoding="utf-8",
        )
        raise SystemExit(f"failed to bind PDF fixture server on 127.0.0.1:{port}: {err}")
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
    stdout_path = log_dir / "roamium.stdout"
    stderr_path = log_dir / "roamium.stderr"
    while time.time() < deadline:
        text = read_text(stdout_path) + "\n" + read_text(stderr_path)
        match = DEVTOOLS_RE.search(text)
        if match:
            return int(match.group(1))
        time.sleep(0.1)
    return None


def run_capture(
    log_dir: pathlib.Path,
    devtools_port: int,
    url_contains: str,
    out_name: str,
    timeout_seconds: int,
    settle_seconds: int,
) -> tuple[str, str]:
    out_dir = log_dir / out_name
    out_dir.mkdir(parents=True, exist_ok=True)
    proc = subprocess.run(
        [
            "node",
            str(CAPTURE_SCRIPT),
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
            "--mode",
            "probe",
        ],
        cwd=str(ROOT),
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    (out_dir / "capture.stdout").write_text(proc.stdout, encoding="utf-8")
    (out_dir / "capture.stderr").write_text(proc.stderr, encoding="utf-8")
    return ("ok" if proc.returncode == 0 else "error", proc.stderr.strip())


def load_json(path: pathlib.Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def all_probe_states(summary: dict[str, Any]) -> list[dict[str, Any]]:
    states: list[dict[str, Any]] = []
    if isinstance(summary.get("_baselineState"), dict):
        states.append(summary["_baselineState"])
    baseline = summary.get("results", [{}])[0]
    after = baseline.get("details", {}).get("after", {})
    if isinstance(after.get("state"), dict):
        states.append(after["state"])
    for attempt in summary.get("attachAttempts", []):
        state = attempt.get("state")
        if isinstance(state, dict):
            states.append(state)
    return states


def load_probe_summary(out_dir: pathlib.Path) -> dict[str, Any]:
    summary = load_json(out_dir / "summary.json")
    try:
        baseline = load_json(out_dir / "baseline.json")
        if isinstance(baseline.get("state"), dict):
            summary["_baselineState"] = baseline["state"]
    except FileNotFoundError:
        pass
    return summary


def elements_from_summary(summary: dict[str, Any]) -> list[dict[str, Any]]:
    elements: list[dict[str, Any]] = []
    for state in all_probe_states(summary):
        value = state.get("value") or {}
        elements.extend(value.get("elements") or [])
        for child in value.get("childStates") or []:
            child_value = child.get("state", {}).get("value") or {}
            elements.extend(child_value.get("elements") or [])
    return elements


def element_bounds(element: dict[str, Any]) -> dict[str, float] | None:
    rect = element.get("rect") or element.get("bounds") or element
    try:
        x = float(rect["x"])
        y = float(rect["y"])
        width = float(rect["width"])
        height = float(rect["height"])
    except (KeyError, TypeError, ValueError):
        return None
    if width <= 0 or height <= 0:
        return None
    return {"x": x, "y": y, "width": width, "height": height}


def choose_scroll_point(
    summary: dict[str, Any],
    viewport_width: int,
    viewport_height: int,
) -> tuple[str, dict[str, float]]:
    candidates = []
    for element in elements_from_summary(summary):
        tag = str(element.get("tagName") or element.get("tag") or "").upper()
        element_id = str(element.get("id") or "").lower()
        bounds = element_bounds(element)
        if not bounds:
            continue
        candidates.append((tag, element_id, bounds))

    preferences = [
        ("plugin-bounds", lambda tag, eid: tag == "EMBED" and eid == "plugin"),
        ("container-bounds", lambda _tag, eid: eid == "container"),
        ("viewer-bounds", lambda tag, eid: tag == "PDF-VIEWER" or eid == "viewer"),
    ]
    for source, predicate in preferences:
        for tag, element_id, bounds in candidates:
            if predicate(tag, element_id):
                return (
                    source,
                    {
                        "x": bounds["x"] + bounds["width"] / 2,
                        "y": bounds["y"] + min(bounds["height"] / 2, 300.0),
                        "width": bounds["width"],
                        "height": bounds["height"],
                    },
                )

    viewport = viewport_from_summary(summary)
    fallback_width = viewport.get("innerWidth", viewport_width)
    fallback_height = viewport.get("innerHeight", viewport_height)
    return (
        "fixed-fallback",
        {
            "x": fallback_width / 2,
            "y": fallback_height / 2,
            "width": float(fallback_width),
            "height": float(fallback_height),
        },
    )


def viewport_from_summary(summary: dict[str, Any]) -> dict[str, float]:
    for state in all_probe_states(summary):
        viewport = (state.get("value") or {}).get("viewport") or {}
        try:
            width = float(viewport["innerWidth"])
            height = float(viewport["innerHeight"])
        except (KeyError, TypeError, ValueError):
            continue
        if width > 0 and height > 0:
            return {"innerWidth": width, "innerHeight": height}
    return {}


def sha256_file(path: pathlib.Path) -> str | None:
    try:
        return hashlib.sha256(path.read_bytes()).hexdigest()
    except FileNotFoundError:
        return None


def significant_state(summary: dict[str, Any]) -> dict[str, Any]:
    states = []
    for state in all_probe_states(summary):
        value = state.get("value") or {}
        states.append(
            {
                "url": value.get("url"),
                "title": value.get("title"),
                "scrollY": value.get("scrollY"),
                "scrollTop": value.get("scrollTop"),
                "scroll": value.get("scroll"),
                "viewport": value.get("viewport"),
                "activeElement": value.get("activeElement"),
                "elements": value.get("elements"),
                "childStates": value.get("childStates"),
            }
        )
    return {"states": states}


def classify(state: HarnessState, trace_file: pathlib.Path) -> None:
    trace = read_text(trace_file)
    state.roamium_trace_init = "trace-init" in trace
    state.roamium_scroll_event_line = "scroll-event" in trace
    state.roamium_ffi_line = "ffi=ts_forward_scroll_event" in trace

    if not state.server_register_received:
        state.first_failing_hop = "roamium-not-registered"
    elif not state.tab_ready_id:
        state.first_failing_hop = "tab-not-ready"
    elif not state.resize_sent:
        state.first_failing_hop = "resize-not-sent"
    elif state.scroll_events_sent == 0:
        state.first_failing_hop = "protocol-scroll-not-sent"
    elif not state.roamium_trace_init and not state.roamium_scroll_event_line:
        state.first_failing_hop = "trace-env-not-inherited"
    elif not state.roamium_scroll_event_line:
        state.first_failing_hop = "roamium-receive-missing"
    elif not state.roamium_ffi_line:
        state.first_failing_hop = "chromium-ffi-missing"
    elif not (
        state.before_after_state_changed or state.before_after_screenshot_changed
    ):
        state.first_failing_hop = "chromium-or-pdf-routing"
    else:
        state.first_failing_hop = "no-failure-observed"


def write_summary(
    log_dir: pathlib.Path,
    state: HarnessState,
    extra: dict[str, Any] | None = None,
) -> None:
    data = {
        "server_register_received": state.server_register_received,
        "create_tab_sent": state.create_tab_sent,
        "tab_ready_id": state.tab_ready_id,
        "resize_sent": state.resize_sent,
        "devtools_port": state.devtools_port,
        "scroll_events_sent": state.scroll_events_sent,
        "scroll_events": state.scroll_events,
        "scroll_coordinate_source": state.coordinate_source,
        "scroll_coordinates": state.coordinates,
        "before_capture_status": state.before_capture_status,
        "after_capture_status": state.after_capture_status,
        "roamium_trace_init": state.roamium_trace_init,
        "roamium_scroll_event_line": state.roamium_scroll_event_line,
        "roamium_ffi_line": state.roamium_ffi_line,
        "before_after_state_changed": state.before_after_state_changed,
        "before_after_screenshot_changed": state.before_after_screenshot_changed,
        "first_failing_hop": state.first_failing_hop,
    }
    if extra:
        data.update(extra)
    (log_dir / "protocol-scroll-summary.json").write_text(
        json.dumps(data, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )


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
                    messages.flush()
            except socket.timeout:
                pass


def send_scroll_events(
    conn: socket.socket,
    state: HarnessState,
    count: int,
    x: float,
    y: float,
    delta_y: float,
) -> None:
    if not state.tab_ready_id:
        return
    for index in range(count):
        phase = 1 if index == 0 else 4
        send_message(
            conn,
            8,
            scroll_payload(
                state.tab_ready_id,
                x,
                y,
                0.0,
                delta_y,
                phase,
                0,
                True,
                0,
            ),
        )
        state.scroll_events_sent += 1
        state.scroll_events.append(
            {
                "index": index,
                "tab_id": state.tab_ready_id,
                "x": x,
                "y": y,
                "delta_x": 0.0,
                "delta_y": delta_y,
                "phase": phase,
                "momentum_phase": 0,
                "precise": True,
                "modifiers": 0,
            }
        )
        time.sleep(0.05)
    send_message(
        conn,
        8,
        scroll_payload(state.tab_ready_id, x, y, 0.0, 0.0, 8, 0, True, 0),
    )
    state.scroll_events_sent += 1
    state.scroll_events.append(
        {
            "index": count,
            "tab_id": state.tab_ready_id,
            "x": x,
            "y": y,
            "delta_x": 0.0,
            "delta_y": 0.0,
            "phase": 8,
            "momentum_phase": 0,
            "precise": True,
            "modifiers": 0,
        }
    )


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("url", nargs="?")
    parser.add_argument("--log-dir", required=True)
    parser.add_argument("--width", type=int, default=1200)
    parser.add_argument("--height", type=int, default=900)
    parser.add_argument("--serve-bitcoin-pdf", action="store_true")
    parser.add_argument("--url-contains", default="bitcoin.pdf")
    parser.add_argument("--pdf-port", type=int, default=9787)
    parser.add_argument("--setup-timeout", type=float, default=30)
    parser.add_argument("--capture-timeout-seconds", type=int, default=30)
    parser.add_argument("--settle-seconds", type=int, default=8)
    parser.add_argument("--scroll-count", type=int, default=5)
    parser.add_argument("--scroll-delta-y", type=float, default=-600.0)
    parser.add_argument("--post-scroll-settle-seconds", type=float, default=1.0)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    log_dir = pathlib.Path(args.log_dir).resolve()
    log_dir.mkdir(parents=True, exist_ok=True)
    trace_file = pathlib.Path(
        os.environ.get(
            "TERMSURF_PDF_INPUT_TRACE_FILE",
            str(log_dir / "pdf-input.log"),
        )
    ).resolve()
    trace_file.parent.mkdir(parents=True, exist_ok=True)

    if not ROAMIUM.exists():
        raise SystemExit(f"missing Roamium binary: {ROAMIUM}")
    if args.serve_bitcoin_pdf and not BITCOIN_PDF.exists():
        raise SystemExit(f"missing Bitcoin PDF fixture: {BITCOIN_PDF}")

    url = args.url
    pdf_server = None
    if args.serve_bitcoin_pdf:
        pdf_server = start_pdf_server(log_dir, args.pdf_port)
        url = url or f"http://127.0.0.1:{args.pdf_port}/bitcoin.pdf"
    if not url:
        raise SystemExit("url is required unless --serve-bitcoin-pdf is used")

    state = HarnessState()
    extra: dict[str, Any] = {"url": url, "trace_file": str(trace_file)}
    socket_path = log_dir / "gui.sock"
    try:
        socket_path.unlink()
    except FileNotFoundError:
        pass

    listener = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    listener.bind(str(socket_path))
    listener.listen(1)
    listener.settimeout(args.setup_timeout)

    stdout = (log_dir / "roamium.stdout").open("wb")
    stderr = (log_dir / "roamium.stderr").open("wb")
    env = os.environ.copy()
    env["TERMSURF_PDF_INPUT_TRACE"] = "1"
    env["TERMSURF_PDF_INPUT_TRACE_FILE"] = str(trace_file)
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
        if not state.devtools_port:
            extra["error"] = "missing DevTools port"
            classify(state, trace_file)
            write_summary(log_dir, state, extra)
            return 1

        state.before_capture_status, before_error = run_capture(
            log_dir,
            state.devtools_port,
            args.url_contains,
            "before",
            args.capture_timeout_seconds,
            args.settle_seconds,
        )
        extra["before_capture_error"] = before_error
        if state.before_capture_status != "ok":
            classify(state, trace_file)
            write_summary(log_dir, state, extra)
            return 1

        before_summary = load_probe_summary(log_dir / "before")
        source, coords = choose_scroll_point(
            before_summary,
            args.width,
            args.height,
        )
        state.coordinate_source = source
        state.coordinates = coords
        try:
            send_scroll_events(
                conn,
                state,
                args.scroll_count,
                coords["x"],
                coords["y"],
                args.scroll_delta_y,
            )
        except (BrokenPipeError, ConnectionResetError) as err:
            extra["scroll_send_error"] = repr(err)
            classify(state, trace_file)
            write_summary(log_dir, state, extra)
            return 1
        time.sleep(args.post_scroll_settle_seconds)

        state.after_capture_status, after_error = run_capture(
            log_dir,
            state.devtools_port,
            args.url_contains,
            "after",
            args.capture_timeout_seconds,
            args.settle_seconds,
        )
        extra["after_capture_error"] = after_error
        if state.after_capture_status == "ok":
            after_summary = load_probe_summary(log_dir / "after")
            before_sig = significant_state(before_summary)
            after_sig = significant_state(after_summary)
            state.before_after_state_changed = before_sig != after_sig
            before_hash = sha256_file(log_dir / "before" / "baseline.png")
            after_hash = sha256_file(log_dir / "after" / "baseline.png")
            state.before_after_screenshot_changed = bool(
                before_hash and after_hash and before_hash != after_hash
            )
            extra["before_screenshot_sha256"] = before_hash
            extra["after_screenshot_sha256"] = after_hash
        classify(state, trace_file)
        write_summary(log_dir, state, extra)
        return 0
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
