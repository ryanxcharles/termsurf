+++
[implementer]
agent = "claude-code"
model = "claude-opus-4-8"
reasoning = "high"

[review.design]
agent = "claude-code"
model = "claude-opus-4-8"
reasoning = "high"
+++

# Experiment 26: Phase C — clipboard copy of a selection (deferred Exp-20 probe)

## Description

Exp 25 wired mouse-drag selection. This experiment verifies **copy to the
clipboard** — the second half of the deferred Exp-20 probe. Unlike selection,
copy appears **wired** end to end:

- App: the `copy(_:)` IBAction (the standard Copy menu / ⌘C) calls
  `roastty_surface_binding_action(surface, "copy_to_clipboard", …)`
  (`SurfaceView_AppKit.swift:1599`).
- Rust: the bare string `"copy_to_clipboard"` parses
  (`copy_to_clipboard_format_from_str(None)` →
  **`CopyToClipboardFormat::Mixed`**, `lib.rs:4861`/`5259`) and dispatches to
  `Surface::copy_to_clipboard` (`lib.rs:2798`), which reads
  `active_selection()`, formats it
  (`selection_format(_, unwrap=true, trim=true, …)`), and for `Mixed` writes
  **two** entries — `("text/plain", plain)` + `("text/html", html)` — via
  `app.runtime.write_clipboard_cb`.
- App: `write_clipboard_cb` → `App.writeClipboard` → `NSPasteboard.setString`
  (`Roastty.App.swift:67,289,414`).

So this is a **probe**, and the binding→copy→clipboard mechanics are **already
proven** by existing green tests:
`surface_binding_action_copy_to_clipboard_writes_formats` (`lib.rs:23333`)
drives the real `binding_action(surface,"copy_to_clipboard")` path with a
`termio_worker` + `write_clipboard_record_cb` and asserts the recorded
`[(text/plain,…),(text/html,…)]`;
`surface_binding_action_copy_to_clipboard_false_paths` (`lib.rs:23280`) covers
no-selection→no-write

- null/freed-app→false. So copy is **not** an unwired gap like selection was.
  The **only untested seam** is the _end-to-end_ flow: a **drag-gesture**
  selection (Exp 25) → binding copy → clipboard (the existing copy tests set the
  selection programmatically, not via a mouse drag).

## Approach

**Phase 1 — one end-to-end integration test** (the binding→copy→clipboard
mechanics + no-selection case are already covered; do not duplicate them). Using
`new_test_app_with_clipboard_write` (records via `write_clipboard_record_cb`,
`lib.rs:15699/15662`) + a `termio_worker`:

1. `set_test_size_80x24` + feed text + **drag-select** via the Exp-25 path
   (`mouse_pos`/ `mouse_button`), so the selection comes from the _gesture_ (the
   untested seam), not `set_selection`.
2. `roastty_surface_binding_action(surface, "copy_to_clipboard", len)`.
3. Assert the recorded write contains a `("text/plain", …)` entry whose text
   equals the dragged substring (and an HTML entry exists — `Mixed` default).
   Locate the `text/plain` pair among the two; do not assert a lone-string
   equality.

**Phase 2 — live confirmation.** Launch with known text, drag-select it (the
Exp-25 `drag.swift` driver), trigger ⌘C, and read back the **system clipboard
with `pbpaste`** — its `text/plain` must equal the selected text. Primary: ⌘C
via a CGEvent key-down/up (`c` + command) at `.cghidEventTap` with Roastty
frontmost. Fallback (no app code added): **AX-press the Copy menu item**
(`menuCopy`, `AppDelegate.swift:1155`) via accessibility, which drives the real
`copy(_:)` IBAction → `roastty_surface_binding_action`. Clear the pasteboard
first (`pbcopy </dev/null`) so a stale value can't pass; Partial if neither can
be driven. App + descendant tree killed (0 dangling); screen unlocked (check
`CGSSessionScreenIsLocked`).

**No `libroastty` change is expected** (copy is already wired + unit-tested);
this experiment adds the one missing _integration_ test (gesture-select → copy)
and the live system-clipboard proof. Code changes only if the live probe
surfaces a real gap.

## Verification

1. **Headless integration test:** a **drag-gesture** selection → binding-action
   `copy_to_clipboard` records a `text/plain` entry equal to the dragged
   substring (+ an HTML entry). `cargo test -p roastty` (full) green. (The
   binding→copy mechanics + no-selection→no-write are already covered by
   existing tests — not duplicated.)
2. **Live confirmation:** ⌘C after a live drag-select makes `pbpaste` return the
   selected text (pasteboard pre-cleared). Out-of-repo notes; app cleaned up (0
   dangling).
3. Faithful to upstream copy semantics — the `Mixed` (plain+html) default +
   `selection_format` unwrap/trim — cite.

**Pass** = the binding-action copy records the selection headlessly, the suite
is green, and live ⌘C puts the selected text on the system clipboard
(`pbpaste`).

**Partial** = headless copy works + tested, but the live ⌘C can't be driven from
the harness (documented as a tooling limit, with the headless + binding path
proving the logic).

**Fail** = copy doesn't put the selection on the clipboard (a real gap — then
fix it and re-probe).

## Design Review

**Reviewer:** `adversarial-reviewer` subagent (Claude Opus, fresh context,
read-only). **Verdict: CHANGES REQUIRED → addressed.** It confirmed the binding
`"copy_to_clipboard"` resolves to `CopyToClipboard` →
`Surface::copy_to_clipboard` and the recording-cb + worker combo is feasible.
Two Required + an Optional + a Nit, folded in:

