import { spawn } from "node:child_process";
import { watch } from "node:fs";
import { copyFile, mkdir, readdir, rename, unlink, writeFile } from "node:fs/promises";
import { basename, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const desktopRoot = resolve(import.meta.dirname, "../..");
const repositoryRoot = resolve(desktopRoot, "..");
const cargoWorkspace = join(repositoryRoot, "Cargo.toml");
const targetDirectory = join(repositoryRoot, "target");
const generationDirectory = join(desktopRoot, ".tmp", "dev-server-host");
const generationFile = join(generationDirectory, "current.json");
const skipInitial = process.argv.includes("--skip-initial");
const debounceMs = 250;

let activeBuild;
let buildRequested = !skipInitial;
let buildSequence = 0;
let debounce;
let stopped = false;

const watchers = [];

if (process.argv[1] && fileURLToPath(import.meta.url) === resolve(process.argv[1])) start();

export function shouldRebuildServerHost(file) {
  if (typeof file !== "string") return false;
  const name = basename(file);
  return file.endsWith(".rs") || name === "Cargo.toml" || name === "Cargo.lock" || name === "build.rs";
}

export function parseRustHostTriple(output) {
  const host = output.match(/^host:\s*(\S+)\s*$/mu)?.[1];
  if (!host) throw new Error("rustc did not report a host target triple");
  return host;
}

function start() {
  watchers.push(
    watch(join(repositoryRoot, "zeta-rs"), { recursive: true }, (_event, file) => requestBuild(file)),
    watch(repositoryRoot, (_event, file) => requestBuild(file)),
  );
  console.log("[server-host] Watching Rust App Server sources");
  if (buildRequested) void drainBuilds();
  process.once("SIGINT", stop);
  process.once("SIGTERM", stop);
}

function requestBuild(file) {
  if (stopped || !shouldRebuildServerHost(file)) return;
  clearTimeout(debounce);
  debounce = setTimeout(() => {
    buildRequested = true;
    void drainBuilds();
  }, debounceMs);
}

async function drainBuilds() {
  if (activeBuild || stopped) return;
  while (buildRequested && !stopped) {
    buildRequested = false;
    try {
      await buildAndPublish();
    } catch (error) {
      console.error(`[server-host] ${error instanceof Error ? error.message : String(error)}`);
    }
  }
}

async function buildAndPublish() {
  const host = parseRustHostTriple(await capture("rustc", ["-vV"]));
  console.log("[server-host] Building zeta-server-host");
  await runCargo(host);
  const executableName = process.platform === "win32" ? "zeta-server.exe" : "zeta-server";
  const source = join(targetDirectory, host, "debug", executableName);
  const generationIdentity = `${Date.now()}.${buildSequence++}`;
  const generationName = process.platform === "win32" ? `zeta-server.${generationIdentity}.exe` : `zeta-server.${generationIdentity}`;
  const staging = join(generationDirectory, `.${generationName}.tmp`);
  const executable = join(generationDirectory, generationName);
  await mkdir(generationDirectory, { recursive: true });
  await copyFile(source, staging);
  await rename(staging, executable);
  const nextGeneration = `${generationFile}.${process.pid}.tmp`;
  await writeFile(nextGeneration, `${JSON.stringify({ version: 1, executable: generationName })}\n`, "utf8");
  await replaceGenerationFile(nextGeneration, generationFile);
  await pruneGenerations(generationName);
  console.log(`[server-host] Published ${generationName}`);
}

function runCargo(host) {
  return new Promise((resolvePromise, reject) => {
    const child = spawn("cargo", [
      "build",
      "--manifest-path", cargoWorkspace,
      "--package", "zeta-server-host",
      "--bin", "zeta-server",
      "--profile", "dev",
      "--target", host,
      "--target-dir", targetDirectory,
    ], { cwd: repositoryRoot, env: process.env, stdio: "inherit", windowsHide: true });
    activeBuild = child;
    child.once("error", reject);
    child.once("exit", (code, signal) => {
      activeBuild = undefined;
      if (code === 0) resolvePromise();
      else reject(new Error(signal ? `cargo build stopped by ${signal}` : `cargo build exited with status ${code ?? "unknown"}`));
    });
  });
}

function capture(command, args) {
  return new Promise((resolvePromise, reject) => {
    const child = spawn(command, args, { cwd: repositoryRoot, env: process.env, stdio: ["ignore", "pipe", "inherit"], windowsHide: true });
    const chunks = [];
    child.stdout.on("data", chunk => chunks.push(Buffer.from(chunk)));
    child.once("error", reject);
    child.once("exit", code => {
      if (code === 0) resolvePromise(Buffer.concat(chunks).toString("utf8"));
      else reject(new Error(`${command} exited with status ${code ?? "unknown"}`));
    });
  });
}

async function pruneGenerations(current) {
  const entries = await readdir(generationDirectory);
  const generations = entries
    .filter(entry => entry.startsWith("zeta-server") && entry !== current)
    .sort()
    .reverse();
  await Promise.all(generations.slice(4).map(entry => unlink(join(generationDirectory, entry)).catch(() => {})));
}

async function replaceGenerationFile(staging, destination) {
  try {
    await rename(staging, destination);
  } catch (error) {
    if (process.platform !== "win32" || !isReplaceError(error)) throw error;
    await unlink(destination).catch(unlinkError => {
      if (!isMissingFileError(unlinkError)) throw unlinkError;
    });
    await rename(staging, destination);
  }
}

function isReplaceError(error) {
  return error instanceof Error && "code" in error && (error.code === "EEXIST" || error.code === "EPERM");
}

function isMissingFileError(error) {
  return error instanceof Error && "code" in error && error.code === "ENOENT";
}

function stop() {
  if (stopped) return;
  stopped = true;
  clearTimeout(debounce);
  for (const watcher of watchers) watcher.close();
  activeBuild?.kill("SIGTERM");
}
