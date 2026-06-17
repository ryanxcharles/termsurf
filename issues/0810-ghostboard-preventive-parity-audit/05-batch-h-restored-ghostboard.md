# Experiment 5: Batch H Restored Ghostboard Audit

## Description

Classify the newest historical audit batch from Experiment 4: issues
`0800`-`0809`. This batch covers Roastty architecture, libroastty completion,
GUI automation, parity with the Ghostty base commit, Ghostboard restoration, and
Issue 809's viewport-geometry proof. It is the highest-signal historical slice
for current restored Ghostboard behavior.

This experiment should read every Batch H issue and map each durable lesson to
current Ghostboard risk using the schema defined in Experiment 4. The output is
a classification table, not fixes.

This is an audit/documentation experiment only. It must not change application
code, generated code, historical issue files, closed issue files, scripts, or
test harnesses.

## Changes

Planned files:

- `issues/0810-ghostboard-preventive-parity-audit/05-batch-h-restored-ghostboard.md`
  - record this experiment design, design review, Batch H classification result,
    completion review, and conclusion;
  - classify every issue in Batch H using the Experiment 4 historical audit row
    schema.
- `issues/0810-ghostboard-preventive-parity-audit/README.md`
  - add Experiment 5 to the `## Experiments` index with status `Designed`, then
    update status after the result.

No application code, generated protobuf code, historical issue files, closed
issue files, scripts, or test harnesses should be edited.

## Verification

Design-gate pass criteria:

- The issue README links this experiment as `Designed`.
- A fresh-context adversarial design review approves the plan.
- The plan commit exists before implementation begins.

Implementation pass criteria:

- The result audits every Batch H issue exactly once:
  - `0800-roastty-architecture`
  - `0801-roastty-libghostty-rewrite`
  - `0802-libroastty-completion-and-mac-app`
  - `0803-roastty-debug-overlay`
  - `0804-roastty-gui-automation-readiness`
  - `0805-roastty-ghostty-parity`
  - `0806-roastty-input-latency`
  - `0807-restore-ghostboard-code`
  - `0808-recreate-ghostboard-from-ghostty-1-3-1`
  - `0809-ghostboard-viewport-geometry`
- The result uses the Experiment 4 row schema for every classification: source
  issue, batch, subsystem, durable lesson, current Ghostboard relevance,
  evidence paths, likelihood, risk or impact, recommended follow-up, and
  historical classification note.
- The result classifies each row as `Highly likely`, `Maybe`, or `No`, and
  explains the classification from issue evidence plus current code/test/doc
  evidence.
- The result treats Issue 803 as open historical evidence without trying to
  close or modify it.
- The result incorporates the highest-signal protocol findings already learned
  in this issue where relevant, especially likely missing Ghostboard handling
  for `CursorChanged` and `SetGuiActive`.
- The result distinguishes proven current coverage from historical success. A
  closed historical issue is not enough by itself to classify current Ghostboard
  risk as `No`.
- The result identifies the next audit slice after Batch H.
- Markdown is formatted:

  ```bash
  prettier --write --prose-wrap always --print-width 80 \
    issues/0810-ghostboard-preventive-parity-audit/README.md \
    issues/0810-ghostboard-preventive-parity-audit/05-batch-h-restored-ghostboard.md
  ```

- Whitespace check passes:

  ```bash
  git diff --check
  ```

- A fresh-context completion review approves the completed result before the
  result commit.
- All real completion-review findings are fixed and recorded in this experiment
  file.
- The result commit is made after completion-review approval and before any next
  experiment is designed.

Fail criteria:

- Any Batch H issue is omitted or classified more than once.
- The experiment edits historical issue files or application code.
- The result treats historical completion as proof of current Ghostboard parity
  without current evidence.
- The result fixes a finding instead of recording it for follow-up.
- The result expands into other historical batches before Batch H is concluded.

## Design Review

Fresh-context adversarial design review returned **APPROVED**.

Reviewer checks confirmed:

- The README links Experiment 5 as `Designed`.
- The design has `Description`, `Changes`, and `Verification`.
- Scope is audit-only and excludes application code, generated code, historical
  issue files, scripts, and test harnesses.
