import { spawnSync } from "node:child_process";
import { createHash, randomUUID } from "node:crypto";
import { constants } from "node:fs";
import { chmod, copyFile, lstat, mkdir, readFile, readdir, rename, rm, stat, writeFile } from "node:fs/promises";
import { basename, dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { cargoArtifactExecutable, cargoRenderedDiagnostic, cargoTargetDirectory, parseCargoMessage } from "../lib/cargo.ts";
import { desktopBuildPath } from "../lib/paths.ts";
import { APP_SERVER_PROTOCOL_MAJOR, APP_SERVER_PROTOCOL_REVISION, APP_SERVER_SCHEMA_HASH } from "../../zeta-ts/generated/app-server/types.ts";

const repositoryRoot = resolve(import.meta.dirname, "..", "..");
const cargoWorkspace = repositoryRoot;
const sharedRustSource = join(repositoryRoot, "zeta-rs");
const outputDirectory = desktopBuildPath(repositoryRoot, "dev", "zeta-package");
const ripgrepLockPath = join(repositoryRoot, "third_party", "ripgrep", "runtime-lock.json");
const ripgrepCacheRoot = join(repositoryRoot, "third_party", ".cache", "ripgrep");
const nodeLockPath = join(repositoryRoot, "third_party", "node", "runtime-lock.json");
const nodeCacheRoot = join(repositoryRoot, "third_party", ".cache", "node");
const bubblewrapLockPath = join(repositoryRoot, "third_party", "bubblewrap", "runtime-lock.json");
const bubblewrapCacheRoot = join(repositoryRoot, "third_party", ".cache", "bubblewrap");
const v8LockPath = join(repositoryRoot, "third_party", "v8", "runtime-lock.json");
const v8CacheRoot = join(repositoryRoot, "third_party", ".cache", "v8");
const archiveBufferLimit = 256 * 1024 * 1024;
const javascriptRuntimeKinds = new Set<JavaScriptRuntimeKind>(["host-provided-node", "packaged-node"]);

type JavaScriptRuntimeKind = "host-provided-node" | "packaged-node";

interface RemoteRuntimeRelease {
  readonly sha256: string;
  readonly url: string;
}

interface PackageOptions {
  readonly javascriptRuntime: JavaScriptRuntimeKind;
  readonly remoteRuntimeBundle: string | undefined;
  readonly remoteRuntimeRelease: RemoteRuntimeRelease | undefined;
}

interface LockedRuntimeArtifact {
  readonly archive: string;
  readonly executable: string;
  readonly format: string;
  readonly license?: string;
  readonly sha256: string;
  readonly size: number;
  readonly url?: string;
}

interface RuntimeLock {
  readonly artifacts?: Readonly<Record<string, LockedRuntimeArtifact>>;
  readonly packageTargets?: Readonly<Record<string, string>>;
  readonly runtime: string;
  readonly schemaVersion: number;
  readonly source?: { readonly baseUrl?: string; readonly release?: string; readonly repository?: string };
  readonly version: string;
}

interface V8LockedFile {
  readonly name: string;
  readonly sha256: string;
}

interface V8RuntimeLock {
  readonly artifacts: Readonly<Record<string, { readonly archive: V8LockedFile; readonly binding: V8LockedFile }>>;
  readonly profile: string;
  readonly runtime: string;
  readonly schemaVersion: number;
  readonly source: { readonly release: string; readonly repository: string };
  readonly version: string;
}

interface ResolvedV8File extends V8LockedFile {
  readonly url: string;
}

interface ResolvedV8ArtifactPair {
  readonly archive: ResolvedV8File;
  readonly binding: ResolvedV8File;
  readonly version: string;
}

interface ResolvedArchiveArtifact extends LockedRuntimeArtifact {
  readonly key: string;
  readonly url: string;
  readonly version: string;
}

interface ResolvedNodeArchiveArtifact extends ResolvedArchiveArtifact {
  readonly license: string;
}

interface ResolvedRipgrep {
  readonly archive: string;
  readonly archiveSha256: string;
  readonly binarySha256: string;
  readonly executable: string;
  readonly source: "upstream-release";
  readonly version: string;
}

interface ResolvedNode extends ResolvedRipgrep {
  readonly license: string;
}

interface ResolvedBubblewrap {
  readonly archive: string;
  readonly archiveSha256: string;
  readonly binary: string;
  readonly license: string;
  readonly sourceDirectory: string;
  readonly version: string;
}

interface FirstPartyExecutables {
  readonly appServerDaemon: string;
  readonly bubblewrap?: ResolvedBubblewrap;
  readonly commandRunner?: string;
  readonly sandboxSetup?: string;
  readonly serverHost: string;
  readonly codeModeHost: string;
}

interface PackageMetadata {
  readonly buildId: string;
  readonly components: Record<string, unknown> & { node?: unknown };
  readonly entrypoint: string;
  readonly javascriptRuntime: { readonly kind: string };
  readonly layoutVersion: number;
  readonly pathDir: string;
  readonly protocol: { readonly major: number; readonly revision: number; readonly schemaHash: string };
  remoteRuntimeCatalog?: { readonly path?: string; readonly sha256: string; readonly trustBinding: string; readonly url?: string };
  readonly resourcesDir: string;
  readonly target: string;
  readonly version: string;
}

export function parseJavaScriptRuntime(cliArguments: readonly string[]): JavaScriptRuntimeKind {
  return parsePackageOptions(cliArguments).javascriptRuntime;
}

export function parsePackageOptions(cliArguments: readonly string[]): PackageOptions {
  let javascriptRuntime: JavaScriptRuntimeKind = "host-provided-node";
  let javascriptRuntimeSpecified = false;
  let remoteRuntimeBundle;
  let remoteRuntimeCatalogUrl;
  let remoteRuntimeCatalogSha256;
  for (let index = 0; index < cliArguments.length; index += 2) {
    const name = cliArguments[index];
    const value = cliArguments[index + 1];
    if (value === undefined) throw packageUsage();
    if (name === "--javascript-runtime" && isJavaScriptRuntimeKind(value) && !javascriptRuntimeSpecified) {
      javascriptRuntime = value;
      javascriptRuntimeSpecified = true;
    } else if (name === "--remote-runtime-bundle" && value.length > 0 && remoteRuntimeBundle === undefined) {
      remoteRuntimeBundle = resolve(value);
    } else if (name === "--remote-runtime-catalog-url" && value.length > 0 && remoteRuntimeCatalogUrl === undefined) {
      remoteRuntimeCatalogUrl = value;
    } else if (name === "--remote-runtime-catalog-sha256" && /^[a-f0-9]{64}$/.test(value) && remoteRuntimeCatalogSha256 === undefined) {
      remoteRuntimeCatalogSha256 = value;
    } else {
      throw packageUsage();
    }
  }
  if ((remoteRuntimeCatalogUrl === undefined) !== (remoteRuntimeCatalogSha256 === undefined)) throw packageUsage();
  if (remoteRuntimeCatalogUrl !== undefined) validateRemoteRuntimeCatalogUrl(remoteRuntimeCatalogUrl);
  const remoteRuntimeRelease = remoteRuntimeCatalogUrl === undefined
    ? undefined
    : { url: remoteRuntimeCatalogUrl, sha256: remoteRuntimeCatalogSha256 as string };
  return { javascriptRuntime, remoteRuntimeBundle, remoteRuntimeRelease };
}

function packageUsage(): Error {
  return new Error("Usage: node build/desktop/prepareDevPackage.ts [--javascript-runtime host-provided-node|packaged-node] [--remote-runtime-bundle <bundle-directory>] [--remote-runtime-catalog-url <https-catalog.json> --remote-runtime-catalog-sha256 <digest>]");
}

function validateRemoteRuntimeCatalogUrl(value: string): void {
  let url: URL;
  try {
    url = new URL(value);
  } catch (error) {
    throw new Error("Remote runtime catalog URL is invalid", { cause: error });
  }
  if (url.protocol !== "https:" || !url.hostname || url.username || url.password || url.search || url.hash || !url.pathname.endsWith("/catalog.json")) {
    throw new Error("Remote runtime catalog URL must be a credential-free HTTPS catalog.json URL without query or fragment");
  }
}

export function hostTarget(platform: NodeJS.Platform = process.platform, architecture: string = process.arch): string {
  const targets: Readonly<Record<string, string>> = {
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

export function selectV8ArtifactPair(lock: V8RuntimeLock, target: string): ResolvedV8ArtifactPair {
  if (lock.schemaVersion !== 1 || lock.runtime !== "rusty-v8" || lock.profile !== "ptrcomp_sandbox_release") {
    throw new Error("Unsupported rusty_v8 runtime lock");
  }
  const release = lock.source?.release;
  const repository = lock.source?.repository;
  let parsedRepository: URL;
  try {
    parsedRepository = new URL(repository);
  } catch (error) {
    throw new Error("rusty_v8 repository URL is invalid", { cause: error });
  }
  if (parsedRepository.protocol !== "https:" || parsedRepository.username || parsedRepository.password || parsedRepository.search || parsedRepository.hash) {
    throw new Error("rusty_v8 repository must be a credential-free HTTPS URL");
  }
  if (release !== `rusty-v8-v${lock.version}`) {
    throw new Error("rusty_v8 release does not match its locked version");
  }
  const pair = lock.artifacts?.[target];
  if (!pair) throw new Error(`No locked rusty_v8 artifacts for ${target}`);
  const windows = target.includes("windows");
  const expectedArchive = windows
    ? `rusty_v8_${lock.profile}_${target}.lib.gz`
    : `librusty_v8_${lock.profile}_${target}.a.gz`;
  const expectedBinding = `src_binding_${lock.profile}_${target}.rs`;
  const baseUrl = `${repository.replace(/\/+$/u, "")}/releases/download/${release}`;
  const resolveFile = (file: V8LockedFile, expectedName: string): ResolvedV8File => {
    if (file.name !== expectedName || !/^[a-f0-9]{64}$/u.test(file.sha256)) {
      throw new Error(`Invalid locked rusty_v8 artifact for ${target}: ${file.name}`);
    }
    return { ...file, url: `${baseUrl}/${file.name}` };
  };
  return {
    archive: resolveFile(pair.archive, expectedArchive),
    binding: resolveFile(pair.binding, expectedBinding),
    version: lock.version,
  };
}

export function selectRipgrepArtifact(lock: RuntimeLock, target: string): ResolvedArchiveArtifact {
  if (lock.schemaVersion !== 1 || lock.runtime !== "ripgrep") {
    throw new Error("Unsupported ripgrep runtime lock");
  }
  const artifactKey = lock.packageTargets?.[target];
  const artifact = artifactKey ? lock.artifacts?.[artifactKey] : undefined;
  if (!artifactKey || !artifact) {
    throw new Error(`No locked ripgrep artifact for ${target}`);
  }
  for (const field of ["archive", "sha256", "format", "executable"] as const) {
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

export function selectNodeArtifact(lock: RuntimeLock, target: string): ResolvedNodeArchiveArtifact {
  if (lock.schemaVersion !== 1 || lock.runtime !== "node") {
    throw new Error("Unsupported Node.js runtime lock");
  }
  const artifactKey = lock.packageTargets?.[target];
  const artifact = artifactKey ? lock.artifacts?.[artifactKey] : undefined;
  if (!artifactKey || !artifact) {
    throw new Error(`No locked Node.js artifact for ${target}`);
  }
  for (const field of ["archive", "sha256", "format", "executable", "license"] as const) {
    if (typeof artifact[field] !== "string" || artifact[field].length === 0) {
      throw new Error(`Invalid Node.js artifact field ${field} for ${target}`);
    }
  }
  if (!/^[0-9a-f]{64}$/.test(artifact.sha256)) {
    throw new Error(`Invalid Node.js artifact SHA-256 for ${target}`);
  }
  if (artifact.format !== "tar.xz" && artifact.format !== "zip") {
    throw new Error(`Unsupported Node.js archive format for ${target}: ${artifact.format}`);
  }
  if (!Number.isSafeInteger(artifact.size) || artifact.size <= 0) {
    throw new Error(`Invalid Node.js artifact size for ${target}`);
  }
  const baseUrl = lock.source?.baseUrl;
  if (typeof baseUrl !== "string" || baseUrl.length === 0) {
    throw new Error("Node.js lock is missing its upstream release URL");
  }
  return {
    ...artifact,
    key: artifactKey,
    license: artifact.license as string,
    url: `${baseUrl.replace(/\/+$/, "")}/${artifact.archive}`,
    version: lock.version,
  };
}

async function sha256(path: string): Promise<string> {
  return createHash("sha256").update(await readFile(path)).digest("hex");
}

async function verifyArchive(path: string, artifact: Pick<ResolvedArchiveArtifact, "sha256" | "size">): Promise<boolean> {
  try {
    const metadata = await stat(path);
    return metadata.isFile() && metadata.size === artifact.size && await sha256(path) === artifact.sha256;
  } catch {
    return false;
  }
}

async function downloadArchive(artifact: ResolvedArchiveArtifact, destination: string): Promise<void> {
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

async function materializeArchive(artifact: ResolvedArchiveArtifact, cacheDirectory: string): Promise<string> {
  const archive = join(cacheDirectory, artifact.archive);
  if (!await verifyArchive(archive, artifact)) {
    await rm(archive, { force: true });
    await downloadArchive(artifact, archive);
  }
  return archive;
}

function extractArchiveMember(archive: string, member: string): Buffer {
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

async function resolveRipgrep(target: string, isWindows: boolean): Promise<ResolvedRipgrep> {
  const lock = JSON.parse(await readFile(ripgrepLockPath, "utf8")) as RuntimeLock;
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

async function resolveNode(target: string, isWindows: boolean): Promise<ResolvedNode> {
  const lock = JSON.parse(await readFile(nodeLockPath, "utf8")) as RuntimeLock;
  const artifact = selectNodeArtifact(lock, target);
  const cacheDirectory = join(nodeCacheRoot, artifact.version, artifact.key);
  const archive = await materializeArchive(artifact, cacheDirectory);
  const executable = join(cacheDirectory, isWindows ? "node.exe" : "node");
  const license = join(cacheDirectory, "LICENSE");
  const executablePartial = `${executable}.partial-${randomUUID()}`;
  const licensePartial = `${license}.partial-${randomUUID()}`;
  try {
    await writeFile(executablePartial, extractArchiveMember(archive, artifact.executable), { flag: "wx" });
    await writeFile(licensePartial, extractArchiveMember(archive, artifact.license), { flag: "wx" });
    if (!isWindows) {
      await chmod(executablePartial, 0o755);
    }
    await rm(executable, { force: true });
    await rename(executablePartial, executable);
    await rm(license, { force: true });
    await rename(licensePartial, license);
  } finally {
    await rm(executablePartial, { force: true });
    await rm(licensePartial, { force: true });
  }
  return {
    archive: artifact.archive,
    archiveSha256: artifact.sha256,
    binarySha256: await sha256(executable),
    executable,
    license,
    source: "upstream-release",
    version: artifact.version,
  };
}

async function materializeV8File(file: ResolvedV8File, cacheDirectory: string): Promise<string> {
  const destination = join(cacheDirectory, file.name);
  if (await pathExists(destination) && await sha256(destination) === file.sha256) return destination;
  await mkdir(cacheDirectory, { recursive: true });
  const partial = `${destination}.partial-${randomUUID()}`;
  try {
    const response = await fetch(file.url, {
      headers: { "user-agent": "zeta-development-package-builder" },
      signal: AbortSignal.timeout(120_000),
    });
    if (!response.ok) throw new Error(`Could not download ${file.url}: HTTP ${response.status}`);
    const body = Buffer.from(await response.arrayBuffer());
    if (body.byteLength > archiveBufferLimit) {
      throw new Error(`rusty_v8 artifact exceeds the download limit: ${file.name}`);
    }
    await writeFile(partial, body, { flag: "wx" });
    if (await sha256(partial) !== file.sha256) {
      throw new Error(`Downloaded rusty_v8 artifact failed SHA-256 validation: ${file.name}`);
    }
    await rm(destination, { force: true });
    await rename(partial, destination);
    return destination;
  } finally {
    await rm(partial, { force: true });
  }
}

async function v8CargoEnvironment(target: string): Promise<NodeJS.ProcessEnv> {
  if (/^(1|true|yes)$/iu.test(process.env.V8_FROM_SOURCE ?? "")) return { ...process.env };
  const archiveOverride = process.env.RUSTY_V8_ARCHIVE;
  const bindingOverride = process.env.RUSTY_V8_SRC_BINDING_PATH;
  if (archiveOverride && bindingOverride) return { ...process.env };
  if (archiveOverride || bindingOverride) {
    throw new Error("RUSTY_V8_ARCHIVE and RUSTY_V8_SRC_BINDING_PATH must be set together");
  }
  const lock = JSON.parse(await readFile(v8LockPath, "utf8")) as V8RuntimeLock;
  const pair = selectV8ArtifactPair(lock, target);
  const cacheDirectory = join(v8CacheRoot, pair.version, target);
  const [archive, binding] = await Promise.all([
    materializeV8File(pair.archive, cacheDirectory),
    materializeV8File(pair.binding, cacheDirectory),
  ]);
  return {
    ...process.env,
    RUSTY_V8_ARCHIVE: archive,
    RUSTY_V8_SRC_BINDING_PATH: binding,
  };
}

function cargoBuild(packageName: string, binaryArgs: readonly string[], expectedTargets: readonly string[], environment: NodeJS.ProcessEnv = process.env): Map<string, string> {
  const result = spawnSync("cargo", [
    "build",
    "--manifest-path",
    join(cargoWorkspace, "Cargo.toml"),
    "--package",
    packageName,
    ...binaryArgs,
    "--profile",
    "dev-small",
    "--target-dir",
    cargoTargetDirectory(cargoWorkspace, environment),
    "--message-format",
    "json-render-diagnostics",
  ], {
    cwd: repositoryRoot,
    encoding: "utf8",
    env: environment,
    maxBuffer: archiveBufferLimit,
    stdio: ["inherit", "pipe", "pipe"],
    windowsHide: true,
  });
  if (result.stderr) process.stderr.write(result.stderr);
  if (result.error) throw result.error;
  const executables = new Map<string, string>();
  for (const line of result.stdout?.split(/\r?\n/u) ?? []) {
    const message = parseCargoMessage(line);
    const diagnostic = cargoRenderedDiagnostic(message);
    if (diagnostic) process.stderr.write(diagnostic);
    for (const targetName of expectedTargets) {
      const executable = cargoArtifactExecutable(message, targetName);
      if (executable) executables.set(targetName, executable);
    }
  }
  if (result.status !== 0) throw new Error(`cargo exited with status ${result.status}`);
  for (const targetName of expectedTargets) {
    if (!executables.has(targetName)) throw new Error(`Cargo did not report the ${targetName} executable`);
  }
  return executables;
}

async function buildFirstPartyExecutables(platform: NodeJS.Platform): Promise<FirstPartyExecutables> {
  const cargoEnvironment = await v8CargoEnvironment(hostTarget(platform));
  const serverArtifacts = cargoBuild("zeta-server-host", ["--bin", "zeta-server"], ["zeta-server"], cargoEnvironment);
  const daemonArtifacts = cargoBuild("zeta-app-server-daemon", ["--bin", "zeta-app-server-daemon"], ["zeta-app-server-daemon"], cargoEnvironment);
  const codeModeHostArtifacts = cargoBuild("zeta-code-mode-host", ["--bin", "zeta-code-mode-host"], ["zeta-code-mode-host"], cargoEnvironment);
  const executables: {
    appServerDaemon: string;
    bubblewrap?: ResolvedBubblewrap;
    commandRunner?: string;
    sandboxSetup?: string;
    serverHost: string;
    codeModeHost: string;
  } = {
    appServerDaemon: requiredExecutable(daemonArtifacts, "zeta-app-server-daemon"),
    codeModeHost: requiredExecutable(codeModeHostArtifacts, "zeta-code-mode-host"),
    serverHost: requiredExecutable(serverArtifacts, "zeta-server"),
  };
  if (platform === "win32") {
    const sandboxArtifacts = cargoBuild("zeta-windows-sandbox", ["--bins"], ["zeta-command-runner", "zeta-windows-sandbox-setup"], cargoEnvironment);
    executables.commandRunner = requiredExecutable(sandboxArtifacts, "zeta-command-runner");
    executables.sandboxSetup = requiredExecutable(sandboxArtifacts, "zeta-windows-sandbox-setup");
  }
  if (platform === "linux") {
    const bubblewrap = await materializeBubblewrapSource();
    const bubblewrapArtifacts = cargoBuild("zeta-bwrap", ["--bin", "bwrap"], ["bwrap"], {
      ...cargoEnvironment,
      ZETA_BWRAP_SOURCE_DIR: bubblewrap.sourceDirectory,
    });
    executables.bubblewrap = {
      ...bubblewrap,
      binary: requiredExecutable(bubblewrapArtifacts, "bwrap"),
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

function isJavaScriptRuntimeKind(value: string): value is JavaScriptRuntimeKind {
  return javascriptRuntimeKinds.has(value as JavaScriptRuntimeKind);
}

function requiredExecutable(executables: ReadonlyMap<string, string>, name: string): string {
  const executable = executables.get(name);
  if (!executable) throw new Error(`Cargo did not report the ${name} executable`);
  return executable;
}

async function materializeBubblewrapSource(): Promise<Omit<ResolvedBubblewrap, "binary">> {
  const lock = JSON.parse(await readFile(bubblewrapLockPath, "utf8")) as {
    readonly archive: { readonly format: string; readonly members: readonly string[]; readonly name: string; readonly root: string; readonly sha256: string; readonly size: number; readonly url?: string };
    readonly runtime: string;
    readonly schemaVersion: number;
    readonly source: { readonly release: string; readonly repository: string };
    readonly version: string;
  };
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

async function copyExecutable(source: string, destination: string, isWindows: boolean): Promise<void> {
  await copyFile(source, destination, constants.COPYFILE_FICLONE);
  if (!isWindows) {
    await chmod(destination, 0o755);
  }
}

async function copyRegularTree(source: string, destination: string, kind: string): Promise<void> {
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

async function copyBuiltinSkills(destination: string): Promise<void> {
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

export async function copyBuiltinExtensions(destination: string, source = join(repositoryRoot, "extensions")): Promise<void> {
  const sourceMetadata = await lstat(source);
  if (!sourceMetadata.isDirectory() || sourceMetadata.isSymbolicLink()) {
    throw new Error(`Built-in extension source is not a real directory: ${source}`);
  }
  const entries = (await readdir(source, { withFileTypes: true })).filter(
    (entry) => entry.name !== "README.md" && entry.name !== "BUILD.bazel",
  );
  if (entries.length === 0) {
    throw new Error("Built-in extension source is empty");
  }
  await mkdir(destination, { recursive: true });
  for (const entry of entries) {
    if (!entry.isDirectory()) {
      throw new Error(`Invalid built-in extension package: ${entry.name}`);
    }
    const manifest = join(source, entry.name, "package.json");
    let manifestMetadata;
    try {
      manifestMetadata = await lstat(manifest);
    } catch (error: unknown) {
      if (isErrorCode(error, "ENOENT")) throw new Error(`Built-in extension is missing package.json: ${entry.name}`, { cause: error });
      throw error;
    }
    if (!manifestMetadata.isFile() || manifestMetadata.isSymbolicLink()) {
      throw new Error(`Built-in extension package.json is not a regular file: ${entry.name}`);
    }
    await copyRegularTree(join(source, entry.name), join(destination, entry.name), "extension package");
  }
}

async function workspaceVersion(): Promise<string> {
  const manifest = await readFile(join(repositoryRoot, "Cargo.toml"), "utf8");
  const workspacePackage = manifest.match(/\[workspace\.package\]([\s\S]*?)(?:\n\[|$)/)?.[1];
  const version = workspacePackage?.match(/^\s*version\s*=\s*"([^"]+)"/m)?.[1];
  if (!version) {
    throw new Error("Could not read workspace.package.version");
  }
  return version;
}

export async function assemblePackage(
  staging: string,
  target: string,
  platform: NodeJS.Platform,
  executables: FirstPartyExecutables,
  ripgrep: ResolvedRipgrep,
  node?: ResolvedNode,
  remoteRuntimeBundle?: string,
  remoteRuntimeRelease?: RemoteRuntimeRelease,
): Promise<void> {
  const isWindows = platform === "win32";
  const appServerDaemonName = isWindows ? "zeta-app-server-daemon.exe" : "zeta-app-server-daemon";
  const codeModeHostName = isWindows ? "zeta-code-mode-host.exe" : "zeta-code-mode-host";
  const serverHostName = isWindows ? "zeta-server.exe" : "zeta-server";
  const rgName = isWindows ? "rg.exe" : "rg";
  const binDirectory = join(staging, "bin");
  const pathDirectory = join(staging, "zeta-path");
  const resourcesDirectory = join(staging, "zeta-resources");
  const ripgrepLicenseDirectory = join(resourcesDirectory, "licenses", "ripgrep");
  const vscodeLicenseDirectory = join(resourcesDirectory, "licenses", "vscode");
  await mkdir(binDirectory, { recursive: true });
  await mkdir(pathDirectory, { recursive: true });
  await mkdir(ripgrepLicenseDirectory, { recursive: true });
  await mkdir(vscodeLicenseDirectory, { recursive: true });
  await copyBuiltinSkills(join(resourcesDirectory, "skills"));
  await copyBuiltinExtensions(join(resourcesDirectory, "extensions"));
  await copyRegularTree(join(repositoryRoot, "resources", "product-services"), join(resourcesDirectory, "product-services"), "product services");
  if (remoteRuntimeBundle) {
    await copyRegularTree(remoteRuntimeBundle, join(staging, "zeta-remote-runtimes"), "Remote runtime bundle");
  }
  await copyExecutable(executables.serverHost, join(binDirectory, serverHostName), isWindows);
  await copyExecutable(executables.appServerDaemon, join(binDirectory, appServerDaemonName), isWindows);
  await copyExecutable(executables.codeModeHost, join(binDirectory, codeModeHostName), isWindows);
  await copyExecutable(ripgrep.executable, join(pathDirectory, rgName), isWindows);
  if (node) {
    const nodeDirectory = join(resourcesDirectory, "node", "bin");
    const nodeLicenseDirectory = join(resourcesDirectory, "licenses", "node");
    await mkdir(nodeDirectory, { recursive: true });
    await mkdir(nodeLicenseDirectory, { recursive: true });
    await copyExecutable(node.executable, join(nodeDirectory, isWindows ? "node.exe" : "node"), isWindows);
    await copyFile(node.license, join(nodeLicenseDirectory, "LICENSE"));
  }
  for (const name of ["LICENSE-MIT", "UNLICENSE"]) {
    await copyFile(join(repositoryRoot, "third_party", "ripgrep", name), join(ripgrepLicenseDirectory, name));
  }
  await copyFile(join(repositoryRoot, "third_party", "vscode", "LICENSE.txt"), join(vscodeLicenseDirectory, "LICENSE.txt"));

  const components: Record<string, unknown> & { node?: unknown } = {
    appServerDaemon: {
      binarySha256: await sha256(join(binDirectory, appServerDaemonName)),
      source: "cargo-build",
    },
    codeModeHost: {
      binarySha256: await sha256(join(binDirectory, codeModeHostName)),
      source: "cargo-build",
    },
    ripgrep: {
      archive: ripgrep.archive,
      archiveSha256: ripgrep.archiveSha256,
      binarySha256: ripgrep.binarySha256,
      source: ripgrep.source,
      version: ripgrep.version,
    },
    serverHost: {
      binarySha256: await sha256(join(binDirectory, serverHostName)),
      source: "cargo-build",
    },
  };
  if (node) {
    components.node = {
      archive: node.archive,
      archiveSha256: node.archiveSha256,
      binarySha256: node.binarySha256,
      source: node.source,
      version: node.version,
    };
  }
  if (isWindows) {
    const commandRunner = requiredPath(executables.commandRunner, "Windows command runner");
    const sandboxSetup = requiredPath(executables.sandboxSetup, "Windows sandbox setup");
    await copyExecutable(commandRunner, join(resourcesDirectory, "zeta-command-runner.exe"), true);
    await copyExecutable(sandboxSetup, join(resourcesDirectory, "zeta-windows-sandbox-setup.exe"), true);
    components.windowsSandbox = {
      commandRunnerSha256: await sha256(commandRunner),
      sandboxSetupSha256: await sha256(sandboxSetup),
      source: "cargo-build",
    };
  }
  if (platform === "linux") {
    const bubblewrap = executables.bubblewrap;
    if (!bubblewrap) throw new Error("Linux package is missing the Bubblewrap build");
    await copyExecutable(bubblewrap.binary, join(resourcesDirectory, "bwrap"), false);
    const licenseDirectory = join(resourcesDirectory, "licenses", "bubblewrap");
    await mkdir(licenseDirectory, { recursive: true });
    await copyFile(bubblewrap.license, join(licenseDirectory, "COPYING"));
    components.bubblewrap = {
      binarySha256: await sha256(bubblewrap.binary),
      source: "source-build",
      sourceArchive: bubblewrap.archive,
      sourceArchiveSha256: bubblewrap.archiveSha256,
      version: bubblewrap.version,
    };
  }
  const version = await workspaceVersion();
  const protocol = {
    major: APP_SERVER_PROTOCOL_MAJOR,
    revision: APP_SERVER_PROTOCOL_REVISION,
    schemaHash: APP_SERVER_SCHEMA_HASH,
  };
  const appServerDaemonSha256 = (components.appServerDaemon as { readonly binarySha256: string }).binarySha256;
  const codeModeHostSha256 = (components.codeModeHost as { readonly binarySha256: string }).binarySha256;
  const serverHostSha256 = (components.serverHost as { readonly binarySha256: string }).binarySha256;
  const buildId = packageBuildId({ appServerDaemonSha256, codeModeHostSha256, protocol, serverHostSha256, target, version });
  const metadata: PackageMetadata = {
    buildId,
    components,
    entrypoint: `bin/${serverHostName}`,
    javascriptRuntime: { kind: node ? "packagedNode" : "hostProvidedNode" },
    layoutVersion: 2,
    pathDir: "zeta-path",
    protocol,
    resourcesDir: "zeta-resources",
    target,
    version,
  };
  if (remoteRuntimeBundle || remoteRuntimeRelease) {
    const packagedCatalogSha256 = remoteRuntimeBundle ? await sha256(join(staging, "zeta-remote-runtimes", "catalog.json")) : undefined;
    if (remoteRuntimeRelease && packagedCatalogSha256 && remoteRuntimeRelease.sha256 !== packagedCatalogSha256) throw new Error("Network Remote runtime catalog SHA-256 does not match the packaged catalog");
    metadata.remoteRuntimeCatalog = remoteRuntimeRelease
      ? { url: remoteRuntimeRelease.url, sha256: remoteRuntimeRelease.sha256, trustBinding: "signedProductPackage" }
      : { path: "zeta-remote-runtimes/catalog.json", sha256: packagedCatalogSha256 as string, trustBinding: "signedProductPackage" };
  }
  await writeFile(join(staging, "zeta-package.json"), `${JSON.stringify(metadata, null, 2)}\n`);
  await validatePackage(staging, platform);
}

function requiredPath(path: string | undefined, description: string): string {
  if (!path) throw new Error(`${description} is missing`);
  return path;
}

async function requireFile(path: string): Promise<void> {
  const metadata = await stat(path);
  if (!metadata.isFile()) {
    throw new Error(`Missing package file: ${path}`);
  }
}

async function pathExists(path: string): Promise<boolean> {
  try {
    await stat(path);
    return true;
  } catch (error: unknown) {
    if (isErrorCode(error, "ENOENT")) return false;
    throw error;
  }
}

async function validatePackage(packageRoot: string, platform: NodeJS.Platform): Promise<void> {
  const isWindows = platform === "win32";
  const metadataPath = join(packageRoot, "zeta-package.json");
  await requireFile(metadataPath);
  const metadata = JSON.parse(await readFile(metadataPath, "utf8")) as PackageMetadata;
  if (metadata.layoutVersion !== 2 || typeof metadata.components !== "object" || metadata.components === null) {
    throw new Error("Invalid package metadata");
  }
  await requireFile(join(packageRoot, "bin", isWindows ? "zeta-server.exe" : "zeta-server"));
  await requireFile(join(packageRoot, "bin", isWindows ? "zeta-app-server-daemon.exe" : "zeta-app-server-daemon"));
  await requireFile(join(packageRoot, "bin", isWindows ? "zeta-code-mode-host.exe" : "zeta-code-mode-host"));
  await requireComponentDigest(metadata, "serverHost", join(packageRoot, "bin", isWindows ? "zeta-server.exe" : "zeta-server"));
  await requireComponentDigest(metadata, "appServerDaemon", join(packageRoot, "bin", isWindows ? "zeta-app-server-daemon.exe" : "zeta-app-server-daemon"));
  await requireComponentDigest(metadata, "codeModeHost", join(packageRoot, "bin", isWindows ? "zeta-code-mode-host.exe" : "zeta-code-mode-host"));
  const appServerDaemonSha256 = (metadata.components.appServerDaemon as { readonly binarySha256: string }).binarySha256;
  const codeModeHostSha256 = (metadata.components.codeModeHost as { readonly binarySha256: string }).binarySha256;
  const serverHostSha256 = (metadata.components.serverHost as { readonly binarySha256: string }).binarySha256;
  if (metadata.buildId !== packageBuildId({ appServerDaemonSha256, codeModeHostSha256, protocol: metadata.protocol, serverHostSha256, target: metadata.target, version: metadata.version })) {
    throw new Error("Package build identity does not match its first-party artifacts");
  }
  await requireFile(join(packageRoot, "zeta-path", isWindows ? "rg.exe" : "rg"));
  if (metadata.javascriptRuntime?.kind === "packagedNode") {
    if (typeof metadata.components.node !== "object" || metadata.components.node === null) {
      throw new Error("Packaged Node runtime metadata is missing");
    }
    await requireFile(join(packageRoot, "zeta-resources", "node", "bin", isWindows ? "node.exe" : "node"));
    await requireFile(join(packageRoot, "zeta-resources", "licenses", "node", "LICENSE"));
  } else if (metadata.javascriptRuntime?.kind === "hostProvidedNode") {
    if (metadata.components.node !== undefined) {
      throw new Error("Host-provided runtime package contains Node metadata");
    }
    if (await pathExists(join(packageRoot, "zeta-resources", "node")) || await pathExists(join(packageRoot, "zeta-resources", "licenses", "node"))) {
      throw new Error("Host-provided runtime package contains a standalone Node payload");
    }
  } else {
    throw new Error("Invalid package JavaScript runtime declaration");
  }
  await requireFile(join(packageRoot, "zeta-resources", "licenses", "ripgrep", "LICENSE-MIT"));
  await requireFile(join(packageRoot, "zeta-resources", "licenses", "ripgrep", "UNLICENSE"));
  await requireFile(join(packageRoot, "zeta-resources", "licenses", "vscode", "LICENSE.txt"));
  await requireFile(join(packageRoot, "zeta-resources", "product-services", "product-services.json"));
  await requireFile(join(packageRoot, "zeta-resources", "product-services", "marketplace-root.json"));
  if (isWindows) {
    await requireFile(join(packageRoot, "zeta-resources", "zeta-command-runner.exe"));
    await requireFile(join(packageRoot, "zeta-resources", "zeta-windows-sandbox-setup.exe"));
  }
  if (platform === "linux") {
    await requireFile(join(packageRoot, "zeta-resources", "bwrap"));
    await requireFile(join(packageRoot, "zeta-resources", "licenses", "bubblewrap", "COPYING"));
  }
  const extensionEntries = await readdir(join(packageRoot, "zeta-resources", "extensions"), { withFileTypes: true });
  if (extensionEntries.length === 0) {
    throw new Error("Package contains no built-in extensions");
  }
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
  if (metadata.remoteRuntimeCatalog !== undefined) {
    if (metadata.remoteRuntimeCatalog.trustBinding !== "signedProductPackage" || !/^[a-f0-9]{64}$/.test(metadata.remoteRuntimeCatalog.sha256)) {
      throw new Error("Invalid Remote runtime catalog package binding");
    }
    if (metadata.remoteRuntimeCatalog.path === "zeta-remote-runtimes/catalog.json" && metadata.remoteRuntimeCatalog.url === undefined) {
      const catalog = JSON.parse(await readFile(join(packageRoot, "zeta-remote-runtimes", "catalog.json"), "utf8"));
      if (catalog.formatVersion !== 1 || !Array.isArray(catalog.artifacts) || catalog.artifacts.length === 0) throw new Error("Invalid packaged Remote runtime catalog");
    } else if (metadata.remoteRuntimeCatalog.path === undefined && typeof metadata.remoteRuntimeCatalog.url === "string") {
      validateRemoteRuntimeCatalogUrl(metadata.remoteRuntimeCatalog.url);
    } else {
      throw new Error("Invalid Remote runtime catalog package source");
    }
  }
}

export async function replaceDirectoryAtomically(output: string, build: (staging: string) => Promise<void>): Promise<void> {
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
    } catch (error: unknown) {
      if (!isErrorCode(error, "ENOENT")) {
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

export async function prepareDevelopmentPackage(
  javascriptRuntime: JavaScriptRuntimeKind = "host-provided-node",
  remoteRuntimeBundle?: string,
  remoteRuntimeRelease?: RemoteRuntimeRelease,
): Promise<void> {
  if (!javascriptRuntimeKinds.has(javascriptRuntime)) {
    throw new Error(`Unsupported JavaScript runtime package mode: ${javascriptRuntime}`);
  }
  const target = hostTarget();
  const isWindows = process.platform === "win32";
  const executables = await buildFirstPartyExecutables(process.platform);
  const ripgrep = await resolveRipgrep(target, isWindows);
  const node = javascriptRuntime === "packaged-node" ? await resolveNode(target, isWindows) : undefined;
  await replaceDirectoryAtomically(outputDirectory, (staging) => assemblePackage(
    staging,
    target,
    process.platform,
    executables,
    ripgrep,
    node,
    remoteRuntimeBundle,
    remoteRuntimeRelease,
  ));
  console.log(`Prepared Zeta development package (${javascriptRuntime}) at ${outputDirectory}`);
}

const isMain = process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url);
if (isMain) {
  const options = parsePackageOptions(process.argv.slice(2));
  prepareDevelopmentPackage(options.javascriptRuntime, options.remoteRuntimeBundle, options.remoteRuntimeRelease).catch((error) => {
    console.error(error instanceof Error ? error.message : error);
    process.exitCode = 1;
  });
}

async function requireComponentDigest(metadata: PackageMetadata, name: string, path: string): Promise<void> {
  const component = metadata.components[name];
  if (typeof component !== "object" || component === null || !("binarySha256" in component)) {
    throw new Error(`Package component metadata is missing: ${name}`);
  }
  const expected = (component as { readonly binarySha256?: unknown }).binarySha256;
  if (typeof expected !== "string" || !/^[a-f0-9]{64}$/.test(expected) || await sha256(path) !== expected) {
    throw new Error(`Package component digest does not match: ${name}`);
  }
}

function packageBuildId(identity: {
  readonly appServerDaemonSha256: string;
  readonly codeModeHostSha256: string;
  readonly protocol: PackageMetadata["protocol"];
  readonly serverHostSha256: string;
  readonly target: string;
  readonly version: string;
}): string {
  return `sha256:${createHash("sha256").update(JSON.stringify(identity)).digest("hex")}`;
}

function isErrorCode(error: unknown, code: string): boolean {
  return error instanceof Error && "code" in error && error.code === code;
}
