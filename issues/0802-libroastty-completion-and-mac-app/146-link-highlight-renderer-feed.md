# Experiment 146: Phase H — link-highlight renderer feed

## Description

Port the renderer-side link-highlight matcher and feed live `link_ranges` into
the cell rebuild path.

Roastty already has the link config surface (`link-url`, default URL link,
`link-previews`), the input-side `Link` shape, terminal hyperlink storage, a
`StringMap` helper for byte-to-cell regex matches, and renderer cell code that
underlines any supplied `link_ranges`. The missing Phase H piece is the live
matcher/feed: `FrameRenderState::from_terminal` currently leaves `link_ranges`
empty, so detected links and OSC8 hover links never affect the presented frame.

Upstream Ghostty's renderer performs this in two layers:

1. seed the link cell set from OSC8 hyperlinks only when the mouse hovers a
   hyperlink with `ctrlOrSuper` modifiers;
2. run configured regex links over a flattened render-state string and add
   matches whose highlight predicate is active (`always`, `always_mods`,
   `hover`, or `hover_mods`).

This experiment ports that behavior into Roastty's live Rust renderer using a
Rust-side look-around-capable regex engine and existing surface mouse state.

Out of scope:

- click/open behavior for detected regex links or OSC8 links;
- link preview popovers/tooltips;
- new config parsing for arbitrary `link = ...` entries beyond the existing
  default/config structures;
- debug overlay work.

## Changes

- `roastty/src/renderer/link.rs` or adjacent renderer module
  - Add a renderer link matcher equivalent to upstream `renderer/link.zig`
    `renderCellMap`.
  - Use a look-around-capable regex engine for configured `input::link::Link`
    patterns. The default upstream URL/path matcher contains negative
    look-behind, so `regex::bytes::Regex` is not sufficient. Prefer a focused
    `fancy-regex` dependency (already used elsewhere in the workspace) unless
    implementation finds an Oniguruma-compatible crate is a better fit.
  - Flatten the current terminal viewport to a string plus per-byte viewport
    coordinates through the existing terminal `StringMap`/page-string machinery,
    without trimming or breaking soft-wrap unwrapping. Regex match offsets must
    continue to map back to cell coordinates, including multi-byte UTF-8 text
    around a match.
  - Evaluate all four highlight predicates against the current mouse viewport
    coordinate and modifiers:
    - `Always` always matches;
    - `AlwaysMods` requires exact modifier equality;
    - `Hover` requires the mouse to be inside the match;
    - `HoverMods` requires both hover and exact modifier equality.
  - Convert matched cells into per-row inclusive `[start, end]` column ranges
    suitable for `rebuild_row`'s existing underline override.
  - Merge contiguous cells on the same row so renderer input stays compact and
    deterministic.
  - Compile a reusable renderer link set when the effective configured link list
    changes, matching upstream's derived renderer link set instead of compiling
    patterns every frame.
  - Treat invalid configured regexes as non-fatal at link-set rebuild time:
    skip/log the bad pattern and keep rendering with the valid subset. This
    intentionally differs from upstream's Zig init error propagation because
    Roastty's current live config reload path is already non-fatal; record the
    divergence in the result if it remains the implemented behavior.
- `roastty/src/terminal/...`
  - Expose only the minimum crate-private terminal helper needed by the renderer
    to build the viewport string map and/or to query OSC8 hyperlink cells.
  - Preserve existing terminal ABI and C-facing APIs.
- `roastty/src/lib.rs`
  - Thread finalized app config link definitions and the surface's current mouse
    viewport/modifier state into live presentation.
  - Derive mouse viewport from `SurfaceMouseState::position` and current cell
    size; `None` when outside the viewport or cell size is unavailable.
  - Seed OSC8 hover link ranges when the mouse is over an OSC8 hyperlink and
    `ctrlOrSuper` is held, matching upstream's default OSC8 hover policy.
- `roastty/src/renderer/frame_renderer.rs`
  - Replace the live frame state's empty `link_ranges` placeholder with the
    computed ranges.
  - Keep search `highlights` unchanged; search-highlight work is not part of
    this slice.
- `issues/0802-libroastty-completion-and-mac-app/README.md`
  - Link this experiment as `Designed`.
  - After the result, mark Phase H's link-highlight matcher/feed complete if
    tests prove regex and OSC8 hover ranges reach the live cell rebuild path.

## Verification

- Format markdown:
  - `prettier --write --prose-wrap always --print-width 80 issues/0802-libroastty-completion-and-mac-app/146-link-highlight-renderer-feed.md issues/0802-libroastty-completion-and-mac-app/README.md`
- Format Rust:
  - `cargo fmt`
- Run focused link and renderer tests:
  - `cargo test -p roastty renderer::link -- --test-threads=1`
  - `cargo test -p roastty link_ranges -- --test-threads=1`
  - `cargo test -p roastty default_url_link -- --test-threads=1`
  - `cargo test -p roastty frame_renderer -- --test-threads=1`
