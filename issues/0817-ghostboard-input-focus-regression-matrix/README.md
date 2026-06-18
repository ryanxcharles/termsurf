+++
status = "closed"
opened = "2026-06-17"
closed = "2026-06-18"
+++

# Issue 817: Ghostboard Input and Focus Regression Matrix

## Goal

Design and run a focused Ghostboard input/focus regression matrix, then fix any
confirmed Ghostboard-owned gaps.

## Background

Issue 810 grouped input and focus as a `Maybe` finding. Current code has
keyboard, mouse, scroll, mode, and focus paths, and Issue 809 covered important
geometry/input cases, but the historical archive shows that browser input needs
more complete coverage.

The matrix should cover:

- keyboard text input and special keys;
- Cmd/menu shortcuts;
- clipboard behavior;
- mode transitions;
- focus stealing and pane focus;
- dimming or inactive visual feedback;
- caret visibility;
- mouse click, hover, scroll, double-click, triple-click, modifier-click;
- drag selection and terminal-selection suppression;
- mouse hot-path performance.

## Analysis

The first experiments should establish reliable automation and pass/fail
criteria. The matrix should stay small enough to run repeatedly, with slower
manual or screenshot-heavy cases separated from fast smoke tests.

## Experiments

- [Experiment 1: Establish input/focus baseline matrix](01-establish-input-focus-baseline.md)
  — **Partial**
- [Experiment 2: Prove browser input granularity](02-prove-browser-input-granularity.md)
  — **Partial**
- [Experiment 3: Fix browser drag forwarding](03-fix-browser-drag-forwarding.md)
  — **Pass**

## Conclusion

Issue 817 produced a focused Ghostboard input/focus regression matrix and fixed
the only confirmed Ghostboard-owned gap found by that matrix: browser drag
selection failed because AppKit drag-generated TermSurf mouse moves did not
preserve the active left-button modifier.

The durable automated coverage now includes:

- baseline mouse geometry/click routing after geometry changes;
- keyboard routing across tab and window focus changes;
- GUI active/inactive focus delivery;
- Control-mode and Browse-mode `Cmd+C` clipboard behavior;
- browser-observed text input, special-key editing, logical caret/focus state,
  enter activation, click counts, modifier-click, drag selection, browser
  selection copy, Control-mode focus return, and ordered overlay drag
  consumption.

Mouse hot-path performance remains intentionally deferred to Issue 820. Any
future product requirement for screenshot-level visual dimming or visible caret
assertions should be added as a new targeted issue rather than reopening this
closed record.