- **Required — format is `Mixed`, not `Plain`** (bare `"copy_to_clipboard"` →
  `Mixed` → two entries `text/plain` + `text/html`); the assertion must locate
  the `text/plain` pair, not a lone string. **Fixed.**
- **Required — the binding→copy→clipboard mechanics are ALREADY covered** by
  `surface_binding_action_copy_to_clipboard_writes_formats` + `_false_paths` (so
  the "likely gap" is already closed; don't duplicate). **Fixed:** re-scoped
  Phase 1 to the one untested seam — a **drag-gesture** selection → binding copy
  (end-to-end), acknowledging the existing tests.
- **Optional — the live fallback shouldn't add app code;** use the **AX
  Copy-menu** route instead of a probe path. **Fixed.**
- **Nit — faithfulness cite** `Mixed`/trim, not Plain. **Fixed.**

**Re-review: APPROVED.** Confirmed the integration test is genuinely
non-duplicative (the existing copy tests set the selection via
`set_surface_worker_active_selection`, never the drag gesture; the drag test
never invokes copy — the join is novel), feasible
(`new_test_app_with_clipboard_write` + `termio_worker` + the drag path +
`binding_action` all compose), and the `text/plain`-pair assertion is right.
Optional folded in: select a substring with **no trailing whitespace** in any
row (copy's `trim=true` strips it), e.g. "TARGET" — exactly the Exp-25 probe
text.

## Result

**Result:** Pass — copy of a drag-selection reaches the system clipboard. **No
`libroastty` code change** (copy was already wired + unit-tested); this
experiment adds the missing _integration_ test and the live system-clipboard
proof.

### Verification

- **Headless integration test** `mouse_drag_then_copy_to_clipboard` (`lib.rs`):
  a **drag-gesture** selection (the Exp-25 `mouse_pos`/`mouse_button` path) of
  "TARGET", then `roastty_surface_binding_action(surface, "copy_to_clipboard")`,
  records a `("text/plain", "TARGET")` entry (Mixed default — plain + html) on
  the test clipboard. This joins the drag gesture to copy — the one seam the
  existing copy tests (which set the selection programmatically) didn't cover.
  Fails if the gesture-selection doesn't reach copy; passes.
- **Full `cargo test -p roastty`:** lib **4409 passed**, 0 failures.
- **Live confirmation** (screen unlocked; app + descendant tree killed, 0
  dangling): pasteboard pre-set to a stale sentinel (`CLIPBOARD_PROBE_STALE`);
  launched with `echo DRAGSELECTME_TARGET_HERE`; drag-selected a span
  (`drag.swift`), then **Edit ▸ Copy** (AX menu via `osascript`, driving the
  real `copy(_:)` IBAction → `roastty_surface_binding_action`) — **`pbpaste`
  returned `SELECTME_`** (the highlighted span), replacing the sentinel. So the
  full path drag→select→copy→`write_clipboard_cb`→`NSPasteboard` works end to
  end. (CGEvent ⌘C did **not** land — keyboard events are focus-dependent, the
  Exp-19/20 caveat; the Edit▸Copy menu is what ⌘C triggers when focused, so this
  proves the copy path; the ⌘C focus issue is a harness limitation, not an app
  bug.)

## Conclusion

Clipboard copy of a selection works end to end (headless integration + live
`pbpaste`), faithful to the `Mixed` (plain+html) default with `selection_format`
trim. **Both halves of the last Exp-20-deferred probe — mouse selection (Exp 25)
and clipboard copy (Exp 26) — are now done.** The remaining work is refinements
(selection word/line double/triple-click + drag-autoscroll +
shift-while-reporting; the reporting clear+reset widening; CJK ideographic
wide-pitch; CVDisplayLink vsync; DPI-change rebuild) and the `shape_run_options`
cursor-shaping-hint viewport-gating — none a core conformance gap. Issue 802's
core goal (a faithful, feature-conformant renamed-Ghostty app on libroastty) is
essentially met; the next step is to weigh closing the issue vs. continuing the
refinements.

## Result Review

**Reviewer:** `adversarial-reviewer` subagent (Claude Opus, fresh context,
read-only). **Verdict: APPROVED.** It tried to break the result on vacuity,
path, suite, and Pass-honesty; all held. The test **passes + is load-bearing**
(the selection comes from the real gesture; if the drag selected nothing/a wider
span the exact `text/plain == "TARGET"` assertion fails;
`reset_clipboard_write_records` + `supports_selection_clipboard=false` mean the
recorded entry can only come from the copy — the genuinely novel drag→copy
seam). Full lib **4409 passed, 0 failed**. **Pass is honest re: ⌘C** — verified
in source that the Edit▸Copy menu item's action is `#selector(copy(_:))` and ⌘C
drives the same first-responder `copy:` selector, so the copy _path_
(drag→select→IBAction→`binding_action`→`write_clipboard`→`NSPasteboard`) is
proven live; only the generic AppKit keystroke→selector binding was unexercised
(disclosed), and the design pre-sanctioned the AX Copy-menu route. Live evidence
sound (stale sentinel **replaced** by "SELECTME\_" — stronger than a bare
clear). Scope clean (test-only diff, one hunk inside `mod tests`). Nits:
Pass-bullet wording (fixed → "⌘C or the equivalent Edit▸Copy action");
`keychord.swift` unused-but-harmless (kept as a documented probe artifact,
parallel to `drag.swift`). Informational (pre-existing, NOT this diff):
`lib.rs:4506,4531` + comments `14399/14420` carry `ghostty` literals predating
this work — a separate cleanup, out of scope for Exp 26.
