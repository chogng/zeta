import { type ChildProcessWithoutNullStreams, spawn } from "node:child_process";
import { existsSync } from "node:fs";
import { mkdir } from "node:fs/promises";
import type { IncomingMessage } from "node:http";
import { dirname, join, resolve } from "node:path";
import type { Writable } from "node:stream";
import type { Plugin, WebSocket, WebSocketClient } from "vite";
import { desktopBuildPath, developmentZetaPackagePath } from "../lib/paths.ts";

export const WEB_APP_SERVER_PROTOCOL_VERSION = 1;
export const WEB_APP_SERVER_CONNECT_EVENT = "zeta:app-server:connect";
export const WEB_APP_SERVER_CONNECTED_EVENT = "zeta:app-server:connected";
export const WEB_APP_SERVER_DISCONNECT_EVENT = "zeta:app-server:disconnect";
export const WEB_APP_SERVER_FRAME_EVENT = "zeta:app-server:frame";
export const WEB_APP_SERVER_CLOSED_EVENT = "zeta:app-server:closed";

const MAX_FRAME_BYTES = 320 * 1024 * 1024;
const MAX_STDERR_BYTES = 65_536;
const MAX_PENDING_WRITES = 128;

interface WebAppServerPluginOptions {
  readonly desktopRoot?: string;
  readonly repositoryRoot?: string;
  readonly workspaceRoot?: string;
  readonly profileRoot?: string;
  readonly executable?: string;
  readonly ripgrep?: string;
}

interface AppServerEnvironmentOptions {
  readonly profileRoot: string;
  readonly ripgrep: string;
  readonly workspaceRoot: string;
  readonly sourceEnvironment?: NodeJS.ProcessEnv;
  readonly platform?: NodeJS.Platform;
}

/**
 * Creates the loopback-only development bridge between Vite's authenticated
 * HMR WebSocket and one connection carrier per browser, sharing the profile daemon.
 */
