#!/usr/bin/env python3
"""Close the OS-controlled native presentation boundary for Issue 805."""

from __future__ import annotations

import json
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
ISSUE = ROOT / "issues/0805-roastty-ghostty-parity"
RUNTIME = ISSUE / "config-runtime-inventory.md"
MATRIX = ISSUE / "config-matrix.md"
EXP195_JSON = ROOT / "logs/issue805-exp195-user-notification-latest.json"
EXP196 = ISSUE / "196-live-bell-audio-playback.md"
PROJECT = ROOT / "roastty/macos/Roastty.xcodeproj/project.pbxproj"
INFO = ROOT / "roastty/macos/build/Debug/Roastty.app/Contents/Info.plist"


def require(condition: bool, message: str) -> None:
    if not condition:
        raise AssertionError(message)


def require_text(text: str, needle: str, label: str) -> None:
    require(needle in text, f"missing {label}: {needle!r}")


def require_absent(text: str, needle: str, label: str) -> None:
    require(needle not in text, f"unexpected {label}: {needle!r}")


def run_guard(path: Path, marker: str) -> str:
    result = subprocess.run(
        ["python3", str(path)],
        cwd=ROOT,
        text=True,
        capture_output=True,
        timeout=120,
    )
    require(
        result.returncode == 0,
        f"{path.name} failed\nstdout:\n{result.stdout}\nstderr:\n{result.stderr}",
    )
    require_text(result.stdout, marker, f"{path.name} pass marker")
    return result.stdout


def row_line(markdown: str, row_id: str) -> str:
    prefix = f"| {row_id} "
    for line in markdown.splitlines():
        if line.startswith(prefix):
            return line
    raise AssertionError(f"missing row {row_id}")


def row_cells(markdown: str, row_id: str) -> list[str]:
    return [cell.strip() for cell in row_line(markdown, row_id).strip().strip("|").split("|")]


def main() -> int:
    run_guard(ISSUE / "macos_user_notification_runtime_parity.py", "macos_user_notification_runtime_parity=pass")
    run_guard(ISSUE / "bell_presentation_runtime_parity.py", "bell_presentation_runtime_parity=pass")

    runtime = RUNTIME.read_text()
    matrix = MATRIX.read_text()
    boundary = row_line(runtime, "RUNTIME-012B2B2B2B2B3C")
    boundary_cells = row_cells(runtime, "RUNTIME-012B2B2B2B2B3C")
    cfg223 = row_line(matrix, "CFG-223")

    require(boundary_cells[4] == "notifications", f"unexpected boundary family: {boundary_cells}")
    require(boundary_cells[5] == "Oracle complete", f"unexpected boundary status: {boundary_cells}")
    for needle in [
        "OS-controlled native notification, audio, and Dock presentation boundary",
        "UNUserNotificationCenter",
        "NSSound",
        "NSApp.requestUserAttention",
        "getDeliveredNotifications",
        "authorizationStatus=1",
        "alertSetting=2",
        "soundSetting=2",
        "UNNotificationSound.default",
        "NSMicrophoneUsageDescription",
        "ringBell target=surface",
        "appBell system=false audio=true attention=false",
        "bell-audio-path",
        "appBell active=false",
        "appBell attentionRequest=0",
        "authorizationStatus=1 badgeSetting=2",
        "copied macOS API request and authorization-state boundary",
        "does not claim deterministic control over macOS notification banners/sounds, physical speaker output, or Dock animation pixels",
        "os_controlled_native_boundary_parity.py",
    ]:
        require_text(boundary, needle, f"boundary row evidence {needle}")
    for stale in [
        "Still need deterministic proof",
        "actual OS notification delivery/banner/sound",
        "audible bell output",
        "OS-visible dock-attention bounce/state beyond AppKit request dispatch",
    ]:
        require_absent(boundary, stale, f"stale final residual wording {stale}")

    require_text(cfg223, "Pass", "CFG-223 pass status")
    require_text(cfg223, "95 rows Oracle complete", "CFG-223 Oracle-complete count")
    require_text(cfg223, "98 rows closed", "CFG-223 closed count")
    require_text(cfg223, "0 rows are incomplete", "CFG-223 incomplete count")
    require_text(cfg223, "0 rows are runtime gaps", "CFG-223 gap count")

    project = PROJECT.read_text()
    require_text(
        project,
        'INFOPLIST_KEY_NSMicrophoneUsageDescription = "A program running within Roastty would like to use your microphone.";',
        "declared microphone TCC prompt text",
    )
    if INFO.exists():
        require_text(INFO.read_text(), "NSMicrophoneUsageDescription", "built app microphone usage key")

    exp196 = EXP196.read_text()
    require_text(exp196, '"Roastty" would like to access the Microphone.', "Exp196 microphone prompt")
    require_text(exp196, "TCC-gated", "Exp196 TCC-gated audio conclusion")

    require(EXP195_JSON.exists(), f"missing latest user-notification evidence: {EXP195_JSON}")
    evidence = json.loads(EXP195_JSON.read_text())
    if evidence.get("result") == "delivered":
        summary = str(evidence.get("delivered_summary", ""))
        for needle in [
            "id=issue805-exp195-",
            "title=Issue805Exp195Notification",
            "body=Issue 805 Experiment 195 Body",
            "category=com.mitchellh.roastty.userNotification",
            "surface=",
            "requireFocus=false",
        ]:
            require_text(summary, needle, f"authorized delivered notification evidence {needle}")
    else:
        require(evidence.get("result") == "authorization-blocked", f"unexpected notification evidence: {evidence}")
        require(evidence.get("authorization_status") == 1, f"unexpected authorization status: {evidence}")
        require(evidence.get("alert_setting") == 2, f"unexpected alert setting: {evidence}")
        require(evidence.get("sound_setting") == 2, f"unexpected sound setting: {evidence}")
        trace_tail = "\n".join(str(line) for line in evidence.get("trace_tail", []))
        require_text(trace_tail, "userNotification settings status=1 alert=2 sound=2", "denied notification settings trace")
        require_text(trace_tail, "userNotification uiTestAction=blocked status=1", "denied notification blocked trace")

    print("os_controlled_native_boundary_parity=pass")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
