# Surfari Real-App Matrix

This matrix tracks Issue 756's remaining real-app Surfari coverage. Status
values are deliberately conservative:

- `Proven` means current issue evidence directly proves the row.
- `Partial` means the row has some evidence, but the exact requirement is not
  fully proven.
- `Missing` means there is no direct real-app Surfari evidence yet.
- `Blocked` means the row cannot currently be tested without a known external
  fix or permission change.

## Matrix

| Area               | Status  | Current Evidence                                                                                                                                                                                                      | Required Proof To Mark Proven                                                                                                         | Proposed Harness / Scenario                                     |
| ------------------ | ------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------- |
| Navigation         | Proven  | Experiment 25 run `20260621-190346` loaded fixture A, sent explicit `Navigate` to fixture B, and observed URL/title updates in Surfari and WebTUI traces.                                                             | Already proven for single-pane explicit navigation after initial load; pane/tab/window navigation remains part of later matrix rows.  | `scripts/test-issue-756-surfari-lifecycle-tranche.sh`.          |
| Keyboard input     | Proven  | Experiments 21 and 23 proved Surfari `key-event` and fixture `kind=input value=a` in the real app.                                                                                                                    | Already proven for single-pane real app; future matrix rows must ensure it survives tabs/windows/focus changes.                       | `scripts/test-issue-756-surfari-input-regression.sh`.           |
| Click              | Proven  | Experiment 28 run `20260621-201208` proved Surfari receives click-zone mouse events and the fixture observes DOM `click detail=1` in the real app.                                                                    | Already proven for single-click DOM delivery in a single-pane real-app Surfari fixture; broader click-count parity remains separate.  | `scripts/test-issue-756-surfari-click-drag-input-details.sh`.   |
| Drag               | Proven  | Experiment 28 run `20260621-201208` proved Surfari receives drag down/move/up input and the fixture observes browser text selection `ISSUE756_EXP28_BROWSER_DRAG_TEXT`.                                               | Already proven for page-visible browser text selection from a real-app Surfari drag.                                                  | `scripts/test-issue-756-surfari-click-drag-input-details.sh`.   |
| Scroll / wheel     | Proven  | Experiment 22 and guard run `20260621-183959` proved Surfari `scroll-event` and fixture `kind=wheel`.                                                                                                                 | Already proven for page-visible wheel delivery; coordinate fidelity still needs a later assertion if required.                        | `scripts/test-issue-756-surfari-input-regression.sh`.           |
| Resize             | Proven  | Experiments 20 and 25 proved real-app window resize produced Surfari resize to the new pixel size.                                                                                                                    | Already proven for a single-window app resize; pane-specific resize remains separate.                                                 | `scripts/test-issue-756-surfari-lifecycle-tranche.sh`.          |
| Pane resize        | Proven  | Experiment 26 run `20260621-191750` proved right-split divider resize changed the Surfari overlay frame/pixels, sent Surfari `resize`, preserved inside hit testing, and rejected sibling-pane hit testing.           | Already proven for right-split divider resize; tab/window/focus interactions remain separate.                                         | `scripts/test-issue-756-surfari-pane-split-geometry.sh`.        |
| Split panes        | Proven  | Experiment 26 run `20260621-191750` proved right and down splits move/resize the Surfari overlay, send Surfari pixel resize, keep inside hit testing, and reject sibling-pane hit testing.                            | Already proven for single-browser right/down split geometry; tab/window/focus interactions remain separate.                           | `scripts/test-issue-756-surfari-pane-split-geometry.sh`.        |
| Tab switching      | Proven  | Experiment 27 run `20260621-194431` proved browser A hidden and not hit-testable in a plain terminal tab, restored browser A hit testing/keyboard routing, opened browser B in tab 2, and isolated A/B routing.       | Already proven for plain-terminal-tab switching and two Surfari browsers in separate native tabs; profile isolation remains separate. | `scripts/test-issue-756-surfari-tab-window-focus-geometry.sh`.  |
| Window switching   | Proven  | Experiment 27 run `20260621-194431` created a second native window, opened browser C there, proved it used distinct pane/tab/context IDs, presented on that window, and isolated hit testing/keyboard routing.        | Already proven for opening a Surfari browser in a second native window and keeping it out of the original window.                     | `scripts/test-issue-756-surfari-tab-window-focus-geometry.sh`.  |
| Focus changes      | Proven  | Experiment 27 run `20260621-194431` proved browse/control mode focus true/false and keyboard routing across selected Surfari browsers A, B, and C without leaking to inactive Surfari tabs/windows.                   | Already proven for selected Surfari panes/tabs/windows; broader app activation/deactivation outside TermSurf remains separate.        | `scripts/test-issue-756-surfari-tab-window-focus-geometry.sh`.  |
| Shutdown           | Proven  | Experiments 20, 22, 23, and 25 proved direct `CloseTab`, tab removal, no-tabs-remaining shutdown, and clean guard shutdown.                                                                                           | Already proven for single-tab close/no-tabs-remaining shutdown; crash handling is separate.                                           | `scripts/test-issue-756-surfari-lifecycle-tranche.sh`.          |
| Restart            | Proven  | Experiment 25 run `20260621-190346` closed the first Surfari tab/process path, relaunched TermSurf, saw a fresh Surfari trace init, `BrowserReady`, overlay presentation, fixture A creation, and WebTUI title state. | Already proven for same-fixture app relaunch after clean shutdown; crash restart and profile isolation remain separate.               | `scripts/test-issue-756-surfari-lifecycle-tranche.sh`.          |
| Profile isolation  | Missing | No Surfari profile isolation evidence yet.                                                                                                                                                                            | Prove separate profiles maintain separate localStorage/cookies/state and route to the correct Surfari process.                        | Profile tranche adapted from Roamium `multi-profile-isolation`. |
| Crash handling     | Missing | No Surfari renderer/process crash evidence yet.                                                                                                                                                                       | Prove renderer/process crash notification, UI state, cleanup, and restart behavior.                                                   | Crash tranche adapted from Roamium `renderer-crash-smoke`.      |
| Roamium comparison | Missing | No full Ghostboard/Roamium feature-matrix comparison has been rerun against Ghostboard/Surfari yet.                                                                                                                   | Re-run each comparable Roamium matrix row against Surfari, mark parity/difference/unsupported, and document engine-specific gaps.     | Final comparison tranche after Surfari real-app rows stabilize. |

