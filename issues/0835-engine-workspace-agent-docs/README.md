+++
status = "open"
opened = "2026-06-22"
+++

# Issue 835: Engine Workspace Agent Docs

## Goal

Make the Chromium and WebKit engine workspaces use analogous `AGENTS.md`
instructions for build, branch, and patch workflow documentation.

## Background

TermSurf maintains two large browser-engine workspaces:

- `chromium/` for Roamium's Chromium fork;
- `webkit/` for Surfari's WebKit fork.

Both workspaces have the same broad shape:

- an ignored upstream source checkout inside the workspace;
- tracked TermSurf documentation at the workspace root;
- tracked patch archives generated from issue branches;
- issue-specific branches inside the engine source checkout;
- build commands that should be easy for agents to find before modifying or
  building the engine.

Today the two workspaces are close, but not aligned:

- Chromium has detailed workflow and build instructions in `chromium/README.md`
  and a separate `skills/chromium/SKILL.md`.
- WebKit has detailed workflow and build instructions in `webkit/README.md`.
- Neither workspace has a root `AGENTS.md`.
- There is no WebKit skill, and the desired direction is not to add one.

The desired end state is simpler and more symmetric: each engine workspace has
its own `AGENTS.md` containing the instructions an agent needs when working in
that workspace. No separate Chromium or WebKit skill should be required for
basic build/branch/patch workflow discovery.

## Requirements

- Add `chromium/AGENTS.md` with full Chromium workspace instructions.
- Add `webkit/AGENTS.md` with full WebKit workspace instructions.
- Make the two files structurally analogous where the engines allow it:
  - purpose and source layout;
  - ignored source checkout policy;
  - build prerequisites;
  - setup/bootstrap commands;
  - build commands;
  - branch naming and branch table maintenance;
  - patch generation and application;
  - cache/build-output cautions;
  - verification commands;
  - rules for when engine source changes require issue-specific branches and
    patch archive updates.
- Keep engine-specific differences explicit instead of pretending the workflows
  are identical:
  - Chromium uses `depot_tools`, `gclient`, `gn`, and `autoninja`;
  - WebKit uses the upstream WebKit checkout and
    `Tools/Scripts/build-webkit --debug`;
  - Chromium's source checkout must be named `chromium/src`;
  - WebKit's source checkout currently lives at `webkit/src`;
  - Chromium tracks Electron's Chromium version;
  - WebKit currently tracks an upstream commit.
- Decide whether `chromium/README.md` and `webkit/README.md` should remain the
  canonical human documentation, point to `AGENTS.md`, or be reduced to
  workspace summaries. Avoid contradictory duplicated instructions.
- Update or remove the separate Chromium skill dependency if it becomes stale or
  misleading. The target behavior is that agents find the engine instructions in
  `chromium/AGENTS.md` and `webkit/AGENTS.md`, not in a separate skill.
- Do not modify Chromium or WebKit source code as part of the first
  documentation alignment experiment.

## Analysis

The safest implementation is documentation-only:

1. Extract the durable Chromium workflow from `chromium/README.md` and
   `skills/chromium/SKILL.md` into `chromium/AGENTS.md`.
2. Extract the durable WebKit workflow from `webkit/README.md` and Issue 756
   Experiment 4 into `webkit/AGENTS.md`.
3. Keep README files either as workspace overviews or pointers to the new agent
   instructions.
4. Run markdown formatting and verify that the issue index and docs remain
   consistent.

The new files should not be placeholders. They should be complete enough that an
agent entering either engine workspace can answer:

- what source checkout is local and ignored;
- which branch to create for a new issue;
- how patches are archived and replayed;
- how to build safely;
- what not to do to avoid destroying expensive build state.

This issue is complete when Chromium and WebKit have analogous root `AGENTS.md`
files, stale or conflicting instructions are removed or redirected, and the new
documentation has been reviewed through the normal issues-and-experiments
workflow.