export function webAppServerVitePlugin(options: WebAppServerPluginOptions = {}): Plugin {
  const desktopRoot = resolve(options.desktopRoot ?? resolve(import.meta.dirname, "../../zeta-ts"));
  const repositoryRoot = resolve(options.repositoryRoot ?? resolve(desktopRoot, ".."));
  const workspaceRoot = resolve(options.workspaceRoot ?? process.env.ZETA_WORKSPACE_ROOT ?? repositoryRoot);
  const profileRoot = resolve(options.profileRoot ?? process.env.ZETA_WEB_APP_SERVER_PROFILE ?? desktopBuildPath(repositoryRoot, "dev", "web-profile"));
  let packageRoot: string | undefined;
  const developmentPackage = () => packageRoot ??= developmentZetaPackagePath(repositoryRoot, "packaged-node");
  const executable = resolve(options.executable ?? join(
    developmentPackage(),
    "bin",
    process.platform === "win32" ? "zeta-server.exe" : "zeta-server",
  ));
  const ripgrep = resolve(options.ripgrep ?? process.env.ZETA_RG_PATH ?? join(
    developmentPackage(),
    "zeta-path",
    process.platform === "win32" ? "rg.exe" : "rg",
  ));
  const sessions = new Map<WebSocketClient, Promise<WebAppServerSession>>();

  return {
    name: "zeta-web-app-server",
    apply: "serve",
    configureServer(server) {
      assertLoopbackServer(server.config.server.host);
      const onConnection = (socket: WebSocket, request: IncomingMessage): void => {
        if (!isAllowedDevOrigin(request?.headers?.origin, request?.headers?.host)) {
          socket.close(1008, "Zeta Web development bridge requires a same-origin loopback client");
        }
      };
      const onConnect = (_payload: unknown, client: WebSocketClient): void => {
        void connectClient(client).catch((error) => {
          closeClient(client, error instanceof Error ? error.message : "App Server startup failed");
        });
      };
      const onFrame = (payload: unknown, client: WebSocketClient): void => {
        const pending = sessions.get(client);
        if (!pending) {
          send(client, WEB_APP_SERVER_CLOSED_EVENT, { message: "App Server bridge is not connected" });
          return;
        }
        void pending.then((session) => session.send(readFrame(payload))).catch((error: unknown) => {
          closeClient(client, error instanceof Error ? error.message : "App Server bridge failed");
        });
      };
      const onDisconnect = (_payload: unknown, client: WebSocketClient): void => closeClient(client, "Browser disconnected");

      server.ws.on("connection", onConnection);
      server.ws.on(WEB_APP_SERVER_CONNECT_EVENT, onConnect);
      server.ws.on(WEB_APP_SERVER_FRAME_EVENT, onFrame);
      server.ws.on(WEB_APP_SERVER_DISCONNECT_EVENT, onDisconnect);
      server.httpServer?.once("close", () => {
        for (const client of sessions.keys()) closeClient(client, "Vite development server stopped");
      });

      async function connectClient(client: WebSocketClient): Promise<WebAppServerSession> {
        let pending = sessions.get(client);
        if (!pending) {
          pending = createSession(client);
          sessions.set(client, pending);
          client.socket?.once?.("close", () => closeClient(client, "Browser connection closed"));
        }
        const session = await pending;
        send(client, WEB_APP_SERVER_CONNECTED_EVENT, {
          protocolVersion: WEB_APP_SERVER_PROTOCOL_VERSION,
          workspaceId: `web-dev:${workspaceRoot}`,
          workspaceRoot,
        });
        return session;
      }

      async function createSession(client: WebSocketClient): Promise<WebAppServerSession> {
        if (!existsSync(executable)) {
          throw new Error(`Packaged Zeta binary is missing: ${executable}`);
        }
        if (!existsSync(ripgrep)) {
          throw new Error(`Packaged ripgrep binary is missing: ${ripgrep}`);
        }
        await mkdir(profileRoot, { recursive: true });
        const child = spawn(executable, ["app-server", "connect"], {
          cwd: workspaceRoot,
          env: { ...appServerEnvironment({ profileRoot, ripgrep, workspaceRoot }), ZETA_APP_SERVER_DAEMON_PATH: join(dirname(executable), process.platform === 'win32' ? 'zeta-app-server-daemon.exe' : 'zeta-app-server-daemon') },
          shell: false,
          stdio: "pipe",
          windowsHide: true,
        });
        const session = new WebAppServerSession(child, (frame) => {
          send(client, WEB_APP_SERVER_FRAME_EVENT, { frame });
        }, (message: string) => {
          send(client, WEB_APP_SERVER_CLOSED_EVENT, { message });
          sessions.delete(client);
        });
        child.once("error", (error) => session.fail(`Could not start App Server: ${error.message}`));
        return session;
      }

      function closeClient(client: WebSocketClient, reason: string): void {
        const pending = sessions.get(client);
        if (!pending) return;
        sessions.delete(client);
        void pending.then((session) => session.close(reason)).catch(() => {});
      }
    },
  };
}

export function isAllowedDevOrigin(origin: unknown, host: unknown): boolean {
  if (typeof origin !== "string" || typeof host !== "string") return false;
  let parsed;
  try {
    parsed = new URL(origin);
  } catch {
    return false;
  }
  return (
    (parsed.protocol === "http:" || parsed.protocol === "https:") &&
    parsed.host === host &&
    isLoopbackHostname(parsed.hostname) &&
    !parsed.username &&
    !parsed.password
  );
}

export class JsonlFrameDecoder {
  private readonly onFrame: (frame: string) => void;
  private readonly onError: (error: Error) => void;
  private readonly maxFrameBytes: number;
  private parts: Buffer[] = [];
  private bytes = 0;
  private failed = false;

  constructor(onFrame: (frame: string) => void, onError: (error: Error) => void, maxFrameBytes = MAX_FRAME_BYTES) {
    this.onFrame = onFrame;
    this.onError = onError;
    this.maxFrameBytes = maxFrameBytes;
  }

  public accept(chunk: string | Uint8Array): void {
    if (this.failed) return;
    const bytes = Buffer.isBuffer(chunk) ? chunk : Buffer.from(chunk);
    let start = 0;
    for (let index = 0; index < bytes.length; index += 1) {
      if (bytes[index] !== 0x0a) continue;
      this.append(bytes.subarray(start, index));
      if (this.failed) return;
      this.emit();
      if (this.failed) return;
      start = index + 1;
    }
    this.append(bytes.subarray(start));
  }

  public end(): void {
    if (!this.failed && this.bytes > 0) this.fail("App Server stdout ended with an unterminated JSONL frame");
  }

  private append(part: Uint8Array): void {
    if (part.length === 0) return;
    if (this.bytes + part.length > this.maxFrameBytes) {
      this.fail(`App Server JSONL frame exceeds ${this.maxFrameBytes} bytes`);
      return;
    }
    this.parts.push(Buffer.from(part));
    this.bytes += part.length;
  }

