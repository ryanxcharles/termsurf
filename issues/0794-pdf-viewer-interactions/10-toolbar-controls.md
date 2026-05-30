# Experiment 10: Prove Safe PDF Toolbar Controls

## Description

Experiments 4-9 proved the core viewer interaction plumbing:

- wheel scrolling reaches the PDF viewer;
- mouse routing reaches Chromium's input router;
- keyboard select/copy reaches the focused PDF widget;
- mouse drag selection works when the drag starts on PDF text;
- direct protocol resize reaches PDFium;
- real Wezboard split-pane resize reaches PDFium.

The remaining Issue 794 surface is now the browser/PDF-viewer feature surface:
toolbar controls, save/download, print, title propagation, and local-file
parity. Experiment 10 should start with the safe in-page toolbar controls that
should not open native dialogs:

- zoom in / zoom out;
- fit controls;
- rotate;
- page navigation / page selector.

Experiment 1 only detected toolbar controls. That is not enough. Experiment 10
must click controls and prove that the viewer state changes. Save/download and
print are only probed if the harness can contain side effects inside a
controlled output directory and prevent native dialogs. Otherwise, record them
as the next experiment target.

This experiment must receive Codex design review before implementation. After
the result is recorded, Codex must review the completed output before the next
experiment is designed.

## Changes

1. Add an automated toolbar-control harness.

   Prefer a new script such as `scripts/test-issue-794-pdf-toolbar.py` that
   reuses the direct Roamium/fake-GUI startup pattern from the existing Issue
   794 protocol harnesses:
   - start a local HTTP fixture server for `test-html/public/bitcoin.pdf`;
   - launch repo-built debug Roamium from `chromium/src/out/Default/roamium`;
   - send `CreateTab` and the initial `Resize` over the fake TermSurf socket;
   - discover the DevTools port from Roamium logs;
   - attach to the top-level PDF target and all child targets through CDP;
   - write all artifacts under `logs/issue-794-exp10-toolbar-*`.

   Do not rely on the installed app or installed Roamium. Do not use network
   `bitcoin.org` during the test.

2. Add a robust PDF toolbar DOM inspector.

   The harness should traverse open shadow roots from the top-level document and
   attached child targets. It should record each candidate control with:
   - target id/session;
   - shadow path or enough selector data to re-find it;
   - tag, id, class, role, aria label, title, text/value;
   - bounding rect;
   - disabled/hidden state.

   Control matching should use semantic labels first (`aria-label`, `title`,
   role, id), not brittle child indexes. If a known Chromium PDF control cannot
   be found semantically, record the full candidate inventory and mark the
   missing control explicitly.

3. Click safe controls and prove state changes.

   For each safe control, capture before/after state and screenshots. A click is
   successful only if the harness observes a relevant state change, not merely
   that CDP accepted the click.

   Use one activation method consistently and record it. Prefer DOM activation
   in the target/session that owns the control (`Runtime.callFunctionOn` or an
   equivalent evaluated helper that re-finds the semantic control and invokes
   `click()` / `input` / `change`). This avoids mixing top-level and child-frame
   coordinate spaces while testing the PDF viewer's own control behavior. If the
   implementation uses mouse coordinates instead, it must record the coordinate
   space, target/session, rect, and a post-click signal proving the intended
   control, not merely the page, received activation.

   Required checks:
   - **Zoom in:** click the zoom-in control. Pass only if zoom text/value,
     plugin geometry, page rendered size, or another semantic viewer state
     changes in the zoom-in direction. A whole-viewport screenshot diff is
     supporting evidence only; it is not enough by itself.
   - **Zoom out:** click the zoom-out control after zoom-in. Pass only if state
     changes back in the zoom-out direction.
   - **Rotate:** click rotate. Pass only if the page aspect/bitmap changes or a
     viewer rotation state changes.
   - **Fit control:** click the fit control or select a fit mode if exposed.
     Pass only if the fit mode label/state, zoom, or page rendered size changes.
   - **Page navigation:** use the Bitcoin PDF fixture as the multi-page PDF.
     Click next-page or set the page selector to a later page. Pass only if the
     page indicator, scroll position, visible page text/page number, or
     page-area-scoped screenshot diff proves navigation to a later page. Then
     navigate back and prove it returns. A whole-viewport screenshot diff is
     supporting evidence only; it is not enough by itself.

   If a control is present but disabled, record whether that disabled state is
   expected for the current document/page. For example, previous-page is
   expected to be disabled on page 1 before navigating forward.

