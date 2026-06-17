# Experiment 27: Full Matrix Regression Sweep

## Description

Experiments 1-26 tested and fixed each row of the viewport matrix one at a time.
The issue README requires a final experiment that re-tests the complete matrix
and proves the behaviors work together, not just individually.

The goal of this experiment is to run the complete Ghostboard geometry matrix
against the current worktree, collect the per-row runtime evidence, and close
Issue 809 only if the final evidence satisfies the issue acceptance criteria.

This experiment should not add new feature behavior. If a matrix row fails, the
result should be recorded as `Fail` or `Partial`, the issue should remain open,
and the next experiment should localize and fix that row. If the only remaining
limitation is the already documented single-display VM constraint for display
move/backing-scale, record that as a known environment-limited partial and do
not pretend a multi-display move was verified.

## Changes

Changed files:

- `issues/0809-ghostboard-viewport-geometry/27-full-matrix-regression-sweep.md`
  - recorded the full-matrix scenario list;
  - recorded the design review, verification, per-row result table, and
    conclusion.
- `issues/0809-ghostboard-viewport-geometry/README.md`
  - linked Experiment 27 in the experiment index;
  - updated Experiment 27 status after the result.
- `scripts/ghostboard-geometry-matrix.sh`
  - fixed the `split-down` sibling-pane negative hit-test coordinate. The first
    final sweep showed that the prior fixed `SPLIT_WY + 285` coordinate still
    landed inside the browser overlay after a preceding window-resize row. The
    harness now derives the window-to-surface Y offset from the observed
    positive hit-test `top_point`, combines it with the presented root-frame
    height, and clicks below the browser-owning split.
  - added `extract_top_point` and `point_y` helpers for that calculation.

No product code changed.

## Verification

Pass criteria:

- Markdown is formatted:

  ```bash
  prettier --write --prose-wrap always --print-width 80 \
    issues/0809-ghostboard-viewport-geometry/README.md \
    issues/0809-ghostboard-viewport-geometry/27-full-matrix-regression-sweep.md
  ```

- Shell syntax and whitespace checks pass:

  ```bash
  bash -n scripts/ghostboard-geometry-matrix.sh
  git diff --check
  ```

- The full matrix is run sequentially, one scenario at a time:

  ```bash
  scripts/ghostboard-geometry-matrix.sh initial-open
  scripts/ghostboard-geometry-matrix.sh window-resize
  scripts/ghostboard-geometry-matrix.sh split-right
  scripts/ghostboard-geometry-matrix.sh split-down
  scripts/ghostboard-geometry-matrix.sh split-right-resize
  scripts/ghostboard-geometry-matrix.sh split-right-equalize
  scripts/ghostboard-geometry-matrix.sh split-right-zoom
  scripts/ghostboard-geometry-matrix.sh split-right-close-sibling
  scripts/ghostboard-geometry-matrix.sh split-right-close-browser-pane
  scripts/ghostboard-geometry-matrix.sh split-right-focus-switch
  scripts/ghostboard-geometry-matrix.sh new-terminal-tab-visibility
  scripts/ghostboard-geometry-matrix.sh open-browser-in-new-tab
  scripts/ghostboard-geometry-matrix.sh close-browser-tab
  scripts/ghostboard-geometry-matrix.sh open-browser-in-new-window
  scripts/ghostboard-geometry-matrix.sh multiple-windows-with-browsers
  scripts/ghostboard-geometry-matrix.sh display-move-backing-scale
  scripts/ghostboard-geometry-matrix.sh fullscreen-unfullscreen
  scripts/ghostboard-geometry-matrix.sh minimize-hide-restore
  scripts/ghostboard-geometry-matrix.sh font-size-cell-metrics
  scripts/ghostboard-geometry-matrix.sh tui-overlay-resize-command
  scripts/ghostboard-geometry-matrix.sh terminal-scrollback-movement
  scripts/ghostboard-geometry-matrix.sh browser-navigation-geometry
  scripts/ghostboard-geometry-matrix.sh devtools-split-geometry
  scripts/ghostboard-geometry-matrix.sh mouse-after-geometry-change
  scripts/ghostboard-geometry-matrix.sh keyboard-after-tab-window-switch
  ```

