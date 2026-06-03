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

# Experiment 347: default font features in shaping

## Description

`shape_run` binds the face's plain `CTFont` to the attributed string, applying
no OpenType feature settings. Upstream's shaper always applies a set of
**default features** (currently `liga` on) — and, in general, user-configured
features — by copying the run's font with a descriptor carrying
`kCTFontFeatureSettingsAttribute`. This experiment ports the **`Feature` type,
the `default_features` list, and the feature-settings application** in
`shape_run` (the always-on defaults). Parsing user feature _strings_ and
threading shaping `Options` through are follow-ups (Experiments 348+).

## Upstream behavior (`shaper/coretext.zig` `makeFeaturesDict` + `getFont`)

```zig
// shape.zig: the always-on defaults.
pub const default_features = [_]Feature{ .{ .tag = "liga".*, .value = 1 } };

// makeFeaturesDict: { kCTFontFeatureSettingsAttribute: [ {tag, value}, … ] }
fn makeFeaturesDict(feats: []const Feature) !*Dictionary {
    const list = MutableArray.create();
    for (feats) |feat| {
        const dict = Dictionary.create(
            &.{ kCTFontOpenTypeFeatureTag, kCTFontOpenTypeFeatureValue },
            &.{ String(feat.tag), Number(feat.value) },
        );
        list.appendValue(dict);
    }
    return Dictionary.create(&.{ kCTFontFeatureSettingsAttribute }, &.{ list });
}

// getFont: apply the features by copying the font with a descriptor.
const desc = FontDescriptor.createWithAttributes(features_dict);
const run_font = original.copyWithAttributes(0, null, desc);   // size 0 ⇒ preserve
// …bind run_font (not the plain font) to the attributed string's font attribute…
```

