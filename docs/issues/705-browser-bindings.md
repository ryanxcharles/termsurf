# Issue 705: Browser bindings for Chromium (continued)

## Goal

Same goal as Issue 704: create standalone browser binaries (Roamium, Zoomium,
Plusium) that wrap Chromium's Content API through a shared C library
(`libtermsurf_content`). Each binary speaks the TermSurf IPC protocol (Unix
sockets + length-prefixed protobuf). The TUI's `--browser` flag selects which
binary to use: `web google.com --browser plusium`. After all three work, Roamium
becomes the default and the Chromium Profile Server is retired.

## Background

### What Issue 704 accomplished

Issue 704 ran five experiments across two sessions:

1. **C library extraction** — Confirmed `libtermsurf_content` already provides a
   clean C API boundary (`chromium/src/content/libtermsurf_content/`). No new
   library needed.

2. **Profile server dependency audit** — Verified `libtermsurf_content` has no
   dependencies on the Chromium Profile Server's forked code. It links against
   stock `content/shell`.

3. **Plusium C++ binary** — Built a working C++ binary
   (`content/plusium/plusium_main.cc`) with its own `BUILD.gn` and protobuf IPC.
   Compiles and links successfully.

4. **`--browser` flag** — Added end-to-end support for selecting browser
   binaries: proto schema changes (`browser` field on SetOverlay/
   SetDevtoolsOverlay, `browsers` on HelloReply), TUI `--browser` CLI flag, GUI
   browser registry with composite `(profile, browser)` server keys, and dynamic
   binary path resolution in `spawnServerProcess()`. All compiles clean.

5. **Skip bundle path check** — Overrode `BasicStartupComplete()` in
   `TsMainDelegate` to skip `EnsureCorrectResolutionSettings()`, which crashes
   with a DCHECK when the binary isn't inside a `.app` bundle. Fixed the crash.

### Where it stopped

Plusium starts without crashing but a Content Shell window appears on screen
because stock `shell_platform_delegate_mac.mm` has no `--hidden` support.

The existing Chromium Profile Server solved this with a **forked**
`shell_platform_delegate_mac.mm` in `content/chromium_profile_server/browser/`
that checks for a `--hidden` flag:

```objc
if (base::CommandLine::ForCurrentProcess()->HasSwitch(switches::kHidden)) {
    [window setAlphaValue:0.0];
    [window orderWindow:NSWindowBelow relativeTo:0];
} else {
    [window makeKeyAndOrderFront:nil];
}
```

This makes the window fully transparent and orders it behind all other windows.
`orderOut:` would detach the compositor (it triggers
`NSWindowDidChangeOcclusionStateNotification` which sets
`render_widget_host_is_hidden_ = true`). The `setAlphaValue:0` trick keeps the
window in the window list so the compositor stays active and CAContext survives
navigation.

The GUI already passes `--hidden` to server processes (line 1004 of `xpc.zig`),
but stock Content Shell doesn't recognize it.

### Chromium branch

All Chromium work is on `146.0.7650.0-issue-704` (4 commits: C library
extraction, profile server dependency removal, Plusium binary, bundle path
DCHECK fix). This issue continues on a new branch created from it.

### Code already in place

**Chromium fork (`146.0.7650.0-issue-704`):**

- `content/libtermsurf_content/` — C library with `TsMainDelegate` (overrides
  `BasicStartupComplete()`), `TsBrowserClient`, `TsBrowserMainParts`, tab
  management, input forwarding, persistent compositor, CALayerHost bridge
- `content/plusium/plusium_main.cc` — C++ binary with socket IPC, protobuf
  message dispatch, tab registry, callback wiring
- `content/plusium/BUILD.gn` — Build target linking `libtermsurf_content`
- `content/plusium/termsurf.proto` — Local copy of proto schema

**Main repo (`main`):**

- `proto/termsurf.proto` — `browser` field on SetOverlay/SetDevtoolsOverlay,
  `browsers` on HelloReply
- `tui/src/main.rs` — `--browser` CLI flag, forwarded to overlay/devtools
  messages
- `tui/src/ipc.rs` — `browser` parameter on `send_set_overlay()` and
  `send_set_devtools_overlay()`
- `gui/src/apprt/xpc.zig` — Browser registry (`browser_paths` map), composite
  `(profile, browser)` server keys, `resolveBrowserPath()`,
  `initBrowserRegistry()`, `spawnServerProcess()` with dynamic binary path
