# Experiment 171: Phase F — FreeType load flags config

## Description

Remove `freetype-load-flags` from the remaining Phase F public-config tail.

Upstream defines `freetype-load-flags` immediately after `grapheme-width-method`
and before `theme` as a packed boolean struct with five flags:

- `hinting`, default `true`
- `force-autohint`, default `false`
- `monochrome`, default `false`
- `autohint`, default `true`
- `light`, default `true`

The CLI/config syntax is upstream's packed-struct behavior: a standalone boolean
sets every flag, and comma-separated `[no-]flag` tokens override individual
flags while omitted flags keep their defaults.

This experiment wires config parser/formatter/storage behavior only. Applying
these flags during live FreeType glyph loading remains font-backend runtime
work, and macOS continues to use CoreText.

## Changes

- `roastty/src/config/mod.rs`
  - Add a `FreetypeLoadFlags` packed-flag config type with upstream defaults and
    keywords.
  - Add `Config::freetype_load_flags`, defaulting to upstream's `.{}` defaults.
  - Format `freetype-load-flags` after `grapheme-width-method` and before
    `theme`.
  - Route `Config::set("freetype-load-flags", ...)` through the existing
    `set_packed_field` helper:
    - missing values report `ValueRequired`;
    - empty set values reset to the default packed flags;
    - `true` and `false` set all five flags;
    - comma-separated `[no-]hinting`, `[no-]force-autohint`, `[no-]monochrome`,
      `[no-]autohint`, and `[no-]light` tokens update only named flags;
    - unknown tokens report `InvalidValue`.
  - Update default/order tests.
  - Add a focused `freetype_load_flags_config_*` test covering defaults,
    formatting, bool-all parsing, partial token parsing, whitespace around comma
    tokens, reset, load diagnostics, CLI set, clone, and invalid values.

- `issues/0802-libroastty-completion-and-mac-app/README.md`
  - Link Experiment 171 as `Designed`.
  - After result, update the Phase F remaining-public-options count from 3 to 2
    and leave only `input` and `keybind` in that public-config tail if this
    passes.
  - After result, add an operating note that `freetype-load-flags` is
    parser/formatter-only until a FreeType font backend consumes it.

## Verification

Before implementation:

- Codex-native adversarial design review approves this experiment.
- Commit the reviewed plan separately from the result.

After implementation:

- `cargo test -p roastty freetype_load_flags_config`
- `cargo test -p roastty config_format_config_emits_fields_in_upstream_order`
- `cargo test -p roastty`
- `cargo fmt -p roastty`
- `cargo fmt --check -p roastty`
- `prettier --check --prose-wrap always --print-width 80 issues/0802-libroastty-completion-and-mac-app/171-freetype-load-flags-config.md issues/0802-libroastty-completion-and-mac-app/README.md`
- `git diff --check`

**Pass** = `freetype-load-flags` parses, formats, resets, loads, clones, reports
diagnostics, and appears in upstream order with packed-struct defaults, and the
full roastty suite passes.

**Partial** = the direct parser/formatter field lands, but ordering, reset
behavior, diagnostics, or full-suite verification remains incomplete.

**Fail** = the field cannot be added without conflicting with existing config
formatting, packed-flag helpers, or font config storage.

## Design Review

**Reviewer:** Codex-native adversarial review subagent `Averroes`, fresh
context.

**Verdict:** Approved after one required verification fix.

The first review found that the verification plan listed
`cargo fmt --check -p roastty` but not the required actual
`cargo fmt -p roastty` run for planned Rust edits. I added
`cargo fmt -p roastty` before the check-only formatter command. Re-review
confirmed the finding was resolved and found no new required issues.

## Result

**Result:** Pass

Roastty now stores, parses, and formats upstream `freetype-load-flags` as a
packed boolean config struct. The field defaults to upstream's `.{}` equivalent:

- `hinting = true`
- `force-autohint = false`
- `monochrome = false`
- `autohint = true`
- `light = true`

The formatted config line appears after `grapheme-width-method` and before
`theme` as:

```text
freetype-load-flags = hinting,no-force-autohint,no-monochrome,autohint,light
```

The parser accepts standalone `true` and `false` all-flag values,
comma-separated `[no-]flag` tokens with upstream keywords, whitespace around
comma tokens, empty-value reset to defaults, config-file loading, CLI setting,
and clone/equality storage. Missing values report `ValueRequired`; unknown or
snake_case tokens report `InvalidValue`.

This is parser/formatter/storage only. Applying the configured flags to live
glyph loading remains FreeType backend work, and the current macOS path still
uses CoreText.

The completion review found an unrelated race in
`surface_foreground_pid_reports_worker_foreground_pid_after_start`: the test
asserted the PTY foreground process group immediately after spawning the worker,
but `tcgetpgrp` can briefly report the parent/test process group before the
child owns the foreground group. The result fixes that test oracle by waiting
until `roastty_surface_foreground_pid` reports the spawned child id before the
final assertion.

The Phase F public-config tail is now two keys: `input` and `keybind`.

Verification:

- `cargo test -p roastty freetype_load_flags_config` passed 1 filtered unit test
  plus the ABI harness filter.
- `cargo test -p roastty config_format_config_emits_fields_in_upstream_order`
  passed 1 filtered unit test plus the ABI harness filter.
- `cargo test -p roastty surface_foreground_pid_reports_worker_foreground_pid_after_start`
  passed 1 filtered unit test plus the ABI harness filter after hardening the
  test oracle.
- `cargo fmt -p roastty` passed.
- `cargo fmt --check -p roastty` passed.
- `cargo test -p roastty` passed 4,867 Rust unit tests, 0 failed, 4 ignored; the
  C ABI harness passed with the existing enum-conversion warnings; doc tests
  passed with 0 tests.
- `prettier --check --prose-wrap always --print-width 80 issues/0802-libroastty-completion-and-mac-app/171-freetype-load-flags-config.md issues/0802-libroastty-completion-and-mac-app/README.md`
  passed.
- `git diff --check` passed.

## Conclusion

`freetype-load-flags` is no longer a public config field gap. The remaining
Phase F public config tail is `input` and `keybind`; these are larger
parser/runtime surfaces and should be handled one at a time.

## Completion Review

**Reviewer:** Codex-native adversarial review subagent `Curie`, fresh context.

**Verdict:** Approved after one required test-oracle fix.

The first completion review found no required issues in the
`freetype-load-flags` implementation, but it could not reproduce the full-suite
gate because `surface_foreground_pid_reports_worker_foreground_pid_after_start`
hit a PTY foreground-process-group race. I fixed the test to wait until
`roastty_surface_foreground_pid` reports the spawned child id before asserting.
Re-review approved the result: the focused foreground PID test passed, the full
`cargo test -p roastty` suite passed with 4,867 unit tests, the ABI harness
passed with the existing enum-conversion warnings, doc tests had 0 tests, and
the reviewer also reran `cargo fmt --check -p roastty`, prettier check, and
`git diff --check` successfully.
