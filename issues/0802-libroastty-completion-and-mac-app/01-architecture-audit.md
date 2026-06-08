+++
[implementer]
agent = "claude-code"
model = "claude-opus-4-8"
reasoning = "high"

[method]
agent = "claude-code"
model = "claude-opus-4-8"
subaudits = 7
+++

# Experiment 1: Architecture audit — what's done, what remains, and the order to finish

## Description

The seed for Issue 802: a comprehensive, evidence-based audit of `libroastty`
(Rust) against `libghostty` (Zig, vendored at `vendor/ghostty/src`), to
determine which subsystems are **Complete / Partial / Missing**, and to
recommend the order in which to finish the port to 100% — with the end goal of
running a copied, `ghostty→roastty`-renamed macOS app on `libroastty` as the
conformance test.

## Method

Seven independent, parallel subsystem audits (one per area), each required to
base findings on actual file reads/greps, to grep for _functionality_ rather
than filenames (roastty folds many Zig files into fewer, larger Rust files —
`lib.rs` alone is ~39.5k lines), and to say "not found" rather than guess:

1. Terminal core (`terminal/`, `termio`, `unicode/`, `terminfo/`, `pty`)
2. Renderer (`renderer/`, the Metal stack)
3. Font & text (`font/`)
4. Input (`input/`)
5. Configuration (`config/`)
6. The **embedded app-facing C ABI** (`apprt/embedded.zig`, `App`, `Surface`,
   `main_c`, `ghostty.h`, the runtime-callback + action surface) — the crux
7. Supporting subsystems & dependencies (`os/`, `datastruct/`, `simd/`,
   `crash/`, `inspector/`, `Command`, `pty`, `quirks`, `shell-integration`,
   vendored deps)

## Findings — subsystem status

### Terminal core — **largely complete**, one dominant gap

Complete and substantive (full ports, not stubs): `page` / `PageList` (20k LOC)
/ `Screen` / `Terminal` control flow, the full VT parser + CSI/DCS/OSC/APC state
machine (`stream.rs`, 8.6k LOC), SGR, styles, colors (named/SGR/x11),
selection + gestures, Kitty graphics + keyboard, hyperlinks, scrollback search
(all 6 files), StringMap, ScreenSet, **tmux control mode** (5k LOC), PTY, and
the formatter (plain/VT/HTML). `datastruct/` is fully ported (all 9 structures,
folded into `terminal/`).

- **Partial — `Terminal::print()`:** writes **narrow, single-codepoint cells
  only**. The page/cell layer fully models `Wide`/`SpacerTail`/`SpacerHead` +
  grapheme bytes, but the print path never sets them
  (`set_wide`/`append_grapheme_at` have no production callers). → no CJK/emoji
  wide chars, no grapheme clustering, no mode-2027 handling.
- **Missing — `unicode/`:** no grapheme-break tables, no codepoint-width table,
  no symbol/Nerd-Font width tables, no crate. This is the _cause_ of the
  `print()` gap, and the highest-impact terminal item.
- **Partial — shell-integration injection** (`termio/shell_integration.zig`):
  only a config enum exists; no script/env injection.
- **Out of scope / resource:** terminfo source/compilation.

### Renderer — **offscreen-complete; no live on-screen path**

The entire CPU rebuild stack and the Metal backend are ported and densely
tested: `generic.zig`'s `updateFrame` (frame rebuild, dirty tracking, atlas
upload, uniforms, cursor/preedit/selection/padding) → `frame_rebuild.rs` /
`cell.rs` / `frame_renderer.rs`; the Metal `metal/` subdir
(api/buffer/frame/pipeline/
render_pass/sampler/target/texture/iosurface_layer/compositor); shaders + all 5
pipelines. roastty can rebuild a frame and present it to an **offscreen
IOSurface-backed texture**. OpenGL/WebGL are correctly N/A (Metal only).

What's missing is everything that makes it a _running_ renderer:

