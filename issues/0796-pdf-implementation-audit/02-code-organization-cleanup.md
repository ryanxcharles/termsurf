# Experiment 2: Code Organization Cleanup

## Description

This experiment performs the behavior-preserving cleanup identified by
Experiment 1. It should make the PDF implementation easier to read and audit
without changing PDF viewer behavior.

The cleanup has three priorities:

1. Give durable PDF diagnostics stable TermSurf names instead of historical
   issue/experiment labels.
2. Move PDF-specific browser and renderer support behind named helper layers so
   generic Chromium embedder clients are easier to scan.
3. Deduplicate the most obvious repeated PDF data/trace/test harness plumbing
   where doing so does not create broad dependency churn.

This experiment is not allowed to fix security issues, add missing PDF features,
or revisit native PDF printing. If a real security or completeness issue is
noticed during cleanup, record it for the later audit track and leave behavior
unchanged.

The cleanup must receive Codex design review before implementation. After the
cleanup and verification are complete, Codex must review the result before this
experiment is marked complete.

## Branching

This experiment modifies Chromium code, so it must use a fresh Chromium branch:

1. Start from the current known-good PDF branch `148.0.7778.97-issue-794-exp20`.
2. Create `148.0.7778.97-issue-796-exp2`.
3. Add the branch to `chromium/README.md` with a link to this issue and a short
   description such as "PDF organization cleanup".
4. Commit Chromium changes on that branch before regenerating the patch archive.
5. Regenerate the cumulative patch archive from the Chromium base tag:

   ```bash
   cd chromium/src
   rm -rf ../../chromium/patches/issue-796-exp2/
   git format-patch 148.0.7778.97..HEAD -o ../../chromium/patches/issue-796-exp2/
   ```

Do not modify the existing Issue 792, 793, or 794 Chromium branches.

## Changes

### 1. Rename durable PDF diagnostics

Replace stable diagnostics that still use historical issue labels with stable
names.

Use a small set of prefixes:

- `[termsurf-pdf]` for extension, stream, MimeHandler, browser/renderer setup,
  and resource loading events.
- `[termsurf-pdf-input]` for keyboard, mouse, wheel, selection, and routing
  events.
- `[termsurf-pdf-resize]` for view, plugin, and PDFium geometry events.
- `[termsurf-pdf-title]` for PDF title propagation events.
- `[termsurf-pdf-print]` for print containment and print trace events.

Update the automation scripts that match old labels in the same commit. Every
old required event must have a stable replacement. Do not delete useful
diagnostics unless an equivalent stable event remains.

Primary files:

- `chromium/src/content/libtermsurf_chromium/ts_browser_client.cc`
- `chromium/src/content/libtermsurf_chromium/ts_browser_main_parts.cc`
- `chromium/src/content/libtermsurf_chromium/ts_content_client.cc`
- `chromium/src/content/libtermsurf_chromium/ts_content_renderer_client.cc`
- `chromium/src/content/libtermsurf_chromium/ts_mime_handler_binders.cc`
- `chromium/src/content/libtermsurf_chromium/ts_pdf_*`
- `chromium/src/content/libtermsurf_chromium/ts_plugin_*`
- `chromium/src/content/libtermsurf_chromium/extensions/ts_*`
- PDF-specific TermSurf edits in `chromium/src/pdf/pdf_view_web_plugin.cc` and
  `chromium/src/pdf/pdfium/pdfium_engine.cc`
- `scripts/test-issue-794-*.py`

Leave non-PDF historical diagnostics from other issues alone unless they are
directly part of a PDF test assertion.

### 2. Extract renderer-side PDF support

Keep `TsContentRendererClient` as the Chromium override owner, but move
PDF-specific implementation details into a new helper module, tentatively:

- `chromium/src/content/libtermsurf_chromium/ts_pdf_renderer_support.h`
- `chromium/src/content/libtermsurf_chromium/ts_pdf_renderer_support.cc`

The helper should own behavior that is currently inline in
`ts_content_renderer_client.cc`, including:

- PDF viewer `chrome://resources` origin allow-list setup;
- MimeHandlerView container-manager binding;
- extension frame helper / dispatcher setup call helpers where that improves
  scanability;
- PDF print helper delegate construction;
- PDF-specific `IsPluginHandledExternally` decision logic;
- PDF-specific `OverrideCreatePlugin` decision logic.

Do not change the order in which `TsContentRendererClient` calls Chromium base
class methods, extension renderer startup, dispatcher hooks, or plugin creation.
This is a move/rename cleanup only.

### 3. Extract browser-side PDF support

