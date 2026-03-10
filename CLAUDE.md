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
with the terminal (board) via the TermSurf protocol. One process per profile.

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

### Multiple terminals (boards)

Although TermSurf currently ships as a Ghostty fork (`ghostboard/`), we will
implement forks of all major terminal emulators:

- **Ghostboard** (ghostboard/) — Current board. Active development.
- **WezTerm** (Wezboard) — Researched in Issue 709. Strong architectural match.
- **Kitty** — Planned.
- **Alacritty** — Planned.
- **iTerm2** — Planned.

Any terminal that implements the TermSurf protocol can host browser overlays. A
"board" is a terminal emulator that listens on a Unix socket, accepts
connections from TUIs and browser engines, and renders browser content as
overlays at pixel coordinates.

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
protocol, the board doesn't care which engine is behind it. A user can have one
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
           │    Board    │                1 board (terminal emulator)
           │  (Ghostty)  │
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

N TUIs connect to 1 board. The board manages M browser engine processes, each
serving one profile. Different profiles can use different engines. The board is
the hub — it routes messages between TUIs and engines, manages pane layout, and
composites browser overlays into the terminal window.

### Unix sockets + protobuf for all IPC

All inter-process communication uses Unix domain sockets with length-prefixed
protobuf messages. The board (terminal) listens on a PID-scoped socket
(`$TMPDIR/termsurf/termsurf-ghostboard-{pid}.sock`), and both TUIs and browser
engines connect to it as clients.

- **TUI → Board:** The TUI reads the `TERMSURF_SOCKET` env var (set by the
  board) to discover the socket path.
- **Board → Engine:** The board passes `--ipc-socket={path}` when launching
  browser engine processes.
- **Wire format:** 4-byte little-endian length prefix + serialized protobuf
  (`termsurf.proto`).
- **Serialization:** protobuf-c in Zig (board), prost in Rust (TUI and engines),
  C++ protobuf in Chromium.

Earlier generations (ts3–ts5) used XPC for IPC. Issues 698–701 replaced XPC with
sockets, and Issue 702 removed all dead XPC code. CALayerHost compositing
(zero-copy GPU rendering) does not require XPC — Window Server routes
`CAContext` layer IDs between processes natively.

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

- `ghostboard/` — Ghostboard (Ghostty fork, Zig-first). **Active development.**
- `webtui/` — The `web` TUI (Rust/ratatui). Browser chrome in the terminal pane.
- `chromium/` — Chromium fork build workspace (gitignored).
- `issues/` — Issue documents across all generations (immutable history).
- `website/` — termsurf.com project website.
- `docs/early-prototypes.md` — Archived prototype documentation (ts1–ts5,
  cef-rs).

## Ghostboard (ghostboard/) — Active Development

### Architecture

Ghostboard forks Ghostty with all browser integration in Zig. Swift remains a
thin macOS wrapper — window creation, menu bar, application lifecycle — matching
Ghostty's own architecture. This is a clean break from ts5, where browser
integration lived in Swift (CompositorXPC.swift).

Key architectural decisions:

- **Socket IPC in Zig.** All IPC uses Unix domain sockets with length-prefixed
  protobuf. Ghostboard listens on
  `$TMPDIR/termsurf/termsurf-ghostboard-{pid}.sock`. TUI and Chromium connect as
  clients. The IPC module (`ghostboard/src/apprt/xpc.zig`) handles accept,
  framing, dispatch, and per-connection lifecycle.
- **CALayerHost in Zig.** Browser panes render via `CALayerHost` — a CALayer
  subclass that displays a remote `CAContext` from Chromium's GPU process.
  Window Server composites directly from GPU VRAM. Zero per-frame IPC, zero
  texture copies. The Metal renderer sets up the CALayerHost layer tree in Zig.
- **Input routing in Zig.** Zig already receives all keyboard/mouse events
  through `Surface.keyCallback()` and `mouseButtonCallback()`. In browse mode,
  these route to Chromium via socket IPC instead of to the terminal.
- **Single source of truth.** Browse mode, focus state, pane profiles, overlay
  coordinates — all live in Zig's Surface struct.

### Current State

