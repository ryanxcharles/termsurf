# TermSurf 3.0: Electron FrameSinkVideoCapturer Study

## Problem

TermSurf's browser renders at ~20fps instead of 60fps. Scrolling, mouse
interactions, and animations are visibly laggy compared to native Chrome. Users
expect browser-quality smoothness; we're delivering slideshow-quality
choppiness.

The goal: achieve 60fps rendering with no perceptible lag.

## What We Learned from Issue 338

Issue 338 documented five experiments attempting to fix the lag through CEF
configuration. All failed. The key findings:

### CEF's Architecture is the Problem

The throttling occurs in `CefCopyFrameGenerator::GenerateCopyFrame()`:

```cpp
if (frame_in_progress_)
    return;  // Discards >66% of frames
```

This is baked into CEF's code. No setting, flag, or API call can bypass it:

| Experiment | Approach                       | Result                                      |
| ---------- | ------------------------------ | ------------------------------------------- |
| 1          | Cache IOSurface texture        | Disproven — import is fast (0.37ms)         |
| 2          | Investigate frame pacing       | Found CEF's 30fps cap                       |
| 3          | `multi_threaded_message_loop`  | Failed — incompatible with OSR on macOS     |
| 4          | `external_begin_frame_enabled` | Partial — reduced stutters, still ~20fps    |
| 5          | Chrome command-line flags      | Marginal — flags don't affect OSR code path |

### The Proven Solution Exists

Electron achieves **240fps** GPU-accelerated off-screen rendering. They hit the
same CEF limitations we did, then bypassed them entirely by using a different
Chromium API: `FrameSinkVideoCapturer`.

|                  | CEF (current)            | Electron                 |
| ---------------- | ------------------------ | ------------------------ |
| Capture API      | `OnAcceleratedPaint`     | `FrameSinkVideoCapturer` |
| Frame throttling | Hard-coded 30fps cap     | None                     |
| Texture sharing  | `shared_texture_enabled` | `kGpuMemoryBuffer`       |
| Measured fps     | ~20                      | 240+                     |

This isn't a hack or workaround — it's the architecturally correct solution that
ships in production today.

## Why Mimic Electron

### It's Proven at Scale

Electron powers VS Code, Slack, Discord, Figma, and thousands of other apps.
Their GPU-accelerated OSR implementation handles billions of renders daily. When
they say it works at 240fps, it's battle-tested.

### Same Constraints, Same Solution

Electron faced identical constraints:

- Need GPU-accelerated rendering (can't use software fallback)
- Need shared textures for compositor integration
- Need high frame rates for smooth UX
- CEF's APIs weren't sufficient

They solved it. We should learn from their solution rather than reinvent it.

### Clear Implementation Path

[Electron PR #42953](https://github.com/electron/electron/pull/42953) provides a
complete reference implementation. The code is open source. We can study exactly
how they:

- Set up `FrameSinkVideoCapturer`
- Configure `kGpuMemoryBuffer` for zero-copy texture sharing
- Integrate with their compositor
- Handle frame pacing

### Lower Risk Than Alternatives

| Alternative           | Risk                                        |
| --------------------- | ------------------------------------------- |
| Fork CEF              | Maintenance burden of entire CEF codebase   |
| Use OBS fork          | May not have macOS support, different goals |
| Use Chromium directly | Enormous complexity, no abstraction layer   |
| **Mimic Electron**    | **Well-documented, proven, focused scope**  |

## Architecture Overview

### Current: CEF OnAcceleratedPaint

```
CEF Browser
    │
    ▼
CefCopyFrameGenerator
    │ (throttled to ~30fps)
    ▼
OnAcceleratedPaint callback
    │
    ▼
IOSurface → Mach port → GUI
```

### Target: FrameSinkVideoCapturer

```
Chromium Browser
    │
    ▼
viz::FrameSinkVideoCapturer
    │ (no throttle)
    ▼
GPU Memory Buffer callback
    │
    ▼
IOSurface → Mach port → GUI
```

The key difference: `FrameSinkVideoCapturer` operates at the compositor level,
capturing frames directly from the GPU without the frame-dropping logic in CEF's
`CefCopyFrameGenerator`.

## Key Resources

### Electron Implementation

- [PR #42953: GPU OSR with FrameSinkVideoCapturer](https://github.com/electron/electron/pull/42953)
- [Electron OSR documentation](https://www.electronjs.org/docs/latest/tutorial/offscreen-rendering)

### Chromium APIs

- `viz::FrameSinkVideoCapturer` — Frame capture from compositor
- `media::VideoFrame` — Frame container with GPU memory buffer
- `gpu::GpuMemoryBufferHandle` — Cross-process texture handle

### CEF Reference (what we're replacing)

- `CefRenderHandler::OnAcceleratedPaint` — Current callback
- `CefCopyFrameGenerator::GenerateCopyFrame` — The throttling code
- [CEF Issue #1368: OSR frame rate limit](https://bitbucket.org/chromiumembedded/cef/issues/1368)

### Previous Work

- [Issue 338: Browser lag investigation](./338-lag.md) — Full context on why CEF
  doesn't work

## Success Criteria

| Metric                 | Current | Target        |
| ---------------------- | ------- | ------------- |
| Frame rate             | ~20fps  | 60fps         |
| Frame interval         | 50ms    | 16ms          |
| Stutter frames (>50ms) | 14%     | <1%           |
| Scrolling feel         | Laggy   | Chrome-smooth |
| Input latency          | ~100ms  | <32ms         |

## Timeline

This is a significant undertaking. Rough phases:

1. **Research**: Understand Electron's approach, choose integration strategy
2. **Prototype**: Prove feasibility with standalone demo
3. **Integrate**: Bring into TermSurf
4. **Polish**: Production-ready quality

Each phase should be completed before starting the next. Findings from earlier
phases may change the approach for later ones.

## Ideas for Future Experiments

These ideas may become experiments after we complete the initial research. What
we learn from studying Electron's implementation will inform which approaches
are viable.

- **Assess integration options** — Evaluate paths: patch CEF, use libcef +
  Chromium APIs, embed Electron, or custom Chromium embedding
- **Prototype FrameSinkVideoCapturer** — Standalone proof-of-concept that
  captures frames and renders via wgpu
- **Integrate into TermSurf** — Replace CEF's `OnAcceleratedPaint` with the new
  capture API in termsurf-profile
- **Optimize and polish** — Frame pacing, vsync alignment, edge cases, production
  quality

## Experiments

### Experiment 1: Study Electron's Implementation

**Status:** NOT STARTED

**Goal:** Understand exactly how Electron implements `FrameSinkVideoCapturer`
and what Chromium APIs it uses.

**Tasks:**

1. Read [Electron PR #42953](https://github.com/electron/electron/pull/42953)
   - What files were changed?
   - What Chromium headers are included?
   - How is the capturer configured?

2. Study the key classes:
   - `viz::FrameSinkVideoCapturer` — What is it? How is it created?
   - `media::VideoFrame` — How are frames delivered?
   - `kGpuMemoryBuffer` — How does zero-copy work on macOS?

3. Trace the data flow:
   - Browser renders to compositor
   - Compositor → FrameSinkVideoCapturer
   - Capturer → VideoFrame callback
   - VideoFrame → texture extraction
   - Texture → application

4. Document dependencies:
   - What Chromium components are required?
   - Are there macOS-specific APIs?
   - What's the minimum Chromium version?

**Deliverable:** Architecture document explaining Electron's approach in detail.
