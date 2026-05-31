#!/usr/bin/env python3
"""Verify TermSurf regular-profile and incognito session isolation."""

from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import http.server
import importlib.util
import json
import os
import pathlib
import socket
import socketserver
import struct
import subprocess
import sys
import tempfile
import threading
import time
from typing import Any
from urllib.parse import parse_qs, quote, urlparse


ROOT = pathlib.Path(__file__).resolve().parents[1]
DEFAULT_ROAMIUM = ROOT / "chromium/src/out/Default/roamium"
DEFAULT_LOG_ROOT = ROOT / "logs/issue-799-browser-api-audit"
HELPER_PATH = ROOT / "scripts/test-issue-799-browser-api-audit.py"

spec = importlib.util.spec_from_file_location("issue799_browser_api_audit", HELPER_PATH)
if spec is None or spec.loader is None:
    raise RuntimeError(f"failed to load helper module: {HELPER_PATH}")
helper = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = helper
spec.loader.exec_module(helper)


STATE_KEY = "termsurfIssue799State"
COOKIE_NAME = "termsurf_issue_799_cookie"


class SessionState:
    def __init__(self, run_dir: pathlib.Path) -> None:
        self.run_dir = run_dir
        self.lock = threading.Lock()
        self.reports: list[dict[str, Any]] = []
        self.cookie_events: list[dict[str, Any]] = []

    def add_report(self, report: dict[str, Any]) -> None:
        with self.lock:
            report["received_at"] = time.time()
            self.reports.append(report)
            append_jsonl(self.run_dir / "session-reports.jsonl", report)

    def add_cookie_event(self, event: dict[str, Any]) -> None:
        with self.lock:
            event["received_at"] = time.time()
            self.cookie_events.append(event)
            append_jsonl(self.run_dir / "session-cookie-events.jsonl", event)

    def reports_for(self, probe: str) -> list[dict[str, Any]]:
        with self.lock:
            return [item for item in self.reports if item.get("probe") == probe]

    def cookie_events_for(self, probe: str) -> list[dict[str, Any]]:
        with self.lock:
            return [item for item in self.cookie_events if item.get("probe") == probe]


class ThreadingTcpServer(socketserver.ThreadingTCPServer):
    allow_reuse_address = True
    daemon_threads = True


def append_jsonl(path: pathlib.Path, value: dict[str, Any]) -> None:
    with path.open("a", encoding="utf-8") as file:
        file.write(json.dumps(value, sort_keys=True) + "\n")


def timestamp() -> str:
    return dt.datetime.now(dt.timezone.utc).strftime("%Y%m%d-%H%M%S-%f")


def start_server(state: SessionState) -> ThreadingTcpServer:
    class Handler(http.server.BaseHTTPRequestHandler):
        def log_message(self, fmt: str, *args: object) -> None:
            return

        def send_bytes(self, body: bytes, content_type: str) -> None:
            self.send_response(200)
            self.send_header("Content-Type", content_type)
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)

        def do_GET(self) -> None:
            parsed = urlparse(self.path)
            query = parse_qs(parsed.query)
            probe = query.get("probe", ["unknown"])[-1]
            state.add_cookie_event(
                {
                    "probe": probe,
                    "path": parsed.path,
                    "cookie": self.headers.get("Cookie", ""),
                }
            )
            if parsed.path == "/session/write.html":
                storage = query.get("storage", [""])[-1]
                cookie = query.get("cookie", [""])[-1]
                self.send_bytes(
                    render_write_page(probe, storage, cookie),
                    "text/html; charset=utf-8",
                )
                return
            if parsed.path == "/session/read.html":
                self.send_bytes(render_read_page(probe), "text/html; charset=utf-8")
                return
            self.send_error(404)

        def do_POST(self) -> None:
            parsed = urlparse(self.path)
            if parsed.path != "/session/report":
                self.send_error(404)
                return
            length = int(self.headers.get("Content-Length", "0") or "0")
            raw = self.rfile.read(length)
            try:
                report = json.loads(raw.decode("utf-8"))
            except json.JSONDecodeError as error:
                report = {"probe": "unknown", "status": "bad_json", "error": str(error)}
            report["cookie_header"] = self.headers.get("Cookie", "")
            state.add_report(report)
            self.send_bytes(b"ok", "text/plain; charset=utf-8")

    server = ThreadingTcpServer(("127.0.0.1", 0), Handler)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    return server


