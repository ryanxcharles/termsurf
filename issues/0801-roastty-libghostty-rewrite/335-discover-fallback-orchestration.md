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

# Experiment 335: the discoverFallback orchestration

## Description

`font_for_codepoint` (Experiment 334) can find a font for a specific codepoint
via `CTFontCreateForString`, but nothing calls it yet. Upstream's
`discoverFallback` orchestrates **when** to use it: for CJK unified ideographs
it goes straight to the codepoint search (CoreText handles the locale);
otherwise it uses the general `discover`, falling back to the codepoint search
only when the general match is empty. This experiment ports that orchestration
and **rewires the resolver's discovery fallback** to use it — so CJK and other
hard-to-match codepoints now resolve.

## Upstream behavior (`discovery.zig` `discoverFallback`)

```zig
fn discoverFallback(self, alloc, collection, desc) !DiscoverIterator {
    // CJK unified ideographs: CoreText's locale-aware codepoint search.
    if (desc.codepoint >= 0x4E00 and desc.codepoint <= 0x9FFF) {
        const han = (try self.discoverCodepoint(collection, desc)) orelse break;
        return DiscoverIterator{ .list = &.{han}, … };   // 0-or-1 result
    }

    const it = try self.discover(alloc, desc);

    // If the general discovery found nothing and we have a codepoint, fall back
    // to CTFontCreateForString.
    if (it.list.len == 0 and desc.codepoint > 0) {
        const ct_desc = (try self.discoverCodepoint(collection, desc)) orelse break;
        return DiscoverIterator{ .list = &.{ct_desc}, … };
    }

    return it;
}
```

`discoverCodepoint` picks an **original font** by the requested style
(bold_italic → bold → italic → regular) and runs `font_for_codepoint` on it (the
Experiment-334 call).

## Rust mapping

- `roastty/src/font/discovery.rs`:
  `pub(crate) fn discover_fallback_faces(&self, original: &Face) -> Vec<Face>`:
  ```rust
  // CJK unified ideographs → the locale-aware codepoint search. Only return
  // early when it succeeds; on `None` fall through to the general path
  // (upstream's `orelse break :han`).
  if (0x4E00..=0x9FFF).contains(&self.codepoint) {
      if let Some(face) = original.font_for_codepoint(self.codepoint) {
          return vec![face];
      }
  }
  // General discovery; fall back to the codepoint search if it found nothing.
  let descriptors = self.discover_descriptors();
  if descriptors.is_empty() && self.codepoint > 0 {
      return original.font_for_codepoint(self.codepoint).into_iter().collect();
  }
  descriptors.into_iter().map(deferred_face).collect()
  ```
  The caller supplies the **original** face (the codepoint search needs a source
  `CTFont`). `font_for_codepoint` yields 0-or-1 face; the general path yields
  the matched faces.
- `roastty/src/font/codepoint_resolver.rs`: in `get_index`'s discovery fallback,
  fetch the original (regular primary) face and use `discover_fallback_faces`
  instead of the plain `discover_faces`:
  ```rust
  let faces = match self.collection.get_face(Index::new(Style::Regular, 0)) {
      Ok(original) => req.discover_fallback_faces(original),
      Err(_) => Vec::new(),
  };
  for face in faces {
      if fallback_face_has_codepoint(&face, cp, p_mode) { /* add + return */ }
  }
  ```
  The `match` ends the immutable borrow of the original face before the loop's
  mutable `add_with_adjustment` (NLL).

## Scope / faithfulness notes

- **Ported**: `discoverFallback`'s orchestration — the CJK `0x4E00..=0x9FFF`
  gate, the general-discovery path, and the empty-match codepoint fallback — and
  its use as the resolver's discovery fallback (CJK and previously-unmatched
  codepoints now resolve).
- **Faithful deviations**:
  - The **original-font style selection** (bold_italic → bold → italic →
    regular) is collapsed to the **regular primary** face — the resolver's
    discovery fallback only runs for **regular** style (a non-regular request
    already recursed to regular), so the original is always the regular face
    here. The full multi-style selection matters only for a direct bold/italic
    `discoverFallback` call, which the resolver does not make. Noted.
  - `discover_fallback_faces` returns an eager `Vec<Face>` rather than a lazy
    iterator: the CJK/empty cases are a single face, and the general fallback
    match list (for a codepoint the primary lacks) is small, so the eager
    creation is acceptable. Noted (upstream's `DiscoverIterator` is lazy).
- **Deferred**: codepoint overrides, the variation-axis score, and variations
  application.
- No C ABI/header/ABI-inventory change (internal Rust).

## Changes

1. `roastty/src/font/discovery.rs`: add `Descriptor::discover_fallback_faces`.
2. `roastty/src/font/codepoint_resolver.rs`: rewire the discovery fallback to
   use `discover_fallback_faces` over the regular primary face.
3. Tests:
   - `discovery.rs` `discover_fallback_cjk`:
     `Descriptor { codepoint: 0x4E00, .. } .discover_fallback_faces(&Face::new("Menlo", 24.0))`
     returns a non-empty `Vec`, and its first face renders `0x4E00` (the CJK
     gate → `font_for_codepoint`).
   - `discovery.rs` `discover_fallback_general`:
     `Descriptor { family: Some("Menlo"), .. }.discover_fallback_faces(&menlo)`
     returns a non-empty `Vec` (the general path; a family that matches).
   - `codepoint_resolver.rs` `discovery_fallback_resolves_cjk`: a Menlo-only
     resolver with discovery enabled now resolves
     `get_index(0x4E00, Regular, Some(Text))` to `Some(idx)` (the CJK gate finds
     a Han font and adds it). The existing `discovery_fallback_finds_emoji` (a
     non-CJK codepoint via the general path) still passes.
4. Format and test (`cargo fmt`, accept output).

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

- `discover_fallback_faces` reproduces `discoverFallback`'s CJK gate and
  empty-match codepoint fallback, and the resolver uses it (CJK now resolves);
- the CJK, general, and resolver-CJK tests pass, and the existing
  discovery/resolver tests still pass;
- the multi-style original selection, codepoint overrides, the variation-axis
  score, and variations stay deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment is **partial** if the host lacks a CJK font (the orchestration is
still exercised and the general path proven).

The experiment **fails** if the CJK gate, the empty-match fallback, or the
resolver rewire diverges from upstream, or any public C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and raised **one Required
finding**: the original CJK branch returned
`font_for_codepoint(..).into_iter(). collect()` unconditionally, so on a `None`
result it would return an **empty** `Vec` rather than **falling through** to the
general path. Upstream's CJK block is
`han: { const han = discoverCodepoint(..) orelse break :han; return …; }` — it
only returns early when the codepoint search **succeeds**; on `null` it breaks
out and continues to the general `discover` (and then the empty-match codepoint
fallback). Fixed: the CJK branch now returns early **only** inside
`if let Some(face) = original.font_for_codepoint(..)`, otherwise falling through
to the general discovery path.

Codex confirmed the rest is sound: collapsing the original-font selection to the
regular primary is faithful for the resolver's regular-only fallback call site;
the owned `Vec<Face>` ends the immutable borrow before the collection mutation;
the eager `Vec` (vs a lazy iterator) is acceptable for this scoped fallback; and
the non-CJK emoji/general path is preserved by the rewire.

Review artifacts:

- Prompt: `logs/codex-review/20260603-123438-240951-prompt.md`
- Result: `logs/codex-review/20260603-123438-240951-last-message.md`
