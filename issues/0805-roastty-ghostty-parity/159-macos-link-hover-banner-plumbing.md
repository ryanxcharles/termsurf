# Experiment 159: macOS Link Hover Banner Plumbing

## Description

`RUNTIME-012B2B2B2B2` still contains several link-related GUI effects. One
narrow source-parity slice is already present in the copied macOS app: the
`mouse_over_link` app action updates a surface's `hoverUrl`, `OSSurfaceView`
publishes it, `SurfaceView` renders `URLHoverBanner`, and `URLHoverBanner`
matches pinned Ghostty after expected `Ghostty`/`Roastty` and C-symbol renames.

This experiment will split that copied macOS link-hover banner plumbing out as
Oracle complete. It will not claim that real mouse movement over terminal links,
link preview policy, pointer cursor changes, or context menus have been proven
in the running app; those remain in the reduced notification/link/bell GUI gap.

## Changes

- Add a static source-parity guard:
  - `issues/0805-roastty-ghostty-parity/macos_link_hover_banner_runtime_parity.py`
  - Compare the pinned Ghostty and Roastty Swift link-hover plumbing after
    expected renames:
    - `OSSurfaceView.hoverUrl`;
    - `SurfaceView` rendering `URLHoverBanner(url:)`;
    - `URLHoverBanner.swift` layout and hover-side switching;
    - `Ghostty.App.swift` / `Roastty.App.swift` `mouse_over_link` action
      dispatch and `setMouseOverLink` implementation.
  - Assert the remaining gap still includes actual link hover/cursor UI, link
    previews, and context/menu link flows.
- Update `config_runtime_inventory.py` to split `RUNTIME-012B2B2B2B2` into:
  - an Oracle complete copied macOS link-hover banner plumbing row owned by this
    experiment;
  - a narrower remaining notification/link/bell GUI gap for actual OS
    notification delivery, actual bell side effects, real app link hover/cursor
    UI, link previews, and context/menu link flows.
- Regenerate `config-runtime-inventory.md` and `config-matrix.md`.
- Update existing runtime parity guards and `terminal_runtime_residual_audit.py`
  for the new CFG-223 row counts and remaining gap id.
- Update Issue 805 learnings after the result is known.

## Verification

Pass criteria:

- The new static parity guard passes:

```bash
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/macos_link_hover_banner_runtime_parity.py
```

- The runtime inventory generator reports one additional Oracle-complete row and
  the same total number of unresolved CFG-223 gaps unless implementation
  uncovers a real additional gap. Expected output after this split:
  `runtime_rows=67`, `oracle_complete=60`, `closed=63`, `incomplete=4`, `gap=4`,
  and `cfg223=Gap`.

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
prettier --check --prose-wrap always --print-width 80 issues/0805-roastty-ghostty-parity/README.md issues/0805-roastty-ghostty-parity/159-macos-link-hover-banner-plumbing.md
git diff --check
```

## Design Review

**Reviewer:** Hegel the 2nd

**Verdict:** Approved

The fresh-context design review found no required issues. It confirmed the
README links the experiment as `Designed`, the design has the required sections,
the scope stays limited to copied macOS hover-banner plumbing, and the expected
CFG-223 count change is coherent.
