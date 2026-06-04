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

# Experiment 512: the aggregate per-field config formatter (Config::format_config)

## Description

With every leaf `format_entry` and the generic field-dispatch helpers in place
(Experiments 491–511, including `Theme::format_entry` from Experiment 511), this
experiment ports the **top-level config dump** — upstream `FileFormatter.format`
(`config/formatter_file.zig`) — as
`Config::format_config(&self, out: &mut String)`. It walks the `Config` struct's
fields **in upstream declaration order** and emits each as a `key = value\n`
line via that field's `format_entry` (or the matching `entry_*` primitive for
raw `bool` / `Option<T>` fields).

## Upstream behavior

`FileFormatter.format` (`formatter_file.zig:26`), in its default (non-docs,
non-changed) path:

```zig
inline for (@typeInfo(Config).@"struct".fields) |field| {
    if (field.name[0] == '_') continue;
    const value = @field(self.config, field.name);
    // (docs/changed paths omitted)
    formatter.formatEntry(field.type, field.name, value, writer);
}
```

So it iterates fields in **declaration order**, skips `_`-private fields, and
calls the generic `formatEntry` (the field-dispatch we've bridged across
Experiments 491–510) with the field name as the config key. The Zig field name
is the config key (e.g. `@"copy-on-select"` → `copy-on-select`; the unquoted
`theme` / `background` / `foreground` / `fullscreen` are their own keys).

## Upstream declaration order (the 44 roastty `Config` keys)

Extracted from `config/Config.zig` (line → key), the order roastty must emit:

```
186 font-style                 698  background-image-repeat   2433 right-click-action
187 font-style-bold            707  selection-foreground      2443 middle-click-action
188 font-style-italic          708  selection-background      2499 confirm-close-surface
189 font-style-bold-italic     851  cursor-color              2813 shell-integration
374 font-shaping-break         902  cursor-text               2858 shell-integration-features
400 alpha-blending             938  scroll-to-bottom          2920 osc-color-report-format
507 grapheme-width-method      965  mouse-shift-capture       3067 custom-shader-animation
593 theme                      1061 background-blur           3198 macos-non-native-fullscreen
597 background                 1218 notify-on-command-finish  3219 macos-window-buttons
601 foreground                 1232 notify-on-command-finish-action 3261 macos-titlebar-style
639 background-image-opacity   1436 link-previews             3282 macos-titlebar-proxy-icon
657 background-image-position  1469 fullscreen                3358 macos-hidden
687 background-image-fit       1999 window-padding-color      3709 bold-color
                               2110 window-subtitle
                               2142 window-colorspace
                               2361 clipboard-read
                               2362 clipboard-write
                               2416 copy-on-select
```

## The float-blocked gap: background-image-opacity

`background-image-opacity` (`Config.zig:639`) is an `f32`. Upstream emits it via
the generic **float `{d}`** branch, which is **float-formatting-blocked**
(Experiment 509 — Rust `Display` is not a faithful substitute for Zig `{d}`,
which needs the ~1700-line Ryū formatter). This one field is therefore
**deferred**: it is omitted from the dump (a documented, tracked gap), and
re-inserted at its declared position (after `foreground`, before
`background-image-position`) once a faithful float formatter lands. All other 43
fields are emitted in order.

## Rust mapping (`roastty/src/config/mod.rs`)

