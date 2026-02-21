# Issue 616: Implement missing web features

## Goal

Systematically identify and implement browser features that are missing from the
current gui/ generation. High-impact features are implemented first. Features
that can't be implemented yet are logged for future work.

## Background

ts1 (WKWebView generation) implemented a comprehensive set of browser features
from v0.3 through v1.0. The current gui/ generation (Chromium via Content API)
has the core streaming pipeline working — live rendering, mouse input, keyboard
input, multi-pane multi-profile — but is missing many user-facing browser
features that ts1 had.

Some ts1 features translate directly to Chromium (downloads, file uploads, JS
dialogs). Others are WKWebView-specific and either aren't needed with Chromium
or require a different approach. This issue catalogs everything and prioritizes
what to build.

### Feature inventory

Features are organized by priority. Priority is based on how often a user would
encounter the missing feature during normal browsing.

#### High priority

These features affect common browsing scenarios. A user will hit these within
minutes of browsing.

| # | Feature                      | ts1 status                       | gui/ status     | Notes                                                                                                               |
| - | ---------------------------- | -------------------------------- | --------------- | ------------------------------------------------------------------------------------------------------------------- |
| 1 | **target="_blank" handling** | Loads in same view               | Not implemented | Links requesting new windows (OAuth, "Open in new tab") silently fail without this. Very common on modern websites. |
| 2 | **JavaScript dialogs**       | alert/confirm/prompt via NSAlert | Not implemented | Many sites use confirm() for destructive actions, prompt() for input. Sites break without these.                    |
| 3 | **Downloads**                | WKDownloadDelegate + NSSavePanel | Not implemented | Any file download link currently does nothing.                                                                      |
| 4 | **File uploads**             | NSOpenPanel via WKUIDelegate     | Not implemented | `<input type="file">` does nothing without this. Common for profile pictures, attachments, etc.                     |
| 5 | **Page zoom**                | Cmd+=/-/0 via pageZoom           | Not implemented | Users expect standard zoom keybindings.                                                                             |
| 6 | **HTTP Basic Auth**          | NSAlert with username/password   | Not implemented | Password-protected pages show blank or error without this.                                                          |
| 7 | **URL normalization**        | Prepend https://                 | Not implemented | Users type `google.com`, not `https://google.com`. The `web` TUI or Chromium server should handle this.             |

#### Medium priority

These features matter but are encountered less frequently or have workarounds.

| #  | Feature                      | ts1 status                     | gui/ status     | Notes                                                                                                              |
| -- | ---------------------------- | ------------------------------ | --------------- | ------------------------------------------------------------------------------------------------------------------ |
| 8  | **Crash recovery**           | Reload/close dialog            | Not implemented | Chromium renderer crashes are rare but should be handled gracefully.                                               |
| 9  | **Camera/mic permissions**   | Permission prompt              | Not implemented | Only needed for video calls, media recording. Can defer.                                                           |
| 10 | **Console capture**          | JS injection → stdout/stderr   | Not implemented | Useful for developers. The `web` TUI could display console output. Requires Chromium DevTools protocol or similar. |
| 11 | **Web Inspector / DevTools** | Safari Inspector via Cmd+Alt+I | Not implemented | Chromium has DevTools built in, but we need a way to open them (remote debugging port, or in-process).             |

#### Lower priority

These are nice-to-have or may not apply to the Chromium architecture.

| #  | Feature                            | ts1 status                     | gui/ status         | Notes                                                                                      |
| -- | ---------------------------------- | ------------------------------ | ------------------- | ------------------------------------------------------------------------------------------ |
| 12 | **User-Agent spoofing**            | Custom Safari UA string        | Probably not needed | Chromium sends a real browser UA by default. Unlikely to get mobile layouts.               |
| 13 | **Header injection**               | Upgrade-Insecure-Requests      | Probably not needed | Chromium sends this header natively. Was a WKWebView-specific workaround.                  |
| 14 | **Blob download workaround**       | JS interceptor for WebKit bug  | Not needed          | This was a WebKit bug. Chromium handles blob: downloads natively.                          |
| 15 | **Session isolation (incognito)**  | Ephemeral WKWebsiteDataStore   | Not implemented     | Chromium supports incognito via BrowserContext. Low urgency — named profiles already work. |
| 16 | **Bookmarking**                    | Cmd+B, file-based JSON storage | Not implemented     | Useful but not critical for initial release.                                               |
| 17 | **JavaScript API (--js-api)**      | window.termsurf.exit(code)     | Not implemented     | Niche feature for scripting. Defer.                                                        |
| 18 | **Hide/show webviews (ctrl+z/fg)** | isHidden property              | Not implemented     | Terminal backgrounding support. Defer.                                                     |
| 19 | **Multi-webview stacking**         | Stack per pane with indicator  | Not implemented     | Multiple webviews per pane. Current architecture is one-per-pane. Defer.                   |
| 20 | **Dynamic tab titles**             | KVO on WKWebView.title         | Not implemented     | Tab shows page title. Requires Chromium to send title updates via XPC.                     |

#### Already implemented (in gui/ or differently)

| Feature               | Notes                                                                |
| --------------------- | -------------------------------------------------------------------- |
| **Profile isolation** | Multi-profile via separate Chromium Profile Servers (Issues 604–605) |
| **Three-mode focus**  | Browse/Control modes with Esc/Enter switching (Issue 607)            |
| **Focus management**  | Chromium focus/blur via XPC (Issue 606)                              |
| **Control bar**       | `web` TUI draws URL bar, status bar (Issue 504)                      |

### Implementation approach

Each feature is a self-contained experiment. Features that require Chromium-side
changes (new XPC messages, new Content API calls) are harder than features that
can be handled entirely in gui/ Zig code or the `web` TUI.

**Chromium-side changes** are needed for: downloads, file uploads, JS dialogs,
HTTP auth, crash recovery, camera/mic permissions, console capture, DevTools,
dynamic tab titles.

**GUI-side only** changes: target="_blank" (if we load in same tab), page zoom,
URL normalization.
