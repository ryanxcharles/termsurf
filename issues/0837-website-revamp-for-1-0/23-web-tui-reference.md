# Experiment 23: Web TUI reference (Phase 4)

## Description

The first **Phase 4** (TermSurf-specific documentation) experiment, and the
issue's stated center of gravity: the **`web` TUI**. The existing
`components/webtui.mdx` predates a source audit and has **real inaccuracies**.
This experiment reworks it into a fully **source-verified** reference, checked
line-by-line against `webtui/src/main.rs`.

Audit findings (current page vs. `webtui/src/main.rs`):

1. **Mode count is wrong.** The page says "four modes"; the source `Mode` enum
   (`main.rs:49`) has **six**: `Browse`, `Control`, `Edit`, `Command`,
   **`Dialog`**, **`Auth`**.
2. **`q` does not quit.** The page lists `q` → Quit in Control mode. The Control
   handler (`main.rs:955-1042`) has **no `q`**. Quit is **`Ctrl+C`** (any mode,
   `main.rs:756`) or the `:quit`/`:q` command (`main.rs:1101`).
3. **`Cmd+C` copy-URL is missing.** Control mode binds `Cmd+C` (SUPER+C) to copy
   the current URL (`main.rs:1016`), disabled in DevTools — the page omits it.
4. **The `:viewport` command is missing entirely.** `COMMANDS` (`main.rs:206`)
   includes `viewport`/`vp` with `height <rows>` and `reset` (`main.rs:221`);
   the page documents only quit/dark/devtools.
5. **Command aliases/args are incomplete.** `dark`/`da` accepts `on|yes|y`,
   `off|no|n`, `system|s`, or none (toggle); the `vp` shortcut and these aliases
   are missing.
6. **No URL-resolution docs.** `resolve_input` (`main.rs:1530`) defines the
   smart URL rules — core to the UX — and the page doesn't cover them.
7. **Browse-mode "keybindings" are browser shortcuts.** `Cmd+[`/`Cmd+]`/`Cmd+R`
   are not TUI bindings; in Browse mode the TUI forwards everything except `Esc`
   to the browser (`main.rs:930-936`), so those are standard Chromium shortcuts.

## Key decisions

1. **Rework `components/webtui.mdx` in place** (keep route
   `/docs/components/webtui`, `section: Components`, `order: 1`). Components is
   still the transitional home for this page (per `website/CLAUDE.md` IA — it
   folds into a TermSurf group later); no nav/section change here.
