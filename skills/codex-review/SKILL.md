---
name: codex-review
description:
  "Ask Codex to review TermSurf work. Use when the user requests Codex review or
  during large/risky experiments before acting on conclusions."
---

# Codex Review

Use Codex as an independent reviewer for TermSurf issue designs, code diffs,
experiment results, and architectural decisions.

The review is advisory. The calling agent remains responsible for deciding
whether Codex's feedback is correct, acting on valid findings, and explaining
any rejected findings.

Do not rely on Codex session memory alone. Session IDs are useful for grouping
related reviews and may help continuity, but every important review prompt must
include the essential issue, diff, log, and question context explicitly.

This skill is the Codex counterpart of `claude-review`. Use either or both when
you want a second model's read on risky work.

## When This Skill Applies

Use this skill when:

- the user asks for "codex review", "ask Codex", or similar;
- an experiment is large, risky, or touches Chromium;
- an experiment has already failed once and the next design needs review;
- a code change affects architecture, protocol boundaries, browser process
  behavior, popup/input routing, rendering geometry, or persistent state;
- before closing an issue after a complex series of experiments.

## Review Timeout

Every Codex review has a hard timeout of 15 minutes.

Use the helper script in this skill instead of calling `codex` directly. The
script enforces the timeout with Python's subprocess API and terminates Codex's
process group on timeout because macOS does not provide a portable `timeout`
command by default.

## Files and Logs

All review state and output lives under:

```text
logs/codex-review/
```

The helper script maintains:

| Path                                            | Purpose                         |
| ----------------------------------------------- | ------------------------------- |
| `logs/codex-review/current-session-id`          | reusable Codex session UUID     |
| `logs/codex-review/<timestamp>-prompt.md`       | exact prompt sent to Codex      |
| `logs/codex-review/<timestamp>-events.jsonl`    | raw Codex `--json` event stream |
| `logs/codex-review/<timestamp>-last-message.md` | Codex's final review message    |
| `logs/codex-review/<timestamp>-stderr.log`      | Codex stderr / timeout details  |

These logs are intentionally under `logs/`, which is gitignored. They may still
contain sensitive diffs or local paths. Keep context narrow and do not write
secrets into review prompts.

## Process

1. **Prepare the review context.** Include only the context Codex needs. Prefer
   concrete artifacts:
   - the user's current request;
   - the issue file and experiment section;
   - relevant `git diff` or `git diff --staged`;
   - relevant Chromium branch diff;
   - test commands and observed outputs;
   - screenshots/log paths when those are part of the evidence.

2. **Ask specific questions.** The prompt should tell Codex what kind of review
   to perform. Examples:
   - "Audit this experiment design for contradictions, missing verification, and
     architectural risks."
   - "Review this diff for correctness, regressions, and missing tests."
   - "Given these logs, identify what failed and what experiment should come
     next."

3. **Run the helper script.**

   ```bash
   python3 skills/codex-review/scripts/codex_review.py \
     --context issues/0789-electron-style-pdf-viewer/README.md \
     "Audit Experiment 6. Focus on whether the scope is right and whether the verification would prove the result."
   ```

   For a staged diff review:

   ```bash
   git diff --staged | python3 skills/codex-review/scripts/codex_review.py \
     "Review this staged diff for bugs, regressions, and missing tests."
   ```

   For a fresh thread:

   ```bash
   python3 skills/codex-review/scripts/codex_review.py --new-session \
     "Start a new review thread. Reply with the review session status."
   ```

4. **Read the result.** The helper prints:
   - the session ID;
   - the prompt path;
   - the raw event-stream path;
   - the final-message path;
   - the extracted Codex text when a final message was captured.

5. **Act on valid findings.** If Codex finds a real issue and the user asked for
   implementation, fix it. If Codex is wrong or speculative, do not make the
   change; briefly document why.

6. **Report high-signal results.** Summarize Codex's actionable findings to the
   user. Do not paste long raw output unless the user asks for it.

## Session Handling

Codex differs from Claude here. `claude -p` lets you choose a session id up
front with `--session-id <uuid>`. Codex does not: `codex exec` generates the id
itself. The helper therefore captures the id from the `--json` event stream
after the first run and stores it for follow-ups.

The helper uses `codex exec` for a new review thread and
`codex exec resume <uuid>` for follow-up turns.

