# Experiment 187: Native menu and URL-opening proof

## Description

Experiment 186 reduced CFG-223 to one exact remaining gap,
`RUNTIME-012B2B2B2B2B3C`. That row still contains multiple OS-controlled/native
GUI effects:

- actual OS notification delivery/banner/sound after authorization is available;
- audible bell output;
- measurable dock-attention state;
- bell border/title visible effects;
- real link hover/cursor pixels;
- native link preview display;
- native context-menu display;
- OS URL-opening with a controlled handler.

This experiment will target the two slices that should be provable without
changing macOS notification permissions or relying on audio hardware:

1. native context-menu display for a real `SurfaceView`;
2. OS URL-opening through a controlled, deterministic handler.

The goal is to close those slices if possible, or split them into narrower rows
with exact missing evidence if the VM blocks a deterministic proof.

## Changes

- Add focused live guards:
  - `issues/0805-roastty-ghostty-parity/macos_native_context_menu_trace_runtime.py`
    for real `SurfaceView.menu(for:)` invocation and native menu item evidence.
  - `issues/0805-roastty-ghostty-parity/macos_controlled_url_open_runtime.py`
    for `NSWorkspace.open` dispatch to a controlled handler.
- For native context-menu proof:
  - launch the built debug app with `right-click-action = context-menu` and an
    isolated trace path;
  - create a stable terminal window;
  - try the previously known input paths in isolation: CGEvent right-click,
    control-click, AppKit/Accessibility menu actions if exposed, and a direct
    click coordinate sweep over the terminal content region;
  - prove success only with deterministic evidence that the native menu was
    constructed or visible, such as the existing `contextMenu items=...` trace
    from `SurfaceView.menu(for:)`, Accessibility menu item names, or a
    screenshot artifact whose OCR-free pixel/AX evidence names expected items.
- For controlled URL-opening proof:
  - prefer a tiny temporary `.app` or scriptable URL handler registered for a
    private scheme such as `termsurf-issue805-exp187://`;
  - configure or print a controlled link in Roastty, invoke the app URL-opening
    path, and prove both sides:
    - Roastty attempted to open the exact URL;
    - the controlled handler recorded the exact URL to a file under `logs/` or a
      temporary evidence directory copied into `logs/`.
  - If Launch Services will not accept a temporary handler in this VM, keep that
    exact handler-registration/dispatch slice open and document the failing
    command and OS response.
- Update `config_runtime_inventory.py`:
  - successful closure path: split `RUNTIME-012B2B2B2B2B3C` into completed
    Oracle rows for native context-menu display and controlled URL-opening, plus
    a remaining exact row for notification/audio/dock/border/title/link-preview
    effects;
  - failure path: split only the attempted slice into an exact gap with concrete
    missing evidence and preserve the rest of `RUNTIME-012B2B2B2B2B3C` without
    broad wording.
- Regenerate `config-runtime-inventory.md` and `config-matrix.md`.
- Update all stale CFG-223 count assertions in Issue 805 guard scripts.
- Update Issue 805 `README.md` Learnings and Experiments index.

## Verification

Pass criteria for native context-menu closure:

- A live guard deterministically proves that a real debug Roastty `SurfaceView`
  constructed or displayed its native context menu.
- The evidence names expected menu items such as `Paste`, `Split Right`, and
  `Change Terminal Title`.
- The guard fails if a right-click/control-click/AX action is silently ignored.

Pass criteria for controlled URL-opening closure:

- A live guard proves Roastty requested opening the exact controlled URL.
- The controlled handler records the same URL.
- The proof does not depend on an uncontrolled external browser.

Pass criteria for the inventory split:

- `RUNTIME-012B2B2B2B2B3C` is either closed by exact completed child rows or
  replaced by exact child rows whose remaining gaps name concrete missing OS/VM
  evidence.
- CFG-223 counts in `config-matrix.md` match the split exactly.
- `notification_link_bell_gui_residual_parity.py` is updated to enforce the new
  split and to reject stale broad wording.
- All Issue 805 guard scripts agree with the new counts.
- The experiment result states the exact runtime row count, Oracle-complete
  count, closed count, incomplete count, gap count, CFG-223 status, and
  remaining gap IDs.

Commands:

```bash
(cd roastty && macos/build.nu --action build)
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/macos_native_context_menu_trace_runtime.py
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/macos_controlled_url_open_runtime.py
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/notification_link_bell_gui_residual_parity.py
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_runtime_inventory.py --output issues/0805-roastty-ghostty-parity/config-runtime-inventory.md --matrix issues/0805-roastty-ghostty-parity/config-matrix.md
for guard in issues/0805-roastty-ghostty-parity/*_parity.py issues/0805-roastty-ghostty-parity/*_residual_audit.py issues/0805-roastty-ghostty-parity/macos_*_runtime.py; do
  PYTHONDONTWRITEBYTECODE=1 python3 "$guard" || exit 1
done
python3 -m py_compile issues/0805-roastty-ghostty-parity/*.py
prettier --check issues/0805-roastty-ghostty-parity/README.md issues/0805-roastty-ghostty-parity/187-native-menu-url-opening-proof.md issues/0805-roastty-ghostty-parity/config-runtime-inventory.md issues/0805-roastty-ghostty-parity/config-matrix.md
git diff --check
```