- `gui/src/protobuf/termsurf.pb-c.{h,c}` — Regenerated for new proto fields

## Ideas for future experiments

These are rough ideas for after Plusium is working end-to-end. Each will be
designed when the previous one is complete.

1. **End-to-end Plusium verification** — Once `--hidden` is fixed, run the full
   test matrix: browse, navigate, resize, mouse input, keyboard input, scroll,
   DevTools, dark mode, multiple profiles. Verify Plusium is functionally
   equivalent to the Chromium Profile Server.

2. **Build Roamium (Rust)** — Create a Rust crate that links
   `libtermsurf_content` via FFI (`bindgen` or manual declarations). The main
   challenge is build system integration: Cargo needs to find the Chromium-built
   static library and headers. Verify equivalence.

3. **Build Zoomium (Zig)** — Create a Zig package that links
   `libtermsurf_content` via `@cImport`. Same build system challenge as Roamium
   but for Zig. Verify equivalence.

4. **Make Roamium the default** — Once all three work, switch the default from
   Chromium Profile Server to Roamium. Update the GUI's `initBrowserRegistry()`
   to list Roamium first.

5. **Retire the Chromium Profile Server** — Delete `chromium_profile_server/`
   from the active Chromium branch once all three bindings are verified
   equivalent. This removes ~100 forked files and ~1050 lines of
   TermSurf-specific code.

## Experiments

### Experiment 1: Add `--hidden` support to stock Content Shell

Patch `content/shell/browser/shell_platform_delegate_mac.mm` to recognize the
`--hidden` flag. This is the same `setAlphaValue:0` +
`orderWindow:NSWindowBelow` trick the Profile Server uses, applied to the stock
file that Plusium links against.

#### What to change

**`content/shell/common/shell_switches.h`** — Add:

```cpp
inline constexpr char kHidden[] = "hidden";
```

**`content/shell/browser/shell_platform_delegate_mac.mm`** — In the function
that shows the window (the line `[window makeKeyAndOrderFront:nil]`), wrap it:

```objc
if (base::CommandLine::ForCurrentProcess()->HasSwitch(switches::kHidden)) {
    [window setAlphaValue:0.0];
    [window orderWindow:NSWindowBelow relativeTo:0];
} else {
    [window makeKeyAndOrderFront:nil];
}
```

Also add the `#include` for `shell_switches.h` and `base/command_line.h` if not
already present.

#### Verification

1. `autoninja -C out/Default plusium` — compiles.
2. Run `web google.com --browser plusium` — no Content Shell window appears on
   screen, page loads in the terminal.
3. Verify default browser (no `--browser` flag) still works.

#### Result: Failure

The `--hidden` patch works — no Content Shell window appears on screen. But
Plusium's child processes (renderer, GPU) crash with:

```
FATAL:content/app/content_main_runner_impl.cc:1002]
Check failed: sandbox::Seatbelt::IsSandboxed().
```

Chromium's multi-process architecture on macOS requires child processes to be
sandboxed via `Seatbelt`. The sandbox profile is read from the app bundle's code
signature. Plusium is a bare executable with no bundle, so child processes can't
initialize the sandbox and crash.

The existing Profile Server avoids this because it's packaged as
`Chromium Profile Server.app` — a signed `.app` bundle with entitlements.

**Solution:** Pass `--no-sandbox` when spawning Plusium. Electron apps ship with
`--no-sandbox` by default — it's proven safe for embedders that don't need
Chromium's full browser-grade sandbox. TermSurf's use case (developers browsing
localhost and docs in a terminal) has a narrower attack surface than a
general-purpose browser.

### Experiment 2: Pass `--no-sandbox` to Plusium

Add `--no-sandbox` to the spawn args for non-bundled browser binaries. The GUI
already constructs the argument list in `spawnServerProcess()`. The simplest
approach: always pass `--no-sandbox` for all browser binaries (the Profile
Server's bundle entitlements override it, so it's harmless there).

#### What to change

**`gui/src/apprt/xpc.zig`** — In `spawnServerProcess()`, add `--no-sandbox` to
the argv array passed to the child process. It goes alongside `--hidden`,
`--enable-logging`, and `--log-file`.

#### Verification

1. `cd gui && zig build` — compiles.
2. Run `web google.com --browser plusium` — page loads in the terminal, no
   Content Shell window, no sandbox crash.
