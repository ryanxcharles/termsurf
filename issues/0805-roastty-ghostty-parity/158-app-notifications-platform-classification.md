# Experiment 158: App Notifications Platform Classification

## Description

`RUNTIME-012B2B2B2B` still lists `app-notifications` alongside macOS
notification, bell, and link GUI effects. Pinned Ghostty's `app-notifications`
config, however, is explicitly documented as GTK-only and is only consumed by
GTK runtime code (`src/apprt/gtk`). Roastty is the copied macOS app plus
`libroastty`; there is no GTK in-app toast runtime to reproduce on macOS.

This experiment will split the GTK-only `app-notifications` runtime effect out
of the remaining notification/link/bell GUI gap as `Not applicable`, while
keeping Roastty's parser/formatter coverage and leaving the actual macOS
notification, bell, link hover, link preview, and context/menu GUI effects in
the remaining gap.

## Changes

- Add a static parity guard:
  - `issues/0805-roastty-ghostty-parity/app_notifications_platform_runtime_parity.py`
  - Assert pinned Ghostty documents `app-notifications` as GTK-only.
  - Assert pinned Ghostty runtime consumption is limited to
    `vendor/ghostty/src/apprt/gtk`.
  - Assert pinned Ghostty macOS sources do not consume `app-notifications`.
  - Assert Roastty has parser/formatter coverage for `app-notifications` but no
    macOS runtime consumer.
- Update `config_runtime_inventory.py` to split `RUNTIME-012B2B2B2B` into:
  - a `Not applicable` `app-notifications` GTK-only runtime row owned by this
    experiment;
  - a narrower remaining notification/link/bell GUI gap row for live OS
    notification delivery, actual bell side effects, link hover/cursor UI, link
    previews, and context/menu link flows.
- Regenerate `config-runtime-inventory.md` and `config-matrix.md`.
- Update existing runtime parity guards and `terminal_runtime_residual_audit.py`
  for the new CFG-223 row counts and remaining notification/link/bell gap id.
- Update Issue 805 learnings after the result is known.

## Verification

Pass criteria:

- The new static parity guard passes:

```bash
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/app_notifications_platform_runtime_parity.py
```

- Existing app-notifications parser/formatter coverage still passes:

```bash
cargo test --manifest-path roastty/Cargo.toml app_notifications
```

- The runtime inventory generator reports one additional closed row and the same
  total number of unresolved CFG-223 gaps unless implementation uncovers a real
  additional gap. Expected output after this split: `runtime_rows=66`,
  `oracle_complete=59`, `closed=62`, `incomplete=4`, `gap=4`, and `cfg223=Gap`.

```bash
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_runtime_inventory.py --output issues/0805-roastty-ghostty-parity/config-runtime-inventory.md --matrix issues/0805-roastty-ghostty-parity/config-matrix.md
```

- All runtime parity guards and the terminal residual audit still pass:

```bash
for guard in issues/0805-roastty-ghostty-parity/*_runtime_parity.py; do
  PYTHONDONTWRITEBYTECODE=1 python3 "$guard" || exit 1
done
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/terminal_runtime_residual_audit.py
```

- Markdown formatting and diff hygiene pass:

```bash
prettier --check --prose-wrap always --print-width 80 issues/0805-roastty-ghostty-parity/README.md issues/0805-roastty-ghostty-parity/158-app-notifications-platform-classification.md
git diff --check
```

## Design Review

**Reviewer:** Russell the 2nd

**Verdict:** Approved

The reviewer found no required issues. One optional hygiene note pointed out
that `prettier --write` is mutating and should not be listed as a pure pass/fail
verification command. The design was updated to use `prettier --check` in the
verification section.
