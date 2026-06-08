+++
[implementer]
agent = "claude-code"
model = "claude-opus-4-8"
reasoning = "high"

[review.design]
agent = "claude-code"
model = "claude-opus-4-8"
reasoning = "high"
+++

# Experiment 4: Window-isolated screenshot capture (+ no-screenshots-in-repo policy)

## Description

Experiment 3 proved the real Ghostty app builds and runs, but left one Phase-A
gap: the agent cannot screenshot **just the Ghostty window.** A full-screen
`screencapture` grabs the agent's Wezboard fullscreen Space (not Ghostty's), and
Exp 3's JXA `CGWindowListCopyWindowInfo` call returned no Ghostty window —
almost certainly because the JXA `$.kCGWindowListOptionAll` constant resolved to
`undefined` (not a fundamental limit), so the call got a bad option value.

This experiment delivers a reliable, reusable **window-isolated capture**
primitive — screenshot a named app's window regardless of Space, occlusion, or
which Space the agent's own terminal occupies — which the automated UI harness
(workstream 3) and all visual checks depend on. It also establishes the
project's **screenshots-are-never- committed** policy.

This experiment changes **no roastty source** and **no app source.** It adds
harness tooling under `scripts/ghostty-app/` plus a `.gitignore` rule and policy
text.

## Approach

**Primary — capture by CGWindowID with `screencapture -l<id>`.**
`screencapture -l` captures a single window's backing store **independent of
Space and layering**, which side-steps the Spaces problem entirely.

1. **Window-ID lookup via a small Swift helper.** Swift ships with Xcode (no
   `pyobjc`, no JXA-constant bug):
   `CGWindowListCopyWindowInfo([.optionAll], kCGNullWindowID)`, filter by
   `kCGWindowOwnerName` (or `kCGWindowOwnerPID`), select the frontmost real
   window (window layer `0`, largest bounds), and print `kCGWindowNumber` +
   bounds.
2. **Capture:** `screencapture -x -l<id> -o <out.png>`.
3. **Validate:** the PNG's pixel dimensions ≈ the reported window bounds (i.e. a
   window, not the full display), and it visibly shows the Ghostty terminal.

**Fallback — ScreenCaptureKit** (only if `-l` proves unreliable, e.g. for a
minimized or fully-occluded window): a tiny Swift helper using
`SCShareableContent.current` → match the `SCWindow` by
`owningApplication.bundleIdentifier` / title →
`SCScreenshotManager.captureImage(contentFilter:configuration:)`. Modern, single
call, handles off-screen windows.

