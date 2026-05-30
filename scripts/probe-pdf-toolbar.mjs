#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";

function parseArgs(argv) {
  const args = {
    timeoutSeconds: 30,
    settleSeconds: 8,
    actionSettleMs: 800,
  };
  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    if (!arg.startsWith("--")) {
      throw new Error(`unexpected positional argument: ${arg}`);
    }
    const [rawKey, inlineValue] = arg.slice(2).split("=", 2);
    const key = rawKey.replace(/-([a-z])/g, (_, ch) => ch.toUpperCase());
    const value = inlineValue ?? argv[++i];
    if (value === undefined) {
      throw new Error(`missing value for ${arg}`);
    }
    args[key] = value;
  }
  for (const key of ["devtoolsPort", "urlContains", "outDir"]) {
    if (!args[key]) {
      throw new Error(
        `missing --${key.replace(/[A-Z]/g, (c) => `-${c.toLowerCase()}`)}`,
      );
    }
  }
  args.devtoolsPort = Number(args.devtoolsPort);
  args.timeoutSeconds = Number(args.timeoutSeconds);
  args.settleSeconds = Number(args.settleSeconds);
  args.actionSettleMs = Number(args.actionSettleMs);
  args.outDir = path.resolve(args.outDir);
  return args;
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function writeJson(filePath, value) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, `${JSON.stringify(value, null, 2)}\n`);
}

async function fetchJson(url) {
  const response = await fetch(url);
  if (!response.ok) {
    throw new Error(`GET ${url} failed: HTTP ${response.status}`);
  }
  return await response.json();
}

async function pollTarget(args, summary) {
  const deadline = Date.now() + args.timeoutSeconds * 1000;
  const listUrl = `http://127.0.0.1:${args.devtoolsPort}/json/list`;
  let lastTargets = [];
  while (Date.now() < deadline) {
    try {
      lastTargets = await fetchJson(listUrl);
      const match = lastTargets.find(
        (target) =>
          target.type === "page" &&
          typeof target.url === "string" &&
          target.url.includes(args.urlContains) &&
          target.webSocketDebuggerUrl,
      );
      if (match) {
        return match;
      }
    } catch {
      // Keep polling while Roamium starts DevTools.
    }
    await sleep(250);
  }
  summary.availableTargets = lastTargets.map((target) => ({
    id: target.id,
    type: target.type,
    url: target.url,
    title: target.title,
  }));
  throw new Error(
    `no page target contained ${JSON.stringify(args.urlContains)}`,
  );
}

function connectDevTools(wsUrl) {
  const socket = new WebSocket(wsUrl);
  let nextId = 1;
  const pending = new Map();
  const events = [];

  socket.addEventListener("message", (event) => {
    const message = JSON.parse(event.data);
    if (message.id && pending.has(message.id)) {
      const { resolve, reject } = pending.get(message.id);
      pending.delete(message.id);
      if (message.error) {
        reject(
          new Error(
            `${message.error.message || "DevTools error"} (${message.error.code})`,
          ),
        );
      } else {
        resolve(message.result || {});
      }
      return;
    }
    if (message.method) {
      events.push(message);
    }
  });

  const open = new Promise((resolve, reject) => {
    socket.addEventListener("open", resolve, { once: true });
    socket.addEventListener("error", reject, { once: true });
  });

  function send(method, params = {}, sessionId = undefined) {
    const id = nextId;
    nextId += 1;
    const promise = new Promise((resolve, reject) => {
      pending.set(id, { resolve, reject });
    });
    const message = { id, method, params };
    if (sessionId) {
      message.sessionId = sessionId;
    }
    socket.send(JSON.stringify(message));
    return promise;
  }

  return { socket, open, send, events };
}

async function safeSend(client, method, params = {}, sessionId = undefined) {
  try {
    return { ok: true, result: await client.send(method, params, sessionId) };
  } catch (error) {
    return { ok: false, error: String(error.message || error) };
  }
}

