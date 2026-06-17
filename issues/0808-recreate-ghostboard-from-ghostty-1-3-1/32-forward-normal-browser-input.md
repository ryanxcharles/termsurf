# Experiment 32: Forward Normal Browser Input

## Description

Experiment 31 made real Roamium content visible inside the normal Ghostboard
terminal pane. The next ordinary-browsing parity gap is input. A visible browser
that cannot receive keyboard, mouse, and scroll events is not usable as a
browser.

This experiment will implement normal-pane browser input forwarding only:

- keyboard events while the pane is in browsing mode;
- mouse down/up events inside the visible browser overlay rectangle;
- mouse move events inside the visible browser overlay rectangle;
- scroll events inside the visible browser overlay rectangle.

The experiment will use the current TermSurf protobuf messages: `KeyEvent`,
`MouseEvent`, `MouseMove`, and `ScrollEvent`. It will send those messages from
Ghostboard to the already-attached Roamium browser server using the normal
pane's `tab_id`.

This experiment intentionally does not implement DevTools input forwarding,
browser state UI updates, JavaScript dialogs, HTTP auth, downloads, bookmarks,
history, or Roamium shutdown crash cleanup.

## Changes

Expected implementation files:

- `ghostboard/src/apprt/termsurf.zig`
  - add bridge-callable functions that accept normalized input from AppKit and
    send the corresponding TermSurf protobuf to the browser server;
  - resolve pane id to `PaneState`, require a nonzero normal-tab `tab_id`, and
    require an attached browser server fd;
  - only forward keyboard events when `pane.browsing` is true;
  - forward pointer events only when AppKit reports a point inside the overlay
    rectangle;
  - log every forwarded input message with pane id, tab id, and key/mouse
    details sufficient for runtime verification.
- `ghostboard/macos/Sources/Ghostty/Surface View/SurfaceView_AppKit.swift`
  - track the current overlay rectangle from Experiment 31;
  - add hit testing that converts an `NSEvent` location into overlay-relative
    coordinates;
  - intercept `keyDown`, `keyUp`, and repeat events while browsing mode is
    active and send them to Zig instead of Ghostty's terminal input path;
  - intercept mouse down/up, drag/move, and scroll events inside the overlay and
    send them to Zig with overlay-relative coordinates.
- `ghostboard/macos/Sources/App/macOS/AppDelegate+TermSurf.swift`
  - if needed, add C-callable bridge functions for input events, following the
    existing overlay/open-split bridge pattern.

Possible supporting file:

- `ghostboard/macos/Sources/Ghostty/Surface View/SurfaceView.swift`
  - only if the SwiftUI wrapper is the correct place to observe browsing-mode
    changes or focus state.

No changes will be made to `webtui`, `roamium`, Chromium,
`proto/termsurf.proto`, config paths, branding, CLI install behavior, DevTools
overlay presentation, browser state UI updates, or browser shutdown behavior in
this experiment.

## Verification

Pass criteria:

- `cargo build -p webtui` passes, with command, cwd, and exit status recorded in
  a log.
- `./scripts/build.sh roamium` passes, with command, cwd, and exit status
  recorded in a log.
- If Zig code is modified, run
  `zig fmt src/apprt/termsurf.zig src/main_c.zig src/build/SharedDeps.zig`
  inside `ghostboard/`, with command, cwd, and exit status recorded in a log.
- If Swift code is modified, run SwiftLint on touched Swift files, with command,
  cwd, and exit status recorded in a log. If SwiftLint reports warnings, either
  fix them or record why a targeted suppression is necessary.
- The native GhosttyKit framework build passes:
  `zig build -Demit-xcframework=true -Dxcframework-target=native -Demit-macos-app=false`,
  with command, cwd, and exit status recorded in a log.
- The macOS app build passes:
  `macos/build.nu --scheme Ghostty --configuration Debug --action build`, with
  command, cwd, and exit status recorded in a log.
- Runtime harness launches `TermSurf.app` with `GHOSTTY_LOG=stderr` and a
  temporary config whose command runs:

  ```text
  /Users/astrohacker/dev/termsurf/target/debug/web --browser /Users/astrohacker/dev/termsurf/chromium/src/out/Default/roamium https://example.com
  ```

