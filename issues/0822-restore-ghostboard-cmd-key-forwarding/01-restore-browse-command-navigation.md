+++
reviewer = "adversarial-review"
+++

# Experiment 1: Restore Browse-Mode Command Navigation Forwarding

## Description

Restore the missing Ghostboard-legacy behavior that let browser-owned
Command-key shortcuts reach Chromium while a pane is in browse mode.

The current browser-side path already handles `Cmd+[` as Back if a `KeyEvent`
reaches Roamium/Chromium. Swift maps `[` to `0xDB`; Zig forwards only when
`snapshotBrowserInput(pane_id, true)` proves the pane is browsing with a real
browser tab and attached browser file descriptor; Chromium maps
`Meta + VKEY_OEM_4` to `GoBack()`.

The missing piece is AppKit routing. `performKeyEquivalent(with:)` can receive
Command-key events before `keyDown(with:)`. Current Ghostboard only forwards
events to `keyDown` when Ghostty recognizes them as bindings or when AppKit
redispatches a matching timestamp. Plain `Cmd+[` can be swallowed before
`keyDown`, so the existing TermSurf forwarding path never runs.

This experiment restores a narrow browser-navigation bypass at the top of
`performKeyEquivalent(with:)`: for key-down events on browser navigation
shortcuts, call the existing `forwardTermSurfKeyDown(event)` path before Ghostty
binding/menu fallback. That call is the safety gate. It returns true only if the
current TermSurf pane/browser state accepts the event; otherwise the method
continues through normal Ghostty/AppKit handling. This is intentionally narrower
than legacy's broad `self.keyDown(with:)` browse-mode bypass because the current
direct forwarding path can prove whether the browser accepted the event before
`performKeyEquivalent` consumes it.

## Changes

- `ghostboard/macos/Sources/Ghostty/Surface View/SurfaceView_AppKit.swift`
  - Add a private helper that recognizes browser-owned navigation shortcuts:
    `Cmd+[`, `Cmd+]`, and `Cmd+R`.
  - In `performKeyEquivalent(with:)`, after the existing focused guard and
    before `surface.keyIsBinding`, call `forwardTermSurfKeyDown(event)` only for
    those shortcuts.
  - Return `true` only if forwarding succeeds, so non-browse mode, unattached
    browsers, or non-browser panes fall through to the current behavior.
  - Add trace logging for forwarded and rejected browser key-equivalent attempts
    so the smoke can distinguish AppKit swallowing from browser forwarding.
- `scripts/ghostboard-geometry-matrix.sh`
  - Add a narrow `browser-command-navigation` scenario that opens a page, enters
    browse mode, navigates to a second URL, sends `Cmd+[` through real macOS
    keyboard injection, and verifies:
    - Ghostboard logs `perform_key_equivalent_browser_forwarded`;
    - Zig logs `KeyEvent` with `windows_key_code=219` and `modifiers=8`;
    - Roamium/Chromium reports the original URL after Back.
  - Send `Cmd+[` once before browse mode and verify no
    `perform_key_equivalent_browser_forwarded` log and no browser `KeyEvent` are
    produced for that pre-browse attempt.
  - Include a small `Cmd+]` forward check if it can reuse the same fixture
    cheaply; otherwise leave Forward/Reload for the existing browser-state and
    navigation smoke coverage.
- `docs/keybindings.md`
  - Update the Browser navigation notes only if the implementation details
    change from the current description.

## Verification

Pass criteria:

1. `swiftc`/build verification for the edited macOS Swift source succeeds as
   part of the Ghostboard build.
2. `./scripts/build.sh ghostboard` succeeds.
3. `./scripts/ghostboard-geometry-matrix.sh browser-command-navigation` succeeds
   and its logs prove `Cmd+[` reached the browser as:
   - `perform_key_equivalent_browser_forwarded`;
   - `KeyEvent ... windows_key_code=219 ... modifiers=8`;
   - browser URL/title state returned to the prior page.
4. The same scenario proves a pre-browse `Cmd+[` attempt does not emit
   `perform_key_equivalent_browser_forwarded` or a browser `KeyEvent`.
5. `./scripts/ghostboard-geometry-matrix.sh copy-current-url-smoke` succeeds,
   proving Control-mode `Cmd+C` still copies the URL and Browse-mode `Cmd+C`
   does not run the Ghostboard URL-copy action.
