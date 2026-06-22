# Experiment 11: Implement WebKit cursor callbacks

## Description

Experiment 10 proved that forwarded Cocoa mouse movement reaches WebKit's real
hover-hit-test path. Cursor updates are the next browser-state gap. Roamium
reports Chromium `ui::mojom::CursorType` integer values through
`ts_set_on_cursor_changed`, and Ghostboard currently renders the common web
values it receives:

- `0` = pointer/arrow;
- `2` = hand;
- `3` = i-beam.

WebKit's macOS cursor path does not expose an obvious `WKUIDelegate` callback.
The local source path is `Source/WebKit/UIProcess/mac/PageClientImplMac.mm`,
where `PageClientImpl::setCursor(const WebCore::Cursor&)` converts WebKit cursor
state into AppKit cursor changes. This experiment should first implement the
smallest useful Surfari cursor support without modifying `webkit/src`: after
forwarded mouse movement, observe the AppKit cursor WebKit set in the Surfari
process, map the common `NSCursor` values back to the Chromium-compatible
integer values Ghostboard already understands, suppress duplicates, and fire
`g_callbacks.on_cursor_changed`.

If observing `NSCursor.currentCursor` after WebKit handles forwarded mouse
movement is not reliable, the experiment should stop as **Partial** and record
the exact reason. A later experiment can then decide whether to patch WebKit's
`PageClientImpl::setCursor` or add another source-build hook.

This experiment should not create the Surfari Rust binary, modify Ghostboard,
modify `termsurf.proto`, implement console messages, implement renderer crash
reporting, implement DevTools, or edit `webkit/src`.

## Changes

- Study local cursor references:
  - `Source/WebKit/UIProcess/mac/PageClientImplMac.mm`;
  - `Source/WebKit/WebProcess/WebCoreSupport/WebChromeClient.cpp`;
  - `chromium/src/ui/base/cursor/mojom/cursor_type.mojom`;
  - `chromium/src/content/libtermsurf_chromium/ts_tab_observer.cc`;
  - `ghostboard/macos/Sources/Ghostty/Surface View/SurfaceView_AppKit.swift`.
- Add duplicate-suppressed cursor callback firing to
  `surfari/libtermsurf_webkit/src/libtermsurf_webkit.mm`.
- Map at least the currently supported Ghostboard cursor values:
  - `NSCursor.arrowCursor`/unknown/default -> Chromium `kPointer` (`0`);
  - `NSCursor.pointingHandCursor` -> Chromium `kHand` (`2`);
  - `NSCursor.IBeamCursor` -> Chromium `kIBeam` (`3`).
- Trigger cursor observation only after WebKit has handled real forwarded mouse
  movement, not from JavaScript or local DOM hit testing.
- Extend `surfari/libtermsurf_webkit/test-content/navigation.html` with
  deterministic regions for pointer, hand, and i-beam cursor behavior.
- Extend `surfari/libtermsurf_webkit/smoke-test/smoke_test.c` to move through
  those regions and fail unless it observes the expected callback sequence with
  no duplicate callback for repeated movement over the same cursor type.
- Keep Experiment 6/7/8/9/10 smoke coverage intact: lifecycle, navigation,
  resize, focus, mouse, scroll, keyboard, color scheme, target URL, JavaScript
  dialogs, and HTTP auth must still pass.
- Update `surfari/libtermsurf_webkit/README.md` so cursor updates move from
  unsupported to implemented only if the real WebKit/AppKit cursor path is
  proven.

## Verification

Start from a clean TermSurf repo root:

```bash
git status --short
git -C webkit/src status --short
git -C webkit/src rev-parse HEAD
git -C webkit/src rev-parse --abbrev-ref HEAD
git -C webkit/src rev-parse --is-shallow-repository
```

Build and run the smoke test:

```bash
surfari/libtermsurf_webkit/build.sh

mkdir -p logs
DYLD_FRAMEWORK_PATH="$PWD/webkit/src/WebKitBuild/Debug" \
surfari/libtermsurf_webkit/build/smoke-test \
  "$PWD/surfari/libtermsurf_webkit/test-content/index.html" \
  "$PWD/surfari/libtermsurf_webkit/test-content/navigation.html" \
  > logs/issue756-exp11-cursor.log 2>&1
rc=$?
echo "SMOKE_EXIT_STATUS=$rc" >> logs/issue756-exp11-cursor.log
```