async function evaluate(client, expression, sessionId = undefined) {
  const result = await safeSend(
    client,
    "Runtime.evaluate",
    { expression, awaitPromise: true, returnByValue: true },
    sessionId,
  );
  if (!result.ok) {
    return { ok: false, error: result.error };
  }
  if (result.result.exceptionDetails) {
    return { ok: false, error: JSON.stringify(result.result.exceptionDetails) };
  }
  return { ok: true, value: result.result.result?.value ?? null };
}

async function captureScreenshot(client, args, relativePath) {
  const result = await client.send("Page.captureScreenshot", {
    format: "png",
    fromSurface: true,
  });
  const png = Buffer.from(result.data || "", "base64");
  const filePath = path.join(args.outDir, relativePath);
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, png);
  return { relativePath, bytes: png.length, base64Sha: await sha256(png) };
}

async function sha256(buffer) {
  const digest = await crypto.subtle.digest("SHA-256", buffer);
  return Array.from(new Uint8Array(digest))
    .map((byte) => byte.toString(16).padStart(2, "0"))
    .join("");
}

const STATE_SOURCE = `(() => {
  const textOf = (el) => [
    el.getAttribute("aria-label") || "",
    el.getAttribute("title") || "",
    el.id || "",
    String(el.className || ""),
    el.getAttribute("role") || "",
    el.innerText || "",
    el.value || "",
  ].join(" ").replace(/\\s+/g, " ").trim();
  const rectOf = (el) => {
    const rect = el.getBoundingClientRect();
    return {
      tag: el.tagName,
      id: el.id || "",
      className: String(el.className || ""),
      role: el.getAttribute("role") || "",
      ariaLabel: el.getAttribute("aria-label") || "",
      title: el.getAttribute("title") || "",
      text: (el.innerText || el.value || "").slice(0, 120),
      type: el.getAttribute("type") || "",
      disabled: !!el.disabled || el.getAttribute("aria-disabled") === "true",
      hidden: !!el.hidden || getComputedStyle(el).visibility === "hidden" || getComputedStyle(el).display === "none",
      x: rect.x,
      y: rect.y,
      width: rect.width,
      height: rect.height,
      right: rect.right,
      bottom: rect.bottom,
      token: textOf(el),
    };
  };
  const controls = [];
  const elements = [];
  const visit = (root, depth = 0, scope = "document") => {
    if (!root || depth > 10 || controls.length > 240 || elements.length > 240) {
      return;
    }
    for (const el of root.querySelectorAll("button,input,select,cr-icon-button,*[role=button],viewer-toolbar,viewer-page-selector,viewer-zoom-toolbar,viewer-download-controls")) {
      controls.push({...rectOf(el), scope, depth});
    }
    for (const el of root.querySelectorAll("embed,iframe,canvas,pdf-viewer,#viewer,#plugin,#sizer,#container,#page-container,.page,.page-container,viewer-toolbar,viewer-page-selector,viewer-zoom-toolbar")) {
      elements.push({...rectOf(el), scope, depth});
    }
    for (const el of root.querySelectorAll("*")) {
      if (el.shadowRoot) {
        visit(el.shadowRoot, depth + 1, \`\${scope} > \${el.tagName.toLowerCase()}#\${el.id || ""}\`);
      }
    }
  };
  visit(document);
  const scrolling = document.scrollingElement || document.documentElement || document.body;
  const pageIndicators = controls.filter((c) => /page|頁/i.test(c.token));
  const zoomIndicators = controls.filter((c) => /%|zoom|fit/i.test(c.token));
  const visibleElements = elements.filter((el) => !el.hidden && el.width > 0 && el.height > 0);
  const largest = [...visibleElements].sort((a, b) => b.width * b.height - a.width * a.height)[0] || null;
  return {
    url: location.href,
    title: document.title,
    viewport: {innerWidth, innerHeight, devicePixelRatio},
    scroll: scrolling ? {
      x: window.scrollX,
      y: window.scrollY,
      top: scrolling.scrollTop,
      left: scrolling.scrollLeft,
      height: scrolling.scrollHeight,
      width: scrolling.scrollWidth,
      clientHeight: scrolling.clientHeight,
      clientWidth: scrolling.clientWidth,
    } : null,
    activeElement: document.activeElement ? rectOf(document.activeElement) : null,
    controls,
    elements,
    pageIndicators,
    zoomIndicators,
    largestElement: largest,
    bodyTextSample: document.body ? document.body.innerText.slice(0, 1200) : "",
  };
})()`;

