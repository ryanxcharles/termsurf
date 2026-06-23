# Experiment 39: Make Surfari Snapshot Presentation Normal

## Description

Experiment 38 proved the immediate CAContext hosting gap: Surfari's
`WKWebView.layer` renders internally but does not become visible through
Ghostboard's existing `CALayerHost` path. The `snapshot-layer` candidate did
become visible by publishing a Surfari-owned local `CALayer` through `CAContext`
and refreshing it from `WKWebView.takeSnapshot`.

This experiment should turn that proven candidate into Surfari's normal TermSurf
presentation path, then prove it refreshes after the user-visible interactions
needed before the PDF matrix can continue.

The goal is not to claim this is the final optimal WebKit architecture. The
longer-term route may still be a WebKit fork exposing the real RemoteLayerTree
hosting context. The goal here is to make Surfari visibly usable enough to
continue PDF workflow coverage without requiring WebTUI, Ghostboard, or Roamium
changes.

## Changes

- Make Surfari use the snapshot-backed CAContext layer by default for ordinary
  web contents.
- Keep the baseline `WKWebView.layer` path available only as an explicit
  diagnostic fallback, if useful.
- Remove or contain temporary candidate knobs that are no longer useful after
  Experiment 38.
- Refresh the snapshot layer after every event that can change visible browser
  pixels:
  - navigation finish and initial CAContext export;
  - Ghostboard/AppKit corrective resize;
  - scroll wheel;
  - mouse press, drag, release, and move when buttons are down;
  - keyboard events;
  - zoom or other commands already wired through the Surfari FFI.
- Coalesce refreshes so input does not trigger unbounded overlapping snapshots.
  A simple pending flag or short main-queue debounce is enough for this
  experiment.
- Add or extend a focused harness, tentatively
  `scripts/test-issue-834-surfari-snapshot-presentation.sh`, that runs Surfari
  without setting `TERMSURF_SURFARI_CACONTEXT_LAYER=snapshot`.
- The harness should prove:
  - `TERMSURF_SURFARI_CACONTEXT_LAYER` is unset in the child environment;
  - Surfari reports the default export method as snapshot-backed;
  - Surfari internal render proof still passes for HTML and PDF;
  - HTML and PDF are visible in the Ghostboard app-window capture by default;
  - the visible proof is bounded to the Ghostboard app window/overlay and cannot
    be satisfied by Surfari source/helper windows;
  - a deterministic PDF fixture visibly changes after a scripted interaction,
    measured by before/after Ghostboard-window captures of the overlay region;
  - resize updates the overlay frame/CAContext dimensions and the post-resize
    visible proof is captured at the new size;
  - cleanup terminates Ghostboard, Surfari, WebTUI, and fixture servers;
  - no installed artifacts are used.
- Update this experiment file with the result.

## Verification

Run formatting/build/hygiene checks:

```bash
./surfari/libtermsurf_webkit/build.sh
cargo fmt -p surfari
cargo build -p surfari
cargo build -p webtui
(cd ghostboard && macos/build.nu --configuration Debug --action build)
bash -n scripts/test-issue-834-surfari-cacontext-hosting.sh
bash -n scripts/test-issue-834-surfari-snapshot-presentation.sh
git diff --check
git -C webkit/src status --short
```

Run the default presentation harness:

```bash
rm -rf logs/issue-834-exp39-surfari-snapshot-presentation
scripts/test-issue-834-surfari-snapshot-presentation.sh
```

Pass criteria:

- Surfari uses the snapshot-backed CAContext layer by default, without requiring
  `TERMSURF_SURFARI_CACONTEXT_LAYER=snapshot`;
- the harness summary records that `TERMSURF_SURFARI_CACONTEXT_LAYER` was unset
  for child processes;
- the harness summary records the default Surfari export method as
  snapshot-backed;
- Surfari internal render proof passes for HTML and PDF;
- HTML and PDF Ghostboard-window visible pixel proof passes;
- the visible proof is bounded to the Ghostboard app window/overlay and cannot
  be satisfied by Surfari source/helper windows;
- the snapshot visibly refreshes after at least one deterministic PDF
  interaction. The harness should use a fixture with different pre-scroll and
  post-scroll colors or text regions, send a real scroll/input event through the
  existing TermSurf path, and require the before/after Ghostboard-window pixel
  counts inside the overlay region to change in the expected direction;
- the snapshot visibly refreshes after resize. The harness should record the
  pre-resize and post-resize overlay frame/CAContext pixel dimensions, require
  them to differ, and require the post-resize visible proof to be captured at
  the new size;
- the harness records the Ghostboard app binary, Surfari binary, WebTUI binary,
  and WebKit artifact paths used;
- the WebTUI binary path is a repo-built artifact, not an installed `web` from
  `PATH`;
- cleanup leaves no running app/browser/server process;
- `webkit/src` remains clean;
- design review and completion review are recorded.

Partial criteria:

- default presentation becomes visible, but interaction or resize refresh still
  needs follow-up work;
- or refresh works but is too slow or overlaps enough to require coalescing in a
  follow-up;
- or the harness can prove visibility but not a deterministic pixel change after
  interaction.

Failure criteria:

- Surfari remains blank by default;
- the harness only passes when an env-gated candidate is set;
- internal WebKit render proof regresses;
- product changes require Ghostboard, WebTUI, or Roamium changes;
- cleanup leaves running processes.

## Design Review

An external Codex review checked the design.

Initial verdict: **Changes required**.

Findings:

- the pass criteria did not explicitly guard against accidentally using the
  Experiment 38 env-gated snapshot candidate;