- **Missing — the live render loop / on-screen target.** `FrameRenderer` /
  `MetalFrameCompositor` have **no callers outside the renderer module +
  tests**. No `NSView`/`CALayer` attachment (the layer is built but never
  attached to a host view), no `surface.draw()` consumer.
- **Missing — `Thread.zig`:** no render thread, no 120 FPS pacing, no
  cursor-blink timer, no draw coalescing.
- **Missing — renderer mailbox (`message.zig`) + `Options.zig`:** no
  focus/visible/occlusion/change-config plumbing.
- **Partial — the live draw pass is cells-only:** images (Kitty graphics + bg
  image) and custom shaders have **state + pipelines ported but are never
  invoked** in a presented frame.
- **Missing — link-highlight matcher** (`renderer/link.zig` `renderCellMap`):
  the viewport regex/hover matcher is absent, so `link_ranges` is always fed
  empty.
- **Missing — debug `Overlay.zig`** (minor).

### Font & text — **foundations complete; config→font assembly is the gap**

Complete: Metrics, Atlas, Glyph, sprite Canvas + rasterizer, CoreText Face
rasterization, the CoreText shaper (folded onto the Face) + run iterator +
cache + feature parsing, Collection / CodepointResolver / CodepointMap /
DeferredFace / SharedGrid, all 7 OpenType tables, embedded fonts,
`nerd_font_attributes`, discovery. FreeType/Fontconfig/HarfBuzz/WASM correctly
N/A.

- **Partial — `SharedGridSet` config→font assembly (largest gap):** only the
  generic ref-counted container is ported. The `Key`/`DerivedConfig` derivation
  that turns user font config (`font-family`/`font-style`/`font-feature`/
  codepoint-map/size/variations) into a populated `Collection` (via discovery,
  primary + fallback faces) is **not wired** — every real grid construction is a
  test helper hardcoding "Menlo". → fonts not end-to-end from config.
- **Partial — sprite legacy-computing coverage:** Smooth Mosaics + the rest of
  Symbols for Legacy Computing (U+1FB3C–1FBEF) missing; **branch glyphs**
  (U+F5D0–F5E3) entirely missing; minor legacy-supplement quadrant variants.

### Input — **encoding-complete; keybinding system partial**

Complete: VT/Kitty key **encoding** (the core, heavily tested), Kitty
functional-key table, PC-style function keys, key codes/events, mouse structs +
mouse encoding, bracketed paste, the clipboard-request flow.

- **Partial — keybinding system (`Binding.zig`, 4.9k LOC):** single-trigger
  parsing + ~60-action verb parser + configured/default dispatch exist.
  **Missing:** multi-key sequences/chords (the trie), leader keys, key tables,
  trigger-prefix flags (`global:`/`all:`/`unconsumed:`/`performable:`), reverse
  action→trigger mapping, the default-bindings data table, and ~1/3 of the
  action set.
- **Missing — native keymaps:** `keycodes.zig` (OS-keycode→Key) and
  `KeymapDarwin.zig` (Carbon `UCKeyTranslate` + dead keys). (Architecturally
  offloaded to the GUI today; needed if the app expects roastty to translate.)
- **Partial/Missing:** `RemapSet`/`Mask` (mod-remap config); command-palette
  catalog (`command.zig`); `Link.oniRegex` (URL-regex compilation).

### Configuration — **engine complete; ~⅓ of the option surface**

Complete: the load pipeline — file loading, the per-line iterator, CLI-arg
application, default-file discovery/ordering, recursive `config-file` include
with cycle detection, diagnostics, and the formatter/export — all faithful for
the fields that exist.

- **Partial — option coverage: ~63 of ~201 options (~31%).** Whole families
  absent: all font options (`font-family`/`font-size`/`adjust-*`), `palette`,
  `link`, `command`, cursor/mouse styling, scrollback, most
  `macos-*`/window-inherit. Unknown keys → recorded diagnostics (does not
  crash).
