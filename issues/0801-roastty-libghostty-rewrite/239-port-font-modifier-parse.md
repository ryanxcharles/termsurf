+++
[implementer]
agent = "claude-code"
model = "claude-opus-4-8"
reasoning = "high"

[review.design]
agent = "codex"
model = "gpt-5.5"
reasoning = "medium"

[review.result]
agent = "codex"
model = "gpt-5.5"
reasoning = "medium"
+++

# Experiment 239: Port Font `Modifier` and `Modifier::parse`

## Description

Begin the metric-modifier machinery from `font/Metrics.zig` by porting the
`Modifier` value type and its `parse` string parser. A `Modifier` is a
per-metric adjustment (a percent or an absolute delta) that the config applies
to the derived `Metrics`. This slice ports the type and parsing; `apply` (the
numeric application), `parseCLI`, `formatEntry`, the `ModifierSet` map, and
`Metrics::apply` are later slices.

### Type and `parse` (upstream lines 450–479)

```
pub const Modifier = union(enum) {
    percent: f64,
    absolute: i32,

    pub fn parse(input: []const u8) !Modifier {
        if (input.len == 0) return error.InvalidFormat;
        if (input[input.len - 1] == '%') {
            var percent = std.fmt.parseFloat(f64, input[0 .. input.len - 1])
                catch return error.InvalidFormat;
            percent /= 100;
            if (percent <= -1) return .{ .percent = 0 };
            if (percent < 0) return .{ .percent = 1 + percent };
            return .{ .percent = 1 + percent };
        }
        return .{ .absolute = std.fmt.parseInt(i32, input, 10)
            catch return error.InvalidFormat };
    }
};
```

Key semantics:

- A modifier is a **delta**, not a target: `"20%"` means 20% larger. Percent is
  stored as a **multiplier** `1 + fraction` (so `"20%"` → `1.2`, `"-20%"` →
  `0.8`, `"0%"` → `1.0`).
- A percent `<= -1` (i.e. `"-100%"` or more negative) clamps the multiplier to
  `0`. The two upstream `1 + percent` branches are identical, so the logic is:
  parse the float before `%`, divide by 100; if `<= -1` → `Percent(0.0)`,
  otherwise → `Percent(1.0 + fraction)`.
- A value without a trailing `%` parses as an `i32` `Absolute`.
- An empty string, an unparseable float-before-`%`, or an unparseable integer is
  an error.

### Rust mapping

- `pub(crate) enum Modifier { Percent(f64), Absolute(i32) }`
  (`Debug, Clone, Copy, PartialEq`).
- `parse(input: &str) -> Result<Modifier, ModifierParseError>`:
  - empty → `Err`;
  - `input.strip_suffix('%')` present → parse the prefix as `f64` (`map_err` →
    `Err`), `/100.0`; `<= -1.0` → `Percent(0.0)` else `Percent(1.0 + fraction)`;
  - otherwise parse the whole input as `i32` (`map_err` → `Err`) → `Absolute`.
- `ModifierParseError` is a small unit error type (the upstream
  `error.InvalidFormat`), `Debug, Clone, Copy, PartialEq, Eq`.

### Faithfulness and scope notes

- Rust `f64::from_str` / `i32::from_str` stand in for Zig
  `parseFloat`/`parseInt` for the numeric config strings the parser sees;
  behavior agrees for ordinary decimal/integer inputs.
- Added to `roastty/src/font/metrics.rs` (mirroring upstream's placement in
  `Metrics.zig`).
- No `apply`/`parseCLI`/`formatEntry`/`ModifierSet`/`Metrics::apply` behavior.
- No C ABI, header, or ABI inventory changes; no new dependencies.

## Changes

1. `roastty/src/font/metrics.rs`: add
   `pub(crate) enum ModifierParseError { InvalidFormat }`,
   `pub(crate) enum Modifier { Percent(f64), Absolute(i32) }`, and
   `impl Modifier { pub(crate) fn parse(input: &str) -> Result<Modifier, ModifierParseError> }`.

2. Tests in `roastty/src/font/metrics.rs` (the `approx` helper exists; match the
   `Percent` variant and approx-compare its multiplier):
   - `modifier_parse_percent`: `"20%"` → `Percent(1.2)`; `"-20%"` →
     `Percent(0.8)`; `"0%"` → `Percent(1.0)`.
   - `modifier_parse_percent_clamps`: `"-100%"` → `Percent(0.0)` (exactly `-1`);
     `"-150%"` → `Percent(0.0)`.
   - `modifier_parse_absolute`: `"5"` → `Absolute(5)`; `"-3"` → `Absolute(-3)`;
     `"+5"` → `Absolute(5)` (a leading `+` parses for both Zig and Rust).
   - `modifier_parse_errors`: `""`, `"abc"`, `"abc%"`, and `"%"` (empty
     float-before-`%`) are `Err`.

3. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo test -p roastty font
cargo test -p roastty
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `Modifier`/`parse` reproduce upstream exactly, including the `1 + fraction`
  multiplier, the `<= -1` clamp to `0`, the absolute fallback, and the error
  cases;
- the parse tests pass (percent, clamp, absolute, errors);
- no `apply`/`parseCLI`/`formatEntry`/`ModifierSet` scope is pulled in;
- no C ABI, header, or ABI inventory changes;
- `cargo fmt` accepted and `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment is **partial** if `parse` needs a behavior (e.g. a specific
numeric-format edge) that should be reconciled with the Zig parser separately.

The experiment **fails** if the percent multiplier/clamp or the absolute parse
diverges from upstream, if `apply`/config scope leaks in, or if any public C
API/ABI changes.

## Design Review

Codex reviewed this design before implementation and found **no required
changes**.

Review artifacts:

- Prompt: `logs/codex-review/20260602-084238-013744-prompt.md`
- Result: `logs/codex-review/20260602-084238-013744-last-message.md`

Codex confirmed `parse` is faithful (empty → error, trailing `%` →
float-of-prefix ÷ 100, `<= -1.0` → `Percent(0.0)` else `1.0 + fraction`,
otherwise `i32` absolute), that collapsing the two identical upstream
`1 + percent` branches is correct, that `f64`/`i32` `from_str` are acceptable
analogs, and that the percent expectations (`20% → 1.2`, `-20% → 0.8`,
`-100%`/`-150% → 0.0`) are right. It suggested two optional extra cases — `"+5"`
(accepted by both parsers) and `"%"` (empty prefix) — which were added to the
test plan above.

## Result

**Result:** Pass

Added `pub(crate) enum ModifierParseError { InvalidFormat }`,
`pub(crate) enum Modifier { Percent(f64), Absolute(i32) }`, and
`Modifier::parse` to `roastty/src/font/metrics.rs`. `parse` errors on empty
input; for a trailing `%` it parses the prefix as `f64`, divides by 100, and
returns `Percent(0.0)` when `<= -1.0` else `Percent(1.0 + fraction)`; otherwise
it parses the whole string as `i32` into `Absolute`. Parse failures map to
`InvalidFormat`.

Tests added (4): `modifier_parse_percent` (`20% → 1.2`, `-20% → 0.8`,
`0% → 1.0`), `modifier_parse_percent_clamps` (`-100%`/`-150% → 0.0`),
`modifier_parse_absolute` (`5`, `-3`, `+5`), and `modifier_parse_errors` (`""`,
`"abc"`, `"abc%"`, `"%"`). Percent comparisons use the `1e-9` epsilon helper.

### Verification

```bash
cargo fmt -p roastty
cargo test -p roastty font
cargo test -p roastty
```

Observed:

- `font`: 27 passed (23 prior + 4 new).
- Full `roastty`: 2303 unit tests passed (2299 prior + 4 new), plus the C ABI
  harness passed.
- `cargo fmt -p roastty -- --check`: clean.
- `cargo build -p roastty`: no warnings.
- No-`ghostty`-name gates passed for `roastty/src/font` and for
  `roastty/src/lib.rs`, `roastty/include/roastty.h`,
  `roastty/tests/abi_harness.c`.
- `git diff --check`: clean.

No C ABI, header, or ABI inventory changes; no `apply`/`parseCLI`/`formatEntry`/
`ModifierSet` scope pulled in.

### Completion Review

Codex reviewed the completed implementation and found **no issues** ("nothing
needs to change before the result commit").

Review artifacts:

- Prompt: `logs/codex-review/20260602-084523-003390-prompt.md`
- Result: `logs/codex-review/20260602-084523-003390-last-message.md`

Codex confirmed `parse` is faithful to upstream (empty → `InvalidFormat`,
trailing `%` → `f64` prefix ÷ 100 with the `<= -1.0` clamp to `Percent(0.0)`
else `Percent(1.0 + fraction)`, otherwise `i32` `Absolute`), that the tests
cover the multipliers/clamp/signed-absolute/error paths (including `"%"` failing
via empty-prefix float parse), and that the derives are right (`Modifier`
`PartialEq` only, `ModifierParseError` `Eq`).

## Conclusion

Experiment 239 succeeds. `Modifier` and its `parse` are in place — the config
metric-adjustment type. Both Codex gates passed with zero findings.

The remaining modifier machinery: `Modifier::apply` (the numeric application —
percent rounds `v * multiplier`, absolute saturating-adds, with the int/float
type handling; Experiment 240), then `parseCLI`/`formatEntry` (config
integration), the `ModifierSet` map, and `Metrics::apply` (which carries the
upstream `Metrics.zig` modifier tests). With `Modifier::apply` ported next,
`Metrics::apply` becomes portable, completing the `Metrics.zig` behavior.
