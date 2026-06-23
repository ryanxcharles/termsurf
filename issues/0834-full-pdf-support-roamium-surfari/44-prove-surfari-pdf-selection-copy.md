# Experiment 44: Prove Surfari PDF Text Selection and Copy

## Description

Experiment 43 proved Surfari PDF internal and external links through real
TermSurf-routed mouse input. The next Surfari matrix slice should prove text
selection and clipboard copy from a PDF.

This experiment should test Surfari only. It should start as a probe and make no
product changes unless the probe exposes a real TermSurf integration gap. The
goal is not to prove arbitrary PDF text extraction through WebKit internals; the
goal is to prove the user workflow: visible PDF text can be selected with real
mouse input inside Ghostboard and copied with real keyboard input through
Surfari's PDF surface.

## Changes

- Add a focused harness, tentatively
  `scripts/test-issue-834-surfari-pdf-selection-copy.sh`.
- Generate a deterministic PDF fixture inside the harness with real PDF text
  operators, not a raster image. The fixture must contain a unique ASCII marker
  such as `ISSUE834_SURFARI_PDF_COPY_SENTINEL`. The fixture metadata must define
  the exact accepted copied substring before the run starts. The accepted
  substring must be a unique, substantial part of the marker and must not appear
  in the URL, shell command, log labels, HTML source, or surrounding non-PDF
  text.
- Serve the PDF as `application/pdf` from a deterministic local server.
- Launch repo-built Ghostboard with repo-built WebTUI and repo-built Surfari. Do
  not use installed artifacts.
- Run with `TERMSURF_SURFARI_CACONTEXT_LAYER` unset so the test proves the
  default Surfari presentation path.
- Establish the baseline:
  - WebTUI requested `browser=surfari`;
  - Surfari emitted `BrowserReady`;
  - WebTUI reached ready state;
  - Surfari trace recorded the PDF URL;
  - Surfari emitted nonzero CAContext;
  - Surfari internal render proof passed;
  - Ghostboard-window overlay-cropped visible proof shows the PDF page and the
    intended selectable text region, or records a concrete reason why text
    pixels cannot be isolated while still proving the generated PDF text
    operator coordinates and target rectangle;
  - the fixture server recorded the PDF request with `application/pdf`.
- Enter Browse mode and focus Surfari through the same path used by the Surfari
  PDF input harnesses.
- Protect the system clipboard:
  - save the pre-existing clipboard contents before writing any sentinel;
  - record the pre-existing clipboard length and hash, not the full contents;
  - record pasteboard `changeCount` or an equivalent macOS pasteboard change
    indicator before sentinel write, after sentinel write, and after the copy
    attempt;
  - prime the system clipboard with a unique sentinel value using `pbcopy`;
  - verify `pbpaste` returns that sentinel before the copy attempt;
  - restore the pre-existing clipboard contents during cleanup and record
    restoration status.
- Select text through real TermSurf-routed mouse input:
  - compute global drag coordinates from the Ghostboard overlay frame and the
    known fixture text rectangle;
  - record the generated PDF text operator coordinates, PDF-space text bounding
    box, overlay frame/crop, web-space drag start/end points, and computed
    global drag start/end points;
  - capture a post-drag overlay screenshot and record whether a visible
    selection highlight can be detected; if WebKit/PDFKit does not expose a
    stable highlight, record that explicitly and rely on clipboard plus input
    routing evidence for the behavioral proof;
  - send a real drag with `scripts/ghostty-app/inject.swift`;
  - require Ghostboard geometry/input evidence for mouse down, move, and up;
  - require Surfari trace evidence for mouse down, move, and up;
  - do not use WebKit internals, JavaScript DOM selection, direct clipboard
    mutation after the copy attempt, or direct Surfari state mutation to create
    the selection.
- Copy through real keyboard input:
  - send Browse-mode `Cmd+C` through `scripts/ghostty-app/inject.swift`;
  - require matched Ghostboard input evidence that the key was forwarded to the
    Surfari overlay and was not handled as terminal selection or
    copy-current-url fallback;
  - require Surfari trace evidence for the key event if Surfari exposes it, and
    otherwise record the absence of Surfari key trace as an evidence limitation
    rather than silently ignoring it;
  - read the clipboard with `pbpaste`.
- Pass only if the clipboard changes away from the pre-copy sentinel and
  contains the unique PDF fixture marker or the predefined accepted marker
  substring if WebKit/PDFKit normalizes whitespace while preserving the marker.
- Record a JSON summary with:
  - env state proving `TERMSURF_SURFARI_CACONTEXT_LAYER` was unset;
  - repo binary paths;
  - PDF URL request/content-type evidence;
  - fixture text marker and expected copied substring;
  - generated PDF text operator coordinates, PDF-space text rectangle, and
    overlay/frame mapping evidence;
  - global drag and key input coordinates/events;
  - matched Ghostboard, WebTUI, Surfari, and server evidence lines;
  - clipboard original/sentinel/after-copy/restored lengths and hashes,
    pasteboard change indicators, restoration status, and a bounded sample of
    copied fixture text;
  - pass/partial/fail classification;
  - cleanup status for Ghostboard, Surfari, WebTUI, and the fixture server.
