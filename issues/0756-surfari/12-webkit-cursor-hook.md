# Experiment 12: Hook WebKit cursor changes

## Description

Experiment 11 proved that cursor updates cannot be implemented reliably from
outside WebKit. WebKit's macOS cursor path rejects many cursor changes unless
the real active-window and global mouse-location guards pass:
`Source/WebKit/UIProcess/mac/PageClientImplMac.mm` implements
`PageClientImpl::setCursor(const WebCore::Cursor&)`, receives the authoritative
`WebCore::Cursor`, then may return before AppKit's `NSCursor` is changed.

This experiment should add the smallest WebKit source hook needed for Surfari:
when `PageClientImpl::setCursor` receives a cursor for a `WKWebView`, post an
in-process `NSNotification` whose object is that `WKWebView` and whose payload
contains the WebCore cursor type. `libtermsurf_webkit` can observe that
notification for the exact `WKWebView` it owns, map WebCore cursor types to
Chromium-compatible `ui::mojom::CursorType` integer values, suppress duplicates,
and fire `ts_set_on_cursor_changed`.

This keeps the public Surfari C ABI unchanged and avoids modifying Ghostboard or
`termsurf.proto`. It also keeps the WebKit patch narrow and auditable.

This experiment should not create the Surfari Rust binary, modify Ghostboard,
modify `termsurf.proto`, implement console messages, implement renderer crash
reporting, implement DevTools, or change unrelated WebKit behavior.

## Changes

- Create/use a dedicated WebKit experiment branch:
  `webkit-1452a439-issue-756-exp12`, based on
  `1452a43959523449099b2616793fd2c5b6a6487e`.
- Patch `webkit/src/Source/WebKit/UIProcess/mac/PageClientImplMac.mm`:
  - define a TermSurf cursor notification name string;
  - in `PageClientImpl::setCursor`, post the notification before WebKit's
    active-window/global-mouse guards can reject the AppKit cursor update;
  - use `this->webView()` as the notification object so embedders can route the
    callback to the correct `WKWebView`;
  - include the WebCore cursor type integer in `userInfo`.
- Commit the WebKit source change inside `webkit/src`.
- Generate and track the patch archive under `webkit/patches/issue-756/`.
- Update `webkit/README.md` Branches table with the experiment branch and patch
  purpose.
- Update `surfari/libtermsurf_webkit/src/libtermsurf_webkit.mm`:
  - add a notification observer for each owned `WKWebView`;
  - remove that observer during `ts_destroy_web_contents`;
  - map WebCore `PlatformCursorType` values to Chromium-compatible
    `ui::mojom::CursorType` values for at least pointer (`0`), hand (`2`), and
    i-beam (`3`);
  - suppress duplicate cursor callbacks.
- Extend `surfari/libtermsurf_webkit/test-content/navigation.html` with
  deterministic pointer, link, and text/input regions.
- Extend `surfari/libtermsurf_webkit/smoke-test/smoke_test.c` to move through
  those regions and fail unless it observes pointer `0`, hand `2`, and i-beam
  `3` in that order, with no duplicate callback for repeated movement over the
  same cursor type.
- Keep Experiment 6/7/8/9/10 smoke coverage intact: lifecycle, navigation,
  resize, focus, mouse, scroll, keyboard, color scheme, target URL, JavaScript
  dialogs, and HTTP auth must still pass.
- Update `surfari/libtermsurf_webkit/README.md` so cursor updates move from
  unsupported to implemented only if the smoke proof passes.

## Verification

Start from a clean TermSurf repo root and clean WebKit checkout:

```bash
git status --short
git -C webkit/src status --short
git -C webkit/src rev-parse HEAD
git -C webkit/src rev-parse --abbrev-ref HEAD
git -C webkit/src rev-parse --is-shallow-repository
```

Create or verify the WebKit experiment branch:

```bash
git -C webkit/src switch -C webkit-1452a439-issue-756-exp12 \
  1452a43959523449099b2616793fd2c5b6a6487e
```

After patching WebKit, rebuild WebKit debug:

```bash
webkit/src/Tools/Scripts/build-webkit --debug
```

Build and run the Surfari smoke test:

```bash
surfari/libtermsurf_webkit/build.sh

mkdir -p logs
DYLD_FRAMEWORK_PATH="$PWD/webkit/src/WebKitBuild/Debug" \
surfari/libtermsurf_webkit/build/smoke-test \
  "$PWD/surfari/libtermsurf_webkit/test-content/index.html" \
  "$PWD/surfari/libtermsurf_webkit/test-content/navigation.html" \
  > logs/issue756-exp12-cursor-hook.log 2>&1
rc=$?
echo "SMOKE_EXIT_STATUS=$rc" >> logs/issue756-exp12-cursor-hook.log
```

The smoke log must prove:

- Experiment 6/7/8/9/10 evidence still passes.
- Cursor callbacks are emitted from the WebKit notification path after forwarded
  mouse movement reaches WebKit.
- Moving over the deterministic plain region emits pointer/arrow `0`.
- Moving over the deterministic link emits hand `2`.
- Moving over the deterministic text/input region emits i-beam `3`.
- Repeated movement over the same cursor region does not emit duplicate cursor
  callbacks.
- The smoke harness fails, rather than merely logging, if the cursor callback
  sequence is not the expected pointer/hand/i-beam sequence.

Generate and verify the WebKit patch archive:

