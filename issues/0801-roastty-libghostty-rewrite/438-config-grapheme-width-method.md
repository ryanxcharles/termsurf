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

# Experiment 438: the grapheme-width-method config enum and its grapheme-cluster mapping (GraphemeWidthMethod, grapheme_cluster)

## Description

This experiment ports the `grapheme-width-method` config enum —
`GraphemeWidthMethod { Legacy, Unicode }` — **and the consumer logic** that maps
it to the terminal's grapheme-cluster mode. Upstream's termio init switches on
the method to set the initial `grapheme_cluster` mode (`.unicode` enables it,
`.legacy` does not); this experiment captures that switch as a
`GraphemeWidthMethod::grapheme_cluster` method. roastty already has the terminal
`Mode::GraphemeCluster` (the bit this gates); the termio init call site that
sets the mode stays deferred.

## Upstream behavior

In `config/Config.zig`, the enum and its `Config` field:

```zig
@"grapheme-width-method": GraphemeWidthMethod = .unicode,

pub const GraphemeWidthMethod = enum {
    legacy,
    unicode,
};
```

In `termio/Termio.zig`, the initial terminal modes switch on it:

```zig
// Setup our initial grapheme cluster support if enabled. We use a
// switch to ensure we get a compiler error if more cases are added.
switch (opts.full_config.@"grapheme-width-method") {
    .unicode => modes.grapheme_cluster = true,
    .legacy => {},
}
```

`unicode` (the `Config` field default) enables the terminal's `grapheme_cluster`
mode (full grapheme-cluster width); `legacy` leaves it off (legacy per-codepoint
width). The exhaustive `switch` is deliberate so a new variant forces a compile
error.

## Rust mapping (`roastty/src/config/mod.rs`)

```rust
/// The `grapheme-width-method` config (upstream `GraphemeWidthMethod`): how the
/// terminal measures grapheme width. The `Config` default is `Unicode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GraphemeWidthMethod {
    /// Legacy per-codepoint width (grapheme clustering off).
    Legacy,
    /// Full grapheme-cluster width (grapheme clustering on).
    Unicode,
}

