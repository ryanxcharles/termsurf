# Experiment 4: Prove Roamium PDF Links

## Description

Experiment 3 proved current Roamium PDF keyboard page/scroll navigation and
toolbar page-selector navigation. The next unproven core Roamium workflow is PDF
link behavior:

- internal links inside a PDF;
- external links from a PDF to a normal web page.

Issue 796 explicitly left both rows unknown because no current harness fixture
or log asserted them. This experiment adds a deterministic PDF link fixture and
probes current Roamium behavior before changing product code.

## Changes

1. Add deterministic PDF link fixtures for automation.

   Add a small generated fixture under `test-html/public/` or generate it inside
   the test log directory. It should contain:

   - at least two pages;
   - a visible internal link on page 1 that navigates to page 2;
   - a visible external link that navigates to a local HTTP page served by the
     harness, such as `/pdf-link-target.html`.

   Prefer generating the PDF in the harness if that keeps the fixture
   self-contained and avoids binary churn. If a checked-in PDF is easier, keep
   it small and deterministic.

2. Add a narrow Roamium PDF link probe.

   Create or extend an issue-specific harness, likely
   `scripts/test-issue-834-pdf-links.py` plus a small DevTools helper if useful.
   Reuse the TermSurf socket/protobuf and DevTools patterns from the Issue 794
   and Experiment 3 harnesses.

   The harness should:

   - launch `chromium/src/out/Default/roamium` with trace/log output under
     `--log-dir`;
   - serve the PDF fixture and external target page through a deterministic
     local HTTP server;
   - create a Roamium tab through the TermSurf protocol;
   - resize the tab to a stable viewport;
   - discover the DevTools port and PDF extension child target;
   - capture before/after screenshots and viewer state;
   - click the internal link through TermSurf protocol mouse input, not DevTools
     synthetic DOM click;
   - click the external link through TermSurf protocol mouse input, not DevTools
     synthetic DOM click;
   - write one summary JSON file at `<log-dir>/pdf-links-summary.json`.

   The harness may use DevTools to inspect coordinates and state, but the link
   activation path must be user-level TermSurf mouse input.

3. Run the new probes.

   Use fresh log directories:

   ```bash
   python3 scripts/test-issue-834-pdf-links.py \
     --log-dir logs/issue-834-exp4-internal-link \
     --probe internal-link
   python3 scripts/test-issue-834-pdf-links.py \
     --log-dir logs/issue-834-exp4-external-link \
     --probe external-link
   ```

4. If a probe fails, stop and record the first failing layer.

   Do not continue into find/search, restrictions, password PDFs, malformed
   PDFs, Surfari, or PDF advanced features in this experiment. If link behavior
   fails, record whether the failure is in fixture generation, coordinate
   discovery, protocol mouse delivery, Chromium/PDF input routing, PDF viewer
   navigation, external navigation, or evidence collection.

## Verification

Verification for the completed result is:

- the link fixture is deterministic and documented in the result;
- internal-link activation uses TermSurf protocol mouse input;
- external-link activation uses TermSurf protocol mouse input;
- the internal-link probe records click coordinates, protocol mouse message
  count, Roamium mouse trace evidence, Chromium/PDF input evidence when
  available, before/after page or scroll state, before/after screenshot hashes,
  and the pass/fail delta;
- the external-link probe records click coordinates, protocol mouse message
  count, Roamium mouse trace evidence, Chromium/PDF input evidence when
  available, before/after URL or target state, before/after screenshot hashes,
  and the pass/fail delta;
- both probes write summary JSON files under `logs/issue-834-exp4-*`;
- the experiment result cites command, exit status, summary file, summary
  status, first failing hop, and matrix rows proven or not proven;
- no product code is changed unless a probe exposes a real TermSurf integration
  bug and that fix is explicitly documented in this experiment;
- no Chromium source is changed unless a fresh Chromium branch and patch archive
  are created according to `chromium/AGENTS.md`;
- design review is recorded and the plan commit exists before implementation
  begins;
- markdown is formatted with Prettier;
- `git diff --check` passes;
- completion review is recorded before the result commit.

## Design Review

Fresh-context adversarial review by Codex subagent `Hilbert`: **Approved**.

Findings: none.

## Pass Criteria

This experiment passes if both the internal PDF link and external PDF link
probes pass with current evidence from TermSurf protocol mouse activation.

## Partial Criteria

This experiment is partial if one link class is proven and the other fails or
cannot be automated with a concrete first failing layer.

## Failure Criteria

This experiment fails if neither link class can be proven, if the harness claims
success without state/screenshot evidence, or if it bypasses the TermSurf mouse
path for link activation.
