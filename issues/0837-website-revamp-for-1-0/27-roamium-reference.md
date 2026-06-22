# Experiment 27: Roamium reference rework (Phase 4)

## Description

A Phase-4 experiment reworking the **Roamium** page to be source-verified.
Roamium's page predates a source audit and is **incomplete by omission**: its C
API and callback tables are missing real, shipped FFI surface. Against
`roamium/src/ffi.rs` (32 `ts_*` functions) the page is missing:

- `ts_create_incognito_browser_context`, `ts_destroy_browser_context`
- `ts_reply_http_auth`, `ts_reply_javascript_dialog`, `ts_set_gui_active`
- the callbacks `ts_set_on_http_auth_request`,
  `ts_set_on_javascript_dialog_request`, `ts_set_on_console_message`,
  `ts_set_on_renderer_crashed`

These back features already documented elsewhere on the site — HTTP auth and
JavaScript dialogs (the Web TUI's Auth/Dialog modes, Exp 23) and renderer-crash
recovery (the 1.0 inventory). The fix completes the C API and callback tables
from the source, with each callback's protobuf message **verified to exist** in
`proto/termsurf.proto`.

## Key decisions

1. **Rework `components/roamium.mdx` in place** (route
   `/docs/components/roamium`, `section: Components`, `order: 2`). Components
   remains the transitional home; no nav/section change.
2. **Complete the C API table from `ffi.rs`** (the authoritative 32-function
   set), regrouped:
   - **Lifecycle** — `ts_content_main`, `ts_set_on_initialized`, `ts_post_task`,
     `ts_quit`.
   - **Profiles (browser contexts)** — `ts_create_browser_context`,
     `ts_create_incognito_browser_context`, `ts_destroy_browser_context`.
   - **Tabs** — `ts_create_web_contents`, `ts_create_devtools_web_contents`,
     `ts_destroy_web_contents`.
   - **Navigation & input** — `ts_load_url`, `ts_forward_mouse_event`,
     `ts_forward_mouse_move`, `ts_forward_scroll_event`, `ts_forward_key_event`.
   - **State** — `ts_set_focus`, `ts_set_color_scheme`, `ts_set_view_size`,
     `ts_set_gui_active`.
   - **Dialog & auth replies** — `ts_reply_javascript_dialog`,
     `ts_reply_http_auth`.
   - Plus the `ts_set_on_*` callback registrations (listed under Callbacks).
3. **Complete the Callbacks table** — the `ts_set_on_*` set, each mapped to its
   protobuf message (all confirmed present in `proto/termsurf.proto`):
   `on_tab_ready`→`TabReady`, `on_ca_context_id`→`CaContext`,
   `on_url_changed`→`UrlChanged`, `on_loading_state`→`LoadingState`,
   `on_title_changed`→`TitleChanged`, `on_cursor_changed`→`CursorChanged`,
   `on_target_url_changed`→`TargetUrlChanged`,
   `on_console_message`→`ConsoleMessage`,
   `on_renderer_crashed`→`RendererCrashed`,
   `on_http_auth_request`→`HttpAuthRequest`,
   `on_javascript_dialog_request`→`JavaScriptDialogRequest`. (The request/reply
   pairs — auth and dialogs — show the round trip: a `*_request` callback to the
   GUI/TUI, answered by a `ts_reply_*` FFI call.)
4. **Accuracy — verified surface, no invention.** Every `ts_*` function on the
   page exists in `ffi.rs`; every callback's protobuf message exists in
   `proto/termsurf.proto`. Keep the conceptual two-layer architecture
   (`libtermsurf_chromium` C lib + ~400-line Rust binary), the source-layout
   table (update to the actual `roamium/src/` files: `main.rs`, `dispatch.rs`,
   `ipc.rs`, `ffi.rs`, **`proto.rs`** — there is no `build.rs` in `src/`), and
   the multi-engine pattern. The startup sequence is kept at the conceptual
   level and checked against the source for no contradiction; don't assert
   ordering the source doesn't support. Install paths
   (`/opt/homebrew/opt/termsurf-roamium/`) are unchanged (the
   issue-833-corrected paths).
5. **macOS-accurate, design system, zero JS.** Chromium/macOS; `prose-termsurf`;
   semantic tokens; links only to **built** pages (`/docs/architecture`,
   `/docs/components/webtui`, `/docs/protocol/messages`,
   `/docs/getting-started`).

## Changes

Files in `website/`:

1. **`src/content/docs/components/roamium.mdx`** — reworked: complete C API (32
   `ts_*`) + complete callbacks (with verified proto messages), corrected
   source-layout table (`proto.rs`, no `build.rs`), conceptual architecture /
   multi-engine / install retained.

No other files change: schema, `docs-nav.ts`, generated references, the fork,
and `proto/termsurf.proto` are untouched. Page count stays **83** (rework).

## Verification

1. **Accuracy (source-verified).** Every `ts_*` on the page is in `ffi.rs` (32
   functions; none invented, none missing the nine previously-omitted ones); the
   source-layout table matches `roamium/src/` (`main.rs`, `dispatch.rs`,
   `ipc.rs`, `ffi.rs`, `proto.rs`; no `build.rs`); every callback's protobuf
   message exists in `proto/termsurf.proto`. Spot-check against the source.
2. **Builds + checks.** `bun run build` 83 pages; `bunx astro check` 0 errors;
   `gen:references --check` + `import:vt --check` exit 0.
3. **Design system, zero JS, links resolve.** `prose-termsurf`; no hardcoded
   hex; no `<astro-island>` beyond the inherited Pagefind search; dead-link
   crawl over `/docs/components/roamium` = 0 broken.
4. **a11y.** Exactly one `<h1>` ("Roamium"), ordered `<h2>`/`<h3>` (no skipped
   levels); descriptive link text.
5. **No regressions.** Route/nav position unchanged;
   sidebar/search/`/`/`/welcome`/ other pages unchanged.

A full pass makes the Roamium reference complete and source-accurate (the
HTTP-auth / JS-dialog / console / crash-recovery / incognito FFI surface now
documented). Next Phase-4 candidate: the protocol refresh (overview + the
41-message reference, verified against `proto/termsurf.proto`), after which
Phase 4 is complete.

## Design Review

Independent `adversarial-reviewer`. **Verdict: APPROVE** (no Required findings).
The reviewer confirmed the C API is a **1:1 match** with `ffi.rs` — exactly 32
functions across the seven groups (4 Lifecycle + 3 Profiles + 3 Tabs + 5
Nav/input + 4 State + 2 Dialog/auth replies + 11 callbacks), none invented, none
omitted, and all nine previously-missing functions confirmed absent from the
current page; every one of the 11 callback→protobuf mappings exists in
`proto/termsurf.proto` **and** is actually constructed in `dispatch.rs` (real
emission, not invention); the source-layout correction is right
(`main/dispatch/ipc/ffi/proto.rs`; `build.rs` is at `roamium/build.rs`, not in
`src/`); the retained two-layer architecture / ~400-line binary / multi-engine /
install-path claims match root `CLAUDE.md`; the kept startup-sequence names
(`ServerRegister`, `CreateTab`) exist in the proto; scope is one reworked MDX
file with unchanged route/links. One **Optional**: the "83 pages" count is a
build-time check, confirmed at the result gate.

## Result

**Result:** Pass

The Roamium reference is reworked to a complete, source-verified surface; all
criteria pass.

### What was built

`src/content/docs/components/roamium.mdx` rewritten in `prose-termsurf`: How it
works; the two-layer Architecture; a corrected **Source layout** (`main.rs`,
`dispatch.rs`, `ipc.rs`, `ffi.rs`, `proto.rs` — no `build.rs`); the complete **C
API** grouped into Lifecycle / Profiles (incl. incognito + destroy) / Tabs /
Navigation & input / State (incl. `ts_set_gui_active`) / Dialog & auth replies
(`ts_reply_javascript_dialog`, `ts_reply_http_auth`); the complete **Callbacks**
table (11 `on_*` → proto messages, adding console/renderer-crash/http-auth/JS-
dialog) with a request/reply round-trip note; Multi-engine pattern (linking the
roadmap); and Installation.

### Verification results

1. **Accuracy (source-verified)** — every `ts_*` in the tables is in `ffi.rs`
   (the only non-table token is the prose glob `ts_set_on_*`); the 21
   direct-call functions + 11 `ts_set_on_*` registrations cover all 32 `ffi.rs`
   functions; all 11 callback protobuf messages exist in `proto/termsurf.proto`;
   the source-layout table matches `roamium/src/`. **Pass.**
2. **Builds + checks** — `bun run build` 83 pages (rework, unchanged);
   `bunx astro check` 0 errors; `gen:references --check` + `import:vt --check`
   exit 0. **Pass.**
3. **Design system, zero JS, links resolve** — `prose-termsurf`; no hardcoded
   hex; 0 `astro-island`; dead-link crawl over `/docs/components/roamium` = 0
   broken (architecture / webtui / protocol-messages / getting-started / roadmap
   all resolve). **Pass.**
4. **a11y** — one `<h1>` ("Roamium") → ordered `<h2>`/`<h3>` (How it works …
   Installation), no skipped levels. **Pass.**
5. **No regressions** — only `roamium.mdx` changed; route/nav position
   unchanged; sidebar/search/`/`/`/welcome`/other pages unaffected. **Pass.**

(Implementation note: the first build failed on a multi-line `<a>` inside a
`<p>` — the same MDX-in-prose constraint as Exp 24 — fixed by keeping the anchor
on one line.)

## Conclusion

The Roamium reference is now complete and source-accurate: the full 32-function
C API (incognito/destroy contexts, dialog/auth replies, gui-active) and all 11
callbacks with verified protobuf messages, plus a corrected source layout. The
HTTP-auth / JS-dialog / console / renderer-crash surface that backs the Web
TUI's Auth/Dialog modes and crash recovery is now documented on the engine side
too. Last Phase-4 item: the protocol refresh (overview + the 41-message
reference, verified against `proto/termsurf.proto`).

## Completion Review

Independent `adversarial-reviewer` at the result gate. **Verdict: APPROVE WITH
CHANGES.** The page itself verified fully clean: the C API is a 1:1 cover of
`ffi.rs` (21 direct `ts_*` rows + 11 `on_*` callbacks = 32, none invented, all
nine previously-omitted present); all 11 callback protobuf messages exist in
`proto/termsurf.proto`; the source-layout table matches `roamium/src/` (no
`build.rs`); retained architecture/binary/multi-engine/install claims match root
`CLAUDE.md` and the "How it works" prose names no fabricated messages; 83 pages,
`astro check` 0 errors, drift checks exit 0, all five links resolve, one
`<h1>` + ordered h2/h3, no hex, 0 `astro-island`, scope only `roamium.mdx`. One
**Required** finding, fixed:

- **(Required) Doubled status line.** A `sed` slip when flipping the README
  index left the Exp 27 entry with a stray second `  **Pass**` line (rendering
  "Pass Pass"). **Resolved** by deleting the stray line so the entry is a single
  line ending `— **Pass**`.
