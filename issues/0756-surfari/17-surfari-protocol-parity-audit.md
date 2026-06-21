# Experiment 17: Audit Surfari protocol parity

## Description

Experiment 16 proved that Surfari can run as a real TermSurf browser process
against a fake GUI socket. The next Issue 756 checklist item is to audit Surfari
against Roamium and the existing TermSurf protobuf messages, marking every
message supported, unsupported, or requiring a protocol extension.

This experiment should produce a concrete protocol/API support matrix. It should
compare:

- `proto/termsurf.proto`;
- `roamium/src/main.rs` and `surfari/src/main.rs`;
- `roamium/src/ffi.rs` and `roamium/src/dispatch.rs`;
- `surfari/src/ffi.rs` and `surfari/src/dispatch.rs`;
- `surfari/libtermsurf_webkit/include/libtermsurf_webkit.h`;
- `surfari/libtermsurf_webkit/src/libtermsurf_webkit.mm`;
- `surfari/libtermsurf_webkit/README.md`;
- existing evidence from Experiments 5-16.

The output should make the remaining work obvious before Ghostboard integration:
which protocol messages are already implemented and tested for Surfari, which
are wired but only partially proven, which are unsupported because the C ABI is
missing a real implementation, and whether any WebKit capability needs a
`termsurf.proto` extension.

This experiment is an audit/documentation experiment. It should not implement
DevTools, modify `termsurf.proto`, integrate Ghostboard, change webtui browser
selection, or patch WebKit source. It may update the Issue 756 checklist for
items already proven by Experiments 15 and 16.

## Changes

- Add an audit artifact under the Issue 756 folder, for example
  `17-surfari-protocol-parity-audit.md` itself or a companion table if the
  result is too large.
- Build a support matrix for all TermSurf protocol messages relevant to the
  browser engine process:
  - GUI -> engine: `CreateTab`, `CreateDevtoolsTab`, `Resize`, `CloseTab`,
    `Navigate`, `MouseEvent`, `MouseMove`, `ScrollEvent`, `KeyEvent`,
    `FocusChanged`, `SetColorScheme`, `SetGuiActive`, `JavaScriptDialogReply`,
    and `HttpAuthReply`;
  - engine -> GUI/TUI: `ServerRegister`, `TabReady`, `CaContext`, `UrlChanged`,
    `LoadingState`, `TitleChanged`, `CursorChanged`, `TargetUrlChanged`,
    `JavaScriptDialogRequest`, `ConsoleMessage`, `HttpAuthRequest`,
    `RendererCrashed`, and `QueryTabsReply`;
  - request messages handled by the engine directly, such as `QueryTabsRequest`;
  - messages intentionally not engine-owned, such as `SetOverlay`,
    `SetDevtoolsOverlay`, `OpenSplit`, `HelloRequest`, `HelloReply`,
    `QueryLastRequest`, `QueryLastReply`, `QueryDevtoolsRequest`,
    `QueryDevtoolsReply`, `BrowserReady`, and `ModeChanged`.
- For each message, record:
  - owner (`GUI`, `TUI`, `engine`, or not engine-owned);
  - Roamium status;
  - Surfari Rust dispatch status;
  - `libtermsurf_webkit` C ABI status;
  - evidence source, such as a smoke test, fake-GUI log, source line, or prior
    experiment;
  - final classification: `Supported`, `Partial`, `Unsupported`,
    `Not engine-owned`, or `Needs protocol extension`.
- Identify exact gaps that must be solved before Ghostboard integration. The
  current expected gap is DevTools: `CreateDevtoolsTab` is wired in Surfari Rust
  but `ts_create_devtools_web_contents` returns `nullptr`.
- Determine whether the current protocol needs extension for WebKit. If no
  extension is needed yet, state that explicitly and leave the `termsurf.proto`
  checklist item unchecked until the claim is backed by a broader in-app test
  matrix.
- Update the Issue 756 end-to-end checklist only for items already proven by
  committed evidence:
  - mark the Surfari Rust binary item complete, citing Experiment 15;
  - mark the fake-GUI driver item complete, citing Experiment 16;
  - do not mark Ghostboard integration, in-app testing, regression guards, or
    full Ghostboard/Roamium matrix parity complete.
- Keep the issue README's experiment index synchronized.

## Verification

Run source inspections:

```bash
rg -n "Msg::|ts_|message " \
  proto/termsurf.proto \
  roamium/src/main.rs surfari/src/main.rs \
  roamium/src/ffi.rs roamium/src/dispatch.rs \
  surfari/src/ffi.rs surfari/src/dispatch.rs \
  surfari/libtermsurf_webkit/include/libtermsurf_webkit.h \
  surfari/libtermsurf_webkit/src/libtermsurf_webkit.mm
```

Run focused build/check commands:

```bash
cargo build -p surfari
cargo fmt -p surfari -- --check
git diff --check
prettier --check --prose-wrap always --print-width 80 \
  issues/0756-surfari/README.md \
  issues/0756-surfari/17-surfari-protocol-parity-audit.md
git -C webkit/src status --short
git -C webkit/src rev-parse HEAD
git -C webkit/src rev-parse --abbrev-ref HEAD
git -C webkit/src rev-parse --is-shallow-repository
```

The audit passes if:

- every TermSurf protocol message is accounted for in the matrix;
- engine-owned messages have a classification with concrete evidence;
- non-engine-owned messages are explicitly separated from Surfari work;
- known Surfari gaps are precise enough to become follow-up experiments;
- the Issue 756 checklist is updated only where existing evidence proves
  completion;
- no source code, protocol, Ghostboard, webtui, or WebKit-source changes are
  made.

**Pass** = the audit matrix is complete, evidence-backed, and identifies exact
remaining gaps without changing implementation.

**Partial** = the audit finds ambiguous ownership or insufficient evidence for
one or more messages. Record the ambiguity and design the next experiment to
produce stronger evidence.

**Fail** = the audit cannot account for protocol ownership, contradicts source
evidence, or discovers a required implementation gap that must be fixed before
the matrix can be meaningful.

## Design Review

Adversarial design review initially found one required issue: the audit included
`ServerRegister`, but the source list omitted `roamium/src/main.rs` and
`surfari/src/main.rs`, where `ServerRegister` is sent. The design now includes
both `main.rs` files in the comparison list and verification command. Re-review
approved the design with no remaining required findings.
