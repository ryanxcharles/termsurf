# Experiment 5: Resolve Non-App Embedded ABI Functions

## Description

Experiment 4 proved the app-facing embedded ABI slice, but the full mapped
Ghostty header still has four public functions that Roastty has not declared and
exported:

- `roastty_app_open_config`
- `roastty_benchmark_cli`
- `roastty_inspector_metal_shutdown`
- `roastty_translate`

This experiment resolves that remaining source-audit gap by comparing each
function against pinned Ghostty commit
`2c62d182cec246764ff725096a70b9ef44996f7f`, adding ABI-compatible Roastty header
declarations and runtime exports where appropriate, and documenting any semantic
divergence that cannot honestly be called parity.

The goal is not to make Roastty's larger C ABI byte-for-byte identical to
Ghostty's header. The goal is to ensure every mapped upstream public function is
either present with equivalent behavior, explicitly not applicable, or recorded
as an intentional divergence with a durable guard.

## Changes

- `roastty/include/roastty.h`
  - Add declarations for the missing mapped public functions when the audit
    determines they apply to Roastty.
  - Keep declarations aligned with the existing Roastty naming and API macro
    style.
- `roastty/src/lib.rs`
  - Add `#[no_mangle] pub extern "C"` exports for applicable declarations.
  - Implement `roastty_app_open_config` by dispatching the existing
    `ROASTTY_ACTION_OPEN_CONFIG` app action through Roastty's runtime action
    callback, matching Ghostty's `performAction(.app, .open_config, {})`
    behavior.
  - Implement `roastty_inspector_metal_shutdown` as an ABI-safe shutdown hook
    for Roastty's current inspector implementation, which has no Metal backend
    state after `roastty_inspector_metal_init` returns unsupported.
  - Implement `roastty_translate` only if an equivalent localization mechanism
    exists. If Roastty has no loaded translation catalog, expose an identity
    fallback and document the semantic limit rather than claiming full
    localization parity.
  - Resolve `roastty_benchmark_cli` with one of two valid outcomes:
    - port behavior equivalent to Ghostty's benchmark C API; or
    - declare/export a deliberate false-returning no-benchmark implementation
      and record it as an `Intentional divergence` with user impact, reason,
      guard, and owner experiment. This function may not be classified as
      `Not applicable` while the copied Roastty macOS benchmark test references
      it behind `ROASTTY_ENABLE_BENCHMARKS`.
- `roastty/tests/abi_harness.c`
  - Add compile/link coverage for any new header declarations where practical,
    so the C ABI harness catches future declaration/export drift.
- `issues/0805-roastty-ghostty-parity/abi-app-symbols.md`
  - Update the full-header delta table from Experiment 4 with the final
    classification for each of the four functions.
- `issues/0805-roastty-ghostty-parity/source-audit.md`
  - Update `SRC-004` from `Gap` only if all four functions have been fixed or
    explicitly accepted as not applicable/intentional divergence.
  - Add a separate source-audit or divergence row if a function's symbol exists
    but its runtime semantics remain intentionally narrower than Ghostty.
- `issues/0805-roastty-ghostty-parity/divergences.md`
  - Record any accepted localization, inspector, or benchmark divergence with
    user impact, reason, guard, and owner experiment.
- `issues/0805-roastty-ghostty-parity/README.md`
  - Add a learning if this experiment proves a reusable rule for handling
    full-header ABI drift.

## Verification

Pass/fail criteria:

- The mapped full-header function comparison no longer reports unexplained
  missing Roastty declarations.
- The Roastty header/export comparison reports no real missing exports for the
  functions added in this experiment.
- Any remaining semantic difference is recorded as `Intentional divergence` or
  `Not applicable` in the appropriate matrix, with evidence and a regression
  guard.
- If any of the four functions remains `Gap`, the experiment result must be
  `Partial` or `Fail`, and `SRC-004` must stay `Gap`.
- The C ABI harness compiles against the new declarations.
- Roastty's Rust tests and macOS debug app build still pass.

Commands:

```bash
perl -ne 'while(/\b(ghostty_[A-Za-z0-9_]+)\s*\(/g){print "$1\n"}' \
  vendor/ghostty/include/ghostty.h | sort -u \
  > /tmp/issue805-exp5-ghostty-header-fns.txt

perl -ne 'while(/\b(roastty_[A-Za-z0-9_]+)\s*\(/g){print "$1\n"}' \
  roastty/include/roastty.h | sort -u \
  > /tmp/issue805-exp5-roastty-header-fns.txt

sed 's/^ghostty_/roastty_/' /tmp/issue805-exp5-ghostty-header-fns.txt \
  > /tmp/issue805-exp5-ghostty-header-fns-mapped.txt

comm -23 /tmp/issue805-exp5-ghostty-header-fns-mapped.txt \
  /tmp/issue805-exp5-roastty-header-fns.txt

perl -0ne 'while(/#\[no_mangle\]\s*pub\s+extern\s+"C"\s+fn\s+(roastty_[A-Za-z0-9_]+)/g){print "$1\n"}' \
  roastty/src/lib.rs | sort -u \
  > /tmp/issue805-exp5-roastty-exported-fns.txt

comm -23 /tmp/issue805-exp5-roastty-header-fns.txt \
  /tmp/issue805-exp5-roastty-exported-fns.txt

rg -n 'roastty_(app_open_config|benchmark_cli|inspector_metal_shutdown|translate)' \
  roastty/include/roastty.h roastty/src/lib.rs roastty/tests/abi_harness.c \
  roastty/macos/Tests/BenchmarkTests.swift

cargo fmt --check -p roastty
cargo test -p roastty -- --test-threads=1
cd roastty && nu macos/build.nu --configuration Debug
git diff --check
```

