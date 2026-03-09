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

### Experiment 1: Can Roamium run without .app bundles?

#### Goal

Determine whether Chromium's helper processes can be plain executables colocated
with the Roamium binary, or whether they require macOS .app bundle structure.

#### Research

**How Chromium finds helper executables**
(`content/browser/child_process_host_impl.cc:59-118`):

1. Check `--browser-subprocess-path` CLI flag — if set, use that path directly
2. Fall back to `CHILD_PROCESS_EXE` path service (current executable or
   overridden via `OverrideChildProcessPath()`)
3. **macOS-only transform (line 90-114):** If `flags != CHILD_NORMAL` AND
   `base::apple::AmIBundled()` returns true, navigate up from the helper path
   and construct `.app/Contents/MacOS/` paths for specialized helpers (Renderer,
   GPU, Plugin)

The .app bundle transform at line 90 has two conditions:

```cpp
if (flags != CHILD_NORMAL && base::apple::AmIBundled()) {
```

`AmIBundled()` returns true only when the process is running from inside a macOS
.app bundle. Roamium runs as a plain binary launched by the board (just like
`web`), so `AmIBundled()` returns false and the entire .app bundle transform is
skipped.

**Roamium already works this way today.** Looking at `roamium/src/main.rs`,
Roamium is a plain Rust binary that:

1. Parses `--ipc-socket=` and `--user-data-dir=` from argv
2. Passes all argv through to `ts_content_main()` (which calls Chromium's
   `ContentMain`)
3. Runs from `chromium/src/out/Default/roamium` — a plain binary, not inside any
   .app bundle

The board (Wezboard) launches it as a plain process via
`std::process::Command::new(&binary)` in `spawn_server()`. No .app bundle
involved.

**The `--browser-subprocess-path` flag** (line 62-63) is checked before any
bundle logic. Roamium can pass this flag to tell Chromium where to find the
helper executable. The helper binary handles all process types (Renderer, GPU,
Plugin) — Chromium dispatches by `--type=` argument, not by executable name.

**Current file layout in `chromium/src/out/Default/`:**

The .app bundles that exist today (Chromium Profile Server Helper.app, etc.) are
build artifacts from Chromium's default macOS build configuration. They are NOT
required when the main process is a plain binary. The actual executables inside
those .app bundles can be extracted and colocated as plain files.

#### Proposed install layout

```
/usr/local/lib/roamium/
  roamium                                  # main binary
  chromium_profile_server_helper           # helper (all types via --type=)
  libtermsurf_chromium.dylib               # + other dylibs
  *.pak, icudtl.dat, v8_context_snapshot*  # resources
/usr/local/bin/roamium → ../lib/roamium/roamium  # symlink
```

Roamium would pass
`--browser-subprocess-path=/usr/local/lib/roamium/chromium_profile_server_helper`
to Chromium, and all specialized helper types (GPU, Renderer, Plugin) use the
same binary dispatched by `--type=`.

#### Result

Research confirms the approach is viable.

#### Conclusion

Roamium can run as a plain binary with `--browser-subprocess-path` pointing to
colocated helper executables. No .app bundles required. `AmIBundled()` returns
false for plain binaries, so Chromium's macOS bundle path transform never fires.
The install would be a directory containing the binary, helper, dylibs, and
resources, with a symlink in `$PATH`. This approach works for both Wezboard and
Ghostboard — any board that speaks the TermSurf protocol can launch Roamium from
its installed location.

### Experiment 2: Install script and test with Ghostboard

#### Goal

Create `scripts/install-roamium.sh` that installs Roamium to
`/usr/local/lib/roamium/` with a symlink at `/usr/local/bin/roamium`, then test
it by running `web --browser /usr/local/bin/roamium termsurf.com` in Ghostboard.

#### Background

Both Ghostboard (`xpc.zig:848`) and Wezboard (`conn.rs:938-939`) hardcode
Roamium's path to `$HOME/dev/termsurf/chromium/src/out/Default/roamium`. Both
also support absolute paths — the TUI's `--browser` flag passes the path through
the protocol, and the board's `resolveBrowserPath()` returns absolute paths
as-is.

Roamium already runs as a plain binary (not in a .app bundle). The existing
`@loader_path/.` rpath in `roamium/build.rs` means dylibs resolve relative to
the binary's actual location. A symlink at `/usr/local/bin/roamium` resolves to
`/usr/local/lib/roamium/roamium`, so `@loader_path` becomes
`/usr/local/lib/roamium/` — exactly where the dylibs will be.

