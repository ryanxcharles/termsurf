# Issue 412: One Profile — Isolate the 2fps Cause

## Goal

Starting from a minimal app that is nearly identical to Content Shell (60fps),
add changes one at a time toward a two-profile side-by-side layout. Each step is
a self-contained experiment. The step where fps drops from 60 to 2 identifies
the exact cause of the rendering degradation.

## Background

Content Shell runs at 60fps with a single profile. The Two Profiles app (Issue
407) runs at 2fps — both panes, including Shell A which uses the same
`Shell::CreateNewWindow` code path. Issues 410 and 411 spent five experiments
targeting throttling and visibility code paths that turned out to be irrelevant.
The actual cause remains unknown.

The Two Profiles app differs from Content Shell in several ways. Any one of them
could be the culprit:

1. Custom `TwoProfilesMainParts` subclass of `ShellBrowserMainParts`
2. `SHELL_DIR_USER_DATA` path override (changes the global profile path)
3. Second `ShellBrowserContext` with a different storage path
4. Second `WebContents` created and navigated
5. View hierarchy manipulation (adding a second NSView, resizing the first)

Rather than guessing, we will isolate the cause by adding these changes one at a
time.

## Branch

Create a new branch `146.0.7650.0-issue-412` in the `termsurf-chromium`
submodule, starting from the vanilla Chromium `146.0.7650.0` tag. Cherry-pick
the Two Profiles app commit to get the build scaffolding, then apply each step
as a commit on top.

## Steps

### Step 1: Baseline — Content Shell equivalent

Strip the Two Profiles app down to a single `Shell::CreateNewWindow` call with
no path overrides, no second BrowserContext, and no view manipulation. This
should be functionally identical to Content Shell.

```
InitializeBrowserContexts: default (inherited from ShellBrowserMainParts)
InitializeMessageLoopContext: Shell::CreateNewWindow(browser_context(), url, ...)
```

**Expected: 60fps.** If this is 2fps, the problem is in the app scaffolding
itself (BUILD.gn, delegates, plists) and not in any of our code changes.

### Step 2: Override SHELL_DIR_USER_DATA

Add the `SHELL_DIR_USER_DATA` override to point profile-a at
`~/.config/termsurf/poc/profile-a`.

```
InitializeBrowserContexts:
  PathService::Override(SHELL_DIR_USER_DATA, GetProfilePath("profile-a"))
  set_browser_context(new ShellBrowserContext(false))
```

**Expected: 60fps.** If this drops to 2fps, the path override is interfering
with the storage service or some other subsystem that depends on the default
path.

### Step 3: Add second BrowserContext

Create `browser_context_b_` with a path override to profile-b. Don't use it for
anything — just create and hold it.

```
InitializeBrowserContexts:
  PathService::Override(SHELL_DIR_USER_DATA, GetProfilePath("profile-a"))
  set_browser_context(new ShellBrowserContext(false))
  PathService::Override(SHELL_DIR_USER_DATA, GetProfilePath("profile-b"))
  browser_context_b_ = make_unique<ShellBrowserContext>(false)
```

**Expected: 60fps.** If this drops to 2fps, creating a second BrowserContext
interferes with Shell A's rendering — possibly through the global
`SHELL_DIR_USER_DATA` being left pointing at profile-b, or through the storage
service trying to serve both contexts from one root.

### Step 4: Add second WebContents (no view attachment)

Create a second `WebContents` with `browser_context_b_` and navigate it to the
test page, but do not add its view to any window.

```
InitializeMessageLoopContext:
  Shell::CreateNewWindow(browser_context(), url, ...)
  web_contents_b_ = WebContents::Create(CreateParams(browser_context_b_))
  web_contents_b_->GetController().LoadURLWithParams(url)
```

**Expected: 60fps.** If this drops to 2fps, the act of creating and navigating a
second WebContents (even without displaying it) triggers something that degrades
Shell A's rendering — possibly the storage service crash, renderer process
contention, or compositor interference.

### Step 5: Attach second view side by side

Add WebContents B's view to Shell A's window, side by side. This is the full Two
Profiles layout.

```
InitializeMessageLoopContext:
  ... (same as step 4)
  [container addSubview:view_b]
  view_a.frame = left half
  view_b.frame = right half
```

**Expected: Shell A 60fps, Shell B unknown.** If Shell A drops to 2fps here, the
view hierarchy manipulation is the cause. If Shell A stays at 60fps but Shell B
is at 2fps, the race condition from Issue 411 is confirmed as the cause for
Shell B specifically.

## Process

For each step:

1. Modify `two_profiles_main_parts.{h,mm}` to match the step's description.
2. Build with `autoninja -C out/Default two_profiles`.
3. Run the app and observe the fps in the test page.
4. Record the result (fps for each visible pane).
5. If fps dropped, stop — the cause is identified. Investigate further.
6. If fps is still 60, proceed to the next step.

## Experiments

### Experiment 1: Step 1 baseline

#### Branch setup

1. `cd ts4/termsurf-chromium/src`
2. `git checkout -b 146.0.7650.0-issue-412 146.0.7650.0`
3. Write the One Profile app from scratch in `content/one_profile/`:
   - `BUILD.gn` — modeled on Content Shell's build target but for our app
   - `one_profile_main_parts.h` and `one_profile_main_parts.mm` — the main parts
     subclass
   - Delegates, plists, and main entry point — minimal scaffolding to produce a
     `.app` bundle
4. Add `//content/one_profile` to the root `BUILD.gn` `gn_all` group.
5. Build with `autoninja -C out/Default one_profile`.

#### Hypothesis

The One Profile app is a minimal Content API embedder written from scratch. It
creates a single `WebContents` with one `ShellBrowserContext` and displays it in
a window we control. The key difference from Content Shell is that we own the
`NSWindow` — Content Shell's `Shell` class creates and manages its own window,
but we need to control the window ourselves so that later steps can place
multiple WebContents in it.

If this baseline runs at 60fps, the app scaffolding is sound and we can proceed
to add a second profile. If it runs at 2fps, then controlling the window
ourselves (rather than letting `Shell` do it) is the problem, and we need to
understand why.

#### Design

`OneProfileMainParts` inherits from `ShellBrowserMainParts`.

- Do NOT override `InitializeBrowserContexts`. The base class creates a single
  `ShellBrowserContext` with the default path.
- Override `InitializeMessageLoopContext`:
  1. Create an `NSWindow` (1200x600, titled, `makeKeyAndOrderFront`).
  2. Create a `WebContents` with `browser_context()`.
  3. Navigate it to `http://localhost:9407`.
  4. Get the `WebContentsViewCocoa` via `GetNativeView()`.
  5. Add it as a subview of the window's `contentView`.
  6. Set its frame to fill the window.

No `Shell` is used. No second BrowserContext, no second WebContents, no
`SHELL_DIR_USER_DATA` override. This is the minimal code needed to display a
single WebContents in a window we control — the foundation that a second profile
will eventually be added to.

#### Expected result

60fps. If this is 2fps, controlling the window ourselves rather than using
`Shell::CreateNewWindow` is the cause, and we need to understand what `Shell`
does that we're missing.
