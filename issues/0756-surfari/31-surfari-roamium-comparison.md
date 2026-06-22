# Experiment 31: Compare Surfari against the Roamium matrix

## Description

Experiment 30 proved Surfari crash handling. The only remaining
`real-app-matrix.md` row is `Roamium comparison`.

This experiment should perform the final Surfari parity comparison against the
Ghostboard/Roamium behavior matrix. The point is not to port the large
Roamium-specific `scripts/ghostboard-geometry-matrix.sh` wholesale. Instead, use
that script as the authoritative list of comparable real-app behaviors, then
prove each comparable behavior with the focused Surfari harnesses created in
Experiments 23 and 25-30.

If the comparison finds a real Surfari parity gap, this experiment should record
`Partial` or `Fail` and the next experiment should fix the gap. If every
comparable row is proven or intentionally engine-specific, this experiment can
mark `Roamium comparison` as `Proven` and close Issue 756.

## Changes

- Add a comparison artifact under `issues/0756-surfari/`, tentatively
  `surfari-roamium-comparison.md`, that maps Roamium scenarios to Surfari
  evidence.
- Create a focused aggregate harness under `scripts/`, tentatively
  `test-issue-756-surfari-final-comparison.sh`, that reruns the Surfari real-app
  evidence suite:
  - `scripts/test-issue-756-surfari-input-regression.sh`;
  - `scripts/test-issue-756-surfari-lifecycle-tranche.sh`;
  - `scripts/test-issue-756-surfari-pane-split-geometry.sh`;
  - `scripts/test-issue-756-surfari-tab-window-focus-geometry.sh`;
  - `scripts/test-issue-756-surfari-click-drag-input-details.sh`;
  - `scripts/test-issue-756-surfari-profile-isolation.sh`;
  - `scripts/test-issue-756-surfari-crash-handling.sh`.
- The aggregate harness should write its own log under
  `logs/issue-756-exp31-surfari-roamium-comparison/` and record each child
  harness run ID or log path.
- The aggregate harness must fail before running child harnesses if the Debug
  `TermSurf.app` binary is missing or stale relative to Ghostboard source/build
  inputs. Reuse the freshness guard pattern from
  `scripts/test-issue-756-surfari-crash-handling.sh`.
- The comparison artifact should classify each relevant Roamium scenario from
  `scripts/ghostboard-geometry-matrix.sh` as one of:
  - `Equivalent` — Surfari has direct current evidence for the same user-visible
    behavior;
  - `Covered by focused Surfari harness` — the exact Roamium scenario name is
    not reused, but the behavior is directly proven by a focused Surfari
    harness;
  - `Engine-specific difference` — the behavior is intentionally different
    because Surfari uses WebKit instead of Chromium/Roamium;
  - `Not applicable` — the scenario tests Roamium launch/resolver behavior or
    another Chromium-specific path that is outside Surfari parity;
  - `Gap` — Surfari lacks direct evidence or fails the comparable behavior.
- At minimum, compare the Roamium scenario groups already listed in
  `real-app-matrix.md`:
  - lifecycle/navigation/resize: `browser-command-navigation`, `window-resize`,
    `browser-navigation-geometry`;
  - pane and split geometry: `split-right`, `split-down`, `split-right-resize`,
    `split-right-equalize`, `split-right-zoom`, `split-right-close-sibling`,
    `split-right-close-browser-pane`;
  - tabs/windows/focus: `new-terminal-tab-visibility`,
    `open-browser-in-new-tab`, `close-browser-tab`,
    `open-browser-in-new-window`, `multiple-windows-with-browsers`,
    `keyboard-after-tab-window-switch`, `gui-active-multi-tab`;
  - input details: `browser-input-granularity`, `mouse-after-geometry-change`;
  - profiles/lifecycle/crash: `multi-profile-isolation`,
    `same-profile-server-lifecycle`, `tui-disconnect-reconnect`,
    `renderer-crash-smoke`.
- Also scan the full scenario list in `scripts/ghostboard-geometry-matrix.sh`
  and explicitly account for any omitted scenario as non-comparable,
  engine-specific, already covered, or a gap. This prevents the comparison from
  silently ignoring a Roamium behavior.