def html_page(script: str) -> bytes:
    return f"""<!doctype html>
<meta charset="utf-8">
<title>Issue 799 session probe</title>
<script>
const stateKey = {json.dumps(STATE_KEY)};
const cookieName = {json.dumps(COOKIE_NAME)};
async function sendReport(report) {{
  await fetch('/session/report', {{
    method: 'POST',
    headers: {{'Content-Type': 'application/json'}},
    body: JSON.stringify(report)
  }});
}}
(async () => {{
  try {{
{script}
  }} catch (error) {{
    await sendReport({{
      probe: 'unknown',
      status: 'error',
      error: String(error),
      errorName: error?.name || null
    }});
  }}
}})();
</script>
""".encode(
        "utf-8"
    )


def render_write_page(probe: str, storage: str, cookie: str) -> bytes:
    return html_page(
        f"""
    const probe = {json.dumps(probe)};
    const storageValue = {json.dumps(storage)};
    const cookieValue = {json.dumps(cookie)};
    localStorage.setItem(stateKey, storageValue);
    document.cookie = `${{cookieName}}=${{encodeURIComponent(cookieValue)}}; path=/; max-age=3600; SameSite=Lax`;
    await sendReport({{
      probe,
      status: 'wrote',
      localStorageValue: localStorage.getItem(stateKey),
      documentCookie: document.cookie
    }});
"""
    )


def render_read_page(probe: str) -> bytes:
    return html_page(
        f"""
    const probe = {json.dumps(probe)};
    await sendReport({{
      probe,
      status: 'read',
      localStorageValue: localStorage.getItem(stateKey),
      documentCookie: document.cookie
    }});
"""
    )


def session_url(base_url: str, action: str, probe: str, storage: str = "", cookie: str = "") -> str:
    url = f"{base_url}/session/{action}.html?probe={quote(probe)}"
    if storage:
        url += f"&storage={quote(storage)}"
    if cookie:
        url += f"&cookie={quote(cookie)}"
    return url


def recv_message(conn: socket.socket) -> tuple[int, bytes] | None:
    try:
        header = conn.recv(4)
    except socket.timeout:
        return None
    if not header:
        return None
    size = struct.unpack("<I", header)[0]
    payload = bytearray()
    while len(payload) < size:
        chunk = conn.recv(size - len(payload))
        if not chunk:
            return None
        payload.extend(chunk)
    return helper.inner_payload(bytes(payload))


