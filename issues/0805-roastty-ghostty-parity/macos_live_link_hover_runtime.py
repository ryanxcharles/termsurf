#!/usr/bin/env python3
"""Live macOS link-hover guard for Issue 805 CFG-223."""

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
    Rect,
    capture_window_id,
    crash_reports,
    create_terminal_window,
    focus_evidence,
    focused_window,
    require,
    terminate_process,
    wait_for_app,
    wait_for_crash_report_settle,
    wait_for_file,
)


INJECT = ROOT / "scripts/ghostty-app/inject.swift"
URL = "https://example.com/issue805-exp188-link-hover"
LINK_ROW = 8
LINK_COL = 10


def write_config(path: Path) -> None:
    path.write_text(
        "\n".join(
            [
                "macos-applescript = true",
                "quit-after-last-window-closed = true",
                "font-size = 16",
                "window-width = 100",
                "window-height = 34",
                "background = #102030",
                "foreground = #ffffff",
                "background-opacity = 1",
                "macos-titlebar-style = hidden",
                "window-padding-x = 0",
                "window-padding-y = 0",
                "link-previews = true",
                "",
            ]
        )
    )


def write_painter(path: Path, marker: Path) -> None:
    path.write_text(
        textwrap.dedent(
            f"""
            from pathlib import Path
            import sys
            import time

            marker = Path({str(marker)!r})
            url = {URL!r}
            for index in range(240):
                sys.stdout.write("\\x1b[?25h\\x1b[?7l\\x1b[2J\\x1b[H")
                sys.stdout.write("\\x1b[{LINK_ROW};{LINK_COL}H" + url)
                sys.stdout.flush()
                if index == 0:
                    marker.write_text("ready")
                time.sleep(0.1)
            time.sleep(30)
            """
        ).lstrip()
    )


def wait_for_trace(trace: Path, needles: list[str], timeout: float = 15.0) -> str:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        text = trace.read_text(errors="replace") if trace.exists() else ""
        if all(needle in text for needle in needles):
            return text
        time.sleep(0.25)
    text = trace.read_text(errors="replace") if trace.exists() else ""
    missing = [needle for needle in needles if needle not in text]
    raise AssertionError(f"trace missing {missing}; trace was:\n{text}")


def wait_for_resize(trace: Path, timeout: float = 10.0) -> dict[str, int]:
    deadline = time.monotonic() + timeout
    pattern = re.compile(r"resize rows=(\d+) cols=(\d+) width_px=(\d+) height_px=(\d+)")
    last_match = None
    while time.monotonic() < deadline:
        text = trace.read_text(errors="replace") if trace.exists() else ""
        for match in pattern.finditer(text):
            last_match = match
        if last_match:
            rows, cols, width_px, height_px = (int(value) for value in last_match.groups())
            return {
                "rows": rows,
                "cols": cols,
                "width_px": width_px,
                "height_px": height_px,
            }
        time.sleep(0.25)
    text = trace.read_text(errors="replace") if trace.exists() else ""
    raise AssertionError(f"trace never reported a terminal resize; trace was:\n{text}")


def global_point(window: Rect, row: int, col: int, cols: int, rows: int) -> tuple[float, float]:
    cell_width = window.width / cols
    cell_height = window.height / rows
    x = window.x + (col - 0.5) * cell_width
    y = window.y + (row - 0.5) * cell_height
    return x, y


def global_point_bottom_origin(window: Rect, row: int, col: int, cols: int, rows: int) -> tuple[float, float]:
    cell_width = window.width / cols
    cell_height = window.height / rows
    x = window.x + (col - 0.5) * cell_width
    y = window.y + window.height - (row - 0.5) * cell_height
    return x, y


def hover_trace_seen(trace_text: str) -> bool:
    return (
        "cursorShape raw=" in trace_text
        and "pointerStyle=link" in trace_text
        and f"mouseOverLink url={URL}" in trace_text
    )


def inject_move(x: float, y: float, *modifiers: str) -> None:
    result = subprocess.run(
        ["swift", str(INJECT), "move", f"{x:.1f}", f"{y:.1f}", *modifiers],
        cwd=ROOT,
        text=True,
        capture_output=True,
        timeout=10,
    )
    if result.returncode != 0:
        raise AssertionError(
            "mouse injection failed\n"
            f"stdout:\n{result.stdout}\n"
            f"stderr:\n{result.stderr}"
        )


