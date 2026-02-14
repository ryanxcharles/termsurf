# Chromium Fork

## Repository

| Remote   | URL                                                |
| -------- | -------------------------------------------------- |
| upstream | https://chromium.googlesource.com/chromium/src.git |

No `origin` remote for now. Remote hosting TBD (likely patch set distribution).

## Current State

- Branch: `146.0.7650.0-termsurf`
- Commit: `b2907d660628a`
- Base version: `146.0.7650.0` (tracking Electron's Chromium version)

## Branch Strategy

Track the same Chromium version as Electron. Branches are named
`{version}-termsurf` for the main working branch and `{version}-issue-{N}` for
issue-specific branches.

## Branches

| Branch                   | Issue                                       | Description                  |
| ------------------------ | ------------------------------------------- | ---------------------------- |
| `146.0.7650.0-termsurf`  | —                                           | Main working branch for v146 |
| `146.0.7650.0-electron`  | —                                           | Electron's v146 base         |
| `146.0.7650.0-issue-411` | [Issue 411](issues/411-two-profiles-3.md)   | Two profiles experiment 3    |
| `146.0.7650.0-issue-412` | [Issue 412](issues/412-one-profile.md)      | One profile                  |
| `146.0.7650.0-issue-413` | [Issue 413](issues/413-one-profile-2.md)    | One profile experiment 2     |
| `146.0.7650.0-issue-414` | [Issue 414](issues/414-two-profiles-xpc.md) | Two profiles via XPC         |
| `146.0.7650.0-issue-415` | [Issue 415](issues/415-swift-receiver.md)   | Swift receiver               |
| `146.0.7650.0-issue-416` | [Issue 416](issues/416-rust-receiver.md)    | Rust receiver                |

## Local Setup

The `chromium/` directory at the repo root is a Chromium build
workspace, gitignored from the main repo. The `src/` subdirectory is the
Chromium git repo (Chromium requires it to be named `src/`). To set up from
scratch, use `fetch chromium` from depot_tools or clone from upstream and apply
our patches (patch distribution TBD).
