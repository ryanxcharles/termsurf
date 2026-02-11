# Issue 410: Apply a Partial Electron Patch Set

## Goal

Fix the 2-3fps throttling problem from Issue 407. Apply only the Electron
patches that TermSurf actually needs to our Chromium fork, rebuild the Two
Profiles app using the new throttling bypass APIs, and prove both profiles
render at 60fps side by side in one window. Start with the three throttling
patches identified in Issue 408, then add more patches individually as the need
arises.

## Background

Issue 409 attempted to apply Electron's full 147-patch set. The patches applied
cleanly, but the build failed because they depend on Node.js,
`is_electron_build=true`, and dozens of other Electron-specific dependencies
that TermSurf doesn't need.

The three throttling patches identified in Issue 408, however, are pure Chromium
modifications. They add flags and methods to Chromium's rendering pipeline with
no external dependencies. They should apply and build cleanly on vanilla
Chromium.

## The Two Profiles App

The Two Profiles app is a minimal Content API embedder built inside the Chromium
source tree at `content/two_profiles/`. It creates two `ShellBrowserContext`
instances with different storage paths (`~/.config/termsurf/poc/profile-a/` and
`profile-b/`) and displays two `WebContents` side by side in one NSWindow. Each
profile gets isolated cookies, localStorage, and cache.

Issue 407 built the first version of this app (15 files, based on content_shell).
It proved profile isolation works — each pane showed a different localStorage
identity string that persisted across restarts. But rendering was throttled to
2-3fps because Chromium's three independent throttling layers all treated the
second WebContents as hidden.

This issue rebuilds the Two Profiles app on top of the three throttling patches.
The patches add APIs (`disable_hidden_`, `SetSchedulerThrottling`,
`SetBackgroundThrottling`) that bypass each throttling layer. With all three
bypassed, both profiles should render at 60fps.

## Principles

1. **Track Electron's Chromium version.** We use the same Chromium version that
   Electron targets (currently 146.0.7650.0). Electron's patches are proven at
   these specific version numbers. By matching versions, we can always refer to
   their patch set to see how they solved a problem — even if we don't adopt
   their full solution.

2. **Learn from Electron, don't depend on it.** Electron's patches are a
   reference, not a dependency. We read them, understand them, and extract what
   we need. We never apply the full patch set.

3. **Apply only what we need.** Each patch in our fork exists because TermSurf
   requires it. No speculative patches, no "might need it later."

4. **Adapt patches when necessary.** Electron's patches may need modification
   to work without the rest of the Electron build system. That's fine — we own
   our fork and can fix whatever needs fixing.

5. **One branch per Chromium version.** The branch `146.0.7650.0-termsurf` is
   based directly on the vanilla Chromium `146.0.7650.0` tag. There is no
   intermediate Electron branch — TermSurf's patches go straight on top of
   vanilla Chromium.

## Relationship to Other Issues

| Issue | Relationship                                                      |
| ----- | ----------------------------------------------------------------- |
| 407   | Proved multi-profile works; identified 2-3fps throttling          |
| 408   | Traced throttling to three layers; discovered Electron's solution |
| 409   | Attempted full patch set; failed at build due to Electron deps    |
| 410   | This issue — applies throttling patches, rebuilds Two Profiles at 60fps |

## The Three Throttling Patches

These are the patches that solve the 2-3fps problem from Issue 407. Each
addresses one of the three independent throttling layers in Chromium's rendering
pipeline.

### Layer 1: `disable_hidden.patch`

Adds a `disable_hidden_` flag to `RenderWidgetHostImpl`. When set, the widget
ignores `WasHidden()` calls — the renderer process continues producing frames
even when the host thinks the widget is not visible.

**Why we need it:** Chromium's macOS occlusion detection marks WebContents as
hidden when they don't own a top-level NSWindow. Our side-by-side layout puts
two WebContents in one window, so the second one gets marked hidden.

### Layer 2: `allow_disabling_blink_scheduler_throttling_per_renderview.patch`

Adds `SetSchedulerThrottling(bool)` to `WebViewImpl`. When throttling is
disabled, Blink's `PageSchedulerImpl` treats the page as visible regardless of
the actual visibility state — `requestAnimationFrame` fires at full rate instead
of being throttled to ~1fps.

