#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";

function parseArgs(argv) {
  const args = { holdSeconds: 0, timeoutSeconds: 10 };

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

  for (const key of ["devtoolsPort", "urlContains", "out"]) {
    if (!args[key]) {
      throw new Error(
        `missing required --${key.replace(/[A-Z]/g, (ch) => `-${ch.toLowerCase()}`)}`,
      );
    }
  }

  args.devtoolsPort = Number(args.devtoolsPort);
  args.holdSeconds = Number(args.holdSeconds);
  args.timeoutSeconds = Number(args.timeoutSeconds);

  if (!Number.isFinite(args.devtoolsPort) || args.devtoolsPort <= 0) {
    throw new Error(`invalid --devtools-port: ${args.devtoolsPort}`);
  }
  if (!Number.isFinite(args.timeoutSeconds) || args.timeoutSeconds <= 0) {
    throw new Error(`invalid --timeout-seconds: ${args.timeoutSeconds}`);
  }
  if (!Number.isFinite(args.holdSeconds) || args.holdSeconds < 0) {
    throw new Error(`invalid --hold-seconds: ${args.holdSeconds}`);
  }

  return args;
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function fetchJson(url) {
  const response = await fetch(url);
  if (!response.ok) {
    throw new Error(`GET ${url} failed: HTTP ${response.status}`);
  }
  return await response.json();
}

function websocketEndpointPath(wsUrl) {
  const parsed = new URL(wsUrl);
  return parsed.pathname;
}

async function pollTarget(args, artifact) {
  const deadline = Date.now() + args.timeoutSeconds * 1000;
  const listUrl = `http://127.0.0.1:${args.devtoolsPort}/json/list`;
  let lastError = null;
  let lastTargets = [];

  while (Date.now() < deadline) {
    try {
      lastTargets = await fetchJson(listUrl);
      artifact.availableTargets = lastTargets.map((target) => ({
        id: target.id,
        type: target.type,
        url: target.url,
        title: target.title,
      }));
      const matches = lastTargets.filter(
        (target) =>
          target.type === "page" &&
          typeof target.url === "string" &&
          target.url.includes(args.urlContains) &&
          target.url.includes("/probe/webauthn-create.html") &&
          target.webSocketDebuggerUrl,
      );
      if (matches.length > 0) {
        const target = matches[0];
        return {
          id: target.id,
          type: target.type,
          url: target.url,
          title: target.title,
          websocketEndpointPath: websocketEndpointPath(
            target.webSocketDebuggerUrl,
          ),
          webSocketDebuggerUrl: target.webSocketDebuggerUrl,
        };
      }
    } catch (error) {
      lastError = error;
    }
    await sleep(200);
  }

  if (lastError) {
    artifact.lastPollError = String(lastError.stack || lastError);
  }
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
    socket.addEventListener(
      "error",
      () => reject(new Error("DevTools websocket error")),
      { once: true },
    );
  });

  return {
    async open() {
      await open;
    },
    command(method, params = {}) {
      const id = nextId;
      nextId += 1;
      const payload = JSON.stringify({ id, method, params });
      const result = new Promise((resolve, reject) => {
        pending.set(id, { resolve, reject });
      });
      socket.send(payload);
      return result;
    },
    close() {
      socket.close();
    },
    events,
  };
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  const artifact = {
    devtoolsPort: args.devtoolsPort,
    urlContains: args.urlContains,
    startedAt: new Date().toISOString(),
  };

  fs.mkdirSync(path.dirname(args.out), { recursive: true });

  let client = null;
  try {
    const target = await pollTarget(args, artifact);
    artifact.target = {
      id: target.id,
      type: target.type,
      url: target.url,
      title: target.title,
      websocketEndpointPath: target.websocketEndpointPath,
    };

    client = connectDevTools(target.webSocketDebuggerUrl);
    await client.open();
    await client.command("WebAuthn.enable");
    artifact.webAuthnEnable = { ok: true };
    const addResult = await client.command("WebAuthn.addVirtualAuthenticator", {
      options: {
        protocol: "ctap2",
        transport: "usb",
        hasResidentKey: true,
        hasUserVerification: true,
        isUserVerified: true,
        automaticPresenceSimulation: true,
      },
    });
    artifact.authenticatorId = addResult.authenticatorId || null;
    artifact.status = artifact.authenticatorId ? "completed" : "failed";
    if (!artifact.authenticatorId) {
      artifact.error = "WebAuthn.addVirtualAuthenticator returned no authenticatorId";
    }
    fs.writeFileSync(args.out, JSON.stringify(artifact, null, 2) + "\n");
    if (artifact.authenticatorId && args.holdSeconds > 0) {
      artifact.holdSeconds = args.holdSeconds;
      await sleep(args.holdSeconds * 1000);
    }
  } catch (error) {
    artifact.status = "failed";
    artifact.error = String(error.stack || error);
  } finally {
    if (client) {
      artifact.events = client.events;
      client.close();
    }
    artifact.finishedAt = new Date().toISOString();
    fs.writeFileSync(args.out, JSON.stringify(artifact, null, 2) + "\n");
  }

  if (artifact.status !== "completed") {
    process.exitCode = 1;
  }
  console.log(JSON.stringify(artifact, null, 2));
}

await main();
