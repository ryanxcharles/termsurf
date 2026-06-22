# Experiment 23: Add focused Surfari input regression guard

## Description

Experiments 21 and 22 proved the single-window, single-tab, single-pane real-app
Surfari path for keyboard input and page-visible wheel input. Issue 756 now
needs a durable guard so later Surfari, Ghostboard, or WebKit-wrapper changes do
not silently break that path.

This experiment should add a focused regression guard around the behavior we
have actually proven, while avoiding an overbroad or slow full-matrix test. It
should not expand into split panes, tab switching, window switching, restart,
profile isolation, crash handling, or the full Ghostboard/Roamium comparison
matrix. Those remain later Issue 756 work.

## Changes

- Add a small, documented regression entry point for the existing real-app
  Surfari input harness.
- Keep the guard focused on the proven single-pane behavior:
  - real Debug `TermSurf.app` launches;
  - repo `web --browser surfari` launches;
  - repo `surfari` launches;
  - WebKit CAContext overlay is presented;
  - Browse mode focuses Surfari;
  - keyboard input reaches the fixture page;
  - wheel input reaches the fixture page;
  - Surfari closes cleanly.
- Avoid adding this guard to a broad default test target if it would make common
  local checks too slow or permission-sensitive.
- Make the command discoverable from scripts/docs so future agents and humans
  know which focused guard protects the Surfari input path.
- Preserve the existing harness behavior that DOM click is warning-only while
  DOM wheel is required for final pass.

Likely implementation:

- Add a wrapper script under `scripts/` with a stable name such as
  `test-issue-756-surfari-input-regression.sh`.
- Have it verify required build artifacts or explain which build commands to run
  if they are missing.
- Have it call `scripts/test-issue-756-real-app-surfari-input-routing.sh`.
- Document when to run it in this experiment result and, if there is an
  appropriate existing test index, link it there without making it part of a
  fast default suite.

## Verification

Pass criteria:

- Run the new focused guard command.
- The guard must fail if the real-app harness fails.
- The guard must preserve the required evidence from Experiments 21 and 22:
  - fixture page logs `kind=input value=a`;
  - Surfari logs keyboard input;
  - Surfari logs wheel input;
  - fixture page logs `kind=wheel`;
  - final harness output includes
    `PASS: issue 756 real-app Surfari input routing`.
- Run hygiene checks:

```bash
git diff --check
bash -n scripts/test-issue-756-real-app-surfari-input-routing.sh
bash -n scripts/test-issue-756-surfari-input-regression.sh
prettier --check --prose-wrap always --print-width 80 \
  issues/0756-surfari/README.md \
  issues/0756-surfari/23-surfari-input-regression-guard.md
```

Run formatting/checks for any source files touched:

```bash
cargo fmt -- <rust-files>
zig fmt <zig-files>
```

Result classification:

- `Pass` means there is a documented focused regression guard for the proven
  Surfari keyboard and wheel path, and it passes in the current environment.
- `Partial` means the guard exists but is not yet sufficiently documented,
  reliable, or verified.
- `Fail` means the guard cannot run or cannot preserve the proven input
  evidence.

## Design Review

Adversarial design review returned `APPROVED` with no findings. The reviewer
confirmed that the README links Experiment 23 as `Designed`, the file has the
required Description, Changes, and Verification sections, the scope is limited
to a focused regression guard for the proven single-pane keyboard and wheel
path, the design avoids default-suite coupling and calls out
permission/slow-test concerns, it preserves the Experiment 22 click limitation
as warning-only while requiring wheel evidence, the verification includes
pass/fail criteria and hygiene checks, and the plan commit had not already been
made.

## Result

**Result:** Pass

Added `scripts/test-issue-756-surfari-input-regression.sh` as the focused guard
for the proven single-pane Surfari keyboard and wheel path. The wrapper checks
for the required debug app, `web`, `surfari`, WebKit framework, and
`libtermsurf_webkit` artifacts. If any are missing, it prints the exact build
commands to run. If all prerequisites exist, it delegates to
`scripts/test-issue-756-real-app-surfari-input-routing.sh`.

The guard intentionally does not join a fast default test suite. It launches the
real macOS app, depends on GUI automation permissions, and takes longer than a
normal unit/integration test. It is a focused, opt-in regression command for the
Surfari real-app input path.

Verification:

- `bash -n scripts/test-issue-756-surfari-input-regression.sh` passed.
- `bash -n scripts/test-issue-756-real-app-surfari-input-routing.sh` passed.
- `git diff --check` passed.
- Guard run `20260621-183959` passed end to end:
  - real Debug `TermSurf.app` launched;
  - repo `target/debug/web --browser surfari` launched;
  - repo `target/debug/surfari` launched;
  - Surfari's WebKit CAContext overlay was presented;
  - Browse mode focused Surfari;
  - Ghostboard stayed frontmost before keyboard injection;
  - Surfari logged keyboard input;
  - the fixture page logged `kind=input value=a`;
  - Surfari logged mouse and wheel input;
  - DOM click remained warning-only;
  - the fixture page logged `kind=wheel`;
  - Surfari accepted `CloseTab` and began clean shutdown;
  - the harness printed final `PASS: issue 756 real-app Surfari input routing`.

## Conclusion

Issue 756 now has a durable focused regression guard for the Surfari behavior
that has actually been proven: real-app single-pane launch, overlay
presentation, Browse-mode focus, keyboard input to the page, wheel input to the
page, and clean shutdown. The next experiments can use this guard to protect the
baseline while expanding into the remaining pane, tab, window, resize, restart,
profile, crash, and Ghostboard/Roamium comparison matrix.

## Completion Review

Adversarial completion review returned `APPROVED` with no findings. The reviewer
confirmed that the result commit had not already been made, the new guard is
opt-in only and delegates to the proven real-app harness, missing-prerequisite
simulation exits nonzero and prints build instructions, the final
`20260621-183959` logs contain the claimed keyboard, page input, wheel, page
wheel, `CloseTab`, clean shutdown, and final `PASS` evidence, the README marks
Experiment 23 as `Pass`, and the documented read-only checks pass.