impl GraphemeWidthMethod {
    /// Whether this method enables the terminal's grapheme-cluster mode (upstream
    /// termio init switch): `Unicode` enables it, `Legacy` does not.
    pub(crate) fn grapheme_cluster(self) -> bool {
        match self {
            GraphemeWidthMethod::Unicode => true,
            GraphemeWidthMethod::Legacy => false,
        }
    }
}
```

The `match` is exhaustive (no wildcard), mirroring upstream's deliberate
exhaustive `switch` — a new variant forces the arm to be handled.
`Unicode → true` / `Legacy → false` is exactly upstream's mode mapping.

## Scope / faithfulness notes

- **Ported (bridged)**: the `GraphemeWidthMethod` config enum
  (`config/Config.zig`) and its grapheme-cluster mapping
  (`GraphemeWidthMethod::grapheme_cluster`, upstream's `Termio.zig` init
  switch).
- **Faithful**: the enum has the two upstream variants (`legacy`, `unicode`);
  `grapheme_cluster` returns `true` for `Unicode` and `false` for `Legacy`,
  exactly the mode the init switch sets, with an exhaustive `match` mirroring
  the deliberate exhaustive `switch`.
- **Faithful adaptation**: the consumer is modeled as a method on the enum
  (upstream inlines the switch in termio init); it returns the
  `grapheme_cluster` bool rather than mutating a `ModePacked` (the mode struct
  mutation is the deferred termio init).
- **Deferred**: the `Config` struct / parsing (and the field default), and the
  termio init call site that sets `Mode::GraphemeCluster` from this method.
  (Consumed by a later slice; this experiment lands the enum and the mapping.)
- No C ABI/header/ABI-inventory change (internal Rust).

## Changes

1. `roastty/src/config/mod.rs`:
   - add `pub(crate) enum GraphemeWidthMethod { Legacy, Unicode }` (derive
     `Debug, Clone, Copy, PartialEq, Eq`) and
     `GraphemeWidthMethod::grapheme_cluster(self) -> bool` (exhaustive `match`).
   - broaden the module-level doc: it currently says the config layer holds "the
     leaf enums the renderer consumes" — `GraphemeWidthMethod` is consumed by
     the termio/terminal-mode bridge, so reword to "the leaf config types
     consumed by the renderer / terminal bridge".
2. Tests (in `config/mod.rs`):
   - `grapheme_cluster`: `Unicode.grapheme_cluster() == true`,
     `Legacy.grapheme_cluster() == false`; the two variants are distinct and
     round-trip `Copy`/`Eq`.
3. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty grapheme_width_method
cargo test -p roastty grapheme_cluster
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font roastty/src/renderer roastty/src/config && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `GraphemeWidthMethod` has the two upstream variants and `grapheme_cluster`
  returns `true` for `Unicode` / `false` for `Legacy` via an exhaustive `match`
  — faithful to upstream's enum and termio init switch;
- the tests pass (the mapping; the distinct variants), and the existing tests
  still pass;
- the `Config` struct and the termio init call site stay deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if a variant is missing/extra, `grapheme_cluster` maps
a variant the wrong way, a wildcard `match` arm hides a future variant, an
unrelated item changes, or any public C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and **approved** it with **one
Low finding** (now folded into the Changes), no Required or Recommended
findings. It verified against the vendored upstream: the variants match exactly
(`legacy` / `unicode`, `Config.zig:9591`); the default belongs on the deferred
`Config` field (`.unicode`, `Config.zig:507`), not the enum;
`grapheme_cluster()` is a faithful extraction of the termio init switch
(`Termio.zig:229`, `Unicode → true`, `Legacy → false`); and an exhaustive
`match` with no wildcard is the right Rust equivalent of upstream's deliberate
exhaustive `switch`. It judged the test adequate (both variants, the bool
mapping, distinctness, `Copy`/`Eq`).

- **Low (fixed)**: the module-level doc says the config layer holds "the leaf
  enums the renderer consumes", but `GraphemeWidthMethod` is consumed by the
  termio/terminal-mode bridge, not the renderer directly. Folded into the
  Changes: the module doc is reworded to "the leaf config types consumed by the
  renderer / terminal bridge".

Review artifacts:

- Prompt: `logs/codex-review/20260604-102141-d438-prompt.md` (design)
- Result: `logs/codex-review/20260604-102141-d438-last-message.md` (design)

## Result

**Result:** Pass

The grapheme-width-method config enum and its grapheme-cluster mapping are now
live.

- `roastty/src/config/mod.rs`:
  `pub(crate) enum GraphemeWidthMethod { Legacy, Unicode }` (upstream
  `GraphemeWidthMethod`) and
  `GraphemeWidthMethod::grapheme_cluster(self) -> bool` — an exhaustive `match`
  (`Unicode → true`, `Legacy → false`), the extraction of upstream's termio init
  switch. The module-level doc was broadened to "the leaf config types consumed
  by the renderer / terminal bridge".

Test (in `config/mod.rs`): `grapheme_width_method_maps_to_grapheme_cluster` —
`Unicode.grapheme_cluster() == true`, `Legacy.grapheme_cluster() == false`, the
variants distinct, `Copy`/`Eq`.

Gate results:

- `cargo fmt -p roastty` accepted; `--check` clean.
- `cargo test -p roastty` → 2925 passed, 0 failed (+1, no regressions).
- `cargo build -p roastty` → no warnings.
- No-`ghostty`-name gates (font + renderer + config +
  `lib.rs`/header/`abi_harness.c`) clean; `git diff --check` clean.

## Conclusion

The config layer now carries `GraphemeWidthMethod` and its grapheme-cluster
mapping — a second config slice (after `FontShapingBreak`) to land its consumer
logic alongside the type, here as a method that returns the bool the termio init
switch sets for `Mode::GraphemeCluster`. The `Config` struct / parsing and the
termio init call site (which sets `Mode::GraphemeCluster` from this method) stay
deferred. The config-type family — increasingly pairing a config type with its
behavior — remains a clean, gated way to advance the rewrite while the larger
coupled subsystems stay deferred.

## Completion Review

Codex reviewed the completed implementation and result and **approved** with
**no findings**. It confirmed the Low resolved (the module doc now covers leaf
config types consumed by the renderer / terminal bridge) and verified
faithfulness against the vendored upstream:
`GraphemeWidthMethod::{Legacy, Unicode}` matches `Config.zig:9591`; the deferred
default `.unicode` (`Config.zig:507`) is documented and left off the enum;
`grapheme_cluster()` exactly extracts the `Termio.zig:229` switch
(`Unicode → true`, `Legacy → false`); and the exhaustive `match` preserves
upstream's compile-time pressure. It judged the test to cover both variants, the
bool mapping, distinctness, and `Copy`/`Eq`. No public C ABI/header impact;
nothing needed to change before the result commit.

Review artifacts:

- Prompt: `logs/codex-review/20260604-102356-r438-prompt.md` (result)
- Result: `logs/codex-review/20260604-102356-r438-last-message.md` (result)
