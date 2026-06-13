# Experiment 6: Config Option Inventory

## Description

Issue 805 cannot close while `config-matrix.md` contains only the temporary
baseline config-file path row. The next source-audit step is to inventory every
configuration option exposed by pinned Ghostty commit
`2c62d182cec246764ff725096a70b9ef44996f7f`, compare it with Roastty's current
config struct, and populate the config matrix with explicit rows that future
experiments can prove or fix one group at a time.

This experiment is an inventory and classification step. It should not claim
behavioral parity for options that have only been found by name. Each option
must leave the experiment as one of:

- represented by Roastty but not behavior-proven yet, and therefore a `Gap`;
- missing from Roastty and therefore a `Gap`;
- platform-specific or otherwise `Not applicable`, with a concrete reason;
- already proven by an existing test/log, with evidence and a guard.

Ghostty compatibility aliases are in scope for this inventory. They must be
counted, listed, and classified separately from canonical config fields so the
matrix can distinguish current option names from accepted legacy spellings.

The C config access surface (`roastty_config_get`, diagnostics, load/finalize
functions, and related structs) remains out of scope for this experiment because
it needs a different oracle than option-name inventory. A later config
experiment should audit that API surface after the option inventory is stable.

## Changes

- `issues/0805-roastty-ghostty-parity/config-matrix.md`
  - Add one row per upstream Ghostty canonical config option or tightly grouped
    alias set only when grouping preserves traceability.
  - Include compatibility aliases either as their own rows or as explicit alias
    lists attached to the canonical row.
  - Keep `CFG-001` as the baseline path row, but stop treating it as sufficient
    config parity proof.
  - For every new row, fill the full matrix schema: status, evidence, guard
    tier, guard command, cadence, sufficiency, owner experiment, and notes.
  - Mark represented-but-unproven options as `Gap`, not as `Pass`.
- `issues/0805-roastty-ghostty-parity/config-inventory.md`
  - Create a supporting artifact that records the extraction commands, raw
    counts, upstream canonical key list, Ghostty compatibility alias list,
    private/internal Ghostty field list, Roastty key list, missing/extra keys,
    and classification notes.
- `issues/0805-roastty-ghostty-parity/README.md`
  - Add a learning about reliable config inventory extraction if the experiment
    establishes one.
- `issues/0805-roastty-ghostty-parity/config_inventory.py`
  - Add a small checked-in helper that extracts config keys from the two source
    files and emits deterministic sorted lists.
  - Use conservative bounded line-oriented extraction instead of scanning all of
    `Config.zig`, because broad scans can include unrelated nested helper data
    such as enum values, loop variables, and diagnostics formatting fields.
  - Extract only the top-level Ghostty `Config` fields before the first
    implementation method. It must not continue into nested helper structs later
    in `Config.zig`.
  - Extract Ghostty compatibility aliases from the static compatibility map near
    the top of `vendor/ghostty/src/config/Config.zig`.
  - Private/internal Ghostty fields such as `_arena`, `_diagnostics`, and
    `_conditional_state` are not user config options. They should be counted and
    classified in `config-inventory.md`, but they should not become ordinary
    config matrix rows.
  - Extract Roastty keys only from `pub(crate) struct Config` in
    `roastty/src/config/mod.rs`.

No Roastty behavior changes are planned for this experiment. If the inventory
finds config gaps, record them and let a later experiment fix one coherent
group.

## Verification

Pass/fail criteria:

- The experiment records the exact upstream Ghostty config option count and
  Roastty config option count.
- The experiment records the exact Ghostty compatibility alias count and
  private/internal Ghostty field count.
- Every upstream canonical option and compatibility alias from
  `vendor/ghostty/src/config/Config.zig` is either represented in
  `config-matrix.md` or explicitly classified in `config-inventory.md` with a
  reason that explains why it is not a matrix row.
- The inventory identifies missing Roastty options, Roastty-only extras, and
  platform-specific options without treating name presence as behavioral proof.
