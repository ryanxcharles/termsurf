#!/usr/bin/env python3
"""Live native context-menu trace guard for Issue 805 Experiment 187."""

from __future__ import annotations

import os
import subprocess
import tempfile
import textwrap
import time
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
APP = ROOT / "roastty/macos/build/Debug/Roastty.app"


def require(condition: bool, message: str) -> None:
    if not condition:
        raise AssertionError(message)


def quote_applescript(value: str | Path) -> str:
    text = str(value)
    return '"' + text.replace("\\", "\\\\").replace('"', '\\"') + '"'


def run_osascript(script: str, timeout: int = 30) -> subprocess.CompletedProcess[str]:
    result = subprocess.run(
        ["osascript", "-e", script],
        cwd=ROOT,
        text=True,
        capture_output=True,
        timeout=timeout,
    )
    if result.returncode != 0:
        raise AssertionError(
            "osascript failed\n"
            f"stdout:\n{result.stdout}\n"
            f"stderr:\n{result.stderr}\n"
            f"script:\n{script}"
        )
    return result


def scoped_pids() -> set[int]:
    scoped = subprocess.run(
        ["pgrep", "-f", f"{APP}/Contents/MacOS/roastty"],
        text=True,
        capture_output=True,
    )
    return {int(pid_text) for pid_text in scoped.stdout.split()}


def launch_app(config: Path, trace: Path) -> int:
    before = scoped_pids()
    require(not before, f"debug Roastty app is already running: {sorted(before)}")
    result = subprocess.run(
        [
            "open",
            "-n",
            "--env",
            f"ROASTTY_CONFIG_PATH={config}",
            "--env",
            "ROASTTY_CLEAR_USER_DEFAULTS=1",
            "--env",
            "ROASTTY_USER_DEFAULTS_SUITE=com.termsurf.roastty.issue805.exp187.contextmenu",
            "--env",
            f"ROASTTY_UI_KEY_TRACE_PATH={trace}",
            "--env",
            "ROASTTY_UI_TEST_ENABLE_CONTEXT_MENU_ACTION=1",
            str(APP),
        ],
        cwd=ROOT,
        text=True,
        capture_output=True,
    )
    require(result.returncode == 0, f"open failed\nstdout:\n{result.stdout}\nstderr:\n{result.stderr}")

    deadline = time.monotonic() + 10
    while time.monotonic() < deadline:
        created = sorted(scoped_pids() - before)
        if created:
            return created[0]
        time.sleep(0.25)
    raise AssertionError("open did not start a scoped debug Roastty process")


def wait_for_app(pid: int, timeout: float = 20.0) -> None:
    app_literal = quote_applescript(APP)
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if subprocess.run(["ps", "-p", str(pid)], stdout=subprocess.DEVNULL).returncode != 0:
            raise AssertionError("Roastty debug process exited before AppleScript was ready")
        try:
            result = run_osascript(f"tell application {app_literal} to count of windows", timeout=5)
        except (AssertionError, subprocess.TimeoutExpired):
            time.sleep(0.5)
            continue
        if result.stdout.strip().isdigit():
            return
        time.sleep(0.5)
    raise AssertionError("Roastty did not become AppleScript-addressable in time")


def terminate_process(pid: int) -> None:
    try:
        try:
            run_osascript(f"tell application {quote_applescript(APP)} to quit", timeout=5)
        except Exception:
            pass
        for _ in range(20):
            if pid not in scoped_pids():
                return
            time.sleep(0.25)
    finally:
        if pid in scoped_pids():
            try:
                os.kill(pid, 9)
            except ProcessLookupError:
                pass


def read_trace(path: Path) -> str:
    if not path.exists():
        return ""
    return path.read_text(errors="replace")


def wait_for_trace(path: Path, needles: list[str], timeout: float = 10.0) -> str:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        text = read_trace(path)
        if all(needle in text for needle in needles):
            return text
        time.sleep(0.25)
    text = read_trace(path)
    missing = [needle for needle in needles if needle not in text]
    raise AssertionError(f"trace did not contain {missing}; trace was:\n{text}")


def write_config(path: Path) -> None:
    path.write_text(
        "\n".join(
            [
                "macos-applescript = true",
                "right-click-action = context-menu",
                "font-size = 16",
                "",
            ]
        )
    )


def main() -> int:
    require(APP.is_dir(), f"app not built: {APP}")

    with tempfile.TemporaryDirectory(prefix="termsurf-issue805-exp187-menu-") as temp_dir:
        temp = Path(temp_dir)
        config = temp / "config.roastty"
        trace = temp / "trace.log"
        write_config(config)

        pid = launch_app(config, trace)
        try:
            wait_for_app(pid)
            app_literal = quote_applescript(APP)
            script = textwrap.dedent(
                f"""
                tell application {app_literal}
                  activate
                  set cfg to new surface configuration from {{command:"/bin/sh -c 'sleep 60'", wait after command:true}}
                  new window with configuration cfg
                  delay 1
                  set t0 to focused terminal of selected tab of front window
                  perform action "ui_test_context_menu" on t0
                end tell
                """
            )
            run_osascript(script, timeout=30)
            wait_for_trace(
                trace,
                [
                    "contextMenu items=Paste",
                    "Split Right",
                    "Change Terminal Title",
                    "contextMenu uiTestAction items=Paste",
                ],
            )
        finally:
            terminate_process(pid)

    print("macos_native_context_menu_trace_runtime=pass")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
