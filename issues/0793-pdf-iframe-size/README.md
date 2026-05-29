+++
status = "closed"
opened = "2026-05-29"
closed = "2026-05-29"
+++

# Issue 793: PDF viewer iframe renders at default size

## Goal

PDFs should render at normal full-pane size in Roamium. A PDF URL such as
`https://bitcoin.org/bitcoin.pdf` or the local `bitcoin.pdf` fixture should fill
the webview the way Chrome's built-in PDF viewer does, not appear inside a tiny
default-sized iframe in the upper-left corner.

## Background

Issue 792 delivered inline PDF rendering through the TermSurf Chromium embedder.
The first visible PDF proof was successful: the Bitcoin paper rendered inline
instead of producing a blank page, a download, or a missing-plugin surface.

After manual testing, the remaining presentation bug is clear: the rendered PDF
content is constrained to a tiny rectangle near the top-left of an otherwise
full-size white page. The same behavior appears for both the local fixture and
`https://bitcoin.org/bitcoin.pdf`, so the problem is not specific to the test
server or URL.

The user-provided `screenshot5.png` shows:

- the TermSurf webview area is full size;
- the outer PDF document/background is full size;
- the actual PDF iframe/content is much smaller, roughly browser default iframe
  size;
- URL fragments such as `#zoom=page-width` or `#zoom=200` do not change the
  frame size.

This means the bug is not PDF zoom. The actual iframe that hosts the PDF content
is failing to size itself to the viewport.

## Analysis

Chromium's PDF wrapper relies on `pdf_embedder.css` to make the wrapper iframe
fill the full page:

```css
iframe {
  position: absolute;
  left: 0;
  top: 0;
  height: 100%;
  width: 100%;
  border-style: none;
}
```

Without that CSS, a normal HTML `<iframe>` falls back to its default dimensions,
which matches the observed tiny PDF rectangle.

The likely TermSurf-specific cause is in the PDF component extension setup from
Issue 792. Chrome's PDF component manifest includes:

```json
"web_accessible_resources": ["pdf_embedder.css"]
```

TermSurf's `CreateTsPdfComponentExtension()` currently strips
`web_accessible_resources` while simplifying the PDF extension manifest. That
was enough to unblock rendering during Issue 792, but it likely prevents the
top-level PDF wrapper document from loading the extension CSS. The result is a
functional PDF plugin inside an incorrectly sized iframe.

## Proposed Direction

Fix this in Chromium/Roamium, not in webtui or Wezboard. The sizing belongs to
the PDF wrapper and extension-resource layer that Issue 792 added.

The likely fix is to preserve or correctly reconstruct the PDF component
extension's `web_accessible_resources` entry for `pdf_embedder.css`, then verify
that:

- `pdf_embedder.css` is actually requested and served;
- the wrapper iframe receives `position: absolute`, `width: 100%`, and
  `height: 100%`;
- the PDF content fills the pane for both local and remote PDF URLs;
- Issue 792's inline rendering path still works.

## Constraints

- Do not reopen or modify Issue 792; it is closed and immutable.
- Do not treat this as a PDF zoom problem.
- Do not change webtui, Wezboard, Roamium Rust, or the TermSurf protocol unless
  Chromium-side CSS/resource fixes prove insufficient.
- Preserve Issue 792's successful inline PDF rendering.
- Every experiment design and completion must receive Claude review before
  proceeding.

## Experiments

- [Experiment 1: Restore PDF embedder CSS access](01-restore-pdf-embedder-css.md)
  — **Pass** (local and remote PDFs now render in a full-size PDF viewer)

## Conclusion

Issue 793 is closed as solved. PDFs no longer render inside a tiny default-sized
iframe; local and remote PDF URLs now render in the normal full-pane Chrome PDF
viewer layout.

The decisive proof is Experiment 1:

- Chromium branch `148.0.7778.97-issue-793-exp1`
- Chromium commit `1277ba9cabd4408a944825ca6d61145fa941d04c`
  (`Restore PDF embedder CSS`)
- Patch archive regenerated at `chromium/patches/issue-793/`
- Local PDF capture:
  `logs/issue-793-exp1-local-devtools-20260529-181248/devtools-smoke.png`
- Remote PDF capture:
  `logs/issue-793-exp1-remote-devtools-20260529-181451/devtools-smoke.png`

Both PDF captures show Chrome's normal PDF toolbar, dark viewer background, and
a full-height Bitcoin paper page. Neither capture shows the tiny upper-left
iframe from `screenshot5.png`. The HTML sanity capture at
`logs/issue-793-exp1-html-devtools-20260529-181556/devtools-smoke.png` also
confirmed normal non-PDF rendering still works.

The root cause was exactly the over-aggressive manifest simplification from
Issue 792: `LoadPdfManifest()` stripped the Manifest V2
`web_accessible_resources` key, which blocked the top-level PDF wrapper document
from loading `chrome-extension://.../pdf_embedder.css`. Without that stylesheet,
the iframe fell back to browser default dimensions. Preserving
`web_accessible_resources: ["pdf_embedder.css"]` restored the cross-origin
extension resource load, and the existing CSS now sizes the wrapper iframe to
the full viewport.

No remaining limitations are known within this issue's scope.