- Batch H is exactly `0800`-`0809`; every listed issue appears once, and
  expansion to other batches is a fail condition.
- Verification requires the Experiment 4 schema, current evidence, Issue 803 as
  open evidence, and carried-forward `CursorChanged` / `SetGuiActive` findings.
- `git diff --check` passed.
- The plan commit had not yet been made before review.

Findings: none.

## Result

**Result:** Pass

Batch H was audited as the newest restored-Ghostboard slice. The classification
unit is each historical issue folder, so the table below has exactly ten rows:
one for every issue from `0800` through `0809`.

### Classification Table

| Source issue                                  | Batch | Subsystem                        | Durable lesson                                                                                                                                                                                    | Current Ghostboard relevance                                                                                                                                                                                                                                                             | Evidence paths                                                                                                                                                                                                                                                                                                                                                                                                                                                   | Likelihood      | Risk or impact                                                                                                                                                                                                | Recommended follow-up                                                                                                                                                                             | Historical classification note                                                                                                                                                             |
| --------------------------------------------- | ----- | -------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `0800-roastty-architecture`                   | H     | Roastty architecture / ABI       | New terminal ports need explicit architecture, naming rules, ABI inventory, and test parity before browser overlays are layered on.                                                               | Current Ghostboard is not the Roastty Rust rewrite; it is a fresh Ghostty `v1.3.1` subtree. The durable process lesson applies, but the specific `roastty_*` ABI and Rust terminal-core work do not map to active `ghostboard/`.                                                         | Issue 800 conclusion records Roastty naming and ABI outcomes in `issues/0800-roastty-architecture/README.md:319`; current active GUI is `ghostboard/`, while Roastty remains separate under `roastty/`.                                                                                                                                                                                                                                                          | `No`            | Low for current Ghostboard. Misapplying this issue would create false positives by auditing a non-active Rust rewrite instead of the active Ghostty-derived GUI.                                              | No Ghostboard follow-up from this issue. Keep the architecture lesson for future non-Ghostty terminal ports.                                                                                      | Classified `No` because current Ghostboard intentionally keeps many upstream Ghostty internals; Issue 808 explicitly made the minimal-port choice for `ghostboard/`.                       |
| `0801-roastty-libghostty-rewrite`             | H     | Roastty terminal core / renderer | Large terminal rewrites require subsystem-by-subsystem test parity; partial live-render and app-integration gaps must be named rather than hidden.                                                | Current Ghostboard imports Ghostty's working Zig terminal core instead of using `libroastty`, so the Rust rewrite gaps do not directly threaten it. The general parity discipline is relevant, but the specific remaining Roastty partials are not current Ghostboard code paths.        | Issue 801 conclusion records 849 experiments, 4394 tests, and remaining Roastty live-render/app gaps in `issues/0801-roastty-libghostty-rewrite/README.md:2437`; active Ghostboard remains under `ghostboard/`, not `roastty/`.                                                                                                                                                                                                                                  | `No`            | Low direct risk. Treating Roastty's Rust gaps as Ghostboard bugs would conflate two implementations.                                                                                                          | No Ghostboard follow-up from this issue. Revisit only if Ghostboard is later rebuilt on `libroastty`.                                                                                             | Classified `No` for current Ghostboard, not for Roastty itself; Issue 801's remaining gaps are real in Roastty's historical context.                                                       |
| `0802-libroastty-completion-and-mac-app`      | H     | Roastty copied macOS app         | A copied Ghostty macOS app needs live app proofs for rendering, config, keybindings, native key handling, and event taps; debug overlay was left optional.                                        | Current Ghostboard uses the upstream Ghostty macOS app lineage directly, so most copied-app proof debt does not transfer. The optional debug-overlay reminder is relevant only as a low-priority observability feature.                                                                  | Issue 802 conclusion states the live copied app rendered ASCII, proved config/key/native input, and left only optional Debug `Overlay` unchecked in `issues/0802-libroastty-completion-and-mac-app/README.md:1676`; current Ghostboard app starts TermSurf IPC in `ghostboard/macos/Sources/App/macOS/AppDelegate.swift:205`.                                                                                                                                    | `No`            | Low direct risk for current Ghostboard feature parity. Debug overlay absence may reduce diagnostics but is not a user workflow blocker.                                                                       | No Ghostboard parity follow-up from this issue. Let open Issue 803 or a later observability issue decide whether a debug overlay is useful.                                                       | Classified `No` because the tested copied Roastty app is not the active restored Ghostboard app.                                                                                           |
| `0803-roastty-debug-overlay`                  | H     | Debug overlay / observability    | A debug overlay can expose internal terminal/render state, but this historical issue remains open and has no experiment evidence.                                                                 | There is no current evidence that restored Ghostboard requires Roastty's optional debug overlay for ordinary browsing. Ghostboard has other geometry instrumentation from Issue 809, but no dedicated audit of a live debug overlay capability in `ghostboard/`.                         | Issue 803 is still open in the inventory; Issue 802 treats Debug `Overlay` as optional in `issues/0802-libroastty-completion-and-mac-app/README.md:1697`; Ghostboard geometry tracing exists in `ghostboard/macos/Sources/App/macOS/AppDelegate+TermSurf.swift`.                                                                                                                                                                                                 | `Maybe`         | Low-to-medium diagnostic risk. If future bugs need app-internal state, missing overlay tooling could slow debugging, but it is not currently proven user-visible breakage.                                    | Defer until a diagnostics/observability issue. If pursued, define the Ghostboard-specific overlay requirement rather than inheriting the Roastty issue blindly.                                   | Classified `Maybe` because the source issue is open and evidence is incomplete, but the only historical blocker was explicitly optional for Roastty.                                       |
| `0804-roastty-gui-automation-readiness`       | H     | GUI automation / permissions     | macOS VM automation works when Accessibility, Automation, Input Monitoring, and Screen Recording are granted, and app-side key path bugs can masquerade as VM permission failures.                | Current Ghostboard has already been exercised by Issue 809's automated geometry matrix, including mouse input and keyboard input after tab/window switching. The VM permission lesson remains operationally important, but it does not indicate a likely current Ghostboard product bug. | Issue 804 conclusion lists required permissions and proves System Events keyboard/mouse automation in `issues/0804-roastty-gui-automation-readiness/README.md:285`; Issue 809 proves Ghostboard mouse and keyboard matrix rows in `issues/0809-ghostboard-viewport-geometry/README.md:233`.                                                                                                                                                                      | `No`            | Low current product risk. The main residual is test-environment setup drift, not an app feature gap.                                                                                                          | Keep using the Issue 804 permission checklist and Issue 809-style harnesses for future Ghostboard GUI tests.                                                                                      | Classified `No` because current Ghostboard automation evidence exists; this row remains a process guard, not a new bug candidate.                                                          |
| `0805-roastty-ghostty-parity`                 | H     | Ghostty parity / config / app QA | Total parity needs source audit, app walkthrough, config parser/formatter/finalization/load/reload matrices, runtime oracles, and explicit divergences.                                           | Current Ghostboard imported Ghostty `v1.3.1` and proved ordinary TermSurf browsing, but it has not had an Issue-805-style full Ghostty feature/config parity certification under the TermSurf app identity and config path.                                                              | Issue 805 final matrices are exhaustive for Roastty in `issues/0805-roastty-ghostty-parity/README.md:1906`; Issue 808 proves ordinary Ghostboard browsing and branding/config path, but only for its closure scope in `issues/0808-recreate-ghostboard-from-ghostty-1-3-1/README.md:286`.                                                                                                                                                                        | `Maybe`         | Medium risk. Ghostboard may still have untested config or native-app edge cases introduced by TermSurf branding, config-path, protocol, and packaging changes.                                                | Add a later Ghostboard parity/config audit modeled on Issue 805, scoped to Ghostty `v1.3.1` plus accepted TermSurf divergences, without duplicating every low-risk upstream Ghostty test blindly. | Classified `Maybe` because upstream import lowers risk, but ordinary browsing proof is narrower than full app/config parity certification.                                                 |
| `0806-roastty-input-latency`                  | H     | Terminal input latency           | Synchronous key paths must avoid blocking on worker locks; latency needs a lightweight runtime guard with a visible-output budget.                                                                | Current Ghostboard is upstream Ghostty Zig, not the Roastty Rust `TermioWorker` implementation. Issue 809 proves keyboard input reaches Roamium after tab/window switching, but it does not measure end-to-end terminal input latency or guard against future latency regressions.       | Issue 806 documents the `34s` Roastty delay, fix, and `2000ms` guard in `issues/0806-roastty-input-latency/README.md:90`; Ghostboard currently forwards browser key events in `ghostboard/src/apprt/termsurf.zig:1452`, and Issue 809 verifies keyboard routing in `issues/0809-ghostboard-viewport-geometry/README.md:238`.                                                                                                                                     | `Maybe`         | Medium-low risk. There is no evidence of a Ghostboard latency bug, but keyboard responsiveness is critical enough that the absence of a cheap guard is worth tracking.                                        | Consider a Ghostboard-specific lightweight input-latency smoke guard after the larger audit identifies all high-priority feature gaps.                                                            | Classified `Maybe` because the exact root cause is Roastty-specific, while the user impact class is still important for Ghostboard.                                                        |
| `0807-restore-ghostboard-code`                | H     | Legacy reference availability    | Archived Ghostboard code should be restored mechanically and preserved as a reference, not modernized in place.                                                                                   | This requirement is already satisfied: `ghostboard-legacy/` exists and current Ghostboard work can compare against it. No active gap is indicated by this historical issue.                                                                                                              | Issue 807 conclusion records restore from `90b966458bd17` and rename to `ghostboard-legacy/` in `issues/0807-restore-ghostboard-code/README.md:98`; the directory exists as `ghostboard-legacy/`.                                                                                                                                                                                                                                                                | `No`            | Low risk. The reference source is available for future audits and fixes.                                                                                                                                      | No follow-up. Continue treating `ghostboard-legacy/` as read-only reference evidence unless a later issue explicitly asks otherwise.                                                              | Classified `No` because the durable lesson is already satisfied in the current tree.                                                                                                       |
| `0808-recreate-ghostboard-from-ghostty-1-3-1` | H     | Restored Ghostboard protocol     | A fresh Ghostty subtree can replace Wezboard for ordinary browsing only after build, branding, config path, protocol lifecycle, overlay presentation, input, DevTools, and `web last` are proven. | Ordinary browsing has strong proof, but the historical closure explicitly leaves lifecycle debt, and this issue's protocol audit found likely missing GUI-responsibility messages. `CursorChanged` and `SetGuiActive` still appear absent from active Ghostboard handling.               | Issue 808 ordinary-browsing proof and socket debt are recorded in `issues/0808-recreate-ghostboard-from-ghostty-1-3-1/README.md:286`; current Ghostboard dispatch handles core messages then ignores unknown cases in `ghostboard/src/apprt/termsurf.zig:490`; `CursorChanged`/`SetGuiActive` appear only in `msgTypeName` at `ghostboard/src/apprt/termsurf.zig:2249`; legacy Ghostboard handled `cursor_changed` at `ghostboard-legacy/src/apprt/xpc.zig:230`. | `Highly likely` | High for protocol polish and lifecycle correctness. Browser cursor shape feedback likely remains wrong, GUI active state is probably not sent to Roamium, and socket cleanup debt can accumulate stale state. | Open focused follow-ups for Ghostboard `CursorChanged`, Ghostboard `SetGuiActive`, and Roamium/browser socket lifecycle cleanup. Keep ordinary browsing marked covered by Issue 808.              | Classified `Highly likely` because the active dispatcher lacks cases for messages that legacy/reference paths or Wezboard handled, while Issue 808's closure scope was ordinary workflows. |
| `0809-ghostboard-viewport-geometry`           | H     | Browser overlay geometry / input | Browser overlays must follow their owning pane across the full geometry matrix, and final proof needs a strict regression sweep.                                                                  | The geometry matrix is currently the strongest Ghostboard-specific proof in Batch H. All tested rows passed, including mouse and keyboard input after geometry changes. The only remaining caveat is that the VM had one display, so actual cross-display movement remains untested.     | Issue 809 conclusion records the full matrix pass and single-display caveat in `issues/0809-ghostboard-viewport-geometry/README.md:231`; current Ghostboard geometry bridge and tracing live in `ghostboard/macos/Sources/App/macOS/AppDelegate+TermSurf.swift`.                                                                                                                                                                                                 | `Maybe`         | Medium-low risk. Geometry is broadly proven, but multi-display/backing-scale behavior could still fail on real multi-monitor setups.                                                                          | Add a later multi-display verification when hardware/VM support exists; keep the Issue 809 matrix as the durable regression reference for all covered geometry rows.                              | Classified `Maybe` only because one matrix dimension was environmentally untested; the tested viewport behavior itself is not a likely gap.                                                |

