#!/usr/bin/env python3
"""Live macOS bell attention request guard for Issue 805 CFG-223."""

from __future__ import annotations

import json
import os
import re
import shlex
import subprocess
import tempfile
import textwrap
import time
from pathlib import Path

from macos_window_padding_pixel_runtime import (
    APP,
    ROOT,
    crash_reports,
    create_terminal_window,
    quote_applescript,
    require,
    run_osascript,
    scoped_pids,
    terminate_process,
    wait_for_app,
    wait_for_crash_report_settle,
    wait_for_file,
)


def write_config(path: Path, attention: bool) -> None:
    features = (
        "no-system,no-audio,attention,no-title,no-border"
        if attention
        else "no-system,no-audio,no-attention,no-title,no-border"
    )
    path.write_text(
        "\n".join(
            [
                "macos-applescript = true",
                "quit-after-last-window-closed = true",
                "cursor-style-blink = false",
                "font-size = 16",
                "window-width = 100",
                "window-height = 34",
                "background = #102030",
                "foreground = #ffffff",
                "background-opacity = 1",
                "macos-titlebar-style = hidden",
                "window-padding-x = 0",
                "window-padding-y = 0",
                f"bell-features = {features}",
                "",
            ]
        )
    )


def write_painter(path: Path, ready: Path, trigger: Path, bell: Path, label: str) -> None:
    path.write_text(
        textwrap.dedent(
            f"""
            from pathlib import Path
            import sys
            import time

            ready = Path({str(ready)!r})
            trigger = Path({str(trigger)!r})
            bell = Path({str(bell)!r})
            label = {label!r}

            sys.stdout.write("\\x1b[?25l\\x1b[?7l\\x1b[2J\\x1b[H")
            sys.stdout.write("\\x1b[10;20HIssue 805 Experiment 192")
            sys.stdout.write("\\x1b[12;20H" + label)
            sys.stdout.flush()
            ready.write_text("ready")

            deadline = time.monotonic() + 20
            while time.monotonic() < deadline:
                if trigger.exists():
                    break
                time.sleep(0.05)
            else:
                raise SystemExit("timed out waiting for trigger")

            time.sleep(2.0)
            sys.stdout.write("\\a")
            sys.stdout.flush()
            bell.write_text("ready")
            time.sleep(60)
            """
        ).lstrip()
    )


def launch_with_trace(config: Path, trace: Path, suite: str) -> int:
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
            f"ROASTTY_USER_DEFAULTS_SUITE={suite}",
            "--env",
            f"ROASTTY_UI_KEY_TRACE_PATH={trace}",
            str(APP),
        ],
        cwd=ROOT,
        text=True,
        capture_output=True,
    )
    if result.returncode != 0:
        raise AssertionError(
            "open failed\n"
            f"stdout:\n{result.stdout}\n"
            f"stderr:\n{result.stderr}"
        )

    deadline = time.monotonic() + 10
    while time.monotonic() < deadline:
        created = sorted(scoped_pids() - before)
        if created:
            return created[0]
        time.sleep(0.25)
    raise AssertionError("open did not start debug Roastty")


def read_trace(path: Path) -> str:
    return path.read_text(errors="replace") if path.exists() else ""


def wait_for_trace(trace: Path, needles: list[str], timeout: float = 15.0) -> str:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        text = read_trace(trace)
        if all(needle in text for needle in needles):
            return text
        time.sleep(0.25)
    text = read_trace(trace)
    missing = [needle for needle in needles if needle not in text]
    raise AssertionError(f"trace missing {missing}; trace was:\n{text}")


def activate_finder() -> int:
    run_osascript('tell application "Finder" to activate', timeout=10)
    result = run_osascript(
        textwrap.dedent(
            """
            tell application "System Events"
              delay 0.25
              return unix id of first application process whose frontmost is true
            end tell
            """
        ),
        timeout=10,
    )
    return int(result.stdout.strip().strip(","))