- Update this experiment file with the result.

## Verification

Run build and hygiene checks:

```bash
./surfari/libtermsurf_webkit/build.sh
cargo fmt -p surfari
cargo build -p surfari
cargo build -p webtui
(cd ghostboard && macos/build.nu --configuration Debug --action build)
bash -n scripts/test-issue-834-surfari-pdf-selection-copy.sh
git diff --check
git -C webkit/src status --short
```

Run the selection/copy harness:

```bash
rm -rf logs/issue-834-exp44-surfari-pdf-selection-copy
env -u TERMSURF_SURFARI_CACONTEXT_LAYER \
  scripts/test-issue-834-surfari-pdf-selection-copy.sh
```

Pass criteria:

- `TERMSURF_SURFARI_CACONTEXT_LAYER` is unset;
- all scenarios use repo-built Ghostboard, WebTUI, Surfari, and WebKit
  artifacts;
- the PDF request is recorded with `application/pdf`;
- Surfari internal render proof passes before selection;
- baseline Ghostboard-window overlay-cropped visible proof shows the PDF page;
- the result records the generated PDF text operator coordinates, PDF-space text
  bounding box, and expected accepted substring before selection begins;
- selection uses real TermSurf-routed mouse drag input and produces Ghostboard
  plus Surfari mouse-event/move trace evidence;
- copy uses real keyboard input after the drag selection;
- Ghostboard evidence shows `Cmd+C` was forwarded to the Surfari overlay and not
  consumed by terminal selection or copy-current-url fallback;
- Surfari key trace evidence is present, or its absence is explicitly recorded
  as a limitation alongside other authoritative proof;
- the clipboard before copy matches the harness sentinel;
- the clipboard after copy differs from the sentinel and contains the unique PDF
  fixture marker or accepted marker substring;
- the pre-existing clipboard is restored during cleanup and restoration status
  is recorded;
- pasteboard change indicators show the sentinel write and copy attempt changed
  the pasteboard in the expected sequence;
- the result records PDF-space text rectangle, overlay frame/crop, and computed
  global drag point evidence;
- visible proof is bounded to the Ghostboard app window and overlay crop;
- source/helper-window pixels cannot satisfy the proof;
- cleanup leaves no running Ghostboard, Surfari, WebTUI, or fixture server
  process;
- `webkit/src` remains clean;
- design review and completion review are recorded.

Partial criteria:

- Surfari receives drag and copy input, but WebKit/PDFKit does not expose a
  copyable text selection for this fixture;
- selection appears visually to work but clipboard evidence is unavailable
  because macOS denies clipboard access to the automation environment;
- clipboard copy works but clipboard restoration fails; record the copied proof,
  restoration failure, and classify the experiment as Partial rather than Pass;
- clipboard changes but the copied text is normalized in a way that prevents
  exact marker matching while still proving PDF text copy with another
  authoritative evidence source;
- real mouse input reaches Surfari, but coordinate mapping cannot yet be proven
  well enough to distinguish a missed text rectangle from a WebKit/PDFKit
  selection behavior gap;
- copy works but one evidence source, such as Surfari key trace, is unavailable
  while another authoritative source proves the behavior.

Failure criteria:

- baseline PDF visibility regresses;
- the harness requires `TERMSURF_SURFARI_CACONTEXT_LAYER=snapshot`;
- selection/copy proof can only pass by directly mutating Surfari internals,
  using JavaScript DOM selection, or writing the final clipboard value directly;
- no real mouse path reaches the Surfari PDF overlay;
- no real keyboard copy path reaches Surfari or the active PDF viewer;
- the harness leaves the user's original clipboard unrestored while claiming
  Pass;
- visible proof can be satisfied by a helper/source window;
- cleanup leaves running processes.

## Design Review

An external Codex review checked the design.

Initial verdict: **Changes required**.

Findings:

- the design mutated the system clipboard without requiring save/restore or
  pasteboard change evidence;
- the copy-routing requirement appeared in the changes section but was not part
  of full Pass criteria;
- baseline visible proof covered only "the PDF page" rather than the intended
  selectable text region;
- the design allowed a post-hoc "documented substring" instead of defining the
  accepted copied substring before the run.

Resolution:

- added clipboard save/restore, bounded hash-only recording of the original
  clipboard, pasteboard change indicators, and restoration status requirements;
- made full Pass require Ghostboard evidence that `Cmd+C` is forwarded to the
  Surfari overlay and not consumed by terminal or copy-current-url handling;
- required fixture metadata for generated PDF text operator coordinates,
  PDF-space text bounding box, overlay mapping, and post-drag highlight
  observation or explicit highlight limitation;
- required the accepted marker substring to be predefined and unique before the
  run starts.

Follow-up verdict: **Approved**.

The reviewer found no remaining required findings and approved the design for
the Experiment 44 plan commit.

## Result

**Result:** Partial