- Update `issues/0756-surfari/real-app-matrix.md` only if the fresh aggregate
  evidence proves the `Roamium comparison` row.
- If all matrix rows become `Proven`, update the Issue 756 README conclusion and
  close the issue only after completion review approves the result.

## Verification

Pass criteria:

- Build or confirm required artifacts:

```bash
surfari/libtermsurf_webkit/build.sh
cargo build -p surfari
cargo build -p webtui
(cd ghostboard && zig build)
(cd ghostboard && macos/build.nu --configuration Debug --action build)
```

- Run the aggregate comparison harness.
- The aggregate harness must fail if any child Surfari harness fails.
- The aggregate harness must fail if any child harness log path is missing.
- The aggregate harness must fail if the Debug `TermSurf.app` binary it launches
  is missing or older than Ghostboard source/build inputs.
- The comparison artifact must include every Roamium scenario from the
  `scripts/ghostboard-geometry-matrix.sh` scenario list or explain why a
  scenario is intentionally excluded from Surfari parity.
- The comparison artifact must contain no `Gap` rows for a `Pass` result.
- The `Roamium comparison` row in `real-app-matrix.md` may become `Proven` only
  if the aggregate harness passes and the comparison artifact has no unresolved
  gaps.
- Run hygiene checks:

```bash
git diff --check
bash -n scripts/test-issue-756-surfari-final-comparison.sh
prettier --check --prose-wrap always --print-width 80 \
  issues/0756-surfari/README.md \
  issues/0756-surfari/31-surfari-roamium-comparison.md \
  issues/0756-surfari/real-app-matrix.md \
  issues/0756-surfari/surfari-roamium-comparison.md
```

Run formatting/checks for any source files touched:

```bash
cargo fmt -p surfari -p webtui -- --check
zig fmt <zig-files>
```

Result classification:

- `Pass` means the aggregate Surfari harness suite passes, every comparable
  Roamium scenario is mapped to current Surfari evidence or an intentional
  engine-specific/non-applicable difference, no `Gap` rows remain, and Issue 756
  can be closed after review.
- `Partial` means most comparison evidence passes, but at least one comparable
  behavior remains unproven, flaky, or too weakly mapped.
- `Fail` means the aggregate harness cannot complete or the comparison exposes a
  fundamental Surfari parity gap.

## Design Review

Adversarial design review initially returned `CHANGES REQUIRED` with one
Required finding: the verification plan could run the aggregate comparison
against a stale Debug `TermSurf.app` bundle because it only required
`zig build`, while the child Surfari harnesses launch
`ghostboard/macos/build/Debug/TermSurf.app`.

The design was updated to require building the Debug app bundle with
`macos/build.nu --configuration Debug --action build` and to require the
aggregate harness to fail if the app binary is missing or stale relative to
Ghostboard source/build inputs.

Focused re-review confirmed that stale-bundle finding was resolved, then found
one new command-sequence issue: two consecutive `cd ghostboard && ...` commands
would not be runnable if copied as a single shell block. The design was updated
to use independent subshells:

```bash
(cd ghostboard && zig build)
(cd ghostboard && macos/build.nu --configuration Debug --action build)
```

Final focused re-review returned `APPROVED` with no Required findings. The
reviewer confirmed the command block is now runnable and no new Required finding
was introduced by the fix.

## Result

**Result:** Pass

Passing run:

```bash
surfari/libtermsurf_webkit/build.sh
cargo build -p surfari
cargo build -p webtui
(cd ghostboard && zig build)
(cd ghostboard && macos/build.nu --configuration Debug --action build)
git diff --check
bash -n scripts/test-issue-756-surfari-final-comparison.sh
prettier --check --prose-wrap always --print-width 80 \
  issues/0756-surfari/README.md \
  issues/0756-surfari/31-surfari-roamium-comparison.md \
  issues/0756-surfari/real-app-matrix.md \
  issues/0756-surfari/surfari-roamium-comparison.md
scripts/test-issue-756-surfari-final-comparison.sh
```

Aggregate run ID: `20260621-212614`.

Aggregate log:

- `logs/issue-756-exp31-surfari-roamium-comparison/harness-20260621-212614.log`

The aggregate harness proved:

- fake-GUI DevTools support passed, with `SMOKE_PASS` showing
  `devtools_supported=1` and `clean_exit=1`;
