# Issue 417: Ghostty vs WezTerm

**Goal:** Choose between Ghostty and WezTerm as the terminal emulator for
TermSurf 4.0.

**Context:** TermSurf is a cross-platform terminal browser — an advanced
terminal emulator multiplexer that also has a `web` command for opening a
Chromium-based browser inside terminal panes. Users can run multiple browser
profiles side by side in the same window. The browser runs out-of-process and
communicates via IPC (XPC on macOS), sending IOSurface frames to the terminal
for compositing.

We must pick exactly one terminal to fork. The browser side is decoupled via IPC
and is not locked to any specific terminal. But the terminal fork is deep — we
need custom pane types, rendering pipeline integration, and long-term
maintainability.

**Prior experience:**

- ts1: Ghostty fork + WKWebView (macOS only, abandoned due to WKWebView limits)
- ts2: WezTerm fork + in-process CEF (abandoned due to one-profile-per-process)
- ts3: WezTerm fork + out-of-process CEF via XPC (superseded by ts4 due to CEF
  fps cap)
- ts4: Active development. Terminal TBD + in-process Chromium via Content API.

## Research Factors

### 1. Dealbreakers

#### 1.1 Windows Support

- Ghostty: macOS + Linux only. No Windows support, no public roadmap for it.
- WezTerm: macOS + Linux + Windows + FreeBSD. Already shipping.
- Question: How important is Windows to TermSurf's target audience (web
  developers)?

#### 1.2 Custom Pane Feasibility

- Can either terminal support a non-terminal pane natively — a pane that renders
  an IOSurface/texture instead of terminal cells?
- Is it a matter of implementing an interface/trait, or does it require
  modifying rendering internals?
- How deep does the integration go?

### 2. Integration Depth

#### 2.1 Language Alignment

- Ghostty: Zig + Swift (macOS shell). All TermSurf browser integration code
  (termsurf-xpc, two-profiles-rust, termsurf-launcher, termsurf-profile) is
  Rust.
- WezTerm: Rust. Same language as all browser integration code.
- What does calling libghostty from Rust look like? How thick is the FFI layer?
- With WezTerm, can you implement a custom `Pane` trait directly in Rust?

#### 2.2 Rendering Pipeline Compatibility

- WezTerm uses wgpu (cross-platform). The Rust IOSurface receiver already uses
  wgpu. Same abstraction layer.
- Ghostty uses Metal on macOS, OpenGL on Linux. IOSurface compositing is
  Metal-native on macOS, but needs a different path per platform.
- How does each terminal composite panes? Single render pass with viewports?
  Separate textures per pane? Where do you inject the IOSurface texture?

#### 2.3 Upstream Merge Difficulty

- We are maintaining a fork. How often does each project make breaking changes
  to pane/rendering systems?
- Code churn rate in the areas we'd modify (pane management, rendering, window
  management).
- How modular is the code? Can our changes live in isolated files, or do we have
  to modify core files that upstream frequently touches?

#### 2.4 Prior Integration Experience

- ts1 (Ghostty): What was easy? What was hard? How deep were the modifications?
- ts2/ts3 (WezTerm): What was easy? What was hard? How deep were the
  modifications?
- Which codebase was easier to understand and navigate?

### 3. Practical Factors

#### 3.1 Codebase Size & Complexity

- Lines of code (excluding vendored/generated code)
- Number of files
- Dependency count
- Build time

#### 3.2 Community & Governance

- Number of contributors
- Commit count and frequency
- Bus factor (both are single-maintainer projects)
- Responsiveness to external contributions
- Would either maintainer accept upstream PRs that make forking easier?

#### 3.3 Build Complexity

- Ghostty: Zig build system
- WezTerm: Cargo + native dependencies
- CI/CD setup difficulty for all target platforms

#### 3.4 Performance Baseline

- Memory usage
- Startup time
- Rendering latency
- Known performance issues that would compound with browser pane rendering

#### 3.5 Release Cadence

- How often does each release?
- Stable vs rolling releases
- How painful are version upgrades for a fork?

### 4. Nice-to-Haves

#### 4.1 Configuration & Extensibility

- WezTerm has Lua scripting. Potential for user plugins.
- Ghostty has a config file but no scripting runtime. Simpler but less
  extensible.

#### 4.2 Image/Graphics Protocol Support

- Sixel, iTerm2, Kitty graphics protocol support
- Relevant as a potential fallback rendering path (browser frames as images)

#### 4.3 Multiplexer Features

- WezTerm has a built-in multiplexer with remote multiplexing (SSH)
- Ghostty's multiplexer capabilities
- Tab/split/pane management API surface

#### 4.4 License

- Ghostty: MIT
- WezTerm: MIT
- CLA requirements for contributing back

## Experiments

(To be designed after research.)

## Conclusion

(To be written after experiments.)