def run_case(temp: Path, attention: bool) -> dict[str, object]:
    label = "attention-enabled" if attention else "attention-disabled"
    config = temp / f"{label}.config"
    trace = temp / f"{label}.trace.log"
    ready = temp / f"{label}.ready"
    trigger = temp / f"{label}.trigger"
    bell = temp / f"{label}.bell"
    painter = temp / f"{label}.py"

    write_config(config, attention)
    write_painter(painter, ready, trigger, bell, label)
    pid = launch_with_trace(
        config,
        trace,
        f"com.termsurf.roastty.issue805.exp192.{label}",
    )

    try:
        wait_for_app(pid)
        command = f"{shlex.quote(str(Path('/usr/bin/python3')))} {shlex.quote(str(painter))}"
        terminal_id = create_terminal_window(command)
        wait_for_file(ready, f"{label} painter")
        front_pid = activate_finder()
        require(front_pid != pid, f"Roastty was still frontmost before BEL: {front_pid}")
        trigger.write_text("go")
        wait_for_file(bell, f"{label} bell")

        needles = [
            "ringBell target=surface",
            f"appBell system=false audio=false attention={'true' if attention else 'false'}",
            "surfaceBell state=true",
            "dockBadge authorizationStatus=",
        ]
        if attention:
            needles.append("appBell active=false")
        trace_text = wait_for_trace(trace, needles)
        request_ids = [
            int(match.group(1))
            for match in re.finditer(r"appBell attentionRequest=(-?\d+)", trace_text)
        ]
        if attention:
            require(request_ids, f"missing attention request ID; trace was:\n{trace_text}")
        else:
            require(not request_ids, f"disabled attention run emitted request IDs: {request_ids}")

        active_values = re.findall(r"appBell active=(true|false)", trace_text)
        if attention:
            require(active_values == ["false"], f"unexpected active trace values: {active_values}")
        else:
            require(not active_values, f"disabled attention run emitted active traces: {active_values}")
        dock_badges = re.findall(r"dockBadge bellCount=(\d+) label=([^\n]+)", trace_text)
        settings_matches = [
            (int(match.group(1)), int(match.group(2)))
            for match in re.finditer(
                r"dockBadge authorizationStatus=(\d+) badgeSetting=(\d+)",
                trace_text,
            )
        ]
        require(settings_matches, f"missing dock badge authorization trace; trace was:\n{trace_text}")
        return {
            "attention": attention,
            "pid": pid,
            "terminal_id": terminal_id,
            "front_pid_before_bel": front_pid,
            "request_ids": request_ids,
            "active_values": active_values,
            "dock_badges": dock_badges,
            "dock_badge_settings": settings_matches,
            "trace_tail": trace_text.splitlines()[-24:],
        }
    finally:
        terminate_process(pid)


def main() -> int:
    require(APP.is_dir(), f"app not built: {APP}")
    before_crashes = crash_reports()

    with tempfile.TemporaryDirectory(prefix="termsurf-issue805-exp192-dock-attention-") as temp_dir:
        temp = Path(temp_dir)
        enabled = run_case(temp, attention=True)
        disabled = run_case(temp, attention=False)

        settings = enabled["dock_badge_settings"][-1]
        badge_allowed = (
            settings[0] == 2
            and settings[1] == 2
        )
        if badge_allowed:
            require(
                ("1", "1") in enabled["dock_badges"],
                f"badge setting allowed but enabled run did not publish label=1: {enabled['dock_badges']}",
            )

        evidence = {
            "notification_settings": {
                "authorizationStatus": settings[0],
                "badgeSetting": settings[1],
            },
            "badge_allowed": badge_allowed,
            "enabled": enabled,
            "disabled": disabled,
        }
        latest = Path("/tmp/termsurf-issue805-exp192-dock-attention-latest.json")
        latest.write_text(json.dumps(evidence, indent=2, sort_keys=True))

    new_crashes = wait_for_crash_report_settle(before_crashes)
    require(
        not new_crashes,
        "Roastty wrote crash reports during dock attention workflow: "
        + ", ".join(str(path) for path in sorted(new_crashes)),
    )

    print("macos_live_bell_attention_dock_state=pass")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