- visible-pixel proof needed to retain the Experiment 38 requirement that proof
  be bounded to the Ghostboard app window/overlay and not satisfiable by Surfari
  source/helper windows;
- refresh verification needed concrete before/after assertions for deterministic
  interaction and resize;
- internal HTML/PDF render proof needed to be a positive pass criterion;
- the verification block recorded the WebTUI binary but did not build or prove a
  repo-built WebTUI artifact.

Resolution:

- the design now requires `TERMSURF_SURFARI_CACONTEXT_LAYER` to be unset in
  child processes and the default export method to be recorded as
  snapshot-backed;
- visible proof must be bounded to the Ghostboard app window/overlay and exclude
  Surfari source/helper-window satisfaction;
- deterministic before/after pixel-change assertions were added for PDF
  interaction and resize;
- internal HTML/PDF render proof is now required for pass;
- `cargo build -p webtui` and repo-built WebTUI path recording are now required.

Follow-up verdict: **Approved**.

The reviewer found no remaining required changes before the plan commit.

## Result

**Result:** Partial

Surfari now uses the snapshot-backed `CAContext` layer by default when
`TERMSURF_SURFARI_CACONTEXT_LAYER` is unset. The explicit `webview-layer`
diagnostic fallback remains available so the Experiment 38 baseline can still
reproduce the blank raw-`WKWebView.layer` hosting behavior.

Implementation notes:

- `libtermsurf_webkit` now treats an unset `TERMSURF_SURFARI_CACONTEXT_LAYER` as
  `snapshot`;
- snapshot refreshes are scheduled after export, navigation finish, corrective
  resize, scroll, keyboard events, mouse button events, and mouse drag events;
- refreshes are coalesced with a pending/again flag so repeated events do not
  create unbounded overlapping snapshot requests;
- delayed refresh callbacks and snapshot completions re-resolve the tab from the
  live `WebContents` registry before touching state, avoiding use-after-free if
  a tab is destroyed while refresh work is pending;
- the render-probe snapshot path now also updates the hosted snapshot layer,
  which fixed the observed HTML case where WebKit's internal pixels existed but
  the hosted layer still showed the initial blank snapshot;
- `scripts/test-issue-834-surfari-cacontext-hosting.sh` now forces
  `TERMSURF_SURFARI_CACONTEXT_LAYER=webview-layer` for its baseline run;
- `scripts/test-issue-834-surfari-snapshot-presentation.sh` verifies that the
  default presentation test runs with the env var unset and records the
  repo-built Ghostboard, WebTUI, Surfari, and WebKit artifact paths.

Verification:

```bash
./surfari/libtermsurf_webkit/build.sh
cargo fmt -p surfari
cargo build -p surfari
cargo build -p webtui
(cd ghostboard && macos/build.nu --configuration Debug --action build)
bash -n scripts/test-issue-834-surfari-cacontext-hosting.sh
bash -n scripts/test-issue-834-surfari-snapshot-presentation.sh
git diff --check
git -C webkit/src status --short
rm -rf logs/issue-834-exp39-surfari-snapshot-presentation \
  logs/issue-834-exp37-surfari-side-render-pixels \
  logs/issue-834-exp36-surfari-visual-compositing
env -u TERMSURF_SURFARI_CACONTEXT_LAYER \
  scripts/test-issue-834-surfari-snapshot-presentation.sh
```

The native WebKit build passed with the existing SDK warning about building for
macOS 26.0 while linking a WebKit framework built for 26.5. The Ghostboard debug
build passed with the existing SwiftLint warning in `SurfaceView_AppKit.swift`.

The final harness run was
`logs/issue-834-exp39-surfari-snapshot-presentation/surfari-snapshot-presentation-summary.json`
with run id `20260622-200744`. Its summary recorded:

- `termsurf_surfari_cacontext_layer`: `unset`;
- `default_export_method`: `snapshot-backed`;
- HTML internal render: pass;
- PDF internal render: pass;
- HTML Ghostboard-window visible proof: pass;
- PDF Ghostboard-window visible proof: pass;
- HTML refresh reasons: `export`, `coalesced`, `render-probe`, `export`,
  `coalesced`;
- PDF refresh reasons: `export`, `coalesced`, `render-probe`;
- classification: `default-snapshot-visible-refresh-deltas-unproven`;
- overall result: `partial`.

## Conclusion

This experiment converted Surfari's proven snapshot-backed candidate into the
default visible presentation path and proved that HTML and PDF overlays are
visible in the real Ghostboard window without an env-gated candidate. It did not
complete the full pass criteria because the harness still does not drive a
deterministic PDF interaction or resize and prove a before/after pixel delta in
the hosted overlay.

The next experiment should focus only on deterministic refresh proof: create or
reuse a fixture whose visible overlay pixels must change after a scripted scroll
or input event, then prove both interaction refresh and resize refresh from
Ghostboard-window captures.

## Completion Review

An external Codex review checked the staged result.

Initial verdict: **Changes required**.

Findings:

- delayed snapshot refresh work captured a raw `WebContents *` across
  `dispatch_after` and `takeSnapshotWithConfiguration` completion callbacks, so
  tab destruction could free the struct before callbacks touched it;
- the completion review needed to be recorded in this experiment file before the
  result commit.

Resolution:

- delayed refresh callbacks now capture the tab id and re-resolve the current
  `WebContents` from the live registry before refreshing;
- snapshot completion callbacks now re-resolve the tab and verify the
  `WKWebView` identity before touching refresh flags, updating the snapshot
  layer, scheduling coalesced work, or firing the render probe;
- this completion review section records the review result.

Follow-up verdict: **Approved**.

The reviewer found no remaining required fixes before committing Experiment 39
as a Partial result.
