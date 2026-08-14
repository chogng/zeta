import { spawn } from "node:child_process";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import process from "node:process";

const mode = process.argv[2];
const playwrightArguments = process.argv.slice(3);
if (mode !== "disconnected" && mode !== "full") {
  throw new Error("Usage: node scripts/run-smoke-browser-tests.mjs <disconnected|full>");
}

if (mode === "full") {
  const preparation = await run(process.execPath, ["scripts/prepare-dev-package.mjs", "--javascript-runtime", "packaged-node"], process.env);
  if (preparation !== 0) {
    process.exitCode = preparation;
    process.exit();
  }
}

const port = mode === "full" ? 5174 : 5173;
const serverUrl = `http://127.0.0.1:${port}/`;
const workspaceDirectory = mode === "full" ? await mkdtemp(join(tmpdir(), "zeta-playwright-browser-workspace-")) : undefined;
const profileDirectory = mode === "full" ? await mkdtemp(join(tmpdir(), "zeta-playwright-browser-profile-")) : undefined;
const productServicesPath = profileDirectory ? join(profileDirectory, "product-services.json") : undefined;
let languageServerExecutable = process.env.ZETA_PLAYWRIGHT_RUST_ANALYZER;
if (profileDirectory && !languageServerExecutable) {
  languageServerExecutable = join(profileDirectory, process.platform === "win32" ? "zeta-test-language-server.exe" : "zeta-test-language-server");
  const compilation = await run("rustc", ["--edition=2024", "test/fixtures/language-server.rs", "-o", languageServerExecutable], process.env);
  if (compilation !== 0) {
    process.exitCode = compilation;
    process.exit();
  }
}
if (profileDirectory && languageServerExecutable) {
  await writeFile(join(profileDirectory, "config.toml"), `[languageServers.servers.rust-analyzer]\nmode = "enabled"\nexecutable = ${JSON.stringify(languageServerExecutable)}\n`, "utf8");
}
if (productServicesPath) {
  await writeFile(productServicesPath, '{"schemaVersion":1,"marketplaces":[]}\n', "utf8");
}
const testEnvironment = workspaceDirectory ? { ...process.env, ZETA_PLAYWRIGHT_WORKSPACE: workspaceDirectory, ...(languageServerExecutable ? { ZETA_PLAYWRIGHT_LANGUAGE_SERVER: languageServerExecutable } : {}) } : process.env;
const serverEnvironment = mode === "full"
  ? { ...testEnvironment, ZETA_WEB_APP_SERVER: "1", ZETA_WORKSPACE_ROOT: workspaceDirectory, ...(profileDirectory ? { ZETA_WEB_APP_SERVER_PROFILE: profileDirectory } : {}), ...(productServicesPath ? { ZETA_PRODUCT_SERVICES_PATH: productServicesPath } : {}) }
  : testEnvironment;
const server = spawn(process.execPath, ["node_modules/vite/bin/vite.js", "--force"], {
  cwd: process.cwd(),
  env: serverEnvironment,
  stdio: "inherit",
});

let exitCode = 1;
try {
  await waitForServer(serverUrl, server);
  const project = mode === "full" ? "browser-app-server" : "browser-ui";
  exitCode = await run(process.execPath, ["node_modules/@playwright/test/cli.js", "test", `--project=${project}`, ...playwrightArguments], {
    ...process.env,
    ...testEnvironment,
    ZETA_PLAYWRIGHT_SERVER: mode,
    ZETA_SMOKE_BROWSER_EXTERNAL_SERVER: "1",
  });
} finally {
  await stop(server);
  if (workspaceDirectory) await rm(workspaceDirectory, { force: true, recursive: true });
  if (profileDirectory) await rm(profileDirectory, { force: true, recursive: true });
}

process.exitCode = exitCode;

async function waitForServer(url, child) {
  const deadline = Date.now() + 120_000;
  while (Date.now() < deadline) {
    if (child.exitCode !== null) throw new Error(`Smoke browser server exited with code ${child.exitCode}`);
    try {
      const response = await fetch(url, { signal: AbortSignal.timeout(1_000) });
      if (response.ok) return;
    } catch {}
    await new Promise(resolve => setTimeout(resolve, 100));
  }
  throw new Error(`Smoke browser server did not become ready at ${url}`);
}

function run(command, args, env) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, { cwd: process.cwd(), env, stdio: "inherit" });
    child.once("error", reject);
    child.once("exit", (code, signal) => {
      if (signal) reject(new Error(`Smoke browser command exited with signal ${signal}`));
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
