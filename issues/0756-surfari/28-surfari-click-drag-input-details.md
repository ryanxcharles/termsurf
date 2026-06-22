# Experiment 28: Prove Surfari click and drag input details

## Description

Experiments 21-23 proved single-pane real-app keyboard input and page-visible
wheel input. Experiment 27 proved keyboard routing remains correct across tabs
and windows. The remaining input-detail matrix gaps are narrower:

- `Click` is still `Partial` because Surfari receives mouse events, but the
  fixture has not proven a DOM `click`.
- `Drag` is still `Missing` because no real-app Surfari drag evidence exists.

This experiment should focus only on DOM click and drag behavior in the real
app. It should not expand into profile isolation, crash handling, or the final
Ghostboard/Roamium comparison. It should preserve the already-proven keyboard
and wheel path while adding stronger pointer-detail evidence.

Use the Roamium `browser-input-granularity` scenario in
`scripts/ghostboard-geometry-matrix.sh` as the reference for expected behavior:
single click, double click, modifier click, triple click, drag selection, and
selection copy. For Surfari, keep the first pass narrower if needed: single DOM
click and one page-visible drag selection are sufficient to move the matrix rows
forward. Broader click-count parity can follow only if the basic click and drag
path is already stable.

## Changes

- Add or extend a focused Surfari real-app input-details harness under
  `scripts/`.
- Reuse the existing real Debug `TermSurf.app` launch pattern with repo-built
  `web --browser surfari` and repo-built `surfari`.
- Build a deterministic fixture page with:
  - a text input to preserve keyboard proof;
  - a click zone that logs DOM `click` events through the WebTUI state trace;
  - a selectable text field or equivalent target that logs drag selection;
  - an optional copy assertion proving the browser selection, not terminal
    selection, owns Browse-mode `cmd+c`.
- First run the harness against current Surfari behavior and record whether DOM
  click and drag fail or pass before code changes.
- If DOM click or drag fails, localize the failure to the narrowest boundary:
  macOS event injection, Ghostboard hit testing, TermSurf IPC forwarding,
  Surfari Rust dispatch, or `libtermsurf_webkit` AppKit/WebKit event delivery.
- Fix only the boundary required for DOM click and drag. Do not modify
  `webkit/src` unless evidence proves the failure is inside WebKit internals and
  a TermSurf-side fix cannot satisfy the experiment.
- Preserve the previously proven keyboard and wheel evidence. A fix for click or
  drag must not regress those paths.
- Update `issues/0756-surfari/real-app-matrix.md` only for directly proven rows:
  - mark `Click` `Proven` only if a DOM click event is observed in the real-app
    Surfari fixture;
  - mark `Drag` `Proven` only if drag produces page-visible behavior, such as
    selected browser text and/or a browser-owned copy result.

## Verification

Pass criteria:

- Build or confirm required artifacts:

```bash
surfari/libtermsurf_webkit/build.sh
cargo build -p surfari
cargo build -p webtui
cd ghostboard && zig build
```

- Run the new or extended Surfari input-details harness.
- The harness must prove, in the real app:
  - keyboard input still reaches the fixture page;
  - wheel input still reaches the fixture page;
  - Surfari receives click/drag mouse events;
  - the fixture observes at least one DOM `click`;
  - the fixture observes page-visible drag behavior, preferably selected browser
    text and a browser-owned copy result.
- The harness must fail if DOM click evidence is missing.
- The harness must fail if drag evidence is missing.
- If DOM click or drag cannot be fixed in this experiment, record `Partial` or
  `Fail` with the exact failing boundary and the next proposed fix; do not mark
  the relevant matrix row `Proven`.
- Run hygiene checks:

```bash
git diff --check
bash -n <new-or-updated-surfari-input-details-harness>
prettier --check --prose-wrap always --print-width 80 \
  issues/0756-surfari/README.md \
  issues/0756-surfari/28-surfari-click-drag-input-details.md \
  issues/0756-surfari/real-app-matrix.md
```

Run formatting/checks for any source files touched:

```bash
cargo fmt -- <rust-files>
zig fmt <zig-files>
```

Result classification:

- `Pass` means DOM click and page-visible drag behavior are both proven in the
  real app without regressing the proven keyboard and wheel paths, allowing the
  `Click` and `Drag` matrix rows to become `Proven`.
- `Partial` means either click or drag is proven, or the remaining failing
  boundary is narrowed, but both rows cannot become `Proven`.
- `Fail` means the harness cannot reach the real Surfari overlay or cannot
  produce stronger click/drag evidence than the existing matrix.

## Design Review

Adversarial design review returned `APPROVED` with no Required findings. The
reviewer confirmed that the README links Experiment 28 as `Designed`, the design
has the required Description, Changes, and Verification sections, the scope
follows the current `Click` and `Drag` matrix gaps, the experiment avoids
profile isolation, crash handling, and the final comparison, the verification
requires real-app DOM click and drag evidence and fails if either is missing,
matrix updates are guarded against overclaiming, hygiene/build checks are
present, and the plan had not already been committed.