  private emit(): void {
    if (this.bytes === 0) {
      this.fail("App Server emitted an empty JSONL frame");
      return;
    }
    const frame = Buffer.concat(this.parts, this.bytes);
    this.parts = [];
    this.bytes = 0;
    if (frame.at(-1) === 0x0d) {
      this.fail("App Server JSONL framing must use LF, not CRLF");
      return;
    }
    try {
      this.onFrame(new TextDecoder("utf-8", { fatal: true }).decode(frame));
    } catch {
      this.fail("App Server emitted invalid UTF-8");
    }
  }

  private fail(message: string): void {
    if (this.failed) return;
    this.failed = true;
    this.onError(new Error(message));
  }
}

class WebAppServerSession {
  private readonly child: ChildProcessWithoutNullStreams;
  private readonly onClose: (reason: string) => void;
  private readonly decoder: JsonlFrameDecoder;
  private pendingWrites = 0;
  private writeTail: Promise<void> = Promise.resolve();
  private stderr = Buffer.alloc(0);
  private closed = false;

  constructor(child: ChildProcessWithoutNullStreams, onFrame: (frame: string) => void, onClose: (reason: string) => void) {
    this.child = child;
    this.onClose = onClose;
    this.decoder = new JsonlFrameDecoder(onFrame, (error) => this.fail(error.message));
    child.stdout.on("data", (chunk: Buffer) => this.decoder.accept(chunk));
    child.stdout.once("end", () => this.decoder.end());
    child.stderr.on("data", (chunk: Buffer) => this.captureStderr(chunk));
    child.once("exit", (code, signal) => {
      const reason = signal
        ? `App Server exited from signal ${signal}`
        : `App Server exited with code ${code ?? "unknown"}`;
      this.finish(this.diagnosticMessage(reason));
    });
  }

  public send(frame: string): Promise<void> {
    validateFrame(frame);
    if (this.closed) return Promise.reject(new Error("App Server bridge is closed"));
    if (this.pendingWrites >= MAX_PENDING_WRITES) {
      this.fail("App Server bridge write queue is full");
      return Promise.reject(new Error("App Server bridge write queue is full"));
    }
    this.pendingWrites += 1;
    const write = this.writeTail.then(() => writeFrame(this.child.stdin, `${frame}\n`));
    this.writeTail = write.catch(() => {});
    return write.finally(() => {
      this.pendingWrites -= 1;
    });
  }

  public close(reason: string): void {
    if (this.closed) return;
    this.finish(reason);
    if (this.child.exitCode !== null || this.child.signalCode !== null) return;
    this.child.kill("SIGTERM");
    const timeout = setTimeout(() => {
      if (this.child.exitCode === null && this.child.signalCode === null) this.child.kill("SIGKILL");
    }, 2_000);
    timeout.unref();
  }

  public fail(reason: string): void {
    this.close(this.diagnosticMessage(reason));
  }

  private finish(reason: string): void {
    if (this.closed) return;
    this.closed = true;
    this.onClose(reason);
  }

  private captureStderr(chunk: string | Uint8Array): void {
    const bytes = Buffer.isBuffer(chunk) ? chunk : Buffer.from(chunk);
    const combined = Buffer.concat([this.stderr, bytes]);
    this.stderr = combined.subarray(Math.max(0, combined.length - MAX_STDERR_BYTES));
  }

  private diagnosticMessage(reason: string): string {
    const diagnostics = redactSecrets(this.stderr.toString("utf8").trim());
    return diagnostics ? `${reason}: ${diagnostics}`.slice(0, 8_000) : reason;
  }
}

function assertLoopbackServer(host: string | boolean | undefined): void {
  const value = host === true || host === undefined ? "127.0.0.1" : String(host);
  if (!isLoopbackHostname(value)) {
    throw new Error(`Zeta Web development bridge must bind to loopback, received: ${value}`);
  }
}