Implemented `scripts/test-issue-834-surfari-pdf-selection-copy.sh`, a focused
Surfari PDF selection/copy harness. The harness generates a deterministic PDF
with real PDF text operators, launches repo-built Ghostboard, WebTUI, and
Surfari, saves and restores the system clipboard, drag-selects the intended PDF
text region through real TermSurf-routed mouse input, sends Browse-mode `Cmd+C`,
and records clipboard plus routing evidence.

Final run:

```bash
rm -rf logs/issue-834-exp44-surfari-pdf-selection-copy
env -u TERMSURF_SURFARI_CACONTEXT_LAYER \
  scripts/test-issue-834-surfari-pdf-selection-copy.sh
```

Run ID: `20260622-215254`

Summary:

- summary JSON:
  `logs/issue-834-exp44-surfari-pdf-selection-copy/surfari-pdf-selection-copy-summary.json`;
- overall result: `partial`;
- classification: `surfari-pdf-selection-copy-partial`;
- `TERMSURF_SURFARI_CACONTEXT_LAYER`: unset;
- repo binaries:
  - Ghostboard:
    `ghostboard/macos/build/Debug/TermSurf.app/Contents/MacOS/termsurf`;
  - WebTUI: `target/debug/web`;
  - Surfari: `target/debug/surfari`;
  - WebKit debug framework: `webkit/src/WebKitBuild/Debug`;
- cleanup: Ghostboard/WebTUI/Surfari and the fixture server terminated;
- clipboard restoration status: `restored`.

What passed:

- WebTUI requested `browser=surfari`;
- Surfari emitted `BrowserReady`;
- WebTUI reached ready state;
- Surfari trace recorded the PDF URL;
- Surfari emitted nonzero CAContext;
- Surfari internal render proof passed;
- the fixture server recorded
  `REQUEST path=/selectable.pdf status=200 content_type=application/pdf`;
- baseline Ghostboard-window overlay-cropped proof showed the PDF page and text
  region:
  - cyan page pixels: `813115`;
  - black text pixels: `9062`;
- the generated PDF text operator was
  `BT /F1 24 Tf 72 620 Td (TS834PDFCOPYQXJZ) Tj ET`;
- the recorded PDF-space text box was `x=72`, `y=604`, `width=280`, `height=32`;
- the drag used web points `593.9,233.9` to `918.7,233.9`, which crossed the
  measured glyph band;
- Ghostboard forwarded mouse down, drag move, and mouse up to the overlay with
  `terminal_fallback=false`;
- Surfari trace recorded mouse down, mouse move, and mouse up for the target
  pane;
- Ghostboard observed Browse-mode `Cmd+C` and did not run copy-current-url;
- Surfari trace recorded `Cmd+C` as `windows_key_code=67` with command modifier;
- the system clipboard was saved, primed with the sentinel, checked before copy,
  and restored after the run.

What did not pass:

- after drag-select plus `Cmd+C`, the clipboard still contained the sentinel
  `ISSUE834_EXP44_CLIPBOARD_SENTINEL_20260622-215254`;
- the accepted PDF marker `TS834PDFCOPYQXJZ` was absent from the clipboard;
- a diagnostic fallback attempted `Cmd+A` then `Cmd+C`, but the clipboard still
  contained the sentinel;
- the post-drag screenshot had the same cyan and black counts as the baseline,
  so there was no stable visible selection-highlight proof.

Hygiene and verification:

```bash
./surfari/libtermsurf_webkit/build.sh
cargo fmt -p surfari
cargo build -p surfari
cargo build -p webtui
(cd ghostboard && macos/build.nu --configuration Debug --action build)
bash -n scripts/test-issue-834-surfari-pdf-selection-copy.sh
git diff --check
git -C webkit/src status --short
```

All checks passed. The WebKit shim build emitted the existing SDK-version linker
warning, and the Ghostboard build emitted the existing SwiftLint warning in
`SurfaceView_AppKit.swift`; neither blocked the build.

## Conclusion

Experiment 44 proves that Surfari PDF selection/copy input routing reaches the
right place: the PDF is visible, the intended text region is visible, drag input
is forwarded by Ghostboard and received by Surfari, and Browse-mode `Cmd+C`
reaches Surfari without falling back to terminal or copy-current-url handling.

It does not prove successful PDF text selection/copy. The clipboard remains
unchanged after both drag-select/`Cmd+C` and the diagnostic `Cmd+A`/`Cmd+C`
fallback. The next experiment should diagnose why WebKit/PDFKit is not creating
or copying a text selection from Surfari's PDF surface. Likely directions are:
compare the same generated PDF in a standalone `WKWebView`/Safari control,
inspect whether PDFKit selection is disabled in the embedded WebKit path, and
determine whether Surfari needs an explicit PDF selection/copy bridge.

## Completion Review

An external Codex review checked the completed Experiment 44 implementation,
result, and evidence.

Verdict: **Approved after recording this section**.

Findings:

- Partial is the correct classification because Surfari received the drag and
  `Cmd+C` routing, but the clipboard remained the sentinel and never contained
  `TS834PDFCOPYQXJZ`;
- clipboard handling, routing evidence, and the next-step conclusion are
  adequate;
- the only required fix was to record the completion review in this file.

Resolution:

- this section records the completion review and its verdict.
