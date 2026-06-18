+++
status = "closed"
opened = "2026-06-17"
closed = "2026-06-17"
+++

# Issue 815: Ghostboard HelloReply Configuration

## Goal

Populate and verify Ghostboard `HelloReply` data needed by webtui, especially
homepage and browser-list configuration.

## Background

Issue 810 classified this as a `Highly likely` gap. Ghostboard replies to
`HelloRequest`, but the audit found likely missing homepage and browser-list
configuration needed by `web`.

## Analysis

The work should compare Wezboard's `HelloReply` behavior, current webtui
expectations, and Ghostboard's config sources. The implementation should avoid
inventing parallel config semantics if Ghostty/Ghostboard already has a suitable
config path.

Verification should include:

- `HelloReply` includes the expected default browser list;
- `HelloReply` includes the configured homepage or documented default;
- webtui consumes the reply and displays/uses the values correctly;
- missing or invalid config falls back deterministically.

## Experiments

- [Experiment 1: Send deterministic HelloReply defaults](01-send-deterministic-hello-reply-defaults.md)
  — **Pass**
- [Experiment 2: Flow configured homepage into HelloReply](02-flow-configured-homepage-into-hello-reply.md)
  — **Pass**
- [Experiment 3: Flow configured browser list into HelloReply](03-flow-configured-browser-list-into-hello-reply.md)
  — **Pass**

## Conclusion

Issue 815 is closed. Ghostboard now sends complete `HelloReply` data needed by
webtui:

- deterministic defaults for `homepage=https://termsurf.com/welcome` and
  `browsers=["roamium"]`;
- configured homepage from the normal Ghostboard config file;
- configured repeatable browser list from `browser = ...` entries;
- fallback to `roamium` when the browser list is missing or reset to empty.

The runtime harness proves webtui consumes the GUI-provided homepage when no URL
is supplied, consumes the first GUI-provided browser when `--browser` is
omitted, and still launches named Roamium through the debug resolver.