```rust
impl Config {
    /// Format the whole config as `key = value\n` lines, one per field, in
    /// upstream `Config` declaration order (upstream `FileFormatter.format`,
    /// `config/formatter_file.zig`, the default non-docs / non-changed path).
    ///
    /// `background-image-opacity` (an `f32`) is omitted: the generic float `{d}`
    /// branch is float-formatting blocked (Experiment 509).
    pub(crate) fn format_config(&self, out: &mut String) {
        self.font_style.format_entry(&mut EntryFormatter::new("font-style", out));
        self.font_style_bold.format_entry(&mut EntryFormatter::new("font-style-bold", out));
        self.font_style_italic.format_entry(&mut EntryFormatter::new("font-style-italic", out));
        self.font_style_bold_italic
            .format_entry(&mut EntryFormatter::new("font-style-bold-italic", out));
        self.font_shaping_break
            .format_entry(&mut EntryFormatter::new("font-shaping-break", out));
        self.alpha_blending.format_entry(&mut EntryFormatter::new("alpha-blending", out));
        self.grapheme_width_method
            .format_entry(&mut EntryFormatter::new("grapheme-width-method", out));
        EntryFormatter::new("theme", out)
            .entry_optional(self.theme.clone(), |v, f| v.format_entry(f));
        self.background.format_entry(&mut EntryFormatter::new("background", out));
        self.foreground.format_entry(&mut EntryFormatter::new("foreground", out));
        // background-image-opacity (f32) — float-formatting blocked (Exp 509), deferred.
        self.bg_image_position
            .format_entry(&mut EntryFormatter::new("background-image-position", out));
        self.bg_image_fit.format_entry(&mut EntryFormatter::new("background-image-fit", out));
        EntryFormatter::new("background-image-repeat", out).entry_bool(self.bg_image_repeat);
        EntryFormatter::new("selection-foreground", out)
            .entry_optional(self.selection_foreground, |v, f| v.format_entry(f));
        EntryFormatter::new("selection-background", out)
            .entry_optional(self.selection_background, |v, f| v.format_entry(f));
        EntryFormatter::new("cursor-color", out)
            .entry_optional(self.cursor_color, |v, f| v.format_entry(f));
        EntryFormatter::new("cursor-text", out)
            .entry_optional(self.cursor_text, |v, f| v.format_entry(f));
        self.scroll_to_bottom.format_entry(&mut EntryFormatter::new("scroll-to-bottom", out));
        self.mouse_shift_capture
            .format_entry(&mut EntryFormatter::new("mouse-shift-capture", out));
        self.background_blur.format_entry(&mut EntryFormatter::new("background-blur", out));
        self.notify_on_command_finish
            .format_entry(&mut EntryFormatter::new("notify-on-command-finish", out));
        self.notify_on_command_finish_action
            .format_entry(&mut EntryFormatter::new("notify-on-command-finish-action", out));
        self.link_previews.format_entry(&mut EntryFormatter::new("link-previews", out));
        self.fullscreen.format_entry(&mut EntryFormatter::new("fullscreen", out));
        self.window_padding_color
            .format_entry(&mut EntryFormatter::new("window-padding-color", out));
        self.window_subtitle.format_entry(&mut EntryFormatter::new("window-subtitle", out));
        self.window_colorspace.format_entry(&mut EntryFormatter::new("window-colorspace", out));
        self.clipboard_read.format_entry(&mut EntryFormatter::new("clipboard-read", out));
        self.clipboard_write.format_entry(&mut EntryFormatter::new("clipboard-write", out));
        self.copy_on_select.format_entry(&mut EntryFormatter::new("copy-on-select", out));
        self.right_click_action
            .format_entry(&mut EntryFormatter::new("right-click-action", out));
        self.middle_click_action
            .format_entry(&mut EntryFormatter::new("middle-click-action", out));
        self.confirm_close_surface
            .format_entry(&mut EntryFormatter::new("confirm-close-surface", out));
        self.shell_integration
            .format_entry(&mut EntryFormatter::new("shell-integration", out));
        self.shell_integration_features
            .format_entry(&mut EntryFormatter::new("shell-integration-features", out));
        self.osc_color_report_format
            .format_entry(&mut EntryFormatter::new("osc-color-report-format", out));
        self.custom_shader_animation
            .format_entry(&mut EntryFormatter::new("custom-shader-animation", out));
        self.macos_non_native_fullscreen
            .format_entry(&mut EntryFormatter::new("macos-non-native-fullscreen", out));
        self.macos_window_buttons
            .format_entry(&mut EntryFormatter::new("macos-window-buttons", out));
        self.macos_titlebar_style
            .format_entry(&mut EntryFormatter::new("macos-titlebar-style", out));
        self.macos_titlebar_proxy_icon
            .format_entry(&mut EntryFormatter::new("macos-titlebar-proxy-icon", out));
        self.macos_hidden.format_entry(&mut EntryFormatter::new("macos-hidden", out));
        EntryFormatter::new("bold-color", out)
            .entry_optional(self.bold_color, |v, f| v.format_entry(f));
    }
}
```

`Theme` is `Clone` (not `Copy`), so its optional is `.clone()`d into
`entry_optional`; the `Option<TerminalColor>` / `Option<BoldColor>` fields are
`Copy` and pass by value. Each field gets a fresh `EntryFormatter` bound to the
same `out`; the calls run sequentially so the `&mut out` borrows never overlap.

## Scope / faithfulness notes

- **Ported (bridged)**: the top-level `FileFormatter.format` default path, as
  `Config::format_config`. Each field is emitted via its already-ported
  `format_entry` (or `entry_bool` / `entry_optional` for raw fields), in
  upstream declaration order.
- **Faithful**: field **order** matches upstream's `Config` declaration order;
  the key names are the upstream config keys; each value uses the per-field
  formatter proven in earlier experiments.
- **Documented deviation**: `background-image-opacity` is omitted
  (float-formatting blocked, Experiment 509). It is the only gap; a test asserts
  its absence so the omission stays intentional and tracked.
- **Deferred**: `background-image-opacity` (until a faithful float formatter),
  `QuickTerminalSize`, the docs / changed-only paths of `FileFormatter`, and the
  config **loader** (`loadCli`, file I/O).
- No C ABI/header/ABI-inventory change (internal Rust).

## Changes

