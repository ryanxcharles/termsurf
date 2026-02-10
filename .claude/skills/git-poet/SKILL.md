---
name: git-poet
description: "Write entertaining commit messages as poetry"
---

# GitPoet

Write commit messages that accurately describe changes while delighting readers with poetic wit.

## Philosophy

Every commit tells a story. GitPoet transforms mundane diffs into memorable verses. The goal is to make people smile when they read the commit log on termsurf.com.

## Format

Each commit message should have two parts:

1. **First line**: A short, accurate summary (50 chars max) - this is the "title"
2. **Body**: A short poem (2-8 lines) that humorously describes the change

## Style Guidelines

- **Accuracy first**: The poem must accurately describe what changed
- **Humor over formality**: Prefer wit, wordplay, puns, and absurdity
- **Keep it short**: Poems should be 2-8 lines, not epics
- **Vary the form**: Mix haikus, limericks, couplets, free verse, etc.
- **Stay tasteful**: Funny but professional enough for public viewing

## Examples

### Haiku style
```
Fix null pointer crash

A pointer walked alone,
Into the void it did fall—
Now it checks its path.
```

### Limerick style
```
Add dark mode toggle

A user who coded at night,
Found the screen far too bright.
So we added a switch,
Now it's dark, what a pitch!
Their retinas now feel just right.
```

### Couplet style
```
Refactor auth module

The auth code was a tangled mess,
Now it's clean—we must confess.
```

### Free verse style
```
Update dependencies

The packages grew old and weary,
Their CVEs made security teary.
We bumped the versions, one by one,
Now npm audit says: "Well done!"
```

## Process

1. **Run git diff --staged** to see what's being committed
2. **Understand the change**: What problem does it solve? What was added/removed/fixed?
3. **Write the title**: Accurate, imperative mood, 50 chars max
4. **Compose the poem**: Pick a style that fits the change, make it fun
5. **Stage and commit** using the poetic message

## Submodule Workflow (Chromium Fork)

The Chromium fork at `ts4/termsurf-chromium/src/` doesn't use `main`. Branches
are named `{version}-termsurf` (e.g., `146.0.7650.0-termsurf`), built as a
series of commits on top of the vanilla Chromium version tag. The branch name
encodes the upstream version.

**Git-poet applies to all TermSurf commits — both inside the submodule and in
the main repo.** Our commits on the `{version}-termsurf` branch get poetic
messages. The main repo commit that pins the submodule also gets one.

**The only exception is `git am` patches.** Patches applied with `git am` (e.g.,
from Electron) keep their original commit messages. We don't rewrite upstream
authorship.

**Typical flow:**

1. Work inside the submodule on the `{version}-termsurf` branch
2. Commit with git-poet
3. Return to the main repo
4. Stage the submodule pointer (`git add ts4/termsurf-chromium/src`) and any
   related files (docs, etc.)
5. Commit with git-poet — this records what TermSurf did at the project level

## When NOT to use GitPoet

- Merge commits (use standard merge messages)
- Reverts (use standard revert messages)
- Version bumps (keep these straightforward)
- Security fixes (be clear, not clever)
- `git am` patches (keep upstream commit messages intact)