The smoke log must prove:

- Experiment 6/7/8/9/10 evidence still passes.
- Cursor callbacks are emitted only after forwarded mouse movement is delivered
  to WebKit.
- Moving over the deterministic plain region emits pointer/arrow `0`.
- Moving over the deterministic link emits hand `2`.
- Moving over the deterministic text/input region emits i-beam `3`.
- Repeated movement over the same cursor region does not emit duplicate cursor
  callbacks.
- The smoke harness fails, rather than merely logging, if the cursor callback
  sequence is not the expected pointer/hand/i-beam sequence.

Verify symbols/linkage and checkout state:

```bash
nm -gU surfari/libtermsurf_webkit/build/libtermsurf_webkit.dylib | rg ' _ts_|_ts_webkit_test' | sort
otool -L surfari/libtermsurf_webkit/build/libtermsurf_webkit.dylib | rg 'WebKit|JavaScriptCore|libtermsurf'
otool -L surfari/libtermsurf_webkit/build/smoke-test | rg 'WebKit|JavaScriptCore|libtermsurf'
git diff --check
prettier --check --prose-wrap always --print-width 80 \
  issues/0756-surfari/README.md \
  issues/0756-surfari/11-webkit-cursor-callbacks.md
git -C webkit/src status --short
git -C webkit/src rev-parse HEAD
git -C webkit/src rev-parse --abbrev-ref HEAD
git -C webkit/src rev-parse --is-shallow-repository
```

There is no project-configured formatter for Objective-C++ or C in
`surfari/libtermsurf_webkit`; keep those edits local-style consistent and use
`git diff --check` as the whitespace guard.

**Pass** = cursor callbacks work through WebKit's real AppKit cursor-setting
path, the smoke test exits 0, all prior evidence still passes, the README
reflects support, and `webkit/src` remains unchanged.

**Partial** = WebKit cursor state cannot be observed reliably from
`NSCursor.currentCursor` after forwarded mouse movement, or additional common
cursor types require a WebKit source hook. The result must record the exact
API/input blocker and the next experiment.

**Fail** = the implementation regresses prior lifecycle/input/focus/target
URL/dialog/auth coverage, fakes cursor changes from JavaScript or local DOM hit
testing, or cannot identify a concrete next step.

## Design Review

Adversarial subagent review, fresh context, read-only.

Verdict: **Approved**. No findings.

## Result

**Result:** Partial

The no-WebKit-patch approach was tested and rejected. The attempted
implementation observed AppKit cursor state after forwarded mouse movement and
then tried a stronger in-process `NSCursor.set()` observer. It also tried
warping the global cursor to the forwarded mouse position before dispatching the
synthetic Cocoa mouse event.

The smoke test still observed only a pointer/arrow cursor callback:

```text
CALLBACK target_url url=https://example.test/surfari-target
CALLBACK target_url url=
CALLBACK cursor cursor_type=0
SMOKE_FAIL cursor callback count mismatch
SMOKE_EXIT_STATUS=1
```

The useful finding is in WebKit's local source:
`Source/WebKit/UIProcess/mac/PageClientImplMac.mm` has
`PageClientImpl::setCursor(const WebCore::Cursor&)`, which rejects cursor
updates unless the real active window and global mouse-location guards pass.
Surfari's synthetic mouse movement is sufficient for WebKit hover hit testing
and target URL callbacks, but it is not sufficient to reliably observe the
non-pointer AppKit cursor changes from outside WebKit.

The failed production/test edits were removed. Cursor updates remain listed as
unsupported in `surfari/libtermsurf_webkit/README.md`.

## Conclusion

Surfari needs a WebKit source hook for cursor updates. The next experiment
should patch the local WebKit branch at or near `PageClientImpl::setCursor` or
`WebPageProxy::setCursor`, expose the WebCore cursor type to
`libtermsurf_webkit`, map it to Chromium-compatible `ui::mojom::CursorType`
integer values, and then re-run the smoke proof for pointer, hand, and i-beam.

## Completion Review

Adversarial subagent review, fresh context, read-only.

Verdict: **Approved**. No findings.
