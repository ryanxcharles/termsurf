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

# Experiment 333: the resolver's discovery-based fallback

## Description

Discovery is now complete (`Descriptor::discover_faces`, Experiment 332), but
the **resolver** still cannot use it: `get_index` has a
`// Discovery-based fallback is deferred here` placeholder. This experiment
wires discovery into the resolver — the architecturally central connection — so
a regular-style codepoint with **no loaded face** triggers a `discover`, and the
first discovered face that has the codepoint (in the requested presentation) is
**added to the collection** as a fallback and returned. This is what makes
discovery useful to the rest of the font system. This experiment lands the
resolver-fallback **structure** over the **general** discovery
(`discover_faces`); the dedicated `discoverFallback` (the
`CTFontCreateForString` codepoint search that surfaces extra candidates, e.g.
for CJK) is a separate later experiment.

## Upstream behavior (`CodepointResolver.zig` `getIndex`)

```zig
// If we are regular, try looking for a fallback using discovery.
if (style == .regular and font.Discover != void) {
    if (self.discover) |disco| discover: {
        const load_opts = self.collection.load_options orelse break :discover;
        var disco_it = disco.discoverFallback(alloc, &self.collection, .{
            .codepoint = cp,
            .size = load_opts.size.points,
            .bold = style == .bold or style == .bold_italic,   // false for regular
            .italic = style == .italic or style == .bold_italic, // false for regular
            .monospace = false,
        }) catch break :discover;
        while (disco_it.next()) |deferred_face| {
            // Discovery can't filter presentation, so check it here.
            const face: Entry = .{ .face = .{ .deferred = deferred_face }, .fallback = true };
            if (!face.hasCodepoint(cp, p_mode)) continue;
            return self.collection.addDeferred(alloc, deferred_face, .{
                .style = style, .fallback = true,
                .size_adjustment = default_fallback_adjustment, // .ic_width
            });
        }
    }
}
```

