#!/usr/bin/env python3
"""Drive PDF mouse input through the TermSurf protocol."""

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
LEFT_BUTTON_DOWN = 64
META_MODIFIER = 8
VKEY_A = 65
VKEY_C = 67


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


def mouse_move_payload(tab_id: int, x: float, y: float, modifiers: int) -> bytes:
    return (
        varint_field(1, tab_id)
        + double_field(2, x)
        + double_field(3, y)
        + varint_field(4, modifiers)
    )


def key_event_payload(
    tab_id: int,
    event_type: str,
    windows_key_code: int,
    utf8: str,
    modifiers: int,
) -> bytes:
    return (
        varint_field(1, tab_id)
        + string_field(2, event_type)
        + varint_field(3, windows_key_code)
        + string_field(4, utf8)
        + varint_field(5, modifiers)
    )


def focus_changed_payload(tab_id: int, focused: bool) -> bytes:
    return varint_field(1, tab_id) + bool_field(2, focused)


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
    devtools_port: int | None = None
    action: str = ""
    messages_sent: list[dict[str, Any]] = dataclass_field(default_factory=list)
    key_messages_sent: list[dict[str, Any]] = dataclass_field(default_factory=list)
    coordinate_source: str = "not-selected"
    points: dict[str, float] | None = None
    before_capture_status: str = "not-run"
    after_capture_status: str = "not-run"
    before_after_state_changed: bool = False
    before_after_screenshot_changed: bool = False
    roamium_trace_init: bool = False
    roamium_mouse_event_line: bool = False
    roamium_mouse_move_line: bool = False
    roamium_focus_line: bool = False
    roamium_focus_ffi_line: bool = False
    roamium_key_event_line: bool = False
    roamium_key_ffi_line: bool = False
    roamium_ffi_line: bool = False
    chromium_focus_line: bool = False
    chromium_key_route_line: bool = False
    chromium_key_focused_widget_line: bool = False
    chromium_key_root_direct_line: bool = False
    chromium_key_target_classification: str = "unknown"
    chromium_route_line: bool = False
    chromium_input_router_line: bool = False
    chromium_direct_fallback_line: bool = False
    pdf_plugin_input_line: bool = False
    pdfium_mousedown_line: bool = False
    pdfium_mousedown_text_area_line: bool = False
    pdfium_mousedown_selecting_true_line: bool = False
    pdfium_mousemove_line: bool = False
    pdfium_mousemove_extend_line: bool = False
    pdfium_extend_reached_line: bool = False
    pdfium_extend_return_true_line: bool = False
    pdfium_selection_nonempty_line: bool = False
    drag_sweep_attempts: list[dict[str, Any]] = dataclass_field(default_factory=list)
    drag_sweep_selected: bool = False
    selected_text_length: int = 0
    clipboard_text_length: int | None = None
    clipboard_before_text_length: int | None = None
    clipboard_before_sha256: str | None = None
    clipboard_after_sha256: str | None = None
    clipboard_after_sample: str | None = None
    clipboard_error: str | None = None
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
    while time.time() < deadline:
        text = read_text(log_dir / "roamium.stdout") + "\n" + read_text(
            log_dir / "roamium.stderr"
        )
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