def main() -> int:
    require(APP.is_dir(), f"app not built: {APP}")
    before_crashes = crash_reports()

    with tempfile.TemporaryDirectory(prefix="termsurf-issue805-exp188-link-hover-") as temp_dir:
        temp = Path(temp_dir)
        config = temp / "config.roastty"
        trace = temp / "trace.log"
        marker = temp / "marker.txt"
        painter = temp / "paint_link.py"
        screenshot = temp / "link-hover.png"
        evidence = temp / "evidence.json"

        write_config(config)
        write_painter(painter, marker)

        env = os.environ.copy()
        env["ROASTTY_CONFIG_PATH"] = str(config)
        env["ROASTTY_CLEAR_USER_DEFAULTS"] = "1"
        env["ROASTTY_USER_DEFAULTS_SUITE"] = "com.termsurf.roastty.issue805.exp188.linkhover"
        env["ROASTTY_UI_KEY_TRACE_PATH"] = str(trace)

        # The shared launcher does not accept extra env, so use `open` directly
        # while preserving its process-isolation assertions.
        before = subprocess.run(
            ["pgrep", "-f", f"{APP}/Contents/MacOS/roastty"],
            text=True,
            capture_output=True,
        )
        require(not before.stdout.split(), f"debug Roastty app is already running: {before.stdout}")
        result = subprocess.run(
            [
                "open",
                "-n",
                "--env",
                f"ROASTTY_CONFIG_PATH={config}",
                "--env",
                "ROASTTY_CLEAR_USER_DEFAULTS=1",
                "--env",
                "ROASTTY_USER_DEFAULTS_SUITE=com.termsurf.roastty.issue805.exp188.linkhover",
                "--env",
                f"ROASTTY_UI_KEY_TRACE_PATH={trace}",
                str(APP),
            ],
            cwd=ROOT,
            text=True,
            capture_output=True,
        )
        require(result.returncode == 0, f"open failed\nstdout:\n{result.stdout}\nstderr:\n{result.stderr}")

        deadline = time.monotonic() + 10
        pid = None
        while time.monotonic() < deadline:
            after = subprocess.run(
                ["pgrep", "-f", f"{APP}/Contents/MacOS/roastty"],
                text=True,
                capture_output=True,
            )
            created = [int(value) for value in after.stdout.split()]
            if created:
                pid = sorted(created)[0]
                break
            time.sleep(0.25)
        require(pid is not None, "open did not start debug Roastty")

        try:
            wait_for_app(pid)
            command = f"{shlex.quote(str(Path('/usr/bin/python3')))} {shlex.quote(str(painter))}"
            terminal_id = create_terminal_window(command)
            wait_for_file(marker, "link painter")
            resize = wait_for_resize(trace)

            focus = focus_evidence(pid)
            focused_bounds = focus["focused_bounds"]
            require(isinstance(focused_bounds, Rect), f"unexpected focused bounds: {focused_bounds}")
            window = focused_window(pid, focused_bounds)

            attempts = []
            target_cols = [LINK_COL + 4, LINK_COL + 8, LINK_COL + 12, LINK_COL + 20, LINK_COL + 32]
            vertical_offsets = [0, 15, 25, 35, 45, -15, -30, 60, 75, 90, 105]
            for target_col in target_cols:
                top_x, top_y = global_point(
                    window.bounds,
                    LINK_ROW,
                    target_col,
                    resize["cols"],
                    resize["rows"],
                )
                bottom_x, bottom_y = global_point_bottom_origin(
                    window.bounds,
                    LINK_ROW,
                    target_col,
                    resize["cols"],
                    resize["rows"],
                )
                candidates = [("top", top_x, top_y + offset, offset) for offset in vertical_offsets]
                candidates.append(("bottom", bottom_x, bottom_y, 0))
                for origin, x, y, vertical_offset in candidates:
                    inject_move(x, y, "command")
                    time.sleep(0.15)
                    inject_move(x, y, "command")
                    time.sleep(0.35)
                    attempts.append(
                        {
                            "origin": origin,
                            "row": LINK_ROW,
                            "col": target_col,
                            "x": x,
                            "y": y,
                            "vertical_offset": vertical_offset,
                        }
                    )
                    trace_text = trace.read_text(errors="replace") if trace.exists() else ""
                    if hover_trace_seen(trace_text):
                        break
                else:
                    continue
                break

            trace_text = wait_for_trace(
                trace,
                [
                    "cursorShape raw=",
                    "pointerStyle=link",
                    f"mouseOverLink url={URL}",
                ],
                timeout=15,
            )

            width, height = capture_window_id(window.id, screenshot)
            require(width > 0 and height > 0, f"empty screenshot dimensions: {width}x{height}")

            data = {
                "pid": pid,
                "terminal_id": terminal_id,
                "window_id": window.id,
                "window_bounds": window.bounds.__dict__,
                "focused_bounds": focused_bounds.__dict__,
                "resize": resize,
                "attempts": attempts,
                "screenshot": str(screenshot),
                "screenshot_size": {"width": width, "height": height},
                "trace_tail": trace_text.splitlines()[-20:],
            }
            evidence.write_text(json.dumps(data, indent=2, sort_keys=True))
            Path("/tmp/termsurf-issue805-exp188-link-hover-latest.json").write_text(evidence.read_text())
            Path("/tmp/termsurf-issue805-exp188-link-hover-latest.png").write_bytes(screenshot.read_bytes())
        finally:
            terminate_process(pid)

    new_crashes = wait_for_crash_report_settle(before_crashes)
    require(
        not new_crashes,
        "Roastty wrote crash reports during live link hover workflow: "
        + ", ".join(str(path) for path in sorted(new_crashes)),
    )

    print("macos_live_link_hover_runtime=pass")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
