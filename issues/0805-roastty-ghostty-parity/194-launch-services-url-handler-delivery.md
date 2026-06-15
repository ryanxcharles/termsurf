# Experiment 194: Launch Services URL handler delivery

## Description

Experiment 193 closed live Quick Look/native definition UI. The remaining
`RUNTIME-012B2B2B2B2B3C` gap is now limited to OS-controlled notification,
audible bell, Dock-attention, and external URL-handler effects. Experiment 187
proved Roastty constructs the expected native URL-opening request, but it
intentionally set `ROASTTY_UI_TEST_SUPPRESS_OPEN_URL=1`, so it did not prove
that `NSWorkspace.open` hands the URL to an external Launch Services handler.

This experiment targets only external Launch Services handler delivery. The goal
is to prove that the unsuppressed Roastty `Roastty.App.openURL` path can deliver
a private-scheme URL to a controlled local handler app, or to document the exact
macOS/VM boundary that prevents deterministic proof.

## Changes

- Add a focused live guard, tentatively
  `issues/0805-roastty-ghostty-parity/macos_launch_services_url_handler_delivery.py`.
  - Build a temporary controlled handler app for a unique per-run private scheme
    such as `termsurf-issue805-exp194-<token>://` and a matching unique bundle
    ID.
  - The handler must record the exact delivered URL to a test-owned evidence
    file and exit without opening an uncontrolled browser.
  - Register the handler with Launch Services and set it as the default handler
    for the private scheme, using system APIs or command-line tools available on
    the VM.
  - Before involving Roastty, run a direct `open <private-url>` sanity check and
    require the handler to record the exact URL. If this fails, the experiment
    must stop there and record the Launch Services registration/delivery
    boundary as the remaining gap.
  - Launch the built debug Roastty app with isolated config/defaults,
    `macos-applescript = true`, `ROASTTY_UI_TEST_ENABLE_OPEN_URL_ACTION=1`, and
    a trace path.
  - Invoke the existing AppleScript `perform action "ui_test_open_url:<url>"`
    hook without `ROASTTY_UI_TEST_SUPPRESS_OPEN_URL`.
  - Require Roastty trace evidence for the exact unsuppressed URL request and
    require the controlled handler evidence file to contain the exact same URL.
  - Check for new Roastty crash reports, terminate all helper/app processes
    started by the guard, and avoid stale Launch Services state by using the
    unique per-run scheme/bundle ID rather than a reusable global test scheme.
- Update `config_runtime_inventory.py` according to the result:
  - If handler delivery passes, split a new Oracle-complete row from
    `RUNTIME-012B2B2B2B2B3C` for external Launch Services handler delivery.
  - Keep `RUNTIME-012B2B2B2B2B3C` as a `Gap` only for actual OS notification
    delivery/banner/sound after authorization is available, audible bell output,
    and OS-visible Dock attention bounce/state beyond AppKit request dispatch.
  - If handler registration or direct `open` delivery fails in this VM, leave an
    exact gap row naming that boundary and do not claim Roastty parity for
    external handler delivery.
- Update `notification_link_bell_gui_residual_parity.py` to enforce the new row
  split or exact failure boundary and reject stale wording that keeps external
  Launch Services handler delivery inside a broad residual after it is proven.
- Regenerate `config-runtime-inventory.md` and `config-matrix.md`.
- Update Issue 805 `README.md` Learnings and Experiments index after the result
  is known.

## Verification

Pass criteria:

- The guard proves the controlled handler was registered for the private URL
  scheme and that direct `open <private-url>` reaches the handler before Roastty
  is involved.
- The guard proves Roastty launched from the built debug app with isolated
  config/defaults and AppleScript enabled.
- The guard invokes the existing `ui_test_open_url:<url>` action without
  `ROASTTY_UI_TEST_SUPPRESS_OPEN_URL`.
- The guard proves Roastty emitted the exact `openURL url=<private-url>` trace
  and did not emit `openURL suppressed=true`.
- The controlled handler records the exact same private URL after Roastty calls
  the unsuppressed `NSWorkspace.open` path.
- The result does not claim actual OS notification delivery, notification
  banner/sound, audible bell output, or OS-visible Dock attention bounce/state.
- Inventory counts and remaining gap IDs are updated exactly and asserted by
  guards.

Commands:

```bash
(cd roastty && macos/build.nu --action build)
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/macos_launch_services_url_handler_delivery.py
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/notification_link_bell_gui_residual_parity.py
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_runtime_inventory.py --output issues/0805-roastty-ghostty-parity/config-runtime-inventory.md --matrix issues/0805-roastty-ghostty-parity/config-matrix.md
python3 -m py_compile issues/0805-roastty-ghostty-parity/*.py
rm -rf issues/0805-roastty-ghostty-parity/__pycache__
prettier --check issues/0805-roastty-ghostty-parity/README.md issues/0805-roastty-ghostty-parity/194-launch-services-url-handler-delivery.md issues/0805-roastty-ghostty-parity/config-runtime-inventory.md issues/0805-roastty-ghostty-parity/config-matrix.md
git diff --check
```