- The result records a per-row table with:
  - viewport matrix row;
  - harness scenario that covers that row;
  - status;
  - screenshot path or `n/a`;
  - harness log path;
  - app log path;
  - Roamium trace path;
  - identity tuple evidence;
  - rect/backing-scale/input notes;
  - pass/fail notes.
- If one harness scenario covers multiple README matrix rows, the table must
  list those README rows separately or explicitly map the combined scenario to
  every row it covers. Examples include focus away/back, tab switch away/back,
  and window open/switch behavior.
- The final conclusion explicitly says whether Issue 809 can close.
- The fresh-context design review is recorded in this experiment file, and the
  Experiment 27 plan is committed before the matrix sweep begins.
- A fresh-context completion review approves the result before the result
  commit.
- If closing the issue:
  - README frontmatter changes to `status = "closed"` with `closed` set to the
    current date;
  - `## Conclusion` is added to the issue README;
  - `scripts/build-issues-index.sh` is run;
  - the issue close is committed separately after the Experiment 27 result
    commit if that creates a clearer history.

Fail criteria:

- Any required matrix scenario fails and the issue is closed anyway.
- The final table omits evidence paths or collapses multiple scenarios into a
  vague summary.
- The display-move/backing-scale row is claimed as a full multi-display pass
  without actual multi-display evidence.
- Product changes are made inside this final sweep instead of being split into a
  new focused experiment.

## Design Review

Fresh-context adversarial design review initially returned **CHANGES REQUIRED**.

Required finding:

- The design did not explicitly require recording the design review and
  committing the plan before running the matrix sweep.

Optional finding:

- The planned final table listed harness scenarios but did not explicitly map
  combined harness scenarios to the README viewport matrix rows they cover.

Fixes:

- Added a pass criterion requiring design-review recording and plan commit
  before running the sweep.
- Added final-table requirements for README viewport matrix row mapping,
  including combined-scenario mappings.

Re-review verdict: **APPROVED**.

The reviewer confirmed the required design/plan commit gate and the viewport
matrix row mapping requirement are now resolved, with no new required findings.

## Result

**Result:** Pass

The first full sweep exposed a harness bug in the `split-down` negative
hit-test. The negative point was intended to land in the sibling pane but was
actually still inside the browser overlay after the preceding `window-resize`
row. Evidence:

- first summary:
  `logs/ghostboard-geometry-full-matrix-summary-20260617-154437.log`;
- failing row:
  `logs/ghostboard-geometry-split-down-harness-20260617-154537.log`;
- bad point: `split_sibling_negative_input_point=756,365`;
- AppKit hit-test proved that point was still inside the original browser
  overlay: `hit=true`, `top_point={716, 221}`, `overlay={{8, 17}, {1416, 357}}`.

After fixing the harness coordinate, standalone `split-down` passed:

```bash
scripts/ghostboard-geometry-matrix.sh split-down
```

Evidence:

- harness: `logs/ghostboard-geometry-split-down-harness-20260617-154852.log`;
- app: `logs/ghostboard-geometry-split-down-app-20260617-154852.log`;
- Roamium trace:
  `logs/ghostboard-geometry-split-down-roamium-20260617-154852.log`;
- corrected point: `split_sibling_negative_input_point=756,631`;
- result: `PASS: scenario split-down`.

A second full sweep reached `FULL MATRIX PASS`, but used an ad hoc loop whose
pipeline did not propagate a nonzero scenario exit through `tee`; that run also
contained a transient scrollback `FAIL:` line. To avoid accepting masked
failure, the full matrix was rerun with `set -o pipefail` and an explicit
summary scan for `^FAIL:` markers.

