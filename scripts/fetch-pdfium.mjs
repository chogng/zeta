import { createHash } from "node:crypto";
import { createWriteStream } from "node:fs";
import { access, mkdir, readFile, rename, rm, stat, writeFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { pipeline } from "node:stream/promises";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";

const scriptDirectory = dirname(fileURLToPath(import.meta.url));
const workspaceRoot = resolve(scriptDirectory, "..");
const lockfilePath = resolve(workspaceRoot, "third_party/pdfium/runtime-lock.json");
const defaultCacheDirectory = resolve(workspaceRoot, "third_party/.cache/pdfium");

function usage() {
  return [
    "Usage: node scripts/fetch-pdfium.mjs [--target <target>] --output <directory>",
    "",
    "Downloads one pinned PDFium archive, validates its SHA-256, and extracts it.",
  ].join("\n");
}

function hostTarget() {
  const targets = {
    "darwin-arm64": "darwin-arm64",
    "darwin-x64": "darwin-x64",
    "linux-x64": "linux-x64",
    "win32-x64": "win-x64",
  };
  const target = targets[`${process.platform}-${process.arch}`];
  if (!target) {
    throw new Error(`No PDFium artifact is locked for ${process.platform}-${process.arch}; pass --target explicitly.`);
  }
  return target;
}

function parseArguments(argumentsList) {
  let target;
  let output;
  for (let index = 0; index < argumentsList.length; index += 1) {
    const argument = argumentsList[index];
    if (argument === "--target") target = argumentsList[++index];
    else if (argument === "--output") output = argumentsList[++index];
    else if (argument === "--help") return { help: true };
    else throw new Error(`Unknown argument: ${argument}`);
  }
  if (!output) throw new Error("--output is required.");
  return { target: target ?? hostTarget(), output: resolve(output) };
}

async function sha256(path) {
  const hash = createHash("sha256");
  const contents = await readFile(path);
  hash.update(contents);
  return hash.digest("hex");
}

async function archiveIsValid(path, expectedDigest) {
  try {
    return (await sha256(path)) === expectedDigest;
  } catch {
    return false;
  }
}

async function download(url, destination) {
  const response = await fetch(url);
  if (!response.ok || !response.body) throw new Error(`Failed to download ${url}: HTTP ${response.status}`);
  await mkdir(dirname(destination), { recursive: true });
  await pipeline(response.body, createWriteStream(destination));
}

async function outputExists(path) {
  try {
    await stat(path);
    return true;
  } catch {
    return false;
  }
}

async function verifyExistingOutput(output, artifact, version) {
  try {
    await access(resolve(output, artifact.library));
    const receipt = JSON.parse(await readFile(resolve(output, ".zeta-pdfium-receipt.json"), "utf8"));
    return receipt.version === version && receipt.archive === artifact.archive && receipt.sha256 === artifact.sha256;
  } catch {
    return false;
  }
}

async function main() {
  const argumentsResult = parseArguments(process.argv.slice(2));
  if (argumentsResult.help) {
    console.log(usage());
    return;
  }
  const lock = JSON.parse(await readFile(lockfilePath, "utf8"));
  const artifact = lock.artifacts[argumentsResult.target];
  if (!artifact) throw new Error(`Unknown PDFium target: ${argumentsResult.target}`);
  if (await verifyExistingOutput(argumentsResult.output, artifact, lock.version)) {
    console.log(`PDFium ${lock.version} already verified at ${argumentsResult.output}`);
    return;
  }
  if (await outputExists(argumentsResult.output)) {
    throw new Error(`Refusing to replace existing output directory: ${argumentsResult.output}`);
  }

  const archive = resolve(defaultCacheDirectory, lock.version, argumentsResult.target, artifact.archive);
  if (!(await archiveIsValid(archive, artifact.sha256))) {
    const temporaryArchive = `${archive}.partial`;
    await rm(temporaryArchive, { force: true });
    await download(`${lock.source.repository}/releases/download/${lock.source.release}/${artifact.archive}`, temporaryArchive);
    const actualDigest = await sha256(temporaryArchive);
    if (actualDigest !== artifact.sha256) {
      await rm(temporaryArchive, { force: true });
      throw new Error(`SHA-256 mismatch for ${artifact.archive}: expected ${artifact.sha256}, got ${actualDigest}`);
    }
    await mkdir(dirname(archive), { recursive: true });
    await rename(temporaryArchive, archive);
  }

  const stagingDirectory = `${argumentsResult.output}.partial`;
  await rm(stagingDirectory, { recursive: true, force: true });
  await mkdir(stagingDirectory, { recursive: true });
  const extraction = spawnSync("tar", ["-xzf", archive, "-C", stagingDirectory], { encoding: "utf8" });
  if (extraction.status !== 0) {
    await rm(stagingDirectory, { recursive: true, force: true });
    throw new Error(`Could not extract ${artifact.archive}: ${extraction.stderr || extraction.error?.message || "tar failed"}`);
  }
  try {
    await access(resolve(stagingDirectory, artifact.library));
  } catch {
    await rm(stagingDirectory, { recursive: true, force: true });
    throw new Error(`Archive ${artifact.archive} does not contain ${artifact.library}`);
  }
  await writeFile(resolve(stagingDirectory, ".zeta-pdfium-receipt.json"), `${JSON.stringify({
    version: lock.version,
    target: argumentsResult.target,
    archive: artifact.archive,
    sha256: artifact.sha256,
  }, null, 2)}\n`);
  await mkdir(dirname(argumentsResult.output), { recursive: true });
  await rename(stagingDirectory, argumentsResult.output);
  console.log(`Verified and extracted PDFium ${lock.version} for ${argumentsResult.target} to ${argumentsResult.output}`);
}

main().catch((error) => {
  console.error(error instanceof Error ? error.message : error);
  process.exitCode = 1;
});
