#!/usr/bin/env python3
"""Verify Roastty terminfo is only a mechanical rename of pinned Ghostty."""

from __future__ import annotations

import difflib
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
UPSTREAM = ROOT / "vendor/ghostty/zig-out/share/terminfo/ghostty.terminfo"
ROASTTY = ROOT / "roastty/resources/terminfo/roastty.terminfo"


def main() -> int:
    expected = UPSTREAM.read_text()
    expected = expected.replace("xterm-ghostty", "xterm-roastty")
    expected = expected.replace("Ghostty", "Roastty")
    expected = expected.replace("ghostty", "roastty")
    actual = ROASTTY.read_text() if ROASTTY.exists() else ""

    if actual == expected:
        return 0

    print(f"{ROASTTY} is not a mechanical rename of {UPSTREAM}", file=sys.stderr)
    for line in difflib.unified_diff(
        actual.splitlines(),
        expected.splitlines(),
        fromfile=str(ROASTTY),
        tofile=f"{UPSTREAM} (renamed)",
        lineterm="",
        n=3,
    ):
        print(line, file=sys.stderr)
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