6. `git diff --check` reports no whitespace errors.

Fail criteria:

- `Cmd+[` still has no `KeyEvent` evidence in browse mode.
- `performKeyEquivalent` consumes `Cmd+[` outside browse mode.
- Control-mode `Cmd+C` regresses.
- The implementation forwards broad Command-key traffic without a browser-owned
  shortcut check or without requiring `forwardTermSurfKeyDown` to accept the
  event.

## Design Review

Reviewed by a fresh-context Codex adversarial subagent.

Initial verdict: **Changes Required**.

- Required: the original design called `forwardTermSurfKeyDown(event)` directly
  but still required the existing `key_down_forwarded` log, which is emitted
  only from `keyDown`. Fixed by making the design consistently require the new
  `perform_key_equivalent_browser_forwarded` log for direct
  `performKeyEquivalent` forwarding.
- Optional: outside-browse non-consumption was listed only as a fail criterion.
  Fixed by adding a concrete pre-browse `Cmd+[` negative check.

Re-review verdict: **Approved**. The reviewer confirmed both findings were
resolved and found no new required issues.

## Result

**Result:** Pass

Implemented the narrow AppKit key-equivalent forwarding path for browser-owned
navigation shortcuts. `performKeyEquivalent(with:)` now recognizes command-only
`[`, `]`, and `R` before Ghostty binding/menu fallback. It calls the existing
TermSurf browser forwarding path and returns `true` only when that path accepts
the event.

Added `browser-command-navigation` to `scripts/ghostboard-geometry-matrix.sh`.
The scenario reuses the local browser-state fixture and proves:

- a pre-browse `Cmd+[` attempt emits no browser-forwarded AppKit log and no
  Roamium key event;
- browse-mode `Cmd+[` emits `perform_key_equivalent_browser_forwarded`, reaches
  Zig/Roamium as `windows_key_code=219` with modifier `8`, and navigates Back to
  the original URL;
- browse-mode `Cmd+]` emits `perform_key_equivalent_browser_forwarded`, reaches
  Zig/Roamium as `windows_key_code=221` with modifier `8`, and navigates Forward
  to the second URL.

Updated `docs/keybindings.md` to state that browser navigation shortcuts are
forwarded from AppKit key equivalents to Chromium.

Verification:

- `bash -n scripts/ghostboard-geometry-matrix.sh` — Pass.
- `./scripts/build.sh ghostboard` — Pass. The build completed successfully;
  pre-existing Swift/dSYM warnings were emitted outside this change.
- `./scripts/ghostboard-geometry-matrix.sh browser-command-navigation` — Pass.
  Evidence logs:
  - `logs/ghostboard-geometry-browser-command-navigation-app-20260618-064336.log`
  - `logs/ghostboard-geometry-browser-command-navigation-roamium-20260618-064336.log`
  - `logs/ghostboard-geometry-browser-command-navigation-webtui-20260618-064336.log`
- `./scripts/ghostboard-geometry-matrix.sh copy-current-url-smoke` — Pass.
  Evidence logs:
  - `logs/ghostboard-geometry-copy-current-url-smoke-app-20260618-064355.log`
  - `logs/ghostboard-geometry-copy-current-url-smoke-roamium-20260618-064355.log`
  - `logs/ghostboard-geometry-copy-current-url-smoke-webtui-20260618-064355.log`
- `git diff --check` — Pass.

## Conclusion

The missing path was AppKit key-equivalent routing, not Chromium navigation
handling. Direct browser forwarding from `performKeyEquivalent(with:)`, gated by
the existing TermSurf browser state check, restores `Cmd+[` Back and `Cmd+]`
Forward in browse mode while preserving Control-mode `Cmd+C`.

## Completion Review

Reviewed by a fresh-context Codex adversarial subagent.

Initial verdict: **Changes Required**.

- Required: the issue README had already been marked closed while completion
  review was still pending and before the result commit existed. Fixed by
  restoring Issue 822 to `status = "open"`, removing the premature README
  conclusion, regenerating the issue index, and leaving final issue closure for
  after the approved result commit.

Re-review verdict: **Approved**. The reviewer confirmed Issue 822 is open again,
the premature README conclusion is gone, the issue index lists Issue 822 as
open, and the result commit has not yet been made.