The fallback search runs only for **regular** style (a non-regular request
already recursed to regular). Its descriptor seeks the **codepoint**, with
`monospace = false` and the style's bold/italic (both `false` here). Each
discovered face is checked for the codepoint **in the requested presentation**
(discovery can't filter presentation); the first match is added to the
collection as a **fallback** face with the `ic_width` size adjustment, and its
index is returned.

## Rust mapping (`roastty/src/font/codepoint_resolver.rs`)

- `get_index` becomes **`&mut self`** (it mutates the collection on a fallback
  hit). The recursive calls and the test call sites adopt `&mut`/`let mut r`.
- A `discover_enabled: bool` toggle (the analog of upstream's
  `self.discover != null`): `new` sets it `false`; add
  `set_discover_enabled(&mut self, bool)`. Discovery requires opt-in, like
  `sprite_metrics`.
- In `get_index`, after the non-regular retry and **before** the last-resort
  `Any`:
  ```rust
  if style == Style::Regular && self.discover_enabled {
      let req = Descriptor { codepoint: cp, monospace: false, ..Default::default() };
      for face in req.discover_faces() {
          if fallback_face_has_codepoint(&face, cp, p_mode) {
              if let Ok(idx) = self.collection.add_with_adjustment(
                  face, Style::Regular, true, SizeAdjustment::IcWidth) {
                  return Some(idx);
              }
          }
      }
  }
  ```
- `fn fallback_face_has_codepoint(face: &Face, cp: u32, p_mode: PresentationMode) -> bool`
  replicates a **fallback** `Entry`'s `has_codepoint`: a fallback entry treats
  `Default(p)` as `Explicit(p)`, so the glyph must be present **and** the
  presentation must match (`Text ⇒ !is_color_glyph`, `Emoji ⇒ is_color_glyph`);
  `Any ⇒ presence only`.

## Scope / faithfulness notes

- **Ported**: the resolver's discovery-based fallback **structure** — the
  regular-style, discovery-enabled gate; iterating discovered candidate faces;
  the per-face presentation check; and adding the first match to the collection
  as a fallback (`ic_width` adjustment) and returning its index. This is the
  integration that connects discovery to `get_index`.
- **Candidate source — explicitly scoped (not a full `discoverFallback`)**: this
  experiment uses `Descriptor::discover_faces` (the **general**
  `CTFontCollection` match, ranked by `Score` with codepoint coverage as the top
  bit) as the candidate source. It is **not** a faithful substitute for
  upstream's `discoverFallback`: upstream's `discoverFallback` can surface
  **additional** candidates via `discoverCodepoint`/`CTFontCreateForString`
  (notably for CJK unified ideographs and for codepoints the general match
  returns nothing for). Porting that dedicated `discoverFallback` (the
  `CTFontCreateForString` codepoint search) is a **separate later experiment**;
  this experiment lands the resolver wiring over the candidates the general
  discover surfaces. The fallback therefore resolves codepoints whose covering
  font the general match finds (e.g. emoji), and may miss some the dedicated
  `discoverFallback` would reach — a documented, scoped limitation, not a
  divergence in the wiring.
- **Faithful deviation**: the fallback descriptor's **size** is left unspecified
  (`0.0`) — roastty's collection does not thread a points size, and the size is
  a discovery hint, not a codepoint filter. Noted.
- **Deferred**: the dedicated `discoverFallback`/`discoverCodepoint`
  (`CTFontCreateForString`) candidate search, codepoint overrides
  (`getIndexCodepointOverride` + the `descriptor_cache`), the variation-axis
  score, and variations application.
- No C ABI/header/ABI-inventory change (the resolver/collection are internal
  Rust).

## Changes

1. `roastty/src/font/codepoint_resolver.rs`: `get_index` → `&mut self`; add
   `discover_enabled` + `set_discover_enabled`; add the discovery-fallback block
   and `fallback_face_has_codepoint`; update the recursive calls and the test
   call sites (`let mut r`). Import `Descriptor`, `SizeAdjustment`.
2. Tests (in `codepoint_resolver.rs`):
   - `discovery_fallback_finds_emoji`: a `menlo_resolver()` (Menlo only) with
     `set_discover_enabled(true)`. `get_index(0x1F600, Regular, Some(Emoji))`
     returns `Some(idx)` (discovery finds Apple Color Emoji, which has the glyph
     as color), and the collection grew by one face; a **second** identical call
     returns a face index **without** growing the collection again (the added
     fallback now satisfies the lookup).
   - `discovery_fallback_disabled`: the same resolver **without**
     `set_discover_enabled` returns `None` for `0x1F600` (Menlo lacks it and the
     last-resort finds nothing).
   - `fallback_presentation_check`: `fallback_face_has_codepoint` on the emoji
     face returns `true` for `Emoji` and `false` for `Text` at `0x1F600` (color
     glyph), and `false` for a codepoint the face lacks.
   - The existing `get_index_*` tests still pass (now with `let mut r`).
3. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `get_index` performs the regular-style discovery fallback (when enabled),
  checks the presentation, and adds the matched face to the collection, faithful
  to upstream;
- the finds-emoji, disabled, presentation-check, and existing resolver tests
  pass;
- codepoint overrides, the variation-axis score, and variations application stay
  deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment is **partial** if the system has no font covering the test
codepoint (the fallback path is still exercised and the disabled/None path
proven).

The experiment **fails** if the fallback search, the presentation check, or the
collection insertion diverges from upstream, or any public C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and raised **one Required
finding**: the original claim that `discover_faces()` faithfully substitutes for
upstream's `discoverFallback()` was too strong — `Score::codepoint` only ranks
the candidates the **general** CoreText match returns; it does not add the extra
candidates upstream's `discoverFallback` produces via
`discoverCodepoint`/`CTFontCreateForString` (notably CJK unified ideographs and
codepoints the general match returns nothing for). Fixed: the design now scopes
this experiment as the resolver-fallback **structure** over the **general**
discovery, explicitly **not** a faithful substitute for `discoverFallback` — the
dedicated `CTFontCreateForString` codepoint search is called out as a separate
later experiment, and the limitation (some codepoints the dedicated fallback
would reach may be missed) is documented as a scoped, intentional gap rather
than a wiring divergence.

Codex confirmed the rest is faithful: the fallback's placement (after the
non-regular retry, before the final regular `Any`) matches upstream `getIndex`
order; `fallback_face_has_codepoint` matches a **fallback**
`Entry::has_codepoint` (`Default(p)` → explicit presentation matching, `Any` →
presence only); the `&mut self` change is correct (a fallback hit mutates the
collection); the no-infinite-growth reasoning holds (the added fallback is found
by the exact collection lookup before discovery runs again); and `bold`/`italic`
false with `monospace = false` are correct for the regular fallback descriptor.