- the input regression child harness passed with run `20260621-212614`;
- the lifecycle child harness passed with run `20260621-212704`;
- the pane/split child harness passed with run `20260621-212745`;
- the tab/window/focus child harness passed with run `20260621-212834`;
- the click/drag child harness passed with run `20260621-212911`;
- the profile-isolation child harness passed with run `20260621-212951`;
- the crash-handling child harness passed with run `20260621-213038`;
- every child-reported log path existed.

Implementation changes:

- Added `scripts/test-issue-756-surfari-final-comparison.sh`, an aggregate
  comparison harness that checks for a fresh Debug `TermSurf.app`, verifies
  required repo-built Surfari/WebTUI/WebKit artifacts, runs the fake-GUI
  DevTools proof, and reruns the focused Surfari real-app harness suite.
- Added `issues/0756-surfari/surfari-roamium-comparison.md`, which accounts for
  all 51 scenarios accepted by `scripts/ghostboard-geometry-matrix.sh`.
- Updated `scripts/test-issue-756-surfari-tab-window-focus-geometry.sh` so the
  B/C browser tab and pane IDs come from stable `BrowserReady` lines and the CA
  context ID comes from Surfari's trace, avoiding interleaved app-log
  `ca_context` lines.
- Updated `issues/0756-surfari/real-app-matrix.md` so `Roamium comparison` is
  `Proven`.

The first aggregate attempt, `20260621-210521`, tried to include the standalone
`libtermsurf_webkit` C smoke as a fresh child run. That failed on the known
standalone DOM-visible focus limitation documented in Experiment 6. A second
attempt moved past focus but failed on standalone synthetic mousemove. The
aggregate harness was narrowed back to the approved Experiment 31 scope: the
Ghostboard/Surfari comparison suite plus fake-GUI DevTools evidence. Lower-level
callback scenarios such as JavaScript dialogs, HTTP auth, color scheme, cursor,
target URL, and console are explicitly mapped to prior committed
`libtermsurf_webkit` smoke evidence in the comparison artifact rather than being
rerun as part of the final real-app aggregate.

The fake-GUI DevTools child initially failed because the aggregate passed a log
directory whose absolute path made `gui.sock` exceed AF_UNIX path length limits.
The aggregate now uses a shorter per-run fake-GUI log directory under
`logs/i756e31fg-<run-id>/`. The next attempt passed fake-GUI DevTools but
expected a nonexistent `surfari.log`; the aggregate now checks the actual
fake-GUI outputs: `messages.log`, `surfari-trace.log`, `surfari.stdout`, and
`surfari.stderr`.

Initial completion review found that the aggregate harness only validated child
log lines beginning at column 1. Some child harnesses also print indented
summary log paths such as `  harness=...`, so the aggregate could pass without
checking every reported path. The parser now accepts optional leading
whitespace, validates `app=`, `harness=`, and scenario trace keys, and the final
aggregate run `20260621-212614` proves those indented paths are captured.

After the parser fix, one aggregate run exposed a tab/window/focus harness
flake: the app-log `ca_context` line for browser B was interleaved with
`[Surfari] client connected`, so the harness extracted a bogus pane ID even
though Surfari and WebTUI later showed browser B loaded correctly. The child
harness now derives B/C pane and tab IDs from `BrowserReady` and context IDs
from Surfari's own trace. A standalone child run `20260621-212527` passed, and
the final aggregate run `20260621-212614` passed with that fix.

## Conclusion

All Issue 756 real-app matrix rows are now `Proven`. The final comparison
artifact accounts for every Roamium scenario and contains no `Gap` rows.

## Completion Review

Adversarial completion review returned `APPROVED` with no Required findings. The
reviewer confirmed:

- the prior `harness=...` parser finding is fixed because the aggregate harness
  now accepts optional leading whitespace and validates the previously missed
  indented child log paths;
- `git diff --check`, shell syntax checks, and Prettier checks pass;
- the comparison artifact covers all 51 Roamium scenarios with no missing
  scenarios and no `Gap` rows;
- the tab/window/focus harness fix is supported by standalone run
  `20260621-212527` and aggregate child run `20260621-212834`;
- the result commit had not been made before review.

With completion review approved, Issue 756 can be closed.
