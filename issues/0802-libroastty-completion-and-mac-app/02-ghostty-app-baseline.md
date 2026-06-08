+++
[implementer]
agent = "claude-code"
model = "claude-opus-4-8"
reasoning = "high"

[review.design]
agent = "claude-code"
model = "claude-opus-4-8"
reasoning = "high"

[review.result]
agent = "claude-code"
model = "claude-opus-4-8"
reasoning = "high"
+++

# Experiment 2: Baseline & feasibility — build, run, and automate the real Ghostty app

## Description

Before porting anything, de-risk the entire conformance strategy by proving — on
the **real, unmodified** Ghostty macOS app (vendored at `vendor/ghostty/macos`,
version **1.3.2-dev**) — that in _this_ environment we can:

1. **build** the app from source,
2. **run** it (signing/permissions),
3. **automate** it programmatically (drive input + capture screenshots), and
4. capture a **golden baseline** (reference screenshots + behavior) that the
   `roastty`-backed app will later be diff-tested against.

The whole Issue-802 plan is a bet that the app can be built, run, and
UI-automated; this experiment settles that bet cheaply against a known-good
binary, and produces a **reusable build + run + automate harness** that Phase D
(UI tests for the roastty app) inherits. A negative result here is just as
valuable — it tells us to adjust the approach (or what permission you must
grant) before sinking work into the port.

This experiment changes **no roastty source.** It builds vendored Ghostty and
produces a harness + baseline artifacts + a documented findings record.

## Environment (already confirmed)

- App project present: `vendor/ghostty/macos/Ghostty.xcodeproj` (+ entitlements,
  `Ghostty-Info.plist`, `Ghostty.sdef` AppleScript dictionary,
  `Ghostty.xctestplan`).
- Toolchain present: `zig` (0.16.0 installed), `xcodebuild`, `osascript`,
  `screencapture`, Xcode at `/Applications/Xcode.app`.
- **Real GUI session** (not headless SSH): `SSH_TTY` unset,
  `TERM_PROGRAM=Wezboard` — so there is a window server to drive.

## Known risks to resolve (the point of the spike)

- **Zig version.** `minimum_zig_version = 0.15.2` is a **floor, not a pin**, and
  the installed zig is `0.16.0`. Zig has breaking changes between minors, so
  0.16.0 may not build a 0.15.x-era dev tag — but since 0.15.2 is only a
  minimum, the spike should **determine and record the exact zig that builds
  1.3.2-dev**: try the installed 0.16.0 once, and if it fails install/select a
  working 0.15.x (pinned download / `zvm` / `asdf`) and pin the precise version.
  **The required zig version is a hard input — do not "upgrade" ghostty to fit a
  newer zig.**
- **Network / dependency fetch.** `build.zig.zon` pulls many dependencies from
  `deps.files.ghostty.org` (some non-lazy), so the first `zig build` needs
  network egress to populate the zig cache. A fetch failure must be triaged as a
  _network_ blocker, not a toolchain one.
- **Build flow.** Per `vendor/ghostty/macos/AGENTS.md`: do **not** `zig build`
  the app directly — run `zig build -Demit-macos-app=false` to produce
  `GhosttyKit.xcframework` (which the Xcode project consumes; the project runs
  no zig build phase), then `macos/build.nu` to build the `.app`. Follow that
  doc as the authoritative build guide. (`nu` 0.113.0 is already installed.)
- **Automation permissions.** Driving + screenshotting a GUI app from the
  agent's shell inherits the controlling terminal's (Wezboard's) TCC grants. It
  will likely require a **one-time manual grant** of **Accessibility** and
  **Screen Recording** (and possibly Automation/AppleEvents) to Wezboard in
  System Settings. If so, document the exact grant needed as the remediation —
  that is a successful finding, not a failure.
- **Signing.** Build a **local/debug** configuration
  (`GhosttyDebug.entitlements` / `GhosttyReleaseLocal.entitlements`) —
  ad-hoc/local signing, no distribution cert.
- **Build duration.** A clean zig + Xcode build may exceed the 15-min
  bounded-run cap. Builds run as **tracked background tasks** (Central-time
  stamped) with a generous timeout, since they are one-off builds, not flaky
  test loops; only the short automation steps use the bounded runner if at all.

## Changes / Deliverables

No roastty code changes. The experiment produces:

- **A reusable harness** under `scripts/ghostty-app/` (or similar), with small,
  documented steps:
  - `build.sh` — select zig 0.15.x, build GhosttyKit
    (`zig build -Demit-macos-app=false`), then `macos/build.nu`; emits the
    `.app` path.
  - `run.sh` — launch the built `.app` (and quit it cleanly).
  - `automate.sh` — drive the app (send keystrokes via `osascript`/AppleEvents
    or Accessibility; type a deterministic command), and `screencapture` the
    window.
  - `screenshot.sh` — capture a named PNG of the app's window. (Exact tool
    choice — `osascript` System Events vs an XCUITest target vs `cliclick` — is
    decided during implementation based on what actually works; the harness
    wraps whatever does.)
