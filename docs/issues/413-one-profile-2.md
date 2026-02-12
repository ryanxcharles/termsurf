# Issue 413: One Profile to Two Profiles

## Goal

Convert the One Profile app (a Content Shell clone running at 60fps) into a Two
Profiles app that renders two isolated browser profiles side by side in a single
window at 60fps. Each change is a separate experiment. When something breaks, we
fix it before moving on.

## Background

### How we got here

Issues 407–412 explored multiple approaches to rendering two Chromium
`BrowserContext` instances in one window:

- **Issue 407** proved in-process Chromium works: multiple profiles coexist with
  full isolation, and a single WebContents renders at 60fps via Content Shell.
  But placing two WebContents in one window dropped both to 2fps.
- **Issue 408** studied Electron's patches. Three throttling bypass patches
  target `Hide()`, `WasOccluded()`, and `WasHidden()`.
- **Issue 409** tried applying all 147 Electron patches. They can't build
  without Electron's full build infrastructure (Node.js, custom DEPS, etc.).
- **Issue 410** applied the three throttling patches in isolation. The bypass
  calls had no effect — both panes still rendered at 2fps.
- **Issue 410.2** added logging and discovered that `Hide()`, `WasOccluded()`,
  and `WasHidden()` are **never called** on either view. The entire throttling
  hypothesis was targeting the wrong code path.
- **Issue 411** attempted a deferred view attachment fix (wait for
  `RenderFrameCreated` before adding the view to the window). WebContents B
  never appeared, and Shell A was still 2fps — even though Shell A uses the
  exact same `Shell::CreateNewWindow` code path that Content Shell uses for
  60fps.
- **Issue 412** took a step back and cloned Content Shell as "One Profile."
  Confirmed it runs at 60fps. Established a known-good baseline.

### What we know

1. Content Shell runs at 60fps with a single profile.
2. One Profile (a Content Shell clone) runs at 60fps.
3. Two Profiles runs at 2fps — both panes, including Shell A which goes through
   the standard Content Shell lifecycle.
4. The throttling patches (Issues 408–410) are irrelevant — `Hide()`,
   `WasOccluded()`, and `WasHidden()` are never called.
5. The deferred view attachment (Issue 411) is irrelevant — Shell A is 2fps
   despite going through the standard lifecycle.
6. Something about the Two Profiles app's setup degrades Shell A's rendering.
   The candidates are: `TwoProfilesMainParts` class, `SHELL_DIR_USER_DATA`
   override, second `ShellBrowserContext`, second `WebContents`, or view
   hierarchy manipulation.

### The key architectural problem

In Content Shell and One Profile, **Chromium owns the window**. The `Shell`
class creates the NSWindow, manages the toolbar, and places the WebContents view
as the sole occupant of the content area. This works perfectly for one profile.

To render two profiles side by side, we need to **own the window ourselves** so
we can place two WebContents views into it. This is the single biggest
architectural change. Chromium's Shell class assumes one WebContents per window,
and its visibility tracking, compositor lifecycle, and platform delegate all
reflect this assumption.

## Branch

Create a new branch `146.0.7650.0-issue-413` in the `termsurf-chromium`
submodule, starting from the `146.0.7650.0-issue-412` branch (which has the One
Profile app at 60fps). Each experiment is a commit on this branch.

## Approach

Start from One Profile (60fps) and make one change at a time toward Two
Profiles. After each change, build and test. If fps drops, stop and fix before
proceeding. The changes, in order:

### Step 1: Override SHELL_DIR_USER_DATA

Add the `SHELL_DIR_USER_DATA` path override to point the profile at
`~/.config/termsurf/poc/profile-a`. This changes where Chromium stores profile
data but should not affect rendering.

**Expected: 60fps.**

### Step 2: Add second BrowserContext

Create a second `ShellBrowserContext` with a `SHELL_DIR_USER_DATA` override
pointing to `~/.config/termsurf/poc/profile-b`. Hold it but don't use it.

