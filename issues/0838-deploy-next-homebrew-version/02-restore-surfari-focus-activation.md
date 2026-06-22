# Experiment 2: Restore Surfari Focus Activation

## Description

Experiment 1 proved that this VM can build WebKit, `libtermsurf_webkit`, and the
Rust `surfari` binary, but the existing `libtermsurf_webkit` smoke test failed
with:

```text
SMOKE_FAIL focus was not observed
CALLBACK focus_state {"focus":false,"focusIn":false,"hasFocus":false,"activeElement":""}
```

The C ABI currently receives `ts_set_focus(web_contents, true)`, but the
implementation only records `contents->focused = focused`. It does not make the
host window key or make the `WKWebView` first responder when focus is gained.
The hidden `TSHostWindow` also returns `NO` from `canBecomeKeyWindow`, which
prevents normal AppKit key-window focus from reaching the page.

This experiment should restore real AppKit/WebKit focus activation while keeping
the smoke test strict. The goal is not to weaken the test; the goal is for
`ts_set_focus(true)` to make WebKit observe focus, and for `ts_set_focus(false)`
/ `ts_set_gui_active(false)` to keep producing the blur and inactive behavior
already covered by the smoke test.

## Changes

- `surfari/libtermsurf_webkit/src/libtermsurf_webkit.mm`:
  - Allow the hidden `TSHostWindow` to become key/main when Surfari explicitly
    focuses a web contents.
  - Update `ts_set_focus(true)` to activate the hidden host window enough for
    WebKit focus, make the `WKWebView` first responder, and leave existing
    unfocus behavior intact.
  - Update `ts_set_gui_active(true)` only if necessary to preserve the intended
    active/focused state model before `ts_set_focus(true)`.
- `surfari/libtermsurf_webkit/README.md`:
  - Update the current limitations section only if the final behavior or caveats
    differ from the existing "AppKit first-responder assignment, page-visible
    focus, and GUI active/inactive state" claim.
- `issues/0838-deploy-next-homebrew-version/README.md`:
  - Mark Experiment 2 as `Pass`, `Partial`, or `Fail` after verification.
  - Mark Stage 3 complete only if the smoke test and Rust Surfari build both
    pass.
- `issues/0838-deploy-next-homebrew-version/02-restore-surfari-focus-activation.md`:
  - Record exact implementation, verification output, result, conclusion, and
    review results.

## Verification

Build `libtermsurf_webkit` and the smoke test:

```bash
surfari/libtermsurf_webkit/build.sh
```

Run the existing smoke test without weakening its focus assertion:

```bash
DYLD_FRAMEWORK_PATH="$(pwd)/webkit/src/WebKitBuild/Debug" \
  surfari/libtermsurf_webkit/build/smoke-test \
  "$(pwd)/surfari/libtermsurf_webkit/test-content/index.html" \
  "$(pwd)/surfari/libtermsurf_webkit/test-content/navigation.html"
```

Build the Rust Surfari binary:

```bash
cargo build -p surfari
```

Format and hygiene checks:

```bash
xcrun clang-format --dry-run --Werror surfari/libtermsurf_webkit/src/libtermsurf_webkit.mm
git diff --check
git status --short
```

Pass criteria:

- `surfari/libtermsurf_webkit/build.sh` completes and produces
  `libtermsurf_webkit.dylib` and `smoke-test`.
- The smoke test prints `SMOKE_PASS`.
- The smoke-test `focus_state` callback shows either `"focus":true` or
  `"hasFocus":true`.
- The smoke-test `input_state` callback still shows `"blur":true`, `"key":"a"`,
  mouse movement, click, scroll, and dark color scheme.
- `cargo build -p surfari` succeeds.
- `git diff --check` reports no whitespace errors.
- The implementation does not weaken or remove the focus assertion in the smoke
  test.

Fail criteria:

- `ts_set_focus(true)` still does not produce page-visible focus.
- Fixing focus breaks keyboard, mouse, scroll, color-scheme, blur, or later
  smoke-test sections.
- The smoke test is loosened instead of fixing the focus implementation.
- `cargo build -p surfari` fails.

## Design Review

Adversarial subagent review, fresh context, completed before implementation.

Verdict: **Approved**.

Findings:

- Optional: the plan says to "activate the hidden host window enough" but does
  not prescribe the exact AppKit call sequence. The result should record the
  actual sequence used.
- Optional: the design originally used `clang-format --dry-run --Werror`, which
  may depend on local PATH/tooling.

