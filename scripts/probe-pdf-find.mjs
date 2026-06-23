#!/usr/bin/env node

import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";

function parseArgs(argv) {
  const args = { timeoutSeconds: 30, settleSeconds: 3 };
  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    if (!arg.startsWith("--")) throw new Error(`unexpected argument: ${arg}`);
    const [rawKey, inlineValue] = arg.slice(2).split("=", 2);
    const key = rawKey.replace(/-([a-z])/g, (_, ch) => ch.toUpperCase());
    const value = inlineValue ?? argv[++i];
    if (value === undefined) throw new Error(`missing value for ${arg}`);
    args[key] = value;
  }
  for (const key of ["devtoolsPort", "urlContains", "outDir"]) {
    if (!args[key]) throw new Error(`missing ${key}`);
  }
  args.devtoolsPort = Number(args.devtoolsPort);
  args.timeoutSeconds = Number(args.timeoutSeconds);
  args.settleSeconds = Number(args.settleSeconds);
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
  if (!response.ok) throw new Error(`GET ${url} failed: HTTP ${response.status}`);
  return await response.json();
}

async function pollTarget(args, summary) {
  const deadline = Date.now() + args.timeoutSeconds * 1000;
  const listUrl = `http://127.0.0.1:${args.devtoolsPort}/json/list`;
  let lastTargets = [];
  while (Date.now() < deadline) {
    lastTargets = await fetchJson(listUrl);
    const target = lastTargets.find(
      (item) =>
        item.type === "page" &&
        typeof item.url === "string" &&
        item.url.includes(args.urlContains) &&
        item.webSocketDebuggerUrl,
    );
    if (target) return target;
    await sleep(250);
  }
  summary.availableTargets = lastTargets.map((item) => ({
    id: item.id,
    type: item.type,
    url: item.url,
    title: item.title,
  }));
  throw new Error(`no page target contained ${JSON.stringify(args.urlContains)}`);
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
        reject(new Error(`${message.error.message || "DevTools error"} (${message.error.code})`));
      } else {
        resolve(message.result || {});
      }
      return;
    }
    if (message.method) events.push(message);
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
    if (sessionId) message.sessionId = sessionId;
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
  if (!result.ok) return { ok: false, error: result.error };
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
  const bytes = Buffer.from(result.data || "", "base64");
  const filePath = path.join(args.outDir, relativePath);
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, bytes);
  return {
    relativePath,
    bytes: bytes.length,
    sha256: crypto.createHash("sha256").update(bytes).digest("hex"),
  };
}

const STATE_SOURCE = `(() => {
  const deepQuery = (root, selector, depth = 0) => {
    if (!root || depth > 8) return null;
    const direct = root.querySelector?.(selector);
    if (direct) return direct;
    for (const el of root.querySelectorAll?.("*") || []) {
      if (el.shadowRoot) {
        const found = deepQuery(el.shadowRoot, selector, depth + 1);
        if (found) return found;
      }
    }
    return null;
  };
  const rect = (el) => {
    const box = el?.getBoundingClientRect?.();
    return box ? {x: box.x, y: box.y, width: box.width, height: box.height} : null;
  };
  const primitive = (value) => {
    if (value === null || ["string", "number", "boolean"].includes(typeof value)) return value;
    if (Array.isArray(value)) return {type: "array", length: value.length};
    return undefined;
  };
  const props = (obj) => {
    const out = {};
    if (!obj) return out;
    let current = obj;
    let depth = 0;
    const seen = new Set();
    while (current && depth < 4) {
      for (const name of Object.getOwnPropertyNames(current).sort()) {
        if (seen.has(name) || name === "constructor") continue;
        seen.add(name);
        try {
          const descriptor = Object.getOwnPropertyDescriptor(current, name);
          if (!descriptor) continue;
          const value = primitive(obj[name]);
          if (value !== undefined) out[name] = {depth, value, accessor: !!descriptor.get};
        } catch (error) {
          out[name] = {depth, error: String(error)};
        }
      }
      current = Object.getPrototypeOf(current);
      depth += 1;
    }
    return out;
  };
  const viewer = document.querySelector("pdf-viewer#viewer") || document.querySelector("pdf-viewer");
  const viewerRoot = viewer?.shadowRoot || null;
  const toolbar = viewerRoot?.querySelector("viewer-toolbar#toolbar") || viewerRoot?.querySelector("viewer-toolbar") || null;
  const pageSelector = deepQuery(toolbar?.shadowRoot, "#pageSelector");
  const findToolbar = deepQuery(viewerRoot, "viewer-pdf-sidenav, viewer-toolbar, cr-toolbar-search-field, viewer-find-toolbar, find-toolbar");
  const plugin = viewerRoot?.querySelector("#plugin") || document.querySelector("embed") || null;
  return {
    url: location.href,
    title: document.title,
    activeElementTag: document.activeElement?.tagName || "",
    activeElementId: document.activeElement?.id || "",
    bodyText: document.body?.innerText || "",
    pageSelectorValue: pageSelector?.value || "",
    pluginRect: rect(plugin),
    viewerRect: rect(viewer),
    findToolbarRect: rect(findToolbar),
    findToolbarText: findToolbar?.innerText || "",
    viewport: {innerWidth, innerHeight, devicePixelRatio},
    documentScroll: {
      scrollX,
      scrollY,
      bodyScrollTop: document.body?.scrollTop || 0,
      documentScrollTop: document.documentElement?.scrollTop || 0,
    },
    viewerProps: props(viewer),
    viewportProps: props(viewer?.viewport_),
    controllerProps: props(viewer?.currentController),
  };
})()`;

async function snapshot(client, args, sessionId, name) {
  const state = await evaluate(client, STATE_SOURCE, sessionId);
  const screenshot = await captureScreenshot(client, args, `${name}.png`);
  return { state, screenshot };
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  fs.mkdirSync(args.outDir, { recursive: true });
  const summary = { devtoolsPort: args.devtoolsPort, urlContains: args.urlContains };
  let client = null;
  try {
    const target = await pollTarget(args, summary);
    summary.selectedTarget = { id: target.id, type: target.type, url: target.url, title: target.title };
    client = connectDevTools(target.webSocketDebuggerUrl);
    await client.open;
    for (const domain of ["Page", "Runtime", "DOM", "Target"]) {
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
    const pdfChild = summary.childTargets.find((child) =>
      child.targetInfo?.url?.includes("chrome-extension://"),
    );
    if (!pdfChild) throw new Error("missing PDF extension child target");
    summary.pdfChild = pdfChild;
    await safeSend(client, "Runtime.enable", {}, pdfChild.sessionId);
    await safeSend(client, "Page.enable", {}, pdfChild.sessionId);
    await safeSend(client, "DOM.enable", {}, pdfChild.sessionId);
    await sleep(args.settleSeconds * 1000);
    summary.snapshot = await snapshot(client, args, pdfChild.sessionId, "snapshot");
    summary.status = summary.snapshot.state.ok ? "pass" : "fail";
    summary.firstFailingHop = summary.snapshot.state.ok
      ? "no-failure-observed"
      : "state-capture-failed";
  } catch (error) {
    summary.status = "error";
    summary.firstFailingHop = "devtools-probe-error";
    summary.error = String(error.stack || error);
    throw error;
  } finally {
    writeJson(path.join(args.outDir, "pdf-find-devtools-summary.json"), summary);
    client?.socket?.close();
  }
}

main().catch((error) => {
  console.error(error.stack || error);
  process.exit(1);
});
