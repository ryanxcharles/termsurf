# Issue 714: Seven-Digit Issue Numbers

## Goal

Rename every issue file to use zero-padded 7-digit numbers (e.g.,
`001-competitors.md` → `0000001-competitors.md`) so that issue numbers never
need to be renamed again.

## Background

Issue files currently use variable-width numbers: 1–3 digits (`001`, `100`,
`714`). As the project grows past 999 issues, the numbering scheme would need to
change — breaking the immutability guarantee that issue documents should never
be modified once concluded.

The question is: how many digits are enough to never need renaming again?
Chromium, arguably the largest open-source project in the world, has ~1.7
million commits. If every commit were an issue, 7 digits (up to 9,999,999) would
still have room to spare. Seven digits is the pragmatic ceiling — virtually no
project will ever exceed it.

By renaming all 224 issue files now (before the archive grows further), we
establish a permanent numbering scheme. Once renamed, issue filenames are truly
immutable — no future growth will force another rename.

## Scope

- Rename every file in `issues/` from `{N}-{name}.md` to `{0000N}-{name}.md`
  (7-digit zero-padded).
- Update all internal cross-references: issue documents that link to other issue
  files (e.g., `[Issue 411](../issues/411-two-profiles-3.md)` →
  `[Issue 411](../issues/0000411-two-profiles-3.md)`).
- Update `chromium/README.md` branch table links (e.g.,
  `../issues/411-two-profiles-3.md` → `../issues/0000411-two-profiles-3.md`).
- Update `docs/early-prototypes.md` issue references.
- Update `CLAUDE.md` issue references.
- Update any other files that reference issue filenames by path.

## Analysis

### What changes

| Before                             | After                                  |
| ---------------------------------- | -------------------------------------- |
| `issues/001-competitors.md`        | `issues/0000001-competitors.md`        |
| `issues/100-bookmarks.md`          | `issues/0000100-bookmarks.md`          |
| `issues/714-seven-digit-issues.md` | `issues/0000714-seven-digit-issues.md` |

### What stays the same

- Issue numbers themselves (714 is still 714)
- Issue document content (immutable once concluded)
- The `issues/` directory location

### Reference locations to update

1. **`chromium/README.md`** — Branch table links (`../issues/{N}-...`)
2. **`docs/early-prototypes.md`** — Issue index links
3. **`CLAUDE.md`** — Issue references in documentation section
4. **Issue documents themselves** — Cross-references between issues
5. **Skill files** — Any hardcoded issue paths