1. `roastty/src/config/mod.rs`: add
   `Config::format_config(&self, out: &mut String)` emitting the 43 non-float
   fields in upstream order.
2. Tests (in `config/mod.rs`): format a `Config::default()` and assert the
   emitted **keys** (the `key` before `=` on each line) equal the expected
   ordered list (43 keys, upstream order, no `background-image-opacity`); assert
   every line is `key = …`; assert `background-image-opacity` is absent.
3. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty config_format
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font roastty/src/renderer roastty/src/config && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `format_config` emits the 43 non-float fields, in upstream `Config`
  declaration order, each via its per-field formatter — faithful to
  `FileFormatter.format`;
- the test confirms the ordered key list and that `background-image-opacity` is
  absent, and the existing tests still pass;
- the float field and the loader stay deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if a field is emitted out of upstream order, a key name
diverges, a field is wrongly included/excluded, an unrelated item changes, or
any public C API/ABI changes.

## Design Review

This design was first reviewed when numbered Experiment 511; Codex raised one
**Required** finding — `Theme::format_entry` was not yet ported. That was
resolved by splitting it out (Experiment 511 ported `Theme::format_entry`,
approved), so the `theme` optional dispatch is now valid.

Codex then re-reviewed this aggregate design (Experiment 512) and **approved**
it with **no findings**: cloning `self.theme` is acceptable (`Theme` is
non-`Copy`) and the closure formats the owned inner value through the same
`EntryFormatter` name, matching the optional-recursion model
(`formatter.zig:62`, `Config.zig:9898`); the per-field dispatch choices are
correct (leaf types via `format_entry`, `background-image-repeat` via bool, the
six optionals via `entry_optional`); the field order matches upstream `Config`
declaration order for the curated subset; and omitting only
`background-image-opacity` is the right documented float-blocked gap
(`formatter_file.zig:40`, `Config.zig:639`). Codex noted the planned key-order
test is adequate and a spot-check for a `None` optional line (e.g.
`cursor-color = \n`) would be harmless — this experiment adds that assertion.

Review artifacts:

- Prompt: `logs/codex-review/20260604-165416-d512-prompt.md` (design)
- Result: `logs/codex-review/20260604-165416-d512-last-message.md` (design)

## Result

**Result:** Pass

`Config::format_config(&self, out: &mut String)` was implemented — the top-level
config dump (upstream `FileFormatter.format`). It emits the 43 non-float
`Config` fields in upstream `Config` declaration order, each via its
`format_entry` (or `entry_bool` for `background-image-repeat`; `entry_optional`
for the six `Option` fields). `theme` is `.clone()`d (non-`Copy`); the `Copy`
optionals pass by value. `background-image-opacity` is omitted (float-blocked,
Experiment 509). The new test
`config_format_config_emits_fields_in_upstream_order` asserts the exact 43-key
ordered list, the absence of `background-image-opacity`, and the default `None`
optionals' void lines (`cursor-color = \n`, `theme = \n`).

Gates:

- `cargo fmt -p roastty` accepted; `--check` clean.
- `cargo test -p roastty`: 2998 passed, 0 failed (one new test; no regressions).
- `cargo build -p roastty`: no warnings.
- no-`ghostty`-name greps (font/renderer/config + lib.rs/header/abi_harness.c)
  clean; `git diff --check` clean.

## Completion Review

Codex reviewed the completed experiment and **approved** it with **no
findings**: it spot-checked the body and test — the emitted keys are in upstream
`Config` declaration order for the curated subset, with only
`background-image-opacity` omitted at its float-blocked position
(`formatter_file.zig:40`, `Config.zig:639`); the dispatch choices are correct
(leaf `format_entry`, `entry_bool` for `background-image-repeat`,
`entry_optional` for `theme` / the terminal colors / `bold-color`); the prior
Theme issue is resolved (`Theme::format_entry` exists and is valid through the
optional recursion); the test verifies order, absence of the float-blocked
field, and representative `None` optional output; gates are clean. "Approved
with no findings."

Review artifacts:

- Prompt: `logs/codex-review/20260604-165636-r512-prompt.md` (result)
- Result: `logs/codex-review/20260604-165636-r512-last-message.md` (result)

## Conclusion

The config **formatter** layer is now complete end-to-end: every leaf type
formats via its `format_entry`, the generic field-dispatch helpers cover every
branch but the float `{d}` case, and `Config::format_config` dumps the whole
struct in upstream declaration order. The remaining float gap
(`background-image-opacity`) is tracked and re-inserts at its declared position
once a faithful float formatter lands. The next major piece is the config
**loader** — the inverse direction: parsing CLI arguments / config files into a
`Config` (upstream `loadCli` / `loadFile` and the per-field `parseCLI` dispatch
over the aggregate struct), most leaf `parse_cli` methods for which already
exist. After that comes the entire non-config rewrite.