2. **Every claim re-derived from `webtui/src/main.rs`.** Concretely:
   - **CLI** — usage `web [URL] [options]`; subcommands `url <url>`, `last`,
     `status`, `file <path>` (`main.rs:314`); global flags
     `-p/--profile <name>`, `--incognito`, `-b/--browser <name|path>`,
     `--primary-screen` (`main.rs:288-312`). `--incognito` can't combine with
     `--profile` unless the profile is `incognito`; default profile is
     `default`; **profile names must be lowercase alphanumeric starting with a
     letter** (`main.rs:349-358`).
   - **URL resolution** (`resolve_input`, `main.rs:1530`), in source order: (1)
     has a `scheme://` → used as-is; (2) starts with `/`, `./`, `../` **and the
     file exists** (canonicalizes) → `file://`; (3) contains `:` → `host:port`
     (`http://` for localhost, else `https://`); (4) the bare string names an
     **existing file** → `file://`; (5) a dotted domain → `https://` (localhost
     → `http://`); otherwise not a URL/file. (Order matters only for exotic
     inputs, e.g. a string with `:` that also names a file resolves via the
     `host:port` rule, and the path/file rules require the file to actually
     exist.) Examples: `web example.com` → `https://example.com`;
     `web localhost:3000` → `http://localhost:3000`; `web ./page.html` →
     `file://…`.
   - **Six modes**: Control (default), Browse, Edit (with edtui
     Normal/Insert/Visual/Search submodes), Command, Dialog (JavaScript
     dialogs), Auth (HTTP auth).
   - **Per-mode keys** (verified): Control — `i`/`A`/`I` (edit URL: insert at
     cursor/end/start), `n` (normal-submode edit), `v`/`V` (visual / visual
     line), `:` (Command), `Cmd+C` (copy URL), `Enter` (Browse); the edit/copy
     keys are disabled in DevTools. Browse — `Esc` → Control, everything else to
     the browser (so `Cmd+[`/`]`/`R` are Chromium's back/forward/reload). Edit —
     `Esc` in Normal submode → Control, `Enter` (outside Search submode)
     navigates and switches to Browse (also guarded by `!is_devtools` —
     `main.rs:1050`); vim editing via edtui. Command — type the command, `Enter`
     runs it, `Esc` (Normal submode) → Control. Dialog — `Enter` accepts
     (submits prompt text), `Esc` cancels, and `y`/`n` for confirm/beforeunload.
     Auth — `Tab` switches the username/password field, type to fill, `Enter`
     advances then submits, `Esc` cancels. Global — `Ctrl+C` quits from any
     mode.
   - **Commands** (`COMMANDS`, `main.rs:206`): `quit`/`q`; `dark`/`da`
     `[on|off|system]` (no arg = toggle); `viewport`/`vp` `height <rows>` |
     `reset`; `devtools`/`de` `[right|down|left|up]` (default `right`). Commands
     use exact name/alias matching.
3. **Accuracy — no stale/invented claims.** Remove the `q`-quits row; add
   `Cmd+C`, the Dialog/Auth modes, the `:viewport` command, and URL resolution.
   Do **not** assert IPC internals from possibly-stale code comments (the
   `main.rs:360` "XPC" comment contradicts the socket architecture — IPC is the
   Architecture page's concern, not this page). For `--browser`, document
   `<name | absolute path>` and use the verified absolute-path form; defer
   engine specifics to the (Phase-4) Browser Engines page / Roamium component
   rather than hardcoding engine names that may drift.
4. **Design system, zero JS.** Plain MDX → `prose-termsurf`; tables + the
   existing `bg-background-dark` `<pre>` token style; semantic tokens only;
   links only to **built** pages (`/docs/components/roamium`,
   `/docs/reference/keybindings`, `/docs/architecture`).

## Changes

Files in `website/`:

1. **`src/content/docs/components/webtui.mdx`** — rewritten, source-verified:
   Overview, CLI (subcommands/flags/profiles), URL resolution, the six Modes,
   per-mode Keybindings, Commands, and brief DevTools/dark-mode notes. Drops the
   inaccurate `q`-quit and the stale loading-screen/IPC specifics that can't be
   verified from source; keeps only what `main.rs` supports.

No other files change: schema, `docs-nav.ts`, generated references, other pages,
and the fork are untouched. Page count stays **80** (rework of an existing
page).

## Verification

1. **Accuracy (source-verified).** Every claim on the rebuilt page maps to
   `webtui/src/main.rs`: six modes (`:49`), the CLI surface (`:288-330`),
   profile validation (`:349`), URL rules (`:1530`), per-mode keys (`:756`,
   `:930-1042`, `:1044-1086`, `:1087-1101`, dialog `:770-800`, auth `:842-876`),
   and the four commands incl. `viewport` (`:206-247`). **No `q`-quit claim**;
   `Ctrl+C` and `:quit`/`:q` are the quit paths; `Cmd+C` copy-URL present;
   `:viewport` present. Spot-check each against the source.
2. **Builds + checks.** `bun run build` 80 pages; `bunx astro check` 0 errors;
   `gen:references --check` + `import:vt --check` exit 0.
3. **Design system, zero JS, links resolve.** `prose-termsurf`; no hardcoded
   hex; no `<astro-island>` beyond the inherited Pagefind search; dead-link
   crawl over `/docs/components/webtui` = 0 broken.
4. **a11y.** Exactly one `<h1>` ("Web TUI"), ordered `<h2>`/`<h3>` (no skipped
   levels); descriptive link text.
5. **No regressions.** Sidebar/search/other pages/`/`/`/welcome` unchanged; the
   page keeps its route and nav position.

A full pass gives an accurate, source-verified `web` TUI reference — the issue's
center of gravity. Next Phase-4 candidates: the end-to-end UX story ("How
TermSurf Works"), the protocol refresh, Browser Engines (Roamium + roadmap),
Ghostboard's pane-border additions, and the roadmap.

## Design Review

Independent `adversarial-reviewer`. **Verdict: APPROVE** (no Required findings).
The reviewer verified every load-bearing claim against `webtui/src/main.rs`: six
modes (`:48-56`); no `q`-quit in the Control handler, quit is `Ctrl+C` (`:756`,
CONTROL) and `:quit`/`:q` (`:1101`); Control keys `i`/`A`/`I`/`n`/`v`/`V`

- `:` + `Cmd+C` (SUPER, `:1016`) + `Enter`, with edit/copy guarded by
  `!is_devtools`; exactly four commands incl. the page's currently-missing
  `viewport`/`vp`; the `resolve_input` rules and the three examples; the full
  CLI surface + profile validation + incognito/profile mutual-exclusion; Dialog
  (Enter/Esc, y/n for confirm/beforeunload, prompt input) and Auth
  (Tab/Enter/Esc) keys; Browse forwarding everything but `Esc` (so
  `Cmd+[`/`]`/`R` are Chromium shortcuts); and that dropping the unverifiable
  loading-screen specifics + the stale `main.rs:360` "XPC" comment is correct.
  Confirmed scope (one MDX file), link targets exist, page count stays 80, and
  deferring `--browser` engine names (help string
  `"chromium", "plusium", or absolute path`) is the right call. Two **Optional**
  refinements, folded in:

1. URL-resolution ordering: the bare existing-file check runs **after** the
   `host:port` rule, and the path/file rules require the file to **exist**
   (canonicalize). The bullet now lists the rules in source order with that
   caveat.
2. Edit-mode `Enter`-navigate is also `!is_devtools`-guarded (`main.rs:1050`) —
   noted.

## Result

**Result:** Pass

The Web TUI page is reworked into an accurate, source-verified reference; all
criteria pass.

### What was built

`src/content/docs/components/webtui.mdx` rewritten in `prose-termsurf`:
**Command line** (subcommands `url`/`file`/`last`/`status`; flags
`-p/--profile`, `--incognito`, `-b/--browser <name|path>`, `--primary-screen`;
profile-name rule; incognito/profile exclusion; the dev `--browser <path>`
form); **URL resolution** (the six ordered `resolve_input` rules + an examples
table); **Modes** (all six); **Keybindings** (per-mode tables for Control incl.
`Cmd+C`, Browse, Edit, Command, Dialog, Auth, and the global `Ctrl+C` quit);
**Commands** (quit/dark/**viewport**/devtools with aliases and args); **DevTools
and dark mode**. The stale `q`-quit row, the unverifiable loading-screen stages,
and the "XPC" IPC claim are gone.

### Verification results

1. **Accuracy (source-verified)** — every claim maps to `webtui/src/main.rs`
   (verified at the design gate line-by-line and re-confirmed in the built
   page): six modes, the CLI surface, profile validation, the URL rules,
   per-mode keys, and the four commands incl. `viewport`. The built page has
   **no** Control-mode `q`-quit (the only "q…Quit" is the legitimate `:q`
   command alias); `Cmd+C` copy-URL, the Dialog/Auth modes, `:viewport`, and URL
   resolution are all present. **Pass.**
2. **Builds + checks** — `bun run build` 80 pages (rework, count unchanged);
   `bunx astro check` 0 errors; `gen:references --check` + `import:vt --check`
   exit 0. **Pass.**
3. **Design system, zero JS, links resolve** — `prose-termsurf`; no hardcoded
   hex; 0 `astro-island`; dead-link crawl over `/docs/components/webtui` = 0
   broken (links to roamium / keybindings / configuration / architecture all
   resolve). **Pass.**
4. **a11y** — one `<h1>` ("Web TUI") → ordered `<h2>`/`<h3>` (Command line, URL
   resolution, Modes, Keybindings[Control/Browse/Edit/Command/Dialog/Auth/Any],
   Commands, DevTools), no skipped levels. **Pass.**
5. **No regressions** — only `webtui.mdx` changed; route/nav position unchanged;
   sidebar/search/`/`/`/welcome`/other pages unaffected. **Pass.**

## Conclusion

The `web` TUI reference — the issue's center of gravity — is now accurate to the
source: six modes, the real keybindings (no phantom `q`-quit, the previously
missing `Cmd+C` and Dialog/Auth modes added), the full command set (the
previously missing `:viewport` added), the CLI surface, and the smart
URL-resolution rules. Next Phase-4 candidates: the end-to-end UX story ("How
TermSurf Works"), the protocol refresh, Browser Engines (Roamium + roadmap),
Ghostboard's pane-border additions, and the roadmap.

## Completion Review

Independent `adversarial-reviewer` at the result gate. **Verdict: APPROVE** (no
Required findings). The reviewer independently re-derived every claim from
`webtui/src/main.rs` with line cites — six modes (49-56), no phantom `q`-quit
(Control 955-1042; the only "q…Quit" is the `:q` alias), `Cmd+C`/edit keys
`!is_devtools`-guarded (957-1017), Browse forwards all but Esc (930-936), Edit
`Enter` double-guarded (1048-1050), all four commands incl. `:viewport`
(206-247), the CLI surface + profile rule + incognito/profile exclusion
(298-358), the URL-resolution order matching `resolve_input` (1534-1569) incl.
bare-file-after-host:port, and Dialog/Auth keys (760-876). Confirmed no XPC /
loading-screen / "four modes" stale text remains; 80 pages; `astro check` 0
errors; drift checks exit 0; all four links resolve; one `<h1>` + ordered h2/h3;
no hex; 0 `astro-island`; scope only `webtui.mdx`. Two **Nits**, one folded in:

- **(Nit, fixed)** URL rule 6 said "the URL bar shows an error," but on a failed
  resolve the error renders in the command bar (Edit mode switches to Command)
  or on stderr (CLI). Reworded to "nothing is navigated and an error is shown
  (in the command bar when editing, or on stderr from the CLI)."
- **(Nit, already covered)** the `./page.html → file://` example fires only if
  the file exists — the caveat is stated in the rules list above the table.
