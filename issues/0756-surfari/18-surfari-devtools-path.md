# Experiment 18: Implement Surfari DevTools Path

## Description

Experiment 17 found one unsupported engine-owned protocol message:
`CreateDevtoolsTab`. Surfari's Rust dispatch already receives that message and
calls `ts_create_devtools_web_contents`, but `libtermsurf_webkit` currently
returns `nullptr`.

This experiment will make the DevTools path concrete before Ghostboard
integration. The preferred outcome is to implement WebKit Inspector-backed
DevTools as a TermSurf-hostable WebKit surface, so `CreateDevtoolsTab` creates a
second tab entry, emits `TabReady` and `CaContext`, participates in
`QueryTabsReply`, resizes, and closes cleanly. If WebKit Inspector cannot expose
a hostable surface in this experiment, the experiment must record the exact
reason and add an explicit, tested unsupported response path instead of leaving
a silent `nullptr` tab.

The experiment should also close the small audit gap from Experiment 17 by
extending the Surfari fake-GUI harness to send `QueryTabsRequest` and verify the
browser/devtools tab counts.

## Changes

- Inspect WebKit and MiniBrowser/Web Inspector APIs needed to create or attach a
  Web Inspector view for an existing `WebContents`.
- Update `surfari/libtermsurf_webkit/src/libtermsurf_webkit.mm` and
  `surfari/libtermsurf_webkit/include/libtermsurf_webkit.h` only as needed to
  implement `ts_create_devtools_web_contents` or a deliberate explicit
  unsupported path.
- Update `surfari/src/dispatch.rs` only if the Rust layer needs to avoid
  registering a `nullptr` DevTools entry or needs to send a clear protocol/log
  result when DevTools is unsupported.
- Extend `scripts/test-issue-756-surfari-fake-gui.py` to exercise: `CreateTab`,
  `QueryTabsRequest`, `CreateDevtoolsTab`, DevTools resize, `QueryTabsRequest`
  after DevTools creation, `CloseTab` for DevTools, and `CloseTab` for the
  inspected browser tab.
- Update `surfari/libtermsurf_webkit/README.md` to move DevTools out of the
  unsupported list only if the implementation is proven. If the result is an
  explicit deferral, document the tested limitation instead.
- Do not modify `webkit/src` in this experiment unless the implementation proves
  that a small WebKit patch is required. If a WebKit patch is required, stop and
  record that finding rather than folding a WebKit fork change into this
  experiment.
- Do not modify Ghostboard, webtui, Roamium, or `termsurf.proto` unless the
  implementation proves a protocol mismatch that cannot be represented by the
  current messages.

## Verification

Pass criteria for the preferred implementation path:

- `cargo build -p surfari` succeeds.
- `cargo fmt -p surfari -- --check` succeeds.
- The fake-GUI harness is run against the deterministic navigation fixture:

```bash
cargo build -p surfari
rm -rf logs/issue-756-exp18-surfari-devtools
scripts/test-issue-756-surfari-fake-gui.py \
  "file://$PWD/surfari/libtermsurf_webkit/test-content/navigation.html" \
  --log-dir logs/issue-756-exp18-surfari-devtools
```

- The fake-GUI harness proves normal tab creation still works and logs
  `ServerRegister`, `TabReady`, `CaContext`, `UrlChanged`, `LoadingState`, and
  `TitleChanged` for the inspected browser tab.
- The harness sends `QueryTabsRequest` before DevTools creation and verifies one
  browser tab, zero DevTools tabs, and the expected `TabInfo`.
- The harness sends `CreateDevtoolsTab` for the inspected tab and receives a
  positive DevTools `TabReady` plus nonzero `CaContext`.
- The harness sends `QueryTabsRequest` after DevTools creation and verifies one
  browser tab, one DevTools tab, and correct `inspected_tab_id` on the DevTools
  `TabInfo`.
- The harness sends `Resize` for the DevTools tab and verifies the Surfari trace
  includes `ffi=ts_set_view_size` for that tab.
- The harness sends `CloseTab` for the DevTools tab, then the inspected browser
  tab, and Surfari exits cleanly after all tabs are closed.
- `git diff --check` succeeds.
- Markdown formatting succeeds for edited issue and README files:

```bash
prettier --check --prose-wrap always --print-width 80 \
  issues/0756-surfari/README.md \
  issues/0756-surfari/18-surfari-devtools-path.md \
  surfari/libtermsurf_webkit/README.md
```

If the implementation path fails because WebKit Inspector cannot provide a
TermSurf-hostable surface without a WebKit fork patch, this experiment may still
pass only if it:

- records the exact API or architectural blocker with source references;
- changes the Rust/C boundary so `CreateDevtoolsTab` does not create a dangling
  `nullptr` tab entry;
- extends the fake-GUI harness to verify the explicit unsupported behavior:
  after `CreateDevtoolsTab`, Surfari must emit no DevTools `TabReady`, emit no
  DevTools `CaContext`, keep `QueryTabsReply` at one browser tab and zero
  DevTools tabs, include no `TabInfo` with `id = 0`, and write a named trace or
  log marker such as `devtools-unsupported` for the rejected request; and
- documents the remaining WebKit patch requirement as the next experiment.

## Design Review

Adversarial design review initially returned `CHANGES REQUIRED` with one
required finding: the fallback criteria allowed an "explicit unsupported
behavior" result without defining the observable protocol/log contract, which
could still hide the existing null-handle DevTools tab problem. The design now
requires no DevTools `TabReady`, no DevTools `CaContext`, one browser tab, zero
DevTools tabs, no `TabInfo` with `id = 0`, and a named unsupported marker in the
trace/log. The review also suggested adding the exact fake-GUI harness command;
the design now includes it with the deterministic navigation fixture.