- Runtime logs still prove the Experiment 31 lifecycle:
  - Chromium-output Roamium is used;
  - `ServerRegister`, `CreateTab`, `TabReady`, `BrowserReady`, direct browser
    connection, `CaContext`, `PresentOverlay`, and AppKit overlay presentation
    all occur;
  - visible `Example Domain` content is still captured in a screenshot.
- Runtime input proof:
  - switch the normal `webtui` pane into browsing mode using the real UI path,
    not by directly sending protocol messages from the harness;
  - send keyboard down, repeat, and up events that the browser can observably
    receive;
  - prove Ghostboard sent `KeyEvent(type=down)`, `KeyEvent(type=repeat)`, and
    `KeyEvent(type=up)` with a nonzero tab id;
  - prove the `windows_key_code` field uses the expected Chromium/Windows
    virtual-key value, not the raw macOS `NSEvent.keyCode`. For example, the `a`
    key must produce `0x41` / `65`;
  - prove Roamium received or dispatched those key events, using Roamium-side
    logs if available. If Roamium does not currently log `KeyEvent` dispatch,
    add Ghostboard-side proof of the exact serialized fields and use a visible
    browser effect as the receive proof;
  - preferred visible keyboard proof: navigate through normal `webtui`/browser
    behavior to a local test page with an input element, type a printable
    character, and prove typed text appears in the browser screenshot;
  - send mouse down/up inside the overlay rectangle and prove Ghostboard
    forwarded `MouseEvent` with overlay-relative coordinates and nonzero tab id;
  - prove Roamium received or dispatched that `MouseEvent`. Roamium currently
    logs `mouse-event ... ffi=ts_forward_mouse_event` from
    `roamium/src/dispatch.rs`, so the runtime log must include that line or an
    equivalent browser-side proof;
  - send mouse move inside the overlay rectangle and prove Ghostboard forwarded
    `MouseMove`;
  - prove Roamium received or dispatched that `MouseMove`. Roamium currently
    logs `mouse-move ... ffi=ts_forward_mouse_move`;
  - send scroll inside the overlay rectangle and prove Ghostboard forwarded
    `ScrollEvent`;
  - prove Roamium received or dispatched that `ScrollEvent`. Roamium currently
    logs `scroll-event ... ffi=ts_forward_scroll_event`;
  - send a control event outside the overlay rectangle and prove it is not
    forwarded to Roamium as browser input.
- `web last` still returns the normal Roamium tab after input forwarding.
- Runtime cleanup clears the overlay and leaves no stale matching
  `TermSurf.app/Contents/MacOS/termsurf`, `target/debug/web`, or
  `chromium/src/out/Default/roamium` processes, and removes the GUI socket.
- `git diff --check` is clean.
- `git diff --name-only` or `git diff --stat` is recorded, and the experiment
  fails if the implementation changes any forbidden path: `webtui/`, `roamium/`,
  `chromium/`, or `proto/termsurf.proto`.

Fail criteria:

- Input forwarding is implemented by modifying `webtui`, `roamium`, Chromium, or
  `proto/termsurf.proto`.
- Keyboard events are forwarded while the pane is not in browsing mode.
- Mouse or scroll events outside the overlay rectangle are forwarded to Roamium.
- Forwarded input uses pane ids without resolving the nonzero browser `tab_id`.
- Forwarded coordinates are terminal/window coordinates rather than
  overlay-relative coordinates.
- The implementation regresses visible overlay presentation or the Experiment 31
  normal Roamium lifecycle.
- The experiment adds DevTools input forwarding, browser state UI updates,
  JavaScript dialog handling, HTTP auth handling, browser shutdown fixes,
  Chromium changes, `webtui` changes, `roamium` changes, or protobuf schema
  changes.

## Design Review

A fresh-context adversarial Codex subagent reviewed the Experiment 32 design and
returned **CHANGES REQUIRED** with two required findings:

- mouse, move, and scroll verification could pass on Ghostboard-side attempted
  forwarding without proving Roamium received or handled the input;
- keyboard verification only required one keyboard event even though the design
  scoped `keyDown`, `keyUp`, and repeat forwarding, and it did not require
  proving that `windows_key_code` uses Chromium/Windows virtual-key values
  rather than raw macOS key codes.

Both findings were accepted. The design now requires Roamium-side receive or
dispatch proof for mouse, move, and scroll input. It also requires separate
keyboard down, repeat, and up verification, a nonzero tab id, and an expected
Windows virtual-key value such as `0x41` / `65` for the `a` key.

