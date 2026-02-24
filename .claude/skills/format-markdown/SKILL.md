---
name: format-markdown
description:
  "Format markdown files with prettier. Use after creating or editing any
  markdown file (.md)."
---

# Format Markdown

Run `prettier` on every markdown file after creating or editing it.

## When This Skill Applies

After every Write or Edit to any `.md` file in the project. This includes:

- Issue documents (`docs/issues/*.md`)
- Documentation (`docs/*.md`)
- `README.md` files
- `CLAUDE.md` files
- Any other markdown file

## Process

After your edits to a markdown file are complete, run:

```bash
prettier --write --prose-wrap always --print-width 80 <file_path>
```

That's it. No other steps needed.
