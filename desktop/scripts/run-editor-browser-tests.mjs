import { spawn } from "node:child_process";
import process from "node:process";

const serverUrl = "http://127.0.0.1:5185/textModel.html";
const server = spawn(process.execPath, ["node_modules/vite/bin/vite.js", "--config", "test/editor/browser/vite.config.ts"], {
  cwd: process.cwd(),
  stdio: "inherit",
});

let exitCode = 1;
try {
  await waitForServer(serverUrl, server);
  exitCode = await run(process.execPath, ["node_modules/@playwright/test/cli.js", "test", "--config", "test/editor/browser/playwright.config.ts"], {
    ...process.env,
    ZETA_EDITOR_BROWSER_EXTERNAL_SERVER: "1",
  });
} finally {
  await stop(server);
}

process.exitCode = exitCode;

async function waitForServer(url, child) {
  const deadline = Date.now() + 120_000;
  while (Date.now() < deadline) {
    if (child.exitCode !== null) throw new Error(`Editor browser server exited with code ${child.exitCode}`);
    try {
      const response = await fetch(url, { signal: AbortSignal.timeout(1_000) });
      if (response.ok) return;
    } catch {}
    await new Promise(resolve => setTimeout(resolve, 100));
  }
  throw new Error(`Editor browser server did not become ready at ${url}`);
}

function run(command, args, env) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, { cwd: process.cwd(), env, stdio: "inherit" });
    child.once("error", reject);
    child.once("exit", (code, signal) => {
      if (signal) reject(new Error(`Editor browser tests exited with signal ${signal}`));
      else resolve(code ?? 1);
    });
  });
}

async function stop(child) {
  if (child.exitCode !== null) return;
  child.kill("SIGTERM");
  await Promise.race([
    new Promise(resolve => child.once("exit", resolve)),
    new Promise(resolve => setTimeout(resolve, 5_000)),
  ]);
  if (child.exitCode === null) child.kill("SIGKILL");
}
