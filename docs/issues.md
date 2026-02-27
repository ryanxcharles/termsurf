# Future Issues

Issues that need their own issue doc when we're ready to work on them. Each
entry is a problem we've identified but haven't started solving yet.

## Renderer crash UX

When the Chromium renderer process dies, the user sees a blank white screen with
no indication of what happened. The progress bar continues as if the page is
still loading, then times out. We need to:

- Detect renderer termination and display an error page (like Chrome's "Aw,
  Snap!" page).
- Clear the progress bar immediately when the renderer dies.
- Show what went wrong so the user or developer can diagnose the issue.

Discovered in Issue 655 Experiment 1.

## Mojo interface audit

Our Content API build is missing handlers for Mojo interfaces that a full Chrome
browser registers. Every missing binder is a ticking time bomb — the renderer
crashes the moment any page's JavaScript calls that API. We fixed
`blink.mojom.BadgeService` in Issue 655, but there are likely many more.

We need to systematically review all Mojo interfaces that Chrome registers and
ensure our build either handles them or registers a stub/no-op so the renderer
isn't killed.

Discovered in Issue 655 Experiment 1.
