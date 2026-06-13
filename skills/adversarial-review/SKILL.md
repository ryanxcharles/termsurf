---
name: adversarial-review
description:
  "Run an in-session adversarial review of TermSurf work using a fresh-context
  subagent (Codex: multi_agent_v1.spawn_agent; Claude: `adversarial-reviewer`).
  Use at experiment design/result gates, or whenever the user asks for
  adversarial / skeptical / red-team review without an external reviewer CLI."
---

# Adversarial Review

Run a fresh-context, read-only adversarial review **inside the current agent
session** by delegating to an adversarial subagent. No external reviewer CLI, no
session id, no logs to manage — spawn a subagent and it returns its verdict and
findings.

Runtime-specific invocation:

- **Codex:** use `multi_agent_v1.spawn_agent`. Pass the adversarial reviewer
  instructions in the spawn prompt, plus concrete artifact paths. Do **not** try
  to use Claude's `Agent` tool.
- **Claude:** use the `Agent` tool with `subagent_type: "adversarial-reviewer"`,
  defined in `.claude/agents/adversarial-reviewer.md`.

This is the in-session counterpart of the `codex-review` and `claude-review`
skills, which shell out to a separate `codex exec` / `claude -p` process. Use
this skill when you want the review to run in the same session; use a
cross-model reviewer when you specifically want a **different model's**
independent read (see "Self-review caveat" below).

## When this skill applies

- The user asks for an "adversarial review", "skeptical review", "red team",
  "try to break this", or similar.
- An experiment reaches its **design gate** (after the design is written, before
  implementation) or its **result gate** (after implementation + result
  recording, before the result commit). These are the two required AI review
  gates in `CLAUDE.md`'s experiment flow.
- A change is large, risky, or touches Chromium, protocol boundaries, browser
  process behavior, input/rendering, persistent state, or `unsafe` Rust.
- Before closing an issue after a complex series of experiments.

## Reviewer posture

The subagent runs in **its own fresh context window** — it does **not** see this
conversation unless you explicitly fork context. Default to
`fork_context: false` for Codex reviews. It receives only what you put in the
spawn prompt plus whatever it reads itself with its available tools. It must be
instructed to try to reject the work on evidence, verify claimed gate results
independently where feasible, and return a structured verdict.

Use this reviewer mandate in Codex spawn prompts, or rely on the Claude agent
file that already contains it:

```text
You are the adversarial reviewer for TermSurf. You are separate from whoever
produced the work under review. Your default posture is skepticism. Try to
reject the work, but every objection must be grounded in evidence you can point
to.

Read-only discipline: do not edit, write, create, move, delete, stage, commit,
push, or run mutating commands. Use shell commands only for inspection and
read-only verification such as git diff/log/show/status, rg, cargo test, cargo
build, and cargo fmt --check. If a check would modify files, do not run it;
state that you could not verify it.

Return:
VERDICT: APPROVED | CHANGES REQUIRED
Then findings, most severe first:
[Required] file:line — issue · Evidence: ... · Fix: ...
[Optional] file:line — issue · Evidence: ... · Fix: ...
[Nit] file:line — issue · Fix: ...

Approve only when zero Required findings remain. Do not invent findings.
```

Because it starts blind, **you must hand it the artifacts** — point it at the
files; do not paraphrase them. Give it:

- the experiment file (`issues/<n>/NN-*.md`);
- the relevant diff (tell it the exact `git diff` / `git diff --staged` /
  `git show <ref>` command to run, or the changed file paths);
- the source files it should scrutinize;
- the upstream source to compare against, for ports (e.g.
  `vendor/ghostty/src/...`);
- `AGENTS.md`/repo instructions available in the prompt and the issue
  `README.md` as the workflow contract;
- any command output whose truth matters (test counts, build logs).

## Invocation

### Codex invocation

Use `multi_agent_v1.spawn_agent` with `fork_context: false` unless the review
explicitly needs the current conversation. Put the reviewer mandate, review
task, and artifact pointers in the prompt. Example:

```text
Spawn a Codex subagent with this task:

Review this TermSurf experiment DESIGN with fresh context. Do not edit anything.
Use the adversarial reviewer posture: try to reject on evidence; return
VERDICT and findings.

Read:
- issue instructions: issues/0801-roastty-libghostty-rewrite/README.md
- experiment file: issues/0801-roastty-libghostty-rewrite/829-*.md
- relevant source: roastty/src/renderer/frame_rebuild.rs

Check scope, workflow, correctness, verification quality, and whether the README
links the experiment as Designed. Approve only if no Required findings remain.
```

Wait for the agent only when its verdict gates your next step. Close the agent
after recording or acting on its result.

### Claude invocation

Spawn the subagent with Claude's `Agent` tool,
`subagent_type: "adversarial-reviewer"`. Put the review task and artifact
pointers in the prompt. Example:

> Use the **adversarial-reviewer** subagent to review the Experiment 620 design.
> Read `issues/0801-roastty-libghostty-rewrite/620-*.md`, `CLAUDE.md`, and the
> upstream `vendor/ghostty/src/config/url.zig`. Try to reject the design; return
> your verdict and findings.