```bash
rm -rf webkit/patches/issue-756
mkdir -p webkit/patches/issue-756
git -C webkit/src format-patch 1452a43959523449099b2616793fd2c5b6a6487e..HEAD \
  -o ../../webkit/patches/issue-756
find webkit/patches/issue-756 -type f | sort
```

Verify symbols/linkage and checkout state:

```bash
nm -gU surfari/libtermsurf_webkit/build/libtermsurf_webkit.dylib | rg ' _ts_|_ts_webkit_test' | sort
otool -L surfari/libtermsurf_webkit/build/libtermsurf_webkit.dylib | rg 'WebKit|JavaScriptCore|libtermsurf'
otool -L surfari/libtermsurf_webkit/build/smoke-test | rg 'WebKit|JavaScriptCore|libtermsurf'
git diff --check
prettier --check --prose-wrap always --print-width 80 \
  issues/0756-surfari/README.md \
  issues/0756-surfari/12-webkit-cursor-hook.md \
  webkit/README.md \
  webkit/patches/README.md
git -C webkit/src status --short
git -C webkit/src rev-parse HEAD
git -C webkit/src rev-parse --abbrev-ref HEAD
git -C webkit/src rev-parse --is-shallow-repository
```

There is no project-configured formatter for Objective-C++ or C in
`surfari/libtermsurf_webkit`; keep those edits local-style consistent and use
`git diff --check` as the whitespace guard. WebKit formatting should follow the
surrounding WebKit source style.

**Pass** = cursor callbacks work through the WebKit source notification hook,
the smoke test exits 0, all prior evidence still passes, the WebKit change is
committed on `webkit-1452a439-issue-756-exp12`, the patch archive is tracked in
the main repo, and the READMEs reflect support/branch state.

**Partial** = the WebKit hook builds but does not emit enough cursor types for
the smoke proof, or rebuilding WebKit exposes a build-system/signature issue.
The result must record the exact blocker and whether the WebKit patch should be
kept, revised, or reverted before the next experiment.

**Fail** = the implementation regresses prior lifecycle/input/focus/target
URL/dialog/auth coverage, cannot route notifications to the owning `WKWebView`,
or creates a broad/unreviewable WebKit patch.

## Design Review

Adversarial subagent review, fresh context, read-only.

Verdict: **Approved**. No findings.

## Result

**Result:** Pass

Experiment 12 added a small WebKit macOS source hook in
`PageClientImpl::setCursor`. The hook posts
`TermSurfWebKitCursorChangedNotification` with the owning `WKWebView` as the
notification object and the raw WebCore cursor enum in `cursorType`. Surfari
observes that notification for its owned `WKWebView`, maps WebCore pointer,
hand, and i-beam values to the existing Chromium/TermSurf cursor integers, and
suppresses duplicate callbacks.

The smoke harness now asserts the cursor sequence instead of only logging it. It
moves through deterministic pointer, hand, and text regions with short delays
between regions so WebKit processes each hover state. The passing log is
`logs/issue756-exp12-cursor-hook.log`.

Key evidence from the passing smoke run:

```text
CALLBACK cursor cursor_type=0
CALLBACK cursor cursor_type=2
CALLBACK cursor cursor_type=3
SMOKE_PASS initialized=1 tab_ready=1 ca_context=5 url=6 loading_started=4 loading_finished=4 title=3 navigations=4 resized=1 focus=1 input=1 target_url=1 cursor=1 js_dialogs=1 http_auth=1
SMOKE_EXIT_STATUS=0
```

WebKit branch state:

- Branch: `webkit-1452a439-issue-756-exp12`
- Base: `1452a43959523449099b2616793fd2c5b6a6487e`
- Commit: `cdfb8cbf86f7c5e52cef0b2f14e8ab30ceeea91c`
- Patch archive:
  `webkit/patches/issue-756/0001-Notify-TermSurf-cursor-changes.patch`

The result also adds a root `.gitattributes` rule for `*.patch` files so
`git diff --check` does not reject syntactically required blank context lines in
`git format-patch` archives while source-file whitespace checks remain active.

## Conclusion

Surfari now has verified pointer, hand, and i-beam cursor callbacks through a
tracked WebKit source hook. The cursor smoke exposed an important testing
detail: WebKit may coalesce back-to-back synthetic hover moves, so cursor tests
must give WebKit a short run-loop interval between distinct hover regions.

The next experiment can move to another unsupported Surfari feature, with cursor
updates removed from the unsupported list.

## Completion Review

Adversarial subagent review, fresh context, read-only.

Verdict: **Approved**. No findings.

The reviewer independently checked the main repo had not yet made the result
commit, the WebKit checkout was on `webkit-1452a439-issue-756-exp12` at
`cdfb8cbf86f7c5e52cef0b2f14e8ab30ceeea91c`, the patch archive matched the WebKit
commit, cursor enum mapping was correct, the smoke log contained the `0`, `2`,
`3` cursor sequence and `SMOKE_EXIT_STATUS=0`, and formatting, whitespace,
symbol, and linkage checks passed.

Focused re-review after adding `.gitattributes`: **Approved**. No findings. The
reviewer checked that the rule applies only to `*.patch`, only disables
`blank-at-eol` and `blank-at-eof` for generated patch archives, leaves normal
source/docs files with default whitespace handling, and keeps
`git diff --staged --check` and Prettier passing.