def run_devtools_expression(
    devtools_port: int,
    url_contains: str,
    expression: str,
    timeout_seconds: int,
) -> dict[str, Any]:
    node_script = r"""
const port = Number(process.argv[1]);
const urlContains = process.argv[2];
const expression = process.argv[3];
const timeoutMs = Number(process.argv[4]) * 1000;

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function findTarget() {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    const response = await fetch(`http://127.0.0.1:${port}/json/list`);
    const targets = await response.json();
    const target = targets.find((item) =>
      item.type === "page" &&
      typeof item.url === "string" &&
      item.url.includes(urlContains) &&
      item.webSocketDebuggerUrl
    );
    if (target) {
      return target;
    }
    await sleep(100);
  }
  throw new Error(`no target contained ${JSON.stringify(urlContains)}`);
}

function connect(wsUrl) {
  const socket = new WebSocket(wsUrl);
  let nextId = 1;
  const pending = new Map();
  socket.addEventListener("message", (event) => {
    const message = JSON.parse(event.data);
    if (!message.id || !pending.has(message.id)) {
      return;
    }
    const {resolve, reject} = pending.get(message.id);
    pending.delete(message.id);
    if (message.error) {
      reject(new Error(message.error.message || "DevTools error"));
    } else {
      resolve(message.result || {});
    }
  });
  const open = new Promise((resolve, reject) => {
    socket.addEventListener("open", resolve, {once: true});
    socket.addEventListener("error", reject, {once: true});
  });
  function send(method, params = {}) {
    const id = nextId++;
    const promise = new Promise((resolve, reject) => {
      pending.set(id, {resolve, reject});
    });
    socket.send(JSON.stringify({id, method, params}));
    return promise;
  }
  return {socket, open, send};
}

(async () => {
  const target = await findTarget();
  const client = connect(target.webSocketDebuggerUrl);
  await client.open;
  await client.send("Browser.grantPermissions", {
    permissions: ["clipboardReadWrite", "clipboardSanitizedWrite"],
  }).catch((error) => ({error: String(error.message || error)}));
  const result = await client.send("Runtime.evaluate", {
    expression,
    awaitPromise: true,
    returnByValue: true,
  });
  client.socket.close();
  console.log(JSON.stringify({ok: true, result}));
})().catch((error) => {
  console.log(JSON.stringify({ok: false, error: String(error.stack || error)}));
  process.exitCode = 1;
});
"""
    proc = subprocess.run(
        ["node", "-e", node_script, str(devtools_port), url_contains, expression, str(timeout_seconds)],
        cwd=str(ROOT),
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    try:
        data = json.loads(proc.stdout.strip() or "{}")
    except json.JSONDecodeError:
        data = {"ok": False, "error": f"invalid JSON: {proc.stdout!r}"}
    if proc.returncode != 0 and data.get("ok") is not False:
        data["ok"] = False
        data["error"] = proc.stderr.strip() or f"node exited {proc.returncode}"
    if proc.stderr.strip():
        data["stderr"] = proc.stderr.strip()
    return data


def devtools_value(result: dict[str, Any]) -> Any:
    if not result.get("ok"):
        return None
    return (
        result.get("result", {})
        .get("result", {})
        .get("value")
    )


def text_sha256(text: str) -> str:
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


def read_clipboard(devtools_port: int, url_contains: str, timeout_seconds: int) -> dict[str, Any]:
    return run_devtools_expression(
        devtools_port,
        url_contains,
        "navigator.clipboard.readText().then((text) => ({ text, length: text.length }))",
        timeout_seconds,
    )


def clipboard_text(result: dict[str, Any]) -> str:
    value = devtools_value(result)
    if isinstance(value, dict):
        return str(value.get("text") or "")
    return ""


def clear_clipboard(devtools_port: int, url_contains: str, timeout_seconds: int) -> dict[str, Any]:
    return run_devtools_expression(
        devtools_port,
        url_contains,
        "navigator.clipboard.writeText('').then(() => true)",
        timeout_seconds,
    )


def load_json(path: pathlib.Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def load_probe_summary(out_dir: pathlib.Path) -> dict[str, Any]:
    summary = load_json(out_dir / "summary.json")
    try:
        baseline = load_json(out_dir / "baseline.json")
        if isinstance(baseline.get("state"), dict):
            summary["_baselineState"] = baseline["state"]
    except FileNotFoundError:
        pass
    return summary


def all_probe_states(summary: dict[str, Any]) -> list[dict[str, Any]]:
    states: list[dict[str, Any]] = []
    if isinstance(summary.get("_baselineState"), dict):
        states.append(summary["_baselineState"])
    for attempt in summary.get("attachAttempts", []):
        state = attempt.get("state")
        if isinstance(state, dict):
            states.append(state)
    return states


def all_elements(summary: dict[str, Any]) -> list[dict[str, Any]]:
    elements: list[dict[str, Any]] = []
    for state in all_probe_states(summary):
        value = state.get("value") or {}
        elements.extend(value.get("elements") or [])
        for child in value.get("childStates") or []:
            child_value = child.get("state", {}).get("value") or {}
            elements.extend(child_value.get("elements") or [])
    return elements


def element_bounds(element: dict[str, Any]) -> dict[str, float] | None:
    try:
        x = float(element["x"])
        y = float(element["y"])
        width = float(element["width"])
        height = float(element["height"])
    except (KeyError, TypeError, ValueError):
        return None
    if width <= 0 or height <= 0:
        return None
    return {"x": x, "y": y, "width": width, "height": height}


def find_element(summary: dict[str, Any], element_id: str) -> dict[str, float] | None:
    for element in all_elements(summary):
        if element.get("id") == element_id:
            return element_bounds(element)
    return None


def choose_pdf_bounds(summary: dict[str, Any]) -> tuple[str, dict[str, float]]:
    preferences = [
        ("plugin-bounds", lambda tag, eid: tag == "EMBED" and eid == "plugin"),
        ("container-bounds", lambda _tag, eid: eid == "container"),
        ("viewer-bounds", lambda tag, eid: tag == "PDF-VIEWER" or eid == "viewer"),
    ]
    candidates = []
    for element in all_elements(summary):
        tag = str(element.get("tagName") or element.get("tag") or "").upper()
        element_id = str(element.get("id") or "").lower()
        bounds = element_bounds(element)
        if bounds:
            candidates.append((tag, element_id, bounds))
    for source, predicate in preferences:
        for tag, element_id, bounds in candidates:
            if predicate(tag, element_id):
                return source, bounds
    return "fixed-fallback", {"x": 0.0, "y": 0.0, "width": 600.0, "height": 450.0}


def center(bounds: dict[str, float]) -> tuple[float, float]:
    return bounds["x"] + bounds["width"] / 2, bounds["y"] + bounds["height"] / 2


def drag_points(bounds: dict[str, float]) -> dict[str, float]:
    x1 = max(5.0, bounds["x"] + min(80.0, bounds["width"] * 0.12))
    x2 = max(x1 + 20.0, bounds["x"] + bounds["width"] * 0.85)
    y1 = max(5.0, bounds["y"] + min(180.0, bounds["height"] * 0.22))
    y2 = max(y1 + 20.0, bounds["y"] + min(360.0, bounds["height"] * 0.42))
    return {"x1": x1, "y1": y1, "x2": x2, "y2": y2}


def pdf_drag_sweep_paths(bounds: dict[str, float]) -> list[dict[str, float]]:
    paths = []
    for start_fraction, end_fraction in ((0.32, 0.72), (0.40, 0.68)):
        for y_fraction in (0.16, 0.22, 0.28, 0.34, 0.40):
            y = bounds["y"] + bounds["height"] * y_fraction
            paths.append(
                {
                    "fraction_x1": start_fraction,
                    "fraction_x2": end_fraction,
                    "fraction_y": y_fraction,
                    "x1": bounds["x"] + bounds["width"] * start_fraction,
                    "y1": y,
                    "x2": bounds["x"] + bounds["width"] * end_fraction,
                    "y2": y,
                }
            )
    return paths


def choose_points(
    summary: dict[str, Any],
    action: str,
    url: str,
) -> tuple[str, dict[str, float]]:
    is_pdf = "bitcoin.pdf" in url or any(
        "mhjfbmdgcfjbbpaeojofohoefgiehjai" in str(state.get("value", {}).get("url", ""))
        for state in all_probe_states(summary)
    )
    if not is_pdf and action in ("click", "key-select-copy"):
        bounds = find_element(summary, "click-target")
        if bounds:
            x, y = center(bounds)
            return "html-click-target", {**bounds, "x": x, "y": y}
    if not is_pdf and action == "drag":
        bounds = find_element(summary, "selection-target")
        if bounds:
            return "html-selection-target", drag_points(bounds)
    source, bounds = choose_pdf_bounds(summary)
    if action in ("click", "key-select-copy"):
        x, y = center(bounds)
        return source, {**bounds, "x": x, "y": y}
    if action == "drag":
        return source, pdf_drag_sweep_paths(bounds)[2]
    return source, drag_points(bounds)


def sha256_file(path: pathlib.Path) -> str | None:
    try:
        return hashlib.sha256(path.read_bytes()).hexdigest()
    except FileNotFoundError:
        return None


def selected_text_length(summary: dict[str, Any]) -> int:
    best = 0
    for state in all_probe_states(summary):
        value = state.get("value") or {}
        selection = value.get("selection") or ""
        best = max(best, len(selection))
    return best


def significant_state(summary: dict[str, Any]) -> dict[str, Any]:
    states = []
    for state in all_probe_states(summary):
        value = state.get("value") or {}
        states.append(
            {
                "url": value.get("url"),
                "activeElement": value.get("activeElement"),
                "selection": value.get("selection"),
                "bodyTextSample": value.get("bodyTextSample"),
                "scroll": value.get("scroll"),
                "elements": value.get("elements"),
            }
        )
    return {"states": states}


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


def send_click(conn: socket.socket, state: HarnessState, x: float, y: float) -> None:
    if not state.tab_ready_id:
        return
    for index, event_type in enumerate(["down", "up"]):
        send_message(
            conn,
            6,
            mouse_event_payload(state.tab_ready_id, event_type, "left", x, y, 1, 0),
        )
        state.messages_sent.append(
            {
                "index": index,
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


def send_focus(conn: socket.socket, state: HarnessState) -> None:
    if not state.tab_ready_id:
        return
    send_message(conn, 10, focus_changed_payload(state.tab_ready_id, True))
    state.focus_sent = True
    time.sleep(0.05)


def send_key(
    conn: socket.socket,
    state: HarnessState,
    event_type: str,
    windows_key_code: int,
    utf8: str,
    modifiers: int,
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


def send_command_shortcut(conn: socket.socket, state: HarnessState, keycode: int) -> None:
    send_key(conn, state, "down", keycode, "", META_MODIFIER)
    send_key(conn, state, "up", keycode, "", META_MODIFIER)


def send_drag(
    conn: socket.socket,
    state: HarnessState,
    x1: float,
    y1: float,
    x2: float,
    y2: float,
) -> None:
    if not state.tab_ready_id:
        return
    start_index = len(state.messages_sent)
    send_message(
        conn,
        6,
        mouse_event_payload(state.tab_ready_id, "down", "left", x1, y1, 1, 0),
    )
    state.messages_sent.append(
        {
            "index": start_index,
            "message": "mouse_event",
            "type": "down",
            "button": "left",
            "x": x1,
            "y": y1,
            "click_count": 1,
            "modifiers": 0,
        }
    )
    steps = 8
    for step in range(1, steps + 1):
        t = step / steps
        x = x1 + (x2 - x1) * t
        y = y1 + (y2 - y1) * t
        send_message(conn, 7, mouse_move_payload(state.tab_ready_id, x, y, LEFT_BUTTON_DOWN))
        state.messages_sent.append(
            {
                "index": start_index + step,
                "message": "mouse_move",
                "x": x,
                "y": y,
                "modifiers": LEFT_BUTTON_DOWN,
            }
        )
        time.sleep(0.02)
    send_message(
        conn,
        6,
        mouse_event_payload(state.tab_ready_id, "up", "left", x2, y2, 1, 0),
    )
    state.messages_sent.append(
        {
            "index": start_index + steps + 1,
            "message": "mouse_event",
            "type": "up",
            "button": "left",
            "x": x2,
            "y": y2,
            "click_count": 1,
            "modifiers": 0,
        }
    )


def classify(state: HarnessState, trace_file: pathlib.Path, stderr_file: pathlib.Path) -> None:
    trace = read_text(trace_file)
    stderr = read_text(stderr_file)
    state.roamium_trace_init = "trace-init" in trace
    state.roamium_focus_line = "focus-changed" in trace
    state.roamium_focus_ffi_line = "ffi=ts_set_focus" in trace
    state.roamium_key_event_line = "key-event" in trace
    state.roamium_key_ffi_line = "ffi=ts_forward_key_event" in trace
    state.roamium_mouse_event_line = "mouse-event" in trace
    state.roamium_mouse_move_line = "mouse-move" in trace
    state.roamium_ffi_line = (
        "ffi=ts_forward_mouse_event" in trace
        or "ffi=ts_forward_mouse_move" in trace
        or state.roamium_key_ffi_line
    )
    state.chromium_focus_line = "[termsurf-pdf-input] focus" in stderr
    state.chromium_key_route_line = "[termsurf-pdf-input] key-route" in stderr
    state.chromium_key_focused_widget_line = "key-route route_mode=focused-widget" in stderr
    state.chromium_key_root_direct_line = "key-route route_mode=root-direct" in stderr
    target_match = re.search(r"\[termsurf-pdf-input\] key-target .*classification=([^ ]+)", stderr)
    if target_match:
        state.chromium_key_target_classification = target_match.group(1)
    state.chromium_route_line = "[termsurf-pdf-input] mouse-route" in stderr
    state.chromium_input_router_line = "route_mode=input-router" in stderr
    state.chromium_direct_fallback_line = "route_mode=direct-fallback" in stderr
    state.pdf_plugin_input_line = "[termsurf-pdf-input] plugin-input" in stderr
    state.pdfium_mousedown_line = "[termsurf-pdf-input] pdfium-mouse event=left-down" in stderr
    state.pdfium_mousedown_text_area_line = bool(
        re.search(
            r"\[termsurf-pdf-input\] pdfium-mouse event=left-down .*area_label=text",
            stderr,
        )
    )
    state.pdfium_mousedown_selecting_true_line = bool(
        re.search(
            r"\[termsurf-pdf-input\] pdfium-mouse event=left-down .*selecting_after=(1|true)",
            stderr,
        )
    )
    state.pdfium_mousemove_line = "[termsurf-pdf-input] pdfium-mouse event=mouse-move" in stderr
    state.pdfium_mousemove_extend_line = bool(
        re.search(
            r"\[termsurf-pdf-input\] pdfium-mouse event=mouse-move .*outcome=extend-selection",
            stderr,
        )
    )
    state.pdfium_extend_reached_line = "extend_reached=1" in stderr or "extend_reached=true" in stderr
    state.pdfium_extend_return_true_line = (
        "extend_return=1" in stderr or "extend_return=true" in stderr
    )
    state.pdfium_selection_nonempty_line = bool(
        re.search(r"\[termsurf-pdf-input\] pdfium-selection-changed length=[1-9]", stderr)
    )

    is_key_action = state.action == "key-select-copy"
    is_drag_sweep = state.action == "pdf-drag-sweep"
    selected_or_copied = state.selected_text_length > 0 or (
        (state.clipboard_text_length or 0) > 0
        and state.clipboard_after_sha256
        and state.clipboard_after_sha256 != state.clipboard_before_sha256
    )

    if not state.server_register_received:
        state.first_failing_hop = "roamium-not-registered"
    elif not state.tab_ready_id:
        state.first_failing_hop = "tab-not-ready"
    elif not state.resize_sent:
        state.first_failing_hop = "resize-not-sent"
    elif (is_key_action or is_drag_sweep) and not state.focus_sent:
        state.first_failing_hop = (
            "protocol-focus-not-sent" if is_drag_sweep else "focus-not-sent"
        )
    elif not state.messages_sent:
        state.first_failing_hop = (
            "protocol-drag-not-sent" if is_drag_sweep else "protocol-mouse-not-sent"
        )
    elif is_key_action and not state.key_messages_sent:
        state.first_failing_hop = "protocol-key-not-sent"
    elif not state.roamium_trace_init and not (
        state.roamium_mouse_event_line
        or state.roamium_mouse_move_line
        or state.roamium_key_event_line
    ):
        state.first_failing_hop = "trace-env-not-inherited"
    elif is_key_action and not state.roamium_focus_line:
        state.first_failing_hop = "roamium-focus-receive-missing"
    elif is_key_action and not state.roamium_focus_ffi_line:
        state.first_failing_hop = "roamium-focus-ffi-missing"
    elif is_key_action and not state.chromium_focus_line:
        state.first_failing_hop = "chromium-focus-missing"
    elif not state.roamium_mouse_event_line:
        state.first_failing_hop = "roamium-receive-missing"
    elif not state.roamium_ffi_line:
        state.first_failing_hop = "roamium-ffi-missing"
    elif is_key_action and not state.roamium_key_event_line:
        state.first_failing_hop = "roamium-key-receive-missing"
    elif is_key_action and not state.roamium_key_ffi_line:
        state.first_failing_hop = "roamium-key-ffi-missing"
    elif is_key_action and not state.chromium_key_route_line:
        state.first_failing_hop = "chromium-key-route-missing"
    elif (
        is_key_action
        and state.chromium_key_target_classification == "unknown"
        and not state.chromium_key_root_direct_line
    ):
        state.first_failing_hop = "chromium-key-target-ambiguous"
    elif not state.chromium_route_line:
        state.first_failing_hop = "chromium-route-missing"
    elif state.chromium_direct_fallback_line and not state.chromium_input_router_line:
        state.first_failing_hop = "chromium-route-direct-fallback"
    elif is_drag_sweep and not state.pdf_plugin_input_line:
        state.first_failing_hop = "pdf-plugin-input-missing"
    elif is_drag_sweep and not state.pdfium_mousedown_line:
        state.first_failing_hop = "pdfium-mousedown-missing"
    elif is_drag_sweep and not state.pdfium_mousedown_text_area_line:
        state.first_failing_hop = "pdfium-not-text-area"
    elif is_drag_sweep and not state.pdfium_mousemove_line:
        state.first_failing_hop = "pdfium-move-missing"
    elif is_drag_sweep and not state.pdfium_mousedown_selecting_true_line:
        state.first_failing_hop = "pdfium-selection-not-changing"
    elif is_drag_sweep and not (
        state.pdfium_mousemove_extend_line or state.pdfium_extend_reached_line
    ):
        state.first_failing_hop = "pdfium-selection-not-changing"
    elif is_drag_sweep and not (
        state.drag_sweep_selected
        or state.pdfium_selection_nonempty_line
        or selected_or_copied
    ):
        state.first_failing_hop = "pdfium-selection-not-changing"
    elif is_drag_sweep and state.pdfium_selection_nonempty_line and not selected_or_copied:
        state.first_failing_hop = "selection-not-observable"
    elif (
        is_key_action
        and state.chromium_key_root_direct_line
        and state.coordinate_source != "html-click-target"
    ):
        state.first_failing_hop = "chromium-key-root-target"
    elif is_key_action and not selected_or_copied:
        if state.clipboard_error:
            state.first_failing_hop = "clipboard-unavailable"
        else:
            state.first_failing_hop = "pdf-focus-or-selection"
    elif (
        state.action == "drag"
        and state.selected_text_length == 0
        and not state.pdfium_selection_nonempty_line
    ):
        state.first_failing_hop = "pdf-focus-or-selection"
    elif not (state.before_after_state_changed or state.before_after_screenshot_changed):
        state.first_failing_hop = "pdf-focus-or-selection"
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
        "action": state.action,
        "target_coordinate_source": state.coordinate_source,
        "target_points": state.points,
        "protocol_mouse_messages_sent": len(state.messages_sent),
        "protocol_mouse_messages": state.messages_sent,
        "protocol_key_messages_sent": len(state.key_messages_sent),
        "protocol_key_messages": state.key_messages_sent,
        "roamium_trace_init": state.roamium_trace_init,
        "roamium_focus_line": state.roamium_focus_line,
        "roamium_focus_ffi_line": state.roamium_focus_ffi_line,
        "roamium_mouse_event_line": state.roamium_mouse_event_line,
        "roamium_mouse_move_line": state.roamium_mouse_move_line,
        "roamium_key_event_line": state.roamium_key_event_line,
        "roamium_key_ffi_line": state.roamium_key_ffi_line,
        "roamium_ffi_line": state.roamium_ffi_line,
        "chromium_focus_line": state.chromium_focus_line,
        "chromium_key_route_line": state.chromium_key_route_line,
        "chromium_key_focused_widget_line": state.chromium_key_focused_widget_line,
        "chromium_key_root_direct_line": state.chromium_key_root_direct_line,
        "chromium_key_target_classification": state.chromium_key_target_classification,
        "chromium_route_line": state.chromium_route_line,
        "chromium_input_router_line": state.chromium_input_router_line,
        "chromium_direct_fallback_line": state.chromium_direct_fallback_line,
        "pdf_plugin_input_line": state.pdf_plugin_input_line,
        "pdfium_mousedown_line": state.pdfium_mousedown_line,
        "pdfium_mousedown_text_area_line": state.pdfium_mousedown_text_area_line,
        "pdfium_mousedown_selecting_true_line": state.pdfium_mousedown_selecting_true_line,
        "pdfium_mousemove_line": state.pdfium_mousemove_line,
        "pdfium_mousemove_extend_line": state.pdfium_mousemove_extend_line,
        "pdfium_extend_reached_line": state.pdfium_extend_reached_line,
        "pdfium_extend_return_true_line": state.pdfium_extend_return_true_line,
        "pdfium_selection_nonempty_line": state.pdfium_selection_nonempty_line,
        "drag_sweep_attempts": state.drag_sweep_attempts,
        "drag_sweep_selected": state.drag_sweep_selected,
        "before_after_state_changed": state.before_after_state_changed,
        "before_after_screenshot_changed": state.before_after_screenshot_changed,
        "selected_text_length": state.selected_text_length,
        "clipboard_text_length": state.clipboard_text_length,
        "clipboard_before_text_length": state.clipboard_before_text_length,
        "clipboard_before_sha256": state.clipboard_before_sha256,
        "clipboard_after_sha256": state.clipboard_after_sha256,
        "clipboard_after_sample": state.clipboard_after_sample,
        "clipboard_error": state.clipboard_error,
        "first_failing_hop": state.first_failing_hop,
    }
    data.update(extra)
    (log_dir / "protocol-mouse-summary.json").write_text(
        json.dumps(data, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("url", nargs="?")
    parser.add_argument("--log-dir", required=True)
    parser.add_argument(
        "--action",
        choices=["click", "drag", "key-select-copy", "pdf-drag-sweep"],
        required=True,
    )
    parser.add_argument("--width", type=int, default=1200)
    parser.add_argument("--height", type=int, default=900)
    parser.add_argument("--serve-bitcoin-pdf", action="store_true")
    parser.add_argument("--pdf-port", type=int, default=9787)
    parser.add_argument("--url-contains", default="bitcoin.pdf")
    parser.add_argument("--setup-timeout", type=float, default=30)
    parser.add_argument("--capture-timeout-seconds", type=int, default=30)
    parser.add_argument("--settle-seconds", type=int, default=8)
    parser.add_argument("--post-input-settle-seconds", type=float, default=1.0)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    log_dir = pathlib.Path(args.log_dir).resolve()
    log_dir.mkdir(parents=True, exist_ok=True)
    trace_file = pathlib.Path(
        os.environ.get("TERMSURF_PDF_INPUT_TRACE_FILE", str(log_dir / "pdf-input.log"))
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

    state = HarnessState(action=args.action)
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
            classify(state, trace_file, log_dir / "roamium.stderr")
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
            classify(state, trace_file, log_dir / "roamium.stderr")
            write_summary(log_dir, state, extra)
            return 1

        before_summary = load_probe_summary(log_dir / "before")
        if args.action == "pdf-drag-sweep":
            source, bounds = choose_pdf_bounds(before_summary)
            state.coordinate_source = source
            state.points = bounds
            send_focus(conn, state)
            for attempt_index, points in enumerate(pdf_drag_sweep_paths(bounds), start=1):
                clear_result = clear_clipboard(
                    state.devtools_port,
                    args.url_contains,
                    args.capture_timeout_seconds,
                )
                before_clipboard = read_clipboard(
                    state.devtools_port,
                    args.url_contains,
                    args.capture_timeout_seconds,
                )
                before_text = clipboard_text(before_clipboard)
                before_hash = text_sha256(before_text)

                send_drag(
                    conn,
                    state,
                    points["x1"],
                    points["y1"],
                    points["x2"],
                    points["y2"],
                )
                time.sleep(0.25)
                send_command_shortcut(conn, state, VKEY_C)
                time.sleep(0.25)

                after_clipboard = read_clipboard(
                    state.devtools_port,
                    args.url_contains,
                    args.capture_timeout_seconds,
                )
                after_text = clipboard_text(after_clipboard)
                after_hash = text_sha256(after_text)
                changed = after_hash != before_hash
                selected = changed and len(after_text) > 0
                attempt = {
                    "attempt": attempt_index,
                    "points": points,
                    "clipboard_clear": clear_result,
                    "clipboard_before_ok": before_clipboard.get("ok"),
                    "clipboard_before_text_length": len(before_text),
                    "clipboard_before_sha256": before_hash,
                    "clipboard_after_ok": after_clipboard.get("ok"),
                    "clipboard_after_text_length": len(after_text),
                    "clipboard_after_sha256": after_hash,
                    "clipboard_hash_changed": changed,
                    "clipboard_sample": after_text[:160],
                    "selected": selected,
                }
                state.drag_sweep_attempts.append(attempt)
                if not after_clipboard.get("ok"):
                    state.clipboard_error = str(
                        after_clipboard.get("error") or "read failed"
                    )
                state.clipboard_before_text_length = len(before_text)
                state.clipboard_before_sha256 = before_hash
                state.clipboard_text_length = len(after_text)
                state.clipboard_after_sha256 = after_hash
                state.clipboard_after_sample = after_text[:160]
                if selected:
                    state.drag_sweep_selected = True
                    break
        else:
            source, points = choose_points(before_summary, args.action, url)
            state.coordinate_source = source
            state.points = points
        if args.action == "click":
            send_click(conn, state, points["x"], points["y"])
        elif args.action == "drag":
            send_drag(
                conn,
                state,
                points["x1"],
                points["y1"],
                points["x2"],
                points["y2"],
            )
        elif args.action == "key-select-copy":
            send_focus(conn, state)
            clear_result = clear_clipboard(
                state.devtools_port,
                args.url_contains,
                args.capture_timeout_seconds,
            )
            extra["clipboard_clear"] = clear_result
            before_clipboard = read_clipboard(
                state.devtools_port,
                args.url_contains,
                args.capture_timeout_seconds,
            )
            extra["clipboard_before"] = before_clipboard
            before_text = clipboard_text(before_clipboard)
            state.clipboard_before_text_length = len(before_text)
            state.clipboard_before_sha256 = text_sha256(before_text)

            send_click(conn, state, points["x"], points["y"])
            time.sleep(0.15)
            send_command_shortcut(conn, state, VKEY_A)
            time.sleep(0.25)
            send_command_shortcut(conn, state, VKEY_C)
            time.sleep(0.25)

            after_clipboard = read_clipboard(
                state.devtools_port,
                args.url_contains,
                args.capture_timeout_seconds,
            )
            extra["clipboard_after"] = after_clipboard
            after_text = clipboard_text(after_clipboard)
            if not after_clipboard.get("ok"):
                state.clipboard_error = str(after_clipboard.get("error") or "read failed")
            state.clipboard_text_length = len(after_text)
            state.clipboard_after_sha256 = text_sha256(after_text)
            state.clipboard_after_sample = after_text[:160]
        time.sleep(args.post_input_settle_seconds)

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
            state.before_after_state_changed = significant_state(
                before_summary
            ) != significant_state(after_summary)
            before_hash = sha256_file(log_dir / "before" / "baseline.png")
            after_hash = sha256_file(log_dir / "after" / "baseline.png")
            state.before_after_screenshot_changed = bool(
                before_hash and after_hash and before_hash != after_hash
            )
            state.selected_text_length = selected_text_length(after_summary)
            extra["before_screenshot_sha256"] = before_hash
            extra["after_screenshot_sha256"] = after_hash
        classify(state, trace_file, log_dir / "roamium.stderr")
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
