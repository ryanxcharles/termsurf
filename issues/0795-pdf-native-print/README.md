+++
status = "closed"
opened = "2026-05-30"
closed = "2026-06-22"
+++

# Issue 795: PDF Native Print

## Goal

Make the PDF viewer's print button open usable native print UI from TermSurf's
Chromium/Roamium embedder path.

## Background

Issue 794 completed the interactive PDF viewer surface but deliberately deferred
native PDF printing. The print path is partially wired:

- the PDF toolbar print control is visible;
- the viewer JavaScript posts the print message;
- `PdfViewWebPlugin::Print()` is reached;
- contained print automation can intercept the click without opening native UI;
- Experiment 20 installed Chromium's renderer-side `PrintRenderFrameHelper`;
- after Experiment 20, clicking print no longer turns the PDF viewport gray.

The remaining failure is narrower: clicking the PDF print button still does not
show native print UI.

## Analysis

The likely remaining missing layer is browser-side printing infrastructure,
especially the `PrintManagerHost` / `PrintViewManager` path that Chrome wires
for WebContents. TermSurf currently embeds Chromium through Roamium and
content-shell-derived code, not the full Chrome browser layer.

The next work should begin from Issue 794 Experiment 20's trace points, not from
the toolbar. The toolbar and PDF plugin entry path are already proven.

## Constraints

- Do not reopen Issue 794.
- Do not block basic PDF viewing on this issue.
- Do not submit a real printer job during automation.
- Do not let automated tests click production print unless the print path is
  contained or explicitly mocked.
- If Chromium is modified, create a fresh Chromium branch for this issue and add
  it to `chromium/README.md`.
- Every experiment design and every completed experiment result must be reviewed
  by Codex. Fix real issues from the review before proceeding.

## Conclusion

This issue is superseded by
[Issue 834: Full PDF Support Across Roamium and Surfari](../0834-full-pdf-support-roamium-surfari/README.md).

Native PDF printing remains required, but it should now be solved as part of the
unified cross-engine PDF matrix rather than as a standalone Roamium-only issue.