The only acceptable raw false positive in the final header/export comparison is
`roastty_string_s`, which the Experiment 4 audit classified as a callback
typedef return type rather than a function declaration. Any additional missing
name must be fixed or explicitly classified in the matrices.

The verification output should be saved under `logs/` with an `issue805-exp5-`
prefix.

## Design Review

Fresh-context adversarial review result: **Changes required**.

- Required: the benchmark disposition was too loose because
  `roastty_benchmark_cli` is present in Ghostty's pinned public header,
  implemented upstream, and referenced by Roastty's copied macOS benchmark test.
  Fix: the design now allows only equivalent benchmark behavior or an
  `Intentional divergence`, not `Not applicable`, while the copied benchmark
  surface remains.
- Required: the original pass/fail criteria allowed a remaining `Gap` to count
  as a resolved experiment. Fix: the criteria now require `Partial` or `Fail` if
  any of the four functions remains `Gap`, and require `SRC-004` to stay `Gap`
  in that case.
- Optional: the header/export comparison should account for the known
  `roastty_string_s` regex false positive. Fix: the verification section now
  states that `roastty_string_s` is the only acceptable raw false positive.

Fresh-context adversarial re-review result: **Approved**.

- The reviewer confirmed the benchmark disposition now forbids `Not applicable`
  while the copied benchmark test references `roastty_benchmark_cli`.
- The reviewer confirmed the pass/fail criteria now require `Partial` or `Fail`
  if any of the four functions remains `Gap`.
- The reviewer confirmed the verification section now identifies
  `roastty_string_s` as the only acceptable raw header/export false positive.
- The reviewer confirmed the issue README links Experiment 5 as `Designed`.

Final design-review verdict: **Approved**.

## Result

**Result:** Pass

The four missing mapped non-app Ghostty header functions are now resolved:

- `roastty_app_open_config` is declared, exported, and dispatches
  `ROASTTY_ACTION_OPEN_CONFIG` through Roastty's runtime action callback.
- `roastty_inspector_metal_shutdown` is declared, exported, and safely reports
  success for a valid inspector handle while Roastty's inspector Metal backend
  remains unsupported.
- `roastty_translate` is declared and exported as an identity helper. This is
  recorded as `DIV-001` because Roastty has locale helper plumbing but no
  catalog-backed translation runtime.
- `roastty_benchmark_cli` is declared and exported as a false-returning
  unsupported helper. This is recorded as `DIV-002` because Roastty has no
  benchmark CLI port.

`SRC-004` is no longer a gap. The mapped full-header comparison reports no
missing Roastty declarations. The header/export comparison reports only
`roastty_string_s`, the known callback typedef false positive from Experiment 4.

Verification artifacts:

- `logs/issue805-exp5-static-abi.log`
- `logs/issue805-exp5-cargo-fmt-check.log`
- `logs/issue805-exp5-cargo-test-roastty.log`
- `logs/issue805-exp5-roastty-app-build.log`

Verification summary:

- `cargo fmt --check -p roastty` passed.
- `cargo test -p roastty -- --test-threads=1` passed: 4896 Rust tests passed, 4
  ignored, and the C ABI harness passed.
- `cd roastty && nu macos/build.nu --configuration Debug` passed with
  `** BUILD SUCCEEDED **`.

## Conclusion

Experiment 5 closes the full-header declaration/export gap found by Experiment 4
without overclaiming unsupported behavior. The durable guard is the cheap static
header/export comparison plus the existing C ABI harness, which now links and
exercises the newly declared functions.

Future experiments should use this pattern for parity rows that mix ABI shape
and behavior: make the symbol surface complete, prove the behavior that exists,
and record unsupported semantics as explicit divergence rows with guards.

## Completion Review

Fresh-context adversarial completion review result: **Changes required**.

- Required: `abi-app-symbols.md` had a stale Experiment 4 conclusion saying
  `roastty_benchmark_cli` was still undeclared and remained a later gap. Fix:
  the conclusion now says Experiment 5 resolved the missing symbol gap and that
  `roastty_benchmark_cli` is an accepted semantic divergence, not a missing
  declaration/export.

Fresh-context adversarial completion re-review result: **Approved**.

- The reviewer confirmed the stale conclusion contradiction is resolved.
- The reviewer found no new required findings introduced by the fix.

Final completion-review verdict: **Approved**.
