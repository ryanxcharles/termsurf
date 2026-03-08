# Issue 730: Roamium standalone install

## Goal

Make Roamium installable as a standalone package, separate from any board. Today
Roamium lives inside `chromium/src/out/Default/` alongside ~100 Chromium dylibs,
5 helper .app bundles, .pak files, and resource files. Wezboard hardcodes the
path `$HOME/dev/termsurf/chromium/src/out/Default/roamium`. This needs to become
a proper install — either a single binary or a self-contained bundle — so that
boards discover it via `$PATH` or a known install location.

## Background

### Why this matters

TermSurf is a protocol. Boards (Ghostboard, Wezboard), browser engines (Roamium,
Surfari, Girlbat), and TUIs (`web`) are all separate components that speak the
same protobuf/Unix socket protocol. Users should be able to install each
independently:

- `web` → `/usr/local/bin/web` (already works)
- `roamium` → `/usr/local/bin/roamium` or `/usr/local/lib/roamium/`
- Boards → their own install paths

Third-party apps that implement the TermSurf protocol should be able to launch
Roamium without knowing where Chromium was built.

### Current state

**Roamium binary** (`roamium/`): A ~1 MB Rust binary that links
`libtermsurf_chromium.dylib` and speaks protobuf over Unix sockets. Minimal Rust
code (~400 lines across main.rs, ffi.rs, ipc.rs, dispatch.rs).

**Runtime dependencies** (all in `chromium/src/out/Default/`):

| Category                   | Files                                       | Size   |
| -------------------------- | ------------------------------------------- | ------ |
| libtermsurf_chromium.dylib | 1                                           | ~11 MB |
| Chromium component dylibs  | ~100+                                       | large  |
| Helper .app bundles        | 5 (Server, GPU, Renderer, Plugin, Helper)   | large  |
| Resource files             | .pak, icudtl.dat, v8_context_snapshot\*.bin | ~50 MB |

**Build flow:**

1. Chromium is built in `chromium/src/out/Default/`
2. `scripts/build-roamium.sh` runs `cargo build` and copies the binary there
3. `roamium/build.rs` sets two rpaths: `@loader_path/.` and the chromium build
   dir
4. Wezboard's `resolve_browser_path()` hardcodes
   `$HOME/dev/termsurf/chromium/src/out/Default/roamium`

**Install script** (`scripts/install.sh`): Bundles Chromium files into
`TermSurf Ghostboard.app/Contents/Chromium/` but does NOT copy the roamium
binary itself.

### The challenge

Roamium cannot be a single static binary. Chromium is fundamentally a
multi-process architecture — it launches helper processes (GPU, Renderer,
Plugin) as separate executables. These helpers are .app bundles on macOS. The
~100 component dylibs are how Chromium's build system produces its output. The
.pak files and ICU data are loaded at runtime by path.

Options to investigate:

1. **Bundle directory** — Install Roamium as a directory
   (`/usr/local/lib/roamium/`) containing the binary, all dylibs, helper apps,
   and resources. Put a symlink or wrapper script at `/usr/local/bin/roamium`.

2. **macOS .app bundle** — Package as `Roamium.app` in `/Applications/` or
   `/usr/local/lib/`. Chromium already expects .app structure on macOS.

3. **Static linking** — Investigate whether Chromium can be built as a single
   static library (`is_component_build = false` in GN args). This would
   eliminate the ~100 dylibs but helper .app bundles and resources would still
   be needed.

4. **Single binary with embedded resources** — Investigate whether .pak files
   and ICU data can be embedded in the binary or the dylib. Even if possible,
   helper processes still need to be separate executables.

### Questions to answer

1. What is the minimum set of files Roamium needs at runtime? (Can we trim the
   ~100 dylibs by building non-component?)
2. Can Chromium's helper processes be colocated with the main binary, or do they
   require .app bundle structure on macOS?
3. What does the file layout look like on Linux vs macOS? Linux doesn't use .app
   bundles.
4. How should boards discover Roamium? `$PATH` lookup? A config file? A
   well-known install path?
5. How do debug builds work? Developers need fast iteration without a full
   install step.

### How other projects handle this

- **Electron** — Ships as a .app bundle (macOS) or directory (Linux/Windows)
  containing the framework, helpers, and resources.
- **CEF** — Distributes as a directory with the main binary, libcef.so/dylib,
  helpers, and resources. Applications bundle everything together.
- **Chrome itself** — Installs as a .app bundle on macOS, a directory in
  `/opt/google/chrome/` on Linux.

## Experiments

Experiments will investigate the feasibility of each approach, starting with
understanding the minimum runtime file set.