**Permissions.** The window-ID lookup needs **no** special permission
(bounds/owner are public; only window _titles_ require Screen Recording).
Capturing the bitmap (`screencapture -l` or ScreenCaptureKit) requires **Screen
Recording** granted to the host terminal (Wezboard) — **already granted** (Exp
3's full-screen capture produced a real image). The exact grant is documented as
the only prerequisite.

## Screenshots policy (established here and in the issue README)

**Screenshots are never committed to the repo.** Enforced in two layers:

1. **Outside the repo by default.** The harness writes to
   `${TERMSURF_SHOT_DIR:-$HOME/.cache/termsurf/shots}` (created on demand) —
   never inside the working tree by default.
2. **`.gitignore` safety net.** A `__screenshots__/` directory name is
   git-ignored anywhere in the repo, so even an explicit in-repo override cannot
   be committed.

**Consequence for "golden baselines":** we do **not** commit reference images.
Visual verification is **live A/B** — capture the real Ghostty app and the
roastty app in the **same run** under identical input, diff them, and record
only the **verdict / diff metric** (not the images). Any retained reference
image lives outside the repo. (This supersedes Exp 2's "commit a small baseline
PNG set" idea.) The **diff metric and its tolerance are defined by the later
A/B-diff experiment**, not here (live windows differ by cursor-blink phase,
timing, subpixel AA); the **pinned Ghostty version** (1.3.2-dev, `2c62d18`) is
what substitutes for a committed historical baseline.

## Changes / Deliverables

**Established now (standing policy, per explicit user direction — not gated on
this design; already in the working tree):**

- `.gitignore` — `__screenshots__/` ignored anywhere in the repo.
- `issues/0802-…/README.md` — the "Screenshots policy" subsection; the empty
  `baseline/` dir removed (no committed baselines).

**Gated experiment implementation (after this design is approved +
plan-committed):**

- `scripts/ghostty-app/screenshot.sh <owner-name|bundle-id|pid> [out-name]` —
  resolves the window ID (inline Swift helper) and captures that window to the
  out-of-repo shots dir; prints the path. `--list` dumps candidate windows (id,
  owner, layer, bounds). **Window selection:** `CGWindowListCopyWindowInfo`
  returns windows front-to-back, so the **first** owner/pid match at
  window-layer `0` is the frontmost real window.
- (fallback, only if `-l` returns stale/black pixels for an off-Space window)
  `scripts/ghostty-app/winshot.swift` — ScreenCaptureKit single-window capture,
  using the `SCShareableContent` variant that **includes off-screen windows**.

Out of scope (a later experiment): **driving** the app — synthetic
keyboard/mouse input injection (Accessibility/`osascript`/XCUITest). This
experiment is capture-only.

## Verification

1. Launch the real Ghostty app (Exp 3's build) while the agent's Wezboard is
   fullscreen on its own Space.
2. `screenshot.sh Ghostty` → a PNG in the out-of-repo dir whose **pixel**
   dimensions ≈ the Ghostty window bounds **× the display backing scale** (e.g.
   763×808 pt → 1526×1616 px on a 2× Retina display) — i.e. a window crop,
   aspect-matching the window and well below the full display size (**not** the
   full display).
3. **Cross-Space case (the real Phase-A risk):** with Ghostty on a **different
   Space** than the agent's fullscreen Wezboard, capture again and record
   whether `-l` returns **live** pixels or a **stale/black** backing store. If
   stale, fall back to the ScreenCaptureKit helper (off-screen-inclusive) and
   record which path produced a live image — that determines whether this is a
   Pass or a Partial.
4. Read the PNG back: it shows the Ghostty terminal (prompt/cursor), **not** the
   desktop or Wezboard.
5. `git status` is **clean** after capture (nothing staged/untracked appears in
   the tree).
6. Record the exact Screen-Recording grant relied upon.

**Pass** = a single Ghostty-window PNG is captured reliably — pixel dims ≈
bounds × backing scale, visible terminal content, **including the cross-Space
case** — written outside the repo, with `git status` clean afterward, via the
primary `screencapture -l` path.

**Partial** = capture works but only via the ScreenCaptureKit fallback, or
requires a one-time documented permission grant that wasn't already in place.

**Fail** = no reliable way to isolate the window from the agent (would push
capture into XCUITest / the app's own test target — a heavier Phase-D path),
documented as such.

## Design Review

**Reviewer:** `adversarial-reviewer` subagent (Claude Opus, fresh context,
read-only + cheap macOS probes). **Verdict: CHANGES REQUIRED → addressed.** The
reviewer **independently verified the core primitive works**:
`screencapture -x -l<id>` produced a window crop of `1526×1616 px` for a window
whose bounds were `763×808 pt` on a 2× Retina display (= bounds × scale, i.e.
the window, not the 5120×2880 display); and a proper Swift
`CGWindowListCopyWindowInfo([.optionAll], kCGNullWindowID)` enumerated 24
layer-0 app windows by owner — confirming the Exp-3 failure was the bad JXA
constant, not a fundamental limit, and that window-ID lookup needs no special
permission.

Findings and fixes:

- **Required — workflow: two deliverables landed before plan-commit.** The
  `.gitignore` rule and the README "Screenshots policy" section were already in
  the working tree before approval. **Resolved by reframing:** these are
  **user-directed standing policy** (the user explicitly asked to "establish a
  policy ... in the issue"), now listed separately from the **gated experiment
  implementation** (the capture harness, which remains unbuilt until this design
  is approved + plan-committed). No experiment _code_ precedes the plan commit;
  the plan commit carries the design + the user-ordered policy.
- **Optional — Retina backing scale.** Fixed: the validation/Pass criterion now
  asserts pixel dims ≈ bounds **× backing scale** (with the 763×808→1526×1616
  example), not literal `dims == bounds`.
- **Optional — cross-Space `-l` unverified** (the actual Phase-A risk; `-l` can
  return a stale/black backing store for an off-Space window, and
  `SCShareableContent.current` may omit off-Space windows). Fixed: Verification
  now explicitly tests the cross-Space case and records live-vs-stale, and the
  SCK fallback is specified to use the off-screen-inclusive `SCShareableContent`
  variant.
- **Optional — A/B diff metric/tolerance undefined; no historical baseline.**
  Fixed: the policy now defers the metric/tolerance to the later A/B-diff
  experiment and names the pinned version as the baseline substitute (both here
  and in the README).
- **Nit — "remove empty `baseline/` dir" referenced a non-existent dir.**
  Corrected: the dir has already been removed; the deliverable now says so.
- **Nit — selection heuristic tiebreak.** Fixed: the harness uses CGWindowList's
  front-to-back order — first owner/pid match at layer `0` is frontmost.

## Result

**Result:** Pass.

Implemented `scripts/ghostty-app/winid.swift` (CGWindowID lookup) and
`scripts/ghostty-app/screenshot.sh` (capture wrapper). Against the running Exp-3
Ghostty app (PID 86636), with the agent's Wezboard **fullscreen on its own
Space**:

1. `screenshot.sh --list Ghostty` correctly enumerated the app's windows
   (`Ghostty[DEBUG]`), identifying the main terminal window `15662` at
   `800×632 pt` (`onscreen=true`) and filtering out the off-screen helper
   windows (`3200×30`, `500×500`).
2. `screenshot.sh Ghostty` captured window `15662` to
   `~/.cache/termsurf/shots/ghostty-launch-….png` at **`1600×1264 px`** —
   exactly `800×632 pt × 2` (Retina backing scale): the **window crop, not** the
   `5120×2880` display.
3. The captured pixels are **live** — the image shows the "running a debug build
   of Ghostty" banner and a working Nushell prompt (`ryan:` + cursor), **not** a
   stale/black backing store and **not** the agent's Wezboard Space. So the
   primary `screencapture -l` path handles the **cross-Space** case directly;
   the ScreenCaptureKit fallback (`winshot.swift`) was **not needed** and was
   not built.
4. The PNG is **outside the repo** (`$TERMSURF_SHOT_DIR` default
   `~/.cache/termsurf/shots`); `git status` shows no screenshot artifact in the
   tree (only the new scripts).

**Permission relied upon:** Screen Recording, already granted to the host
terminal (Wezboard) — no new grant required. The window-ID lookup needed no
permission.

## Conclusion

The Phase-A capture gap from Exp 3 is closed: the agent can reliably screenshot
the Ghostty window alone, across Spaces, via `screencapture -l<id>` + the Swift
window-ID lookup — the Exp-3 failure was indeed the bad JXA constant, not a real
limit. The no-screenshots-in-repo policy is enforced (out-of-repo default +
`__screenshots__/` gitignore) and the harness honors it. The ScreenCaptureKit
fallback remains specified in the design if a future off-Space/occluded case
returns stale pixels, but it was unnecessary here.

**Phase A is complete** (build ✓, run ✓, window-isolated capture ✓). The
remaining pre-baseline step — **input injection** (driving the app: type a
deterministic command, then capture) — is the next experiment, and feeds
directly into Phase B's live-A/B comparison of the real app vs the
`roastty`-backed app.

## Result Review

**Reviewer:** `adversarial-reviewer` subagent (Claude Opus, fresh context,
read-only + live re-run). **Verdict: APPROVED, no Required findings.** The
reviewer independently re-ran `screenshot.sh Ghostty` and reproduced the result
— `1600×1264 px` window crop, live terminal content — and noted the window was
`onscreen=false` at their capture, a **stronger** cross-Space confirmation than
this doc's run. It also verified: output lands outside the repo with
`git status` clean of image artifacts; `__screenshots__/` is ignored anywhere;
the `basename` guard blocks `../`/absolute `out-name` escapes; the no-window
case exits non-zero with a clear message; and the status flips are accurate. One
**Optional** (owner-name substring could match a same-prefix app) is
**intentional** — the debug build's owner is `Ghostty[DEBUG]`, so substring +
pid/bundle disambiguation is correct; documented in `winid.swift`.