def run_roamium_sequence(
    *,
    name: str,
    urls: list[tuple[str, str]],
    state: SessionState,
    roamium: pathlib.Path,
    profile_dir: pathlib.Path,
    run_dir: pathlib.Path,
    incognito: bool = False,
    seconds: float = 8.0,
    width: int = 900,
    height: int = 700,
) -> dict[str, Any]:
    session_dir = run_dir / "sessions" / name
    session_dir.mkdir(parents=True, exist_ok=True)
    socket_path = (
        pathlib.Path(tempfile.gettempdir())
        / f"ts799-session-{os.getpid()}-{hashlib.sha1(name.encode()).hexdigest()[:8]}.sock"
    )
    socket_path.unlink(missing_ok=True)
    listener = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    listener.bind(str(socket_path))
    listener.listen(1)
    listener.settimeout(min(20.0, seconds))

    stdout_path = session_dir / "roamium.stdout"
    stderr_path = session_dir / "roamium.stderr"
    messages_path = session_dir / "messages.log"
    stdout = stdout_path.open("wb")
    stderr = stderr_path.open("wb")
    command = [
        str(roamium),
        f"--ipc-socket={socket_path}",
        f"--user-data-dir={profile_dir}",
        "--no-sandbox",
        "--enable-logging=stderr",
    ]
    if incognito:
        command.append("--incognito")
    proc = subprocess.Popen(
        command,
        cwd=str(ROOT / "chromium/src"),
        stdout=stdout,
        stderr=stderr,
        env=os.environ.copy(),
    )

    sent_create = False
    tab_id: int | None = None
    current_index = 0
    completed_at: float | None = None
    sent_close = False
    start = time.time()
    conn: socket.socket | None = None
    try:
        with messages_path.open("w", encoding="utf-8") as messages:
            messages.write("command " + json.dumps(command) + "\n")
            try:
                conn, _ = listener.accept()
                conn.settimeout(0.2)
            except socket.timeout:
                messages.write("accept timeout\n")
                conn = None
            while time.time() - start < seconds:
                if proc.poll() is not None:
                    break
                if current_index < len(urls):
                    probe = urls[current_index][0]
                    if state.reports_for(probe):
                        current_index += 1
                        if tab_id and current_index < len(urls):
                            next_url = urls[current_index][1]
                            helper.send_message(
                                conn,
                                5,
                                helper.navigate_payload(tab_id, next_url),
                            )
                            messages.write(f"sent Navigate url={next_url}\n")
                            messages.flush()
                if current_index >= len(urls):
                    if completed_at is None:
                        completed_at = time.time()
                        messages.write("all reports received; settling before shutdown\n")
                        messages.flush()
                    if tab_id and not sent_close:
                        helper.send_message(conn, 4, helper.varint_field(1, tab_id))
                        sent_close = True
                        messages.write("sent CloseTab before shutdown\n")
                        messages.flush()
                    if time.time() - completed_at >= 2.0:
                        break
                if conn is None:
                    time.sleep(0.1)
                    continue
                message = recv_message(conn)
                if message is None:
                    continue
                top, body = message
                messages.write(f"t={time.time() - start:.3f} top_field={top}\n")
                messages.flush()
                if top == 12 and not sent_create:
                    first_url = urls[0][1]
                    helper.send_message(
                        conn,
                        1,
                        helper.create_tab_payload(first_url, width, height),
                    )
                    sent_create = True
                    messages.write(f"sent CreateTab url={first_url}\n")
                    messages.flush()
                elif top == 13:
                    tab_id = helper.tab_ready_id(body)
                    if tab_id:
                        helper.send_message(
                            conn,
                            3,
                            helper.resize_payload(tab_id, width, height),
                        )
                        helper.send_message(
                            conn,
                            10,
                            helper.focus_changed_payload(tab_id, True),
                        )
                        messages.write(f"tab_ready id={tab_id}\n")
                        messages.flush()
    finally:
        if conn is not None:
            conn.close()
        listener.close()
        socket_path.unlink(missing_ok=True)
        try:
            proc.terminate()
            proc.wait(timeout=3)
        except subprocess.TimeoutExpired:
            proc.kill()
            proc.wait(timeout=3)
        stdout.close()
        stderr.close()

    stdout_text = read_text(stdout_path)
    stderr_text = read_text(stderr_path)
    log_scan = helper.scan_logs(stderr_text)
    completed = all(state.reports_for(probe) for probe, _url in urls)
    result = {
        "name": name,
        "command": command,
        "incognito": incognito,
        "profile_dir": str(profile_dir),
        "completed": completed,
        "reports": {probe: state.reports_for(probe) for probe, _url in urls},
        "cookie_events": {probe: state.cookie_events_for(probe) for probe, _url in urls},
        "process_exit_code": proc.poll(),
        "crashed": bool(log_scan.get("crashed")),
        "missing_interfaces": log_scan.get("missing_interfaces", []),
        "empty_interfaces": log_scan.get("empty_interfaces", []),
        "log_dir": str(session_dir),
    }
    helper.write_json(session_dir / "session-result.json", result)
    return result


def read_text(path: pathlib.Path) -> str:
    try:
        return path.read_text(encoding="utf-8", errors="replace")
    except FileNotFoundError:
        return ""


def latest_report(result: dict[str, Any], probe: str) -> dict[str, Any] | None:
    reports = result.get("reports", {}).get(probe, [])
    return reports[-1] if reports else None


def server_cookie_seen(result: dict[str, Any], probe: str, value: str) -> bool:
    expected = f"{COOKIE_NAME}={value}"
    for event in result.get("cookie_events", {}).get(probe, []):
        if expected in str(event.get("cookie", "")):
            return True
    report = latest_report(result, probe)
    return bool(report and expected in str(report.get("cookie_header", "")))


def report_has_cookie(report: dict[str, Any] | None, value: str) -> bool:
    if not report:
        return False
    return f"{COOKIE_NAME}={value}" in str(report.get("documentCookie", ""))


def assert_report(
    failures: list[str],
    result: dict[str, Any],
    probe: str,
    *,
    storage: str | None,
    cookie: str | None,
    require_server_cookie: bool = False,
) -> None:
    report = latest_report(result, probe)
    if not report:
        failures.append(f"{probe}: missing report")
        return
    if report.get("localStorageValue") != storage:
        failures.append(
            f"{probe}: localStorage={report.get('localStorageValue')!r}, expected {storage!r}"
        )
    if cookie is None:
        if COOKIE_NAME in str(report.get("documentCookie", "")):
            failures.append(f"{probe}: unexpected document cookie {report.get('documentCookie')!r}")
        if server_cookie_seen(result, probe, ""):
            failures.append(f"{probe}: unexpected server cookie")
    else:
        if not report_has_cookie(report, cookie):
            failures.append(f"{probe}: missing document cookie {cookie!r}")
        if require_server_cookie and not server_cookie_seen(result, probe, cookie):
            failures.append(f"{probe}: server did not observe cookie {cookie!r}")


