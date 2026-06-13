+++
status = "open"
opened = "2026-06-13"
+++

# Issue 805: Roastty Parity with Ghostty 2c62d182

## Goal

Prove that Roastty is functionally equivalent to Ghostty commit
`2c62d182cec246764ff725096a70b9ef44996f7f` for the copied macOS app, the
embedded C ABI surface, and the terminal/runtime behavior reimplemented in
`libroastty`.

The issue is complete only when every relevant Ghostty feature, configuration
option, app workflow, input path, renderer behavior, terminal behavior, and
integration point is either proven passing in Roastty, explicitly marked not
applicable, or accepted as an intentional documented divergence.

## Background

Roastty is a Rust reimplementation of Ghostty's `libghostty` behavior with a
renamed/adapted macOS GUI on top. Issues 800 through 804 established the
architecture, ported the terminal/runtime surface, copied and renamed the macOS
app, finished GUI automation readiness, and proved keyboard and mouse automation
can drive the current app in this VM.

The upstream reference is fixed:

- Ghostty commit: `2c62d182cec246764ff725096a70b9ef44996f7f`
- Branch: `main`
- `git describe`: `tip-1608-g2c62d182c`
- Date recorded in Issue 802: 2026-05-29
- `build.zig.zon`: `version = "1.3.2-dev"`
- Zig version: `0.15.2`

Issue 802 records this exact commit as the vendored Ghostty pin and states that
the copied app and embedded ABI must match it. This issue turns that pin into a
full parity certification gate.

## Scope

Parity means user-visible and integration-visible behavior, not byte-for-byte
implementation identity. Roastty may differ internally when the Rust
implementation produces equivalent behavior.

Required audit domains:

- source-code parity against `vendor/ghostty/`;
- macOS app behavior and workflows;
- embedded C ABI shape and semantics;
- terminal parser/state behavior;
- PTY and process lifecycle behavior;
- renderer/display behavior;
- font discovery, shaping, glyph fallback, and metrics behavior;
- keyboard, mouse, dead-key, IME/preedit, and shortcut handling;
- selection, clipboard, links, menus, tabs, splits, windows, and command palette
  behavior;
- shell integration and terminfo behavior;
- configuration parsing, defaults, formatting, diagnostics, precedence, reload,
  and runtime effects;
- tests, fixtures, and app walkthrough evidence.

Allowed outcomes for each audited item:

- **Pass**: Roastty behavior is proven equivalent with tests, logs, screenshots,
  or another deterministic oracle.
- **Gap**: Roastty behavior diverges and must be fixed before closing this
  issue.
- **Intentional divergence**: Roastty deliberately differs; the difference,
  reason, user impact, and acceptance decision are documented.
- **Not applicable**: The Ghostty behavior does not apply to Roastty, with a
  concrete reason.

## Required Artifacts

This issue should maintain durable parity matrices as experiments progress:

- `feature-matrix.md` — upstream feature/workflow inventory and Roastty status.
- `config-matrix.md` — every relevant Ghostty config option, default, parser,
  formatter, diagnostic, precedence rule, and runtime behavior.
- `source-audit.md` — source subsystem audit findings and fixes.
- `walkthrough-matrix.md` — app walkthrough scenarios, automation commands,
  oracles, and results.
- `divergences.md` — accepted intentional divergences and not-applicable items.

Do not treat the matrices as optional notes. They are the proof surface for
closing the issue.

Each matrix row should record its regression guard:

- guard tier;
- guard command or checklist path;
- when it should run;
- why that tier is sufficient.

Passing behavior without a durable guard remains provisional and does not count
toward final parity certification unless it is explicitly accepted as a manual
walkthrough item.

## Regression Guard Policy

Every passing experiment result should leave behind the cheapest sufficient
guard that would catch a later regression. Do not default to slow GUI tests when
a static check, unit test, or focused integration test proves the behavior.

Guard tiers:

- **Tier 0: Static or matrix check.** Fastest. Use for inventories, ABI symbol
  drift, forbidden-name scans, generated table completeness, config coverage,
  and documentation/matrix consistency.
- **Tier 1: Unit test.** Fast. Use for parsers, config defaults, formatters,
  terminal state transitions, key encoding, selection math, renderer data
  structures, and pure helper behavior.
- **Tier 2: Focused integration test.** Medium. Use for PTY worker behavior,
  config load/reload, shell integration, clipboard helpers, process lifecycle,
  and non-GUI end-to-end paths.
- **Tier 3: GUI smoke test.** Slower. Use when the real macOS app surface is the
  behavior under test, such as launch, keyboard delivery, mouse
  click/drag/scroll, screenshots, menu actions, or accessibility state.
- **Tier 4: Full A/B parity walkthrough.** Slowest. Use for milestone checks,
  nightly/manual certification, or broad Ghostty-versus-Roastty workflow
  comparisons. Do not require this tier for every code change.

Rules:

- Add the cheapest guard that would have caught the regression.
- Slow GUI or full A/B guards require a short justification in the relevant
  matrix row or experiment result.
- Most config behavior should be guarded by static matrix checks and
  parser/default/formatter/runtime unit tests. Only representative runtime
  config behavior should require GUI proof.
- GUI tests should be small, stable, and high-signal. Prefer one representative
  smoke per app/input/rendering surface over exhaustive GUI permutations.
- If a behavior cannot yet be automated, record it as a manual walkthrough row
  with the reason and revisit automation before final certification.

## Proposed Stages

Experiments must still be created one at a time. The stages below define the
intended coverage, not a precommitted experiment list.