- **Golden baseline artifacts** under
  `issues/0802-libroastty-completion-and-mac-app/baseline/` — a small set of
  reference PNGs: (a) a fresh window, (b) after typing a deterministic command
  (e.g. a fixed `printf` of ASCII + a color SGR line), (c) a basic Unicode/emoji
  line. Committed as the reference set (kept small).
- **A documented findings record** (this experiment's Result + a short
  `scripts/ghostty-app/README.md`): the exact zig version used, the build
  incantation that worked, the launch steps, the automation mechanism, and **the
  precise permissions that had to be granted** (with the System Settings path).

## Verification

Per the bounded-run convention for any test-suite-like steps (Central-stamped);
builds as tracked background tasks. Steps:

1. Read `vendor/ghostty/macos/AGENTS.md` (+ ghostty build docs) and follow the
   documented macOS build flow.
2. Obtain/select zig **0.15.x**; obtain `nu` if missing.
3. Build GhosttyKit + the `.app`; record the working commands.
4. Launch the `.app`; confirm it shows a working terminal (capture a
   screenshot).
5. Programmatically send a deterministic input and capture the resulting window
   — confirming automation works (or document the exact permission grant
   required).
6. Save the baseline PNGs and write the harness `README` + findings.

**Pass** = the real Ghostty 1.3.2-dev app **builds, launches, shows a working
terminal in a captured screenshot, and is driven + screenshotted
programmatically from this environment**, with the harness, baseline artifacts,
and required permissions all documented.

