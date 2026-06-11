+++
implementer = "codex"
review_design = "codex-adversarial"
+++

# Experiment 94: Phase F — GTK CSS, notifications, and progress config

## Description

Port the next pinned upstream config fields from
`vendor/ghostty/src/config/Config.zig` into `roastty/src/config/mod.rs`:

- `gtk-custom-css: RepeatablePath = .{}`
- `desktop-notifications: bool = true`
- `progress-style: bool = true`

These fields are adjacent immediately after the GTK chrome group and before
`bold-color` upstream. This experiment remains parser/formatter-only for the new
fields. Loading GTK CSS files into a GTK application, enforcing the documented
5MiB stylesheet limit, runtime desktop notification escape handling, progress
bar escape handling, app C ABI exposure, and GTK app integration remain later
work.

## Changes

- `roastty/src/config/mod.rs`
  - Add `Config` fields to the current local GTK struct/default block after
    `gtk_wide_tabs`:
    - `gtk_custom_css: RepeatableConfigPath`
    - `desktop_notifications: bool`
    - `progress_style: bool`
  - Initialize defaults to upstream values:
    - `gtk_custom_css = RepeatableConfigPath::default()`
    - `desktop_notifications = true`
    - `progress_style = true`
  - Format the fields after `gtk-wide-tabs` and before `bold-color`, preserving
    upstream order in formatter output and format-order tests. Do not reorder
    unrelated existing struct fields such as the local `bold_color` field.
  - Route `gtk-custom-css` through the existing `RepeatableConfigPath`
    parser/formatter, matching local `custom-shader` and `config-file`
    semantics:
    - repeated entries append;
    - `?path` marks entries optional;
    - quoted `"?literal"` preserves a literal leading `?`;
    - raw empty resets the list to empty;
    - missing value reports `ValueRequired`;
    - parsed-empty paths are ignored.
  - Expand `gtk-custom-css` relative to config file and CLI bases in
    `expand_paths_from_base`.
  - Route `desktop-notifications` and `progress-style` through `set_bool_field`,
    so explicit bools parse normally, bare keys set `true`, empty values reset
    to `true`, and invalid values report `InvalidValue`.
  - Extend default-value, format-order, repeatable-path, bool-route, path-base
    expansion, diagnostics, and clone/equality tests.

- `issues/0802-libroastty-completion-and-mac-app/README.md`
  - Link this experiment as `Designed` in the experiment index.
  - After implementation, add an operating note describing the parser-only
    status and runtime work left open.

## Verification

Before implementation:

- Codex-native adversarial design review approves the experiment.
- Plan commit exists before source edits begin.

After implementation:

- `cargo fmt`
- `cargo test -p roastty gtk_css_notifications_progress`
- `cargo test -p roastty config_format_config`
- `cargo test -p roastty`
- `cargo fmt --check`
- `git diff --check`

Pass criteria:

- The three fields are present in defaults, formatter output, `Config::set`, and
  format-order tests in the current local formatter region.
- `gtk-custom-css` preserves upstream repeatable path syntax and local path-base
  expansion behavior for both config-file and CLI sources.
- `desktop-notifications` and `progress-style` preserve local bool semantics:
  bare key sets `true`, empty value resets to default `true`, and invalid values
  diagnose as `InvalidValue`.
- Runtime GTK CSS loading, terminal desktop-notification escape behavior, and
  progress-style escape behavior are not claimed or changed by this experiment.

## Design Review

Codex-native adversarial reviewer `019eb587-061c-7730-a1ac-71bda8a63dd7`
returned **Changes Required**: the initial design said to add the new `Config`
fields after `gtk_wide_tabs` and before `bold_color`, but local struct order
already has `bold_color` before the GTK block. The plan was updated to add the
fields to the local GTK struct/default block after `gtk_wide_tabs`, while
separately preserving upstream formatter output after `gtk-wide-tabs` and before
`bold-color`.

Codex-native adversarial re-reviewer `019eb587-061c-7730-a1ac-71bda8a63dd7`
returned **Approved** with no findings.
