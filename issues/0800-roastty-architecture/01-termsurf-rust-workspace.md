# Experiment 1: Create the TermSurf Rust Workspace

## Description

Create a top-level Cargo workspace for TermSurf-owned Rust crates so `webtui`,
`roamium`, future `roastty`, and shared crates can evolve together without
pulling in Wezboard.

Wezboard remains intentionally separate. It is a large WezTerm-derived project
with its own Cargo workspace, lockfile, dependencies, and build behavior.
Roastty is expected to replace Wezboard if the Ghostty-based architecture works
out, so the new workspace should not absorb Wezboard's inherited dependency
universe.

This experiment is structural only. It should not move protobuf generation,
deduplicate IPC code, create `roastty/`, or change runtime behavior. Those are
follow-up experiments once the workspace boundary is proven.

## Changes

1. Add a top-level `Cargo.toml` with a `[workspace]` containing only
   TermSurf-owned Rust members:
   - `webtui`
   - `roamium`

2. Use Cargo resolver 2.

3. Add an explicit `exclude` list for directories that must not be pulled into
   this workspace by accident:
   - `wezboard`
   - `vendor`
   - `chromium`
   - `proto/test-rust`

4. Generate a top-level `Cargo.lock` by running a workspace Cargo command.

5. Audit build scripts that run Cargo for TermSurf-owned Rust crates:
   - `scripts/build.sh`
   - `scripts/install.sh`
   - any helper scripts called by those scripts

   If the scripts still work correctly by changing into `webtui/` or `roamium/`,
   keep that behavior. If the workspace changes the canonical path, update the
   scripts to run from the repository root with package selectors such as
   `cargo build -p webtui` and `cargo build -p roamium`.

6. Decide what to do with the existing per-crate lockfiles:
   - Remove `webtui/Cargo.lock` and `roamium/Cargo.lock` if Cargo now uses the
     top-level lockfile for both packages.
   - Keep them only if a concrete build/test path still needs standalone
     lockfiles, and document why in the result.

7. Update documentation that describes Rust build workflows if the canonical
   commands change:
   - `AGENTS.md`
   - any README or docs file that mentions running Cargo directly for `webtui`
     or `roamium`

   If no documentation needs to change, record that in the result.

8. Do not edit `wezboard/Cargo.toml`, `wezboard/Cargo.lock`, or any file under
   `wezboard/`.

9. Do not create `roastty/` in this experiment. The ABI skeleton experiment
   should add it as a new workspace member.

## Verification

Run:

```bash
cargo metadata --format-version 1 --no-deps
cargo check -p webtui
cargo check -p roamium
```

Expected results:

- `cargo metadata` lists `webtui` and `roamium` as workspace members.
- `cargo metadata` does not list any `wezboard` package as a workspace member.
- `cargo check -p webtui` succeeds.
- `cargo check -p roamium` succeeds.
- Existing component build scripts still work:

```bash
./scripts/build.sh webtui
./scripts/build.sh roamium
```

Inspect any changed build scripts and documentation. They must describe and use
the same canonical build flow. If the workspace did not require script or doc
changes, the result must say why.

Also run:

```bash
git status --short
```

The only expected changes are:

- top-level `Cargo.toml`;
- top-level `Cargo.lock`;
- removal of `webtui/Cargo.lock` and `roamium/Cargo.lock`, if Cargo no longer
  needs them;
- build script updates, if needed;
- documentation updates, if needed;
- issue documentation updates.

## Failure Criteria

This experiment fails if:

- Wezboard becomes part of the top-level Cargo workspace.
- Any Wezboard files are edited.
- `cargo check -p webtui` or `cargo check -p roamium` fails because of the
  workspace change.
- Existing `scripts/build.sh webtui` or `scripts/build.sh roamium` behavior
  breaks.
- Build scripts and documentation disagree about whether Cargo should run from
  the repo root or inside each package directory.
- The experiment attempts to deduplicate protocol code, add `roastty/`, or
  otherwise expand beyond workspace setup.

## Follow-Up

If this passes, the next experiment should add the first shared crate or the
Roastty ABI skeleton. The likely order is:

1. add `crates/termsurf-proto` and move shared protobuf generation out of
   `webtui` and `roamium`; or
2. add `roastty/` as the first Rust ABI skeleton crate and include it in the
   workspace.

The choice should be made after seeing whether the workspace setup causes any
build-script or lockfile friction.