**Why we need it:** Even if Layer 1 prevents `WasHidden()`, Blink has its own
visibility tracking that throttles background pages independently.

### Layer 3: `fix_disabling_background_throttling_in_compositor.patch`

Modifies `ui::Compositor` to keep the display visible when background throttling
is disabled. Without this, the compositor unsubscribes from vsync for hidden
views, meaning no `BeginFrame` signals are sent and rendering stops entirely.

**Why we need it:** The compositor layer is the final gatekeeper. Even if Layers
1 and 2 are bypassed, the compositor must continue requesting frames from the
display for rendering to proceed.

## Electron as Reference

Electron's patch set lives in `electron/patches/chromium/` in the TermSurf
monorepo. These patch files can be read and studied at any time. Issue 408
contains a full inventory of all 147 patches organized by category. When we
encounter a new Chromium problem, we check Electron's patches first to see if
they've already solved it.

## Fork Structure

```
146.0.7650.0           tag    ← vanilla Chromium (matches Electron's target)
146.0.7650.0-termsurf  branch ← TermSurf's patches on top (submodule points here)
```

The `-termsurf` branch is based directly on the vanilla Chromium tag. TermSurf's
patches are informed by Electron's patches but applied and maintained
independently.

## Implementation Plan

### Phase 1: Return to vanilla Chromium

Reset the submodule back to `146.0.7650.0` and create the TermSurf branch.

```bash
cd ts4/termsurf-chromium/src
git checkout 146.0.7650.0
git checkout -b 146.0.7650.0-termsurf
```

- [x] Submodule on `146.0.7650.0-termsurf` branched from vanilla Chromium
- [x] Content Shell builds on vanilla Chromium (sanity check)

### Phase 2: Extract and apply the three throttling patches

Read each patch from `electron/patches/chromium/`, extract only the
throttling-related changes, and apply them to the `-termsurf` branch. The
patches may need adaptation to remove Electron-specific parts — check each one.

The three patch files:

```
electron/patches/chromium/disable_hidden.patch
electron/patches/chromium/allow_disabling_blink_scheduler_throttling_per_renderview.patch
electron/patches/chromium/fix_disabling_background_throttling_in_compositor.patch
```

For each patch:

1. Read the patch file from the Electron repo
2. Verify it modifies only Chromium core code (no `//electron/` references)
3. If clean, apply it directly with `git am`
4. If it has Electron-specific parts, extract only the relevant hunks and
   adapt as needed

- [x] Layer 1 patch applied (disable_hidden)
- [x] Layer 2 patch applied (scheduler throttling)
- [x] Layer 3 patch applied (compositor background throttling)

### Phase 3: Verify Content Shell still builds

Content Shell must still build and run at 60fps after the three patches. This
confirms the patches don't break vanilla Chromium.

```bash
gn gen out/Default --args='is_debug=false symbol_level=0 is_component_build=true'
autoninja -C out/Default content_shell
```

- [x] Content Shell builds successfully
- [x] Content Shell renders test page at 60fps

### Phase 4: Rewrite Two Profiles with throttling bypass

Rebuild the Two Profiles app using the new APIs from the three patches:

```cpp
// After creating each WebContents and laying out the views:
rwh_impl->disable_hidden_ = true;                                 // Layer 1
web_contents->GetRenderViewHost()->SetSchedulerThrottling(false);  // Layer 2
// Layer 3 is handled automatically by the compositor patch
```

- [ ] Two Profiles app created with throttling bypass
- [ ] App builds successfully

### Phase 5: Verify Two Profiles at 60fps

Launch the Two Profiles app with the test server. Both panes should render at
60fps with different localStorage identity strings.

```bash
autoninja -C out/Default content/two_profiles:two_profiles
./out/Default/Two\ Profiles.app/Contents/MacOS/Two\ Profiles
```

- [ ] Both panes render at 60fps
- [ ] Different localStorage strings (profile isolation works)
- [ ] Strings persist across app restarts

### Phase 6: Push and update submodule

Push the `-termsurf` branch upstream and update the parent repo's submodule
reference.

```bash
git push origin 146.0.7650.0-termsurf
```

- [ ] Branch pushed upstream
- [ ] Parent repo submodule updated
