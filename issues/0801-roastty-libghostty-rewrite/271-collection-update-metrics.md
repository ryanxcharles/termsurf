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

# Experiment 271: Collection updateMetrics — grid metrics from the primary face

## Description

A `Collection` computes the terminal's **grid metrics** (cell size, baseline,
underline, etc.) from its primary face. This experiment ports `updateMetrics`
(`font/Collection.zig` lines 669–683): load the primary face (index 0), compute
its `FaceMetrics`, derive the grid `Metrics` via the already-ported
`Metrics::calc`, and store both. The `metric_modifiers` (`Metrics.ModifierSet` +
`Metrics.apply`) are a config feature that defaults to identity; they are
deferred.

## Upstream behavior (`font/Collection.zig`)

```zig
pub fn updateMetrics(self: *Collection) UpdateMetricsError!void {
    const primary_face = self.getFace(.{ .idx = 0 }) catch return error.CannotLoadPrimaryFont;
    self.primary_face_metrics = primary_face.getMetrics();
    var metrics = Metrics.calc(self.primary_face_metrics.?);
    metrics.apply(self.metric_modifiers);
    self.metrics = metrics;
}
```

- Requires a primary font at index 0; errors `CannotLoadPrimaryFont` otherwise.
- Caches the primary's `FaceMetrics` (the same cache the scale factor uses),
  derives the grid `Metrics`, applies the modifiers, and stores `self.metrics`.

## Rust mapping (`roastty/src/font/collection.rs`)

- `enum UpdateMetricsError { CannotLoadPrimaryFont }`.
- `Collection.metrics: Option<Metrics>` field (init `None`; `Metrics` from
  `crate::font::metrics`).
- `update_metrics(&mut self) -> Result<(), UpdateMetricsError>`:
  `let face = self.get_face(Index::default()).map_err(|_| CannotLoadPrimaryFont)?;`
  then `let fm = face.get_metrics();` (owned, ending the `&self` borrow);
  `self.primary_face_metrics = Some(fm);`
  `self.metrics = Some(Metrics::calc(fm));` (the `metric_modifiers` apply is
  deferred — the default modifier set is identity). `Ok(())`.
- `metrics(&self) -> Option<&Metrics>` accessor.

## Scope / faithfulness notes

- **Deferred**: `metric_modifiers` (`Metrics.ModifierSet` + `Metrics.apply`) — a
  config-driven per-metric adjustment that defaults to identity, so omitting it
  yields the same `metrics` for an unmodified collection. The modifiers land
  with the config subsystem.
- `update_metrics` refreshes `primary_face_metrics` (the scale-factor cache)
  from the primary, matching upstream (which assigns it unconditionally here).
- No C ABI/header/ABI-inventory change.

## Changes

1. `roastty/src/font/collection.rs`: add `UpdateMetricsError`, the
   `metrics: Option<Metrics>` field (init `None` in `new`), `update_metrics`,
   and the `metrics()` accessor; import `crate::font::metrics::Metrics`.
2. Tests (live CoreText, macOS):
   - `update_metrics_from_primary`: a collection with Menlo `Regular`;
     `update_metrics()` is `Ok`, `metrics()` is `Some` with `cell_width > 0`,
     `cell_height > 0`, and `cell_baseline <= cell_height`; and it equals
     `Metrics::calc(get_face(Index::default()).get_metrics())`.
   - `update_metrics_no_primary`: an empty collection → `update_metrics()` is
     `Err(CannotLoadPrimaryFont)` and `metrics()` stays `None`.
   - `update_metrics_caches_primary`: after `update_metrics`, the in-module test
     directly asserts `c.primary_face_metrics.is_some()` (the cache is
     populated).
3. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty collection
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `update_metrics` derives the grid `Metrics` from the primary face via
  `Metrics::calc`, caches the primary `FaceMetrics`, and errors
  `CannotLoadPrimaryFont` when there's no primary;
- `metrics()` exposes the computed grid metrics;
- the `metric_modifiers` apply is cleanly deferred (identity default);
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment is **partial** if the borrow shape (load primary → cache → store)
needs restructuring.

The experiment **fails** if the metrics derivation diverges from upstream
(beyond the documented deferred modifiers) or any public C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and raised one **Low** finding:
the `update_metrics_caches_primary` test (a later `add_with_adjustment`
succeeds) wouldn't actually prove the cache was populated, since
`compute_scale_factor` loads the primary lazily anyway. The design was updated
to **directly assert** `c.primary_face_metrics.is_some()` after `update_metrics`
(the test is in-module and can read the private field). No other findings: the
design is faithful to upstream `updateMetrics` aside from the documented
modifier deferral, the empty `ModifierSet` identity claim is reasonable, the
`get_face` → owned `get_metrics` → assign borrow shape is sound,
`Index::default()` is the primary `{regular, 0}`, and the unconditional cache
refresh matches upstream.

Review artifacts:

- Prompt: `logs/codex-review/20260602-225056-067207-prompt.md`
- Result: `logs/codex-review/20260602-225056-067207-last-message.md`