Ghostboard is a Ghostty fork with browser integration built in Zig. Current
additions: IPC gateway and connection management (Issues 601, 698–702), pink
texture proof-of-concept (Issue 602), live Chromium streaming at 60fps with
dynamic resize (Issue 603), multi-pane multi-profile server reuse (Issues
604–605), mouse input forwarding with cursor changes and text selection (Issue
606), keyboard input forwarding with Cmd+key bypass (Issues 607–609), branding
and app icon (Issues 611–612), directory rename from ghost/web to gui/tui (Issue
613), XDG directory compliance (Issue 615), loading progress indicator and
browser navigation keybindings (Issue 616), CALayerHost migration replacing
FrameSinkVideoCapturer with zero-copy Window Server compositing (Issues
624–632), reproducible rename script for upstream merges (Issue 656), purple
Edit mode border (Issue 657), vim-like editor modes and keybindings (Issue 658),
vim-style command mode (Issue 659), per-mode submode colors (Issue 660), tight
title spacing (Issue 661), clap CLI parser with subcommands (Issue 664),
context-sensitive Esc key navigation (Issue 665), Esc latency fix via unified
mpsc channel (Issue 666), active pane indicator with borders and desaturation
(Issues 667–669), click-to-focus without pass-through (Issue 670), app icon
update (Issue 671), inner border padding (Issue 672), script consolidation
(Issue 673), configurable homepage (Issue 674), hello message for live config
(Issue 675), URL normalization (Issue 676), website deps and linting (Issues
677–678), MIT license and trademark (Issue 679), dark mode with `:colorscheme`
command (Issue 680), `:quitall` and subsequence matching (Issue 681), Chrome
DevTools in split panes (Issues 684, 687, 690–691), multi-profile tracking fix
(Issue 685), tab lifecycle — close tabs when panes close (Issue 689), `web file`
subcommand (Issue 692), smart input resolution (Issue 693), replace pane_id with
tab_id in Chromium (Issue 694), activation drag suppression (Issue 695), double
click suppression fix (Issue 696), Unix socket + protobuf IPC replacing XPC
(Issues 698–702), click suppression removal (Issue 703), browser bindings C
library (Issues 704–706), Roamium Rust browser binary (Issue 707), clean
Chromium fork with renamed libtermsurf_chromium (Issue 708).

### Source Layout

- `ghostboard/src/` — Shared Zig core (libtermsurf)
- `ghostboard/src/Surface.zig` — Core surface (holds browser state)
- `ghostboard/src/renderer/Metal.zig` — Metal renderer
- `ghostboard/src/renderer/metal/` — Metal pipeline, shaders, IOSurface layer
- `ghostboard/src/apprt/embedded.zig` — C API exports
- `ghostboard/include/ghostty.h` — libghostty C API headers
- `ghostboard/macos/` — macOS app (Swift, thin wrapper)
- `ghostboard/build.zig` — Build system
- `ghostboard/build.zig.zon` — Dependencies

### Build & Install

All build scripts live in `scripts/`. They handle Ghostboard, Chromium, TUI, and
Roamium together.

| Script                                                   | Purpose                                                                              |
| -------------------------------------------------------- | ------------------------------------------------------------------------------------ |
| `scripts/build.sh <comp> [--release] [--clean] [--open]` | Build a component. Components: ghostboard, wezboard, roamium, webtui, chromium, all. |
| `scripts/install.sh <comp>`                              | Install a component. Components: ghostboard, wezboard, roamium, webtui, all.         |
| `scripts/uninstall.sh <comp>`                            | Uninstall a component. Components: ghostboard, wezboard, roamium, webtui, all.       |
| `scripts/clean-zig.sh`                                   | Clean Zig build artifacts + Xcode DerivedData. Preserves Chromium cache.             |
| `scripts/rename-ghostty.sh [dir]`                        | Rename all Ghostty references to TermSurf in `ghostboard/`. Re-runnable.             |
| `scripts/rename-wezterm.sh [dir]`                        | Rename all WezTerm references to Wezboard in `wezboard/`. Re-runnable.               |
| `scripts/generate-icons.sh [image]`                      | Generate app icon assets from a source image.                                        |
| `scripts/nerd-font-test.sh`                              | Print Nerd Font test glyphs for visual verification.                                 |

For Ghostboard-only iteration, `cd ghostboard && zig build` still works. The
full build scripts also auto-detect Chromium's `protoc` so you don't need a
system install.

### Upstream Merges

Same approach as ts5 (Issue 418 Experiment 3):

```bash
git fetch upstream
git subtree pull --prefix=ghostboard upstream main -m "Merge upstream Ghostty into ghostboard"
```

## Documentation

All documentation is in `docs/` or in `README.md` files throughout the codebase.

### Ghostboard (active)

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