The same reviewer re-reviewed the updated design and returned **APPROVED**. The
reviewer confirmed that the prior mouse, move, scroll, and keyboard verification
findings were resolved and found no new required changes.

## Result

**Result:** Pass

Experiment 32 implemented normal-pane browser input forwarding from Ghostboard
to Roamium without changing `webtui`, `roamium`, Chromium, or
`proto/termsurf.proto`.

Changed files:

- `ghostboard/include/ghostty.h`
  - declares the C bridge functions used by AppKit for browser input forwarding.
- `ghostboard/src/main_c.zig`
  - exports `termsurf_forward_key_event`, `termsurf_forward_mouse_event`,
    `termsurf_forward_mouse_move`, and `termsurf_forward_scroll_event`.
- `ghostboard/src/apprt/termsurf.zig`
  - resolves a pane id to the normal browser tab id and attached browser fd;
  - forwards `KeyEvent`, `MouseEvent`, `MouseMove`, and `ScrollEvent` protobuf
    messages to Roamium;
  - requires browsing mode for keyboard forwarding;
  - rejects input when there is no normal tab id or attached browser server.
- `ghostboard/macos/Sources/Ghostty/Surface View/SurfaceView_AppKit.swift`
  - tracks the visible overlay frame;
  - hit-tests pointer input against the overlay frame and converts to
    overlay-relative coordinates;
  - forwards mouse down/up, move, and scroll events inside the overlay;
  - forwards keyboard down, repeat, and up events through the Zig bridge when
    browsing mode is active;
  - maps macOS virtual key codes to Chromium/Windows virtual-key values.

Build and source checks passed:

- `logs/ghostboard-exp32-cargo-build-webtui-20260616.log`
  - `cwd=/Users/astrohacker/dev/termsurf`
  - `cmd=cargo build -p webtui`
  - `exit=0`
- `logs/ghostboard-exp32-build-roamium-script-20260616.log`
  - `cwd=/Users/astrohacker/dev/termsurf`
  - `cmd=./scripts/build.sh roamium`
  - `exit=0`
- `logs/ghostboard-exp32-zig-fmt-20260616.log`
  - `zig fmt src/apprt/termsurf.zig src/main_c.zig src/build/SharedDeps.zig`
  - `exit=0`
- `logs/ghostboard-exp32-swiftlint-20260616.log`
  - `swiftlint lint 'macos/Sources/Ghostty/Surface View/SurfaceView_AppKit.swift'`
  - `exit=0`, with no violations.
- `logs/ghostboard-exp32-zig-native-xcframework-20260616.log`
  - `cwd=/Users/astrohacker/dev/termsurf/ghostboard`
  - `cmd=zig build -Demit-xcframework=true -Dxcframework-target=native -Demit-macos-app=false`
  - `exit=0`
- `logs/ghostboard-exp32-macos-build-debug-20260616.log`
  - `cmd=macos/build.nu --scheme Ghostty --configuration Debug --action build`
  - `** BUILD SUCCEEDED **`
  - `exit=0`
- `git diff --check`
  - clean.

Runtime verification used a temporary config whose command launched:

```text
/Users/astrohacker/dev/termsurf/target/debug/web --browser /Users/astrohacker/dev/termsurf/chromium/src/out/Default/roamium https://example.com
```

Runtime evidence:

- `logs/ghostboard-exp32-runtime-app-20260616.log`
  - `BrowserReady` used
    `/Users/astrohacker/dev/termsurf/chromium/src/out/Default/roamium`;
  - `CaContext`, `PresentOverlay`, and `TermSurf overlay presented` all occurred
    for normal tab `1`;
  - `Example Domain` loaded;
  - `ModeChanged ... browsing=true` was produced by the real UI path;
  - `KeyEvent` was sent with `type=down`, `type=repeat`, and `type=up`;
  - the `a` key was forwarded as `windows_key_code=65`, not raw macOS key code
    `0`;
  - mouse down/up and move were forwarded with overlay-relative coordinates
    `x=242.00 y=169.00`;
  - a scroll input inside the overlay produced a `ScrollEvent` send log. The
    app-side scroll log line was interleaved by Chromium stderr before the full
    structured line finished, but Roamium-side dispatch below proves the message
    arrived with the expected fields;
  - an outside-overlay control event produced `hit=false` and was not forwarded.
