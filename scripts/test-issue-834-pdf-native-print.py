#!/usr/bin/env python3
"""Safely probe Roamium native PDF print dialog behavior."""

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

from termsurf_pdf_protocol_harness import (
    bool_field,
    create_tab_payload,
    double_field,
    inner_payload,
    send_message,
    tab_ready_id,
    varint_field,
)


ROOT = pathlib.Path(__file__).resolve().parents[1]
ROAMIUM = ROOT / "chromium/src/out/Default/roamium"
BITCOIN_PDF = ROOT / "test-html/public/bitcoin.pdf"
SAVE_PRINT_TITLE_LOCAL_PROBE = ROOT / "scripts/probe-pdf-save-print-title-local.mjs"
DEVTOOLS_RE = re.compile(r"DevTools listening on ws://127\.0\.0\.1:(\d+)/")
PREFLIGHT_TITLE = "TermSurf Native Print Safety Preflight"


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


class ReusableTcpServer(socketserver.TCPServer):
    allow_reuse_address = True


class PdfHandler(http.server.BaseHTTPRequestHandler):
    log_dir: pathlib.Path

    def log_message(self, fmt: str, *args: object) -> None:
        with (self.log_dir / "http.log").open("a", encoding="utf-8") as log:
            log.write((fmt % args) + "\n")

    def do_GET(self) -> None:
        request_path = self.path.split("?", 1)[0]
        if request_path == "/bitcoin.pdf":
            data = BITCOIN_PDF.read_bytes()
            self.send_response(200)
            self.send_header("Content-Type", "application/pdf")
            self.send_header("Content-Length", str(len(data)))
            self.end_headers()
            self.wfile.write(data)
            return
        if request_path == "/embedded-pdf.html":
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
        self.send_response(404)
        self.end_headers()


@dataclass
class HarnessState:
    server_register_received: bool = False
    create_tab_sent: bool = False
    tab_ready_id: int | None = None
    resize_sent: bool = False
    focus_sent: bool = False
    devtools_port: int | None = None
    probe_status: str = "not-run"
    roamium_exited_before_shutdown: bool = False
    roamium_exit_code_before_shutdown: int | None = None
    first_failing_hop: str = "automation-gap"


def read_text(path: pathlib.Path) -> str:
    try:
        return path.read_text(encoding="utf-8", errors="replace")
    except FileNotFoundError:
        return ""


def run_cmd(cmd: list[str], timeout: float = 10) -> dict[str, Any]:
    try:
        proc = subprocess.run(
            cmd,
            cwd=str(ROOT),
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            timeout=timeout,
            check=False,
        )
        return {
            "cmd": cmd,
            "returncode": proc.returncode,
            "stdout": proc.stdout,
            "stderr": proc.stderr,
            "timed_out": False,
        }
    except subprocess.TimeoutExpired as exc:
        return {
            "cmd": cmd,
            "returncode": None,
            "stdout": exc.stdout or "",
            "stderr": exc.stderr or "",
            "timed_out": True,
        }


def print_queue_state() -> dict[str, Any]:
    return {
        "lpstat_o": run_cmd(["lpstat", "-o"], timeout=5),
        "lpstat_W_completed_o": run_cmd(["lpstat", "-W", "completed", "-o"], timeout=5),
    }


def start_pdf_server(log_dir: pathlib.Path, port: int) -> socketserver.TCPServer:
    PdfHandler.log_dir = log_dir
    server = ReusableTcpServer(("127.0.0.1", port), PdfHandler)
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


def watcher_observe_title(title: str, timeout: float) -> dict[str, Any]:
    script = f'''
set deadline to (current date) + {timeout}
repeat while (current date) < deadline
  tell application "System Events"
    repeat with proc in application processes
      repeat with win in windows of proc
        try
          if name of win contains "{title}" then
            return "observed process=" & name of proc & " window=" & name of win
          end if
        end try
      end repeat
    end repeat
  end tell
  delay 0.2
end repeat
return "not-observed"
'''
    result = run_cmd(["osascript", "-e", script], timeout=timeout + 3)
    return {
        "mechanism": "osascript-system-events-window-title",
        "title": title,
        "result": result,
        "observed": result["returncode"] == 0
        and "observed process=" in result["stdout"],
    }


