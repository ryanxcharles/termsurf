#!/usr/bin/env node

import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";

function parseArgs(argv) {
  const args = { timeoutSeconds: 30, settleSeconds: 1, name: "snapshot" };
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
  if (args.actionX !== undefined) args.actionX = Number(args.actionX);
  if (args.actionY !== undefined) args.actionY = Number(args.actionY);
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
  const deepQueryAll = (root, selector, depth = 0) => {
    if (!root || depth > 12) return [];
    const out = [...(root.querySelectorAll?.(selector) || [])];
    for (const el of root.querySelectorAll?.("*") || []) {
      if (el.shadowRoot) out.push(...deepQueryAll(el.shadowRoot, selector, depth + 1));
    }
    return out;
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
    while (current && depth < 5) {
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
  const plugin = viewerRoot?.querySelector("#plugin") || document.querySelector("embed") || null;
  const pageEls = deepQueryAll(document, ".page, .pdfViewer .page, viewer-page, #viewerContainer");
  const controls = deepQueryAll(document, "input,textarea,button,*[role=button],embed,pdf-viewer")
    .slice(0, 200)
    .map((el) => ({
      tag: el.tagName,
      id: el.id || "",
      className: String(el.className || ""),
      role: el.getAttribute("role") || "",
      ariaLabel: el.getAttribute("aria-label") || "",
      title: el.getAttribute("title") || "",
      text: (el.innerText || el.value || "").slice(0, 160),
      disabled: !!el.disabled || el.getAttribute("aria-disabled") === "true",
      hidden: !!el.hidden || getComputedStyle(el).visibility === "hidden" || getComputedStyle(el).display === "none",
      rect: rect(el),
    }));
  return {
    url: location.href,
    title: document.title,
    activeElement: {
      tag: document.activeElement?.tagName || "",
      id: document.activeElement?.id || "",
      className: String(document.activeElement?.className || ""),
    },
    viewport: {innerWidth, innerHeight, devicePixelRatio},
    viewerPresent: !!viewer,
    pluginPresent: !!plugin,
    pluginRect: rect(plugin),
    toolbarRect: rect(toolbar),
    pageRects: pageEls.map((el) => ({
      tag: el.tagName,
      id: el.id || "",
      className: String(el.className || ""),
      rect: rect(el),
    })).filter((item) => item.rect),
    viewerProps: props(viewer),
    toolbarProps: props(toolbar),
    controllerProps: props(viewer?.currentController),
    viewportProps: props(viewer?.viewport_),
    controls,
  };
})()`;

async function collectStates(client) {
  const eventStart = client.events.length;
  await safeSend(client, "Target.setAutoAttach", {
    autoAttach: true,
    waitForDebuggerOnStart: false,
    flatten: true,
  });
  await sleep(500);
  const children = client.events
    .slice(eventStart)
    .filter((event) => event.method === "Target.attachedToTarget")
    .map((event) => ({
      sessionId: event.params.sessionId,
      targetInfo: event.params.targetInfo,
    }));
  for (const child of children) {
    await safeSend(client, "Runtime.enable", {}, child.sessionId);
    await safeSend(client, "Page.enable", {}, child.sessionId);
    await safeSend(client, "DOM.enable", {}, child.sessionId);
  }
  const states = [
    {
      sessionId: null,
      targetInfo: null,
      state: await evaluate(client, STATE_SOURCE),
    },
  ];
  for (const child of children) {
    states.push({
      sessionId: child.sessionId,
      targetInfo: child.targetInfo,
      state: await evaluate(client, STATE_SOURCE, child.sessionId),
    });
  }
  const values = states
    .filter((state) => state.state.ok && state.state.value)
    .map((state) => ({
      sessionId: state.sessionId,
      targetInfo: state.targetInfo,
      value: state.state.value,
    }));
  return { children, states, values };
}

function pluginLoaded(collected) {
  return (collected.values || []).some((item) => {
    const props = item.value.viewerProps || {};
    const rect = item.value.pluginRect || {};
    return (
      props.loadState_?.value === "success" &&
      props.docLength_?.value &&
      rect.width > 0 &&
      rect.height > 0
    );
  });
}

async function dispatchMouseClick(client, x, y) {
  const events = [
    { type: "mouseMoved", x, y, button: "none", buttons: 0 },
    { type: "mousePressed", x, y, button: "left", buttons: 1, clickCount: 1 },
    { type: "mouseReleased", x, y, button: "left", buttons: 0, clickCount: 1 },
  ];
  const results = [];
  for (const params of events) {
    results.push(await safeSend(client, "Input.dispatchMouseEvent", params));
    await sleep(40);
  }
  return results;
}

async function dispatchText(client, text) {
  const results = [];
  for (const ch of text) {
    const code = ch.length === 1 ? ch.toUpperCase().charCodeAt(0) : 0;
    const key = ch;
    results.push(
      await safeSend(client, "Input.dispatchKeyEvent", {
        type: "keyDown",
        key,
        text: ch,
        unmodifiedText: ch,
        windowsVirtualKeyCode: code,
        nativeVirtualKeyCode: code,
      }),
    );
    await sleep(30);
    results.push(
      await safeSend(client, "Input.dispatchKeyEvent", {
        type: "keyUp",
        key,
        windowsVirtualKeyCode: code,
        nativeVirtualKeyCode: code,
      }),
    );
    await sleep(30);
  }
  return results;
}

async function dispatchEscape(client) {
  return [
    await safeSend(client, "Input.dispatchKeyEvent", {
      type: "keyDown",
      key: "Escape",
      code: "Escape",
      windowsVirtualKeyCode: 27,
      nativeVirtualKeyCode: 27,
    }),
    await safeSend(client, "Input.dispatchKeyEvent", {
      type: "keyUp",
      key: "Escape",
      code: "Escape",
      windowsVirtualKeyCode: 27,
      nativeVirtualKeyCode: 27,
    }),
  ];
}

async function runAction(client, args, summary) {
  if (!args.action) return;
  if (args.action === "click") {
    if (!Number.isFinite(args.actionX) || !Number.isFinite(args.actionY)) {
      throw new Error("click action requires --action-x and --action-y");
    }
    summary.actionResult = await dispatchMouseClick(client, args.actionX, args.actionY);
  } else if (args.action === "text") {
    summary.actionResult = await dispatchText(client, args.actionText || "");
  } else if (args.action === "escape") {
    summary.actionResult = await dispatchEscape(client);
  } else {
    throw new Error(`unknown action: ${args.action}`);
  }
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  fs.mkdirSync(args.outDir, { recursive: true });
  const summary = {
    devtoolsPort: args.devtoolsPort,
    urlContains: args.urlContains,
    name: args.name,
  };
  let client = null;
  try {
    const target = await pollTarget(args, summary);
    summary.selectedTarget = { id: target.id, type: target.type, url: target.url, title: target.title };
    client = connectDevTools(target.webSocketDebuggerUrl);
    await client.open;
    for (const domain of ["Page", "Runtime", "DOM", "Target"]) {
      summary[`${domain}Enabled`] = await safeSend(client, `${domain}.enable`);
    }
    await safeSend(client, "Page.bringToFront");
    await runAction(client, args, summary);
    await sleep(args.settleSeconds * 1000);
    const collected = await collectStates(client);
    summary.children = collected.children;
    summary.states = collected.states;
    summary.values = collected.values;
    summary.pluginLoaded = pluginLoaded(collected);
    summary.screenshot = await captureScreenshot(client, args, `${args.name}.png`);
    summary.status = "pass";
  } catch (error) {
    summary.status = "error";
    summary.error = String(error.stack || error);
    throw error;
  } finally {
    writeJson(path.join(args.outDir, `${args.name}.json`), summary);
    client?.socket?.close();
  }
}

main().catch((error) => {
  console.error(error.stack || error);
  process.exit(1);
});
