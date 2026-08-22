import { spawn } from "node:child_process";
import { createHash } from "node:crypto";
import { constants, createReadStream, watch } from "node:fs";
import { copyFile, mkdir, readFile, readdir, rename, stat, unlink, writeFile } from "node:fs/promises";
import { basename, isAbsolute, join, relative, resolve, sep } from "node:path";
import { createInterface } from "node:readline";
import { fileURLToPath } from "node:url";

import { cargoArtifactExecutable, cargoRenderedDiagnostic, cargoTargetDirectory, parseCargoMessage } from "../../scripts/cargo-target.mjs";

const desktopRoot = resolve(import.meta.dirname, "../..");
const repositoryRoot = resolve(desktopRoot, "..");
const sharedRustSource = join(repositoryRoot, "zeta-rs");
const cargoWorkspace = join(repositoryRoot, "Cargo.toml");
const targetDirectory = cargoTargetDirectory(repositoryRoot);
const watchedTargetDirectory = relativeWatchedDirectory(sharedRustSource, targetDirectory);
const generationDirectory = join(desktopRoot, ".tmp", "dev-server-host");
const generationFile = join(generationDirectory, "current.json");
const skipInitial = process.argv.includes("--skip-initial");
const debounceMs = 250;
const retainedPreviousGenerations = 1;

let activeBuild;
let buildRequested = !skipInitial;
let debounce;
let stopped = false;

const watchers = [];

if (process.argv[1] && fileURLToPath(import.meta.url) === resolve(process.argv[1])) {
  void start().catch(error => {
    console.error(`[server-host] ${error instanceof Error ? error.message : String(error)}`);
    process.exitCode = 1;
  });
}

export function shouldRebuildServerHost(file, ignoredDirectory) {
  if (typeof file !== "string") return false;
  const normalized = file.replaceAll("\\", "/");
  if (/(?:^|\/)target(?:\/|$)/u.test(normalized)) return false;
  const ignored = ignoredDirectory?.replaceAll("\\", "/").replace(/\/+$/u, "");
  if (ignored !== undefined && (ignored === "" || normalized === ignored || normalized.startsWith(`${ignored}/`))) return false;
  const name = basename(file);
  return file.endsWith(".rs") || name === "Cargo.toml" || name === "Cargo.lock" || name === "build.rs";
}

export function relativeWatchedDirectory(watchRoot, directory) {
  const candidate = relative(resolve(watchRoot), resolve(directory));
  if (isAbsolute(candidate) || candidate === ".." || candidate.startsWith(`..${sep}`)) return undefined;
  return candidate.replaceAll("\\", "/");
}

export function shouldRebuildWorkspaceManifest(file) {
  return file === "Cargo.toml" || file === "Cargo.lock";
}

async function start() {
  await prunePublishedGenerations();
  watchers.push(
    watch(sharedRustSource, { recursive: true }, (_event, file) => requestBuild(file, fileName => shouldRebuildServerHost(fileName, watchedTargetDirectory))),
    watch(repositoryRoot, (_event, file) => requestBuild(file, shouldRebuildWorkspaceManifest)),
  );
  console.log("[server-host] Watching Rust App Server sources");
  if (buildRequested) void drainBuilds();
  process.once("SIGINT", stop);
  process.once("SIGTERM", stop);
}

function requestBuild(file, shouldRebuild) {
  if (stopped || !shouldRebuild(file)) return;
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
  console.log("[server-host] Building zeta-server-host");
  const source = await runCargo();
  const published = await publishServerHostGeneration(source, generationDirectory, generationFile, process.platform);
  console.log(published.changed ? `[server-host] Published ${published.generation}` : `[server-host] Unchanged ${published.generation}`);
}

