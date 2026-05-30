+++
status = "open"
opened = "2026-05-30"
+++

# Issue 796: PDF Implementation Audit

## Goal

Audit and harden the PDF implementation created across the recent PDF issues.
The work has three ordered audit tracks: code organization, security, and
feature completeness. Each track must be audited first, then cleaned up in a
separate follow-up experiment before the next track begins.

## Background

Issues 792 through 794 brought TermSurf's Chromium PDF viewer from non-loading
to a usable in-pane PDF viewer. That work intentionally moved quickly through a
large amount of Chromium embedder plumbing:

- component-extension registration and PDF viewer resources;
- MimeHandlerView and stream plumbing;
- PDF extension APIs and resource/template replacements;
- full-page and embedded PDF behavior;
- local-file and extensionless PDF parity;
- PDF input routing for scroll, mouse, keyboard, resize, selection, toolbar
  controls, save/download, and title propagation;
- print-path containment and a renderer print-helper installation that prevents
  the print button from breaking the viewer, while native print remains deferred
  to Issue 795.

The implementation now works well enough to justify a deliberate audit pass.
This issue is not for adding unrelated PDF features or revisiting native print.
It is for making the existing PDF implementation easier to maintain, safer to
ship, and complete enough for the non-print PDF viewer scope established by
Issue 794.

## Scope

Focus only on the PDF implementation added or modified in the recent PDF issues.
Relevant areas include, but are not limited to:

- `chromium/src/content/libtermsurf_chromium/` PDF, extension, MimeHandler,
  stream, resource, title, toolbar, and print-containment code;
- Chromium PDF patches under `chromium/patches/issue-792/`,
  `chromium/patches/issue-793/`, and `chromium/patches/issue-794-*`;
- Roamium dispatch/input code touched for PDF behavior;
- Wezboard PDF-related input/resize routing touched for PDF behavior;
- PDF automation scripts under `scripts/`;
- issue records for Issues 792, 793, and 794 where needed for context.

Do not audit unrelated browser features, general popup work, split-pane
rendering, DevTools, or native PDF printing except where those surfaces directly
touch PDF viewer safety or maintainability.

## Audit Tracks

### Track 1: Code Organization and Readability

The first audit asks whether the PDF implementation is understandable and
maintainable.

Questions:

- Are PDF responsibilities split cleanly between browser client, renderer
  client, extension support, stream management, resource serving, input routing,
  and test harnesses?
- Are there helpers, filenames, comments, or call paths that are now misleading
  because they were introduced during experiments?
- Are there duplicated trace helpers, env-var parsers, resource lookup patterns,
  or PDF-specific shims that should be consolidated without changing behavior?
- Are there temporary experiment names, stale logs, or confusing comments that
  should be renamed or removed?
- Can the code be made easier to read without changing runtime behavior?

The cleanup experiment for this track must be behavior-preserving. It may
rename, move, deduplicate, comment, or split helpers, but it must not change PDF
viewer behavior.

### Track 2: Security

The second audit asks whether opening PDFs from the internet is safe in the
TermSurf embedder context.

Questions:

- Are URL, origin, extension, file, stream, and MIME checks as narrow as they
  should be?
- Are there places where TermSurf grants the PDF extension broader access than
  Chrome or Electron would?
- Are `file://` and extensionless local PDF handling scoped correctly?
- Are stream IDs, frame tree node IDs, tab IDs, render frame IDs, URLs, and
  origins validated before use?
- Can an untrusted PDF or web page use the TermSurf PDF APIs, resource loader,
  stream manager, or extension bindings outside the intended PDF viewer path?
- Are there unsafe assumptions in C++ code, including unchecked nulls,
  stale-frame use, lifetime hazards, path handling, integer/size conversions, or
  unbounded reads/writes?
- Do automation-only env vars or intercept paths fail closed and stay out of
  production behavior?

The cleanup experiment for this track must fix real security issues identified
by the audit. If the audit finds no exploitable issues, the cleanup experiment
should record that conclusion and may still tighten comments, assertions, or
tests that make the security boundary easier to verify.

### Track 3: Completeness

The third audit asks whether the non-print PDF viewer is complete enough after
Issue 794.

Questions:

- Are there missing non-print features expected from Chromium/Electron's PDF
  viewer?
- Do full-page, embedded, HTTP, `file://`, extensionless, titled, untitled, and
  restricted PDFs behave correctly?
- Do scroll, resize, mouse, keyboard, selection, copy, toolbar navigation, zoom,
  fit, rotate, save/download, title propagation, and normal web regressions have
  enough coverage?
- Are search, links, context menus, forms, accessibility/searchify, annotations,
  permissions, download restrictions, and error pages either implemented,
  explicitly out of scope, or captured in follow-up issues?
- Is any required behavior still only manually tested when it can be automated?

Native PDF printing is explicitly out of scope for this completeness track
because Issue 795 owns it. The completeness audit may mention Issue 795 as an
open follow-up, but it must not try to solve print here.

The cleanup experiment for this track must implement or document the missing
non-print pieces identified by the audit. If a missing feature is too large for
this issue, open a separate follow-up issue and clearly explain why it is not a
blocker for the audited PDF viewer scope.

## Required Experiment Sequence

This issue must proceed one experiment at a time. Do not run the next audit
until the prior audit and cleanup have both completed.

Required sequence:

1. Design and run the code organization audit.
2. Design and run the code organization cleanup.
3. Design and run the security audit.
4. Design and run the security cleanup.
5. Design and run the completeness audit.
6. Design and run the completeness cleanup.

Do not list experiment files in this README until each experiment is actually
designed. When an experiment is designed, create its own numbered file and add
it to the `## Experiments` index.

## Experiments

- [Experiment 1: Code organization audit](01-code-organization-audit.md) —
  **Pass**
- [Experiment 2: Code organization cleanup](02-code-organization-cleanup.md) —
  **Designed**

## Experiment Rules

Every experiment in this issue must follow these process rules:

- The experiment design must be reviewed by Codex before implementation.
- Real issues found by Codex during design review must be fixed before
  proceeding.
- After the design is accepted, commit the experiment design before
  implementation.
- The completed experiment result must be reviewed by Codex.
- Real issues found by Codex during completion review must be fixed before the
  experiment is marked complete.
- After the completion review is accepted, commit the completed experiment
  result and any cleanup changes.
- If Rust code is edited, run `cargo fmt` and accept its output.
- If Markdown is edited, run Prettier.
- If Chromium code is edited, use a fresh Chromium branch for that experiment
  and add it to `chromium/README.md`.
- Build and test scope must match the risk of the change. Behavior-preserving
  cleanup still needs enough verification to prove it stayed
  behavior-preserving.

## Constraints

- Do not reopen closed Issues 792, 793, or 794.
- Do not implement native PDF printing in this issue; Issue 795 owns that work.
- Do not change protocol surface unless an audit proves the current protocol is
  unsafe or insufficient for a non-print PDF requirement.
- Do not broaden PDF extension, file, or origin access without a specific
  security justification and test coverage.
- Do not delete diagnostics that are still useful for proving PDF behavior
  unless the cleanup experiment replaces them with clearer diagnostics or
  documents why they are obsolete.
- Do not combine audit and cleanup in one experiment. The audit result must
  drive the cleanup design.
