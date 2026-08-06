import { spawnSync } from "node:child_process";
import { createHash, randomUUID } from "node:crypto";
import { chmod, copyFile, lstat, mkdir, readFile, readdir, rename, rm, stat, writeFile } from "node:fs/promises";
import { basename, dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const repositoryRoot = resolve(import.meta.dirname, "..", "..");
const cargoWorkspace = repositoryRoot;
const sharedRustSource = join(repositoryRoot, "zeta-rs");
const outputDirectory = join(repositoryRoot, "desktop", ".tmp", "zeta-package");
const ripgrepLockPath = join(repositoryRoot, "third_party", "ripgrep", "runtime-lock.json");
const ripgrepCacheRoot = join(repositoryRoot, "third_party", ".cache", "ripgrep");
const bubblewrapLockPath = join(repositoryRoot, "third_party", "bubblewrap", "runtime-lock.json");
const bubblewrapCacheRoot = join(repositoryRoot, "third_party", ".cache", "bubblewrap");
const archiveBufferLimit = 64 * 1024 * 1024;

export function hostTarget(platform = process.platform, architecture = process.arch) {
  const targets = {
    "darwin-arm64": "aarch64-apple-darwin",
    "darwin-x64": "x86_64-apple-darwin",
    "linux-arm64": "aarch64-unknown-linux-gnu",
    "linux-x64": "x86_64-unknown-linux-gnu",
    "win32-arm64": "aarch64-pc-windows-msvc",
    "win32-x64": "x86_64-pc-windows-msvc",
  };
  const target = targets[`${platform}-${architecture}`];
  if (!target) {
    throw new Error(`Unsupported Zeta development host: ${platform}/${architecture}`);
  }
  return target;
}

export function selectRipgrepArtifact(lock, target) {
  if (lock.schemaVersion !== 1 || lock.runtime !== "ripgrep") {
    throw new Error("Unsupported ripgrep runtime lock");
  }
  const artifactKey = lock.packageTargets?.[target];
  const artifact = artifactKey ? lock.artifacts?.[artifactKey] : undefined;
  if (!artifactKey || !artifact) {
    throw new Error(`No locked ripgrep artifact for ${target}`);
  }
  for (const field of ["archive", "sha256", "format", "executable"]) {
    if (typeof artifact[field] !== "string" || artifact[field].length === 0) {
      throw new Error(`Invalid ripgrep artifact field ${field} for ${target}`);
    }
  }
  if (!Number.isSafeInteger(artifact.size) || artifact.size <= 0) {
    throw new Error(`Invalid ripgrep artifact size for ${target}`);
  }
  const repository = lock.source?.repository;
  const release = lock.source?.release;
  if (typeof repository !== "string" || typeof release !== "string") {
    throw new Error("Ripgrep lock is missing its upstream release");
  }
  return {
    ...artifact,
    key: artifactKey,
    url: artifact.url ?? `${repository.replace(/\/+$/, "")}/releases/download/${release}/${artifact.archive}`,
    version: lock.version,
  };
}

async function sha256(path) {
  return createHash("sha256").update(await readFile(path)).digest("hex");
}

async function verifyArchive(path, artifact) {
  try {
    const metadata = await stat(path);
    return metadata.isFile() && metadata.size === artifact.size && await sha256(path) === artifact.sha256;
  } catch {
    return false;
  }
}

async function downloadArchive(artifact, destination) {
  await mkdir(dirname(destination), { recursive: true });
  const partial = `${destination}.partial`;
  await rm(partial, { force: true });
  const response = await fetch(artifact.url, {
    headers: { "user-agent": "zeta-development-package-builder" },
    signal: AbortSignal.timeout(60_000),
  });
  if (!response.ok) {
    throw new Error(`Could not download ${artifact.url}: HTTP ${response.status}`);
  }
  await writeFile(partial, Buffer.from(await response.arrayBuffer()), { flag: "wx" });
  if (!await verifyArchive(partial, artifact)) {
    await rm(partial, { force: true });
    throw new Error(`Downloaded archive failed locked size or SHA-256 validation: ${artifact.archive}`);
  }
  await rm(destination, { force: true });
  await rename(partial, destination);
}

async function materializeArchive(artifact, cacheDirectory) {
  const archive = join(cacheDirectory, artifact.archive);
  if (!await verifyArchive(archive, artifact)) {
    await rm(archive, { force: true });
    await downloadArchive(artifact, archive);
  }
  return archive;
}

function extractArchiveMember(archive, member) {
  const result = spawnSync("tar", ["-xOf", archive, member], {
    encoding: null,
    maxBuffer: archiveBufferLimit,
    windowsHide: true,
  });
  if (result.error) {
    throw new Error(`Could not run the host tar utility: ${result.error.message}`);
  }
  if (result.status !== 0) {
    throw new Error(`Could not extract ${member}: ${result.stderr?.toString().trim()}`);
  }
  return result.stdout;
}

async function resolveRipgrep(target, isWindows) {
  const lock = JSON.parse(await readFile(ripgrepLockPath, "utf8"));
  const artifact = selectRipgrepArtifact(lock, target);
  const cacheDirectory = join(ripgrepCacheRoot, artifact.version, artifact.key);
  const archive = await materializeArchive(artifact, cacheDirectory);
  const executable = join(cacheDirectory, isWindows ? "rg.exe" : "rg");
  const partial = `${executable}.partial-${randomUUID()}`;
  try {
    await writeFile(partial, extractArchiveMember(archive, artifact.executable), { flag: "wx" });
    if (!isWindows) {
      await chmod(partial, 0o755);
    }
    await rm(executable, { force: true });
    await rename(partial, executable);
  } finally {
    await rm(partial, { force: true });
  }
  return {
    archive: artifact.archive,
    archiveSha256: artifact.sha256,
    binarySha256: await sha256(executable),
    executable,
    source: "upstream-release",
    version: artifact.version,
  };
}

function run(command, args, options = {}) {
  const result = spawnSync(command, args, {
    cwd: repositoryRoot,
    env: options.env ?? process.env,
    stdio: "inherit",
    windowsHide: true,
  });
  if (result.error) {
    throw result.error;
  }
  if (result.status !== 0) {
    throw new Error(`${command} exited with status ${result.status}`);
  }
}

function cargoBuild(target, packageName, binaryArgs, environment = process.env) {
  run("cargo", [
    "build",
    "--manifest-path",
    join(cargoWorkspace, "Cargo.toml"),
    "--package",
    packageName,
    ...binaryArgs,
    "--profile",
    "dev",
    "--target",
    target,
    "--target-dir",
    join(cargoWorkspace, "target"),
  ], { env: environment });
}

async function buildFirstPartyExecutables(target, platform) {
  cargoBuild(target, "zeta-cli", ["--bin", "zeta"]);
  const debugDirectory = join(cargoWorkspace, "target", target, "debug");
  const executables = {
    zeta: join(debugDirectory, platform === "win32" ? "zeta.exe" : "zeta"),
  };
  if (platform === "win32") {
    cargoBuild(target, "zeta-windows-sandbox", ["--bins"]);
    executables.commandRunner = join(debugDirectory, "zeta-command-runner.exe");
    executables.sandboxSetup = join(debugDirectory, "zeta-windows-sandbox-setup.exe");
  }
  if (platform === "linux") {
    const bubblewrap = await materializeBubblewrapSource();
    cargoBuild(target, "zeta-bwrap", ["--bin", "bwrap"], {
      ...process.env,
      ZETA_BWRAP_SOURCE_DIR: bubblewrap.sourceDirectory,
    });
    executables.bubblewrap = {
      ...bubblewrap,
      binary: join(debugDirectory, "bwrap"),
    };
  }
  for (const path of Object.values(executables).filter((value) => typeof value === "string")) {
    const metadata = await stat(path);
    if (!metadata.isFile()) {
      throw new Error(`Cargo did not produce an expected executable: ${path}`);
    }
  }
  return executables;
}

async function materializeBubblewrapSource() {
  const lock = JSON.parse(await readFile(bubblewrapLockPath, "utf8"));
  if (lock.schemaVersion !== 1 || lock.runtime !== "bubblewrap-source" || lock.archive?.format !== "tar.xz") {
    throw new Error("Unsupported Bubblewrap source lock");
  }
  const archiveMetadata = lock.archive;
  const artifact = {
    archive: archiveMetadata.name,
    executable: "",
    format: archiveMetadata.format,
    key: lock.version,
    sha256: archiveMetadata.sha256,
    size: archiveMetadata.size,
    url: archiveMetadata.url ?? `${lock.source.repository.replace(/\/+$/, "")}/releases/download/${lock.source.release}/${archiveMetadata.name}`,
    version: lock.version,
  };
  const versionDirectory = join(bubblewrapCacheRoot, lock.version);
  const archive = await materializeArchive(artifact, versionDirectory);
  const sourceDirectory = join(versionDirectory, "source");
  const marker = join(sourceDirectory, ".zeta-source-sha256");
  try {
    if ((await readFile(marker, "utf8")).trim() === artifact.sha256) {
      return {
        archive: artifact.archive,
        archiveSha256: artifact.sha256,
        license: join(sourceDirectory, "COPYING"),
        sourceDirectory,
        version: lock.version,
      };
    }
  } catch {
    // Rebuild the verified source cache below.
  }
  const staging = `${sourceDirectory}.partial-${randomUUID()}`;
  await rm(staging, { force: true, recursive: true });
  await mkdir(staging, { recursive: true });
  try {
    for (const member of archiveMetadata.members) {
      if (typeof member !== "string" || member.length === 0 || member.includes("..")) {
        throw new Error(`Unsafe Bubblewrap archive member: ${member}`);
      }
      const destination = join(staging, member);
      await mkdir(dirname(destination), { recursive: true });
      await writeFile(destination, extractArchiveMember(archive, `${archiveMetadata.root}/${member}`));
    }
    await writeFile(join(staging, ".zeta-source-sha256"), `${artifact.sha256}\n`);
    await rm(sourceDirectory, { force: true, recursive: true });
    await rename(staging, sourceDirectory);
  } catch (error) {
    await rm(staging, { force: true, recursive: true });
    throw error;
  }
  return {
    archive: artifact.archive,
    archiveSha256: artifact.sha256,
    license: join(sourceDirectory, "COPYING"),
    sourceDirectory,
    version: lock.version,
  };
}

async function copyExecutable(source, destination, isWindows) {
  await copyFile(source, destination);
  if (!isWindows) {
    await chmod(destination, 0o755);
  }
}

async function copyRegularTree(source, destination, kind) {
  const sourceMetadata = await lstat(source);
  if (!sourceMetadata.isDirectory() || sourceMetadata.isSymbolicLink()) {
    throw new Error(`Built-in ${kind} source is not a real directory: ${source}`);
  }
  await mkdir(destination);
  for (const entry of await readdir(source, { withFileTypes: true })) {
    const sourcePath = join(source, entry.name);
    const destinationPath = join(destination, entry.name);
    const metadata = await lstat(sourcePath);
    if (metadata.isSymbolicLink()) {
      throw new Error(`Built-in ${kind} asset is a symbolic link: ${sourcePath}`);
    }
    if (metadata.isDirectory()) {
      await copyRegularTree(sourcePath, destinationPath, kind);
    } else if (metadata.isFile() && metadata.nlink === 1) {
      await copyFile(sourcePath, destinationPath);
    } else {
      throw new Error(`Built-in ${kind} asset is not a regular unlinked file: ${sourcePath}`);
    }
  }
}

async function copyBuiltinSkills(destination) {
  const source = join(sharedRustSource, "skills", "assets");
  const entries = (await readdir(source, { withFileTypes: true })).filter((entry) => entry.name !== "BUILD.bazel");
  if (entries.length === 0) {
    throw new Error("Built-in Skill source is empty");
  }
  await mkdir(destination, { recursive: true });
  for (const entry of entries) {
    if (!entry.isDirectory() || !/^[a-z0-9]+(?:-[a-z0-9]+)*$/.test(entry.name)) {
      throw new Error(`Invalid built-in Skill directory: ${entry.name}`);
    }
    await stat(join(source, entry.name, "SKILL.md"));
    await copyRegularTree(join(source, entry.name), join(destination, entry.name), "Skill");
  }
}

async function copyBuiltinExtensions(destination) {
  const source = join(repositoryRoot, "extensions");
  const entries = (await readdir(source, { withFileTypes: true })).filter(
    (entry) => entry.name !== "README.md" && entry.name !== "BUILD.bazel",
  );
  await mkdir(destination, { recursive: true });
  for (const entry of entries) {
    if (!entry.isDirectory()) {
      throw new Error(`Invalid built-in extension package: ${entry.name}`);
    }
    await stat(join(source, entry.name, "package.json"));
    await copyRegularTree(join(source, entry.name), join(destination, entry.name), "extension package");
  }
}

async function workspaceVersion() {
  const manifest = await readFile(join(repositoryRoot, "Cargo.toml"), "utf8");
  const workspacePackage = manifest.match(/\[workspace\.package\]([\s\S]*?)(?:\n\[|$)/)?.[1];
  const version = workspacePackage?.match(/^\s*version\s*=\s*"([^"]+)"/m)?.[1];
  if (!version) {
    throw new Error("Could not read workspace.package.version");
  }
  return version;
}

export async function assemblePackage(staging, target, platform, executables, ripgrep) {
  const isWindows = platform === "win32";
  const zetaName = isWindows ? "zeta.exe" : "zeta";
  const rgName = isWindows ? "rg.exe" : "rg";
  const binDirectory = join(staging, "bin");
  const pathDirectory = join(staging, "zeta-path");
  const resourcesDirectory = join(staging, "zeta-resources");
  const ripgrepLicenseDirectory = join(resourcesDirectory, "licenses", "ripgrep");
  await mkdir(binDirectory, { recursive: true });
  await mkdir(pathDirectory, { recursive: true });
  await mkdir(ripgrepLicenseDirectory, { recursive: true });
  await copyBuiltinSkills(join(resourcesDirectory, "skills"));
  await copyBuiltinExtensions(join(resourcesDirectory, "extensions"));
  await copyExecutable(executables.zeta, join(binDirectory, zetaName), isWindows);
  await copyExecutable(ripgrep.executable, join(pathDirectory, rgName), isWindows);
  for (const name of ["LICENSE-MIT", "UNLICENSE"]) {
    await copyFile(join(repositoryRoot, "third_party", "ripgrep", name), join(ripgrepLicenseDirectory, name));
  }

  const components = {
    ripgrep: {
      archive: ripgrep.archive,
      archiveSha256: ripgrep.archiveSha256,
      binarySha256: ripgrep.binarySha256,
      source: ripgrep.source,
      version: ripgrep.version,
    },
  };
  if (isWindows) {
    await copyExecutable(executables.commandRunner, join(resourcesDirectory, "zeta-command-runner.exe"), true);
    await copyExecutable(executables.sandboxSetup, join(resourcesDirectory, "zeta-windows-sandbox-setup.exe"), true);
    components.windowsSandbox = {
      commandRunnerSha256: await sha256(executables.commandRunner),
      sandboxSetupSha256: await sha256(executables.sandboxSetup),
      source: "cargo-build",
    };
  }
  if (platform === "linux") {
    await copyExecutable(executables.bubblewrap.binary, join(resourcesDirectory, "bwrap"), false);
    const licenseDirectory = join(resourcesDirectory, "licenses", "bubblewrap");
    await mkdir(licenseDirectory, { recursive: true });
    await copyFile(executables.bubblewrap.license, join(licenseDirectory, "COPYING"));
    components.bubblewrap = {
      binarySha256: await sha256(executables.bubblewrap.binary),
      source: "source-build",
      sourceArchive: executables.bubblewrap.archive,
      sourceArchiveSha256: executables.bubblewrap.archiveSha256,
      version: executables.bubblewrap.version,
    };
  }
  const metadata = {
    components,
    entrypoint: `bin/${zetaName}`,
    layoutVersion: 1,
    pathDir: "zeta-path",
    resourcesDir: "zeta-resources",
    target,
    version: await workspaceVersion(),
  };
  await writeFile(join(staging, "zeta-package.json"), `${JSON.stringify(metadata, null, 2)}\n`);
  await validatePackage(staging, platform);
}

async function requireFile(path) {
  const metadata = await stat(path);
  if (!metadata.isFile()) {
    throw new Error(`Missing package file: ${path}`);
  }
}

async function validatePackage(packageRoot, platform) {
  const isWindows = platform === "win32";
  await requireFile(join(packageRoot, "zeta-package.json"));
  await requireFile(join(packageRoot, "bin", isWindows ? "zeta.exe" : "zeta"));
  await requireFile(join(packageRoot, "zeta-path", isWindows ? "rg.exe" : "rg"));
  await requireFile(join(packageRoot, "zeta-resources", "licenses", "ripgrep", "LICENSE-MIT"));
  await requireFile(join(packageRoot, "zeta-resources", "licenses", "ripgrep", "UNLICENSE"));
  if (isWindows) {
    await requireFile(join(packageRoot, "zeta-resources", "zeta-command-runner.exe"));
    await requireFile(join(packageRoot, "zeta-resources", "zeta-windows-sandbox-setup.exe"));
  }
  if (platform === "linux") {
    await requireFile(join(packageRoot, "zeta-resources", "bwrap"));
    await requireFile(join(packageRoot, "zeta-resources", "licenses", "bubblewrap", "COPYING"));
  }
  const extensionEntries = await readdir(join(packageRoot, "zeta-resources", "extensions"), { withFileTypes: true });
  for (const extensionEntry of extensionEntries) {
    if (!extensionEntry.isDirectory()) {
      throw new Error(`Package contains an invalid built-in extension entry: ${extensionEntry.name}`);
    }
    await requireFile(join(packageRoot, "zeta-resources", "extensions", extensionEntry.name, "package.json"));
  }
  const skillNames = await readdir(join(packageRoot, "zeta-resources", "skills"));
  if (skillNames.length === 0) {
    throw new Error("Package contains no built-in Skills");
  }
  for (const skillName of skillNames) {
    await requireFile(join(packageRoot, "zeta-resources", "skills", skillName, "SKILL.md"));
  }
}

export async function replaceDirectoryAtomically(output, build) {
  await mkdir(dirname(output), { recursive: true });
  const generation = randomUUID();
  const staging = join(dirname(output), `.${basename(output)}.next-${generation}`);
  const previous = join(dirname(output), `.${basename(output)}.previous-${generation}`);
  let movedPrevious = false;
  await rm(staging, { force: true, recursive: true });
  try {
    await build(staging);
    try {
      await rename(output, previous);
      movedPrevious = true;
    } catch (error) {
      if (error.code !== "ENOENT") {
        throw error;
      }
    }
    try {
      await rename(staging, output);
    } catch (error) {
      if (movedPrevious) {
        await rename(previous, output);
        movedPrevious = false;
      }
      throw error;
    }
    if (movedPrevious) {
      await rm(previous, { force: true, recursive: true }).catch(() => {});
    }
  } finally {
    await rm(staging, { force: true, recursive: true }).catch(() => {});
    if (movedPrevious) {
      try {
        await stat(output);
      } catch {
        await rename(previous, output);
      }
    }
  }
}

export async function prepareDevelopmentPackage() {
  const target = hostTarget();
  const isWindows = process.platform === "win32";
  const executables = await buildFirstPartyExecutables(target, process.platform);
  const ripgrep = await resolveRipgrep(target, isWindows);
  await replaceDirectoryAtomically(outputDirectory, (staging) => assemblePackage(
    staging,
    target,
    process.platform,
    executables,
    ripgrep,
  ));
  console.log(`Prepared Zeta Desktop development package at ${outputDirectory}`);
}

const isMain = process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url);
if (isMain) {
  prepareDevelopmentPackage().catch((error) => {
    console.error(error instanceof Error ? error.message : error);
    process.exitCode = 1;
  });
}
