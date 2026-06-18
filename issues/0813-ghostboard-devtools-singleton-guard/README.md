+++
status = "open"
opened = "2026-06-17"
+++

# Issue 813: Ghostboard One-DevTools-Per-Tab Guard

## Goal

Prevent duplicate DevTools frontends for the same inspected tab in Ghostboard
and verify close/reopen behavior.

## Background

Issue 810 classified this as a `Highly likely` Ghostboard gap. Historical Issues
686 and 687 showed that duplicate DevTools sessions for one inspected page can
recreate a Chromium crash class. Current Ghostboard evidence validates that the
inspected tab exists, but did not show a guard that rejects a second DevTools
frontend for the same tab.

## Analysis

The fix should restore the one-DevTools-per-tab invariant in the current
socket/protobuf Ghostboard architecture. It should reject duplicate launches
before opening a second DevTools pane, return a clear error to webtui, and allow
DevTools to reopen after the original DevTools pane closes.

Verification should include:

- first DevTools open succeeds;
- second DevTools open for the same inspected tab is rejected;
- DevTools close cleans up the guard state;
- reopening DevTools after close succeeds;
- multi-profile or multi-tab DevTools cases do not block unrelated inspected
  tabs.

## Experiments

- [Experiment 1: Guard duplicate DevTools requests](01-guard-duplicate-devtools-requests.md)
  — **Designed**