The subagent's final message — its `VERDICT` plus findings — comes back to you
as the tool result. It is not shown to the user automatically; relay the
high-signal parts.

### Design-gate prompt template

```text
Review this TermSurf experiment DESIGN with fresh context. Do not edit anything.

Read:
- the experiment file: issues/<n>/NN-<slug>.md
- the workflow contract: repo instructions/AGENTS.md and issues/<n>/README.md
- the upstream being ported (if any): vendor/ghostty/src/<path>

Try to reject this design. Check:
- the issue README links this experiment with status Designed;
- the experiment has Description, Changes, and Verification;
- scope is narrow enough for one experiment, and matches exactly what was asked;
- the technical plan is correct and faithful to upstream;
- verification has concrete pass/fail criteria that would actually prove the goal;
- required hygiene checks are present (fmt, build-no-warnings, tests, no-ghostty
  grep, git diff --check).

Return VERDICT (APPROVED | CHANGES REQUIRED) then findings (Required/Optional/Nit)
with file:line, evidence, and a concrete fix. Approve only if no Required remain.
```

### Result-gate prompt template

```text
Review this COMPLETED TermSurf experiment with fresh context. Do not edit anything.

Read:
- the experiment file (Description, Changes, Verification, Result): issues/<n>/NN-<slug>.md
- the implementation diff: run `git diff <plan-commit>..HEAD -- <paths>` (or the
  working tree if not yet committed)
- the changed source and the upstream it ports: vendor/ghostty/src/<path>
- the workflow contract: repo instructions/AGENTS.md

Try to reject this result. Check:
- the implementation matches the approved scope — no unrequested changes;
- it is correct and faithful to upstream; find the specific divergence if any;
- the tests actually prove the claim (not vacuous, cover the interesting cases);
- independently verify the claimed gate results where feasible: run
  `cargo build -p <crate>`, `cargo test -p <crate>`, `cargo fmt -p <crate> -- --check`,
  and the no-ghostty grep; report any mismatch with the stated numbers;
- the experiment file has Result and Conclusion, and the README status matches;
- the result commit has NOT been made before this review.

Return VERDICT then findings (Required/Optional/Nit) with file:line, evidence, and
a concrete fix. Approve only if no Required remain.
```

### Re-review prompt template

```text
Re-review ONLY the fixes for your prior findings, with fresh context. Do not edit.
For each prior finding, confirm whether it is now resolved, citing the new
file:line. Report any new Required finding the fix introduced. Approve only if no
Required remain.
```

## After the review: lead-agent judgment

You (the implementing agent) stay responsible for the outcome. The review is
input, not a verdict you must obey blindly.

1. **Accept** findings that are real correctness, fidelity, verification, scope,
   or workflow issues. Fix them before proceeding.
2. **Reject** false positives explicitly, with a one-line reason — do not
   silently ignore a finding.
3. **Re-review** after non-trivial fixes (use the re-review template) until no
   Required findings remain.
4. **Record** the review in the experiment file: that it was an adversarial
   subagent with fresh context, whether it was Codex-native or Claude's named
   `adversarial-reviewer`, the findings, the fixes, and the final verdict.
5. Respect the commit gates: do not implement after a design review until the
   plan commit exists; do not design the next experiment after a result review
   until the result commit exists.
6. Give the adversarial reviewer up to **15 minutes** to finish. Do not
   interrupt it for a bounded verdict, close it, or proceed around the review
   before that time has elapsed unless the user explicitly asks you to stop or
   change direction. If it completes earlier, use the completed verdict
   normally.

## Self-review caveat (read this)

This subagent is usually the **same model family** as the implementer (Codex
reviewing Codex, or Claude reviewing Claude). That is convenient and fast, but a
same-model reviewer shares blind spots and can drift toward agreement. The
subagent's design fights this with fresh context, a hard "try to reject on
evidence" mandate, read-only discipline, independent re-verification of claimed
results, and a no-approval-with-Required-findings gate — but it does not fully
replace a genuinely different model.

Therefore:

- For routine gates, the in-session adversarial subagent is a reasonable
  default.
- For **high-risk** work (Chromium, protocol changes, tricky `unsafe`, anything
  that already failed once), prefer a **cross-model** check via `codex-review`
  or `claude-review`, or run both and reconcile.
- You can raise rigor by spawning the subagent **two or three times in
  parallel** with different emphases (e.g. one on correctness, one on upstream
  fidelity, one on verification quality) and treating any Required finding from
  any pass as blocking. This breaks single-perspective blind spots without
  leaving the session.

## Notes

- The subagent is **read-only by discipline**, not necessarily by hard sandbox.
  It may have shell access so it can run `git diff` and read-only builds/tests.
  The prompt must forbid mutating commands.
- The Claude named agent's `model` is set in
  `.claude/agents/adversarial-reviewer.md`. Codex subagents inherit the current
  model by default; do not request a different model unless the user asks or the
  task clearly requires it.
- Claude named subagents are loaded at session start. Codex-native use does not
  depend on the Claude agent registry; it relies on `multi_agent_v1.spawn_agent`
  plus this skill's reviewer mandate.