const COMMON_HOST_ENVIRONMENT_KEYS = ["HOME", "LANG", "LOGNAME", "PATH", "SHELL", "TEMP", "TMP", "TMPDIR", "USER", "ZETA_PRODUCT_SERVICES_PATH"];
const POSIX_HOST_ENVIRONMENT_KEYS = ["XDG_CACHE_HOME", "XDG_CONFIG_HOME", "XDG_DATA_HOME", "XDG_RUNTIME_DIR", "XDG_STATE_HOME"];
const WINDOWS_HOST_ENVIRONMENT_KEYS = ["ALLUSERSPROFILE", "APPDATA", "COMMONPROGRAMFILES", "COMMONPROGRAMFILES(X86)", "COMSPEC", "HOMEDRIVE", "HOMEPATH", "LOCALAPPDATA", "NUMBER_OF_PROCESSORS", "OS", "PATHEXT", "PROCESSOR_ARCHITECTURE", "PROCESSOR_IDENTIFIER", "PROCESSOR_LEVEL", "PROCESSOR_REVISION", "PROGRAMDATA", "PROGRAMFILES", "PROGRAMFILES(X86)", "PROGRAMW6432", "PSMODULEPATH", "PUBLIC", "SYSTEMDRIVE", "SYSTEMROOT", "USERDOMAIN", "USERNAME", "USERPROFILE", "WINDIR"];

export function appServerEnvironment({ profileRoot, ripgrep, workspaceRoot, sourceEnvironment = process.env, platform = process.platform }: AppServerEnvironmentOptions): NodeJS.ProcessEnv {
  const environment: NodeJS.ProcessEnv = {};
  const hostKeys = platform === "win32" ? [...COMMON_HOST_ENVIRONMENT_KEYS, ...WINDOWS_HOST_ENVIRONMENT_KEYS] : [...COMMON_HOST_ENVIRONMENT_KEYS, ...POSIX_HOST_ENVIRONMENT_KEYS];
  for (const key of hostKeys) {
    const value = environmentValue(sourceEnvironment, key, platform);
    if (typeof value === "string" && !value.includes("\0")) environment[key] = value;
  }
  for (const [key, value] of Object.entries(sourceEnvironment)) {
    if (!key.toUpperCase().startsWith("LC_") || key.includes("=") || key.includes("\0") || typeof value !== "string" || value.includes("\0")) continue;
    environment[platform === "win32" ? key.toUpperCase() : key] = value;
  }
  return {
    ...environment,
    ZETA_PROFILE_ROOT: profileRoot,
    ZETA_RG_PATH: ripgrep,
    ZETA_WORKSPACE_ROOT: workspaceRoot,
  };
}

function environmentValue(source: NodeJS.ProcessEnv, key: string, platform: NodeJS.Platform): string | undefined {
  if (platform !== "win32") return source[key];
  return Object.entries(source).find(([candidate]) => candidate.toUpperCase() === key)?.[1];
}

function isLoopbackHostname(hostname: string): boolean {
  return hostname === "127.0.0.1" || hostname === "localhost" || hostname === "::1" || hostname === "[::1]";
}

function readFrame(payload: unknown): string {
  if (!payload || typeof payload !== "object" || !("frame" in payload) || typeof payload.frame !== "string") {
    throw new TypeError("Web App Server bridge frame is invalid");
  }
  return payload.frame;
}

function validateFrame(frame: string): void {
  if (frame.includes("\n") || frame.includes("\r")) throw new Error("JSONL frame must not contain CR or LF");
  if (Buffer.byteLength(frame, "utf8") > MAX_FRAME_BYTES) throw new Error(`JSONL frame exceeds ${MAX_FRAME_BYTES} bytes`);

}

function writeFrame(stream: Writable, frame: string): Promise<void> {
  return new Promise<void>((resolvePromise, reject) => {
    let callbackComplete = false;
    let drainComplete = true;
    const settle = () => {
      if (callbackComplete && drainComplete) resolvePromise();
    };
    const accepted = stream.write(frame, "utf8", (error) => {
      if (error) {
        reject(error);
        return;
      }
      callbackComplete = true;
      settle();
    });
    if (!accepted) {
      drainComplete = false;
      stream.once("drain", () => {
        drainComplete = true;
        settle();
      });
    }
  });
}

function send(client: WebSocketClient, event: string, payload: unknown): void {
  try {
    client.send(event, payload);
  } catch {
    // Socket teardown owns process cleanup.
  }
}

function redactSecrets(value: string): string {
  return value
    .replace(/(bearer\s+)[^\s"',}]+/giu, "$1[REDACTED]")
    .replace(/((?:api[-_ ]?key|authorization|token|secret|password)["']?\s*[:=]\s*["']?)[^"'\s,}]+/giu, "$1[REDACTED]")
    .replace(/\bsk-[A-Za-z0-9_-]{8,}\b/gu, "[REDACTED]");
}