4. Keep save/download and print side-effect safe.

   The harness may probe save/download only if it first installs controlled
   download behavior through CDP, such as `Browser.setDownloadBehavior`, with a
   per-run downloads directory under the log directory. A save/download probe
   passes only if:
   - clicking the viewer's download/save control produces a file in that
     directory; and
   - the file size or hash matches the fixture PDF, or the result records the
     exact browser-side path that produced a different expected file.

   If clicking save/download would open a native panel or write outside the
   controlled directory, do not click it. Record `save-download-not-contained`
   and design the next experiment around the missing containment/browser helper.

   Do not click the print control unless the harness can prove it will not open
   a native print dialog. If print cannot be safely contained, record
   `print-not-contained` and leave print to a dedicated follow-up experiment.

5. Record title and local-file observations, but do not expand scope.

   During the run, record:
   - top-level target title;
   - PDF extension child target title;
   - webtui/Wezboard title-related log lines if available;
   - whether toolbar state exposes the document title.

   This experiment does not need to fix title propagation or local `file://`
   parity. If those remain unproven, list them as follow-up targets in the
   conclusion.

6. Preserve existing working behavior.

   After toolbar probing, run a small regression set:
   - PDF wheel scroll still changes state or screenshot;
   - PDF keyboard select-all/copy still copies non-empty text after toolbar
     focus has moved, preserving the Experiment 6 path;
   - PDF drag selection still copies non-empty text using the successful
     Experiment 7 geometry;
   - normal HTML scroll/click still works with the same Roamium binary.

   Reuse existing scripts where practical instead of duplicating all protocol
   message code.

7. Formatting and review.

   If the experiment changes Markdown, run Prettier:

   ```bash
   prettier --write --prose-wrap always --print-width 80 \
     issues/0794-pdf-viewer-interactions/README.md \
     issues/0794-pdf-viewer-interactions/10-toolbar-controls.md
   ```

   If the experiment changes Rust, run `cargo fmt` and accept its output. This
   experiment should not need Rust changes unless the toolbar test proves a
   product bug.

## Verification

1. Run the toolbar harness against the local Bitcoin PDF fixture.

   Required artifacts:
   - command log;
   - Roamium stdout/stderr;
   - HTTP fixture server log;
   - toolbar control inventory JSON;
   - before/after JSON and screenshots for each clicked control;
   - summary JSON with pass/partial/fail status per control.

2. Required pass evidence for each safe control:
   - target/session that received the click;
   - exact semantic selector or recorded control identity;
   - before/after viewer state;
   - before/after screenshot path;
   - concrete semantic changed field proving the control worked, with screenshot
     diffs as supporting evidence only for the safe controls.

3. Required result table:

   | Feature           | Control found | Clicked | State changed | Evidence | Status |
   | ----------------- | ------------- | ------- | ------------- | -------- | ------ |
   | Zoom in           |               |         |               |          |        |
   | Zoom out          |               |         |               |          |        |
   | Rotate            |               |         |               |          |        |
   | Fit mode          |               |         |               |          |        |
   | Page next         |               |         |               |          |        |
   | Page previous     |               |         |               |          |        |
   | Save/download     |               |         |               |          |        |
   | Print             |               |         |               |          |        |
   | Title observation |               |         |               |          |        |

