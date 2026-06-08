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

# Experiment 846: FrameRenderKnobs::from_config

## Description

Exp 845 began the configuration sub-arc by porting `font-thicken` /
`font-thicken-strength`. This experiment ties config to the renderer: a
`FrameRenderKnobs::from_config(&Config)` constructor that sources the knobs from
a `Config` instead of caller-supplied literals — the bridge from the config
surface to the render input.

Five knobs have a `Config` source today and are sourced now; four have no config
option yet and take ghostty-faithful default constants (named, not hidden):

| knob                       | source                                                                                                                                                                                                                    |
| -------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `bold`                     | `config.bold_color.map(\|c\| c.to_terminal())` (config→terminal `BoldColor`)                                                                                                                                              |
| `background_opacity`       | `config.background_opacity` (f64)                                                                                                                                                                                         |
| `padding_color`            | `config.window_padding_color`                                                                                                                                                                                             |
| `thicken`                  | `config.font_thicken` (Exp 845)                                                                                                                                                                                           |
| `thicken_strength`         | `config.font_thicken_strength` (Exp 845)                                                                                                                                                                                  |
| `alpha`                    | `255` — text foreground is opaque; no config option (faithful)                                                                                                                                                            |
| `overlay_alpha`            | `255` — overlay text opaque; no config option (faithful)                                                                                                                                                                  |
| `faint_opacity`            | `128` = `ceil(0.5 × 255)` (upstream `generic.zig:623` uses `@ceil`) — upstream `faint-opacity` **f64 = 0.5** default; the f64 option is not yet ported (a later config slice), so this is the default as a placeholder u8 |
| `background_opacity_cells` | `false` — upstream `background-opacity-cells` **bool = false** default; not yet ported, so the default constant                                                                                                           |

`config::BoldColor::to_terminal` (`config/mod.rs:1500`) is the existing
config→terminal `BoldColor` conversion (an explicit color resolves via
`to_terminal_rgb`; `Bright` maps through). `alpha`/`overlay_alpha` `= 255` are
the **correct** ghostty values (opaque), not placeholders; only `faint_opacity`
and `background_opacity_cells` are placeholders awaiting their config ports.

## Changes

`roastty/src/renderer/frame_renderer.rs` (production code + tests).

- Add `use crate::config::Config;`.
- Add:

  ```rust
  impl FrameRenderKnobs {
      /// Source the render knobs from a `Config`. Knobs without a config option
      /// yet take ghostty-faithful default constants (see Exp 846).
      pub(crate) fn from_config(config: &Config) -> Self {
          Self {
              bold: config.bold_color.map(|c| c.to_terminal()),
              alpha: 255,
              faint_opacity: 128,
              thicken: config.font_thicken,
              thicken_strength: config.font_thicken_strength,
              background_opacity_cells: false,
              background_opacity: config.background_opacity,
              padding_color: config.window_padding_color,
              overlay_alpha: 255,
          }
      }
  }
  ```

Also update the now-stale `FrameRenderKnobs` doc comment: with `from_config`
landing and Exp 845's options, `thicken`/`thicken_strength` now have config
sources; only `alpha`/`overlay_alpha` (faithful opaque constants) and
`faint_opacity`/`background_opacity_cells` (placeholders awaiting their config
ports) remain unsourced.

No change to `FrameRenderState` or the pipeline; the caller-supplied
`render_knobs()` test helper stays for tests that want arbitrary values.

## Verification

Per the bounded-run convention (15-min cap, Central-stamped, single tracked
task, no poll-watcher). Fast non-Metal unit tests in `frame_renderer.rs`:

- **Defaults flow through:** `FrameRenderKnobs::from_config(&Config::default())`
  yields `bold == None`, `thicken == false`, `thicken_strength == 255`,
  `background_opacity == 1.0`,
  `padding_color == WindowPaddingColor::Background`, and the constants
  `alpha == 255`, `overlay_alpha == 255`, `faint_opacity == 128`,
  `background_opacity_cells == false`.
- **Config values flow through:** a `Config` with `font-thicken` set true,
  `font-thicken-strength` 200, `background-opacity` 0.7, and a `bold-color` set
  → `from_config` reflects each (`thicken == true`, `thicken_strength == 200`,
  `background_opacity == 0.7`, `bold == Some(BoldColor::…)` via `to_terminal`).
- **Drives a frame:**
  `FrameRenderState::from_terminal(&term).rebuild_input( &FrameRenderKnobs::from_config(&Config::default()))`
  feeds `FrameRenderer::update_frame` on a 4×3 terminal and rebuilds the full
  frame — the config-sourced knobs produce a valid input end to end.
- `cargo build -p roastty` — no warnings. `cargo fmt -p roastty -- --check` —
  clean. Full suite via `scripts/bounded-run.sh` (default parallelism) stays
  green. No-ghostty grep on changed lines — clean. `git diff --check` — clean.

**Pass** = the new `from_config` tests pass, a config-sourced input rebuilds a
frame, and the full suite stays green. **Partial/Fail** = any test fails or the
suite regresses.

## Design Review

**Reviewer:** `adversarial-reviewer` subagent (Claude Opus, fresh context,
read-only). **Verdict: APPROVED, no Required findings.** Independently
confirmed: all 9 assignments type-check (config `BoldColor`/`WindowPaddingColor`
are `Copy`, so `config.bold_color.map(|c| c.to_terminal())` copies out of
`&Config` with no `as_ref`/clone); `to_terminal` is `pub(crate)`, same crate;
the constants are ghostty-faithful — **no** text/cell foreground alpha config
exists and upstream `generic.zig:2878` hardcodes non-faint text alpha to `255`,
so `alpha`/`overlay_alpha = 255` are correct (not a missed source), and
`faint_opacity = 128` matches upstream `@intFromFloat(@ceil(0.5 × 255))`
(`generic.zig:623`); `faint-opacity`/`background-opacity-cells` are genuinely
unported (Config.zig:3716/1019, defaults 0.5/false); `Config::default()` gives
the asserted values; the test module sees the new `use crate::config::Config`
via `use super::*`; `#![allow(dead_code)]` covers the test-only API. Two
adopted:

- **Optional — stale struct doc.** The `FrameRenderKnobs` doc said
  `thicken`/`thicken_strength` have no config option and the struct is wholly
  caller-fed; both are now untrue. **Fixed:** the doc-comment update is part of
  the changes.
- **Nit — round vs ceil.** **Fixed:** the `faint_opacity` derivation cites
  `ceil(0.5 × 255)` (upstream `@ceil`), so the eventual f64→u8 port matches.

## Conclusion

_(to be written after the run)_
