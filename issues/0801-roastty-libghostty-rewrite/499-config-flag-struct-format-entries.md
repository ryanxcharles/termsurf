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

# Experiment 499: the packed-struct flag formatters (ShellIntegrationFeatures / ScrollToBottom / NotifyOnCommandFinishAction)

## Description

Continuing the config **formatter** layer (Experiments 491–498), this experiment
ports the **packed-struct flag** format for three config flag-structs —
`ShellIntegrationFeatures`, `ScrollToBottom`, and `NotifyOnCommandFinishAction`.
These have no custom `formatEntry`; upstream auto-formats them via the generic
formatter's packed-struct branch as a comma-joined `[no-]field` list (e.g.
`cursor,no-sudo,title,…`). This experiment ports that branch as a shared helper
and wires `format_entry` for the three flag-structs, grounded by the
`EntryFormatter` from Experiment 491.

## Upstream behavior

In `config/formatter.zig`, the generic `formatEntry` packed-struct branch:

```zig
.@"packed" => {
    try writer.print("{s} = ", .{name});
    inline for (info.fields, 0..) |field, i| {
        if (i > 0) try writer.print(",", .{});
        try writer.print("{s}{s}", .{
            if (!@field(value, field.name)) "no-" else "",
            field.name,
        });
    }
    try writer.print("\n", .{});
}
```

A packed struct of bools formats to `name = ` followed by each field,
comma-joined, each rendered as its field name prefixed with `no-` when the field
is `false`. The field names are the config keywords (e.g. `@"ssh-env"` →
`ssh-env`). So a default `ShellIntegrationFeatures` (`cursor = true`,
`sudo = false`, `title = true`, `ssh-env = false`, `ssh-terminfo = false`,
`path = true`) formats to
`name = cursor,no-sudo,title,no-ssh-env,no-ssh-terminfo,path\n`.

The three flag-structs (upstream `packed struct`s):

- `ShellIntegrationFeatures`: `cursor`, `sudo`, `title`, `ssh-env`,
  `ssh-terminfo`, `path`.
- `ScrollToBottom`: `keystroke`, `output`.
- `NotifyOnCommandFinishAction`: `bell`, `notify`.

## Rust mapping

`roastty/src/config/formatter.rs` — a shared packed-struct flag helper (the
generic branch):

```rust
impl EntryFormatter<'_> {
    /// `name = [no-]field,[no-]field…\n` (upstream the packed-struct case): each
    /// flag is its keyword, prefixed with `no-` when `false`.
    pub(crate) fn entry_flags(&mut self, fields: &[(&str, bool)]) {
        let joined = fields
            .iter()
            .map(|&(name, on)| if on { name.to_string() } else { format!("no-{}", name) })
            .collect::<Vec<_>>()
            .join(",");
        self.entry_str(&joined);
    }
}
```

`roastty/src/config/mod.rs` — `format_entry` for the three flag-structs:

```rust
impl ShellIntegrationFeatures {
    pub(crate) fn format_entry(self, formatter: &mut EntryFormatter) {
        formatter.entry_flags(&[
            ("cursor", self.cursor),
            ("sudo", self.sudo),
            ("title", self.title),
            ("ssh-env", self.ssh_env),
            ("ssh-terminfo", self.ssh_terminfo),
            ("path", self.path),
        ]);
    }
}

impl ScrollToBottom {
    pub(crate) fn format_entry(self, formatter: &mut EntryFormatter) {
        formatter.entry_flags(&[("keystroke", self.keystroke), ("output", self.output)]);
    }
}

impl NotifyOnCommandFinishAction {
    pub(crate) fn format_entry(self, formatter: &mut EntryFormatter) {
        formatter.entry_flags(&[("bell", self.bell), ("notify", self.notify)]);
    }
}
```

`entry_flags` mirrors upstream's packed-struct branch: each field is rendered as
its keyword (prefixed with `no-` when `false`), comma-joined, written as one
`name = …\n` entry. Each `format_entry` passes its fields in upstream order with
the config keywords (the hyphenated `ssh-env` / `ssh-terminfo` mapped from the
Rust `ssh_env` / `ssh_terminfo` field names). All three structs are `Copy`, so
`format_entry` takes `self` by value.

