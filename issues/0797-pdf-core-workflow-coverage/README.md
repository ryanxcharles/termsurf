+++
status = "closed"
opened = "2026-05-30"
closed = "2026-06-22"
+++

# Issue 797: PDF Core Workflow Coverage

## Goal

Prove and, where necessary, implement the remaining common non-print PDF viewer
workflows that Issue 796 identified as unverified.

## Background

Issue 796 audited the PDF implementation after the code organization and
security cleanup tracks. The core PDF viewer path works: full-page rendering,
embedded rendering, scrolling, resizing, selection/copy, ordinary save/download,
title propagation, and the PDF extension security boundary are all covered by
automation.

The completeness audit found that several common workflows were still unproven
or weakly covered. They are important enough to track explicitly, but they do
not block closing the audit issue because they require new fixtures and focused
probes rather than broad PDF architecture changes.

## Scope

This issue covers these non-print PDF workflows:

- keyboard scroll and page navigation;
- current-branch toolbar event coverage for zoom in/out, fit, rotate, and page
  selector navigation;
- internal PDF links;
- external links from PDFs;
- find/search within PDFs;
- copy-restricted PDFs;
- save/download-restricted PDFs;
- disabled toolbar states for document restrictions;
- password-protected PDFs;
- malformed or error-page PDFs.

Native PDF printing is out of scope and remains tracked by Issue 795.

Advanced surfaces such as forms, annotations, context menus, and
accessibility/searchify are tracked separately by Issue 798.

## Analysis

Most of these behaviors are likely provided by Chromium's upstream PDF viewer
once TermSurf's embedder plumbing is active. The missing work is evidence and
small integration fixes if a probe proves a TermSurf-specific gap.

The first experiment should add deterministic fixtures and probes before making
product changes. If a probe fails, design the next experiment around the exact
failed layer.

## Conclusion

This issue is superseded by
[Issue 834: Full PDF Support Across Roamium and Surfari](../0834-full-pdf-support-roamium-surfari/README.md).

The core workflow coverage listed here remains required, but it should now be
tracked in the unified cross-engine PDF matrix so Roamium and Surfari can be
completed and regression-tested together.