## Roamium Scenario Map

The existing `scripts/ghostboard-geometry-matrix.sh` is Roamium-oriented and too
broad to reuse wholesale for Surfari. The relevant scenario names to mine are:

- Lifecycle/navigation/resize: `browser-command-navigation`, `window-resize`,
  `browser-navigation-geometry`.
- Pane and split geometry: `split-right`, `split-down`, `split-right-resize`,
  `split-right-equalize`, `split-right-zoom`, `split-right-close-sibling`,
  `split-right-close-browser-pane`.
- Tabs/windows/focus: `new-terminal-tab-visibility`, `open-browser-in-new-tab`,
  `close-browser-tab`, `open-browser-in-new-window`,
  `multiple-windows-with-browsers`, `keyboard-after-tab-window-switch`,
  `gui-active-multi-tab`.
- Input details: `browser-input-granularity`, `mouse-after-geometry-change`.
- Profiles/lifecycle/crash: `multi-profile-isolation`,
  `same-profile-server-lifecycle`, `tui-disconnect-reconnect`,
  `renderer-crash-smoke`.

Surfari experiments should reuse the assertions and fixtures from these
scenarios where practical, but they should not require Roamium-specific paths or
trace names. Surfari logs currently use `surfari-trace` files and
WebKit-specific callbacks.

## Recommended Tranches

1. **Lifecycle/navigation/resize/shutdown/restart.** Extend the existing Surfari
   smoke harness to prove explicit navigation after initial load and restart
   after close. This should also preserve the existing resize and shutdown
   proof.
2. **Pane/split/tab/window/focus geometry.** Add Surfari-specific variants of
   the core geometry scenarios: split right/down, pane resize, tab visibility,
   window attachment, and active/inactive focus routing. Experiments 26 and 27
   completed this tranche.
3. **Input details.** Keep the existing keyboard/wheel guard as baseline, then
   add click, drag, and coordinate-fidelity checks. Experiment 28 proved DOM
   single click and browser text drag selection. Broader click-count parity and
   selection copy parity remain separate from the matrix rows proven here.
4. **Profile isolation and crash handling.** Prove profile storage separation
   and Surfari crash/restart behavior after the normal lifecycle and geometry
   rows are stable.
5. **Ghostboard/Roamium comparison.** Re-run the comparable Roamium matrix
   against Surfari, record feature parity, and document engine-specific
   differences.

## Next Experiment Recommendation

Experiment 28 completed the click/drag portion of the input-details tranche. The
next experiment should move to profile isolation or crash handling without
mixing in the final Roamium comparison unless the previous result specifically
calls for it.