function runCargo() {
  return new Promise((resolvePromise, reject) => {
    let executable;
    let settled = false;
    const child = spawn("cargo", [
      "build",
      "--manifest-path", cargoWorkspace,
      "--package", "zeta-server-host",
      "--bin", "zeta-server",
      "--profile", "dev-small",
      "--target-dir", targetDirectory,
      "--message-format", "json-render-diagnostics",
    ], { cwd: repositoryRoot, env: process.env, stdio: ["inherit", "pipe", "inherit"], windowsHide: true });
    activeBuild = child;
    const messages = createInterface({ input: child.stdout });
    messages.on("line", line => {
      const message = parseCargoMessage(line);
      const diagnostic = cargoRenderedDiagnostic(message);
      if (diagnostic) process.stderr.write(diagnostic);
      executable = cargoArtifactExecutable(message, "zeta-server") ?? executable;
    });
    child.once("error", error => {
      if (settled) return;
      settled = true;
      activeBuild = undefined;
      reject(error);
    });
    child.once("close", (code, signal) => {
      activeBuild = undefined;
      if (settled) return;
      settled = true;
      if (code !== 0) {
        reject(new Error(signal ? `cargo build stopped by ${signal}` : `cargo build exited with status ${code ?? "unknown"}`));
      } else if (!executable) {
        reject(new Error("cargo build did not report the zeta-server executable"));
      } else {
        resolvePromise(executable);
      }
    });
  });
}

export async function publishServerHostGeneration(source, directory, pointer, platform = process.platform) {
  const digest = await sha256(source);
  const generation = platform === "win32" ? `zeta-server.${digest}.exe` : `zeta-server.${digest}`;
  const executable = join(directory, generation);
  await mkdir(directory, { recursive: true });
  const current = await readCurrentGeneration(pointer);
  if (current === generation && await pathExists(executable)) {
    await pruneGenerations(directory, generation);
    return { changed: false, generation };
  }
  if (!await pathExists(executable)) {
    const staging = join(directory, `.${generation}.${process.pid}.tmp`);
    try {
      await copyFile(source, staging, constants.COPYFILE_FICLONE);
      await rename(staging, executable);
    } finally {
      await unlink(staging).catch(() => {});
    }
  }
  const nextGeneration = `${pointer}.${process.pid}.tmp`;
  try {
    await writeFile(nextGeneration, `${JSON.stringify({ version: 1, executable: generation })}\n`, "utf8");
    await replaceGenerationFile(nextGeneration, pointer);
  } finally {
    await unlink(nextGeneration).catch(() => {});
  }
  await pruneGenerations(directory, generation);
  return { changed: true, generation };
}

async function prunePublishedGenerations() {
  const current = await readCurrentGeneration(generationFile);
  if (!current) return;
  await pruneGenerations(generationDirectory, current);
}

async function pruneGenerations(directory, current) {
  if (!await pathExists(join(directory, current))) return;
  const entries = await readdir(directory, { withFileTypes: true });
  const generations = await Promise.all(entries
    .filter(entry => entry.isFile() && entry.name.startsWith("zeta-server.") && entry.name !== current)
    .map(async entry => ({ name: entry.name, modified: (await stat(join(directory, entry.name))).mtimeMs })));
  generations.sort((left, right) => right.modified - left.modified || right.name.localeCompare(left.name));
  const retainedDigests = new Set([await generationDigest(directory, current)]);
  let retained = 0;
  for (const generation of generations) {
    const digest = await generationDigest(directory, generation.name);
    if (retainedDigests.has(digest) || retained >= retainedPreviousGenerations) {
      await unlink(join(directory, generation.name)).catch(() => {});
      continue;
    }
    retainedDigests.add(digest);
    retained += 1;
  }
}

function generationDigest(directory, generation) {
  const addressed = generation.match(/^zeta-server\.([a-f0-9]{64})(?:\.exe)?$/u)?.[1];
  return addressed ?? sha256(join(directory, generation));
}

async function readCurrentGeneration(pointer) {
  try {
    const value = JSON.parse(await readFile(pointer, "utf8"));
    return value?.version === 1 && typeof value.executable === "string" && value.executable === basename(value.executable)
      ? value.executable
      : undefined;
  } catch (error) {
    if (error?.code === "ENOENT") return undefined;
    return undefined;
  }
}

async function pathExists(path) {
  try {
    return (await stat(path)).isFile();
  } catch (error) {
    if (error?.code === "ENOENT") return false;
    throw error;
  }
}

function sha256(path) {
  return new Promise((resolvePromise, reject) => {
    const hash = createHash("sha256");
    const stream = createReadStream(path);
    stream.on("data", chunk => hash.update(chunk));
    stream.once("error", reject);
    stream.once("end", () => resolvePromise(hash.digest("hex")));
  });
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
