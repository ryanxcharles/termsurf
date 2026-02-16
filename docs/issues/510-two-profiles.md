# Issue 510: Two Profiles Side by Side

## Background

Multiple browser profiles in the same window is a core TermSurf product
requirement. Few browsers support this — Chrome doesn't. It took five
generations (ts1–ts5) and hundreds of experiments to develop an architecture
that works. The breakthrough was forking Chromium and using its Content API
directly, where `BrowserContext` natively supports multiple isolated instances
with separate cookies, localStorage, and cache (Issue 406).

Issue 509 proved the full streaming pipeline: Chromium renders a webpage,
streams IOSurface frames at 60fps over XPC, and the Metal renderer composites
them at pixel-perfect Retina resolution inside a terminal pane. The pipeline
handles resize, correct sRGB colors, and clean lifecycle management.

This issue demonstrates the two-profile capability by rendering two different
browser profiles side by side in split panes in the same window. Each profile
gets its own Chromium Profile Server process with its own `--user-data-dir`,
producing fully isolated browser sessions — different cookies, different
localStorage, different cache.

## Product requirements

### Profile naming

A profile name must:

- Consist of lowercase alphanumeric characters only (`a-z`, `0-9`)
- Start with a letter (`a-z`)
- Be non-empty

This is intentionally strict. Profile names are compatible with variable names
in software, filesystem paths, URL slugs, and configuration keys. This gives
maximum flexibility for future use. Examples: `default`, `work`, `personal`,
`guest`, `dev`.

The `web` CLI accepts `--profile <name>` (default: `default`). The profile name
must be validated before use.

### Profile data isolation

Each profile maps to a separate Chromium `--user-data-dir`:

```
~/.config/termsurf/profiles/<name>/
```

Two panes with `--profile work` share the same server process and browser
session (same cookies, same localStorage). Two panes with different profiles
(`work` vs `personal`) get separate server processes with separate data.

### Display

The `web` TUI already renders the profile name in the URL bar's top-right
corner. This is purely cosmetic — no changes needed to the UI layout itself.

## Current state

### What already works

| Component                  | Status  | Notes                                             |
| -------------------------- | ------- | ------------------------------------------------- |
| `web` CLI `--profile` flag | Working | Parses flag, displays in URL bar                  |
| Profile name in URL bar    | Working | Renders icon + name in top-right                  |
| Chromium `--user-data-dir` | Working | Per-process data isolation                        |
| One server per pane        | Working | Spawns, streams, terminates cleanly               |
| IOSurface streaming        | Working | 60fps, pixel-perfect Retina                       |
| Dynamic resize             | Working | XPC `resize` message, never stretch               |
| xpc-gateway                | Working | Stateless rendezvous, no profile awareness needed |

### What needs to change

**1. `web` must send the profile name over XPC.**

Currently `set_overlay` does not include the profile name. The app has no way to
know which profile the `web` process is using. The profile name must be added to
`set_overlay` so the app can route to the correct server process.

**2. The app must route server processes by profile, not just by pane.**

Currently `CompositorXPC.swift` maps everything by pane UUID. It spawns one
server per pane with a hardcoded profile path (`profiles/default`). Two panes
with the same profile should share one server process (and thus one browser
session). Two panes with different profiles must get different server processes.

The server process mapping needs to change from `[UUID: Process]` to something
that groups panes by profile. When a second pane requests `--profile work` and a
server for `work` is already running, the app should reuse that server and send
a second `create_tab` rather than spawning a new process.

**3. The hardcoded profile path must use the actual profile name.**

`CompositorXPC.swift` line 407 hardcodes `profiles/default`. This must become
`profiles/<name>` using the profile name from the `set_overlay` message.

### What should work without changes

**xpc-gateway** — Pure stateless rendezvous. Returns the app's endpoint to any
process that asks. No profile awareness needed.

**Chromium Profile Server** — Already accepts `--user-data-dir` as a flag. Each
instance is a separate process with a separate data directory. No source changes
should be needed — just pass a different path per profile.

**Metal renderer** — Already composites multiple overlays by pane UUID. Each
pane independently receives IOSurface frames and renders them. No changes
needed.

## XPC protocol changes

### `web` → app: `set_overlay`

Add `profile` field:

```
{ action: "set_overlay",
  pane_id: "<uuid>",
  col: N,
  row: N,
  width: N,
  height: N,
  url: "http://...",
  profile: "work" }          // (new) profile name
```

### app → server: `create_tab`

Unchanged — the app already knows the profile because it spawned the server with
the correct `--user-data-dir`. The tab just needs a URL.

```
{ action: "create_tab",
  url: "http://...",
  tab_id: "<uuid>",
  pixel_width: N,
  pixel_height: N }
```

### app → server: `resize`

Unchanged.

```
{ action: "resize",
  pixel_width: N,
  pixel_height: N }
```

### server → app: `server_register`, `tab_ready`, `display_surface`

Unchanged.

## Architecture note: one server per profile

The current code spawns one server per pane. The two-profile demo needs one
server per profile. This means:

- Pane A (`--profile work`) → spawns server with
  `--user-data-dir=~/.config/termsurf/profiles/work`
- Pane B (`--profile personal`) → spawns server with
  `--user-data-dir=~/.config/termsurf/profiles/personal`
- Pane C (`--profile work`) → reuses server from pane A, sends `create_tab`

For the initial demo (two profiles, one pane each), pane-per-server and
profile-per-server are equivalent — each profile has exactly one pane. Server
reuse (pane C scenario) is a future optimization. The demo should still be
designed with this in mind, but it's not required to pass.

## Ideas for experiments

1. **Profile name validation + XPC propagation.** Add validation to `web`,
   include `profile` in `set_overlay`, and update `CompositorXPC.swift` to
   extract it and use it for the `--user-data-dir` path. Test with a single pane
   using `--profile work` and verify the data goes to
   `~/.config/termsurf/profiles/work/`.

2. **Two profiles side by side.** Open two split panes, run `web` with different
   `--profile` flags in each, verify two separate server processes spawn with
   separate data directories, and both render independently at 60fps.

3. **Session isolation verification.** Load a page that writes to localStorage
   in both profiles. Verify each profile sees only its own data. Close and
   reopen — verify persistence within each profile and isolation between them.
