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

# Experiment 268: completeStyles synthesis — synthesize vs. alias

## Description

Experiment 266 made `complete_styles` always alias a missing style; Experiment
267 ported the synthetic-face primitives. This experiment wires them together:
`complete_styles` takes a synthetic-style config and, per missing style, either
**synthesizes** a face (when enabled) or **aliases** to the regular face (when
disabled). This completes the faithful `completeStyles` (`font/Collection.zig`
lines 374–465).

### Upstream behavior (`font/Collection.zig`)

- A `synthetic_config: FontSyntheticStyle` with `italic`/`bold`/`bold-italic`
  booleans.
- **italic** (lines 377–398): if absent — disabled → alias to regular; else
  synthesize italic from the regular entry (on failure → alias).
- **bold** (lines 401–422): same shape, synthesizing bold from regular.
- **bold-italic** (lines 426–465): if absent — disabled → alias to regular; else
  prefer synthesizing **italic-on-bold** when bold was _originally_ present
  (`have_bold`, captured before bold completion), using `bold[0]`'s entry; else
  synthesize **bold-on-italic** using `italic[0]`'s entry; on total failure →
  alias to that base entry.

`have_italic`/`have_bold` are captured **before** the respective completion so
the bold-italic preference reflects the originally-configured styles.

### Rust mapping (`roastty/src/font/collection.rs`)

- `struct SyntheticStyle { italic: bool, bold: bool, bold_italic: bool }` — the
  config (upstream's `config.FontSyntheticStyle`; the config subsystem is a
  separate future area, so it's defined here for now).
- `complete_styles(&mut self, syn: SyntheticStyle) -> Result<(), CompleteError>`
  (the `syn` parameter is new): after finding `regular` (unchanged), capture
  `have_italic`/`have_bold`, then:
  - **italic** (if `faces[Italic]` empty): if `syn.italic`, synthesize —
    `let f = self.get_face(regular).expect(…).synthetic_italic(); self.add(f, Italic, false).expect(…);`
    — else `self.add_alias(Italic, regular).expect(…)`.
  - **bold** (if `faces[Bold]` empty): if `syn.bold`, `synthetic_bold` of the
    regular face → `add(_, Bold, false)`; else `add_alias(Bold, regular)`.
  - **bold-italic** (if `faces[BoldItalic]` empty): if `!syn.bold_italic`,
    `add_alias(BoldItalic, regular)`; else if `have_bold`, synthesize **italic**
    from `get_face(Index::new(Bold, 0))` → `add(_, BoldItalic, false)`; else
    synthesize **bold** from `get_face(Index::new(Italic, 0))` →
    `add(_, BoldItalic, false)`.

`CompleteError` has only `DefaultUnavailable`, so the
`get_face`/`add`/`add_alias` calls are **invariant-backed `expect(...)`s**, not
`?`: `regular` (and `Bold,0`/ `Italic,0` after those styles are completed) is a
validated direct entry so `get_face` can't error, and each destination style
list is known empty so `add` can't hit `CollectionFull`. (This mirrors the
Experiment 266 `add_alias` `expect`.) Because the Rust synthetic methods are
infallible, the upstream "synthesis failed → alias" fallbacks don't occur
(documented). Borrows: `get_face` returns a `&Face`; calling a `synthetic_*`
method on it yields an **owned** `Face`, ending the immutable borrow before the
`&mut self` `add`.

- **Two `Face` accessors** (for tests and the future renderer/shaper):
  `synthetic_bold_width(&self) -> Option<f64>` (the stored line width) and
  `is_skewed(&self) -> bool` (the face's transform has a non-zero shear `c`,
  i.e. it's an oblique/synthetic-italic face).

### Scope / faithfulness notes

- **Deferred**: the real `config` subsystem (`SyntheticStyle` is a local
  stand-in) and the synthesis-failure alias fallbacks (synthesis can't fail
  here).
- The Experiment 266 `complete_styles` tests are updated to pass an all-disabled
  `SyntheticStyle` (pure aliasing — their existing assertions hold unchanged).
- No C ABI/header/ABI-inventory change.

## Changes

1. `roastty/src/font/face/coretext.rs`: add `synthetic_bold_width` and
   `is_skewed` accessors.
2. `roastty/src/font/collection.rs`: add `SyntheticStyle`; add the `syn`
   parameter and the synthesis branches to `complete_styles`.
3. Update the Experiment 266 `complete_styles` tests to pass
   `SyntheticStyle { italic: false, bold: false, bold_italic: false }`.
4. New tests (live CoreText, macOS):
   - `complete_styles_synthesizes`: Menlo-only `Regular` + all-enabled
     `SyntheticStyle`; after completion,
     `get_face(Bold,0).synthetic_bold_width()` is `Some`,
     `get_face(Italic,0).is_skewed()` is `true`, and `get_face(BoldItalic,0)` is
     both bold (`synthetic_bold_width().is_some()`) and skewed (italic-on-bold
     preferred, but `have_bold` is false here so it's bold-on-italic → still
     bold; assert it has the bold width and, since its base is the synthetic
     italic, also skewed).
   - `complete_styles_bold_italic_prefers_bold`: Menlo under `Regular` **and**
     `Bold` (so `have_bold` is true), all-enabled; `BoldItalic` is synthesized
     as **italic-on-bold** → `is_skewed()` true and (since synthesized from the
     plain bold, which is itself only `synthetic_bold` if Bold was synthetic —
     here Bold is a real Menlo entry, so not bold-width) — assert `is_skewed()`
     is true (italic applied to the bold base).
   - `complete_styles_alias_when_disabled`: Menlo-only `Regular` + all-disabled
     `SyntheticStyle`; `get_face(Bold,0).synthetic_bold_width()` is `None` and
     `get_face(Italic,0).is_skewed()` is `false` (they alias the plain regular).
5. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty collection
cargo test -p roastty face
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `complete_styles` synthesizes the missing style when its config flag is set
  (italic from regular, bold from regular, bold-italic preferring italic-on-bold
  when bold was originally present else bold-on-italic) and aliases when the
  flag is off;
- `have_bold`/`have_italic` are captured before completion so the bold-italic
  preference is faithful;
- the synthesized faces are distinguishable (bold width / skew) from aliases;
- the Experiment 266 aliasing tests still pass under an all-disabled config;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment is **partial** if the borrow shape (get_face → synthesize → add)
needs restructuring.

The experiment **fails** if the synthesize/alias selection or the bold-italic
preference diverges from upstream, or any public C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and raised one **Medium**
finding: the sketched control flow used `?` on `get_face`/`add`, which won't
typecheck because `CompleteError` has only `DefaultUnavailable` (no `From`
conversions for `EntryError`/`AddError`). The design was revised to use
invariant-backed `expect(...)`s instead — `regular` (and `Bold,0`/`Italic,0`
after their styles are completed) is a validated direct entry so `get_face`
can't error, and each destination style list is known empty so `add` can't hit
`CollectionFull` (mirroring the Experiment 266 `add_alias` `expect`). Codex's
re-review confirmed the finding is resolved, approved the design, and
sanity-checked the borrow (`get_face(regular).expect(..).synthetic_italic()`
yields an owned `Face`, ending the `&Face` borrow before the `&mut self` `add`).

Review artifacts:

- Prompts: `logs/codex-review/20260602-222514-898742-prompt.md`,
  `…-222620-054216-prompt.md`
- Results: `logs/codex-review/20260602-222514-898742-last-message.md`,
  `…-222620-054216-last-message.md`