Keep `TsBrowserClient` as the Chromium override owner, but move PDF-specific
browser support into a new helper module, tentatively:

- `chromium/src/content/libtermsurf_chromium/ts_pdf_browser_support.h`
- `chromium/src/content/libtermsurf_chromium/ts_pdf_browser_support.cc`

The helper should own behavior that is currently inline in
`ts_browser_client.cc`, including:

- adding PDF navigation throttles;
- adding the TermSurf plugin response URL loader throttle;
- adding PDF URL loader request interceptors;
- collecting external plugin MIME types for PDF;
- registering PDF extension frame binders;
- registering PDF host and MimeHandler binders;
- appending PDF trace/print switches to renderer command lines;
- creating extension-scheme URL loader factories;
- process-per-site and handled-URL decisions for the PDF extension;
- process-map insertion and PDF extension process activation/grants.

The generic browser client should become a thin dispatcher that delegates to
named PDF helpers while preserving its existing fallback calls to
`ShellContentBrowserClient`.

### 4. Centralize PDF load-time data

Replace the duplicated PDF localized-string and feature-flag tables in:

- `chromium/src/content/libtermsurf_chromium/extensions/ts_component_extension_resource_manager.cc`
- `chromium/src/content/libtermsurf_chromium/extensions/ts_resources_private_api.cc`

with one shared helper, tentatively:

- `chromium/src/content/libtermsurf_chromium/extensions/ts_pdf_load_time_data.h`
- `chromium/src/content/libtermsurf_chromium/extensions/ts_pdf_load_time_data.cc`

The helper should expose adapters for both existing call shapes:

- populate `ui::TemplateReplacements`;
- return/populate `base::DictValue`.

The data values must remain unchanged, including:

- `presetZoomFactors`;
- `pdfOopifEnabled`;
- `pdfGetSaveDataInBlocks`;
- `pdfGlicSummarizeEnabled`;
- `pdfInk2Enabled`;
- `pdfSaveToDrive`;
- `pdfSearchifySaveEnabled`;
- `pdfTextAnnotationsEnabled`;
- `pdfUseShowSaveFilePicker`;
- `printingEnabled`;
- `termsurfPdfPrintBridgeTrace`.

### 5. Deduplicate trace helpers only where dependency boundaries permit

Deduplicate PDF trace/env helpers, but avoid broad Chromium dependency churn.

Required cleanup:

- keep switch/env-var names in one Chromium-side helper visible to the
  TermSurf-owned Chromium files that already share a target;
- keep behavior unchanged for command-line switch precedence over environment
  variables;
- preserve Wezboard's existing trace-file truncation behavior;
- preserve Roamium's existing append behavior;
- preserve the `wezboard` and `roamium` line prefixes in `pdf-input.log`.

If sharing one helper between `content/libtermsurf_chromium`, `components/pdf`,
`components/printing`, and `pdf` would require invasive BUILD changes, do not
force that in this experiment. In that case, document the remaining duplication
as deferred cleanup in the result and complete only the safe subset.

### 6. Begin automation harness cleanup without breaking old entrypoints

Move duplicated Python protocol harness pieces into a shared module,
tentatively:

- `scripts/termsurf_pdf_protocol_harness.py`

Prioritize shared helpers for:

- varint and protobuf field encoding;
- length-prefixed TermSurf message writes;
- fake GUI socket setup;
- Roamium launch env setup;
- Bitcoin PDF fixture serving;
- trace file path setup;
- DevTools port discovery.

Migrate the protocol scroll, resize, and mouse scripts to use the shared helper.
Keep their existing filenames and command-line behavior intact.

JavaScript DevTools probe cleanup is optional for this experiment. If it would
make the experiment too large, defer it explicitly to a later organization
follow-up and do not block the security audit on it.

## Non-Negotiable Invariants

- No PDF behavior changes.
- No protocol changes.
- No native PDF printing implementation.
- No security tightening or feature additions in this experiment.
- Do not modify closed issue documents.
- Do not remove diagnostics without replacing them with stable equivalents.
- Do not change public script entrypoints unless compatibility wrappers remain.
- Do not use `ninja`; use `autoninja` for Chromium builds.
- Run `cargo fmt` if Rust files are edited and accept its output.
- Run Prettier on Markdown files after edits.

## Verification

### Static checks

1. Search for remaining PDF-specific historical labels:

   ```bash
   rg -n "\\[issue-79[234].*pdf|\\[issue-794-exp[45678]|\\[issue-792-exp" \
     chromium/src/content/libtermsurf_chromium \
     chromium/src/pdf \
     chromium/src/components/pdf \
     chromium/src/components/printing \
     scripts
   ```

   Remaining matches are allowed only if they are non-PDF historical context,
   closed issue records, or explicitly documented deferred cleanup.