Without `--browser-subprocess-path`, Chromium falls back to `CHILD_PROCESS_EXE`
(the current executable). Since `AmIBundled()` is false, no .app transform
happens — Chromium will re-invoke `roamium` itself as the helper process,
dispatching by `--type=`. This is the same pattern as content_shell on Linux.

#### Design

**1. Create `scripts/install-roamium.sh`**

```bash
#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_DIR="$(dirname "$SCRIPT_DIR")"
CHROMIUM_OUT="$REPO_DIR/chromium/src/out/Default"
ROAMIUM_SRC="$REPO_DIR/roamium/target/release/roamium"
INSTALL_DIR="/usr/local/lib/roamium"

# Verify release build exists.
if [ ! -f "$ROAMIUM_SRC" ]; then
  echo "Error: Release build not found at $ROAMIUM_SRC"
  echo "Run: scripts/build-roamium.sh --release"
  exit 1
fi

echo "==> Installing Roamium to $INSTALL_DIR..."
sudo mkdir -p "$INSTALL_DIR"

# Copy roamium binary.
sudo cp "$ROAMIUM_SRC" "$INSTALL_DIR/roamium"

# Copy dylibs.
echo "==> Copying dylibs..."
sudo cp "$CHROMIUM_OUT"/*.dylib "$INSTALL_DIR/"

# Copy resources.
echo "==> Copying resources..."
sudo cp "$CHROMIUM_OUT"/*.pak "$INSTALL_DIR/"
sudo cp "$CHROMIUM_OUT/icudtl.dat" "$INSTALL_DIR/"
sudo cp "$CHROMIUM_OUT"/v8_context_snapshot*.bin "$INSTALL_DIR/"

# Symlink to /usr/local/bin.
echo "==> Symlinking /usr/local/bin/roamium..."
sudo ln -sf "$INSTALL_DIR/roamium" /usr/local/bin/roamium

echo ""
echo "Done."
echo "  Dir:  $INSTALL_DIR"
echo "  Bin:  /usr/local/bin/roamium"
```

**2. Test with Ghostboard**

```bash
# Build roamium release
scripts/build-roamium.sh --release

# Install
scripts/install-roamium.sh

# Test — launch Ghostboard, then in the terminal:
web --browser /usr/local/bin/roamium termsurf.com
```

#### Verification

1. `scripts/install-roamium.sh` completes without errors
2. `/usr/local/bin/roamium` exists and is a symlink to
   `/usr/local/lib/roamium/roamium`
3. `/usr/local/lib/roamium/` contains the binary, dylibs, .pak files, and
   resource files
4. In Ghostboard: `web --browser /usr/local/bin/roamium termsurf.com` opens
   termsurf.com with the installed Roamium — page renders, input works
5. In Ghostboard: `web --browser /usr/local/bin/roamium lite.duckduckgo.com` —
   second test to confirm navigation works
6. DevTools: `:devtools right` — verify helper processes spawn correctly from
   the installed location

**Result:** Fail

The install script works — files are copied, symlink resolves, dylibs load via
`@loader_path/.`. But Roamium crashes immediately:

```
[ERROR:base/i18n/icu_util.cc:177] icudtl.dat not found in bundle
[FATAL:base/i18n/icu_util.cc:306] Check failed: result.
```

Chromium on macOS loads resources (`icudtl.dat`, `.pak` files) via
`PathForFrameworkBundleResource()` (`base/apple/foundation_util.mm:108`), which
calls `[NSBundle URLForResource:withExtension:]` on the framework bundle. This
API expects an actual macOS bundle structure (`*.framework/Resources/`). A flat
directory isn't an NSBundle, so the lookup returns nil.

The resource loading path on macOS (`base/i18n/icu_util.cc:167-170`):

```cpp
#else  // !BUILDFLAG(IS_APPLE)
  // Assume it is in the framework bundle's Resources directory.
  FilePath data_path = apple::PathForFrameworkBundleResource(kIcuDataFileName);
```

On non-Apple platforms (line 148), Chromium uses `DIR_ASSETS` which resolves to
the directory containing the executable — exactly what we want. But on macOS it
unconditionally uses the framework bundle API.