- **Partial — `finalize()` is a stub:** ghostty's ~199-line cross-field
  validation/derivation/clamping is not ported (only working-dir tilde
  expansion).
- **Missing — theme loading:** the `theme` value parses, but no themes-dir
  locator, no theme-file read, no palette application.
- **Partial/unwired — conditionals** (`conditional.rs` exists but nothing calls
  it) and **keybind parsing** (no sequences/prefixes); **`font-codepoint-map`
  absent**.

### Embedded app-facing C ABI — **mostly faithful; the render contract diverges**

Stronger than expected: **63 of 71** embedded exports present by name; the `app`
object model, the `ghostty_runtime_config_s` callback struct (all 8 fields, all
invoked), action dispatch (the tagged-union-over-C shape, ~45 of ~65 actions),
and surface lifecycle/input/selection/split/inspector-core/QuickLook are
**faithful**, with extern struct layouts deliberately mirrored.

- **DIVERGENT — `surface_draw` rendering (the crux).** ghostty draws Metal
  directly into the app-provided `NSView` via `ghostty_surface_draw`; roastty's
  `roastty_surface_draw` only sets a dirty flag + `wakeup`, **silently ignores
  the `nsview` pointer**, and its renderer is **not connected to the C ABI at
  all**. roastty instead exposes a `roastty_render_state_*` **pull** model (the
  embedder draws) — which the unmodified ghostty app does not use. An unmodified
  app would present an empty view.
- **Missing — 7 exports:** `app_key`, `app_keyboard_changed`, `app_open_config`,
  `inspector_metal_init`/`render`/`shutdown`, `set_window_background_blur`; plus
  `ghostty_translate`/`cli_try_action`/`benchmark_cli`. ~1/3 of the action set.
- Note: `roastty/ABI_INVENTORY.md` **overstates** coverage (lists
  `inspector_metal_*` as mapped though unimplemented) — trust the actual
  `lib.rs` exports, not the inventory.

### Supporting subsystems & dependencies — **mostly complete or correctly excluded**

Complete: all macOS-relevant `os/` utilities (1:1, 22 files), `Command` + `pty`
(folded into `os/pty.rs`), the UTF-8 DFA decoder, `crash/`'s local dir +
envelope half, `surface_mouse`. Dependencies satisfied idiomatically — zlib via
`flate2`, macOS frameworks via `objc2*`, and **stb_image deliberately replaced**
by an embedder PNG-decode callback (`sys_decode_png`) + native raw/zlib.

- **Missing — `unicode/` width/grapheme tables** (same as the terminal gap).
- **Partial — `crash/sentry.zig`** (local-only; no Sentry init/capture);
  **`termio/`** (synchronous; no mailbox/message layer + no shell-integration
  injection); **SIMD** (scalar-only by design — base64/VT/index-of/width;
  `BUILD_INFO_SIMD=false`).
- **Missing (minor/perf):** `os/cf_release_thread.zig`.
- **Out of scope (library, not app):** the `inspector/` ImGui UI (only the C-API
  input-forwarding shim is kept), `cli/`, `shell-integration/` scripts,
  `terminfo/` compilation, `synthetic/`, `quirks` (a no-op upstream),
  Linux/Windows/wasm `os/`.

## The gaps that block "100% finish" (prioritized)

1. **Live render path** — grow `surface_draw` to own a Metal renderer bound to
   the app's `NSView`/`CALayer`, attach + present on-screen; add the render
   thread (pacing + cursor-blink) and the renderer mailbox/Options; retire the
   `render_state` pull divergence. _(The single thing that makes the app show a
   terminal.)_
2. **Embedded ABI completion** — the 7 missing exports + `ghostty_translate`/
   `cli_try_action` + the remaining ~⅓ of the action set; struct-layout fidelity
   against the pinned ghostty version.
3. **Unicode width + grapheme clustering** — port `unicode/` tables and rewrite
   the `print()` path (width lookup + grapheme accumulation; mode 2027).