const ACTIVATE_SOURCE = `((kind) => {
  const aliases = {
    zoomIn: ["zoom in", "zoom-in", "zoomin", "increase zoom", "add"],
    zoomOut: ["zoom out", "zoom-out", "zoomout", "decrease zoom", "remove"],
    rotate: ["rotate", "clockwise"],
    fit: ["fit to", "fit page", "fit width", "fit", "display settings"],
    pageNext: ["next page", "next"],
    pagePrevious: ["previous page", "prev page", "previous", "back"],
    download: ["download", "save"],
    print: ["print"],
  };
  const haystack = (el) => [
    el.getAttribute("aria-label") || "",
    el.getAttribute("title") || "",
    el.id || "",
    String(el.className || ""),
    el.getAttribute("role") || "",
    el.innerText || "",
    el.value || "",
  ].join(" ").replace(/\\s+/g, " ").trim().toLowerCase();
  const rectOf = (el) => {
    const rect = el.getBoundingClientRect();
    return {
      tag: el.tagName,
      id: el.id || "",
      className: String(el.className || ""),
      role: el.getAttribute("role") || "",
      ariaLabel: el.getAttribute("aria-label") || "",
      title: el.getAttribute("title") || "",
      text: (el.innerText || el.value || "").slice(0, 120),
      disabled: !!el.disabled || el.getAttribute("aria-disabled") === "true",
      hidden: !!el.hidden || getComputedStyle(el).visibility === "hidden" || getComputedStyle(el).display === "none",
      x: rect.x,
      y: rect.y,
      width: rect.width,
      height: rect.height,
      token: haystack(el),
    };
  };
  const candidates = [];
  const visit = (root, depth = 0, scope = "document") => {
    if (!root || depth > 10 || candidates.length > 240) {
      return;
    }
    for (const el of root.querySelectorAll("button,input,select,cr-icon-button,*[role=button]")) {
      const info = {...rectOf(el), scope, depth};
      const token = info.token;
      const score = (aliases[kind] || []).reduce((value, alias) => value + (token.includes(alias) ? alias.length : 0), 0);
      candidates.push({info, score, element: el});
    }
    for (const el of root.querySelectorAll("*")) {
      if (el.shadowRoot) {
        visit(el.shadowRoot, depth + 1, \`\${scope} > \${el.tagName.toLowerCase()}#\${el.id || ""}\`);
      }
    }
  };
  visit(document);
  if (kind === "pageNext" || kind === "pagePrevious") {
    const input = candidates.find((candidate) => candidate.info.id === "pageSelector" || candidate.info.token.includes("pageselector"));
    if (input) {
      const current = Number(String(input.element.value || "1").replace(/\\D+/g, "")) || 1;
      const next = kind === "pageNext" ? current + 1 : Math.max(1, current - 1);
      input.element.focus?.();
      input.element.value = String(next);
      input.element.dispatchEvent(new Event("input", {bubbles: true, composed: true}));
      input.element.dispatchEvent(new Event("change", {bubbles: true, composed: true}));
      input.element.dispatchEvent(new KeyboardEvent("keydown", {key: "Enter", code: "Enter", bubbles: true, composed: true}));
      input.element.dispatchEvent(new KeyboardEvent("keyup", {key: "Enter", code: "Enter", bubbles: true, composed: true}));
      return {
        found: true,
        activated: true,
        method: "dom-page-selector",
        kind,
        selected: input.info,
        beforeValue: current,
        afterValue: next,
      };
    }
  }
  const matches = candidates
    .filter((candidate) => candidate.score > 0)
    .sort((a, b) => b.score - a.score || Number(a.info.disabled) - Number(b.info.disabled));
  const selected = matches.find((candidate) => !candidate.info.disabled && !candidate.info.hidden) || matches[0];
  if (!selected) {
    return {
      found: false,
      kind,
      candidates: candidates.map((candidate) => candidate.info).slice(0, 120),
    };
  }
  if (selected.info.disabled || selected.info.hidden) {
    return {
      found: true,
      activated: false,
      reason: selected.info.disabled ? "disabled" : "hidden",
      kind,
      selected: selected.info,
      candidates: matches.map((candidate) => candidate.info).slice(0, 20),
    };
  }
  return {
    found: true,
    activated: false,
    method: "candidate-found",
    kind,
    selected: selected.info,
    candidates: matches.map((candidate) => candidate.info).slice(0, 20),
  };
})`;

