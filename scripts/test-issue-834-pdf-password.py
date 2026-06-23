#!/usr/bin/env python3
"""Probe Roamium password-protected PDFs through TermSurf protocol input."""

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
PASSWORD_PROBE = ROOT / "scripts/probe-pdf-password.mjs"
DEVTOOLS_RE = re.compile(r"DevTools listening on ws://127\.0\.0\.1:(\d+)/")

TEST_PASSWORD = "issue834pdf"
WRONG_PASSWORD = "issue834wrong"
OWNER_PASSWORD = "owner-issue834-exp7"
FIXTURE_PASSWORDS = (TEST_PASSWORD, WRONG_PASSWORD, OWNER_PASSWORD)
META_MODIFIER = 8
VKEY_BACKSPACE = 8
VKEY_A = 65
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


class PasswordPdfHandler(http.server.BaseHTTPRequestHandler):
    log_dir: pathlib.Path
    unrestricted_pdf: pathlib.Path
    protected_pdf: pathlib.Path

    def log_message(self, fmt: str, *args: object) -> None:
        with (self.log_dir / "http.log").open("a", encoding="utf-8") as log:
            log.write((fmt % args) + "\n")

    def do_GET(self) -> None:
        request_path = self.path.split("?", 1)[0]
        if request_path == "/unrestricted.pdf":
            data = self.unrestricted_pdf.read_bytes()
        elif request_path == "/password.pdf":
            data = self.protected_pdf.read_bytes()
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
    focus_sent: bool = False
    devtools_port: int | None = None
    mouse_messages_sent: list[dict[str, Any]] = dataclass_field(default_factory=list)
    key_messages_sent: list[dict[str, Any]] = dataclass_field(default_factory=list)
    typed_secret_lengths: list[int] = dataclass_field(default_factory=list)
    roamium_trace_init: bool = False
    roamium_focus_line: bool = False
    roamium_key_event_line: bool = False
    roamium_key_ffi_line: bool = False
    chromium_key_route_line: bool = False
    chromium_key_target_classification: str = "unknown"
    before_probe_status: str = "not-run"
    wrong_probe_status: str = "not-run"
    after_probe_status: str = "not-run"
    first_failing_hop: str = "automation-gap"


def read_text(path: pathlib.Path) -> str:
    try:
        return path.read_text(encoding="utf-8", errors="replace")
    except FileNotFoundError:
        return ""


def redacted_cmd(cmd: list[str]) -> list[str]:
    redacted = []
    for item in cmd:
        value = item
        for password in FIXTURE_PASSWORDS:
            value = value.replace(password, "<redacted-test-password>")
        redacted.append(value)
    return redacted


def run_checked(cmd: list[str], cwd: pathlib.Path, redact: bool = False) -> dict[str, Any]:
    proc = subprocess.run(
        cmd,
        cwd=str(cwd),
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    return {
        "cmd": redacted_cmd(cmd) if redact else cmd,
        "returncode": proc.returncode,
        "stdout": redact_text(proc.stdout),
        "stderr": redact_text(proc.stderr),
    }


def redact_text(text: str) -> str:
    value = text
    for password in FIXTURE_PASSWORDS:
        value = value.replace(password, "<redacted-test-password>")
    return value


def prepare_fixtures(log_dir: pathlib.Path) -> dict[str, Any]:
    fixtures = log_dir / "fixtures"
    fixtures.mkdir(parents=True, exist_ok=True)
    unrestricted = fixtures / "unrestricted.pdf"
    protected = fixtures / "password.pdf"
    unrestricted.write_bytes(BITCOIN_PDF.read_bytes())
    qpdf_cmd = [
        "qpdf",
        "--encrypt",
        TEST_PASSWORD,
        OWNER_PASSWORD,
        "256",
        "--",
        str(unrestricted),
        str(protected),
    ]
    qpdf_result = run_checked(qpdf_cmd, ROOT, redact=True)
    encryption_result = (
        run_checked(["qpdf", "--show-encryption", f"--password={TEST_PASSWORD}", str(protected)], ROOT, redact=True)
        if protected.exists()
        else {"returncode": -1, "stdout": "", "stderr": "protected PDF missing"}
    )
    return {
        "unrestricted": str(unrestricted),
        "protected": str(protected),
        "unrestricted_bytes": unrestricted.stat().st_size,
        "protected_bytes": protected.stat().st_size if protected.exists() else 0,
        "test_password_length": len(TEST_PASSWORD),
        "wrong_password_length": len(WRONG_PASSWORD),
        "qpdf": qpdf_result,
        "encryption": encryption_result,
    }


def start_pdf_server(
    log_dir: pathlib.Path,
    port: int,
    unrestricted: pathlib.Path,
    protected: pathlib.Path,
) -> socketserver.TCPServer:
    PasswordPdfHandler.log_dir = log_dir
    PasswordPdfHandler.unrestricted_pdf = unrestricted
    PasswordPdfHandler.protected_pdf = protected
    server = ReusableTcpServer(("127.0.0.1", port), PasswordPdfHandler)
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
                        send_message(conn, 3, resize_payload(state.tab_ready_id, width, height))
                        state.resize_sent = True
                        messages.write("sent Resize\n")
                        messages.flush()
                        return
            except socket.timeout:
                pass


def send_focus(conn: socket.socket, state: HarnessState) -> None:
    if state.tab_ready_id:
        send_message(conn, 10, focus_payload(state.tab_ready_id, True))
        state.focus_sent = True
        time.sleep(0.2)


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
            "type": event_type,
            "windows_key_code": windows_key_code,
            "utf8_len": len(utf8),
            "modifiers": modifiers,
        }
    )
    time.sleep(0.04)