### Findings Summary

`Highly likely` findings:

- Ghostboard probably needs active `CursorChanged` handling. Current
  `ghostboard/src/apprt/termsurf.zig` dispatch handles core TermSurf lifecycle
  messages but falls through unmatched messages to the ignored branch, while
  `CursorChanged` appears only in `msgTypeName`. Legacy Ghostboard had a
  `cursor_changed` handler.
- Ghostboard probably needs `SetGuiActive` signaling. The message appears in
  current `msgTypeName`, but no active sender/handler evidence was found in
  current Ghostboard.

`Maybe` findings:

- Ghostboard may need a Ghostty `v1.3.1` app/config parity audit modeled on
  Issue 805, scoped carefully to TermSurf's accepted branding and protocol
  divergences.
- Ghostboard may benefit from a lightweight input-latency guard, even though the
  Issue 806 root cause was Roastty-specific.
- Ghostboard may need multi-display/backing-scale verification once a real
  multi-display environment is available.
- A Ghostboard-specific debug overlay remains optional and unproven useful; keep
  it as diagnostics debt, not product parity debt.

`No` findings:

- Roastty architecture, `libroastty`, copied-app, automation-readiness, and
  legacy-restore issues do not directly indicate current Ghostboard feature gaps
  beyond the process lessons recorded above.