2. Search for duplicated PDF load-time string tables and confirm there is one
   source table:

   ```bash
   rg -n "annotationsShowToggle|presetZoomFactors|printingEnabled|termsurfPdfPrintBridgeTrace" \
     chromium/src/content/libtermsurf_chromium/extensions
   ```

3. Confirm old script entrypoints still exist:

   ```bash
   test -x scripts/test-issue-794-protocol-scroll.py
   test -x scripts/test-issue-794-protocol-resize.py
   test -x scripts/test-issue-794-protocol-mouse.py
   test -x scripts/test-issue-794-pdf-toolbar.py
   ```

### Build and format checks

1. Run Chromium formatting for changed Chromium C++ files using the project's
   existing Chromium formatting workflow.
2. Build Chromium:

   ```bash
   cd chromium/src
   export PATH="$HOME/dev/termsurf/chromium/depot_tools:$PATH"
   autoninja -C out/Default libtermsurf_chromium
   ```

3. Build Roamium from the repo, not with `autoninja`:

   ```bash
   cd /Users/ryan/dev/termsurf
   CARGO_BIN="$(/opt/homebrew/bin/rustup which cargo)"
   PATH="$(dirname "$CARGO_BIN"):$PATH" ./scripts/build.sh roamium
   ```

4. Run `cargo fmt` if Rust files are edited.
5. Run Prettier on this issue file and the README.

### Behavioral regression checks

Run the existing PDF harnesses after the cleanup:

```bash
scripts/test-issue-794-pdf-toolbar.py \
  --log-dir logs/issue-796-exp2-toolbar \
  --serve-bitcoin-pdf \
  --probe toolbar
```

```bash
scripts/test-issue-794-pdf-toolbar.py \
  --log-dir logs/issue-796-exp2-title-save-local \
  --serve-bitcoin-pdf \
  --probe save-print-title-local
```

This default production print check must confirm the `save-print-title-local`
summary reports the known non-print outcome: production print is available but
not clicked, no native print dialog is expected, and no print intercept logs are
created.

```bash
scripts/test-issue-794-pdf-toolbar.py \
  --log-dir logs/issue-796-exp2-title-save-local-intercept \
  --serve-bitcoin-pdf \
  --probe save-print-title-local \
  --enable-pdf-print-intercept
```

```bash
scripts/test-issue-794-protocol-scroll.py \
  --log-dir logs/issue-796-exp2-scroll \
  --serve-bitcoin-pdf
```

```bash
scripts/test-issue-794-protocol-resize.py \
  --log-dir logs/issue-796-exp2-resize \
  --serve-bitcoin-pdf
```

```bash
scripts/test-issue-794-protocol-mouse.py \
  --log-dir logs/issue-796-exp2-mouse \
  --serve-bitcoin-pdf
```

Also run a deterministic non-PDF smoke test against a local fixture, not a
network URL. The smoke test must prove:

- a normal HTML page loads through Roamium;
- scrolling changes page position;
- clicking a button or link changes DOM-visible state;
- text input accepts typed text if the fixture includes an input.

Record the fixture path, command, and pass/fail evidence in the result.

## Pass Criteria

This experiment passes if:

- Chromium/Roamium builds successfully on the new branch;
- the PDF viewer behavior is unchanged under the regression matrix above;
- useful PDF diagnostics have stable names;
- old required automation log checks have stable replacements;
- browser and renderer client PDF responsibilities are clearer and delegated to
  named helpers;
- PDF load-time data has one source of truth;
- protocol harness duplication is reduced without breaking old entrypoints;
- Codex completion review agrees the cleanup is behavior-preserving and
  sufficient to proceed to the security audit.

## Partial Criteria

This experiment is partial if the most important cleanup succeeds but one
cleanup area must be deferred because it would require disproportionate BUILD
dependency churn or would risk behavior changes. A partial result must clearly
state what remains and whether the security audit can proceed safely.

## Failure Criteria

This experiment fails if:

- PDF rendering, scroll, resize, toolbar controls, selection, title propagation,
  save/download, local-file behavior, or known print containment regresses;
- the cleanup changes protocol surface;
- native PDF printing is implemented or re-scoped here;
- useful diagnostics are deleted without stable replacements;
- old script entrypoints stop working without compatibility wrappers;
- Chromium changes are made on an existing issue branch instead of the fresh
  Issue 796 experiment branch;
- verification is too narrow to prove behavior was preserved.