4. Regression checks:
   - PDF wheel scroll still passes.
   - PDF keyboard select-all/copy still passes.
   - PDF drag selection still passes.
   - HTML interaction smoke test still passes.

5. Codex must review the completed output.

   Do not proceed to Experiment 11 until real issues from Codex's completion
   review are addressed.

## Pass Criteria

Experiment 10 passes if:

- zoom in, zoom out, rotate, fit mode, and page navigation are all proven
  functional with semantic state changes;
- save/download and print are either safely contained and tested or explicitly
  classified as not safely containable for a follow-up;
- the regression checks still pass;
- the result identifies the exact next remaining Issue 794 target.

## Partial Criteria

Experiment 10 is partial if:

- the harness can inventory controls but cannot reliably click through shadow
  DOM;
- one or more safe controls are present and clickable but produce no observable
  state change;
- one or more required safe controls are missing, unless the missing or disabled
  state is expected for the current document/page and an alternate path proves
  the same feature;
- save/download or print containment is attempted but the result is ambiguous
  rather than clearly contained/tested or clearly classified as not containable;
- regression checks pass but a remaining toolbar feature requires a product
  change.

## Failure Criteria

Experiment 10 fails if:

- it only detects controls without clicking and proving behavior;
- it relies on visual inspection without objective state/screenshot evidence;
- it opens native save or print dialogs during automation;
- it writes downloads outside the per-run log directory;
- it uses the installed/stable Roamium instead of the repo-built debug Roamium;
- it omits Codex design or completion review.

## Result

**Result:** Partial

Primary toolbar run: `logs/issue-794-exp10-toolbar-20260530-102243`.

Implementation artifacts:

- Added `scripts/test-issue-794-pdf-toolbar.py`, a direct Roamium/fake-GUI
  launcher for the local Bitcoin PDF fixture.
- Added `scripts/probe-pdf-toolbar.mjs`, a CDP toolbar probe that attaches to
  the top-level PDF target and PDF extension iframe target, traverses open
  shadow roots, inventories controls, activates safe controls in their owning
  target/session, captures before/after screenshots, and writes semantic state
  diffs.

Validation:

- `python3 -m py_compile scripts/test-issue-794-pdf-toolbar.py` passed.
- `node --check scripts/probe-pdf-toolbar.mjs` passed.
- Prettier ran on `scripts/probe-pdf-toolbar.mjs`.

Toolbar result table:

| Feature           | Control found                 | Clicked                                                           | State changed                             | Evidence                                                                                                                                                                                | Status    |
| ----------------- | ----------------------------- | ----------------------------------------------------------------- | ----------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------- |
| Zoom in           | Yes                           | Yes, CDP mouse in PDF extension child target                      | No semantic zoom/page geometry change     | `toolbar/zoomIn.json`; screenshots changed only by toolbar ripple/focus; zoom text stayed `36%`                                                                                         | Partial   |
| Zoom out          | Yes                           | Yes, CDP mouse in PDF extension child target                      | No semantic zoom/page geometry change     | `toolbar/zoomOut.json`; zoom text stayed `36%`                                                                                                                                          | Partial   |
| Rotate            | Yes                           | Yes, CDP mouse in PDF extension child target                      | No semantic rotation/page geometry change | `toolbar/rotate.json`; page geometry and zoom text stayed unchanged                                                                                                                     | Partial   |
| Fit mode          | Yes                           | Yes, CDP mouse in PDF extension child target                      | Yes                                       | `toolbar/fit.json`; zoom text changed from `36%` to `28%`                                                                                                                               | Pass      |
| Page next         | Yes, via `pageSelector` input | Yes, DOM page-selector value change in PDF extension child target | Yes                                       | `toolbar/pageNext.json`; page selector changed from `1` to `2`; `toolbar/pageNext-before.png` shows page `1`, and `toolbar/pageNext-after.png` shows page `2`                           | Pass      |
| Page previous     | Yes, via `pageSelector` input | Yes, DOM page-selector value change in PDF extension child target | Yes                                       | `toolbar/pagePrevious.json`; page selector changed back toward page `1`; `toolbar/pagePrevious-before.png` shows page `2`, and `toolbar/pagePrevious-after.png` returns toward page `1` | Pass      |
| Save/download     | Yes                           | No                                                                | Not tested                                | `toolbar/toolbar-summary.json`; classified `save-download-not-contained` to avoid native dialog/uncontrolled writes                                                                     | Follow-up |
| Print             | Yes                           | No                                                                | Not tested                                | `toolbar/toolbar-summary.json`; classified `print-not-contained` to avoid native print dialog                                                                                           | Follow-up |
| Title observation | N/A                           | N/A                                                               | Observed                                  | top-level title empty; PDF extension child target title `bitcoin.pdf`                                                                                                                   | Follow-up |