The `OverrideFrameworkBundlePath()` in `paths_apple.mm` is called before
`ContentMain` and navigates up from the binary expecting
`.app/Contents/Frameworks/Content Shell Framework.framework`. When the binary is
at `/usr/local/lib/roamium/roamium`, this navigation produces a nonsensical
path.

#### Conclusion

The install script and symlink approach is correct — dylibs load fine via
`@loader_path`. The blocker is Chromium's macOS resource loading, which
unconditionally uses NSBundle APIs that require framework bundle structure. The
Chromium fork needs a patch to fall back to the executable's directory when no
framework bundle exists. This is the next experiment.

### Experiment 3: Patch Chromium resource loading for flat install

#### Goal

Patch `paths_apple.mm` in the Chromium fork so that when Roamium runs as a plain
binary (not inside a `.app` or `.framework` bundle), resource loading falls back
to the directory containing the executable.

#### Background

Chromium's macOS resource loading chain:

1. `shell_content_main.cc:30` calls `OverrideFrameworkBundlePath()`
2. `paths_apple.mm:52` navigates up from the binary to find
   `Contents/Frameworks/Content Shell Framework.framework`
3. `SetOverrideFrameworkBundlePath()` sets this as the framework bundle
4. `icu_util.cc:169` calls `PathForFrameworkBundleResource("icudtl.dat")`
5. `foundation_util.mm:108` calls `[NSBundle URLForResource:...]` on the
   framework bundle → returns nil for flat directories

On non-Apple platforms, `icu_util.cc:148` uses `DIR_ASSETS` (the executable's
directory). The `.pak` loading in content_shell likely has the same pattern.

The fix: when the binary is not inside a bundle, override the framework bundle
path to point at the executable's own directory. `NSBundle` can represent any
directory if created with `bundleWithPath:` — the `URLForResource` API will then
look for files directly in that directory.

#### Branch

Fork `146.0.7650.0-issue-708` → `146.0.7650.0-issue-730`:

```bash
cd ~/dev/termsurf/chromium/src
export PATH="$(cd ../depot_tools && pwd):$PATH"
git checkout 146.0.7650.0-issue-708
git checkout -b 146.0.7650.0-issue-730
```

#### Design

**1. `content/shell/app/paths_apple.mm` — Handle non-bundled binaries**

Modify `GetContentsPath()` to detect when the binary isn't inside a `.app`
bundle. When it's a flat binary, `OverrideFrameworkBundlePath()` should point
the framework bundle at the directory containing the executable.

```cpp
void OverrideFrameworkBundlePath() {
  base::FilePath exe_path;
  base::PathService::Get(base::FILE_EXE, &exe_path);
  base::FilePath exe_dir = exe_path.DirName();

  // Check if we're inside a .app bundle by looking for "Contents" ancestor.
  bool in_bundle = false;
  base::FilePath check = exe_dir;
  for (int i = 0; i < 10 && !check.empty(); i++) {
    if (check.BaseName().value() == "Contents") {
      in_bundle = true;
      break;
    }
    check = check.DirName();
  }

  base::FilePath framework_path;
  if (in_bundle) {
    // Standard bundle path: Contents/Frameworks/Content Shell Framework.framework
    framework_path =
        GetFrameworksPath().Append("Content Shell Framework.framework");
  } else {
    // Flat binary: use the executable's directory as the "framework bundle".
    // NSBundle will look for resources directly in this directory.
    framework_path = exe_dir;
  }

  base::apple::SetOverrideFrameworkBundlePath(framework_path);
}
```

**2. Similarly patch `OverrideChildProcessPath()`** to handle the flat case —
when not in a bundle, the child process is the executable itself (already the
default behavior when `CHILD_PROCESS_EXE` isn't overridden, so this may be a
no-op).

**3. Similarly patch `OverrideOuterBundlePath()` and `OverrideBundleID()`** to
handle the non-bundled case gracefully (return early or use defaults).

#### Verification

1. Apply patches to Chromium fork on a new branch
2. Rebuild Chromium: `autoninja -C out/Default chromium_profile_server`
3. Rebuild Roamium: `scripts/build-roamium.sh --release`
4. Reinstall: `scripts/install-roamium.sh`
5. Test: `/usr/local/bin/roamium --help 2>&1` — no ICU crash
6. Test in Ghostboard: `web --browser /usr/local/bin/roamium termsurf.com` —
   page renders
7. Verify `.pak` files also load (page content renders, not just process start)