Resolution:

- Accepted the hygiene-tooling note. Verified Xcode provides
  `xcrun clang-format` and updated the verification command accordingly.
- Accepted the AppKit-sequence note for result recording. The implementation
  will keep the experiment narrow and the result will document the exact calls
  used.

## Result

**Result:** Pass

The implementation restored page-visible WebKit focus without weakening the
smoke test.

Changed `surfari/libtermsurf_webkit/src/libtermsurf_webkit.mm`:

- `TSHostWindow` now returns `YES` from `canBecomeKeyWindow` and
  `canBecomeMainWindow`.
- Added `applyFocusState(WebContents *)` so focus and GUI-active state share the
  same AppKit behavior.
- On active focus, Surfari now calls:
  - `[NSApp activateIgnoringOtherApps:YES]`;
  - `[contents->window makeKeyAndOrderFront:nil]`;
  - `[contents->window makeMainWindow]`;
  - `[contents->window makeFirstResponder:contents->web_view]`.
- On inactive or unfocused state, Surfari preserves the existing behavior:
  - `[contents->window makeFirstResponder:nil]`;
  - `[contents->window resignKeyWindow]`.

`surfari/libtermsurf_webkit/build.sh` completed and produced the expected
artifacts. It still emitted the pre-existing SDK-version linker warning:

```text
ld: warning: building for macOS-26.0, but linking with dylib
'/System/Library/Frameworks/WebKit.framework/Versions/A/WebKit' which was built
for newer version 26.5
built surfari/libtermsurf_webkit/build/libtermsurf_webkit.dylib
built surfari/libtermsurf_webkit/build/smoke-test
```

The unchanged smoke test passed:

```text
CALLBACK focus_state {"focus":true,"focusIn":false,"hasFocus":true,"activeElement":""}
CALLBACK input_state {"focus":true,"focusIn":false,"blur":true,"move":"120,130","click":"140,150,0","scroll":-120,"key":"a","colorScheme":"dark"}
SMOKE_PASS initialized=1 tab_ready=1 ca_context=5 url=6 loading_started=4 loading_finished=4 title=3 navigations=4 resized=1 focus=1 input=1 target_url=1 cursor=1 console=1 js_dialogs=1 http_auth=1 renderer_crash=1
```

The smoke test also completed cursor, target URL, console, JavaScript dialog,
HTTP auth, and renderer-crash checks. It printed one non-fatal WebKit navigation
log while canceling the expected auth-reject navigation:

```text
[libtermsurf_webkit] provisional navigation failed: Error Domain=NSURLErrorDomain Code=-999 "cancelled"
```

`cargo build -p surfari` succeeded:

```text
Compiling surfari v0.1.0 (/Users/astrohacker/dev/termsurf/surfari)
Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.56s
```

`git diff --check` reported no whitespace errors.

`xcrun clang-format --dry-run --Werror surfari/libtermsurf_webkit/src/libtermsurf_webkit.mm`
was attempted and failed because the existing Objective-C++ file is not
clang-format-clean as a whole and the repo does not currently carry a
`.clang-format` baseline for this file. This is not evidence of a new formatting
regression in the focused patch; the result therefore relies on the successful
build, strict smoke test, Rust build, and `git diff --check`.

## Conclusion

Stage 3 is complete. This machine now builds WebKit, builds
`libtermsurf_webkit`, passes the strict Surfari C ABI smoke test, and builds the
Rust `surfari` binary.

The next experiment should move to Stage 4: wire Surfari into the build,
install, release, Homebrew cask, and installed Ghostboard browser-resolution
paths. The production integration should pay attention to the fact that WebKit
page-visible focus currently requires activating the Surfari process; installed
app testing should verify that this does not steal usable keyboard focus from
Ghostboard when `web --browser surfari` is running.

## Completion Review

Adversarial subagent review, fresh context, completed after implementation and
result recording.

Verdict: **Approved**.

Findings:

- No required fixes.

Independent checks:

- `surfari/libtermsurf_webkit/build.sh` passed.
- The unchanged smoke test passed with `focus_state` showing `focus` and
  `hasFocus` true, and printed `SMOKE_PASS`.
- `cargo build -p surfari` passed.
- `git diff --check` passed.
- `xcrun clang-format --dry-run --Werror` failed across the whole pre-existing
  Objective-C++ file, matching the recorded formatting-baseline limitation.

Resolution:

- No changes required.
