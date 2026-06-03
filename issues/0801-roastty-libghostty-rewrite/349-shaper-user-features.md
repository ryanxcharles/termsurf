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

# Experiment 349: threading user features into shaping

## Description

Experiment 347 applies the hardcoded `default_features` (`liga`) during shaping;
Experiment 348 parses user feature **strings** into `Feature`s. This experiment
connects them: a shaping `Options` (the user's configured feature strings) is
merged with the defaults and applied during shaping — upstream's `Shaper.init`
behavior (`feats_df = default_features ++ parsed`), exposed here as
`Options::merged_features` plus a feature-list-taking shaping entry.

## Upstream behavior (`shaper/coretext.zig` `Shaper.init`)

```zig
// Parse every configured feature string, then prepend the defaults:
for (opts.features) |feature_str| feature_list.appendFromString(alloc, feature_str);
const feats = feature_list.features.items;
const feats_df = try alloc.alloc(Feature, feats.len + default_features.len);
@memcpy(feats_df[0..default_features.len], &default_features);   // defaults first
@memcpy(feats_df[default_features.len..], feats);                 // then user
const features = try makeFeaturesDict(feats_df);                  // applied at shape
```

The merged list is **defaults first, then user features** (so a user `-liga`
appears _after_ the default `liga = 1` and overrides it — later settings win in
CoreText's feature-settings array). Each `opts.features` entry is itself a
comma-separated list (`appendFromString` parses it).

## Rust mapping (`roastty/src/font/shape.rs`, `face/coretext.rs`)

- `roastty/src/font/shape.rs`:
  `Options::merged_features(&self) -> Vec<Feature>`:
  ```rust
  pub(crate) fn merged_features(&self) -> Vec<Feature> {
      let mut out = default_features();                       // defaults first
      for s in &self.features {
          out.extend(parse_features(s));                      // then parsed user
      }
      out
  }
  ```
- `roastty/src/font/face/coretext.rs`: extract the `shape_run` body into
  `shape_run_with_features(&self, run: &[shape::Codepoint], features: &[shape::Feature])`
  (using the given feature list to build the descriptor), and reduce:
  - `shape_run(run)` →
    `self.shape_run_with_features(run, &shape::default_features())` (unchanged
    behavior);
  - add `shape_run_options(&self, run, options: &shape::Options)` →
    `self.shape_run_with_features(run, &options.merged_features())`.
- `roastty/src/font/face/coretext.rs` (`feature_settings_descriptor`): now that
  user-supplied values reach this path, replace the wrapping `f.value as i32`
  with a **checked** conversion — `i32::try_from(f.value)` — and **skip** any
  feature whose value does not fit a signed 32-bit int (CoreText's
  `kCTFontOpenTypeFeatureValue` is a signed int, as upstream's `@intCast` to
  `c_int`; real feature values are small, so an out-of-range value is
  degenerate). A list that becomes empty after filtering yields `None` (no
  descriptor).

## Scope / faithfulness notes

- **Ported**: the merge of `default_features` with the parsed user features
  (defaults first), and its application during shaping — upstream's
  `Shaper.init` feature assembly.
- **Faithful**: the order (defaults then user) matches upstream's `feats_df`
  layout; each `Options.features` string is parsed as a comma-separated list
  (`parse_features`), matching `appendFromString`.
- **Faithful simplification**: upstream builds the feature dict **once** at
  `Shaper.init` and caches it; roastty rebuilds it per shape call (via
  `shape_run_with_features`). This is an efficiency difference only — the
  applied features are identical. The caching `Shaper` struct (and the
  `features_no_default` variant for faces that disable defaults) lands with the
  `Shaper`/`Collection` wiring.
- **Hardening**: `feature_settings_descriptor`'s value conversion is changed
  from the wrapping `as i32` to a checked `i32::try_from` (skipping out-of-range
  values), because user-supplied values (up to `u32::MAX` from the parser) now
  reach it. Upstream's `@intCast` to `c_int` is checked under Zig safety;
  skipping the degenerate out-of-range case is the safe analog (real feature
  values are small).
- **Deferred** (unchanged): the `features_no_default` variant, the special-font
  path, the `Shaper` struct + `RunIterator`.
- No C ABI/header/ABI-inventory change (internal Rust).

## Changes

1. `roastty/src/font/shape.rs`: add `Options::merged_features`.
2. `roastty/src/font/face/coretext.rs`: extract `shape_run_with_features` from
   `shape_run`; `shape_run` delegates with `default_features()`; add
   `shape_run_options`; harden `feature_settings_descriptor` with a checked
   `i32::try_from` that skips out-of-range values.
3. Tests:
   - `merged_features_defaults_then_user` (in `shape.rs`): an `Options` with
     `features = ["-liga", "kern=2"]` yields `[liga = 1, liga = 0, kern = 2]`
     (defaults first, then the parsed user features in order); an `Options` with
     one comma-list string `["calt, -dlig"]` yields
     `[liga = 1, calt = 1, dlig = 0]`; an empty `Options` yields just
     `[liga = 1]`.
   - `shape_run_options_regression` (in `coretext.rs`): `shape_run_options` with
     `Options::default()` shapes Menlo `"ABC"` to the same three cells as
     `shape_codepoints` (the merged-features path matches the default path).
   - `feature_settings_descriptor_skips_out_of_range` (in `coretext.rs`): a
     feature with `value > i32::MAX` is skipped — a single such feature yields
     `None`; `[liga = 1, <out-of-range>]` yields `Some` (only the valid feature
     remains). Confirms the checked conversion does not wrap.
   - The existing `shape_*` / feature tests still pass.
4. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty merged_features
cargo test -p roastty shape
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `Options::merged_features` merges `default_features` with the parsed user
  features (defaults first), and `shape_run_options` applies them, faithful to
  upstream's `Shaper.init`;
- `shape_run` is an unchanged delegate over `shape_run_with_features`;
- the merge and regression tests pass, and the existing shaping/feature tests
  still pass;
- the `features_no_default` variant, the special-font path, and the
  `Shaper`/`RunIterator` stay deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if the merge order or parsing diverges from upstream,
`shape_run`'s behavior changes, or any public C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and found **one Required
finding**, now fixed:

- **Required (fixed):** with user features now flowing into
  `feature_settings_descriptor`, its `f.value as i32` cast would silently wrap a
  large `u32` (the parser accepts up to `u32::MAX`, so `"aalt=4294967295"` would
  become `-1` in the feature dictionary). The design now replaces it with a
  checked `i32::try_from(f.value)` that **skips** out-of-range values
  (CoreText's `kCTFontOpenTypeFeatureValue` is a signed int, as upstream's
  `@intCast` to `c_int`, which is checked under Zig safety; real feature values
  are small, so an out-of-range value is degenerate). A
  `feature_settings_descriptor_skips_out_of_range` test was added.

Codex confirmed the rest: defaults-first-then-user is the correct upstream
order; the override claim holds (CoreText's `kCTFontFeatureSettingsAttribute`
uses the **last** setting for duplicates, so a trailing `-liga` overrides the
default `liga = 1`); parsing each `Options.features` entry with `parse_features`
matches `appendFromString`; the per-shape descriptor rebuild vs upstream's
once-cached build is an efficiency-only difference (identical applied features);
and the `shape_run` delegate refactor is behavior-preserving (it still uses only
`default_features()`, with `shape_run_options` the sole new user-feature path).

Review artifacts:

- Prompt: `logs/codex-review/20260603-143046-872583-prompt.md` (design)
- Result: `logs/codex-review/20260603-143046-872583-last-message.md` (design)
