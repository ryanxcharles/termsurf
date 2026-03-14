# TermSurf

TermSurf is a protocol for embedding web browsers inside terminal emulators. Any
terminal, any browser engine, any TUI — connected by a protobuf/Unix socket
protocol. Users type `web localhost:3000` and see their work without ever
leaving the terminal. No alt+tab, no context switch.

[Agent development guide](https://agents.md/).

## Rules

Do exactly what your user says. No more, no less. NEVER assume they want
something they didn't ask for. NEVER change code unless explicitly asked.

## Vision

TermSurf is a protocol, not just an app. It is a network of interchangeable
components — terminals, browser engines, and TUIs — all speaking the same
protobuf/Unix socket protocol (`termsurf.proto`).

### Cross-platform

TermSurf will work on macOS, Linux, and Windows. iOS and Android can be added
later.

### Every major browser engine

Each browser engine runs as a separate "profile server" process, communicating
with the GUI (terminal emulator) via the TermSurf protocol. One process per
profile.

| Engine   | C library              | Rust binary | Status     |
| -------- | ---------------------- | ----------- | ---------- |
| Chromium | `libtermsurf_chromium` | Roamium     | Done       |
| WebKit   | `libtermsurf_webkit`   | Surfari     | Planned    |
| Gecko    | `libtermsurf_gecko`    | Waterwolf   | Researched |
| Ladybird | `libtermsurf_ladybird` | Girlbat     | Researched |

Each engine follows the same pattern: a C shared library wrapping the engine's
embedding API (`ts_*` functions), linked by a Rust binary that handles Unix
socket IPC, protobuf parsing, and process lifecycle. The Rust binary (~400
lines) is almost entirely reusable across engines.

### Multiple GUIs

TermSurf currently ships as a WezTerm fork (`wezboard/`). We will implement
forks of all major terminal emulators:

- **Ghostboard** — Archived. Will return from a fresh Ghostty fork after the
  protocol stabilizes.
- **Wezboard** (wezboard/) — Active GUI. WezTerm fork, Rust. Full protocol
  support, CALayerHost rendering, input forwarding, DevTools, direct TUI↔Browser
  connection (Issues 715–741).
- **Kitty** — Planned.
- **Alacritty** — Planned.
- **iTerm2** — Planned.

Any terminal that implements the TermSurf protocol can host browser overlays. A
GUI is a terminal emulator that listens on a Unix socket, accepts connections
from TUIs and browser engines, and renders browser content as overlays at pixel
coordinates.

### Many TUIs

The first TUI, `web`, provides browser chrome (URL bar, navigation, modes) in
the terminal pane. But TermSurf is really a webview overlay protocol — many TUIs
can embed web browsers with any engine:

- `web` — General-purpose web browser TUI (current)
- Future TUIs could include: documentation viewers, API explorers, email
  clients, dashboard monitors, or any application that benefits from rendering
  web content inside a terminal.

### The protocol is the product

The TermSurf protocol (`termsurf.proto`) is the most important artifact. It
defines 30 message types covering tab lifecycle, navigation, input forwarding,
GPU compositing, state synchronization, and request/reply pairs. The protocol
will be extended to support:

- All common web browser features (bookmarks, history, downloads, etc.)
- Terminal-specific features (keyboard-based navigation, shrink/grow overlay,
  split management)
- New message types as needs arise

Care goes into the protocol first. Individual apps (boards, engines, TUIs) are
implementations of the protocol.

## Architectural Decisions

### Multi-process architecture

TermSurf is multi-process by necessity, not by choice. Each browser engine
process serves exactly one profile (one set of cookies, storage, and cache).
This is a hard constraint imposed by Chromium (one `BrowserContext` per
process), and Gecko and Ladybird have the same limitation. This constraint is
the defining architectural fact of TermSurf — it shaped every generation from
ts2 onward.

The multi-process design has a second benefit: it enables multi-engine support.
Because each browser process is an independent program speaking the TermSurf
protocol, the GUI doesn't care which engine is behind it. A user can have one
pane running Roamium (Chromium), another running Surfari (WebKit), and a third
running Girlbat (Ladybird) — all in the same terminal window, all speaking the
same protobuf messages.

WebKit is the exception — `WKWebsiteDataStore` supports multiple profiles in one
process — but the one-process-per-profile model still works for it, and keeping
the architecture uniform is more valuable than optimizing for one engine.

**Process topology:**

```
┌─────────┐  ┌─────────┐  ┌─────────┐
│  TUI 1  │  │  TUI 2  │  │  TUI N  │    N TUIs (e.g., `web`)
└────┬────┘  └────┬────┘  └────┬────┘
     │            │            │
     └────────────┼────────────┘
                  │  Unix socket
           ┌──────┴──────┐
           │     GUI     │                1 GUI (terminal emulator)
           │ (Wezboard)  │
           └──┬───┬───┬──┘
              │   │   │
              │   │   │  Unix sockets
              │   │   │
     ┌────────┘   │   └──────┐
     │            │          │
┌────┴────┐ ┌─────┴───┐ ┌────┴────┐
│ Roamium │ │ Surfari │ │ Roamium │    M engines (one per profile)
│ profile │ │ profile │ │ profile │
│   "A"   │ │   "B"   │ │   "C"   │
└─────────┘ └─────────┘ └─────────┘
```

N TUIs connect to 1 GUI. The GUI manages M browser engine processes, each
serving one profile. Different profiles can use different engines. The GUI is
the hub — it routes messages between TUIs and engines, manages pane layout, and
composites browser overlays into the terminal window.

### Unix sockets + protobuf for all IPC

All inter-process communication uses Unix domain sockets with length-prefixed
protobuf messages. The GUI (terminal) listens on a PID-scoped socket
(`$TMPDIR/termsurf/termsurf-wezboard-{pid}.sock`), and both TUIs and browser
engines connect to it as clients.

- **TUI → GUI:** The TUI reads the `TERMSURF_SOCKET` env var (set by the GUI) to
  discover the socket path.
- **GUI → Engine:** The GUI passes `--ipc-socket={path}` when launching browser
  engine processes.
- **Wire format:** 4-byte little-endian length prefix + serialized protobuf
  (`termsurf.proto`).
- **Serialization:** prost in Rust (GUI, TUI, and engines), C++ protobuf in
  Chromium.

Earlier generations (ts3–ts5) used XPC for IPC. Issues 698–701 replaced XPC with
sockets, and Issue 702 removed all dead XPC code. CALayerHost compositing
(zero-copy GPU rendering) does not require XPC — Window Server routes
`CAContext` layer IDs between processes natively.

- **Graceful shutdown:** The GUI sends a `Shutdown` protobuf message to browser
  engine processes before terminating them (Issue 732 added the message, Issue
  733 made Ghostboard use it instead of SIGKILL). This allows engines to clean
  up resources and exit gracefully.

### Every Chromium issue gets its own branch

When modifying the Chromium fork (`chromium/src/`), ALWAYS create a new branch
for the current issue. Never commit directly to an existing issue's branch.

1. Find the most relevant recent branch (usually the one with the latest
   TermSurf modifications).
2. Create a new branch from it: `{version}-issue-{N}` (e.g.,
   `146.0.7650.0-issue-625`).
3. Add the new branch to the Branches table in `chromium/README.md`.

This keeps every issue's Chromium changes isolated and traceable.

## Directory Structure

- `ghostboard/` — Archived. See docs/early-prototypes.md.
- `wezboard/` — Wezboard (WezTerm fork, Rust). **Active development.**
- `webtui/` — The `web` TUI (Rust/ratatui). Browser chrome in the terminal pane.
- `roamium/` — Roamium (Chromium browser binary, Rust).
- `chromium/` — Chromium fork build workspace (gitignored).
- `issues/` — Issue documents across all generations (immutable history).
- `website/` — termsurf.com project website.
- `docs/early-prototypes.md` — Archived prototype documentation (ts1–ts5,
  cef-rs, Ghostboard Legacy).

## Wezboard (wezboard/) — Active Development

### Current State

Wezboard is a WezTerm fork with browser integration built in Rust. Current
additions: WezTerm fork with rename script and initial build (Issue 715), build
warning cleanup (Issue 716), cocoa crate removal and objc2 migration (Issues
717–719), manual testing after migration (Issue 720), wgpu 25→28 upgrade (Issue
721), cargo dependency updates (Issue 722), focused/unfocused split pane borders
(Issue 723), TermSurf protocol implementation (Issue 724), CALayerHost browser
overlay rendering (Issue 725), overlay lifecycle and protocol (Issue 726),
second webview positioning (Issue 727), remaining protocol messages (Issues
728–729), Roamium standalone install (Issue 730), scroll crash fix (Issue 731),
Shutdown message and tab reopen fix (Issue 732).

### Source Layout

#### Wezboard

- `wezboard/wezboard/src/main.rs` — Main GUI application entry point
- `wezboard/wezboard-gui/` — GUI rendering and window management
- `wezboard/wezboard-surface/` — Surface/pane rendering
- `wezboard/mux/` — Terminal multiplexer core
- `wezboard/termwiz/` — Terminal widget library
- `wezboard/config/` — Configuration parsing

#### Roamium

- `roamium/src/main.rs` — Entry point, process initialization and lifecycle
- `roamium/src/dispatch.rs` — Message dispatch and routing (core IPC handler)
- `roamium/src/ipc.rs` — Unix socket IPC protocol (socket framing)
- `roamium/src/ffi.rs` — FFI bindings to libtermsurf_chromium C library
- `roamium/build.rs` — Build script for protobuf code generation

### Build & Install

All build scripts live in `scripts/`. They handle Wezboard, Chromium, TUI, and
Roamium together.

| Script                                                   | Purpose                                                                  |
| -------------------------------------------------------- | ------------------------------------------------------------------------ |
| `scripts/build.sh <comp> [--release] [--clean] [--open]` | Build a component. Components: wezboard, roamium, webtui, chromium, all. |
| `scripts/install.sh <comp>`                              | Install a component. Components: wezboard, roamium, webtui, all.         |
| `scripts/uninstall.sh <comp>`                            | Uninstall a component. Components: wezboard, roamium, webtui, all.       |
| `scripts/rename-wezterm.sh [dir]`                        | Rename all WezTerm references to Wezboard in `wezboard/`. Re-runnable.   |
| `scripts/nerd-font-test.sh`                              | Print Nerd Font test glyphs for visual verification.                     |

The build scripts auto-detect Chromium's `protoc` so you don't need a system
install.

## Documentation

All documentation is in `docs/` or in `README.md` files throughout the codebase.

### Recent issues

Recent issues:

- `issues/0000700-tui-gui-sockets.md` — Replace TUI↔GUI XPC with Unix sockets
- `issues/0000701-chromium-sockets.md` — Replace GUI↔Chromium XPC with Unix
  sockets
- `issues/0000702-socket-cleanup.md` — Dead XPC removal and unlimited client
  connections
- `issues/0000703-remove-click-suppression.md` — Remove click-to-activate
  suppression
- `issues/0000704-browser-bindings.md` — Browser bindings (libtermsurf_content)
- `issues/0000705-browser-bindings.md` — Browser bindings continued (DevTools
  fix)
- `issues/0000706-plusium-devtools.md` — Plusium DevTools crash fix
- `issues/0000707-roamium.md` — Roamium (shared lib + Rust rewrite)
- `issues/0000708-roamium-only.md` — Roamium-only (clean fork, renamed lib)
- `issues/0000709-wezboard.md` — Wezboard (WezTerm fork research)
- `issues/0000710-gecko-webkit-ladybird.md` — Gecko, WebKit & Ladybird engine
  research
- `issues/0000711-rename-ghostboard-webtui.md` — Rename GUI to Ghostboard, TUI
  to webtui
- `issues/0000715-wezboard.md` — Wezboard (WezTerm fork, rename, initial build)
- `issues/0000716-wezboard-warnings.md` — Wezboard build warnings
- `issues/0000717-remove-cocoa-crate.md` — Remove `cocoa` crate from Wezboard
- `issues/0000718-finish-cocoa-removal.md` — Finish `cocoa` and `objc` 0.2
  removal
- `issues/0000719-wezboard-code-smells.md` — Wezboard code smells from objc2
  migration
- `issues/0000720-wezboard-manual-test.md` — Manual test after objc2 migration
- `issues/0000721-wgpu-upgrade.md` — Upgrade wgpu from 25 to 28
- `issues/0000722-cargo-deps.md` — Update outdated cargo dependencies
- `issues/0000723-pane-borders.md` — Split pane borders for Wezboard
- `issues/0000724-wezboard-protocol.md` — Implement TermSurf protocol in
  Wezboard
- `issues/0000725-wezboard-overlay.md` — Wezboard browser overlay rendering
- `issues/0000726-wezboard-overlay-lifecycle.md` — Overlay lifecycle and
  remaining protocol
- `issues/0000727-wezboard-second-webview.md` — Second webview positioning
- `issues/0000728-wezboard-remaining-protocol.md` — Complete remaining protocol
- `issues/0000729-wezboard-reposition-and-protocol.md` — Overlay reposition on
  resize
- `issues/0000730-roamium-standalone-install.md` — Roamium standalone install
- `issues/0000731-wezboard-scroll-crash.md` — Wezboard scroll crashes Roamium
- `issues/0000732-wezboard-reopen-tab.md` — Shutdown message and tab reopen fix
- `issues/0000733-ghostboard-shutdown.md` — Ghostboard sends Shutdown instead of
  SIGKILL
- `issues/0000734-build-scripts.md` — Consistent build and install scripts
- `issues/0000735-ghostboard-release-icon.md` — Ghostboard app icons
- `issues/0000736-roamium-process-leak.md` — Roamium process leak on GUI crash
- `issues/0000746-overlay-positioning.md` — Fix webview overlay positioning
  (render-pass based)
- `issues/0000747-multiscreen-overlay.md` — Overlay doesn't reposition on split
  (second screen)
- `issues/0000748-clipboard.md` — Browser clipboard (copy/cut/paste)

### Early Prototypes (ts1–ts5)

Issue docs for all prototype generations are indexed in
[docs/early-prototypes.md](docs/early-prototypes.md).

### General

- `issues/0000002-merge-upstream.md` — How to merge changes from upstream repos
- `issues/0000001-competitors.md` — Terminal-browser hybrid comparison
- `issues/0000003-website.md` — termsurf.com website
- `TODO.md` — Task checklist and future issues. Only one issue is active at a
  time (the highest-numbered issue doc without a `## Conclusion`). When a new
  problem is identified during work on the active issue, add it to the "Future
  issues" section of TODO.md instead of creating an issue doc.

### Immutability

Issue documents in `issues/` that have a `## Conclusion` are historical records.
They are **immutable** and must NEVER be modified. They capture what happened at
the time — even if details (like directory names or paths) are now outdated.
History stays as it was written.

## Issues and Experiments

Every significant piece of work gets an issue document in `issues/`. Issues
describe the problem, provide background, and propose solutions. Experiments are
the incremental steps that solve the problem.

### One Issue at a Time

Only one issue is active at a time. The active issue is the highest-numbered
issue doc in `issues/` that does not have a `## Conclusion`. All work focuses on
this issue until it is closed.

When a new problem is discovered during work on the active issue, do NOT create
a new issue doc for it. Instead, add it to the "Future issues" section of
`TODO.md` — a checkboxed list of problems waiting to become issues. When the
active issue is closed and we're ready to start the next piece of work, promote
a TODO item to a full issue doc (next sequential number) and remove it from the
TODO.

### Issue Documents

#### Location and naming

All issue documents live in `issues/`. Each has a sequential number and a short
descriptive name:

```
issues/0000514-mouse.md
issues/0000513-ctrl-esc.md
issues/0000512-vsync.md
```

The number is globally sequential across all generations (ts1–ts5). The name is
lowercase, hyphenated, and describes the topic — not the solution.

#### Structure of a new issue

A new issue document has these sections:

1. **Title** (H1) — `# Issue {N}: {descriptive title}`
2. **Goal** — One or two sentences describing the desired outcome from the
   user's perspective.
3. **Background** — Context: what led to this issue, what prior work is
   relevant, what constraints exist.
4. **Architecture** / **Analysis** / **Proposed Solutions** — Technical details,
   diagrams, trade-offs, ideas for how to solve the problem. Use whatever
   heading name fits the content.

A new issue does **not** have an Experiments section yet. The issue is a problem
statement and analysis, not a solution plan.

#### What NOT to put in a new issue

**Never list experiments upfront.** Do not write "Experiment 1: ..., Experiment
2: ..., Experiment 3: ..." when creating an issue. The outcome of each
experiment may change what comes next. Listing them in advance creates false
commitments and wastes design effort on experiments that may never happen.

Instead, the issue body may include sections like:

- "Ideas for experiments"
- "Proposed solutions"
- "Possible approaches"

These are loose, exploratory. They are not numbered experiments with
verification criteria.

### Experiments

#### When to create an experiment

Only after the issue's product requirements are clear and the team is ready to
implement the next step. Each experiment is designed, implemented, and concluded
before the next one is designed.

#### Adding the Experiments section

When the first experiment is ready to be designed, add an `## Experiments`
heading at the bottom of the issue document, followed by the experiment:

```markdown
## Experiments

### Experiment 1: {short descriptive title}

{design content}
```

#### Experiment structure

Each experiment has:

1. **Title** (H3) — `### Experiment {N}: {descriptive title}`
2. **Description** — What this experiment will do and why. What hypothesis is
   being tested or what capability is being added.
3. **Changes** — The specific code changes required, listed by file.
4. **Verification** — How to test that the experiment worked. Include concrete
   steps and a pass/fail criterion.

#### Chromium branches

If an experiment modifies Chromium code, it MUST create a new branch in the
Chromium repo (`chromium/src/`). Always fork the most relevant recent branch —
usually the branch from the previous issue or experiment that has the code you
need. Never work directly on an existing branch from a different issue.

Each experiment that touches Chromium includes a `### Chromium branch` section
documenting:

1. The new branch name: `{version}-issue-{N}` (e.g., `146.0.7650.0-issue-608`)
2. Which branch it forks from and why (e.g., "from `146.0.7650.0-issue-607`
   because we need the keyboard forwarding code")

Also add the new branch to the table in `chromium/README.md`.

#### One at a time

Design and implement one experiment at a time. After Experiment 1 is concluded,
then — and only then — design Experiment 2. The result of Experiment 1 (success,
partial success, or failure) directly informs what Experiment 2 should be.

#### Recording results

After implementing and testing an experiment, add a result and conclusion
directly below the experiment's verification section:

```markdown
**Result:** Pass / Partial / Fail

{description of what happened}

#### Conclusion

{what we learned, what changed, what to do next}
```

Use the appropriate result:

- **Pass** — The experiment achieved its verification criteria.
- **Partial** — Some goals were met, others were not. Describe what worked and
  what didn't.
- **Fail** — The approach did not work. Describe why and what was learned.

All three outcomes are valuable. Failed experiments eliminate dead ends and
inform better designs.

### Closing an Issue

When all experiments have satisfied the issue's product requirements (the Goal),
add a top-level conclusion:

```markdown
## Conclusion

{summary of what was accomplished, key findings, and any follow-up work}
```

This goes after the last experiment, still inside the issue document.

### Process Summary

1. **Check the TODO** — If starting fresh, pick an item from the "Future issues"
   section of `TODO.md` and promote it to a new issue doc.
2. **Create the issue** — Problem statement, background, analysis. No
   experiments yet.
3. **Design Experiment 1** — Add `## Experiments` and `### Experiment 1` when
   ready.
4. **Implement Experiment 1** — Write the code.
5. **Record the result** — Pass, partial, or fail with a conclusion.
6. **Repeat** — Design the next experiment based on what was learned. Continue
   until the issue's goal is met.
7. **Close the issue** — Write the issue-level conclusion.
8. **New problems discovered along the way** — Add to the "Future issues"
   section of `TODO.md`, not to a new issue doc.

## Remember

NEVER change code unless explicitly asked. NEVER make unrequested changes.
Always do EXACTLY what your user asks — no more, no less.
