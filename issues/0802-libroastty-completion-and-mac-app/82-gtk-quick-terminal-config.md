+++
[implementer]
agent = "codex"
model = "gpt-5"
reasoning = "high"

[review.design]
agent = "codex"
+++

# Experiment 82: Phase F — GTK quick terminal config

## Description

Experiment 81 wired `quick-terminal-size`. The next unported upstream config
fields are the adjacent GTK Wayland quick-terminal settings:

- `gtk-quick-terminal-layer`
- `gtk-quick-terminal-namespace`

Upstream declares `gtk-quick-terminal-layer` as `QuickTerminalLayer = .top`,
where the valid enum tags are `overlay`, `top`, `bottom`, and `background`.
Upstream declares `gtk-quick-terminal-namespace` as a sentinel-terminated string
with default `"ghostty-quick-terminal"`.

This experiment adds the Rust config parser/formatter surface for both fields.
GTK/Wayland runtime behavior, quick-terminal window layering, and app C ABI
accessors are out of scope.

## Changes

- `roastty/src/config/mod.rs`
  - Add `Config::gtk_quick_terminal_layer` with upstream default `top`.
  - Add `QuickTerminalLayer::{Overlay, Top, Bottom, Background}`.
  - Route `gtk-quick-terminal-layer` through `Config::set`, config loading
    diagnostics, clone/equality, and formatting. An empty value resets to the
    upstream default `top`.
  - Add `Config::gtk_quick_terminal_namespace` with upstream default
    `"ghostty-quick-terminal"`.
  - Route `gtk-quick-terminal-namespace` through `Config::set`, config loading
    diagnostics, clone/equality, and formatting as a required string field. An
    empty value resets to the upstream default `"ghostty-quick-terminal"`.
  - Preserve the current local formatter convention by inserting both keys after
    `quick-terminal-size`, matching upstream declaration order.

Out of scope:

- Any GTK/Wayland layer-shell runtime behavior.
- Runtime quick-terminal creation, positioning, sizing, focus, autohide, or
  toggle actions.
- C ABI `roastty_config_get` exposure for either field; Exp 10 documented that
  the app accessor is currently inert and that remains a later
  feature-completion item.
- The following quick-terminal fields: `quick-terminal-screen`,
  `quick-terminal-animation-duration`, and `quick-terminal-autohide`.
- Any broader formatter reordering of already-ported keys.

## Verification

- Run formatting:
  - `cargo fmt`
  - `prettier --write --prose-wrap always --print-width 80 issues/0802-libroastty-completion-and-mac-app/README.md issues/0802-libroastty-completion-and-mac-app/82-gtk-quick-terminal-config.md`
- Run targeted tests:
  - `cargo test -p roastty gtk_quick_terminal`
  - `cargo test -p roastty config_format_config`
- Add concrete test cases proving:
  - defaults are `QuickTerminalLayer::Top` and `"ghostty-quick-terminal"`;
  - default `format_config` emits both keys after `quick-terminal-position` and
    before `font-family`, because default `quick-terminal-size` emits no entry;
  - when `quick-terminal-size` is present, both GTK keys emit after it and
    before `font-family`;
  - all four layer keywords parse and format;
  - an empty layer value resets to `top`;
  - unknown layer keywords are `ConfigSetError::InvalidValue`;
  - missing layer values are `ConfigSetError::ValueRequired`;
  - the namespace parses and formats a non-empty custom string;
  - an empty namespace value resets to `"ghostty-quick-terminal"`, matching
    upstream's generic empty-value reset before string parsing;
  - a missing namespace value is `ConfigSetError::ValueRequired`;
  - an embedded NUL namespace is rejected as `ConfigSetError::InvalidValue`;
  - `Config::load_str` records diagnostics for invalid neighboring GTK
    quick-terminal lines while preserving valid parsed values;
  - clone/equality preserves both field values.
- Run full Roastty tests:
  - `cargo test -p roastty`