## Scope / faithfulness notes

- **Ported (bridged)**: the shared `EntryFormatter::entry_flags` (upstream's
  generic packed-struct format branch) and `format_entry` for
  `ShellIntegrationFeatures`, `ScrollToBottom`, and
  `NotifyOnCommandFinishAction`.
- **Faithful**: the `name = ` prefix; each field as `[no-]keyword`; the comma
  separator; the upstream field order and keyword names (incl. the hyphenated
  `ssh-env` / `ssh-terminfo`) — exactly upstream's packed-struct branch.
- **Faithful adaptation**: the comptime `inline for` over the packed fields → an
  explicit `&[(keyword, bool)]` slice per struct (Rust has no comptime field
  reflection, and the Rust field names use `_` where the keywords use `-`);
  `formatEntry`-into-writer → `entry_str` of the joined string.
- **Deferred**: the other generic field-dispatch cases (enum `{t}` tag-keyword,
  float `{d}`, optional recurse), the remaining custom `formatEntry` (only
  `QuickTerminalSize`, deferred with its `parseFloat`-blocked parser), and the
  broader config parser/formatter.
- No C ABI/header/ABI-inventory change (internal Rust).

## Changes

1. `roastty/src/config/formatter.rs`: add
   `EntryFormatter::entry_flags(&[(&str, bool)])`.
2. `roastty/src/config/mod.rs`: add `ShellIntegrationFeatures::format_entry`,
   `ScrollToBottom::format_entry`, and
   `NotifyOnCommandFinishAction::format_entry` (each in its existing `impl`, or
   a new `impl` if none).
3. Tests:
   - in `config/formatter.rs`: `entry_flags(&[("a", true), ("b", false)])` →
     `"x = a,no-b\n"`.
   - in `config/mod.rs`: a `ShellIntegrationFeatures` with
     `cursor=true, sudo=false, title=true, ssh_env=false, ssh_terminfo=false, path=true`
     → `"a = cursor,no-sudo,title,no-ssh-env,no-ssh-terminfo,path\n"`; a
     `ScrollToBottom` `{keystroke=true, output=false}` →
     `"a = keystroke,no-output\n"`, and `{false,false}` →
     `"a = no-keystroke,no-output\n"`; a `NotifyOnCommandFinishAction`
     `{bell=true, notify=false}` → `"a = bell,no-notify\n"`.
4. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty entry_flags
cargo test -p roastty flag_struct_format
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font roastty/src/renderer roastty/src/config && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `entry_flags` and the three `format_entry` methods write the `[no-]keyword`
  comma-joined entry with the upstream field order and keywords — faithful to
  upstream's packed-struct branch;
- the tests pass (the helper; the three structs' defaults / toggles), and the
  existing tests still pass;
- the other generic field-dispatch cases and the remaining custom `formatEntry`
  stay deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if a formatted entry differs from upstream (wrong
keyword, wrong `no-` handling, wrong order/separator), an unrelated item
changes, or any public C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and **approved** it with **no
findings**. It confirmed `entry_flags` is a faithful adaptation of the upstream
packed-struct formatter (one line of comma-separated field names, prefixing
`no-` for false values — `formatter.zig:98`), and that the three field lists and
orders match upstream exactly: `ShellIntegrationFeatures` —
`cursor`/`sudo`/`title`/`ssh-env`/`ssh-terminfo`/`path` (`Config.zig:8672`);
`ScrollToBottom` — `keystroke`/`output` (`:10206`);
`NotifyOnCommandFinishAction` — `bell`/`notify` (`:10221`); and the proposed
tests cover the helper and each struct's expected output shape, including the
hyphenated keyword mapping.

Review artifacts:

- Prompt: `logs/codex-review/20260604-154952-d499-prompt.md` (design)
- Result: `logs/codex-review/20260604-154952-d499-last-message.md` (design)