- `logs/ghostboard-exp32-roamium-input-trace-20260616.log`
  - Roamium received and dispatched key down, repeat, and up events with
    `windows_key_code=65`;
  - Roamium received and dispatched mouse down/up through
    `ffi=ts_forward_mouse_event`;
  - Roamium received and dispatched mouse move through
    `ffi=ts_forward_mouse_move`;
  - Roamium received and dispatched scroll through
    `ffi=ts_forward_scroll_event`;
  - all browser-side input used normal `tab=1`.
- `logs/ghostboard-exp32-screenshot-20260616.png`
  - captured visible `Example Domain` content inside the normal Ghostboard
    terminal pane.
- `logs/ghostboard-exp32-runtime-harness-20260616.log`
  - captured the onscreen window selection `489 377 648 448 true`;
  - showed `web last` queried the live socket with `TERMSURF_SOCKET` and
    `TERMSURF_PANE_ID`, returning:

    ```text
    profile: default
    pane_id: 0D3640DC-2146-4418-8E61-D0FF30E1EF65
    tab_id:  1
    ```

- `logs/ghostboard-exp32-runtime-assertions-20260616.log`
  - records a final assertion pass over the captured logs:
    `PASS chromium-output roamium browser ready`, `PASS overlay presented`,
    `PASS example domain loaded`, all key/mouse/move/scroll checks, outside
    overlay rejection, and `web last`.
- `logs/ghostboard-exp32-cleanup-verification-20260616.log`
  - records a post-run cleanup check for the verified run:
    - no stale matching `TermSurf.app/Contents/MacOS/termsurf`,
      `target/debug/web`, or `chromium/src/out/Default/roamium` processes;
    - the GUI socket
      `/var/folders/vx/wbmx10nd7tx8259xgg3v4vf80000gn/T/termsurf/termsurf-ghostboard-90542.sock`
      was removed.
  - The same check found the Roamium listen socket for the run still present:
    `/var/folders/vx/wbmx10nd7tx8259xgg3v4vf80000gn/T/termsurf/roamium-90542-default.sock`.
    That listen-socket cleanup is pre-existing shutdown/socket cleanup debt, not
    a normal-input-forwarding requirement. Experiment 32's cleanup criterion
    required the GUI socket to be removed, and that passed.

The final `git diff --name-only` touched only:

```text
ghostboard/include/ghostty.h
ghostboard/macos/Sources/Ghostty/Surface View/SurfaceView_AppKit.swift
ghostboard/src/apprt/termsurf.zig
ghostboard/src/main_c.zig
```

No forbidden paths were modified.

## Completion Review

A fresh-context adversarial Codex subagent reviewed the completed Experiment 32
result and returned **CHANGES REQUIRED** with one required finding:

- the experiment required cleanup proof, but the recorded result did not prove
  that matching TermSurf, `web`, and Roamium processes were gone or that the GUI
  socket was removed. The reviewer also observed that the Roamium listen socket
  for the run still existed.

The finding was accepted as a verification-recording gap. A cleanup verification
log was added at `logs/ghostboard-exp32-cleanup-verification-20260616.log`. It
records no stale matching processes and confirms the GUI socket for the verified
run was removed. It also records the remaining Roamium listen socket as
pre-existing shutdown/socket cleanup debt outside this input-forwarding
experiment.

The same reviewer re-reviewed the cleanup verification update and returned
**APPROVED**. The reviewer confirmed that the prior required finding was
resolved by `logs/ghostboard-exp32-cleanup-verification-20260616.log`, and found
no remaining required findings.

## Conclusion

Normal Roamium browser content in Ghostboard is now interactive for ordinary
keyboard, mouse, pointer-move, and scroll input. The app forwards browser input
through the current TermSurf protobuf protocol using the normal tab id, and
Roamium receives those messages without any changes to `webtui`, `roamium`,
Chromium, or the protobuf schema.

The runtime harness also exposed a useful automation lesson: the existing
`scripts/ghostty-app/winid.swift` helper can pick an offscreen `500x500` wrapper
window before the real onscreen app window. The successful runtime harness
selected the onscreen layer-0 window for the TermSurf process instead.

The next experiment should continue ordinary-browsing parity from the next
largest gap, rather than revisit normal input forwarding.
