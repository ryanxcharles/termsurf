+++
[implementer]
agent = "claude-code"
model = "claude-opus-4-8"
reasoning = "high"

[review.design]
agent = "claude-code"
model = "claude-opus-4-8"
reasoning = "high"

[review.result]
agent = "claude-code"
model = "claude-opus-4-8"
reasoning = "high"
+++

# Experiment 849: minimum-contrast config + MetalUniforms::from_config

## Description

Exp 846/848 made `FrameRenderKnobs::from_config` fully config-faithful (the
row-format/overlay knobs). The other config-derived half of the renderer is the
**`MetalUniforms`** — `FrameRenderer::new` takes a caller-built `MetalUniforms`,
and `MetalUniforms::new` (`shaders.rs:170`) consumes five config-derived values:
`min_contrast`, `background`, `background_opacity`, `colorspace`, `blending`.

This experiment closes that half: it ports the one missing config option,
`minimum-contrast`, and adds `MetalUniforms::from_config(&Config)` so the caller
builds the uniforms from a `Config` instead of loose literals — mirroring
`FrameRenderKnobs::from_config`.

Type wiring (all verified):

- `min_contrast: f32` ← `config.minimum_contrast` (f64), **clamped to
  `[1, 21]`** at the use site (upstream clamps in `finalize`, `Config.zig:4680`;
  roastty has no finalize, so clamp here — same pattern as Exp 848's
  `faint-opacity`). Upstream default `1.0` (`Config.zig:776`).
- `background: Rgb` ← `config.background.to_terminal_rgb()`
  (`config/mod.rs:1390`).
- `background_opacity: f64` ← `config.background_opacity`.
- `colorspace: WindowColorspace` ← `config.window_colorspace` (the **same** type
  — `shaders.rs` imports `WindowColorspace`/`AlphaBlending` from
  `crate::config`).
- `blending: AlphaBlending` ← `config.alpha_blending`.

## Changes

### config/mod.rs — port minimum-contrast

Mirroring the f64 option `background-opacity` (`set_f64_field` / `entry_float`):

- **Struct field** `pub minimum_contrast: f64` with the upstream-key doc
  comment, placed in upstream-declaration order — upstream `minimum-contrast`
  (776) sits between `selection-background` (708) and `cursor-color` (851), so
  the formatter entry and keys-vec entry go at that slot (verified against the
  exact ordered-keys test; roastty's own cursor/selection ordering is checked
  during implementation and the slot adjusted to match what the test emits).
- **Default** `minimum_contrast: 1.0`.
- **Parse arm**
  `"minimum-contrast" => self.minimum_contrast = set_f64_field(value, default.minimum_contrast)?`
  (stored raw; clamped at use).
- **Formatter entry** `entry_float`, at the matching position.

### shaders.rs — MetalUniforms::from_config

```rust
impl MetalUniforms {
    /// Build the per-frame uniforms from a `Config` (Issue 801, Exp 849).
    /// `min_contrast` is clamped to `[1, 21]` at this use site (roastty has no
    /// config finalize step), matching upstream's finalize clamp.
    pub(crate) fn from_config(config: &Config) -> Self {
        Self::new(
            config.minimum_contrast.clamp(1.0, 21.0) as f32,
            config.background.to_terminal_rgb(),
            config.background_opacity,
            config.window_colorspace,
            config.alpha_blending,
        )
    }
}
```

(`use crate::config::Config;` added to `shaders.rs`.)

## Verification

Per the bounded-run convention (15-min cap, Central-stamped, single tracked
task, no poll-watcher). Fast non-Metal unit tests
(`MetalUniforms::new`/`from_config` need no GPU):

- **config defaults/parse:** a default `Config` has `minimum_contrast == 1.0`;
  `minimum-contrast 5.0` parses to 5.0; the formatter round-trips it; the
  ordered-keys test passes with the new key.
- **`MetalUniforms::from_config` sources the values:** since `MetalUniforms`
  derives `PartialEq`,
  `assert_eq!(MetalUniforms::from_config(&Config::default()), MetalUniforms::new(1.0, default_bg.to_terminal_rgb(), 1.0, WindowColorspace::Srgb, AlphaBlending::Native))`
  — a single exact-equality assertion.
- **clamp at use:** a `Config` with `minimum-contrast 50.0` (stored raw) →
  `from_config().min_contrast == 21.0`; `minimum-contrast 0.0` →
  `min_contrast == 1.0`.
- **config-sourced value flows:** `minimum-contrast 7.0` →
  `from_config().min_contrast == 7.0`.
- `cargo build -p roastty` — no warnings. `cargo fmt -p roastty -- --check` —
  clean. Full suite via `scripts/bounded-run.sh` (default parallelism) stays
  green. No-ghostty grep on changed lines — clean. `git diff --check` — clean.

**Pass** = the new config + `MetalUniforms::from_config` tests pass and the full
suite stays green. **Partial/Fail** = any test fails or the suite regresses.