3. Verify default browser (no `--browser` flag) still works.
4. Check `~/.local/state/termsurf/chromium-server.log` — no sandbox errors.

#### Result: Failure (partial progress)

The `--no-sandbox` fix works — Plusium starts successfully. The GUI log
confirms:

```
[libtermsurf_content] Initialized, firing callback
DevTools listening on ws://127.0.0.1:56508/devtools/browser/...
```

No sandbox crash, no Content Shell window. Both the Experiment 1 (`--hidden`)
and Experiment 2 (`--no-sandbox`) fixes are working.

But the page still doesn't render. Plusium initializes but the TUI times out
waiting for a response. The IPC handshake (socket connect → ServerRegister →
CreateTab → TabReady → CaContext) is breaking somewhere downstream.

The GUI's Zig logs (`std.log.scoped(.ipc)`) don't appear in stdout/stderr — they
use Ghostty's internal logging system. Without these logs, we can't see whether:

- Plusium connected to the socket
- The GUI received the `ServerRegister` message
- The GUI matched it to a server entry
- The GUI sent `CreateTab`
- Plusium sent back `TabReady` / `CaContext`

The next experiment needs to add debug tracing to identify where the handshake
breaks.

### Experiment 3: Add debug traces to pinpoint IPC handshake failure

Add `std.debug.print` traces (raw stderr writes that bypass Ghostty's log
framework) at every step of the IPC handshake in the GUI, and `fprintf(stderr)`
traces in Plusium. The Zig `log.info` calls didn't appear in gui.log despite
stderr logging being the default — `std.debug.print` writes directly to fd 2 and
cannot be filtered.

#### What to change

**`gui/src/apprt/xpc.zig`** — Add `std.debug.print` at these points:

1. `spawnServerProcess()` — before and after `child.spawn()`
2. `handleSocketMessage()` — when a message arrives (print case number and
   connection type)
3. `handleSocketServerRegister()` — print the profile received, whether a
   matching server was found, and the server's composite key
4. `handleSetOverlay()` / `handleSetDevtoolsOverlay()` — print browser value and
   whether `getOrCreateServer()` succeeded
5. `getOrCreateServer()` — print the composite key lookup result

**`chromium/src/content/plusium/plusium_main.cc`** — Add `fprintf(stderr)` at:

1. `OnInitialized()` — after browser context creation, before/after socket
   connect, after sending ServerRegister
2. `SocketReaderLoop()` — when a message is received and dispatched
3. `HandleMessage()` — print the message type received

#### Verification

1. Both repos compile (`zig build` + `autoninja`).
2. Run with
   `open TermSurf-Debug.app --stdout ./logs/gui.log --stderr ./logs/gui.log`.
3. Run `web google.com --browser plusium`.
4. Read `logs/gui.log` — the traces will show exactly where the handshake stops.
5. Remove debug traces after diagnosis.

#### Result: Success

The debug traces revealed the IPC handshake is **mostly working**. The full
chain completes up to a point:

1. TUI → GUI: hello (case=23) and set_overlay (case=19) arrive correctly.
2. GUI creates server, spawns Plusium — Plusium starts, creates browser context,
   connects to the GUI's Unix socket, sends ServerRegister.
3. GUI receives ServerRegister (case=12), matches it to the spawned server,
   flushes 1 pending tab by sending CreateTab.
4. Plusium receives CreateTab (case=1) and calls `ts_create_web_contents()`.
5. Plusium sends back ca_context (14), url_changed (15), loading_state (16),
   title_changed (17) — all arrive at the GUI.

**The bug: case=13 (tab_ready) is never sent.** Plusium sends ca_context (14)
but never sends tab_ready (13). The `OnTabReady` callback assigns the `tab_id`
to the tab entry, and tab_ready carries the `tab_id` + `pane_id` back to the
GUI. Without it, the GUI can't associate the ca_context with the right pane —
the ca_context message has `tab_id=0`.

The `OnTabReady` callback in `libtermsurf_content` is either not firing, or
`FindByHandle()` fails because the handle hasn't been stored in `g_tabs` yet
(race between `ts_create_web_contents` returning and the callback firing).

### Experiment 4: Diagnose missing tab_ready

