---
name: fix-nerd-fonts
description: "Verify and fix Nerd Font icons after editing files. Use whenever writing or editing a file that contains Nerd Font characters (Private Use Area codepoints)."
---

# Fix Nerd Fonts

Nerd Font icons use Unicode Private Use Area codepoints (U+E000-U+F8FF and
U+F0000-U+10FFFF). The Write and Edit tools may silently strip or corrupt these
characters. This skill ensures they survive every edit.

## When This Skill Applies

**After every Write or Edit to a file that contains Nerd Font icons.** Currently
the known files are:

| File | Icons |
|------|-------|
| `tui/src/main.rs` | nf-md-web (U+F059F), nf-fa-keyboard_o (U+F11C), nf-fa-user (U+F007) |
| `docs/issues/504-web-tui.md` | nf-md-web (U+F059F), nf-fa-keyboard_o (U+F11C), nf-fa-user (U+F007), nf-md-refresh (U+F0450) |

Update this table when new files or icons are added.

## The Problem

The Write and Edit tools transmit file content as text. Characters in the
Private Use Area — especially Supplementary Private Use Area codepoints above
U+FFFF — may be silently dropped, replaced with replacement characters, or
truncated. The file saves without error, but the icons are gone.

This is invisible in diffs and code review because the missing character leaves
no trace — just an empty string where the icon was.

## Step 1: Verify Icons

After any edit, run the verification script:

```bash
python3 -c "
src = open('<file_path>').read()
icons = {
    'nf-md-web (U+F059F)': '\U000F059F',
    'nf-fa-keyboard_o (U+F11C)': '\uF11C',
    'nf-fa-user (U+F007)': '\uF007',
    'nf-md-refresh (U+F0450)': '\U000F0450',
}
for name, char in icons.items():
    if char in src:
        print(f'  OK  {name}')
    else:
        print(f'  MISSING  {name}')
"
```

Adjust the icon list to match what the file should contain.

## Step 2: Fix Missing Icons

If any icon is missing, use the placeholder-and-replace pattern:

1. **In the Edit tool**, use a unique ASCII placeholder string where the icon
   should go (e.g., `PLACEHOLDER_WEB`, `PLACEHOLDER_KEYBOARD`).
2. **Then run Python** to replace the placeholder with the real Unicode
   character:

```bash
python3 -c "
src = open('<file_path>').read()
src = src.replace('PLACEHOLDER_WEB', '\U000F059F')
src = src.replace('PLACEHOLDER_KEYBOARD', '\uF11C')
src = src.replace('PLACEHOLDER_USER', '\uF007')
src = src.replace('PLACEHOLDER_REFRESH', '\U000F0450')
open('<file_path>', 'w').write(src)
"
```

## Step 3: Verify Again

Re-run the verification script from Step 1 to confirm all icons are present.

## Python Escape Syntax

Codepoints at or below U+FFFF use `\uXXXX`:

```python
'\uF11C'   # U+F11C  (nf-fa-keyboard_o)
'\uF007'   # U+F007  (nf-fa-user)
```

Codepoints above U+FFFF use `\U00XXXXXX` (8 hex digits, zero-padded):

```python
'\U000F059F'  # U+F059F  (nf-md-web)
'\U000F0450'  # U+F0450  (nf-md-refresh)
```

Using `\uF059F` for a codepoint above U+FFFF is **wrong** — Python interprets it
as `\uF059` + literal `F`, producing a garbage character.

## Adding New Icons

When introducing a new Nerd Font icon to the project:

1. Add the icon name, codepoint, and file to the table in this skill.
2. Add it to the verification script's `icons` dict.
3. Add a placeholder constant for it.
4. Use the placeholder-and-replace pattern for the first embed.
