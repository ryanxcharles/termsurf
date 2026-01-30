# Issue 312: Profile Process Steals Focus on macOS

## Problem Statement

When opening a webview with the `web ...` command, a second instance of
wezterm-gui appears in the macOS dock. This second instance:

1. Has no visible window
2. Appears as a separate dock icon
3. Steals keyboard focus from the original terminal window
4. Forces the user to click back on the original window to continue typing

This makes the webview feature extremely frustrating to use, as every `web`
command interrupts the user's workflow.

## Research Findings

### App Bundle Structure

The built app has this structure:

```
wezterm-gui.app/
└── Contents/
    ├── MacOS/
    │   ├── wezterm-gui        ← Main executable
    │   ├── termsurf-profile   ← CEF browser process (PROBLEM)
    │   ├── web
    │   └── wezterm
    ├── Frameworks/
    │   ├── Chromium Embedded Framework.framework/
    │   ├── WezTerm Helper.app/           ← Has LSUIElement=1
    │   ├── WezTerm Helper (GPU).app/     ← Has LSUIElement=1
    │   ├── WezTerm Helper (Renderer).app/
    │   ├── WezTerm Helper (Plugin).app/
    │   └── WezTerm Helper (Alerts).app/
    └── XPCServices/
        └── com.termsurf.launcher.xpc/
```

### Key Discovery: LSUIElement

The WezTerm Helper apps include `LSUIElement=1` in their Info.plist:

```xml
<key>LSUIElement</key>
<string>1</string>
```

This tells macOS the process is a "UI Element" — a background process that
should not appear in the dock or receive focus.

### How termsurf-profile Gets Launched

The launcher (`termsurf-launcher/src/main.rs`) spawns termsurf-profile via
`std::process::Command`:

```rust
let mut cmd = Command::new(&profile_bin_path);
cmd.args(["--session-id", &session_id])
   .args(["--url", &url])
   // ...
cmd.spawn()
```

The path resolves to `wezterm-gui.app/Contents/MacOS/termsurf-profile`.

### Why Focus Gets Stolen

1. `termsurf-profile` is spawned as a child process
2. CEF initializes and creates an `NSApplication` instance internally
3. macOS sees a new process from inside an app bundle starting up
4. Without `LSUIElement=1` or explicit activation policy, macOS treats it as a
   regular app
5. macOS gives the new "app" focus (standard behavior for newly launched apps)
6. The original terminal window loses keyboard focus

### NSApplication Activation Policies

macOS supports three activation policies:

| Policy                                    | Dock Icon | Can Activate     | Menu Bar |
| ----------------------------------------- | --------- | ---------------- | -------- |
| `NSApplicationActivationPolicyRegular`    | Yes       | Yes              | Yes      |
| `NSApplicationActivationPolicyAccessory`  | No        | Yes (if clicked) | No       |
| `NSApplicationActivationPolicyProhibited` | No        | No               | No       |

The main wezterm-gui uses `NSApplicationActivationPolicyRegular` (in
`window/src/os/macos/connection.rs:37`):

```rust
ns_app.setActivationPolicy_(NSApplicationActivationPolicyRegular);
```

termsurf-profile has no activation policy code — CEF sets up NSApplication with
default behavior.

## Hypotheses

### Hypothesis A: Missing Activation Policy

termsurf-profile relies on CEF to initialize NSApplication, but CEF doesn't set
an appropriate activation policy for a headless/background renderer. Without
explicit configuration, macOS defaults to regular app behavior.

### Hypothesis B: Bundle Association

When a process runs from inside an app bundle's `Contents/MacOS/` directory,
macOS may associate it with that bundle. However, since termsurf-profile is a
separate binary (not the CFBundleExecutable), macOS might treat it as a new app
instance rather than a helper.

### Hypothesis C: CEF's NSApplication Initialization

CEF requires NSApplication for Cocoa event handling even in off-screen rendering
mode. When CEF calls `[NSApplication sharedApplication]` and runs its event
loop, macOS may interpret this as a new foreground application launching.

## Solution Options

### Option A: Set Activation Policy Programmatically (Recommended)

Add code to `termsurf-profile/src/main.rs` to set
`NSApplicationActivationPolicyProhibited` before CEF initializes:

```rust
#[cfg(target_os = "macos")]
fn set_background_activation_policy() {
    use objc::{class, msg_send, sel, sel_impl};
    unsafe {
        let ns_app: *mut objc::runtime::Object =
            msg_send![class!(NSApplication), sharedApplication];
        // NSApplicationActivationPolicyProhibited = 2
        let _: () = msg_send![ns_app, setActivationPolicy: 2i64];
    }
}

fn main() {
    #[cfg(target_os = "macos")]
    set_background_activation_policy();

    // ... rest of main
}
```

**Pros:**

- Simple code change
- No build system modifications
- Immediate effect

**Cons:**

- Must be called before CEF's NSApplication initialization
- Relies on objc crate (may need to add dependency)

### Option B: Bundle as Separate Helper App

Create `TermSurf Profile Helper.app` in the Frameworks folder with its own
Info.plist:

```
wezterm-gui.app/
└── Contents/
    └── Frameworks/
        └── TermSurf Profile Helper.app/
            └── Contents/
                ├── MacOS/
                │   └── termsurf-profile
                └── Info.plist  ← Contains LSUIElement=1
```

**Pros:**

- Follows macOS app bundle conventions
- LSUIElement is the "proper" way to declare background processes
- Works regardless of CEF's NSApplication behavior

**Cons:**

- Requires build script modifications
- More complex bundle structure
- Need to update launcher's path resolution

### Option C: Use LSBackgroundOnly in Main App

Add `LSBackgroundOnly=1` to the main app's Info.plist... but this would affect
wezterm-gui itself, making the terminal a background-only app. **Not viable.**

### Option D: Prevent CEF's NSApplication from Activating

CEF may have settings to control its NSApplication behavior. Research needed:

- `CefSettings` flags for background mode
- `--disable-features` command line flags
- Custom `CefApp` implementation that overrides activation

**Pros:**

- Addresses root cause within CEF

**Cons:**

- May not be possible with CEF's architecture
- Could break other CEF functionality

## Recommended Approach

**Start with Option A** (programmatic activation policy) as it's the simplest
fix with the least risk. If that doesn't work or has issues, fall back to
**Option B** (proper helper app bundling).

## Success Criteria

1. [ ] Opening a webview with `web ...` does not create a new dock icon
2. [ ] Opening a webview does not steal focus from the terminal window
3. [ ] Keyboard input continues working in the terminal after opening a webview
4. [ ] The webview still renders correctly (CEF functionality not broken)

## Files to Modify

| File                               | Changes                                    |
| ---------------------------------- | ------------------------------------------ |
| `ts3/termsurf-profile/src/main.rs` | Add activation policy code before CEF init |
| `ts3/termsurf-profile/Cargo.toml`  | Add objc/cocoa dependencies if needed      |

## References

- [Apple: LSUIElement](https://developer.apple.com/documentation/bundleresources/information_property_list/lsuielement)
- [Apple: NSApplicationActivationPolicy](https://developer.apple.com/documentation/appkit/nsapplication/activationpolicy)
- [CEF Forum: Background/Headless Mode](https://magpcss.org/ceforum/)
