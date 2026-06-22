+++
status = "closed"
opened = "2026-05-30"
closed = "2026-06-22"
+++

# Issue 798: PDF Advanced Features

## Goal

Audit and implement advanced non-print PDF viewer features that are outside the
core workflow coverage tracked by Issue 797.

## Background

Issue 796 found that TermSurf's PDF viewer is usable for core non-print viewing,
but advanced browser PDF features are not yet proven. These surfaces are larger
than the audit cleanup budget because they may involve Chromium PDF UI,
PDFium/plugin behavior, browser commands, or accessibility infrastructure.

## Scope

This issue covers:

- PDF forms beyond basic display;
- PDF annotations, including Ink/text annotation UI where available;
- PDF context menu behavior;
- PDF accessibility/searchify behavior.

Native PDF printing is out of scope and remains tracked by Issue 795.

Core workflow probes for keyboard navigation, links, search, restrictions,
passwords, and error pages are tracked by Issue 797.

## Analysis

Chromium contains upstream code for forms, annotations, context-sensitive
commands, accessibility, and searchification. TermSurf should first determine
which pieces already work through the current PDF viewer plumbing, then add only
the missing embedder integration needed for the in-pane terminal browser model.

The first experiment should be diagnostic: inventory upstream support, identify
the TermSurf integration points, and choose the smallest feature slice to prove
end to end.

## Conclusion

This issue is superseded by
[Issue 834: Full PDF Support Across Roamium and Surfari](../0834-full-pdf-support-roamium-surfari/README.md).

The advanced PDF surfaces listed here remain part of the PDF roadmap, but they
should now be audited, implemented, and regression-tested through the unified
cross-engine PDF matrix.
