# Issue 708: Roamium-only Chromium fork

## Goal

Create a clean Chromium fork branch that contains only `libtermsurf_chromium`
(renamed from `libtermsurf_content`) and no legacy binaries. Delete the Chromium
Profile Server and Plusium entirely. Roamium — built outside the Chromium tree
with Cargo — is the only browser binary.

## Background

### How we got here

The Chromium fork has accumulated 68 commits across Issues 411–707. These
commits contain three separate binaries:

1. **Chromium Profile Server** — The original monolithic fork. ~1,050 lines of
   TermSurf-specific code spread across ~100 files inside `content/shell/`. It
   was the browser from ts5 through Issue 703. Every feature required modifying
   Chromium's Content Shell internals directly.

2. **Plusium** — A 511-line C++ binary at `content/plusium/` that wraps
   `libtermsurf_content` through its C API. Created in Issue 704, debugged
   through Issue 705, DevTools crash fixed in Issue 706. It was the proof that
   the C library design works — a stepping stone to Roamium.

3. **Roamium** — A ~400-line Rust binary at `roamium/` (repo root, outside the
   Chromium tree). Created in Issue 707. It links `libtermsurf_content.dylib`,
   uses prost for protobuf, and is a verified drop-in replacement for Plusium.
   All 27 feature tests pass in both debug and release.

Now that Roamium works, the other two binaries are dead weight:

- **Chromium Profile Server** adds ~100 forked files to the Chromium tree. Every
  upstream merge requires rebasing these. Its functionality is fully replaced by
  `libtermsurf_content` + Roamium.
- **Plusium** was always a stepping stone. It copies proto files into the
  Chromium tree (Roamium links them directly from `proto/`). Its only remaining
  value is as a debugging reference, but the C API is simple enough that Roamium
  serves that purpose.

### What stays

**`libtermsurf_content`** — Renamed to **`libtermsurf_chromium`**. This is the C
library that wraps Chromium's Content API. It exports 20 functions with C types
only. It lives inside the Chromium tree at `content/libtermsurf_chromium/` and
builds as a shared library via GN. This is the only TermSurf code inside the
Chromium fork.

The rename reflects what it actually is: TermSurf's Chromium binding layer. The
old name (`libtermsurf_content`) was an artifact of it living in `content/` — it
described the Chromium module it wraps, not its purpose.

### What gets deleted

- `content/chromium_profile_server/` — The entire Profile Server fork
- All forked Content Shell files (the ~100 files in `content/shell/` that the
  Profile Server modified)
- `content/plusium/` — Plusium binary and its local proto copy
- All GN targets for `chromium_profile_server` and `plusium`

### What gets renamed

- `content/libtermsurf_content/` → `content/libtermsurf_chromium/`
- `libtermsurf_content.h` → `libtermsurf_chromium.h`
- `libtermsurf_content.dylib` → `libtermsurf_chromium.dylib`
- All internal references (GN targets, include paths, symbol names)

### What gets updated outside Chromium

- `roamium/build.rs` — Link `libtermsurf_chromium` instead of
  `libtermsurf_content`
- `roamium/src/ffi.rs` — Update comments if header name appears
- `scripts/build-debug.sh` — Remove `plusium` from autoninja targets, update
  target name if needed
- `scripts/build-release.sh` — Same
- `gui/src/apprt/xpc.zig` — Remove `plusium` and `chromium` from browser
  registry, keep only `roamium`
- `chromium/README.md` — New branch, updated build target

## Approach

Start from the vanilla `146.0.7650.0` tag — not from the current issue-707
branch. Cherry-pick or rewrite only the commits that create
`libtermsurf_chromium`. This gives us a minimal, clean branch with no historical
baggage from the Profile Server, Plusium, or the dozens of intermediate
experiments.

The key things to preserve (from issue-707) are:

**The library itself** (`content/libtermsurf_chromium/`):

- `libtermsurf_chromium.h` — C API header (20 functions, `TS_EXPORT` macros)
- `libtermsurf_chromium.cc` — Implementation
- `ts_browser_main_parts.h` / `ts_browser_main_parts.cc` — BrowserMainParts
  subclass (tab registry, callbacks, DevTools via `tab_id`)
- `ts_browser_client.h` / `ts_browser_client.cc` — ContentBrowserClient subclass
  (dark mode `OverrideWebPreferences`, BadgeService stub)
- `ts_main_delegate.h` / `ts_main_delegate.cc` — ContentMainDelegate subclass
  (skip bundle path DCHECK)
- `BUILD.gn` — Shared library target

**Patches to stock Chromium files:**

- `content/shell/browser/shell_devtools_frontend.h` — Made
  `ShellDevToolsFrontend` constructor **public** (was private). The library
  needs to construct DevTools frontends directly rather than going through the
  static `Show()` method.
- `content/shell/browser/shell_platform_delegate_mac.mm` — Added `--hidden`
  flag: `setAlphaValue:0` + `orderWindow:NSWindowBelow`. Keeps the compositor
  active while hiding the Content Shell window. Without this, CALayerHost
  compositing breaks.
- `content/shell/common/shell_switches.h` — Added `kHidden` switch constant.
- `BUILD.gn` (root) — Add `libtermsurf_chromium` to `gn_all` group.

**Patches that can be removed** (only existed for Profile Server or Plusium):

- `net/dns/BUILD.gn` — Profile Server visibility allowlist entry.
- `tools/gritsettings/resource_ids.spec` — Profile Server resource IDs.

Everything else — Profile Server forks, Plusium, XPC code,
FrameSinkVideoCapturer code — gets left behind.

## Ideas for experiments

1. **Create the clean branch.** Start from `146.0.7650.0`, create
   `146.0.7650.0-issue-708`. Cherry-pick or rewrite the essential commits.
   Rename `libtermsurf_content` → `libtermsurf_chromium`. Build with autoninja.

2. **Update Roamium.** Change `build.rs` and `ffi.rs` to reference the new
   library name. Update build scripts. Verify Roamium still works end-to-end.

3. **Update GUI.** Remove `plusium` and `chromium` from the browser registry.
   Make `roamium` the default browser (empty `--browser` flag resolves to
   roamium). Verify `web google.com` works without `--browser roamium`.

4. **Clean up patches.** Generate a fresh patch set for issue-708. This should
   be dramatically smaller than issue-707's 68 patches.