Add `fprintf(stderr)` traces to the three callback functions in
`plusium_main.cc` that interact with `FindByHandle()`. The goal is to determine
whether `OnTabReady` fires at all, and if so, whether `FindByHandle()` returns
null because the tab entry hasn't been pushed to `g_tabs` yet.

The leading theory: `ts_create_web_contents()` fires `OnTabReady`
**synchronously** (on the same call stack), before the `push_back` on the next
line. So `FindByHandle(wc)` searches `g_tabs` before the entry exists and
silently returns null.

#### What to change

**`chromium/src/content/plusium/plusium_main.cc`** — Add `fprintf(stderr)` at:

1. `kCreateTab` handler — print the handle returned by
   `ts_create_web_contents()` before and after `push_back`, and the current
   `g_tabs` size at each point.
2. `OnTabReady()` — print the handle received, `g_tabs` size, and whether
   `FindByHandle()` succeeded.
3. `OnCaContextId()` — same: print handle, `g_tabs` size, and `FindByHandle()`
   result.

#### Verification

1. `autoninja -C out/Default plusium` — compiles.
2. Run `web google.com --browser plusium` with GUI logs redirected.
3. Read `logs/gui.log` — the traces will show whether `OnTabReady` fires and
   whether `FindByHandle` finds the entry.
4. If `OnTabReady` fires with `g_tabs` size=0 (before `push_back`), the fix is
   to push the entry before calling `ts_create_web_contents` and update the
   handle afterward.

#### Result: Success

The timing theory is confirmed. The log shows the exact sequence:

1. `kCreateTab`: `g_tabs size=0` BEFORE `ts_create_web_contents`
2. `OnTabReady`: fires **synchronously** during `ts_create_web_contents` —
   `g_tabs size=0`, `FindByHandle FAILED — handle not in g_tabs`
3. `kCreateTab`: `g_tabs size=0` AFTER `ts_create_web_contents`, BEFORE
   `push_back`
4. `kCreateTab`: `g_tabs size=1` AFTER `push_back` — too late
5. `OnCaContextId`: fires later (async), `g_tabs size=1`,
   `FindByHandle succeeded` — but `tab_id=0` because `OnTabReady` never set it

`OnTabReady` fires synchronously on the same call stack during
`ts_create_web_contents()`, before the entry is pushed to `g_tabs`. So
`FindByHandle()` searches an empty vector and silently returns. The ca_context
arrives later (async) and finds the entry, but with `tab_id=0` because
`OnTabReady` never ran.

**Fix:** Push the entry to `g_tabs` before calling `ts_create_web_contents()`,
then update the handle afterward.

### Experiment 5: Fix tab_ready timing bug

Push the `TabEntry` to `g_tabs` **before** calling `ts_create_web_contents()`,
so that `OnTabReady` (which fires synchronously during `ts_create_web_contents`)
can find it via `FindByHandle()`. The handle field is set to a sentinel
(`nullptr`) initially, then updated after `ts_create_web_contents` returns.

The same bug exists in `kCreateDevtoolsTab` — fix both.

#### What to change

**`chromium/src/content/plusium/plusium_main.cc`** — In `kCreateTab`:

```cpp
case termsurf::TermSurfMessage::kCreateTab: {
  auto& m = msg->create_tab();
  // Push entry FIRST so OnTabReady can find it.
  TabEntry entry;
  entry.pane_id = m.pane_id();
  g_tabs->push_back(std::move(entry));
  TabEntry& ref = g_tabs->back();
  // OnTabReady fires synchronously here — ref is already in g_tabs.
  ref.handle = ts_create_web_contents(
      g_browser_context, m.url().c_str(),
      static_cast<int>(m.pixel_width()),
      static_cast<int>(m.pixel_height()),
      m.dark());
  break;
}
```

Apply the same pattern to `kCreateDevtoolsTab`.

Also update `FindByHandle()` to skip entries with `handle == nullptr` (the
sentinel), so an in-flight creation doesn't match a stale lookup.

#### Verification

1. `autoninja -C out/Default plusium` — compiles.
2. Run `web google.com --browser plusium` with GUI logs redirected.
3. Read `logs/gui.log` — `OnTabReady` should now succeed
   (`FindByHandle succeeded`), and `OnCaContextId` should report a non-zero
   `tab_id`.
4. The page should render in the terminal.

#### Result: Success

The page renders in the terminal. The fix:

1. Push `TabEntry` (with `handle = nullptr`) to `g_tabs` **before** calling
   `ts_create_web_contents()`, so the entry exists when `OnTabReady` fires
   synchronously.
2. `OnTabReady` tries `FindByHandle(wc)` first (async case), then falls back to
   finding the entry with `handle == nullptr` (sync case) and assigns the handle
   immediately.
3. `FindByHandle()` skips `nullptr` entries so stale lookups don't match.
4. Same push-first pattern applied to `kCreateDevtoolsTab`.

Plusium now completes the full IPC handshake: ServerRegister → CreateTab →
tab_ready → ca_context → page renders.

### Experiment 6: Fix dark mode in Plusium

Dark mode is broken in two ways: (1) pages don't respect system dark mode on
load, and (2) `:colorscheme dark` (`c d`) has no effect. Both have the same root
cause.

The GUI and Plusium correctly pass the `dark` flag through the full chain. The
`TsBrowserMainParts` stores it in `tab->preferred_color_scheme`. But when
Chromium calls `OverrideWebPreferences()` to apply the setting, the base
`ShellContentBrowserClient` implementation runs — which is hardcoded to check
`--force-dark-mode` and default to light. It never reads
`tab->preferred_color_scheme`.

The Profile Server solved this with its own forked
`ShellContentBrowserClient::OverrideWebPreferences` that calls
`main_parts->GetColorSchemeForWebContents(web_contents)`. Since
`libtermsurf_content` uses `TsBrowserClient` (which extends
`ShellContentBrowserClient`), the fix is to:

1. Add `GetColorSchemeForWebContents()` to `TsBrowserMainParts`
2. Override `OverrideWebPreferences()` in `TsBrowserClient`

#### What to change

**`content/libtermsurf_content/ts_browser_main_parts.h`** — Add public method:

```cpp
std::optional<blink::mojom::PreferredColorScheme>
GetColorSchemeForWebContents(WebContents* web_contents) const;
```

**`content/libtermsurf_content/ts_browser_main_parts.cc`** — Implement it:

```cpp
std::optional<blink::mojom::PreferredColorScheme>
TsBrowserMainParts::GetColorSchemeForWebContents(
    WebContents* web_contents) const {
  for (const auto& tab : tabs_) {
    if (tab->shell && tab->shell->web_contents() == web_contents) {
      return tab->preferred_color_scheme;
    }
  }
  return std::nullopt;
}
```

**`content/libtermsurf_content/ts_browser_client.h`** — Add override:

```cpp
void OverrideWebPreferences(
    WebContents* web_contents,
    SiteInstance& main_frame_site,
    blink::web_pref::WebPreferences* prefs) override;
```

