+++
[implementer]
agent = "codex"
model = "gpt-5"
reasoning = "high"

[review.design]
agent = "codex"
+++

# Experiment 68: Phase F — class config

## Description

Experiment 67 added the `link-url` and `maximize` config surfaces. The next
upstream config fields after the already-ported `title` field are:

- `class`
- `x11-instance-name`

Upstream declares both as optional sentinel strings in
`vendor/ghostty/src/config/Config.zig`:

- `class: ?[:0]const u8 = null`
- `@"x11-instance-name": ?[:0]const u8 = null`

Both fields affect GTK/X11/Wayland application identity at runtime. This
experiment ports only their config surfaces: fields, defaults, parsing/reset
behavior, formatting, diagnostics, and focused tests. Runtime GTK/X11/Wayland
application identity behavior is intentionally out of scope because roastty's
current target app path is the copied macOS app and later app-runtime wiring
should apply these values where relevant.

## Changes

- `roastty/src/config/mod.rs`
  - Add `Config::class: Option<String> = None`.
  - Add `Config::x11_instance_name: Option<String> = None`.
  - Route both keys through defaults, `Config::set`, `format_config`,
    clone/equality, and diagnostics using the existing optional string helper.
  - Preserve local formatter order around the upstream sequence:
    - `fullscreen`
    - `title`
    - `class`
    - `x11-instance-name`
  - Leave `working-directory` out of scope because it already has a richer value
    type and depends on finalize/launcher inheritance semantics that deserve a
    separate experiment.

Out of scope:

- Runtime GTK/X11/Wayland class/application-ID behavior.
- DBus single-instance behavior.
- `working-directory` as a config field and its finalize behavior.

## Verification

- Run formatting:
  - `cargo fmt`
  - `prettier --write --prose-wrap always --print-width 80 issues/0802-libroastty-completion-and-mac-app/README.md issues/0802-libroastty-completion-and-mac-app/68-class-config.md`
- Run targeted tests:
  - `cargo test -p roastty class_config`
  - `cargo test -p roastty config_format_config`
- Add concrete test cases proving:
  - both defaults are unset and format as empty optional string lines;
  - non-empty values parse and format for both keys;
  - empty values reset both fields to unset;
  - missing values return `ValueRequired`;
  - NUL-containing values return `InvalidValue`;
  - `Config::load_str` records `ConfigDiagnostic` line/key/error entries for
    invalid `class` and `x11-instance-name` lines while keeping valid
    neighboring lines;
  - formatter order places `class` after `title` and `x11-instance-name` after
    `class`;
  - clone/equality preserves both values.
- Run full Roastty tests:
  - `cargo test -p roastty`
- Run `cargo fmt --check`.
- Run `git diff --check`.
- Run `git status --short` and verify only intended source/docs are present.

**Pass** = `class` and `x11-instance-name` are represented faithfully on
`Config`, round-trip through config loading/formatting, match upstream optional
string parser/reset behavior, and have targeted and full tests passing.

**Partial** = one field lands faithfully but the other needs a follow-up, or a
parser/diagnostic/formatter-order edge remains before runtime use.

**Fail** = either field cannot be represented faithfully without first porting
working-directory or app-runtime identity infrastructure.

## Design Review

Codex adversarial reviewer `019eb3de-1563-7f10-821a-ddb3d609b4df` returned
**Approved** with no required findings.

The reviewer verified that the README links Exp68 as `Designed`, the experiment
has the required sections, the scope is narrow, the upstream defaults/order
match `title`, `class`, `x11-instance-name`, then `working-directory`, and
deferring `working-directory` is justified by upstream finalize/default
inheritance behavior. The reviewer had one nit to use canonical `cargo fmt` in
the implementation step; the verification plan was updated accordingly.