The result must state the exact runtime row count, Oracle-complete count, closed
count, incomplete count, gap count, CFG-223 status, and remaining gap IDs.

## Design Review

Fresh-context Codex adversarial reviewer `Ohm the 3rd` reviewed the design
against the Issue 805 workflow, the remaining CFG-223 residual, prior Experiment
187 URL-opening proof, and the Roastty `openURL`/AppleScript source.

Verdict: **Approved**.

Optional finding accepted and fixed: the initial design registered a temporary
Launch Services handler but did not specify how to avoid stale default-handler
state across reruns. The design now requires a unique per-run private URL scheme
and bundle ID, so each run proves delivery through its own handler mapping.

## Result

**Result:** Pass

The experiment proved external Launch Services handler delivery for Roastty's
unsuppressed URL-open path.

Implementation changes:

- Added `macos_launch_services_url_handler_delivery.py`.
- The guard builds a per-run native Cocoa `.app` URL handler under `logs/`, with
  a unique private URL scheme and bundle ID.
- The guard registers the handler with Launch Services, sets it as the default
  handler for that private scheme, and first proves direct `open <private-url>`
  delivery to the controlled handler.
- The guard then launches the built debug Roastty app with isolated
  config/defaults and AppleScript enabled, invokes
  `perform action "ui_test_open_url:<url>"` without
  `ROASTTY_UI_TEST_SUPPRESS_OPEN_URL`, requires the unsuppressed Roastty
  `openURL url=<private-url> kind=unknown` trace, and requires the controlled
  handler to record the same URL after Roastty calls
  `NSWorkspace.shared.open(url)`.

Important learning: handler apps created under the per-user temp directory
returned successful `LSRegisterURL`/`LSSetDefaultHandlerForURLScheme` statuses,
but direct `open <private-url>` still failed with `kLSApplicationNotFoundErr`.
Creating the per-run handler under `logs/` produced the expected current handler
bundle ID and deterministic delivery.

The passing guard recorded:

- `registerStatus=0`;
- `status=0`;
- current default handler equal to the per-run handler bundle ID;
- direct Launch Services delivery of the private URL to the handler;
- Roastty trace `openURL url=<private-url> kind=unknown`;
- no `openURL suppressed=true` trace;
- Roastty Launch Services delivery of the same private URL to the handler;
- no new Roastty crash report.

The inventory now splits `RUNTIME-012B2B2B2B2B3C9` as Oracle complete for
external Launch Services URL handler delivery through an unsuppressed
`NSWorkspace.shared.open(url)` call. `RUNTIME-012B2B2B2B2B3C` remains a `Gap`
only for actual OS notification delivery/banner/sound after authorization is
available, audible bell output, and OS-visible Dock attention bounce/state
beyond AppKit request dispatch.

The regenerated CFG-223 counts are:

- runtime rows: 98
- Oracle complete: 94
- closed: 97
- audit covered: 0
- incomplete: 1
- runtime gaps: 1
- CFG-223 status: `Gap`

Verification logs:

- `logs/issue805-exp194-build-1.log`
- `logs/issue805-exp194-launch-services-8.log`
- `logs/issue805-exp194-config-runtime-inventory-2.log`
- `logs/issue805-exp194-residual-guard-2.log`

Additional verification:

- `python3 -m py_compile issues/0805-roastty-ghostty-parity/macos_launch_services_url_handler_delivery.py`
- `prettier --check issues/0805-roastty-ghostty-parity/README.md issues/0805-roastty-ghostty-parity/194-launch-services-url-handler-delivery.md issues/0805-roastty-ghostty-parity/config-runtime-inventory.md issues/0805-roastty-ghostty-parity/config-matrix.md`
- `git diff --check`

## Conclusion

Experiment 194 closed the external Launch Services URL handler delivery slice.
The remaining CFG-223 residual is now limited to OS notification
delivery/banner/sound, audible bell output, and OS-visible Dock attention
bounce/state beyond AppKit request dispatch.

## Completion Review

Fresh-context Codex adversarial reviewer `Franklin the 3rd` reviewed the
completed experiment, working-tree diff, new guard, issue/inventory updates, and
verification logs.

Verdict: **Approved**.

Findings: none.

The reviewer confirmed the result commit had not been made yet, the Launch
Services log shows matching direct and Roastty handler deliveries with no
suppressed trace and no crash reports, the inventory counts are
`runtime_rows=98`, `oracle_complete=94`, `closed=97`, `incomplete=1`, `gap=1`,
`cfg223=Gap`, and formatting/whitespace checks passed. The reviewer used a
read-only no-bytecode AST compile check instead of `py_compile`.