**Default policy: maintain one continuous review session while it is usable.**
The helper resumes the stored session id on every run. Keep doing that — review
after review goes into the same thread until the session becomes unusable.

- **Do NOT pass `--new-session` on your own judgment** for ordinary topic
  changes. Not for a "different topic," not for a new experiment, not for a new
  issue. The user decides when to start a new thread, except for the automatic
  recovery cases below.
- If `logs/codex-review/current-session-id` exists and holds a valid UUID, the
  script resumes it. If it is missing or unparseable, the script starts a fresh
  session and stores the id Codex reports.
- **Automatic self-heal.** If resuming the stored session fails because the id
  stopped working (expired/unknown), or because the stored session is too full
  and Codex reports a context-window/compaction failure, the helper retries once
  as a fresh session, adopts the new id, and prints a note. You do not need to
  fall back to another reviewer — just use the fresh Codex session result.
- If you call the helper and manually observe a full-session error such as
  `context_length_exceeded`, `context window`, or a failed remote compaction,
  rerun the same Codex review with `--new-session`. Do not switch to Claude or
  another agent merely because the old Codex session is full.
- `--new-session` (explicit user request only) ignores the stored id and starts
  fresh.
- Always include the essential context again on every review. Resuming the same
  session id works, but model-visible history should not be treated as
  guaranteed.
- `codex exec resume --last` is a manual fallback if you ever need to reattach
  to the most recent session by hand.

## Codex Command Shape

The helper invokes Codex like this for a new session:

```bash
codex exec \
  -s read-only \
  --json \
  --color never \
  -o logs/codex-review/<timestamp>-last-message.md \
  -   # prompt is piped in on stdin
```

For follow-up turns it replaces `codex exec` with `codex exec resume <uuid>` and
drops `-s` (see below).

The default sandbox is `read-only`, which is genuinely read-only — Codex can
read the repo to inspect files but cannot modify the working tree. `codex exec`
is non-interactive by design, so it never blocks on an approval prompt (there is
no `-a` flag on `codex exec`). This is the review-oriented analog of
`claude-review`'s read-only Read/Grep/Glob tool set under
`--permission-mode dontAsk`.

The sandbox is fixed when the session is created. `codex exec resume` does not
accept `-s`; a resumed turn inherits the sandbox the session was created with.
To review under a different sandbox, start a new session with `--new-session`.

If a review truly needs to run commands (not just read), create the session with
a workspace-write sandbox and include the reason in the user-facing summary:

```bash
python3 skills/codex-review/scripts/codex_review.py --allow-bash ...
```

If a review should use only supplied context, run with:

```bash
python3 skills/codex-review/scripts/codex_review.py --no-tools ...
```

`--no-tools` is best-effort: Codex has no per-tool allowlist for `codex exec`,
so the helper keeps the read-only sandbox and instructs Codex in the prompt to
use only the supplied context.

## Prompt Template

Use this structure for substantial reviews:

```text
You are reviewing TermSurf work. Take a code-review stance: findings first,
ordered by severity, with file/line references where possible.

Task:
<what Codex should review>

Context:
<issue, experiment, diff, logs, screenshots, test output>

Questions:
1. Is the design/implementation correct?
2. What are the concrete risks or missed cases?
3. What should be changed before implementation/commit?
4. If nothing should change, say that clearly.
```

## Automatic Use in Large Experiments

For large experiments, run Codex review at one or both checkpoints:

1. **After designing the experiment, before implementation.** Ask Codex to audit
   the design for contradictions, missing verification, and scope creep.

2. **After implementation, before final result or commit.** Ask Codex to review
   the diff, logs, and result language.

Do not block forever waiting for Codex. The 15-minute timeout is the maximum. If
Codex times out, record that and continue with normal engineering judgment.

## Smoke Test

After creating or changing this skill, test the helper without giving Codex
repo-changing authority:

```bash
python3 skills/codex-review/scripts/codex_review.py --new-session --no-tools \
  "Smoke test: reply with OK and one sentence describing what session continuity means."
```

Then run a second call without `--new-session`:

```bash
python3 skills/codex-review/scripts/codex_review.py --no-tools \
  "In one sentence, say whether this is the same review session."
```

Both outputs should be saved under `logs/codex-review/`. The printed
`session_id` should match between the two runs. The second answer does not need
to prove memory of the first prompt; the important smoke-test properties are
exit code 0, saved artifacts, and stable session ID reuse.