### Verification

Commands run:

```bash
for d in issues/080{0,1,2,3,4,5,6,7,8,9}-*; do
  sed -n '/^## Conclusion/,$p' "$d/README.md" | sed -n '1,140p'
done

rg -n \
  "CursorChanged|SetGuiActive|ModeChanged|SetOverlay|BrowserReady|Resize|CreateTab|CloseTab|Query|Devtools|TERMSURF_SOCKET|termsurf" \
  ghostboard/src/apprt/termsurf.zig ghostboard/macos/Sources

rg -n \
  "CursorChanged|SetGuiActive|ModeChanged|SetOverlay|BrowserReady|Resize|CreateTab|CloseTab|Query|Devtools|TERMSURF_SOCKET|termsurf" \
  ghostboard-legacy

prettier --write --prose-wrap always --print-width 80 \
  issues/0810-ghostboard-preventive-parity-audit/README.md \
  issues/0810-ghostboard-preventive-parity-audit/05-batch-h-restored-ghostboard.md

git diff --check
```

Verification results:

- All ten Batch H issues are represented exactly once in the classification
  table.
- Every row uses the Experiment 4 schema.
- Issue 803 is treated as open historical evidence and was not modified.
- No historical issue files, application code, generated code, scripts, or test
  harnesses were edited.
- Markdown formatting passed.
- Whitespace check passed.

## Conclusion

Batch H identifies one high-confidence restored-Ghostboard follow-up area:
protocol messages that require GUI responsibility and are not covered by the
direct webtui-to-Roamium path. `CursorChanged` and `SetGuiActive` should be
prioritized after this audit, along with the known Roamium socket lifecycle debt
from Issue 808.

The next audit slice should move backward to Batch G (`0789`-`0799`), because it
covers PDF/browser automation and browser API coverage immediately before the
Roastty/Ghostboard restoration arc.

## Completion Review

Fresh-context adversarial completion review returned **APPROVED**.

Reviewer checks confirmed:

- Batch H `0800`-`0809` appears exactly once each.
- Rows follow the Experiment 4 schema.
- Issue 803 remains open evidence only.
- The `0808` `Highly likely` and `0809` `Maybe` classifications are defensible
  from cited evidence.
- The README marks Experiment 5 as `Pass`.
- Only Issue 810 docs are changed.
- `git diff --check` passes.
- The result commit had not yet been made before review.

Findings: none.
