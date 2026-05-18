---
name: format-rust
description:
  "Format Rust files with cargo fmt. Use after creating or editing any Rust file
  (.rs)."
---

# Format Rust

Run `cargo fmt` on every Rust file after creating or editing it.

## When This Skill Applies

After every Write or Edit to any `.rs` file in the project. This includes:

- TUI source files (`tui/src/*.rs`)
- Any other Rust file in the project

## Process

After your edits to a Rust file are complete, run:

```bash
cargo fmt -- <file_path>
```

That's it. No other steps needed.