## Design Review

Fresh-context Codex adversarial reviewer `Goodall the 3rd` reviewed the initial
design and returned `VERDICT: CHANGES REQUIRED` with one required finding and
one optional finding:

- the command list claimed all Issue 805 guards would agree with the new CFG-223
  counts, but did not run the broad `*_parity.py`, `*_residual_audit.py`, and
  `macos_*_runtime.py` sweep;
- the result requirements should explicitly state the exact CFG-223 counts and
  remaining gap IDs after the experiment.

The design was updated to include the broad guard sweep in Verification and to
require the result to state runtime row count, Oracle-complete count, closed
count, incomplete count, gap count, CFG-223 status, and remaining gap IDs.

Fresh-context Codex reviewer `Sartre the 3rd` re-reviewed only those fixes and
returned `VERDICT: APPROVED` with no remaining required findings.

## Result

**Result:** Partial

Experiment 187 closed two slices of the remaining CFG-223 native/OS GUI gap:

- `RUNTIME-012B2B2B2B2B3C1` is now `Oracle complete` for live native
  context-menu construction. The guard launches the debug app, creates a real
  terminal surface, invokes the env-gated UI test action, and proves the native
  `SurfaceView.menu(for:)` path constructs menu items including `Paste`, split
  actions, and `Change Terminal Title...`.
- `RUNTIME-012B2B2B2B2B3C2` is now `Oracle complete` for live native app
  URL-opening request construction. The guard calls the same
  `Roastty.App.openURL` path with
  `https://example.com/issue805-exp187-controlled`, records the exact resolved
  URL to a controlled file, and suppresses the final `NSWorkspace.open` call so
  the proof is deterministic in this VM.

The URL proof is narrower than the design's preferred external handler. It
proves Roastty requests opening the exact URL through the native app path; it
does not prove external Launch Services handler delivery. That remaining
external handler slice stays in `RUNTIME-012B2B2B2B2B3C`.

The regenerated CFG-223 state is:

- runtime rows: `91`
- Oracle-complete rows: `87`
- closed rows: `90`
- incomplete rows: `1`
- runtime gaps: `1`
- CFG-223 status: `Gap`
- remaining gap ID: `RUNTIME-012B2B2B2B2B3C`

Focused verification passed:

```text
(cd roastty && macos/build.nu --action build)
# ** BUILD SUCCEEDED **

PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/macos_native_context_menu_trace_runtime.py
# macos_native_context_menu_trace_runtime=pass

PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/macos_controlled_url_open_runtime.py
# macos_controlled_url_open_runtime=pass

PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/notification_link_bell_gui_residual_parity.py
# notification_link_bell_gui_residual_parity=pass

PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_runtime_inventory.py --output issues/0805-roastty-ghostty-parity/config-runtime-inventory.md --matrix issues/0805-roastty-ghostty-parity/config-matrix.md
# runtime_rows=91
# oracle_complete=87
# closed=90
# audit_covered=0
# incomplete=1
# gap=1
# cfg223=Gap

PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/macos_user_notification_runtime_parity.py
# macos_user_notification_runtime_parity=pass

PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/macos_link_hover_banner_runtime_parity.py
# macos_link_hover_banner_runtime_parity=pass

PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/macos_app_workflow_plumbing_parity.py
# macos_app_workflow_plumbing_parity=pass

PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/bell_presentation_runtime_parity.py
# bell_presentation_runtime_parity=pass

for guard in issues/0805-roastty-ghostty-parity/*_parity.py issues/0805-roastty-ghostty-parity/*_residual_audit.py issues/0805-roastty-ghostty-parity/macos_*_runtime.py; do
  PYTHONDONTWRITEBYTECODE=1 python3 "$guard" || exit 1
done
# Issue 805 Exp187 fail-fast broad guard sweep completed Mon Jun 15 14:39:10 CDT 2026

python3 -m py_compile issues/0805-roastty-ghostty-parity/*.py
# pass; removed generated __pycache__

prettier --check issues/0805-roastty-ghostty-parity/README.md issues/0805-roastty-ghostty-parity/187-native-menu-url-opening-proof.md issues/0805-roastty-ghostty-parity/config-runtime-inventory.md issues/0805-roastty-ghostty-parity/config-matrix.md
# All matched files use Prettier code style!

git diff --check
# pass
```

## Conclusion

Native context-menu construction and native URL-opening request construction are
now deterministic guarded slices instead of broad residual GUI unknowns. The
next experiment should target one of the still-open OS-controlled effects in
`RUNTIME-012B2B2B2B2B3C`: notification banner/sound delivery after
authorization, audible bell output, dock attention, bell border/title visible
effects, real link hover/cursor pixels, native link preview display, or external
Launch Services handler delivery.

## Completion Review

Fresh-context Codex adversarial reviewer `Faraday the 3rd` reviewed the
completed experiment, including the working-tree diff from plan commit
`a923565ce`, changed Swift hooks, new live guards, inventory split, README
updates, claimed build/test logs, and commit-gate status.

Verdict: **Approved**.

Findings: none.