async function collectState(client, childSessions) {
  const top = await evaluate(client, STATE_SOURCE);
  const children = [];
  for (const child of childSessions) {
    children.push({
      sessionId: child.sessionId,
      targetInfo: child.targetInfo,
      state: await evaluate(client, STATE_SOURCE, child.sessionId),
    });
  }
  return { top, children };
}

function allStateValues(state) {
  const values = [];
  if (state.top?.ok && state.top.value) {
    values.push({ sessionId: null, targetInfo: null, value: state.top.value });
  }
  for (const child of state.children || []) {
    if (child.state?.ok && child.state.value) {
      values.push({
        sessionId: child.sessionId,
        targetInfo: child.targetInfo,
        value: child.state.value,
      });
    }
  }
  return values;
}

function semanticSnapshot(state) {
  return allStateValues(state).map(({ sessionId, targetInfo, value }) => ({
    sessionId,
    url: value.url,
    title: value.title,
    scroll: value.scroll,
    pageIndicators: value.pageIndicators,
    zoomIndicators: value.zoomIndicators,
    largestElement: value.largestElement,
    bodyTextSample: value.bodyTextSample,
    targetUrl: targetInfo?.url || "",
  }));
}

function flattenControls(state) {
  return allStateValues(state).flatMap(({ sessionId, targetInfo, value }) =>
    (value.controls || []).map((control) => ({
      sessionId,
      targetUrl: targetInfo?.url || value.url,
      ...control,
    })),
  );
}

function inventoryHasControl(inventory, kind) {
  const aliases =
    {
      download: ["download", "save"],
      print: ["print"],
    }[kind] || [];
  return inventory.some((control) => {
    const token = String(control.token || "").toLowerCase();
    return aliases.some((alias) => token.includes(alias));
  });
}

async function activateControl(client, childSessions, kind) {
  const expression = `${ACTIVATE_SOURCE}(${JSON.stringify(kind)})`;
  const attempts = [];
  const sessions = [
    { sessionId: undefined, targetInfo: { url: "top-level" } },
    ...childSessions,
  ];
  for (const session of sessions) {
    const result = await evaluate(client, expression, session.sessionId);
    attempts.push({
      sessionId: session.sessionId || null,
      targetInfo: session.targetInfo,
      result,
    });
    if (result.ok && result.value?.activated) {
      return {
        ok: true,
        sessionId: session.sessionId || null,
        targetInfo: session.targetInfo,
        activation: result.value,
        attempts,
      };
    }
    if (result.ok && result.value?.found && result.value?.selected) {
      const selected = result.value.selected;
      const x = Math.round(selected.x + selected.width / 2);
      const y = Math.round(selected.y + selected.height / 2);
      await client.send(
        "Input.dispatchMouseEvent",
        {
          type: "mouseMoved",
          x,
          y,
          button: "none",
          buttons: 0,
        },
        session.sessionId,
      );
      await client.send(
        "Input.dispatchMouseEvent",
        {
          type: "mousePressed",
          x,
          y,
          button: "left",
          buttons: 1,
          clickCount: 1,
        },
        session.sessionId,
      );
      await client.send(
        "Input.dispatchMouseEvent",
        {
          type: "mouseReleased",
          x,
          y,
          button: "left",
          buttons: 0,
          clickCount: 1,
        },
        session.sessionId,
      );
      return {
        ok: true,
        sessionId: session.sessionId || null,
        targetInfo: session.targetInfo,
        activation: {
          ...result.value,
          activated: true,
          method: "cdp-mouse",
          x,
          y,
          coordinateSpace: session.sessionId
            ? "child-target-viewport"
            : "top-level-viewport",
        },
        attempts,
      };
    }
  }
  return { ok: false, attempts };
}

