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

# Experiment 236: Port Font `FaceMetrics` and Its Convenience Methods

## Description

Continue `font/Metrics.zig` by porting `FaceMetrics` — the **raw metrics read
from a font face**, the input to the (deferred) `calc` routine that derives a
`Metrics`. It carries five required and eight optional `f64` measurements plus
three convenience accessors with fallback estimates.

`Minimums`, `calc`, and constraint application remain deferred to later slices.

### Fields to port (all `f64`)

Required: `px_per_em`, `cell_width`, `ascent`, `descent`, `line_gap`.

Optional (`Option<f64>`, upstream default `null`): `underline_position`,
`underline_thickness`, `strikethrough_position`, `strikethrough_thickness`,
`cap_height`, `ex_height`, `ascii_height`, `ic_width`.

Semantics (preserved in doc comments): metrics are relative to the baseline with
`+Y` up; `descent` is typically negative; `cap_height`/`ex_height` are the
capital/lowercase heights when the font provides them.

### Convenience methods (real logic with fallback estimates)

Upstream:

```
pub fn lineHeight(self) f64 { return self.ascent - self.descent + self.line_gap; }
pub fn capHeight(self) f64 {
    if (self.cap_height) |value| if (value > 0) return value;
    return 0.75 * self.ascent;
}
pub fn exHeight(self) f64 {
    if (self.ex_height) |value| if (value > 0) return value;
    return 0.75 * self.capHeight();
}
```

`capHeight`/`exHeight` use the stored value only when it is present **and
`> 0`** (`Some(0.0)` or a negative falls through to the estimate); otherwise
they estimate `0.75 * ascent` and `0.75 * capHeight()` respectively.
`lineHeight` is `ascent - descent + line_gap`.

### Naming decision (field vs. method clarity)

Upstream relies on Zig's snake/camel split: fields `cap_height`/`ex_height` and
methods `capHeight()`/`exHeight()`. In Rust a `cap_height` field and a
`cap_height()` method **can** coexist (fields and methods are in different
namespaces), so this is a clarity choice rather than a requirement: a
`m.cap_height` (raw `Option<f64>`) sitting next to `m.cap_height()` (computed
`f64`) is a footgun. The port keeps the **raw optional fields**
`cap_height`/`ex_height` and names the computed accessors
**`effective_cap_height()`** / **`effective_ex_height()`** (and `line_height()`
for `lineHeight`), making the raw-vs-estimated distinction explicit at every
call site.

### Faithfulness and scope notes

- `FaceMetrics` derives `Debug, Clone, Copy, PartialEq` (not `Eq` — `f64`
  fields).
- The `> 0` guard is reproduced exactly: a present-but-non-positive
  `cap_height`/`ex_height` uses the estimate.
- Added to `roastty/src/font/metrics.rs` (alongside `Metrics`).
- No `Minimums`/`calc`/constraint behavior.
- No C ABI, header, or ABI inventory changes; no new dependencies.

## Changes

1. `roastty/src/font/metrics.rs`:
   - Add `pub(crate) struct FaceMetrics { ... }` (5 `f64` + 8 `Option<f64>`
     fields, `pub`, upstream doc comments), deriving
     `Debug, Clone, Copy, PartialEq`.
   - Add `impl FaceMetrics` with `line_height(&self) -> f64`,
     `effective_cap_height(&self) -> f64`, and
     `effective_ex_height(&self) -> f64`, all `pub(crate)`.

2. Tests in `roastty/src/font/metrics.rs`:
   - `face_metrics_holds_fields`: round-trip required and a couple of optional
     fields.
   - `face_metrics_line_height`: `ascent 10, descent -2, line_gap 1` → `13`.
   - `effective_cap_height_uses_value_when_positive`: `cap_height = Some(9.0)` →
     `9.0`.
   - `effective_cap_height_estimates_when_absent_or_nonpositive`: `None` →
     `0.75 * ascent`; `Some(0.0)` and `Some(-1.0)` also fall back to
     `0.75 * ascent`.
   - `effective_ex_height_uses_value_when_positive`: `ex_height = Some(5.0)` →
     `5.0`.
   - `effective_ex_height_estimates_when_absent_or_nonpositive`: with
     `ex_height = None` and `cap_height = None`, result is
     `0.75 * (0.75 * ascent)`; `ex_height = Some(0.0)` and `Some(-1.0)` also
     fall back to the same estimate (the same `> 0` guard upstream applies to
     `ex_height`).

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

- `FaceMetrics` is ported with the exact required/optional fields and `f64`
  types;
- `line_height`/`effective_cap_height`/`effective_ex_height` reproduce upstream
  exactly, including the `> 0` guard and the `0.75` estimates;
- the listed tests pass (notably the non-positive fallback and the chained
  ex→cap estimate);
- no `Minimums`/`calc`/constraint scope is pulled in;
- no C ABI, header, or ABI inventory changes;
- `cargo fmt` accepted and `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment is **partial** if a method turns out to need a different
signature once `calc` consumes it.

The experiment **fails** if a field type/optionality or a method's formula/guard
diverges from upstream, if `calc`/constraint behavior leaks in, or if any public
C API/ABI changes.

## Design Review

Codex reviewed this design before implementation.

Review artifacts:

- Prompt: `logs/codex-review/20260602-082308-182257-prompt.md`
- Result: `logs/codex-review/20260602-082308-182257-last-message.md`

Codex confirmed the field set (5 required `f64`, 8 `Option<f64>`) and the three
formulas are faithful, and that `PartialEq` without `Eq` is correct.

Two Low findings, fixed in the design above before this commit:

1. the naming rationale was factually wrong — a `cap_height` field and a
   `cap_height()` method **can** coexist in Rust. The `effective_*` names are
   kept as a clarity choice (avoiding a raw-`Option` vs computed-`f64` same-name
   footgun), and the rationale is corrected.
2. the tests covered the `> 0` fallback for `cap_height` but not `ex_height` —
   added `Some(0.0)` and negative `ex_height` fallback cases (upstream applies
   the same guard to `ex_height`).