**Expected: 60fps.** If this drops to 2fps, creating a second BrowserContext
alone (possibly through the storage service crash that Issue 411 observed)
degrades Shell A.

### Step 3: Own the window

This is the critical step. Stop letting Chromium's `Shell` class own the window.
Instead, create the NSWindow ourselves in `InitializeMessageLoopContext` and
place Shell A's WebContents view into it. The `Shell` still creates its own
window (we can't easily prevent that), but we reparent the WebContents view into
our window.

This tests whether reparenting a single WebContents view out of its Shell-owned
window and into our own window breaks the compositor lifecycle. If it does, we
need to fix it before we can add a second profile.

**Expected: 60fps.** If this drops to 2fps, the view reparenting itself is the
problem and we need to fix the compositor lifecycle for reparented views.

### Step 4: Add second WebContents (no view attachment)

Create a second `WebContents` with `browser_context_b_` and navigate it to the
test page. Don't add its view to any window.

**Expected: 60fps.** If this drops to 2fps, the mere existence of a navigating
second WebContents degrades Shell A's rendering.

### Step 5: Attach second view side by side

Add WebContents B's view to our window, side by side with WebContents A.

**Expected: Both at 60fps.** If Shell A drops, the view hierarchy manipulation
is the cause. If Shell A stays at 60fps but Shell B is at 2fps, the visibility
race condition from Issue 411 applies to Shell B specifically and we need to fix
it (e.g., by deferring attachment until `RenderFrameCreated`).

## Process

For each step:

1. Modify `content/one_profile/` to match the step's description.
2. Build with `autoninja -C out/Default one_profile`.
3. Run the app and observe fps.
4. Record the result.
5. If fps dropped, investigate and fix before proceeding.
6. Commit each step (and each fix) separately.

## Experiments

### Experiment 1: Override SHELL_DIR_USER_DATA (Step 1)

#### Hypothesis

The `SHELL_DIR_USER_DATA` path override changes where Chromium stores profile
data but should not affect the rendering pipeline. One Profile currently uses
the default macOS path (`~/Library/Application Support/Chromium One Profile`).
Overriding it to `~/.config/termsurf/poc/profile-a` should have no effect on
framerate.

If this drops to 2fps, the path override itself interferes with some subsystem
(e.g., the storage service, the network service, or the path provider's
interaction with utility processes). This would be a significant finding — it
would mean the Two Profiles 2fps problem starts with the very first change.

#### Design

Override `InitializeBrowserContexts()` in `shell_browser_main_parts.cc` to call
`base::PathService::Override(SHELL_DIR_USER_DATA, ...)` before constructing the
`ShellBrowserContext`. The context's constructor calls `InitWhileIOAllowed()`,
which calls `base::PathService::Get(SHELL_DIR_USER_DATA, &path_)` — so the
override must happen before construction.

The profile path is `~/.config/termsurf/poc/profile-a`. Resolve the home
directory via `base::GetHomeDir()`.

The change to `InitializeBrowserContexts()`:

```cpp
void ShellBrowserMainParts::InitializeBrowserContexts() {
  base::FilePath profile_path =
      base::GetHomeDir()
          .Append(".config")
          .Append("termsurf")
          .Append("poc")
          .Append("profile-a");
  base::PathService::Override(SHELL_DIR_USER_DATA, profile_path);

  set_browser_context(new ShellBrowserContext(false));
  set_off_the_record_browser_context(new ShellBrowserContext(true));
  browser_context()->GetOriginTrialsControllerDelegate();
  off_the_record_browser_context()->GetOriginTrialsControllerDelegate();
}
```

Two includes are needed at the top of the file:

```cpp
#include "base/path_service.h"
#include "content/one_profile/common/shell_paths.h"
```

(`base/files/file_path.h` is already present.)

Everything else stays the same. `InitializeMessageLoopContext()` still calls
`Shell::CreateNewWindow(browser_context(), GetStartupURL(), nullptr,
gfx::Size())`
— a single Shell, single profile, standard lifecycle.

#### Branch setup

```bash
cd ts4/termsurf-chromium/src
git checkout -b 146.0.7650.0-issue-413 146.0.7650.0-issue-412
```

This starts from Issue 412's confirmed 60fps baseline.

#### Files to modify

- `content/one_profile/browser/shell_browser_main_parts.cc` — add
  `PathService::Override` call in `InitializeBrowserContexts()`, add two
  includes

#### Build and run

```bash
autoninja -C out/Default one_profile
cd /Users/ryan/dev/termsurf/ts4/box-demo && bun run server.ts &
./out/Default/One\ Profile.app/Contents/MacOS/One\ Profile http://localhost:9407
```

#### What this tests

- Whether overriding `SHELL_DIR_USER_DATA` to a custom path affects rendering
  framerate
- Whether the storage service, network service, and other utility processes
  function correctly with a non-default profile path
- Whether localStorage persists correctly at the new path
  (`~/.config/termsurf/poc/profile-a`)

#### What determines success or failure

- **60fps + localStorage persists at the new path:** The override is harmless.
  Proceed to Experiment 2 (Step 2: add second BrowserContext).
- **2fps:** The path override is the root cause (or a contributing factor).
  Investigate which subsystem breaks — likely the storage service failing to
  initialize at the custom path, falling back to degraded mode. Compare with
  `--user-data-dir=~/.config/termsurf/poc/profile-a` on vanilla Content Shell to
  see if the same override works there.
- **Crash or failure to launch:** The custom path doesn't exist or can't be
  created. Ensure `~/.config/termsurf/poc/profile-a` exists before running, or
  let Chromium create it (it should — `ShellBrowserContext` doesn't require the
  directory to pre-exist).

#### Expected result

60fps. This is a data-path-only change with no effect on the rendering pipeline,
compositor lifecycle, or view hierarchy. The storage service should handle the
custom path identically to the default path.

#### Result: PASSED

60fps. The spinning blue square renders smoothly at full framerate with the
profile data stored at `~/.config/termsurf/poc/profile-a`.

#### Build note

`autoninja` is at `/Users/ryan/depot_tools/autoninja` and is not in the default
shell PATH. When building from a script or automation, prefix with
`PATH="/Users/ryan/depot_tools:$PATH"`. The incremental build compiled only 8
steps (~12 seconds) since the only change was `shell_browser_main_parts.cc`.

#### Conclusion

The `SHELL_DIR_USER_DATA` path override has no effect on rendering. Redirecting
profile storage from the default macOS location to a custom path under
`~/.config/termsurf/poc/` does not degrade framerate, break the storage service,
or interfere with any utility processes. The storage service, network service,
and compositor all function identically with the custom path.

This eliminates the path override as a cause of the 2fps degradation seen in the
Two Profiles app. The next experiment (Step 2) adds a second
`ShellBrowserContext` — the first change that introduces a second profile into
the process.

### Experiment 2: Add second BrowserContext (Step 2)

#### Hypothesis

Creating a second `ShellBrowserContext` with a different storage path should not
affect Shell A's rendering. The `BrowserContext` is a data container — it holds
cookies, localStorage, and cache configuration. It does not interact with the
compositor, the view hierarchy, or the rendering pipeline.

If this drops to 2fps, the second `BrowserContext` itself degrades Shell A. The
most likely mechanism is the storage service: `ShellBrowserContext`'s
constructor calls `CreateBrowserContextServices(this)`, which may trigger the
storage service to initialize for the new context. Issue 411 observed a storage
service crash when two profiles coexisted — the service couldn't make
profile-b's paths relative to profile-a's root. If the crash or error handling
blocks or degrades the storage service for profile-a too, that could explain the
systemic 2fps.

#### Design

Add a `browser_context_b_` member to `ShellBrowserMainParts`. In
`InitializeBrowserContexts()`, after creating the profile-a context, override
`SHELL_DIR_USER_DATA` to profile-b and create a second `ShellBrowserContext`.
Don't use it for anything — no `WebContents`, no navigation. Clean it up in
`PostMainMessageLoopRun()`.

The change to the header (`shell_browser_main_parts.h`), add one member:

```cpp
private:
 std::unique_ptr<ShellBrowserContext> browser_context_;
 std::unique_ptr<ShellBrowserContext> off_the_record_browser_context_;
 std::unique_ptr<ShellBrowserContext> browser_context_b_;
```

The change to `InitializeBrowserContexts()`:

```cpp
void ShellBrowserMainParts::InitializeBrowserContexts() {
  base::FilePath profile_a_path =
      base::GetHomeDir()
          .Append(".config")
          .Append("termsurf")
          .Append("poc")
          .Append("profile-a");
  base::PathService::Override(SHELL_DIR_USER_DATA, profile_a_path);

  set_browser_context(new ShellBrowserContext(false));
  set_off_the_record_browser_context(new ShellBrowserContext(true));
  browser_context()->GetOriginTrialsControllerDelegate();
  off_the_record_browser_context()->GetOriginTrialsControllerDelegate();

  base::FilePath profile_b_path =
      base::GetHomeDir()
          .Append(".config")
          .Append("termsurf")
          .Append("poc")
          .Append("profile-b");
  base::PathService::Override(SHELL_DIR_USER_DATA, profile_b_path);

  browser_context_b_ = std::make_unique<ShellBrowserContext>(false);
}
```

The change to `PostMainMessageLoopRun()`, add cleanup before the existing
context resets:

```cpp
browser_context_b_.reset();
browser_context_.reset();
off_the_record_browser_context_.reset();
```

Note: after both contexts are created, the global `SHELL_DIR_USER_DATA` is left
pointing at profile-b. This is the same state the old Two Profiles app was in.
If this causes the storage service to resolve paths against profile-b instead of
profile-a for Shell A's context, that's a finding — but it should not affect
rendering since the storage path is read once during `InitWhileIOAllowed()` and
cached in `ShellBrowserContext::path_`.

#### Files to modify

- `content/one_profile/browser/shell_browser_main_parts.h` — add
  `browser_context_b_` member
- `content/one_profile/browser/shell_browser_main_parts.cc` — expand
  `InitializeBrowserContexts()` with profile-b context creation, add cleanup in
  `PostMainMessageLoopRun()`

#### Build and run

```bash
autoninja -C out/Default one_profile
cd /Users/ryan/dev/termsurf/ts4/box-demo && bun run server.ts &
./out/Default/One\ Profile.app/Contents/MacOS/One\ Profile http://localhost:9407
```

#### What this tests

- Whether creating a second `ShellBrowserContext` (unused) degrades Shell A's
  framerate
- Whether the storage service can handle two `BrowserContext` instances with
  different storage paths without crashing or degrading
- Whether `CreateBrowserContextServices()` for the second context has any side
  effects on the first

#### What determines success or failure

- **60fps:** The second `BrowserContext` is harmless. Proceed to Experiment 3
  (Step 3: own the window).
- **2fps:** The second `BrowserContext` is the culprit. Investigate the storage
  service — check for crashes in the console output (`[ERROR:storage_...]`),
  check whether
  `browser_context_b_ = std::make_unique<ShellBrowserContext>(false, /*delay_services_creation=*/true)`
  (delaying service creation) restores 60fps. If it does,
  `CreateBrowserContextServices` is the trigger.
- **Crash:** Likely the storage service crash from Issue 411. Check whether the
  second `PathService::Override` leaves the storage service in a broken state
  for profile-a.

#### Expected result

60fps. The second `BrowserContext` is a data object with no rendering side
effects. But this is the first experiment that introduces a second profile into
the process, which is exactly what distinguishes the Two Profiles app from
Content Shell — so a failure here would be a major finding.