async function snapshot(client, args, name, childSessions) {
  const state = await collectState(client, childSessions);
  const screenshot = await captureScreenshot(client, args, `${name}.png`);
  writeJson(path.join(args.outDir, `${name}.json`), {
    state,
    semantic: semanticSnapshot(state),
    screenshot,
  });
  return { state, screenshot };
}

function largestArea(state) {
  let largest = null;
  for (const { value } of allStateValues(state)) {
    const el = value.largestElement;
    if (!el) {
      continue;
    }
    const area = (el.width || 0) * (el.height || 0);
    if (!largest || area > largest.area) {
      largest = { ...el, area };
    }
  }
  return largest;
}

function textBlob(state, pattern) {
  const pieces = [];
  for (const { value } of allStateValues(state)) {
    pieces.push(value.title || "");
    pieces.push(value.bodyTextSample || "");
    for (const item of value.pageIndicators || []) {
      pieces.push(item.token || "");
      pieces.push(item.text || "");
    }
    for (const item of value.zoomIndicators || []) {
      pieces.push(item.token || "");
      pieces.push(item.text || "");
    }
  }
  return pieces.join(" ").match(pattern)?.[0] || "";
}

function stateChanged(kind, before, after) {
  const beforeLargest = largestArea(before);
  const afterLargest = largestArea(after);
  const beforeText = textBlob(before, /(?:\d+\s*%|\d+\s*\/\s*\d+|page\s+\d+)/i);
  const afterText = textBlob(after, /(?:\d+\s*%|\d+\s*\/\s*\d+|page\s+\d+)/i);
  const beforeSem = JSON.stringify(semanticSnapshot(before));
  const afterSem = JSON.stringify(semanticSnapshot(after));
  const areaDelta = (afterLargest?.area || 0) - (beforeLargest?.area || 0);
  const widthDelta = (afterLargest?.width || 0) - (beforeLargest?.width || 0);
  const heightDelta =
    (afterLargest?.height || 0) - (beforeLargest?.height || 0);

  if (["zoomIn", "zoomOut", "fit", "rotate"].includes(kind)) {
    return {
      changed:
        beforeText !== afterText ||
        Math.abs(areaDelta) > 1 ||
        Math.abs(widthDelta) > 1 ||
        Math.abs(heightDelta) > 1 ||
        beforeSem !== afterSem,
      beforeText,
      afterText,
      beforeLargest,
      afterLargest,
      areaDelta,
      widthDelta,
      heightDelta,
    };
  }

  if (["pageNext", "pagePrevious"].includes(kind)) {
    const beforeScroll = allStateValues(before).map(
      ({ value }) => value.scroll,
    );
    const afterScroll = allStateValues(after).map(({ value }) => value.scroll);
    return {
      changed:
        beforeText !== afterText ||
        JSON.stringify(beforeScroll) !== JSON.stringify(afterScroll) ||
        beforeSem !== afterSem,
      beforeText,
      afterText,
      beforeScroll,
      afterScroll,
    };
  }

  return { changed: beforeSem !== afterSem, beforeText, afterText };
}