- Represented-but-unproven options are marked `Gap` until later experiments
  prove defaults, parsing, formatting, diagnostics, precedence, and runtime
  effects.
- The C config access surface is explicitly named as out of scope and queued for
  a later config experiment.
- The matrix rows include regression guard fields and do not use vague evidence
  such as "source seems present".
- The helper output is manually spot-checked against representative upstream
  canonical fields, compatibility aliases, private/internal fields, and Roastty
  fields.
- Markdown formatting and diff hygiene pass.

Suggested commands:

```bash
python3 issues/0805-roastty-ghostty-parity/config_inventory.py \
  --upstream vendor/ghostty/src/config/Config.zig \
  --roastty roastty/src/config/mod.rs \
  --output issues/0805-roastty-ghostty-parity/config-inventory.md

prettier --write --prose-wrap always --print-width 80 \
  issues/0805-roastty-ghostty-parity/06-config-option-inventory.md \
  issues/0805-roastty-ghostty-parity/README.md \
  issues/0805-roastty-ghostty-parity/config-matrix.md \
  issues/0805-roastty-ghostty-parity/config-inventory.md

git diff --check
```

If the suggested extractor produces obvious false positives or false negatives,
replace it with a checked-in helper and record the reason in
`config-inventory.md`.

## Design Review

Fresh-context adversarial design review returned `CHANGES REQUIRED` with four
required findings:

- The draft did not require compatibility aliases from Ghostty's compatibility
  map to be inventoried.
- The suggested broad extractor could scan unrelated nested data in `Config.zig`
  and produce false positives.
- The draft introduced an unallowed status bucket for represented-but-unproven
  options.
- The description included C config access surface comparison, but the planned
  changes and verification did not cover it.

Fixes made before re-review:

- Compatibility aliases are now explicitly in scope and must be counted, listed,
  and classified.
- The experiment now requires a checked-in bounded helper that extracts only the
  top-level Ghostty `Config` fields, the static compatibility map, and Roastty's
  `pub(crate) struct Config`.
- Represented-but-unproven options are classified as `Gap`.
- C config access is explicitly out of scope for this experiment and queued for
  a later config audit.

Fresh-context adversarial re-review approved the revised design with no
findings.

## Result

**Result:** Pass

Implemented the bounded config inventory helper and generated the inventory
artifacts.

Verification results:

- `logs/issue805-exp6-config-inventory.log` records:
  - `ghostty_canonical=203`
  - `ghostty_aliases=8`
  - `ghostty_internal=6`
  - `roastty=203`
  - `represented=203`
  - `missing=0`
  - `extra=0`
- `config-inventory.md` records the sorted canonical option list, compatibility
  alias list, private/internal field list, Roastty option list, and
  missing/extra sets.
- `config-matrix.md` now has 212 rows total: `CFG-001`, 203 canonical option
  rows, and 8 compatibility alias rows.
- `logs/issue805-exp6-config-spot-checks.log` records manual spot checks for:
  - all eight Ghostty compatibility aliases;
  - all six private/internal Ghostty fields;
  - representative canonical fields from both Ghostty and Roastty;
  - matrix row counts.
- `python3 -m py_compile issues/0805-roastty-ghostty-parity/config_inventory.py`
  passed.
- `prettier --write --prose-wrap always --print-width 80` passed for the edited
  markdown files.
- `git diff --check` passed.

The config matrix rows intentionally remain `Gap` except for `CFG-001`. This
experiment proves only name-level inventory coverage, not defaults, parsing,
formatting, diagnostics, precedence, runtime effects, or compatibility alias
semantics.

## Conclusion

Roastty has documented names for all 203 canonical Ghostty config options at the
pinned commit, and Ghostty's 8 compatibility aliases are now explicitly tracked.
The next config experiments should prove behavior by coherent groups, starting
with parser/default/formatter behavior or compatibility alias handling, rather
than relying on name presence.

## Completion Review

Fresh-context adversarial completion review approved the result with no required
findings.
