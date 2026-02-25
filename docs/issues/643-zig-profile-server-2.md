# Issue 643: Zig Profile Server (Take 2)

## Goal

Same as Issue 642: rewrite the Chromium Profile Server in Zig. But this time,
the Zig code lives inside `chromium/src/` and is built as part of the Chromium
build. `autoninja` produces a complete, working app bundle in a single step — no
separate `zig build`, no `cp`, no `codesign`.

## Background

Issue 642 proved the Zig-to-Chromium bridge works (Experiments 1–2) but failed
to deploy the Zig binary into a working app bundle (Experiments 3–5). Every
failure came from the same root cause: trying to combine outputs from two
separate build systems (`autoninja` for the bundle, `zig build` for the binary)
into a single coherent app bundle. The two build systems produce binaries with
different code signing properties, different bundle expectations, and different
path resolution behavior.

The lesson: don't fight the Chromium build system. If the Zig code lives inside
`chromium/src/` and `autoninja` builds everything, the app bundle is correct by
construction. No post-build surgery. No codesigning. No path mismatches.

## What Issue 642 proved

- **Experiment 1:** Zig can dlopen the Chromium framework, resolve C API
  symbols, register callbacks, and drive ContentMain. Google.com loads in a
  Shell window.
- **Experiment 2:** Zig can create WebContents directly (no Shell window) and
  receive stable CAContext IDs from the persistent compositor.
- **Experiment 3 (partial):** The XPC gateway code works — when launched
  manually from the terminal, the Zig server connects to the gateway, creates a
  BrowserContext, and sends `server_register`. The code is correct; only the
  deployment failed.

## What Issue 642 failed at

Three deployment approaches, three different failures:

1. Copy Zig binary into autoninja bundle + `codesign --deep` → code signing
   killed the process (`linker-signed` vs full adhoc mismatch).
2. `zig build` assembles bundle with symlinked framework → `realpath()` resolves
   symlink, breaks Chromium's `GetContentsPath()` DCHECK.
3. `zig build` assembles bundle with copied framework → `GetContentsPath()`
   DCHECK still fails, bundle structure doesn't match Chromium's expectations.

All three failures disappear if `autoninja` builds the entire app bundle
including the Zig binary.

## Approach

Move the Zig source files into `chromium/src/content/zig_profile_server/` next
to the existing C++ shim. Add a GN `action()` target that invokes the Zig
compiler as part of the `autoninja` build. The GN target produces the main
executable, which GN then packages into the app bundle alongside the framework
and Helpers — exactly like it does today for the C++ executable.

### Files to move

The uncommitted Zig code from Issue 642 moves into the Chromium fork:

| From (main repo)        | To (chromium/src/content/zig_profile_server/)    |
| ----------------------- | ------------------------------------------------ |
| `browser/build.zig`     | Not needed — GN invokes Zig directly             |
| `browser/build.zig.zon` | Not needed — no Zig package manager in this path |
| `browser/src/main.zig`  | `main.zig`                                       |

The `zig_objc` dependency (ObjC blocks for XPC handlers) needs a different
solution since there's no `build.zig.zon`. Options:

- Vendor the `zig_objc` source into the Chromium tree
- Use `@cImport` to call the ObjC block runtime directly (avoiding the
  dependency)
- Download the dependency as a GN `action()` step

### GN integration

GN doesn't know how to compile `.zig` files natively, but it can run arbitrary
commands via `action()`. The approach:

```gn
action("zig_profile_server_exe") {
  script = "//content/zig_profile_server/build_zig.py"
  sources = [ "//content/zig_profile_server/main.zig" ]
  outputs = [ "$root_out_dir/zig_profile_server" ]
  args = [
    "--zig", "zig",
    "--source", rebase_path("main.zig", root_build_dir),
    "--output", rebase_path("$root_out_dir/zig_profile_server", root_build_dir),
  ]
}
```

The Python script wraps `zig build-exe` with the right flags (rpath, libc
linking, target triple). The resulting binary is then referenced by the existing
`zig_profile_server` bundle target as the main executable.

This is the same pattern Chromium uses for other non-C++ build steps (e.g.,
generating protocol buffers, building Rust targets via `gn_rs`).

### What stays in the main repo

- `gui/src/apprt/xpc.zig` — the GUI's XPC module (unchanged, points at
  `chromium/src/out/Default/Zig Profile Server.app`)
- `docs/issues/643-zig-profile-server-2.md` — this document
- `browser/` — can be cleaned up after the code is moved

### Build workflow

```bash
# Single command builds everything: C++ shim, Zig binary, app bundle
cd chromium/src && autoninja -C out/Default zig_profile_server

# GUI (unchanged)
cd gui && zig build && open zig-out/TermSurf.app
```

## Stages

Same stages as Issue 642, but the deployment problem is solved from the start.

### Stage 1: GN builds Zig binary into app bundle

Move `main.zig` into the Chromium tree. Write the GN `action()` and Python
wrapper. `autoninja` produces a complete app bundle with the Zig executable.
Verify it launches standalone (Experiment 2 equivalent).

### Stage 2: XPC gateway

Port the XPC code from the Issue 642 `main.zig` (already written, already tested
from the terminal). Resolve the `zig_objc` dependency. Connect to the GUI's XPC
gateway, receive `create_tab`, send back `ca_context_id`. Web content renders in
the terminal pane.

### Stage 3: Input forwarding

Port mouse, keyboard, and scroll event forwarding. The Zig code receives XPC
input messages and calls the C shim's `ts_forward_*` functions.

### Stage 4: WebContents observation

Port URL, title, loading state, and cursor change observation. The C++ shim
fires callbacks into Zig, which sends XPC messages back to the GUI.

### Stage 5: Navigation and remaining features

Port navigation actions, new-tab link interception, dock icon hiding, focus
management, and auto-exit.

### Stage 6: Replace chromium_profile_server

Once the Zig Profile Server has feature parity, switch the GUI to connect to it
instead of the C++ profile server. Remove the old `chromium_profile_server`
target.

## Chromium Branch

`146.0.7650.0-issue-643` — forked from `146.0.7650.0-issue-642` to preserve the
C++ shim from Issue 642's Experiments 1–2.

## Experiments