def run_webtui_cli_checks(run_dir: pathlib.Path) -> dict[str, Any]:
    web = ROOT / "target/debug/web"
    if not web.exists():
        return {"status": "skipped", "reason": f"missing web binary: {web}"}
    help_proc = subprocess.run(
        [str(web), "--help"],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        timeout=5,
    )
    reject_proc = subprocess.run(
        [str(web), "--incognito", "--profile", "default", "https://example.test"],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        timeout=5,
    )
    accept_env = os.environ.copy()
    accept_env.pop("TERMSURF_PANE_ID", None)
    accept_proc = subprocess.run(
        [str(web), "--incognito", "--profile", "incognito", "status"],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        timeout=5,
        env=accept_env,
    )
    result = {
        "status": "completed",
        "help_has_incognito": "--incognito" in help_proc.stdout,
        "reject_exit": reject_proc.returncode,
        "reject_stderr": reject_proc.stderr,
        "accept_exit": accept_proc.returncode,
        "accept_stdout": accept_proc.stdout,
        "accept_stderr": accept_proc.stderr,
    }
    helper.write_json(run_dir / "webtui-cli-checks.json", result)
    return result


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--roamium", type=pathlib.Path, default=DEFAULT_ROAMIUM)
    parser.add_argument("--log-dir", type=pathlib.Path)
    parser.add_argument("--seconds", type=float, default=8.0)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    roamium = args.roamium.resolve()
    if not roamium.exists():
        raise SystemExit(f"missing Roamium binary: {roamium}")
    run_dir = (args.log_dir or DEFAULT_LOG_ROOT / timestamp()).resolve()
    run_dir.mkdir(parents=True, exist_ok=True)
    state = SessionState(run_dir)
    server = start_server(state)
    host, port = server.server_address
    base_url = f"http://{host}:{port}"
    profiles_dir = run_dir / "profiles"
    profiles_dir.mkdir(parents=True, exist_ok=True)
    profile_a = profiles_dir / "profilea"
    profile_b = profiles_dir / "profileb"
    profile_incognito = profiles_dir / "incognito"
    failures: list[str] = []

    helper.write_json(
        run_dir / "run.json",
        {
            "command": sys.argv,
            "roamium": str(roamium),
            "chromium_branch": helper.chromium_branch(),
            "fixture_base_url": base_url,
            "started_at": dt.datetime.now(dt.timezone.utc).isoformat(),
        },
    )

    try:
        results: list[dict[str, Any]] = []
        results.append(
            run_roamium_sequence(
                name="profile-a-write",
                urls=[
                    (
                        "profile-a-write",
                        session_url(
                            base_url,
                            "write",
                            "profile-a-write",
                            "regular-a-storage",
                            "regular-a-cookie",
                        ),
                    )
                ],
                state=state,
                roamium=roamium,
                profile_dir=profile_a,
                run_dir=run_dir,
                seconds=args.seconds,
            )
        )
        results.append(
            run_roamium_sequence(
                name="profile-a-read",
                urls=[
                    (
                        "profile-a-read",
                        session_url(base_url, "read", "profile-a-read"),
                    )
                ],
                state=state,
                roamium=roamium,
                profile_dir=profile_a,
                run_dir=run_dir,
                seconds=args.seconds,
            )
        )
        results.append(
            run_roamium_sequence(
                name="profile-b-read",
                urls=[
                    (
                        "profile-b-read",
                        session_url(base_url, "read", "profile-b-read"),
                    )
                ],
                state=state,
                roamium=roamium,
                profile_dir=profile_b,
                run_dir=run_dir,
                seconds=args.seconds,
            )
        )
        results.append(
            run_roamium_sequence(
                name="reserved-incognito-regular-write",
                urls=[
                    (
                        "reserved-incognito-regular-write",
                        session_url(
                            base_url,
                            "write",
                            "reserved-incognito-regular-write",
                            "reserved-regular-storage",
                            "reserved-regular-cookie",
                        ),
                    )
                ],
                state=state,
                roamium=roamium,
                profile_dir=profile_incognito,
                run_dir=run_dir,
                seconds=args.seconds,
            )
        )
        results.append(
            run_roamium_sequence(
                name="reserved-incognito-private-read-write",
                urls=[
                    (
                        "reserved-incognito-private-read",
                        session_url(base_url, "read", "reserved-incognito-private-read"),
                    ),
                    (
                        "reserved-incognito-private-write",
                        session_url(
                            base_url,
                            "write",
                            "reserved-incognito-private-write",
                            "reserved-private-storage",
                            "reserved-private-cookie",
                        ),
                    ),
                ],
                state=state,
                roamium=roamium,
                profile_dir=profile_incognito,
                run_dir=run_dir,
                incognito=True,
                seconds=args.seconds,
            )
        )
        results.append(
            run_roamium_sequence(
                name="reserved-incognito-regular-read",
                urls=[
                    (
                        "reserved-incognito-regular-read",
                        session_url(base_url, "read", "reserved-incognito-regular-read"),
                    )
                ],
                state=state,
                roamium=roamium,
                profile_dir=profile_incognito,
                run_dir=run_dir,
                seconds=args.seconds,
            )
        )
        results.append(
            run_roamium_sequence(
                name="incognito-live",
                urls=[
                    (
                        "incognito-live-initial-read",
                        session_url(base_url, "read", "incognito-live-initial-read"),
                    ),
                    (
                        "incognito-live-write",
                        session_url(
                            base_url,
                            "write",
                            "incognito-live-write",
                            "private-live-storage",
                            "private-live-cookie",
                        ),
                    ),
                    (
                        "incognito-live-read",
                        session_url(base_url, "read", "incognito-live-read"),
                    ),
                ],
                state=state,
                roamium=roamium,
                profile_dir=profile_a,
                run_dir=run_dir,
                incognito=True,
                seconds=args.seconds,
            )
        )
        results.append(
            run_roamium_sequence(
                name="incognito-restart-read",
                urls=[
                    (
                        "incognito-restart-read",
                        session_url(base_url, "read", "incognito-restart-read"),
                    )
                ],
                state=state,
                roamium=roamium,
                profile_dir=profile_a,
                run_dir=run_dir,
                incognito=True,
                seconds=args.seconds,
            )
        )
    finally:
        server.shutdown()

    by_name = {result["name"]: result for result in results}
    for result in results:
        if not result.get("completed"):
            failures.append(f"{result['name']}: did not complete all reports")
        if result.get("crashed"):
            failures.append(f"{result['name']}: Chromium/Roamium crash detected")
        if result.get("missing_interfaces"):
            failures.append(f"{result['name']}: missing interfaces {result['missing_interfaces']}")

    assert_report(
        failures,
        by_name["profile-a-read"],
        "profile-a-read",
        storage="regular-a-storage",
        cookie="regular-a-cookie",
        require_server_cookie=True,
    )
    assert_report(
        failures,
        by_name["profile-b-read"],
        "profile-b-read",
        storage=None,
        cookie=None,
    )
    assert_report(
        failures,
        by_name["reserved-incognito-private-read-write"],
        "reserved-incognito-private-read",
        storage=None,
        cookie=None,
    )
    assert_report(
        failures,
        by_name["reserved-incognito-regular-read"],
        "reserved-incognito-regular-read",
        storage="reserved-regular-storage",
        cookie="reserved-regular-cookie",
        require_server_cookie=True,
    )
    assert_report(
        failures,
        by_name["incognito-live"],
        "incognito-live-initial-read",
        storage=None,
        cookie=None,
    )
    assert_report(
        failures,
        by_name["incognito-live"],
        "incognito-live-read",
        storage="private-live-storage",
        cookie="private-live-cookie",
        require_server_cookie=True,
    )
    assert_report(
        failures,
        by_name["incognito-restart-read"],
        "incognito-restart-read",
        storage=None,
        cookie=None,
    )

    cli_checks = run_webtui_cli_checks(run_dir)
    if cli_checks.get("status") == "completed":
        if not cli_checks.get("help_has_incognito"):
            failures.append("web --help does not include --incognito")
        if cli_checks.get("reject_exit") == 0:
            failures.append("web --incognito --profile default did not fail")
        if cli_checks.get("accept_exit") != 0:
            failures.append("web --incognito --profile incognito was not accepted")
    else:
        failures.append(str(cli_checks.get("reason")))

    run_status = "pass" if not failures else "fail"
    run_info = {
        "command": sys.argv,
        "roamium": str(roamium),
        "chromium_branch": helper.chromium_branch(),
        "fixture_base_url": base_url,
        "finished_at": dt.datetime.now(dt.timezone.utc).isoformat(),
        "status": run_status,
        "failures": failures,
        "results": results,
        "webtui_cli_checks": cli_checks,
    }
    helper.write_json(run_dir / "run.json", run_info)
    helper.write_json(run_dir / "session-results.json", results)
    print(run_dir)
    print(json.dumps(run_info, indent=2, sort_keys=True))
    return 0 if run_status == "pass" else 1


if __name__ == "__main__":
    sys.exit(main())