def send_text(conn: socket.socket, state: HarnessState, text: str) -> None:
    state.typed_secret_lengths.append(len(text))
    for ch in text:
        keycode = ord(ch.upper()) if ch.isalpha() else ord(ch)
        send_key(conn, state, "down", keycode, ch, 0)
        send_key(conn, state, "up", keycode, "", 0)


def send_command_shortcut(conn: socket.socket, state: HarnessState, keycode: int) -> None:
    send_key(conn, state, "down", keycode, "", META_MODIFIER)
    send_key(conn, state, "up", keycode, "", META_MODIFIER)


def send_enter(conn: socket.socket, state: HarnessState) -> None:
    send_key(conn, state, "down", VKEY_ENTER, "", 0)
    send_key(conn, state, "up", VKEY_ENTER, "", 0)


def send_backspaces(conn: socket.socket, state: HarnessState, count: int) -> None:
    for _ in range(count):
        send_key(conn, state, "down", VKEY_BACKSPACE, "", 0)
        send_key(conn, state, "up", VKEY_BACKSPACE, "", 0)


def submit_password_attempt(
    conn: socket.socket,
    state: HarnessState,
    submit: dict[str, float] | None,
    mode: str,
) -> None:
    if mode == "enter":
        send_enter(conn, state)
        return
    if mode == "enter-then-click":
        send_enter(conn, state)
        time.sleep(0.5)
    if submit:
        send_click(conn, state, submit["x"], submit["y"])
    else:
        send_enter(conn, state)


def run_password_probe(
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
            str(PASSWORD_PROBE),
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
        out_dir / "pdf-password-devtools-summary.json",
    )