**Partial** = builds + launches + screenshots, but full input-automation is
blocked on a permission/tooling limitation that is documented with the exact
remediation (e.g. "grant Wezboard Accessibility + Screen Recording, then
re-run") — still a go decision for the approach.

**Fail** = cannot build or cannot run the real app (a toolchain/version blocker
that no reasonable step resolves) — a genuine finding that forces a plan change
before the port proceeds; document the blocker precisely.

**Scope caveat:** this spike proves automation in an **interactive GUI session
with TCC grants** (the agent in Wezboard). It does **not** by itself establish
_headless / CI_ automation (Issue-802 risk (c)). Phase D should therefore treat
"repeatable in this session" as the bar it inherits, and treat headless/CI runs
as a separate, later concern rather than an assumption.

## Design Review

**Reviewer:** `adversarial-reviewer` subagent (Claude Opus, fresh context,
read-only). **Verdict: APPROVED, no Required findings.** Independently verified
the load-bearing facts: `build.zig.zon` version `1.3.2-dev` /
`minimum_zig_version 0.15.2` vs installed zig `0.16.0`; the build flow matches
`macos/AGENTS.md` exactly (no direct `zig build` of the app;
`-Demit-macos-app=false` then `build.nu`; the xcodeproj consumes a prebuilt
`GhosttyKit.xcframework` and runs no zig phase); `nu` 0.113.0 / `osascript` /
`screencapture` / Xcode 26.4 present; GUI session confirmed; `build.nu` skips
`GhosttyUITests` "because it requires special permissions" (corroborating the
TCC caution); `vendor/ghostty` is git-ignored so the tracked harness/baseline
locations are correct. Findings adopted:

- **Optional — network/dep-fetch risk.** **Added:** the first `zig build`
  fetches many deps from `deps.files.ghostty.org`; a fetch failure is a network,
  not toolchain, blocker.
- **Optional — "0.15.x" was imprecise (floor, not pin).** **Fixed:** the spike
  now determines and records the _exact_ zig that builds 1.3.2-dev (try 0.16.0
  once, else a working 0.15.x).
- **Optional — headless/CI over-promise.** **Fixed:** added the scope caveat
  above; Phase D's wording softened from "CI-able" to repeatable-in-session.
- **Nits — `nu` already installed; `cliclick` absent (harness falls back to
  `osascript`/XCUITest).** Noted.

## Result

**Result:** Partial — the conformance approach is feasible and the zig toolchain
is resolved up to the SDK-link boundary, but a **real, precisely-diagnosed
toolchain blocker** prevents building the app on this machine without a
decision. (Per the design, this is the valuable "found the blocker on day one,
cheaply, before any porting" outcome.)

### What works (confirmed)

- **zig 0.16.0 (installed) cannot build ghostty 1.3.2-dev** — `build.zig` uses a
  0.15.x `readFileAlloc` signature and hard-requires
  `minimum_zig_version 0.15.2`.
- **zig 0.15.2** pinned (downloaded to
  `vendor/toolchains/zig-aarch64-macos-0.15.2/`, gitignored) builds `build.zig`.
  Under **`DEVELOPER_DIR=/Library/Developer/CommandLineTools`** it compiles the
  entire ghostty Zig codebase cleanly (built `libghostty-vt`, 0 errors, ~18 s).
- Network/dep fetch works; `nu` 0.113.0 / `osascript` / `screencapture` / Xcode
  26.4 present; real GUI session confirmed.

### The blocker (precisely characterized)

- **zig 0.15.2 cannot link against Xcode 26.4's macOS SDK** —
  `undefined symbol: __availability_version_check`. A sweep of every SDK on the
  machine: **all Xcode-26.4 SDKs FAIL; all CommandLineTools SDKs PASS** a
  `zig build-exe -lc` link. (The CLT SDK is the fix for the macOS link.)
- **But `GhosttyKit.xcframework` also builds an iOS slice**, and
  **CommandLineTools has no iOS SDK** → `DarwinSdkNotFound`; Xcode's iOS SDK is
  also 26.4 (unlinkable by zig 0.15.2). Only **Xcode 26.4** is installed (no
  older Xcode).
- **Net:** no toolchain combination on this machine builds the full
  **macOS+iOS** xcframework with the zig version ghostty pins. Building only the
  macOS **GUI** lib (`GhosttyKit`) is not a simple flag — it is entangled with
  the iOS xcframework path (lib-only flags build `libghostty-vt`, the standalone
  VT library, not `GhosttyKit`), so isolating it requires **patching ghostty's
  `build.zig`** (a deviation).

### Remediations (the decision point)

> **Correction (post-review):** an earlier draft of this section recommended
> "install Xcode 16.x." **That was wrong** — the official docs _require_ **Xcode
> 26** (which this machine has at 26.4), and zig can't be bumped to fit
> (Ghostty's `requireZig` enforces an exact major.minor and even Ghostty `main`
> still pins 0.15.2). The real gap is the **too-new SDK point release** (26.4),
> not the Xcode major version. The chosen, working remediation is **(A) below**,
> implemented and proven in [Experiment 3](03-macos-only-build.md).

- **(A, chosen — see Exp 3) macOS-only build, no new install** — a minimal
  **build-only** patch to Ghostty's `build.zig` to gate out the iOS xcframework
  slice, building the macOS `GhosttyKit` under the **CommandLineTools 26.0** SDK
  (which zig 0.15.2 _can_ link), then packaging + the Swift app under Xcode
  26.4. The app's app-link resolves `__availability_version_check` against
  libSystem 26.4. **Confirmed working in Exp 3** — the app builds and runs.
- **(B) An earlier Xcode 26 point release** (26.0–26.2) alongside 26.4, whose
  SDK zig 0.15.2 links — keeps the full unmodified build with no `build.zig`
  patch, but is a download.
- **(C) Pin a newer Ghostty** whose required zig supports SDK 26.4 — but that
  changes the conformance target version away from what we vendored (and no such
  version exists yet: Ghostty `main` is still on 0.15.2).

### Deliverables produced

- `vendor/toolchains/zig-aarch64-macos-0.15.2/` — the pinned zig (gitignored).
- `scripts/ghostty-app/setup-zig.sh` — re-fetches/pins zig 0.15.2.
- `scripts/ghostty-app/README.md` — the toolchain resolution + the blocker + the
  build commands that work (and the one that's blocked), as the seed for the
  harness.

(The build/run/automate harness and the golden baseline are **deferred** until
the SDK decision above unblocks an actual `.app` — they need a built app to
capture.)

## Conclusion

The spike did exactly its job: **the conformance strategy is sound and the zig
toolchain is resolved, but the only-installed Xcode (26.4) is too new for the
zig version Ghostty 1.3.2-dev pins (0.15.2)**, and the full xcframework's iOS
slice can't be built without an iOS SDK that zig 0.15.2 can also link. This is
an environment/SDK decision, not a flaw in the plan — and finding it now, before
porting, is the point.

**Resolution:** [Experiment 3](03-macos-only-build.md) implemented remediation
**(A)** — the macOS-only build under CommandLineTools — and the real Ghostty app
now **builds and runs** on this machine with no Xcode change. (My earlier
"install Xcode 16" recommendation was wrong and is corrected above.) Phase A's
build/run is unblocked; the remaining Phase-A work is agent-side window-isolated
screenshot capture + the golden baseline.