Important screenshots:

- `toolbar/zoomIn-before.png` and `toolbar/zoomIn-after.png` prove the zoom-in
  button receives visible activation/ripple, but the page does not zoom.
- `toolbar/fit-before.png` and `toolbar/fit-after.png` prove the fit button
  changes the PDF viewer state.

Regression checks:

| Check                    | Log                                                     | Result                                                                                                                                                                                                                                 |
| ------------------------ | ------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| PDF wheel scroll         | `logs/issue-794-exp10-wheel-regression-20260530-102423` | Pass; `first_failing_hop=no-failure-observed`, screenshot changed, Roamium FFI line present                                                                                                                                            |
| PDF keyboard select/copy | `logs/issue-794-exp10-key-regression-20260530-102423`   | Pass; `first_failing_hop=no-failure-observed`, clipboard copied `21230` bytes                                                                                                                                                          |
| PDF drag selection       | `logs/issue-794-exp10-drag-regression-20260530-102423`  | Pass by sweep result; `drag_sweep_selected=true` and clipboard copied `21230` bytes. The older classifier still reports `pdfium-not-text-area` from an attempted drag path, so the sweep result is the authoritative pass signal here. |
| HTML click smoke         | `logs/issue-794-exp10-html-click-20260530-102508`       | Pass; `first_failing_hop=no-failure-observed`, state and screenshot changed                                                                                                                                                            |

No native save or print dialog was opened. No download was written outside the
run log directory.

## Conclusion

Experiment 10 proves the toolbar surface is only partially functional:

- The PDF toolbar exists and is reachable through the PDF extension child
  target.
- Fit mode works.
- Page selector navigation works.
- Save/download and print controls exist but remain deliberately untested until
  a contained browser-side path is designed.
- Zoom in, zoom out, and rotate are present and visibly receive CDP mouse
  activation, but the PDF viewer state does not change.

The next experiment should target the PDF toolbar event/action path for
zoom/rotate specifically. Chromium's PDF source shows the intended path: these
controls should dispatch `zoom-in`, `zoom-out`, and `rotate-left` events from
`viewer-toolbar.ts` to `pdf_viewer_base.ts`. Experiment 10 proves only that the
controls receive CDP mouse interaction/ripple; it does not prove those custom
events fire. The next diagnostic should instrument or probe that path:

```text
cr-icon-button click
  -> viewer-toolbar onZoomInClick_ / onZoomOutClick_ / onRotateClick_
  -> custom event on <viewer-toolbar>
  -> <pdf-viewer> handler in pdf_viewer.html / pdf_viewer_base.ts
  -> viewport_.zoomIn()/zoomOut() or currentController.rotateCounterclockwise()
  -> PDF plugin/PDFium geometry or rotation update
```

Do not keep adjusting CDP coordinate mechanics unless that event-chain trace
shows the click never reaches `viewer-toolbar`. The current screenshots already
show the button receives pointer activation; the missing piece is whether the
toolbar custom event and viewer action run.