### Stage 1: Pin, Scope, and Inventory Schema

Create the parity matrices and define the row schema. Every row should include:

- upstream behavior or source path;
- Roastty implementation path;
- verification method;
- current status;
- evidence artifact;
- owner experiment.

### Stage 2: Source Code Audit

Audit the Roastty implementation against the pinned Ghostty source tree. Look
for bugs, missing behavior, stale assumptions, renamed-app mistakes, and
semantic mismatches. Fix gaps discovered during the audit when they are clearly
in scope for parity.

This stage should cover at least:

- `include/ghostty.h` versus `roastty/include/roastty.h`;
- `src/App.zig`, `src/Surface.zig`, and `src/apprt/embedded.zig` behavior;
- terminal, termio, renderer, font, input, config, shell-integration, and
  macOS-facing bridge behavior;
- Swift app code copied from `vendor/ghostty/macos/Sources`.

### Stage 3: Configuration Parity

Inventory every relevant option in Ghostty's pinned `Config.zig` and related
config helper files. For each option, prove or fix:

- default value;
- parser behavior;
- formatter behavior and order where relevant;
- validation/finalization behavior;
- file, CLI, environment, and runtime precedence;
- diagnostics for invalid values;
- runtime effect in the app when applicable.

Config options that are platform-specific, GTK-only, Linux-only, or otherwise
not applicable to Roastty must still be listed with a reason.

### Stage 4: Upstream Tests and Fixture Port

Identify Ghostty tests, fixtures, generated tables, and behavior examples that
apply to Roastty. Port them directly where practical. Where direct porting is
not practical, create equivalent Roastty tests or document why the upstream test
does not apply.

### Stage 5: App Walkthrough

Build both the pinned Ghostty app and current Roastty app. Walk through the real
macOS app behavior and prove workflows with automation whenever possible.

The walkthrough must cover at least:

- launch, quit, reopen, and cleanup;
- new window, tab, split, focus movement, resize, zoom, and fullscreen;
- text entry, shortcuts, dead keys, IME/preedit, and keybindings;
- mouse click, drag selection, scrollback, shift-click/drag where applicable,
  and mouse reporting;
- copy, paste, clipboard formats, bracketed paste, and selection behavior;
- links, URL opening, hover/cursor behavior, and context menus;
- menus, command palette, titlebar, quick terminal, notifications, bell, and app
  lifecycle behavior;
- font/theme changes, renderer options, window padding, opacity, cursor style,
  and other visible config-driven behavior;
- shell integration, terminfo, title reporting, working directory reporting, and
  process lifecycle.

### Stage 6: A/B Behavioral Matrix

For workflows where visual or terminal behavior matters, run Ghostty and Roastty
with the same recipe and compare deterministic artifacts:

- terminal output;
- screenshots or cropped screenshots;
- pasteboard contents;
- accessibility state;
- process/window state;
- config output;
- logs or trace files.

### Stage 7: Divergence Review

Record every accepted difference in `divergences.md`. Each entry must include:

- upstream Ghostty behavior;
- Roastty behavior;
- why the divergence exists;
- user-visible impact;
- explicit acceptance rationale.

Unaccepted divergences remain gaps and block closure.

### Stage 8: Final Parity Certification

The issue can close only when:

- every matrix row is `Pass`, `Intentional divergence`, or `Not applicable`;
- there are zero unresolved `Gap` rows;
- required automated tests pass;
- the app walkthrough passes;
- source-code audit findings are fixed or accepted as divergences;
- config parity is complete;
- the conclusion names the exact Ghostty commit hash and summarizes the final
  evidence.

## Learnings

Record concrete, reusable findings here as experiments discover how to prove or
fix Roastty parity with Ghostty. Update this section whenever an experiment
learns something that may be useful to future experiments. Keep hypotheses in
experiment files until they are proven.

- **The upstream parity target is fixed.** All source, config, ABI, and app
  behavior comparisons in this issue target Ghostty commit
  `2c62d182cec246764ff725096a70b9ef44996f7f`.
- **Use the resolved Ghostty app build wrapper.** The pinned Ghostty app should
  be built with `scripts/ghostty-app/build-macos-app.sh Debug`, not by
  hand-running `build.zig` from `vendor/ghostty/macos`.
- **Use the current Roastty app bundle.** `scripts/roastty-app/start-app.sh` now
  honors `ROASTTY_APP` and otherwise prefers the newer debug app bundle when
  both Roastty debug layouts exist.
- **The VM can synthesize keyboard and mouse input.** Issue 804 proved System
  Events keyboard input, CGEvent mouse click/drag/scroll, full-window
  screenshots, and cleanup against the current Roastty app after the required
  permissions were granted to Ghostty, the responsible host app for this Codex
  session.
- **Passing behavior needs a durable but cheap guard.** Future experiments
  should record the cheapest sufficient regression guard for each passing parity
  row. Prefer static checks and unit tests when they prove the behavior; reserve
  GUI and full A/B tests for behavior that genuinely requires the real app
  surface.

## Verification

This issue is complete when the matrices prove total accepted parity with
Ghostty commit `2c62d182cec246764ff725096a70b9ef44996f7f`.

The final conclusion must not claim parity from incomplete evidence. If a row
has no deterministic proof, it is not passing. If the behavior is not tested and
not explicitly accepted as a divergence or not-applicable item, the issue
remains open.

## Experiments

- [Experiment 1: Pinned A/B baseline](01-pinned-ab-baseline.md) — **Designed**