## Design Review

**Reviewer:** `adversarial-reviewer` subagent (Claude Opus, fresh context,
read-only). **Verdict: APPROVED, no Required findings.** Independently verified
every load-bearing claim: the types line up and `from_config` compiles (`new`'s
signature at `shaders.rs:170`; `config.background.to_terminal_rgb()` is the
non-optional `Rgb` at `config/mod.rs:1390`, not the `Option` at 1466;
colorspace/blending are the same `crate::config` types imported at
`shaders.rs:6`); the clamp+cast is faithful (`clamp(1,21)` = upstream
`@min(21,@max(1,..))`, default 1.0→1.0); `new` is device-free (no metal-device
guard needed); **`MetalUniforms` derives `PartialEq` (`shaders.rs:124`)**, so
the test is a single
`assert_eq!(from_config(default), new(1.0, default_bg, 1.0, Srgb, Native))`
(defaults `Srgb`/`Native`/`1.0` confirmed); raw-store+clamp-at-use is the
established `background-opacity`/faint pattern; `minimum_contrast` is genuinely
absent; the exact-equality ordered-keys test fully constrains placement. Two
minors, both adopted:

- **Optional — state the slot.** **Fixed:** the design names the upstream
  neighbors (`selection-background` 708 < `minimum-contrast` 776 <
  `cursor-color` 851), with the final slot confirmed against the keys test
  during implementation.
- **Nit — use `assert_eq!`.** **Fixed:** the verification uses a single
  exact-equality assertion (PartialEq).

## Result

**Result:** Pass

`minimum-contrast` (f64, default 1.0) is ported into `Config`
(struct/default/parse/ format, placed between `selection-background` and
`cursor-color` per upstream order), and `MetalUniforms::from_config(&Config)`
was added in `shaders.rs` (sourcing min_contrast clamped [1,21], background via
`to_terminal_rgb`, opacity, colorspace, blending). Production
`cargo build -p roastty` and `--tests` both clean (no warnings); fmt clean,
no-ghostty clean, `git diff --check` clean.

Tests (all passing):

- **`config_default_clipboard_group`** gained `minimum_contrast == 1.0`.
- **`config_opacity_options_parse_and_round_trip`** now also parses
  `minimum-contrast 5.0` and round-trips it.
- **`config_format_config_emits_fields_in_upstream_order`** — the ordered-keys
  test passes with `minimum-contrast` at its upstream slot.
- **`uniforms_from_config_sources_config_values`** (new) —
  `assert_eq!(from_config( default), new(1.0, default_bg, 1.0, Srgb, Native))`;
  clamp at use (50.0→21, 0.0→1); value flows (7.0→7).

**Full suite (default parallelism, `scripts/bounded-run.sh`):**
`4394 passed; 0 failed` (4393 + 1 new), 0 panics, 0 `PoisonError`,
`STATUS=COMPLETED rc=0`, 494 s (under the 900 s cap) — green.

## Conclusion

The **config→renderer bridge is complete**: both halves of the renderer's
config-derived inputs now come from a `Config` — `FrameRenderKnobs::from_config`
(row-format/overlay knobs, 846/848) and `MetalUniforms::from_config` (the
uniforms, 849). Combined with `FrameRenderState::from_terminal` and
`FrameRenderer::render_frame`, the entire renderer is driven from
`(terminal, config)` with no loose literals — the config sub-arc for the
renderer is done.

Remaining 801 library surface (all in-scope, no app):

- Other libghostty subsystems on the checklist: **font/text** (shaping/atlas
  foundations → more), **input encoding**, **supporting subsystems**, and
  **dependencies** (URI/regex, remaining `os/`).
- More `Config` options toward full parity (the surface has many upstream
  options not yet ported), as they're needed.
- The renderer's live `surface.draw()` wiring remains deferred until the app
  provides an NSView (out of current scope).

## Completion Review

**Reviewer:** `adversarial-reviewer` subagent (Claude Opus, fresh context,
read-only). Independently confirmed: scope clean (only `config/mod.rs`,
`shaders.rs`, the experiment doc; plan/result commits separate); `config::` 154
passed + `renderer::metal::shaders::tests` 24 passed (incl. the new
`uniforms_from_config_sources_config_values`); `from_config` correct
(`to_terminal_rgb` non-Option `Rgb`; defaults Srgb/Native/1.0/1.0 confirmed;
clamp asserts real 50→21/0→1/7→7); upstream fidelity (`Config.zig:776` default 1
/ `:4680` clamp [1,21]); placement between `selection-background` and
`cursor-color` enforced by the exact-equality keys-test; fmt clean, no `ghostty`
literal; v1.log 4394 passed / 0 failed, rc=0, 494 s < cap. The "no warnings" was
re-confirmed by a **forced rebuild** (touch + rebuild) after the review's
warm-cache caveat. **Verdict: CHANGES REQUIRED → fixed.** The lone Required was
the stale README index status — flipped 849 `Designed → Pass`.
