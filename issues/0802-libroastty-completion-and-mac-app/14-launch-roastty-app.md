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

# Experiment 14: Phase C — launch Roastty.app and capture what it renders

## Description

Phase B is done: `Roastty.app` (the copied-and-renamed Ghostty app) **compiles +
links** against `libroastty`. Phase C is the runtime proof — and its first step
is the simplest possible: **run the app and look at it.** Building ≠ running;
launching exercises `roastty_app_new`, `roastty_surface_new`, the runtime-config
callbacks, and the live `surface_draw` path (the NSView render the library must
drive — the "crux" deferred from 801) for the first time.

The expected outcome is **unknown and that's the point** — it may (a) crash on
app/surface init, (b) launch but render a blank/garbage surface (the live
`surface_draw` path likely isn't wired), or (c) actually render. Whichever it
is, this experiment **characterizes the live state and pins the first Phase-C
work item.**

## Approach

1. **Add
   `scripts/roastty-app/{start-app.sh, stop-app.sh, screenshot.sh, winid.swift}`**
   — adapted from the proven `scripts/ghostty-app/` versions (Exp 4/5),
   retargeted to `roastty/macos/build/Debug/Roastty.app` and the
   `…/Contents/MacOS/roastty` binary. `stop-app.sh` kills by PID **scoped to the
   `roastty/macos/build/.*Roastty.app` path** with SIGKILL (no quit dialog) —
   never an installed app, never anything else.
2. **Launch** `Roastty.app`, wait for its window (or detect a crash via the
   absence of a PID + a Crashpad/`log` entry).
3. **Capture** a window-isolated screenshot (`screencapture -l<id>` via
   `winid.swift`, resolved by the **PID** `start-app.sh` prints — not owner-name
   matching, which is brittle across child shells) to the **out-of-repo** shot
   dir (`$TERMSURF_SHOT_DIR` / `~/.cache/termsurf/shots`) — never committed, per
   the screenshots policy. Capture the app's stdout/stderr + any crash log to
   the **same out-of-repo dir** (never a raw log into the repo tree); only short
   excerpts are quoted into the Result.
4. **Characterize** the state: launched-and-rendered / launched-blank /
   crashed-at-`<symbol>`. Read the captured logs to find the first failing call
   (e.g. a panic in `roastty_app_new`, a missing `surface_draw`, an assertion).
   **Cross-check a "blank" capture against the logs** before pinning a work
   item: `screencapture -l` of a window needs a Screen-Recording (TCC) grant,
   and a denied grant yields a black image at the right dimensions that looks
   identical to a blank render — so "blank" is only credible if the logs show a
   running surface with no draw, not a capture-permission failure. (Relies on
   the Exp 4/5 TCC grant persisting.)
5. **Kill the spawned app** with `stop-app.sh` (mandatory — leave nothing on the
   user's screen), and verify no `Roastty.app` debug PID remains.
6. **Record** the finding + the precise first work item for Exp 15.

This is a **diagnostic** experiment: it changes no `libroastty` code. It only
adds the launch/capture scripts and records what the built app does.

## Verification

1. The launch/stop/screenshot scripts exist and run; `start-app.sh` prints a PID
   or reports a crash; `stop-app.sh` leaves **zero** `Roastty.app` debug PIDs
   (verified with `pgrep`).
2. A screenshot is captured **out-of-repo** (path printed; nothing added to
   git); the app's stdout/stderr + any crash log are captured.
3. The live state is **characterized** (rendered / blank / crashed-where) with
   the first Phase-C work item named and evidence (log/crash excerpt) quoted in
   the Result.
4. **No process is left running** (the kill-what-you-spawn rule).

**Pass** = the app was launched, a window-isolated screenshot + logs were
captured out-of-repo, the live state is characterized with the first work item
identified, and the spawned app was killed (no dangling PIDs).

**Partial** = launched + killed cleanly, but the state couldn't be fully
characterized (e.g. capture tooling gap) — documented with the gap.

**Fail** = the app can't be launched/observed at all from this harness
(documented as a tooling blocker, with the spawned process still cleaned up).

## Design Review

**Reviewer:** `adversarial-reviewer` subagent (Claude Opus, fresh context,
read-only). **Verdict: APPROVED** (no Required findings). It verified the
**kill-scoping is safe**: the binary is literally
`…/roastty/macos/build/Debug/Roastty.app/Contents/MacOS/roastty`, the SIGKILL
pattern matches only it, `pgrep -f` on it matches nothing else live (not cargo,
rust-analyzer, the shell, or `target/` rustc), there is **no installed Roastty**
anywhere (`/Applications`, Homebrew, `/usr/local`) so `open` + kill can't touch
a stable app — strictly safer than the ghostty case; mirrors the proven
`stop-app.sh` (PID-scoped SIGKILL, no quit dialog, no broad `pkill`/`killall`).
Screenshot policy honored (out-of-repo, `__screenshots__/` gitignored, nothing
staged); scope honest (no libroastty change); start→capture→stop is one flow.
Three non-blocking findings, folded in: pin the log capture out-of-repo (not
just the PNG); cross-check a "blank" capture against logs (a TCC-denied capture
is black at the right dims and mustn't be mischaracterized as a blank render);
resolve the window by the printed **PID** (not the brittle `Ghostty[DEBUG]`
owner-name match).

## Result

**Result:** Pass — the built `Roastty.app` was launched, captured out-of-repo,
characterized, and cleanly killed (0 dangling PIDs). The live state is
**launched-blank**, and the first Phase-C work item is pinned precisely: the
live `surface_draw` present path is unwired (the 801 "crux").

### What happened

- **Launches cleanly — no crash, no panic.** Running the binary directly with
  `RUST_BACKTRACE=1`, the process stayed alive past 4s; `stderr`/`stdout` were
  **empty** (no panic, no error). The whole newly-reconciled embedded ABI
  (`roastty_app_new`, `roastty_surface_new`, the runtime-config callbacks)
  initializes without faulting — a strong result on its own.
- **A window appears but renders blank.** A window-isolated capture (by PID)
  produced a **500×500pt / 1000×1000px white** image. Per the design's
  cross-check, **white (not black) means the capture succeeded**
  (Screen-Recording/TCC grant intact) and the window is genuinely blank — not a
  permission failure.
- **Process hygiene:** the trap + `stop-app.sh` killed the spawned app;
  `pgrep -f 'roastty/macos/build/.*Roastty.app/Contents/MacOS/roastty'` → **0**.
  The screenshot
  - logs are out-of-repo (`~/.cache/termsurf/shots/`); nothing staged.

### Root cause (the first Phase-C work item)

`roastty_surface_draw(surface)` → `Surface::draw()` → `request_render()`, which
only sets `self.dirty = true` and calls `wakeup_app()`. It renders **nothing**.
The surface config carries `platform.macos.nsview`, but the `Surface` struct
**never stores it**, never creates a `CAMetalLayer`/`CALayer` on it, and never
presents a rendered frame. libroastty's renderer (the offscreen/Metal pipeline
composed in 801) has **no bridge to the app's live NSView** — the exact "live
`surface_draw` into the app-provided NSView" wiring that 801 deferred.

(Tooling added:
`scripts/roastty-app/{start-app.sh, stop-app.sh, screenshot.sh, winid.swift}` —
launch/capture/kill by PID, screenshots out-of-repo. No `libroastty` code
changed.)

## Conclusion

**The app runs.** That the entire freshly-reconciled embedded ABI initializes a
window without a single crash or panic is a strong validation of Exp 6–13. The
one thing missing is the live render present path — the renderer produces frames
offscreen but nothing puts them on the app's NSView.

**Next (Exp 15) — the crux:** wire the live present path. Capture the `nsview`
on the `Surface` (from `surface_config.platform.macos.nsview`), stand up a
Metal-backed layer on it (the app's `CAMetalLayer` / a `CALayerHost`, matching
how ghostty's `surface_draw` presents), and make `surface_draw` drive the
existing renderer pipeline to **present a frame into that layer**. Then
re-launch and confirm the terminal content (the shell prompt) actually appears.
This is the single largest Phase-C item; it likely splits into sub-steps (layer
creation → present an offscreen frame → drive from the surface's content).

## Result Review

**Reviewer:** `adversarial-reviewer` subagent (Claude Opus, fresh context,
read-only). **Verdict: APPROVED.** It verified the root cause **from source**
(not inferred from the screenshot): `draw()`→`request_render()` only sets
`dirty`+wakeup (renders nothing); the `Surface` struct has no
`nsview`/`CAMetalLayer`/`CALayer`/metal field; `roastty_surface_new` reads the
config but never touches `platform.macos.nsview` (defaulted null, dropped); and
upstream `apprt/embedded.zig` _does_ store `Platform.macos.nsview` and present
the Metal renderer into a layer on it — so roastty is missing exactly that
bridge. "801 crux / live present path unwired" is correct and precise. Process
hygiene safe (SIGKILL scoped to the build path; no quit dialog / broad pkill);
screenshot + logs genuinely out-of-repo (nothing staged — only the 4 scripts + 2
docs); "Pass" honest (it read the PNG — genuinely blank white; a prior real
`ghostty-launch` capture in the same dir proves the TCC grant is live, so
white≠black is sound). One Nit (the log-capture run vs `start-app.sh`'s `open`)
— folded in. Exp-15 direction (capture nsview → Metal layer → present) confirmed
correct.