Strict full-matrix evidence:

- summary: `logs/ghostboard-geometry-full-matrix-summary-20260617-160236.log`;
- terminal result: `FULL MATRIX PASS`;
- `rg -n '^FAIL:|FULL MATRIX|RESULT .*FAIL' logs/ghostboard-geometry-full-matrix-summary-20260617-160236.log`
  returned only `FULL MATRIX PASS`;
- the display-move/backing-scale row remains environment-limited to the VM's
  single display. It verified the single-display backing-scale, geometry,
  hit-test, focus, and keyboard path, but it did not verify an actual
  cross-display move.

Shell and whitespace checks passed:

```bash
bash -n scripts/ghostboard-geometry-matrix.sh
git diff --check
```

### Final Matrix Table

| Viewport matrix row(s)                                       | Harness scenario                   | Status                      | Evidence paths                                                                                                                                                                                                                                                                                                                                                                                       | Identity / baseline rect evidence                                                                                                              | Rect, scale, input, and cleanup notes                                                                                                                              |
| ------------------------------------------------------------ | ---------------------------------- | --------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Initial browser open                                         | `initial-open`                     | Pass                        | screenshot `logs/ghostboard-geometry-initial-open-screenshot-20260617-160236.png`<br>harness `logs/ghostboard-geometry-initial-open-harness-20260617-160236.log`<br>app `logs/ghostboard-geometry-initial-open-app-20260617-160236.log`<br>trace `logs/ghostboard-geometry-initial-open-roamium-20260617-160236.log`                                                                                 | pane `3290C2BD-15C6-4AE8-8C69-349E59FD2D1C`, browser tab `1`, context `2830717290`, frame `{{8, 17}, {1336, 748}}`, pixel `2672x1496`          | Proved Zig, bridge, AppKit, Roamium resize, presented pixels, and hit-test agree.                                                                                  |
| Window resize larger; window resize smaller                  | `window-resize`                    | Pass                        | screenshot `logs/ghostboard-geometry-window-resize-screenshot-20260617-160243.png`<br>harness `logs/ghostboard-geometry-window-resize-harness-20260617-160243.log`<br>app `logs/ghostboard-geometry-window-resize-app-20260617-160243.log`<br>trace `logs/ghostboard-geometry-window-resize-roamium-20260617-160243.log`                                                                             | pane `3869CD5C-79FF-4B56-A8AB-AC57DC99051D`, browser tab `1`, context `2498740375`, baseline frame `{{8, 17}, {1336, 748}}`, pixel `2672x1496` | Grew to `3312x1870`, shrank to `2832x1632`, and hit-tests used the resized frames.                                                                                 |
| Horizontal pane split                                        | `split-right`                      | Pass                        | screenshot `logs/ghostboard-geometry-split-right-screenshot-20260617-160255.png`<br>harness `logs/ghostboard-geometry-split-right-harness-20260617-160255.log`<br>app `logs/ghostboard-geometry-split-right-app-20260617-160255.log`<br>trace `logs/ghostboard-geometry-split-right-roamium-20260617-160255.log`                                                                                     | pane `DC367F8D-A569-4EEF-9336-09AAE1AE222F`, browser tab `1`, context `180872790`, baseline frame `{{8, 17}, {1416, 816}}`, pixel `2832x1632`  | Split frame resized to `1392x1632`; sibling negative click did not route to the browser.                                                                           |
| Vertical pane split                                          | `split-down`                       | Pass                        | screenshot `logs/ghostboard-geometry-split-down-screenshot-20260617-160336.png`<br>harness `logs/ghostboard-geometry-split-down-harness-20260617-160336.log`<br>app `logs/ghostboard-geometry-split-down-app-20260617-160336.log`<br>trace `logs/ghostboard-geometry-split-down-roamium-20260617-160336.log`                                                                                         | pane `477E95C6-6C84-4D93-A823-BEF6743DA5D1`, browser tab `1`, context `3084998660`, baseline frame `{{8, 17}, {1416, 816}}`, pixel `2832x1632` | Split frame resized to `2832x714`; corrected sibling-pane point `756,631` did not route to the browser.                                                            |
| Resize existing split boundary                               | `split-right-resize`               | Pass                        | screenshot `logs/ghostboard-geometry-split-right-resize-screenshot-20260617-160417.png`<br>harness `logs/ghostboard-geometry-split-right-resize-harness-20260617-160417.log`<br>app `logs/ghostboard-geometry-split-right-resize-app-20260617-160417.log`<br>trace `logs/ghostboard-geometry-split-right-resize-roamium-20260617-160417.log`                                                         | pane `F009B592-B947-49BB-AF36-679EA78EF333`, browser tab `1`, context `3239831871`, baseline frame `{{8, 17}, {1416, 816}}`, pixel `2832x1632` | Divider resize produced `1424x1632` pixels and current-frame hit-tests; sibling negative click did not route.                                                      |
| Equalize or rebalance panes                                  | `split-right-equalize`             | Pass                        | screenshot `logs/ghostboard-geometry-split-right-equalize-screenshot-20260617-160500.png`<br>harness `logs/ghostboard-geometry-split-right-equalize-harness-20260617-160500.log`<br>app `logs/ghostboard-geometry-split-right-equalize-app-20260617-160500.log`<br>trace `logs/ghostboard-geometry-split-right-equalize-roamium-20260617-160500.log`                                                 | pane `FC70735E-0FFD-4DDC-97E3-0BC91FF41A46`, browser tab `1`, context `2093275200`, baseline frame `{{8, 17}, {1416, 816}}`, pixel `2832x1632` | Equalize returned to split baseline `1392x1632`; hit-tests and sibling negative click stayed correct.                                                              |
| Zoom or maximize pane                                        | `split-right-zoom`                 | Pass                        | screenshot `logs/ghostboard-geometry-split-right-zoom-screenshot-20260617-160544.png`<br>harness `logs/ghostboard-geometry-split-right-zoom-harness-20260617-160544.log`<br>app `logs/ghostboard-geometry-split-right-zoom-app-20260617-160544.log`<br>trace `logs/ghostboard-geometry-split-right-zoom-roamium-20260617-160544.log`                                                                 | pane `918DAF34-461B-4E01-BCB2-D76EF88A133B`, browser tab `1`, context `2897564257`, baseline frame `{{8, 17}, {1416, 816}}`, pixel `2832x1632` | Zoom expanded to full `2832x1632`, unzoom restored split `1392x1632`, and stale sibling clicks stayed outside browser input.                                       |
| Close sibling pane                                           | `split-right-close-sibling`        | Pass                        | screenshot `logs/ghostboard-geometry-split-right-close-sibling-screenshot-20260617-160559.png`<br>harness `logs/ghostboard-geometry-split-right-close-sibling-harness-20260617-160559.log`<br>app `logs/ghostboard-geometry-split-right-close-sibling-app-20260617-160559.log`<br>trace `logs/ghostboard-geometry-split-right-close-sibling-roamium-20260617-160559.log`                             | pane `48ADC8C1-B8E9-450B-BCB5-DBB989659C07`, browser tab `1`, context `834866534`, baseline frame `{{8, 17}, {1416, 816}}`, pixel `2832x1632`  | Closing the sibling expanded the browser back to full pane; former sibling area hit-tested the expanded browser frame.                                             |
| Close browser pane                                           | `split-right-close-browser-pane`   | Pass                        | screenshot `logs/ghostboard-geometry-split-right-close-browser-pane-screenshot-20260617-160610.png`<br>harness `logs/ghostboard-geometry-split-right-close-browser-pane-harness-20260617-160610.log`<br>app `logs/ghostboard-geometry-split-right-close-browser-pane-app-20260617-160610.log`<br>trace `logs/ghostboard-geometry-split-right-close-browser-pane-roamium-20260617-160610.log`         | pane `6DCFED06-1229-46F0-8740-093B356597E0`, browser tab `1`, context `3455265492`, baseline frame `{{8, 17}, {1416, 816}}`, pixel `2832x1632` | Clear overlay, `CloseTab`, Roamium tab destruction, stale click rejection, and remaining sibling keyboard receipt all passed.                                      |
| Focus different pane in same tab; focus back to browser pane | `split-right-focus-switch`         | Pass                        | screenshot `logs/ghostboard-geometry-split-right-focus-switch-screenshot-20260617-160726.png`<br>harness `logs/ghostboard-geometry-split-right-focus-switch-harness-20260617-160726.log`<br>app `logs/ghostboard-geometry-split-right-focus-switch-app-20260617-160726.log`<br>trace `logs/ghostboard-geometry-split-right-focus-switch-roamium-20260617-160726.log`                                 | pane `B3FE193B-EBBF-4D3F-9239-40F3155ADEB4`, browser tab `1`, context `1700436948`, baseline frame `{{8, 17}, {1416, 816}}`, pixel `2832x1632` | Sibling focus set Roamium focus false and did not deliver browser keys; browser refocus plus Browse mode restored focus and keyboard delivery.                     |
| New terminal tab; switch back to browser tab                 | `new-terminal-tab-visibility`      | Pass                        | screenshot `logs/ghostboard-geometry-new-terminal-tab-visibility-screenshot-20260617-160739.png`<br>harness `logs/ghostboard-geometry-new-terminal-tab-visibility-harness-20260617-160739.log`<br>app `logs/ghostboard-geometry-new-terminal-tab-visibility-app-20260617-160739.log`<br>trace `logs/ghostboard-geometry-new-terminal-tab-visibility-roamium-20260617-160739.log`                     | pane `F0E47328-D448-4BC0-B162-1556A5243E73`, browser tab `1`, context `956051740`, baseline frame `{{8, 17}, {1416, 816}}`, pixel `2832x1632`  | Plain tab did not present or receive browser input; switching back restored tab-bar-adjusted frame and browser keyboard.                                           |
| Open browser in new tab                                      | `open-browser-in-new-tab`          | Pass                        | screenshot `logs/ghostboard-geometry-open-browser-in-new-tab-screenshot-20260617-160827.png`<br>harness `logs/ghostboard-geometry-open-browser-in-new-tab-harness-20260617-160827.log`<br>app `logs/ghostboard-geometry-open-browser-in-new-tab-app-20260617-160827.log`<br>trace `logs/ghostboard-geometry-open-browser-in-new-tab-roamium-20260617-160827.log`                                     | pane `CBB25952-47A5-4666-868E-6F21FE4982B2`, browser tab `1`, context `773230429`, baseline frame `{{8, 17}, {1416, 816}}`, pixel `2832x1632`  | Browser A and B kept independent tab ids, frames, focus, and keyboard routing across tab switches.                                                                 |
| Close browser tab                                            | `close-browser-tab`                | Pass                        | screenshot `logs/ghostboard-geometry-close-browser-tab-screenshot-20260617-160921.png`<br>harness `logs/ghostboard-geometry-close-browser-tab-harness-20260617-160921.log`<br>app `logs/ghostboard-geometry-close-browser-tab-app-20260617-160921.log`<br>trace `logs/ghostboard-geometry-close-browser-tab-roamium-20260617-160921.log`                                                             | pane `66F27564-0C41-404C-A2E8-C84A9DF8B0FC`, browser tab `1`, context `2799302634`, baseline frame `{{8, 17}, {1416, 816}}`, pixel `2832x1632` | Browser B tab close selected A, cleared B, sent `CloseTab`, destroyed B in Roamium, and prevented closed-tab input.                                                |
| Open browser in new window                                   | `open-browser-in-new-window`       | Pass                        | screenshot `logs/ghostboard-geometry-open-browser-in-new-window-screenshot-20260617-161018.png`<br>harness `logs/ghostboard-geometry-open-browser-in-new-window-harness-20260617-161018.log`<br>app `logs/ghostboard-geometry-open-browser-in-new-window-app-20260617-161018.log`<br>trace `logs/ghostboard-geometry-open-browser-in-new-window-roamium-20260617-161018.log`                         | pane `EA31CCC1-CED5-4A06-A940-11B4812C01BD`, browser tab `1`, context `2628375845`, baseline frame `{{8, 17}, {1416, 816}}`, pixel `2832x1632` | Browser B appeared only in window B; returning to window A preserved A frame and keyboard isolation.                                                               |
| Multiple windows with browsers                               | `multiple-windows-with-browsers`   | Pass                        | screenshot `logs/ghostboard-geometry-multiple-windows-with-browsers-screenshot-20260617-161035.png`<br>harness `logs/ghostboard-geometry-multiple-windows-with-browsers-harness-20260617-161035.log`<br>app `logs/ghostboard-geometry-multiple-windows-with-browsers-app-20260617-161035.log`<br>trace `logs/ghostboard-geometry-multiple-windows-with-browsers-roamium-20260617-161035.log`         | pane `A776C4A6-266B-44A1-B1EC-05741366ADC7`, browser tab `1`, context `3679319445`, baseline frame `{{8, 17}, {1416, 816}}`, pixel `2832x1632` | Windows A, B, and C kept independent surface/window ids and keyboard delivery.                                                                                     |
| Move window between displays                                 | `display-move-backing-scale`       | Partial (single-display VM) | screenshot `logs/ghostboard-geometry-display-move-backing-scale-screenshot-20260617-161100.png`<br>harness `logs/ghostboard-geometry-display-move-backing-scale-harness-20260617-161100.log`<br>app `logs/ghostboard-geometry-display-move-backing-scale-app-20260617-161100.log`<br>trace `logs/ghostboard-geometry-display-move-backing-scale-roamium-20260617-161100.log`                         | pane `C513EB5F-84DF-4827-A687-638B04F15F31`, browser tab `1`, context `2774710255`, baseline frame `{{8, 17}, {1416, 816}}`, pixel `2832x1632` | VM had `display_count=1`, backing scale `2.0`; single-display geometry, hit-test, focus, and keyboard passed, but no cross-display move was performed.             |
| Fullscreen and unfullscreen                                  | `fullscreen-unfullscreen`          | Pass                        | screenshot `logs/ghostboard-geometry-fullscreen-unfullscreen-screenshot-20260617-161108.png`<br>harness `logs/ghostboard-geometry-fullscreen-unfullscreen-harness-20260617-161108.log`<br>app `logs/ghostboard-geometry-fullscreen-unfullscreen-app-20260617-161108.log`<br>trace `logs/ghostboard-geometry-fullscreen-unfullscreen-roamium-20260617-161108.log`                                     | pane `69C94DF3-97FE-4B55-A157-44AE7808A4D9`, browser tab `1`, context `3306399311`, baseline frame `{{8, 17}, {1416, 816}}`, pixel `2832x1632` | Fullscreen resized to `3504x2244`, unfullscreen returned to `2832x1632`; focus and keyboard worked after both transitions.                                         |
| Minimize, hide, and restore                                  | `minimize-hide-restore`            | Pass                        | screenshot `logs/ghostboard-geometry-minimize-hide-restore-screenshot-20260617-161130.png`<br>harness `logs/ghostboard-geometry-minimize-hide-restore-harness-20260617-161130.log`<br>app `logs/ghostboard-geometry-minimize-hide-restore-app-20260617-161130.log`<br>trace `logs/ghostboard-geometry-minimize-hide-restore-roamium-20260617-161130.log`                                             | pane `5F1027C0-10F9-4E41-963D-B6D8A03948D3`, browser tab `1`, context `1040330089`, baseline frame `{{8, 17}, {1416, 816}}`, pixel `2832x1632` | Minimized/hidden windows disappeared from onscreen list, did not route stale hits, and restored geometry and keyboard input.                                       |
| Terminal font-size/cell-metric change                        | `font-size-cell-metrics`           | Pass                        | screenshot `logs/ghostboard-geometry-font-size-cell-metrics-screenshot-20260617-161202.png`<br>harness `logs/ghostboard-geometry-font-size-cell-metrics-harness-20260617-161202.log`<br>app `logs/ghostboard-geometry-font-size-cell-metrics-app-20260617-161202.log`<br>trace `logs/ghostboard-geometry-font-size-cell-metrics-roamium-20260617-161202.log`                                         | pane `871EFA74-ABCB-4B73-8D7F-9E6B527CC026`, browser tab `1`, context `983780846`, baseline frame `{{8, 17}, {1416, 816}}`, pixel `2832x1632`  | Font increase changed cell/grid and frame to `2826x1600`; decrease restored `2832x1632`; hit-test and keyboard worked both times.                                  |
| TUI overlay resize command                                   | `tui-overlay-resize-command`       | Pass                        | screenshot `logs/ghostboard-geometry-tui-overlay-resize-command-screenshot-20260617-161214.png`<br>harness `logs/ghostboard-geometry-tui-overlay-resize-command-harness-20260617-161214.log`<br>app `logs/ghostboard-geometry-tui-overlay-resize-command-app-20260617-161214.log`<br>trace `logs/ghostboard-geometry-tui-overlay-resize-command-roamium-20260617-161214.log`                         | pane `CD3ACC25-B098-4C28-BBE4-76C2A2592F6E`, browser tab `1`, context `3794772387`, baseline frame `{{8, 17}, {1416, 816}}`, pixel `2832x1632` | `:viewport height 12` shrank to `2832x408`; stale lower hit was false; reset restored `2832x1632` and keyboard.                                                    |
| Terminal scrollback movement                                 | `terminal-scrollback-movement`     | Pass                        | screenshot `logs/ghostboard-geometry-terminal-scrollback-movement-screenshot-20260617-161229.png`<br>harness `logs/ghostboard-geometry-terminal-scrollback-movement-harness-20260617-161229.log`<br>app `logs/ghostboard-geometry-terminal-scrollback-movement-app-20260617-161229.log`<br>trace `logs/ghostboard-geometry-terminal-scrollback-movement-roamium-20260617-161229.log`                 | pane `C2EF01AC-39D5-4242-9342-13B5F6CFA777`, browser tab `1`, context `537508546`, baseline frame `{{8, 17}, {1416, 816}}`, pixel `2832x1632`  | Scrollback up and return-to-bottom did not resize Roamium, preserved AppKit frame/pixels, and delivered browser keyboard in Browse mode.                           |
| Browser navigation                                           | `browser-navigation-geometry`      | Pass                        | screenshot `logs/ghostboard-geometry-browser-navigation-geometry-screenshot-20260617-161242.png`<br>harness `logs/ghostboard-geometry-browser-navigation-geometry-harness-20260617-161242.log`<br>app `logs/ghostboard-geometry-browser-navigation-geometry-app-20260617-161242.log`<br>trace `logs/ghostboard-geometry-browser-navigation-geometry-roamium-20260617-161242.log`                     | pane `F6E856C9-24BA-4F47-8E19-CF9FEE16F377`, browser tab `1`, context `3583129273`, baseline frame `{{8, 17}, {1416, 816}}`, pixel `2832x1632` | Navigate, UrlChanged, stable frame/pixels, no extra resize, hit-test, and keyboard delivery all passed.                                                            |
| DevTools split or tab                                        | `devtools-split-geometry`          | Pass                        | screenshot `logs/ghostboard-geometry-devtools-split-geometry-screenshot-20260617-161252.png`<br>harness `logs/ghostboard-geometry-devtools-split-geometry-harness-20260617-161252.log`<br>app `logs/ghostboard-geometry-devtools-split-geometry-app-20260617-161252.log`<br>trace `logs/ghostboard-geometry-devtools-split-geometry-roamium-20260617-161252.log`                                     | pane `250FA933-3CBA-4203-A853-8A7026C2F10C`, browser tab `1`, context `377938449`, baseline frame `{{8, 17}, {1416, 816}}`, pixel `2832x1632`  | DevTools split right created independent DevTools tab/context, resized both panes, and isolated mouse and keyboard between DevTools and normal browser.            |
| Mouse input after geometry change                            | `mouse-after-geometry-change`      | Pass                        | screenshot `logs/ghostboard-geometry-mouse-after-geometry-change-screenshot-20260617-161306.png`<br>harness `logs/ghostboard-geometry-mouse-after-geometry-change-harness-20260617-161306.log`<br>app `logs/ghostboard-geometry-mouse-after-geometry-change-app-20260617-161306.log`<br>trace `logs/ghostboard-geometry-mouse-after-geometry-change-roamium-20260617-161306.log`                     | pane `0CA026A9-978F-4EA1-9AB8-51C06ED2F97B`, browser tab `1`, context `4147879958`, baseline frame `{{8, 17}, {1416, 816}}`, pixel `2832x1632` | Mouse coordinates matched AppKit `web_point` within 1 CSS px after resize, TUI shrink/reset, split, divider resize, and equalize; stale coordinates did not route. |
| Keyboard input after tab/window switch                       | `keyboard-after-tab-window-switch` | Pass                        | screenshot `logs/ghostboard-geometry-keyboard-after-tab-window-switch-screenshot-20260617-161338.png`<br>harness `logs/ghostboard-geometry-keyboard-after-tab-window-switch-harness-20260617-161338.log`<br>app `logs/ghostboard-geometry-keyboard-after-tab-window-switch-app-20260617-161338.log`<br>trace `logs/ghostboard-geometry-keyboard-after-tab-window-switch-roamium-20260617-161338.log` | pane `7B372B58-C5DE-4E79-9907-267175C9EFB5`, browser tab `1`, context `2280090281`, baseline frame `{{8, 17}, {1336, 748}}`, pixel `2672x1496` | Keyboard reached only the active browser after switching through a plain tab, browser B tab, browser C window, and restored browser A.                             |

## Conclusion

The full geometry matrix is now covered by the reusable Ghostboard harness and
passes in this VM, with the explicit caveat that cross-display movement cannot
be fully tested because the VM exposes only one display. The final strict sweep
proved geometry, visibility, cleanup, mouse input, and keyboard input across the
available pane, tab, window, resize, fullscreen, minimize/hide, scrollback,
navigation, DevTools, and input-routing transitions.

Issue 809 can close after completion review and the required commits.

## Completion Review

Fresh-context adversarial completion review verdict: **APPROVED**.

Findings: none.

The reviewer independently checked that:

- the worktree changes are limited to the harness and issue documentation, with
  no product code changes;
- the result commit had not been made before review;
- `bash -n scripts/ghostboard-geometry-matrix.sh` passed;
- `git diff --check` passed;
- the strict final summary scan returned only `FULL MATRIX PASS`;
- every matrix scenario has a `RESULT ... PASS` line in the strict summary;
- the display-move/backing-scale limitation is honestly recorded as
  single-display-limited with `display_count=1`;
- the `split-down` harness fix is coherent and continues to reject any
  `hit=true` routing to the original browser context while allowing no
  browser-layer hit-test for a sibling-pane click.
