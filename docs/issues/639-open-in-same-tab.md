# Issue 639: Open New-Tab Links in Same Tab

## Goal

Intercept links that request a new tab or window (`target="_blank"`,
`window.open()`, middle-click, Cmd+click) and open them in the current tab
instead. This makes these links functional now while deferring true multi-tab
support.

## Background

The Chromium Profile Server's `Shell` class inherits its `WebContentsDelegate`
implementation from `content_shell`. When a link requests a new tab or window,
two delegate methods handle it:

- **`OpenURLFromTab`** — Called for navigations with a non-`CURRENT_TAB`
  disposition (e.g. `target="_blank"` links, Cmd+click). Currently creates a new
  `Shell` window via `Shell::CreateNewWindow`.
- **`AddNewContents`** — Called when Chromium has already created a new
  `WebContents` (e.g. `window.open()` popups). Currently creates a new `Shell`
  to host it via `CreateShell`.

Both methods create standalone windows that TermSurf doesn't manage — they float
outside the terminal, have no XPC connection, no pane ID, and no way to stream
back to the TUI. The links "work" in the sense that Chromium opens them, but the
user never sees the result.

The fix is simple: instead of creating a new window, navigate the current tab to
the target URL.

## Current state

- **`Shell::OpenURLFromTab`** (`shell.cc:427`): For `NEW_FOREGROUND_TAB`,
  `NEW_BACKGROUND_TAB`, `NEW_POPUP`, and `NEW_WINDOW` dispositions, calls
  `Shell::CreateNewWindow` and navigates the new shell.
- **`Shell::AddNewContents`** (`shell.cc:311`): Creates a new `Shell` for the
  incoming `WebContents` via `CreateShell`.

## Experiment 1: Redirect to current tab

### Hypothesis

Modifying `OpenURLFromTab` to navigate the source tab (instead of creating a new
window) for all new-tab/new-window dispositions, and modifying `AddNewContents`
to navigate the source tab to the target URL (discarding the pre-created
`WebContents`), will make all "open in new tab" links work in the current tab.

### Changes

#### 1. `Shell::OpenURLFromTab` (`shell.cc`)

Change the `NEW_POPUP` / `NEW_WINDOW` / `NEW_BACKGROUND_TAB` /
`NEW_FOREGROUND_TAB` cases to navigate the source tab instead of creating a new
window:

```cpp
case WindowOpenDisposition::NEW_POPUP:
case WindowOpenDisposition::NEW_WINDOW:
case WindowOpenDisposition::NEW_BACKGROUND_TAB:
case WindowOpenDisposition::NEW_FOREGROUND_TAB:
  // Issue 639: Open in current tab instead of creating a new window.
  // True multi-tab support deferred.
  target = source;
  break;
```

#### 2. `Shell::AddNewContents` (`shell.cc`)

Navigate the source to the target URL and discard the pre-created `WebContents`.
The `new_contents` unique_ptr will be destroyed when it goes out of scope:

```cpp
WebContents* Shell::AddNewContents(
    WebContents* source,
    std::unique_ptr<WebContents> new_contents,
    const GURL& target_url,
    WindowOpenDisposition disposition,
    const blink::mojom::WindowFeatures& window_features,
    bool user_gesture,
    bool* was_blocked) {
  // Issue 639: Instead of creating a new window, navigate the source tab
  // to the target URL. The pre-created WebContents is discarded.
  if (source && target_url.is_valid()) {
    NavigationController::LoadURLParams params(target_url);
    params.transition_type = ui::PAGE_TRANSITION_LINK;
    source->GetController().LoadURLWithParams(params);
    return source;
  }
  return nullptr;
}
```

Required includes (check if already present):

```cpp
#include "content/public/browser/navigation_controller.h"
#include "ui/base/page_transition_types.h"
```

### Verification

1. Build Chromium (`autoninja -C out/Default chromium_profile_server`)
2. Launch TermSurf, `web google.com`
3. Search for something, click a result — opens in same tab
4. Find a `target="_blank"` link (e.g. footer links on many sites) — opens in
   same tab instead of a new window
5. Test `window.open()` via DevTools or a test page — opens in same tab
6. No stray Chromium windows should appear

### Success criteria

- `target="_blank"` links navigate the current tab
- `window.open()` navigates the current tab
- No new Chromium windows are created
- Back/forward navigation still works after redirected navigations
- Page title and URL bar update correctly after redirected navigations