- Run ABI harness:
  - `cargo test -p roastty --test abi_harness`
- Run full Roastty Rust coverage:
  - `cargo test -p roastty -- --test-threads=1`
- Run hosted app coverage:
  - `cd roastty && macos/build.nu --action test`
- Run hygiene checks:
  - `cargo fmt --check`
  - `git diff --check`
  - `prettier --check --prose-wrap always --print-width 80 issues/0802-libroastty-completion-and-mac-app/146-link-highlight-renderer-feed.md issues/0802-libroastty-completion-and-mac-app/README.md`

**Pass** = live frame construction computes nonempty `link_ranges` for the
default `link-url` URL/path matcher and for configured regex links whose
highlight predicate is active, including hover and modifier-gated variants; OSC8
hyperlink cells are underlined only for the upstream `ctrlOrSuper` hover case;
invalid regex config does not break terminal rendering; focused/full/hosted
checks pass; and the Phase H checklist can mark link highlighting complete.

**Partial** = the matcher computes correct ranges in isolation, but live surface
mouse/config threading or OSC8 hover seeding needs a follow-up.

**Fail** = Roastty lacks a reliable viewport string/pin map or hyperlink query
primitive needed to match upstream behavior without a broader terminal-state
refactor.

## Design Review

**Reviewer:** Codex-native adversarial review subagent `Boyle`, fresh context.

**Initial verdict:** Changes required.

**Findings and fixes:**

- **Required:** the initial plan used `regex::bytes::Regex`, which cannot
  compile Roastty's default upstream URL/path regex because it contains negative
  look-behind. Fixed by requiring a look-around-capable Rust-side regex engine,
  explicitly rejecting `regex::bytes::Regex` for this matcher, and adding
  default `link-url` verification and pass criteria.
- **Optional:** invalid-regex handling differed from upstream's renderer link
  set initialization error propagation. Accepted as an intentional live-reload
  divergence for now, documented as compile-at-link-set-rebuild with non-fatal
  skip/log behavior, and required to be recorded in the result if implemented.
- **Nit:** stale wording said "existing Rust regex engine." Fixed to "Rust-side
  look-around-capable regex engine."

**Final verdict:** Approved.

## Result

**Result:** Pass

Implemented the live link-highlight renderer feed and wired it into Roastty's
frame presentation path.

The renderer now maintains a reusable compiled link set from finalized app
config, flattens the visible terminal viewport to a byte-to-cell map, evaluates
configured link regexes against that map, applies upstream-style `always`,
`always_mods`, `hover`, and `hover_mods` highlight predicates, and feeds merged
per-row inclusive ranges into the existing cell rebuild underline override. Both
normal live frames and custom-shader live frames receive the computed
`link_ranges`.

OSC8 hyperlink hover underlining is also seeded from terminal hyperlink metadata
when the mouse is over a linked cell and `ctrlOrSuper` is held, matching
upstream's default OSC8 hover policy.

The implementation uses the `onig` crate for the renderer link matcher.
`fancy-regex` was tried first, but it rejected the default upstream URL/path
regex with a variable-width look-behind error. `onig` compiles and matches the
default `link-url` pattern, so this slice uses an Oniguruma-compatible regex
engine for renderer link detection.

Invalid configured regexes are skipped and logged at live link-set rebuild time
instead of aborting renderer initialization. That intentionally differs from
upstream Ghostty's Zig init error propagation because Roastty's current live
config reload path is non-fatal; the valid subset continues rendering.

Verification completed:

- `cargo fmt`
- `cargo test -p roastty renderer::link -- --test-threads=1` — 6 passed
- `cargo test -p roastty link_ranges -- --test-threads=1` — 1 passed
- `cargo test -p roastty default_url_link -- --test-threads=1` — 1 passed
- `cargo test -p roastty frame_renderer -- --test-threads=1` — 29 passed
- `cargo test -p roastty --test abi_harness` — 1 passed, with existing C enum
  conversion warnings
- `cargo fmt --check`
- `cargo test -p roastty -- --test-threads=1` — 4800 unit tests plus ABI harness
  and doc tests passed, with existing C enum conversion warnings
- `cd roastty && macos/build.nu --action test` — 210 hosted macOS tests passed
  (`TEST SUCCEEDED`), with existing SwiftLint, main-actor/pasteboard,
  main-thread-checker, macOS-version link, App Intents, and missing testing
  config path warnings/noise

## Conclusion

Phase H's link-highlight matcher/feed is complete. Roastty now computes live
renderer link ranges for the default URL matcher, configured regex links,
hover/modifier-gated predicates, and OSC8 hover links, then presents those
ranges through the existing renderer underline path without changing the C ABI.

## Completion Review

**Reviewer:** Codex-native adversarial review subagent `Ohm`, fresh context.

**Verdict:** Approved.

**Findings:** None.
