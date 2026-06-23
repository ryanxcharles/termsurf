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

## Result

**Result:** Pass

Implemented the deterministic PDF link probe and used it to test both link
classes through TermSurf protocol mouse input.

Added:

- `scripts/test-issue-834-pdf-links.py`
- `scripts/probe-pdf-links.mjs`

The harness generates a two-page PDF in the log directory. Page 1 contains a
top-half internal link to page 2 and a bottom-half external link to
`/pdf-link-target.html`, both served by the harness HTTP server. The harness
launches `chromium/src/out/Default/roamium`, creates a tab through the TermSurf
socket protocol, discovers the PDF plugin rectangle through DevTools, sends
TermSurf protocol mouse down/up events, and writes `pdf-links-summary.json`.

Syntax checks:

```bash
python3 -m py_compile scripts/test-issue-834-pdf-links.py
node --check scripts/probe-pdf-links.mjs
```

Both exited 0.

Initial probe results:

```bash
python3 scripts/test-issue-834-pdf-links.py \
  --log-dir logs/issue-834-exp4-internal-link \
  --probe internal-link
```

Exit status: 0. Summary:
`logs/issue-834-exp4-internal-link/pdf-links-summary.json`.

The internal link passed with `first_failing_hop = "no-failure-observed"`. The
probe sent two protocol mouse messages, clicked the top half of the plugin
rectangle at `(450.5, 154.5)`, observed Roamium mouse trace evidence, observed
Chromium/PDF routing evidence, and verified the PDF viewer moved from page 1 to
page 2 with a changed screenshot hash.

```bash
python3 scripts/test-issue-834-pdf-links.py \
  --log-dir logs/issue-834-exp4-external-link \
  --probe external-link
```

Exit status: 1. Summary:
`logs/issue-834-exp4-external-link/pdf-links-summary.json`.

The first failing layer was `external-navigation-target-missing`. Protocol mouse
input reached Roamium and Chromium, and PDFium reported `outcome=navigate-link`,
but the harness HTTP log showed no request for `/pdf-link-target.html`. The
failure was not geometry or mouse routing. It was the PDF viewer's external
current-tab navigation path.

Root cause: Chromium's PDF viewer calls `BrowserApi.navigateInCurrentTab()`. In
Chrome, that uses `chrome.tabs.update(tabId, {url})`. TermSurf's generated PDF
wrapper runs as a MimeHandlerView-style top-level page with a PDF extension
iframe, but the stream container uses tab id `-1`, and Roamium is not Chrome's
tab-strip browser. With no valid `chrome.tabs` current-tab path, the viewer
silently declined to navigate.

Fix:

- created Chromium branch `148.0.7778.97-issue-834-exp4`;
- committed Chromium change `367f2f2a49` (`Bridge PDF link navigation`);
- added a TermSurf-specific fallback in
  `chrome/browser/resources/pdf/browser_api.ts` that posts
  `termsurfNavigateInCurrentTab` to the parent wrapper when the Chrome tabs path
  is unavailable;
- injected a small wrapper listener from
  `content/libtermsurf_chromium/ts_plugin_response_interceptor_url_loader_throttle.cc`
  that accepts that message only from the PDF extension origin, validates the
  destination URL protocol, and navigates the top-level wrapper with
  `window.location.assign()`;
- rebuilt Chromium with:

```bash
cd /Users/astrohacker/dev/termsurf/chromium/src
PATH="/Users/astrohacker/dev/termsurf/chromium/depot_tools:$PATH" \
  autoninja -C out/Default libtermsurf_chromium
```

Exit status: 0.

The Chromium patch archive was written to `chromium/patches/issue-834/`, copied
from the cumulative Issue 799 parent archive and appended with patch 74,
`0074-Bridge-PDF-link-navigation.patch`. The local Chromium checkout is shallow
and does not contain the vanilla base commit object, so this preserves a
complete archive by extending the latest available cumulative parent archive.

Rerun results:

```bash
python3 scripts/test-issue-834-pdf-links.py \
  --log-dir logs/issue-834-exp4-internal-link-rerun1 \
  --probe internal-link
```

Exit status: 0. Summary:
`logs/issue-834-exp4-internal-link-rerun1/pdf-links-summary.json`.

The internal link still passed: `first_failing_hop = "no-failure-observed"`, two
protocol mouse messages, page 1 to page 2, and a changed screenshot hash.

```bash
python3 scripts/test-issue-834-pdf-links.py \
  --log-dir logs/issue-834-exp4-external-link-rerun1 \
  --probe external-link
```

Exit status: 0. Summary:
`logs/issue-834-exp4-external-link-rerun1/pdf-links-summary.json`.

The external link passed: `first_failing_hop = "no-failure-observed"`, two
protocol mouse messages, click point `(450.5, 351.5)`, Roamium mouse trace
evidence, Chromium/PDF routing evidence, PDFium `outcome=navigate-link`, wrapper
log `has_termsurf_navigation_bridge=1`, HTTP `GET /pdf-link-target.html`, and a
final DevTools target at `http://127.0.0.1:9799/pdf-link-target.html` with title
`PDF Link Target` and body text `PDF external link target reached`.

Matrix rows proven for Roamium:

- internal PDF links: pass, automated by
  `scripts/test-issue-834-pdf-links.py --probe internal-link`;
- external PDF links: pass, automated by
  `scripts/test-issue-834-pdf-links.py --probe external-link`.

## Conclusion

Roamium PDF internal and external links now work through real TermSurf protocol
mouse input. The only product gap found was external current-tab navigation from
Chromium's PDF extension iframe in a non-Chrome embedder. The fix keeps the
Chrome `tabs.update` path intact for real Chrome tab contexts and adds a narrow,
origin-checked TermSurf wrapper bridge for Roamium's MimeHandlerView-style PDF
wrapper.

## Completion Review

Fresh-context adversarial review by Codex subagent `Curie`: **Approved**.

Findings: none.