- Run `cargo fmt --check`.
- Run `git diff --check`.
- Run `git status --short` and verify only intended source/docs are present.

**Pass** = both GTK quick-terminal fields are represented faithfully on
`Config`, round-trip through config loading/formatting, match upstream defaults
and parser behavior for this slice, and have targeted and full tests passing.

**Partial** = one field lands completely, but the other requires a follow-up.

**Fail** = either key cannot be represented faithfully without first
implementing GTK quick-terminal runtime behavior or C ABI accessors.

## Design Review

Codex adversarial reviewer `019eb489-0a39-7a23-8bb3-c8e874224a9a` initially
returned **Changes Required** with one required finding: the design incorrectly
said an empty `gtk-quick-terminal-namespace` value should persist and format as
an empty string. The reviewer pointed to upstream `cli/args.zig`, where a
set-but-empty value resets to the field default before string parsing. The same
empty-reset rule also applies to `gtk-quick-terminal-layer`.

The design was fixed to require empty `gtk-quick-terminal-layer` to reset to
`top` and empty `gtk-quick-terminal-namespace` to reset to
`"ghostty-quick-terminal"`.

Codex re-reviewer `019eb489-0a39-7a23-8bb3-c8e874224a9a` returned **Approved**
with no findings. The reviewer confirmed the prior finding is resolved, the
design matches upstream's empty-reset branch before enum/string parsing, and the
README links Experiment 82 as `Designed`.

## Result

**Result:** Pass

Implemented `gtk-quick-terminal-layer` in `roastty/src/config/mod.rs` as
`QuickTerminalLayer::{Overlay, Top, Bottom, Background}` with upstream default
`Top`. The enum now parses exact upstream keywords, formats through the existing
enum formatter path, resets an empty value to `top`, and reports missing or
unknown values through the expected `ConfigSetError` variants.

Implemented `gtk-quick-terminal-namespace` as a required `String` field with
upstream default `"ghostty-quick-terminal"`. The setter uses the non-optional
field path, so an empty value resets to the default before string parsing,
matching upstream's generic empty-value rule. Missing values report
`ValueRequired`, and embedded NUL bytes report `InvalidValue`.

The plan's initial order wording was refined during implementation:
`quick-terminal-size` emits no config entry at its default, so default
`format_config` places the GTK quick-terminal keys after
`quick-terminal-position`. When `quick-terminal-size` is present, the GTK keys
follow it and precede `font-family`, preserving upstream declaration order among
emitted keys.

Verification passed:

- `cargo fmt`
- `cargo test -p roastty gtk_quick_terminal`
- `cargo test -p roastty config_format_config`
- `cargo test -p roastty`
  - 4520 unit tests passed
  - ABI harness passed with the existing 10 enum-conversion warnings
  - doc tests passed
- `cargo fmt --check`
- `git diff --check`

## Conclusion

The GTK quick-terminal config surface now matches upstream defaults, enum/string
parser behavior, empty-reset behavior, formatter output, and diagnostics for
this slice. GTK/Wayland layer-shell runtime behavior and app C ABI accessors
remain later work. The next upstream quick-terminal fields are
`quick-terminal-screen`, `quick-terminal-animation-duration`, and
`quick-terminal-autohide`.

## Completion Review

Codex adversarial reviewer `019eb492-8d96-7873-9b03-68733cbc1086` returned
**Approved** with no required findings.

The reviewer performed read-only verification that `git diff --check` passed,
`cargo fmt --check` passed, `cargo test -p roastty gtk_quick_terminal` passed,
`cargo test -p roastty config_format_config` passed, and `cargo test -p roastty`
passed with 4520 unit tests plus the ABI harness and doc tests. The reviewer
confirmed the implementation matches the reviewed scope: upstream defaults, enum
keywords, empty-value resets, missing/invalid diagnostics, formatter ordering
nuance, docs/result status, and no runtime behavior or C ABI accessor work.
