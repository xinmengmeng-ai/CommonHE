import { spawn } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { pathToFileURL } from "node:url";

const htmlPath = process.argv[2];
if (!htmlPath) {
  throw new Error("Usage: node tests/desktop-layout-cdp.mjs <html-file>");
}

const browserCandidates = [
  process.env.CHROME_PATH,
  "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe",
  "C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe",
].filter(Boolean);

const browserPath = browserCandidates.find((candidate) => fs.existsSync(candidate));
if (!browserPath) {
  throw new Error("Chrome or Edge is required for desktop lifecycle layout gate.");
}

const profileDir = fs.mkdtempSync(path.join(os.tmpdir(), "commonhe-layout-cdp-"));
const htmlUrl = pathToFileURL(path.resolve(htmlPath)).href;
const browser = spawn(
  browserPath,
  [
    "--headless=new",
    "--window-size=1366,768",
    "--disable-gpu",
    "--disable-extensions",
    "--disable-background-networking",
    "--no-first-run",
    "--no-default-browser-check",
    "--remote-allow-origins=*",
    `--user-data-dir=${profileDir}`,
    "--remote-debugging-port=0",
    htmlUrl,
  ],
  { stdio: ["ignore", "ignore", "pipe"] },
);
const keepAlive = setInterval(() => {}, 1000);

let stderr = "";
browser.stderr.on("data", (chunk) => {
  stderr += chunk.toString();
});

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function waitForDevToolsPort() {
  const portFile = path.join(profileDir, "DevToolsActivePort");
  for (let index = 0; index < 100; index += 1) {
    if (fs.existsSync(portFile)) {
      const [port] = fs.readFileSync(portFile, "utf8").trim().split(/\r?\n/);
      if (port) {
        return Number(port);
      }
    }
    if (browser.exitCode !== null) {
      throw new Error(`Browser exited before DevTools port was available. stderr=${stderr}`);
    }
    await sleep(100);
  }
  throw new Error(`Timed out waiting for DevToolsActivePort. stderr=${stderr}`);
}

async function waitForPage(port) {
  for (let index = 0; index < 100; index += 1) {
    try {
      const response = await fetch(`http://127.0.0.1:${port}/json`);
      const pages = await response.json();
      const page = pages.find((item) => item.type === "page" && item.url === htmlUrl);
      if (page?.webSocketDebuggerUrl) {
        return page;
      }
    } catch (error) {
      if (browser.exitCode !== null) {
        throw new Error(`Browser exited while waiting for page target. stderr=${stderr}`);
      }
    }
    await sleep(100);
  }
  throw new Error(`Timed out waiting for page target: ${htmlUrl}`);
}

async function stopBrowser() {
  if (browser.exitCode !== null || browser.signalCode !== null) {
    return;
  }
  browser.kill();
  await Promise.race([
    new Promise((resolve) => browser.once("exit", resolve)),
    sleep(1000),
  ]);
}

function removeProfileDirBestEffort() {
  try {
    fs.rmSync(profileDir, {
      recursive: true,
      force: true,
      maxRetries: 5,
      retryDelay: 200,
    });
  } catch (error) {
    console.warn(`Warning: failed to remove temporary Chrome profile: ${error.message}`);
  }
}

function openCdpSocket(webSocketDebuggerUrl) {
  return new Promise((resolve, reject) => {
    const socket = new WebSocket(webSocketDebuggerUrl);
    const pending = new Map();
    let sequence = 0;

    socket.addEventListener("open", () => {
      resolve({
        send(method, params = {}) {
          sequence += 1;
          const id = sequence;
          socket.send(JSON.stringify({ id, method, params }));
          return new Promise((innerResolve, innerReject) => {
            pending.set(id, { resolve: innerResolve, reject: innerReject });
          });
        },
        close() {
          socket.close();
        },
      });
    });

    socket.addEventListener("message", (event) => {
      const message = JSON.parse(event.data);
      if (!message.id || !pending.has(message.id)) {
        return;
      }
      const callbacks = pending.get(message.id);
      pending.delete(message.id);
      if (message.error) {
        callbacks.reject(new Error(JSON.stringify(message.error)));
      } else {
        callbacks.resolve(message.result);
      }
    });

    socket.addEventListener("error", (event) => {
      reject(new Error(`CDP websocket error: ${event.message ?? "unknown"}`));
    });

    socket.addEventListener("close", () => {
      for (const callbacks of pending.values()) {
        callbacks.reject(new Error("CDP websocket closed before a response was received."));
      }
      pending.clear();
    });
  });
}

try {
  const port = await waitForDevToolsPort();
  const page = await waitForPage(port);
  const cdp = await openCdpSocket(page.webSocketDebuggerUrl);
  await cdp.send("Runtime.enable");

  let result;
  for (let index = 0; index < 100; index += 1) {
    const evaluation = await cdp.send("Runtime.evaluate", {
      expression: `(() => {
        const node = document.getElementById("layout-result");
        if (!node || !node.textContent.trim()) return null;
        return JSON.parse(node.textContent);
      })()`,
      returnByValue: true,
      awaitPromise: true,
    });
    result = evaluation.result?.value;
    if (result) {
      break;
    }
    await sleep(100);
  }

  cdp.close();
  if (!result) {
    throw new Error("Page did not emit layout-result.");
  }

  for (const [name, passed] of Object.entries(result.assertions ?? {})) {
    if (!passed) {
      throw new Error(`Layout assertion failed: ${name}; result=${JSON.stringify(result)}`);
    }
  }

  console.log(JSON.stringify(result));
} finally {
  clearInterval(keepAlive);
  await stopBrowser();
  removeProfileDirBestEffort();
}