def load_json(path: pathlib.Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def first_password_dialog(summary: dict[str, Any] | None) -> dict[str, Any] | None:
    dialogs = (summary or {}).get("passwordDialogs") or []
    return dialogs[0] if dialogs else None


def password_input_center(summary: dict[str, Any] | None) -> dict[str, float] | None:
    dialog = first_password_dialog(summary)
    if not dialog:
        return None
    rect = dialog.get("nativeInput") or dialog.get("input") or dialog
    try:
        return {
            "x": float(rect["x"]) + float(rect["width"]) / 2,
            "y": float(rect["y"]) + float(rect["height"]) / 2,
            "width": float(rect["width"]),
            "height": float(rect["height"]),
        }
    except (TypeError, KeyError, ValueError):
        return None


def password_submit_center(summary: dict[str, Any] | None) -> dict[str, float] | None:
    dialog = first_password_dialog(summary)
    if not dialog:
        return None
    rect = dialog.get("submit") or dialog
    try:
        return {
            "x": float(rect["x"]) + float(rect["width"]) / 2,
            "y": float(rect["y"]) + float(rect["height"]) / 2,
            "width": float(rect["width"]),
            "height": float(rect["height"]),
        }
    except (TypeError, KeyError, ValueError):
        return None


def plugin_loaded(summary: dict[str, Any] | None) -> bool:
    for item in (summary or {}).get("pluginStates") or []:
        rect = item.get("pluginRect") or {}
        if (
            item.get("loadState") == "success"
            and item.get("docLength")
            and rect.get("width", 0) > 0
            and rect.get("height", 0) > 0
        ):
            return True
    return False


def prompt_present(summary: dict[str, Any] | None) -> bool:
    return bool(first_password_dialog(summary)) or any(
        item.get("showPasswordDialog") is True
        for item in ((summary or {}).get("pluginStates") or [])
    )


def prompt_invalid(summary: dict[str, Any] | None) -> bool:
    dialog = first_password_dialog(summary)
    return bool(dialog and dialog.get("invalid"))


def trace_flags(log_dir: pathlib.Path, state: HarnessState) -> None:
    trace = read_text(log_dir / "pdf-input.log")
    stderr = read_text(log_dir / "roamium.stderr")
    state.roamium_trace_init = "trace-init" in trace
    state.roamium_focus_line = "focus-event" in trace
    state.roamium_key_event_line = "key-event" in trace
    state.roamium_key_ffi_line = "ffi=ts_forward_key_event" in trace
    state.chromium_key_route_line = "[termsurf-pdf-input] key-route" in stderr
    target_match = re.search(r"\[termsurf-pdf-input\] key-target .*classification=([^ ]+)", stderr)
    if target_match:
        state.chromium_key_target_classification = target_match.group(1)


def raw_password_leaks(log_dir: pathlib.Path, summary: dict[str, Any]) -> list[str]:
    leaks: list[str] = []
    files = [
        log_dir / "messages.log",
        log_dir / "pdf-input.log",
        log_dir / "roamium.stdout",
        log_dir / "roamium.stderr",
    ]
    summary_text = json.dumps(summary, sort_keys=True)
    if any(password in summary_text for password in FIXTURE_PASSWORDS):
        leaks.append("pdf-password-summary.json")
    for path in files:
        text = read_text(path)
        if any(password in text for password in FIXTURE_PASSWORDS):
            leaks.append(str(path.name))
    return leaks


def classify(
    args: argparse.Namespace,
    state: HarnessState,
    fixture_info: dict[str, Any],
    before_summary: dict[str, Any] | None,
    wrong_summary: dict[str, Any] | None,
    after_summary: dict[str, Any] | None,
    leaks: list[str],
) -> None:
    if fixture_info.get("qpdf", {}).get("returncode") != 0:
        state.first_failing_hop = "fixture-generation-failed"
    elif not state.server_register_received:
        state.first_failing_hop = "roamium-not-registered"
    elif not state.tab_ready_id:
        state.first_failing_hop = "tab-not-ready"
    elif not state.resize_sent:
        state.first_failing_hop = "resize-not-sent"
    elif not state.devtools_port:
        state.first_failing_hop = "devtools-missing"
    elif state.before_probe_status != "ok" or not before_summary:
        state.first_failing_hop = "devtools-target-discovery-failed"
    elif args.probe == "unrestricted-control":
        if prompt_present(before_summary):
            state.first_failing_hop = "unexpected-password-prompt"
        elif not plugin_loaded(before_summary):
            state.first_failing_hop = "baseline-pdf-load-failed"
        else:
            state.first_failing_hop = "no-failure-observed"
    elif not prompt_present(before_summary):
        state.first_failing_hop = "password-prompt-not-found"
    elif plugin_loaded(before_summary):
        state.first_failing_hop = "protected-pdf-loaded-before-password"
    elif not state.focus_sent:
        state.first_failing_hop = "focus-not-sent"
    elif not state.mouse_messages_sent:
        state.first_failing_hop = "protocol-password-click-not-sent"
    elif not state.key_messages_sent:
        state.first_failing_hop = "protocol-key-not-sent"
    elif not state.roamium_key_event_line:
        state.first_failing_hop = "roamium-key-receive-missing"
    elif not state.roamium_key_ffi_line:
        state.first_failing_hop = "roamium-key-ffi-missing"
    elif not state.chromium_key_route_line:
        state.first_failing_hop = "chromium-key-route-missing"
    elif args.credential_flow == "correct-only":
        if state.after_probe_status != "ok" or not after_summary:
            state.first_failing_hop = "correct-password-state-missing"
        elif prompt_present(after_summary):
            state.first_failing_hop = "correct-password-not-accepted"
        elif not plugin_loaded(after_summary):
            state.first_failing_hop = "pdf-plugin-not-loaded-after-password"
        elif leaks:
            state.first_failing_hop = "raw-password-leaked"
        else:
            state.first_failing_hop = "no-failure-observed"
    elif args.credential_flow == "wrong-only":
        if state.wrong_probe_status != "ok" or not wrong_summary:
            state.first_failing_hop = "wrong-password-state-missing"
        elif not prompt_present(wrong_summary):
            state.first_failing_hop = "wrong-password-not-rejected"
        elif leaks:
            state.first_failing_hop = "raw-password-leaked"
        else:
            state.first_failing_hop = "no-failure-observed"
    elif state.wrong_probe_status != "ok" or not wrong_summary:
        state.first_failing_hop = "wrong-password-state-missing"
    elif not prompt_present(wrong_summary):
        state.first_failing_hop = "wrong-password-not-rejected"
    elif state.after_probe_status != "ok" or not after_summary:
        state.first_failing_hop = "correct-password-state-missing"
    elif prompt_present(after_summary):
        state.first_failing_hop = "correct-password-not-accepted"
    elif not plugin_loaded(after_summary):
        state.first_failing_hop = "pdf-plugin-not-loaded-after-password"
    elif leaks:
        state.first_failing_hop = "raw-password-leaked"
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
        "credential_flow": args.credential_flow,
        "submit_mode": args.submit_mode,
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
        "typed_secret_lengths": state.typed_secret_lengths,
        "roamium_trace_init": state.roamium_trace_init,
        "roamium_focus_line": state.roamium_focus_line,
        "roamium_key_event_line": state.roamium_key_event_line,
        "roamium_key_ffi_line": state.roamium_key_ffi_line,
        "chromium_key_route_line": state.chromium_key_route_line,
        "chromium_key_target_classification": state.chromium_key_target_classification,
        "before_probe_status": state.before_probe_status,
        "wrong_probe_status": state.wrong_probe_status,
        "after_probe_status": state.after_probe_status,
        "first_failing_hop": state.first_failing_hop,
    }
    data.update(extra)
    (log_dir / "pdf-password-summary.json").write_text(
        json.dumps(data, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--log-dir", required=True)
    parser.add_argument("--probe", choices=["unrestricted-control", "password-protected"], required=True)
    parser.add_argument(
        "--credential-flow",
        choices=["wrong-then-correct", "wrong-only", "correct-only"],
        default="wrong-then-correct",
        help="Credential sequence to run for password-protected probes.",
    )
    parser.add_argument(
        "--submit-mode",
        choices=["click", "enter", "enter-then-click"],
        default="click",
        help="How to submit password dialog attempts in password-protected probes.",
    )
    parser.add_argument("--width", type=int, default=1200)
    parser.add_argument("--height", type=int, default=900)
    parser.add_argument("--pdf-port", type=int, default=9801)
    parser.add_argument("--setup-timeout", type=float, default=30)
    parser.add_argument("--capture-timeout-seconds", type=int, default=30)
    parser.add_argument("--settle-seconds", type=int, default=3)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    log_dir = pathlib.Path(args.log_dir).resolve()
    args.log_dir = str(log_dir)
    log_dir.mkdir(parents=True, exist_ok=True)
    if not ROAMIUM.exists():
        raise SystemExit(f"missing Roamium binary: {ROAMIUM}")
    if args.probe == "unrestricted-control" and args.credential_flow != "wrong-then-correct":
        raise SystemExit("--credential-flow is only valid with --probe password-protected")
    if args.probe == "unrestricted-control" and args.submit_mode != "click":
        raise SystemExit("--submit-mode is only valid with --probe password-protected")
    if not BITCOIN_PDF.exists():
        raise SystemExit(f"missing Bitcoin PDF fixture: {BITCOIN_PDF}")
    if not PASSWORD_PROBE.exists():
        raise SystemExit(f"missing password probe: {PASSWORD_PROBE}")

    fixture_info = prepare_fixtures(log_dir)
    unrestricted_path = pathlib.Path(fixture_info["unrestricted"])
    protected_path = pathlib.Path(fixture_info["protected"])
    pdf_server = start_pdf_server(log_dir, args.pdf_port, unrestricted_path, protected_path)
    request_path = "/unrestricted.pdf" if args.probe == "unrestricted-control" else "/password.pdf"
    url = f"http://127.0.0.1:{args.pdf_port}{request_path}"
    url_contains = request_path.strip("/")

    socket_path = log_dir / "gui.sock"
    try:
        socket_path.unlink()
    except FileNotFoundError:
        pass

    state = HarnessState()
    extra: dict[str, Any] = {
        "url": url,
        "url_contains": url_contains,
        "credential_flow": args.credential_flow,
        "submit_mode": args.submit_mode,
        "fixtures": fixture_info,
        "test_password_length": len(TEST_PASSWORD),
        "wrong_password_length": len(WRONG_PASSWORD),
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
        wrong_summary = None
        after_summary = None
        if state.devtools_port:
            state.before_probe_status, before_error, before_path = run_password_probe(
                log_dir,
                state.devtools_port,
                url_contains,
                "before",
                args.capture_timeout_seconds,
                args.settle_seconds,
            )
            extra["before_probe_error"] = before_error
            before_summary = load_json(before_path) if before_path.exists() else None
            extra["before_summary"] = before_summary

            if args.probe == "password-protected":
                center = password_input_center(before_summary)
                submit = password_submit_center(before_summary)
                extra["password_input_click_point"] = center
                extra["password_submit_click_point"] = submit
                send_focus(conn, state)
                if args.credential_flow in ("wrong-then-correct", "wrong-only"):
                    if center:
                        send_click(conn, state, center["x"], center["y"])
                        time.sleep(0.2)
                    send_text(conn, state, WRONG_PASSWORD)
                    submit_password_attempt(conn, state, submit, args.submit_mode)
                    time.sleep(1.5)
                    state.wrong_probe_status, wrong_error, wrong_path = run_password_probe(
                        log_dir,
                        state.devtools_port,
                        url_contains,
                        "wrong",
                        args.capture_timeout_seconds,
                        1,
                    )
                    extra["wrong_probe_error"] = wrong_error
                    wrong_summary = load_json(wrong_path) if wrong_path.exists() else None
                    extra["wrong_summary"] = wrong_summary

                if args.credential_flow in ("wrong-then-correct", "correct-only"):
                    center = password_input_center(wrong_summary) or center
                    submit = password_submit_center(wrong_summary) or submit
                    if center:
                        send_click(conn, state, center["x"], center["y"])
                        time.sleep(0.2)
                    if args.credential_flow == "wrong-then-correct":
                        send_command_shortcut(conn, state, VKEY_A)
                        send_backspaces(conn, state, 1)
                    send_text(conn, state, TEST_PASSWORD)
                    submit_password_attempt(conn, state, submit, args.submit_mode)
                    time.sleep(3.0)
                    state.after_probe_status, after_error, after_path = run_password_probe(
                        log_dir,
                        state.devtools_port,
                        url_contains,
                        "after",
                        args.capture_timeout_seconds,
                        1,
                    )
                    extra["after_probe_error"] = after_error
                    after_summary = load_json(after_path) if after_path.exists() else None
                    extra["after_summary"] = after_summary

        trace_flags(log_dir, state)
        summary_for_leak_check = {
            "extra": extra,
            "state_without_passwords": {
                "typed_secret_lengths": state.typed_secret_lengths,
            },
        }
        leaks = raw_password_leaks(log_dir, summary_for_leak_check)
        extra["raw_password_leaks"] = leaks
        extra["row_results"] = {
            "unrestricted_control_loaded": plugin_loaded(before_summary)
            if args.probe == "unrestricted-control"
            else None,
            "prompt_before_password": prompt_present(before_summary),
            "wrong_password_rejected": prompt_present(wrong_summary) if wrong_summary else None,
            "wrong_password_invalid_observed": prompt_invalid(wrong_summary) if wrong_summary else None,
            "correct_password_loaded": plugin_loaded(after_summary) if after_summary else None,
            "raw_password_leak_free": not leaks,
        }
        classify(args, state, fixture_info, before_summary, wrong_summary, after_summary, leaks)
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