**`content/libtermsurf_content/ts_browser_client.cc`** — Implement it (same as
Profile Server's version):

```cpp
void TsBrowserClient::OverrideWebPreferences(
    WebContents* web_contents,
    SiteInstance& main_frame_site,
    blink::web_pref::WebPreferences* prefs) {
  auto* main_parts = static_cast<TsBrowserMainParts*>(
      shell_browser_main_parts());
  if (main_parts) {
    auto scheme = main_parts->GetColorSchemeForWebContents(web_contents);
    if (scheme.has_value()) {
      prefs->preferred_color_scheme = scheme.value();
    } else {
      prefs->preferred_color_scheme =
          blink::mojom::PreferredColorScheme::kDark;
    }
  }
}
```

#### Verification

1. `autoninja -C out/Default plusium` — compiles.
2. Set system to dark mode. Run `web google.com --browser plusium` — page should
   load in dark mode.
3. Run `:colorscheme light` (`c l`) — page should switch to light mode.
4. Run `:colorscheme dark` (`c d`) — page should switch back to dark mode.
5. Verify default browser (no `--browser` flag) still works.

#### Result: Success

Dark mode works. Both system dark mode on page load and the `:colorscheme`
command (`c d` / `c l`) now correctly control the page color scheme. The fix
overrides `ShellContentBrowserClient::OverrideWebPreferences` in
`TsBrowserClient` to read the per-tab `preferred_color_scheme` stored in
`TsBrowserMainParts`, instead of falling through to the base class
implementation that hardcodes light mode.

### Experiment 7: Diagnose missing cursor changes in Plusium

Hovering over links doesn't change the cursor to a pointing hand. This used to
work with the Profile Server. The full code path from Chromium to GUI is wired:

1. `RenderWidgetHostImpl::SetCursor()` fires `cursor_changed_callback_`
2. `TsTabObserver::OnCursorChanged()` calls `TsNotifyCursorChanged()`
3. `g_on_cursor_changed` global callback fires in `plusium_main.cc`
4. `OnCursorChanged()` calls `FindByHandle()`, builds protobuf, sends over
   socket
5. GUI receives case 18, calls `handleSocketCursorChanged()` →
   `handleCursorChanged()` → sets `surface.overlay_cursor_type`
6. `cursorPosCallback()` reads `overlay_cursor_type` and applies cursor shape

Code inspection found no obvious bug. Add debug traces at each stage to find
where the chain breaks.

#### What to change

**`content/plusium/plusium_main.cc`** — Add `fprintf(stderr)` to
`OnCursorChanged`:

```cpp
static void OnCursorChanged(ts_web_contents_t wc, int cursor_type, void*) {
  fprintf(stderr, "[DEBUG] Plusium OnCursorChanged: handle=%p cursor_type=%d\n",
          (void*)wc, cursor_type);
  TabEntry* t = FindByHandle(wc);
  if (!t) {
    fprintf(stderr, "[DEBUG] Plusium OnCursorChanged: FindByHandle FAILED\n");
    return;
  }
  fprintf(stderr, "[DEBUG] Plusium OnCursorChanged: tab_id=%d\n", t->tab_id);
  // ... rest unchanged ...
}
```

**`gui/src/apprt/xpc.zig`** — Add `std.debug.print` to three points:

1. `handleSocketCursorChanged()` — confirm message arrives:

```zig
std.debug.print("[DEBUG] handleSocketCursorChanged: tab_id={} cursor_type={}\n",
    .{ m.tab_id, m.cursor_type });
```

2. `handleCursorChanged()` — confirm pane lookup succeeds:

```zig
std.debug.print("[DEBUG] handleCursorChanged: tab_id={} cursor_type={} pane_found={}\n",
    .{ tab_id, cursor_type, panes.get(pane_id) != null });
```

3. `cursorPosCallback()` inside the overlay forwarding block — confirm cursor
   type is read:

```zig
std.debug.print("[DEBUG] cursorPosCallback: overlay_cursor_type={}\n",
    .{ self.overlay_cursor_type });
```

#### Verification

1. `autoninja -C out/Default plusium` — compiles.
2. `cd gui && zig build` — compiles.
3. Open a webpage, hover over a link, check stderr for `[DEBUG]` traces.
4. The traces reveal which stage breaks:
   - No `OnCursorChanged` in Plusium → callback not firing (Chromium issue)
   - `FindByHandle FAILED` → handle mismatch
   - No `handleSocketCursorChanged` in GUI → socket delivery issue
   - No `handleCursorChanged` → protobuf parsing issue
   - `cursorPosCallback` shows `overlay_cursor_type=0` → value not persisted
   - `cursorPosCallback` shows correct type → cursor mapping or application
     issue

#### Result: No issue found

Cursor changes work correctly without the debug traces. Stashed all changes,
rebuilt both Plusium and GUI from clean state, and cursor still changes to a
pointing hand over links. The original report was a fluke — likely a stale
binary from before the Experiment 5 timing fix, which fixed `tab_id` assignment
and thereby fixed all tab-id-keyed notifications including cursor changes.

### Experiment 8: Comprehensive Plusium feature audit

Systematic manual test of every Plusium feature. Walk through each category
below and check off items as they pass. Use `web google.com --browser plusium`
to test. Each item maps to a protobuf message or C API function.

#### Checklist

**IPC handshake**

- [x] Plusium connects and registers (ServerRegister, case 12)
- [x] Page renders in pane (CreateTab → TabReady → CaContext pipeline)

**Navigation**

- [x] Type `web google.com` — page loads (Navigate, case 5)
- [x] Click a link — page navigates (Chromium-internal)
- [s] URL bar updates after navigation (UrlChanged, case 15)
- [x] Page title updates in tab bar (TitleChanged, case 17)
- [x] Loading indicator shows progress (LoadingState, case 16)
- [x] `Cmd+[` goes back (KeyEvent → Chromium Cmd+[ handler)
- [x] `Cmd+]` goes forward
- [x] `Cmd+R` reloads

**Rendering**

- [x] Page renders at correct size (CaContext, case 14)
- [x] Resize window — page resizes to match (Resize, case 3)
- [x] Page renders at 60fps (no stuttering or frame drops)

**Mouse input**

- [x] Click on page elements (MouseEvent, case 6)
- [x] Hover over links — cursor changes to pointing hand (CursorChanged,
      case 18)
- [x] Hover over text — cursor changes to I-beam
- [x] Drag to select text (MouseMove, case 7)
- [x] Scroll with trackpad/mouse wheel (ScrollEvent, case 8)
- [x] Momentum scrolling works (phase/momentum_phase)

**Keyboard input**

- [x] Type in search box / form field (KeyEvent, case 9)
- [x] Special keys work: Tab, Enter, Backspace, arrow keys
- [x] `Cmd+A` selects all text
- [x] `Cmd+C` copies selected text
- [x] `Cmd+V` pastes from clipboard
- [x] `Cmd+X` cuts selected text
- [x] `Cmd+Z` undoes

**Focus**

- [x] Click on pane — browser receives focus (FocusChanged, case 10)
- [x] Switch to terminal pane — browser loses focus
- [x] Text input only works in focused pane

**Color scheme**

- [x] System dark mode: page loads in dark mode (SetColorScheme on CreateTab)
- [x] `:colorscheme dark` (`c d`) — page switches to dark (SetColorScheme,
      case 11)
- [x] `:colorscheme light` (`c l`) — page switches to light

**DevTools**

- [ ] `:devtools` opens DevTools in split pane (CreateDevtoolsTab, case 2)
- [ ] DevTools renders and is interactive
- [ ] DevTools shows correct page inspection
- [ ] Close DevTools pane — DevTools tab closes (CloseTab, case 4)

**Tab lifecycle**

- [x] Close TUI pane — browser tab closes (CloseTab, case 4)
- [x] Open second `web` pane — second tab works independently
- [x] Close all panes — Chromium process exits

**Multi-pane**

- [x] Two browser panes render simultaneously
- [x] Each pane has independent navigation
- [x] Click-to-focus switches active pane (Issue 670)
- [x] Active pane indicator visible (Issue 669)

**TUI features**

- [x] URL bar displays current URL
- [x] Mode indicator shows current mode (Normal/Browse/Edit)
- [x] `Esc` returns from Browse to Normal mode
- [x] `i` enters Edit mode from Normal mode
- [x] Overlay position and size correct (SetOverlay, case 19)

**Queries (internal, verify indirectly)**

- [x] Homepage loads on first `web` command (HelloRequest/Reply)
- [x] Second `web` command reuses existing Chromium server
      (QueryLastRequest/Reply)

#### Verification

Walk through the checklist in order. For each item, mark pass or fail. Record
any failures with a short description of the observed behavior.

#### Result

Everything passes except DevTools. See Experiment 9.

### Experiment 9: Fix DevTools for Plusium

DevTools doesn't work with Plusium. The `:devtools` command fails because
Plusium's copy of `termsurf.proto` is out of date — it's missing the `browser`
field that was added to the canonical proto.

#### Root cause

There are three copies of `termsurf.proto`:

1. **`proto/termsurf.proto`** (canonical) — has `string browser = 9` in both
   `SetOverlay` and `SetDevtoolsOverlay`
2. **`chromium/src/content/plusium/termsurf.proto`** (Plusium) — **missing**
   `browser` field in both messages
3. **`chromium/src/content/chromium_profile_server/browser/termsurf.proto`**
   (Profile Server) — separate copy

The GUI's protobuf-c code is generated from the canonical proto, so it correctly
serializes `browser`. But when Plusium deserializes incoming messages, it
silently ignores the unknown field. This doesn't break normal page loads (the
browser is determined by which binary the GUI launched), but it means Plusium
can't see the `browser` field in any message.

More importantly, the DevTools auto-target path in the GUI
(`handleSetDevtoolsOverlay`, line 472) uses `target.server` directly — the
inspected pane's server. This is actually correct for auto-targeting since
DevTools should go to the same Chromium process as the inspected tab. But the
`QueryDevtoolsRequest` proto doesn't include a `browser` field, so validation
can't check browser-specific constraints.

The fix is straightforward: sync Plusium's proto with the canonical version.

#### What to change

1. **Sync `chromium/src/content/plusium/termsurf.proto`** — copy `SetOverlay`
   and `SetDevtoolsOverlay` definitions from `proto/termsurf.proto` to add
   `string browser = 9` to both messages.

2. **Add debug traces** to diagnose DevTools flow in Plusium:

   **`content/plusium/plusium_main.cc`** — Add `fprintf(stderr)` to the
   `kCreateDevtoolsTab` handler (case 2):

   ```cpp
   fprintf(stderr, "[DEBUG] kCreateDevtoolsTab: pane=%s inspected=%d dark=%d\n",
           m.pane_id().c_str(), m.inspected_tab_id(), m.dark());
   ```

   **`gui/src/apprt/xpc.zig`** — Add `std.debug.print` to
   `handleSetDevtoolsOverlay` at the server selection point:

   ```zig
   std.debug.print("[DEBUG] devtools server selection: auto_targeted={} server_fd={}\n",
       .{ p.inspected_tab_id != inspected_tab_id,
          if (p.server) |s| s.fd else -1 });
   ```

#### Result: Partial — proto synced, root cause found

Syncing the proto fixed deserialization, and debug traces revealed the real bug.
The logs show:

```
profile=test browser=plusium → spawns Plusium pid 53437 (server key: test)
DevTools: profile=default browser=plusium inspected_tab_id=1
  → auto_targeted=false (tab_id already resolved)
  → getOrCreateServer("default", "plusium") → NEW server (wrong!)
  → spawns Plusium pid 53460 (server key: default)
  → FindByTabId(1) FAILED — tab_id=1 only exists in pid 53437
```

Two bugs:

1. **Wrong server for DevTools.** The explicit path in
   `handleSetDevtoolsOverlay` calls `getOrCreateServer(profile, browser)` using
   the DevTools TUI's args (`profile=default`), not the inspected tab's server.
   This spawns a new Chromium process that has no tabs.

2. **`tab_to_pane` collision.** The map is keyed by bare `tab_id` (an `i64`).
   Since each Chromium process starts tab IDs at 1, two servers can have
   `tab_id=1`. The second `put` overwrites the first.

See Experiment 10 for the fix.

### Experiment 10: Fix DevTools server routing

DevTools fails because `handleSetDevtoolsOverlay` routes `CreateDevtoolsTab` to
the wrong Chromium process. The DevTools TUI runs as `web devtools` with no
`--browser` or `--profile` flags, so it defaults to
`profile=default, browser=""`. The GUI then calls
`getOrCreateServer("default", "")`, which spawns a new Chromium process that has
no tabs — so `FindByTabId` fails.

DevTools must always run in the same Chromium process as the inspected tab. The
inspected tab's pane already stores a pointer to its server. So instead of
calling `getOrCreateServer` with the DevTools TUI's args, look up the inspected
tab's pane via `tab_to_pane` and use its server directly.

#### What to change

**`gui/src/apprt/xpc.zig`** — In `handleSetDevtoolsOverlay`, replace the entire
server selection block (the
`if (auto_targeted) { ... } else if (getOrCreateServer(...)) { ... }` block)
with a single path that always looks up the inspected tab's server:

```zig
// DevTools must run in the same Chromium process as the inspected tab.
// Look up the inspected tab's pane and use its server.
const inspected_pane_id = tab_to_pane.get(p.inspected_tab_id) orelse {
    log.err("devtools: inspected tab_id={d} not found", .{p.inspected_tab_id});
    cleanupPane(pane_id_key);
    return;
};
const inspected_pane = panes.get(inspected_pane_id) orelse {
    log.err("devtools: inspected pane {s} not found", .{inspected_pane_id});
    cleanupPane(pane_id_key);
    return;
};
if (inspected_pane.server) |target_server| {
    p.server = target_server;
    target_server.pane_count += 1;
    if (target_server.fd >= 0) {
        sendCreateDevToolsTab(p, target_server);
        if (p.browsing) {
            sendFocusChanged(p.pane_id_key, true);
        }
    }
}
```

**`tui/src/main.rs`** — Revert the `--browser` flag in the DevTools split
command. The command should always be `web devtools` — the GUI handles server
routing:

```rust
let cmd = format!("{} devtools", current_exe);
```

#### Verification

1. `cd gui && zig build` — compiles.
2. Open `web google.com --browser plusium`, then `:devtools` — DevTools opens.
3. Open `web google.com` (default browser), then `:devtools` — still works.
4. Check logs: DevTools uses the same server fd as the inspected tab.