Review artifacts:

- Prompt: `logs/codex-review/20260603-121840-072696-prompt.md`
- Result: `logs/codex-review/20260603-121840-072696-last-message.md`

## Result

**Result:** Pass

The resolver's discovery-based fallback lands — discovery is now wired into
`get_index`.

- `roastty/src/font/codepoint_resolver.rs`: `get_index` is now `&mut self`; the
  resolver gained a `discover_enabled` toggle (`set_discover_enabled`, default
  off); after the non-regular retry and before the last-resort `Any`, a
  regular-style discovery-enabled lookup builds a
  `Descriptor { codepoint: cp, monospace: false, .. }`, iterates
  `discover_faces()`, and adds the first face that has the codepoint in the
  requested presentation (`fallback_face_has_codepoint`) to the collection as a
  `Regular` fallback (`SizeAdjustment::IcWidth`), returning its index.
  `fallback_face_has_codepoint` replicates a fallback `Entry::has_codepoint`
  (`Default → Explicit` presentation matching; `Any` → presence). The
  module/`get_index` docs were updated.
- `roastty/src/font/collection.rs`: a `face_count(style)` accessor.

Tests: `discovery_fallback_finds_emoji` (a Menlo-only resolver with discovery
enabled resolves `0x1F600` (emoji) by discovering a color font, grows the
collection by one fallback, and a second lookup returns the **same** index
without growing again), `discovery_fallback_disabled` (no discovery ⇒ `None`),
`fallback_presentation_check` (the emoji color glyph matches `Emoji` not `Text`;
`Any` matches presence; a CJK codepoint the emoji font lacks never matches). The
existing `get_index_*` tests still pass (now `let mut r`).

Gate results:

- `cargo fmt -p roastty` accepted; `--check` clean.
- `cargo test -p roastty` → 2724 passed, 0 failed (+3, no regressions).
- `cargo build -p roastty` → no warnings.
- No-`ghostty`-name gates clean; `git diff --check` clean.

## Conclusion

The resolver now **uses** discovery: a regular-style codepoint no loaded face
covers triggers a system search, and the matching font is added to the
collection and resolved — the integration that makes the whole discovery
pipeline (Experiments 325–332) useful to the rest of the font system. The
`&mut self` change reflects that resolution can now grow the collection.

The remaining font-resolution work is: the dedicated **`discoverFallback`/
`discoverCodepoint`** (`CTFontCreateForString`) candidate search (to reach
codepoints — notably CJK — the general match misses), **codepoint overrides**
(`getIndexCodepointOverride` + the `descriptor_cache`, which also depend on
discovery), the variation-axis score refinement, and applying variations to
produced faces. Beyond resolution: the **shaper** (shape calls, glyph placement)
and the remaining font subsystems.

## Completion Review

Codex reviewed the completed implementation and result and **approved** with
**no Required findings**. It confirmed `get_index(&mut self)` is warranted (a
discovery hit mutates the collection); the fallback block is in the upstream
position (after the exact lookup and non-regular retry, before the final regular
`Any`); the discovered face is presentation-checked before insertion, added as a
`Regular` fallback with `SizeAdjustment::IcWidth`, and returned immediately;
`fallback_face_has_codepoint` matches a fallback `Entry::has_codepoint`
(`Default(p)` like `Explicit(p)`, `Any` presence-only); the second lookup does
not grow the collection (the newly added fallback is found by the exact
`collection.get_index` before discovery runs again); the `discover_faces`
limitation is now correctly documented as resolver wiring over general discovery
(dedicated `discoverFallback`/`CTFontCreateForString` deferred); and there is
**no borrow/lifetime issue** (the discovery iterator does not borrow
`self.collection`, and the temporary borrow of `face` ends before it is moved
into `add_with_adjustment`). No Optional findings.

Review artifacts:

- Result review: `logs/codex-review/20260603-122416-937166-last-message.md`