4. **Config completeness** — the remaining ~140 options, `finalize()`, theme
   loading, conditionals, `font-codepoint-map`, **and the `SharedGridSet`
   config→font assembly** (so the app's font/color config actually applies).
5. **Keybinding system** — sequences/chords/key-tables/leader-keys, trigger
   prefixes, the full action set + default-bindings table, command-palette
   catalog; native keymaps + `RemapSet` if the app expects roastty-side
   translation.
6. **Renderer feature-completion** — invoke the ported image (Kitty + bg) and
   custom-shader stages in the live pass; the link-highlight matcher; debug
   overlay.
7. **Smaller / polish** — shell-integration injection, sprite legacy-computing +
   branch glyphs, Sentry capture, SIMD (perf), `cf_release_thread` (perf),
   terminfo resource.

## Recommended order — get the app running, then fill features behind the conformance test

The strategy: reach a **launchable, drawing app as fast as possible**, then fill
every remaining feature _behind the running app + automated UI tests_, so each
gap is found and fixed against the real conformance oracle.

- **Phase A — Stand up the app shell + ABI link.** Pin a ghostty version; copy
  the macOS app; find/replace `ghostty→roastty`; point it at `libroastty` +
  `roastty.h`. Make it **link** — add the missing embedded exports and match
  struct layouts to the pinned version. _(Linker errors enumerate the exact ABI
  gaps, making Gap #2 demand-driven.)_ Outcome: the app builds and launches
  (likely a blank view).
- **Phase B — The live render path (Gap #1, the crux).** Wire `surface_draw` to
  the offscreen renderer bound to the app's `NSView`; add the render thread +
  mailbox. Outcome: **the app shows a working ASCII terminal you can type into**
  — the conformance test goes live.
- **Phase C — Automated UI-test harness (workstream 3).** Stand up macOS
  UI-automation + screenshots now, so every later change is regression-tested.
- **Phase D — Terminal correctness:** Unicode width + grapheme (Gap #3) — wide
  chars/emoji now visible in the app.
- **Phase E — Config completeness (Gap #4)** incl. `SharedGridSet` font
  assembly + `finalize` + themes — so the app's real config (fonts, colors)
  applies.
- **Phase F — Input / keybindings (Gap #5)** — sequences, key tables, full
  actions, command palette.
- **Phase G — Renderer feature-completion (Gap #6)** — images, custom shaders,
  link highlighting in the live pass.
- **Phase H — Remaining/polish (Gap #7).**

Workstream 3 (UI tests + screenshots) runs continuously from Phase C onward;
each feature gets a UI test and anything broken is fixed in `libroastty` (the
app stays unaltered except for the rename).

## Result

**Result:** Pass (audit complete).

Seven independent evidence-based subsystem audits produced a consistent picture:
**libroastty is a real, tested terminal core + offscreen renderer with a mostly
faithful embedded ABI, blocked from being a _running_ terminal by a small number
of large, well-identified gaps** — chiefly the live on-screen render path (the
`surface_draw`→NSView contract), Unicode width/grapheme, the second two-thirds
of the config option surface (+ font assembly), and the keybinding system. The
out-of-scope set (OpenGL/WebGL, non-CoreText font backends, Linux/Windows/wasm,
inspector UI, CLI, shell scripts, terminfo compilation) is correctly excluded
for an embeddable library.

## Conclusion

The port is **further along than its file counts suggest** — the hard terminal,
renderer, font, and ABI foundations are done; what remains is concentrated
integration work with a clear critical path. The recommended order front-loads a
**launchable, drawing app** (Phases A–B) precisely so the rest can be finished
behind the conformance oracle.

**Next experiment (Experiment 2):** begin Phase A — pin a ghostty version, copy
the macOS app into the roastty project, apply the `ghostty→roastty` rename,
point it at `libroastty`, and drive it to **link** — recording the exact set of
missing/mismatched ABI symbols the app requires (which becomes the concrete
worklist for the embedded-ABI completion).
