# Experiment 27: Run Surfari tab, window, and focus geometry

## Description

Experiment 26 proved pane and split geometry. The remaining geometry gap is
whether Surfari overlays stay attached to the correct terminal surface across
native tab switching, new windows, and focus changes.

This experiment should cover the `Tab switching`, `Window switching`, and
`Focus changes` rows in the real-app matrix. It should not expand into click DOM
parity, drag, profile isolation, crash handling, or the final Ghostboard/Roamium
comparison.

The implementation should reuse the behavior and assertions from the existing
Roamium geometry scenarios in `scripts/ghostboard-geometry-matrix.sh`,
especially:

- `new-terminal-tab-visibility`;
- `open-browser-in-new-tab`;
- `open-browser-in-new-window`;
- `keyboard-after-tab-window-switch`;
- `gui-active-multi-tab`.

The new work should remain Surfari-specific and focused. Prefer adding a new
Surfari harness, or extending the existing Surfari pane/split harness only if
that keeps the scenarios independent and readable. Do not mutate the large
Roamium runner unless a shared helper bug is found and the fix benefits both
engines.

## Changes

- Add or extend a focused Surfari tab/window/focus geometry harness under
  `scripts/`.
- Launch the real Debug `TermSurf.app` with repo-built `web --browser surfari`
  and repo-built `surfari`, using deterministic local fixtures.
- Run independent real-app scenarios for:
  - switching away from a browser tab to a plain terminal tab and back;
  - opening a second browser in a new native tab and switching between browser
    tabs;
  - opening a second window and proving a browser opened there attaches to that
    window, not the original;
  - focus changes across active Surfari tabs/windows, including browse/control
    mode transitions and GUI active/inactive state where practical.
- For tab switching, prove:
  - the original browser overlay is not visible or hit-testable on the selected
    plain terminal tab;
  - switching back restores the original Surfari overlay frame and hit testing;
  - keyboard input in the plain terminal tab does not reach Surfari;
  - keyboard input after returning to browse mode reaches the owning Surfari
    tab.
- For a second browser in a new tab, prove:
  - browser A and browser B have distinct pane IDs, browser tab IDs, and context
    IDs;
  - browser A remains hidden while browser B's tab is selected;
  - browser B hit testing and keyboard input route only to browser B;
  - switching back restores browser A hit testing and keyboard input without
    leaking to browser B.
- For window switching, prove:
  - a new native window is created;
  - a browser opened in the new window presents on that window only;
  - hit testing and keyboard input route to the browser in the active window and
    do not route to a browser in another window.
- For focus changes, prove active/inactive state and keyboard routing change
  only for the selected Surfari tab/pane/window, reusing the Roamium
  `gui-active-multi-tab` and `keyboard-after-tab-window-switch` assertions where
  possible.
- Update `issues/0756-surfari/real-app-matrix.md` only for directly proven rows:
  - mark `Tab switching` `Proven` only if tab visibility, restore, hit testing,
    and keyboard routing pass;
  - mark `Window switching` `Proven` only if browser window attachment and
    routing isolation pass;
  - mark `Focus changes` `Proven` only if active/inactive state and keyboard
    routing are proven across the selected Surfari surfaces.

## Verification

Pass criteria:

- Required builds/artifacts exist:

```bash
surfari/libtermsurf_webkit/build.sh
cargo build -p surfari
cargo build -p webtui
cd ghostboard && zig build
```

- Run the new tab/window/focus geometry harness.
- The harness must prove, in the real app, tab visibility/restore behavior,
  browser-in-new-tab isolation, browser-in-new-window attachment, and focus
  routing.
- The harness must fail if:
  - the original Surfari overlay appears in a non-owning tab or window;
  - hit testing routes to the wrong Surfari context;
  - keyboard input reaches an inactive Surfari tab/window;
  - active/inactive state is sent to the wrong Surfari tab/pane.
- Update `real-app-matrix.md` only for rows directly proven by this experiment.
- Run hygiene checks:

```bash
git diff --check
bash -n <new-or-updated-tab-window-focus-harness>
prettier --check --prose-wrap always --print-width 80 \
  issues/0756-surfari/README.md \
  issues/0756-surfari/27-surfari-tab-window-focus-geometry.md \
  issues/0756-surfari/real-app-matrix.md
```

Result classification:

- `Pass` means tab switching, window switching, and focus changes are all
  directly proven in the real app, allowing all three rows to become `Proven`.
- `Partial` means at least one of tab switching, window switching, or focus
  changes is proven, but one or more remains unproven.
- `Fail` means the harness cannot launch Surfari or cannot produce stronger
  tab/window/focus evidence than the existing matrix.

## Design Review

Adversarial design review returned `APPROVED` with no findings. The reviewer
confirmed that the plan is still uncommitted and that read-only
`git diff --check` and Prettier checks passed.

## Result

**Result:** Pass

Run `20260621-194431` passed with the new real-app harness:

```bash
bash -n scripts/test-issue-756-surfari-tab-window-focus-geometry.sh
scripts/test-issue-756-surfari-tab-window-focus-geometry.sh
```

Logs:

- `logs/issue-756-exp27-surfari-tab-window-focus/harness-20260621-194431.log`
- `logs/issue-756-exp27-surfari-tab-window-focus/app-20260621-194431.log`
- `logs/issue-756-exp27-surfari-tab-window-focus/surfari-trace-20260621-194431.log`
- `logs/issue-756-exp27-surfari-tab-window-focus/webtui-20260621-194431.log`

The harness launched the real Debug `TermSurf.app` with repo-built
`web --browser surfari` and repo-built `surfari`, then proved:

- browser A was hidden and not hit-testable after switching to a plain terminal
  tab;
- keyboard input in the plain terminal tab did not reach Surfari;
- switching back restored browser A hit testing and keyboard routing;
- browser B opened in the second native tab with a distinct pane ID, browser tab
  ID, and CA context ID;
- browser A stayed hidden while browser B's tab was selected;
- browser B hit testing and keyboard input routed only to browser B;
- switching back restored browser A hit testing and keyboard input without
  leaking to browser B;
- a new native window was created and browser C opened there with a distinct
  pane ID, browser tab ID, and CA context ID;
- browser C presented on the new window, not the original window;
- browser C hit testing and keyboard input routed only to browser C;
- browse/control mode transitions produced Surfari focus true/false for the
  selected pane and keyboard events reached only the selected Surfari browser.

One harness assertion was corrected during implementation: tab restoration does
not always emit a fresh AppKit `presented` line because the existing overlay
view can simply become visible again when the native tab is selected. The final
harness therefore proves restoration by reusing the known tab-adjusted frame and
requiring a fresh hit test plus keyboard routing on the restored tab.

## Conclusion

Surfari now has real-app evidence for tab switching, window switching, and focus
routing across selected panes/tabs/windows. The real-app matrix marks those rows
`Proven`. Remaining Issue 756 matrix gaps are click DOM proof, drag, profile
isolation, crash handling, and the final Ghostboard/Roamium comparison.

## Completion Review

Adversarial completion review returned `APPROVED` with no Required findings. The
reviewer had one Optional finding: the harness claimed browser C used a distinct
browser tab ID, but the script did not explicitly assert `C_TAB_ID` differed
from browser A and B. The harness was updated to assert distinct C tab IDs, to
select browser C's CA context while excluding both browser A and B panes, and to
scope the browser-A-in-window-C negative grep to the C window identity.

The focused re-review returned `APPROVED`; it confirmed the optional finding was
resolved and that no new Required findings were introduced.