async function runControl(client, args, childSessions, kind) {
  const before = await snapshot(client, args, `${kind}-before`, childSessions);
  const activation = await activateControl(client, childSessions, kind);
  await sleep(args.actionSettleMs);
  const after = await snapshot(client, args, `${kind}-after`, childSessions);
  const diff = stateChanged(kind, before.state, after.state);
  const status =
    activation.ok && diff.changed
      ? "pass"
      : activation.ok
        ? "partial"
        : "partial";
  return {
    feature: kind,
    status,
    controlFound: activation.attempts.some(
      (attempt) => attempt.result.ok && attempt.result.value?.found,
    ),
    clicked: activation.ok,
    stateChanged: diff.changed,
    activation,
    diff,
    screenshots: [
      before.screenshot.relativePath,
      after.screenshot.relativePath,
    ],
  };
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  fs.mkdirSync(args.outDir, { recursive: true });
  const summary = {
    devtoolsPort: args.devtoolsPort,
    urlContains: args.urlContains,
    outDir: args.outDir,
  };
  let client = null;
  try {
    const target = await pollTarget(args, summary);
    summary.selectedTarget = {
      id: target.id,
      type: target.type,
      url: target.url,
      title: target.title,
    };
    client = connectDevTools(target.webSocketDebuggerUrl);
    await client.open;
    for (const domain of [
      "Page",
      "Runtime",
      "DOM",
      "Input",
      "Target",
      "Browser",
    ]) {
      summary[`${domain}Enabled`] = await safeSend(client, `${domain}.enable`);
    }
    await safeSend(client, "Target.setAutoAttach", {
      autoAttach: true,
      waitForDebuggerOnStart: false,
      flatten: true,
    });
    await safeSend(client, "Page.bringToFront");
    await sleep(args.settleSeconds * 1000);

    summary.childTargets = client.events
      .filter((event) => event.method === "Target.attachedToTarget")
      .map((event) => ({
        sessionId: event.params.sessionId,
        targetInfo: event.params.targetInfo,
      }));
    for (const child of summary.childTargets) {
      await safeSend(client, "Runtime.enable", {}, child.sessionId);
      await safeSend(client, "Page.enable", {}, child.sessionId);
      await safeSend(client, "DOM.enable", {}, child.sessionId);
    }

    const baseline = await snapshot(
      client,
      args,
      "baseline",
      summary.childTargets,
    );
    summary.controlInventory = flattenControls(baseline.state);
    writeJson(
      path.join(args.outDir, "control-inventory.json"),
      summary.controlInventory,
    );

    summary.results = [];
    for (const kind of [
      "zoomIn",
      "zoomOut",
      "rotate",
      "fit",
      "pageNext",
      "pagePrevious",
    ]) {
      const result = await runControl(client, args, summary.childTargets, kind);
      summary.results.push(result);
      writeJson(path.join(args.outDir, `${kind}.json`), result);
    }

    summary.saveDownload = {
      status: "save-download-not-contained",
      controlFound: inventoryHasControl(summary.controlInventory, "download"),
      clicked: false,
      notes:
        "download was inventoried but not clicked because native dialog/download containment is reserved for a follow-up",
    };
    summary.print = {
      status: "print-not-contained",
      controlFound: inventoryHasControl(summary.controlInventory, "print"),
      clicked: false,
      notes:
        "print was inventoried but not clicked because native print dialog containment is not proven",
    };
    summary.titleObservation = semanticSnapshot(baseline.state).map((item) => ({
      sessionId: item.sessionId,
      url: item.url,
      title: item.title,
      targetUrl: item.targetUrl,
    }));
    summary.status = summary.results.every((result) => result.status === "pass")
      ? "pass"
      : "partial";
  } catch (error) {
    summary.status = "error";
    summary.error = String(error.stack || error);
    throw error;
  } finally {
    writeJson(path.join(args.outDir, "toolbar-summary.json"), summary);
    if (client) {
      client.socket.close();
    }
  }
}

main().catch((error) => {
  console.error(error.stack || error);
  process.exit(1);
});