The feature settings are a font-descriptor attribute: each feature is a
dictionary `{ tag: <4-char string>, value: <number> }`, collected in an array
under `kCTFontFeatureSettingsAttribute`. A `CTFontDescriptor` built from that,
applied via `copyWithAttributes(0, …)` (size `0` preserves the size — the same
idiom as Experiment 345's `set_variations`), yields a font that shapes with
those features.

## Rust mapping (`roastty/src/font/shape.rs`, `face/coretext.rs`)

- `roastty/src/font/shape.rs`: add the feature type and the defaults, mirroring
  upstream:

  ```rust
  /// An OpenType feature setting: a 4-byte tag and a numeric value (`0`/`1` for
  /// boolean features; higher for alternates). Faithful port of upstream
  /// `shaper.Feature`.
  #[derive(Debug, Clone, Copy, PartialEq, Eq)]
  pub(crate) struct Feature {
      pub tag: [u8; 4],
      pub value: u32,
  }

  /// Features hardcoded on by default (users can disable, e.g. `-liga`).
  /// Faithful port of upstream `shape.default_features`.
  pub(crate) fn default_features() -> Vec<Feature> {
      vec![Feature { tag: *b"liga", value: 1 }]
  }
  ```

- `roastty/src/font/face/coretext.rs`:
  - `feature_settings_descriptor(features: &[Feature]) -> Option<CFRetained<CTFontDescriptor>>`:
    `None` for an empty list; otherwise build, per feature, a
    `CFMutableDictionary`
    `{ kCTFontOpenTypeFeatureTag: CFString(tag), kCTFontOpenTypeFeatureValue: CFNumber::new_i32(value) }`;
    collect them into a `CFArray` (`CFArray::from_retained_objects`); wrap that
    under `kCTFontFeatureSettingsAttribute` in a `CFMutableDictionary`; and
    `CTFontDescriptor::with_attributes(&dict)`.
  - In `shape_run`, derive the run's font by applying `default_features()` and
    bind **that** font to the attributed string:
    ```rust
    let features = shape::default_features();
    let run_font = match feature_settings_descriptor(&features) {
        // SAFETY: `self.font`/`desc` live; null matrix; size 0.0 preserves size.
        Some(desc) => unsafe {
            self.font.copy_with_attributes(0.0, std::ptr::null(), Some(&desc))
        },
        None => self.font.clone(),
    };
    // …bind `&*run_font` (instead of `&*self.font`) under kCTFontAttributeName…
    ```

## Scope / faithfulness notes

- **Ported**: the `Feature` type, the `default_features` list, the
  feature-settings descriptor (`makeFeaturesDict`), and its application to the
  shaping font (`getFont`'s `copyWithAttributes`) — so shaping now applies the
  default OpenType features (`liga`).
- **Faithful**: the feature dict shape (`kCTFontOpenTypeFeatureTag` /
  `kCTFontOpenTypeFeatureValue` under `kCTFontFeatureSettingsAttribute`) matches
  upstream's `makeFeaturesDict`; `copy_with_attributes(0.0, …)` preserves the
  size (as in `set_variations`); the tag is the 4-byte ASCII feature tag.
- **Deferred** (follow-ups): the **feature-string parser** (`Feature::from_str`,
  the HarfBuzz-syntax state machine — Experiment 348); threading shaping
  **`Options`** (user features) and the `features_no_default` variant (used for
  faces that disable default features) through `shape_run`; the special-font
  fast path; the `Shaper` struct + `RunIterator`.
- No C ABI/header/ABI-inventory change (internal Rust).

## Changes

1. `roastty/src/font/shape.rs`: add `Feature` and `default_features()`.
2. `roastty/src/font/face/coretext.rs`: add `feature_settings_descriptor`; apply
   `default_features()` to the run font in `shape_run`.
3. Tests:
   - `feature_settings_descriptor_some_none` (in `coretext.rs`): the builder
     returns `None` for `&[]` and `Some` for a non-empty feature list (e.g.
     `[liga = 1]`). Deterministic.
   - `shape_run_with_default_features` (in `coretext.rs`): `shape_codepoints`
     (which now applies `default_features()`) still shapes Menlo `"ABC"` to
     three cells whose glyphs match the cmap lookups — the default-feature
     application does not break plain shaping. Deterministic. (Menlo has no
     `liga`, so there is no observable ligation; the regression confirms the
     descriptor/copy path is sound.)
   - `default_features_is_liga` (in `shape.rs`): `default_features()` is
     `[Feature { tag: *b"liga", value: 1 }]`.
   - The existing `shape_*` tests still pass (they now exercise the feature
     path).
4. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty feature
cargo test -p roastty shape
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `shape_run` applies `default_features()` to the run font via a
  `kCTFontFeatureSettingsAttribute` descriptor and binds that font, faithful to
  upstream's `makeFeaturesDict`/`getFont`;
- the builder some/none, the default-features regression, and the
  `default_features` value tests pass, and the existing shaping tests still
  pass;
- the feature-string parser, the `Options` threading, the special-font path, and
  the `Shaper`/`RunIterator` stay deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment is **partial** if no ligating font is available to observe the
`liga` effect at runtime (the descriptor/application path is still exercised,
and plain shaping is verified unchanged).

The experiment **fails** if the feature dict shape or the font-copy application
diverges from upstream, plain shaping regresses, or any public C API/ABI
changes.

## Design Review

Codex reviewed this design before implementation and found **no Required
findings**. It confirmed: the feature-settings descriptor matches upstream's
`makeFeaturesDict` shape (an array of per-feature dicts using
`kCTFontOpenTypeFeatureTag`/`kCTFontOpenTypeFeatureValue`, wrapped under
`kCTFontFeatureSettingsAttribute`, then applied by building a `CTFontDescriptor`
and `copy_with_attributes(0.0, null, Some(&desc))`); binding the copied
`run_font` under `kCTFontAttributeName` is the right place (binding `self.font`
would leave the descriptor unused); and the CF ownership model is sound
(per-feature strings/numbers retained by their dicts, dicts by the array, the
array by the outer dict; the copied `run_font` need only live through the
attributed-string/line creation, which retains it). It noted the 4-byte tag →
`CFString` conversion must be from exactly those four ASCII bytes (not a C
string, no trimming) — validating via `std::str::from_utf8(&feature.tag)` is
fine. Deferring the parser, user `Options`, the `features_no_default` variant,
the special-font path, and the `Shaper`/`RunIterator` is a clean split. The
tests are reasonable smoke coverage (the descriptor/copy path, not a ligature
effect on a known ligating font).

Review artifacts:

- Prompt: `logs/codex-review/20260603-141207-751340-prompt.md` (design)
- Result: `logs/codex-review/20260603-141207-751340-last-message.md` (design)