def watcher_cancel_title(title: str, timeout: float) -> dict[str, Any]:
    script = f'''
set deadline to (current date) + {timeout}
repeat while (current date) < deadline
  tell application "System Events"
    repeat with proc in application processes
      repeat with win in windows of proc
        try
          if name of win contains "{title}" then
            key code 53
            return "cancel-sent process=" & name of proc & " window=" & name of win
          end if
        end try
      end repeat
    end repeat
  end tell
  delay 0.2
end repeat
return "not-cancelled"
'''
    result = run_cmd(["osascript", "-e", script], timeout=timeout + 3)
    return {
        "mechanism": "osascript-system-events-escape",
        "title": title,
        "result": result,
        "cancel_sent": result["returncode"] == 0 and "cancel-sent" in result["stdout"],
    }


def preflight_dialog(timeout: float) -> dict[str, Any]:
    dialog = subprocess.Popen(
        [
            "osascript",
            "-e",
            (
                f'display dialog "Cancel-only preflight for TermSurf native print automation." '
                f'with title "{PREFLIGHT_TITLE}" buttons {{"Cancel"}} default button "Cancel" '
                f'cancel button "Cancel" giving up after {int(timeout)}'
            ),
        ],
        cwd=str(ROOT),
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    time.sleep(0.5)
    observed = watcher_observe_title(PREFLIGHT_TITLE, timeout)
    cancel = watcher_cancel_title(PREFLIGHT_TITLE, timeout) if observed["observed"] else {
        "cancel_sent": False,
        "skipped": "dialog-not-observed",
    }
    try:
        stdout, stderr = dialog.communicate(timeout=timeout + 3)
        dialog_result = {
            "returncode": dialog.returncode,
            "stdout": stdout,
            "stderr": stderr,
            "timed_out": False,
        }
    except subprocess.TimeoutExpired:
        dialog.kill()
        stdout, stderr = dialog.communicate()
        dialog_result = {
            "returncode": dialog.returncode,
            "stdout": stdout,
            "stderr": stderr,
            "timed_out": True,
        }
    disappearance = watcher_observe_title(PREFLIGHT_TITLE, 1)
    return {
        "mechanism": "osascript-display-dialog-plus-system-events",
        "dialog_title": PREFLIGHT_TITLE,
        "observed": observed,
        "cancel": cancel,
        "dialog_result": dialog_result,
        "disappearance": {
            **disappearance,
            "disappeared": not disappearance["observed"],
        },
        "passed": bool(
            observed["observed"]
            and cancel.get("cancel_sent")
            and not disappearance["observed"]
        ),
    }


def watch_and_cancel_print_dialog(timeout: float) -> dict[str, Any]:
    script = f'''
set deadline to (current date) + {timeout}
repeat while (current date) < deadline
  tell application "System Events"
    repeat with proc in application processes
      set procName to name of proc
      repeat with win in windows of proc
        try
          set winName to name of win
          if winName contains "Print" or winName contains "Printer" then
            key code 53
            return "print-dialog-cancel-sent process=" & procName & " window=" & winName
          end if
        end try
      end repeat
    end repeat
  end tell
  delay 0.1
end repeat
return "not-observed"
'''
    result = run_cmd(["osascript", "-e", script], timeout=timeout + 3)
    return {
        "mechanism": "osascript-system-events-print-window-escape",
        "result": result,
        "dialog_observed": result["returncode"] == 0
        and "print-dialog-cancel-sent" in result["stdout"],
        "cancel_sent": result["returncode"] == 0
        and "print-dialog-cancel-sent" in result["stdout"],
    }


def run_save_print_probe(
    log_dir: pathlib.Path,
    devtools_port: int,
    url_contains: str,
    server_port: int,
    allow_native_print_click: bool,
    timeout_seconds: int,
    settle_seconds: int,
) -> tuple[str, str, pathlib.Path]:
    out_dir = log_dir / "native-print"
    out_dir.mkdir(parents=True, exist_ok=True)
    downloads_dir = out_dir / "downloads"
    downloads_dir.mkdir(parents=True, exist_ok=True)
    bridge_trace_file = log_dir / "pdf-print-bridge.log"
    bridge_trace_file.write_text("", encoding="utf-8")
    cmd = [
        "node",
        str(SAVE_PRINT_TITLE_LOCAL_PROBE),
        "--devtools-port",
        str(devtools_port),
        "--url-contains",
        url_contains,
        "--out-dir",
        str(out_dir),
        "--downloads-dir",
        str(downloads_dir),
        "--timeout-seconds",
        str(timeout_seconds),
        "--settle-seconds",
        str(settle_seconds),
        "--http-pdf-url",
        f"http://127.0.0.1:{server_port}/bitcoin.pdf",
        "--file-pdf-url",
        BITCOIN_PDF.resolve().as_uri(),
        "--embedded-html-url",
        f"http://127.0.0.1:{server_port}/embedded-pdf.html",
        "--trace-file",
        str(log_dir / "pdf-input.log"),
        "--roamium-stderr",
        str(log_dir / "roamium.stderr"),
        "--print-bridge-trace-file",
        str(bridge_trace_file),
    ]
    if allow_native_print_click:
        cmd.extend(["--allow-native-print-click", "1"])
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
    return (
        "ok" if proc.returncode == 0 else "error",
        proc.stderr.strip(),
        out_dir / "save-print-title-local-summary.json",
    )


def classify(
    args: argparse.Namespace,
    state: HarnessState,
    preflight: dict[str, Any],
    queue_before: dict[str, Any],
    queue_after: dict[str, Any] | None,
    probe_summary: dict[str, Any] | None,
    print_watch: dict[str, Any] | None,
) -> None:
    if args.probe == "safety-gate":
        state.first_failing_hop = "native-print-safety-gate-not-ready"
        return
    if not args.allow_native_dialog_click:
        state.first_failing_hop = "native-print-safety-gate-not-ready"
    elif not preflight.get("passed"):
        state.first_failing_hop = "native-print-safety-gate-not-ready"
    elif not queue_before:
        state.first_failing_hop = "native-print-safety-gate-not-ready"
    elif not state.server_register_received:
        state.first_failing_hop = "roamium-not-registered"
    elif not state.tab_ready_id:
        state.first_failing_hop = "tab-not-ready"
    elif not state.devtools_port:
        state.first_failing_hop = "devtools-missing"
    elif state.probe_status != "ok" or not probe_summary:
        state.first_failing_hop = "native-print-observation-gap"
    elif state.roamium_exited_before_shutdown:
        state.first_failing_hop = "native-print-observation-gap"
    elif (
        json.dumps(queue_before, sort_keys=True)
        != json.dumps(queue_after or {}, sort_keys=True)
        and (print_watch or {}).get("dialog_observed") is not True
    ):
        state.first_failing_hop = "native-print-job-submitted-unexpectedly"
    else:
        print_summary = probe_summary.get("print") or {}
        status = print_summary.get("status")
        if status == "print-control-missing":
            state.first_failing_hop = "native-print-control-missing"
        elif status == "print-ready-disabled-by-flags":
            state.first_failing_hop = "native-print-disabled-by-load-time-flags"
        elif status != "print-native-click-sent":
            state.first_failing_hop = "native-print-click-not-sent"
        elif print_watch and print_watch.get("dialog_observed") and print_watch.get("cancel_sent"):
            state.first_failing_hop = "native-print-dialog-seen-cancelled"
        elif print_watch and print_watch.get("dialog_observed"):
            state.first_failing_hop = "native-print-dialog-seen-cancel-failed"
        else:
            state.first_failing_hop = "native-print-click-sent-no-dialog"


def write_summary(
    log_dir: pathlib.Path,
    args: argparse.Namespace,
    state: HarnessState,
    extra: dict[str, Any],
) -> None:
    data = {
        "probe": args.probe,
        "allow_native_dialog_click": args.allow_native_dialog_click,
        "server_register_received": state.server_register_received,
        "create_tab_sent": state.create_tab_sent,
        "tab_ready_id": state.tab_ready_id,
        "resize_sent": state.resize_sent,
        "focus_sent": state.focus_sent,
        "devtools_port": state.devtools_port,
        "probe_status": state.probe_status,
        "roamium_exited_before_shutdown": state.roamium_exited_before_shutdown,
        "roamium_exit_code_before_shutdown": state.roamium_exit_code_before_shutdown,
        "first_failing_hop": state.first_failing_hop,
    }
    data.update(extra)
    (log_dir / "pdf-native-print-summary.json").write_text(
        json.dumps(data, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--log-dir", required=True)
    parser.add_argument("--probe", choices=["safety-gate", "native-dialog"], required=True)
    parser.add_argument("--allow-native-dialog-click", action="store_true")
    parser.add_argument("--width", type=int, default=1200)
    parser.add_argument("--height", type=int, default=900)
    parser.add_argument("--pdf-port", type=int, default=0)
    parser.add_argument("--setup-timeout", type=float, default=30)
    parser.add_argument("--capture-timeout-seconds", type=int, default=30)
    parser.add_argument("--settle-seconds", type=int, default=8)
    parser.add_argument("--watch-timeout", type=float, default=8)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    log_dir = pathlib.Path(args.log_dir).resolve()
    log_dir.mkdir(parents=True, exist_ok=True)
    if not ROAMIUM.exists():
        raise SystemExit(f"missing Roamium binary: {ROAMIUM}")
    if not BITCOIN_PDF.exists():
        raise SystemExit(f"missing Bitcoin PDF fixture: {BITCOIN_PDF}")
    if not SAVE_PRINT_TITLE_LOCAL_PROBE.exists():
        raise SystemExit(f"missing print probe: {SAVE_PRINT_TITLE_LOCAL_PROBE}")

    preflight = preflight_dialog(args.watch_timeout)
    queue_before = print_queue_state()
    extra: dict[str, Any] = {
        "preflight": preflight,
        "print_queue_before": queue_before,
        "safety_gate_passed": bool(preflight.get("passed")),
    }
    state = HarnessState()
    if args.probe == "safety-gate" or not args.allow_native_dialog_click or not preflight.get("passed"):
        classify(args, state, preflight, queue_before, None, None, None)
        write_summary(log_dir, args, state, extra)
        return 0 if args.probe == "safety-gate" else 1

    pdf_server = start_pdf_server(log_dir, args.pdf_port)
    host, port = pdf_server.server_address
    url = f"http://{host}:{port}/bitcoin.pdf"
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
    env["TERMSURF_PDF_INPUT_TRACE_FILE"] = str(log_dir / "pdf-input.log")
    env["TERMSURF_PDF_PRINT_BRIDGE_TRACE"] = "1"
    env["TERMSURF_PDF_PRINT_BRIDGE_TRACE_FILE"] = str(log_dir / "pdf-print-bridge.log")
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
    print_watch: dict[str, Any] | None = None
    probe_summary = None
    queue_after = None
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
            watcher_result: dict[str, Any] = {}

            def run_watcher() -> None:
                nonlocal watcher_result
                watcher_result = watch_and_cancel_print_dialog(args.watch_timeout)

            watcher_thread = threading.Thread(target=run_watcher)
            watcher_thread.start()
            state.probe_status, probe_error, probe_path = run_save_print_probe(
                log_dir,
                state.devtools_port,
                "bitcoin.pdf",
                port,
                True,
                args.capture_timeout_seconds,
                args.settle_seconds,
            )
            extra["probe_error"] = probe_error
            watcher_thread.join(timeout=args.watch_timeout + 5)
            print_watch = watcher_result or {
                "dialog_observed": False,
                "cancel_sent": False,
                "timed_out": True,
            }
            extra["print_dialog_watch"] = print_watch
            probe_summary = json.loads(probe_path.read_text(encoding="utf-8")) if probe_path.exists() else None
            extra["probe_summary"] = probe_summary
            queue_after = print_queue_state()
            extra["print_queue_after"] = queue_after

        exit_code = proc.poll()
        state.roamium_exited_before_shutdown = exit_code is not None
        state.roamium_exit_code_before_shutdown = exit_code
        classify(args, state, preflight, queue_before, queue_after, probe_summary, print_watch)
        write_summary(log_dir, args, state, extra)
        return 0 if state.first_failing_hop == "native-print-dialog-seen-cancelled" else 1
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
