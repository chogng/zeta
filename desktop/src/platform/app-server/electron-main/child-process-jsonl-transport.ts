import type { ChildProcessWithoutNullStreams } from "node:child_process";

export const DEFAULT_MAX_JSONL_FRAME_BYTES = 1_048_576;
export const DEFAULT_MAX_STDERR_BYTES = 65_536;
export const DEFAULT_MAX_PENDING_WRITES = 128;

export interface ChildProcessJsonlTransportOptions {
  maxFrameBytes?: number;
  maxStderrBytes?: number;
  maxPendingWrites?: number;
  closeTimeoutMs?: number;
}

type FrameListener = (frame: string) => void;
type CloseListener = (error: Error) => void;

/**
 * Owns bounded JSONL framing and stream lifecycle for one spawned App Server process.
 *
 * This transport deliberately has no knowledge of JSON-RPC methods or request identifiers.
 */
export class ChildProcessJsonlTransport {
  readonly #maxFrameBytes: number;
  readonly #maxStderrBytes: number;
  readonly #maxPendingWrites: number;
  readonly #closeTimeoutMs: number;
  readonly #frameListeners = new Set<FrameListener>();
  readonly #closeListeners = new Set<CloseListener>();
  readonly #writeRejectors = new Set<(error: Error) => void>();
  #frameParts: Buffer[] = [];
  #frameBytes = 0;
  #stderr = Buffer.alloc(0);
  #terminalError?: Error;
  #pendingWrites = 0;
  #writeTail: Promise<void> = Promise.resolve();
  #closePromise?: Promise<void>;

  constructor(
    readonly process: ChildProcessWithoutNullStreams,
    options: ChildProcessJsonlTransportOptions = {},
  ) {
    this.#maxFrameBytes = positiveInteger(
      options.maxFrameBytes,
      DEFAULT_MAX_JSONL_FRAME_BYTES,
      "maxFrameBytes",
    );
    this.#maxStderrBytes = positiveInteger(
      options.maxStderrBytes,
      DEFAULT_MAX_STDERR_BYTES,
      "maxStderrBytes",
    );
    this.#maxPendingWrites = positiveInteger(
      options.maxPendingWrites,
      DEFAULT_MAX_PENDING_WRITES,
      "maxPendingWrites",
    );
    this.#closeTimeoutMs = positiveInteger(options.closeTimeoutMs, 2_000, "closeTimeoutMs");

    process.stdout.on("data", this.#onStdoutData);
    process.stderr.on("data", this.#onStderrData);
    process.stdout.once("end", this.#onStdoutEnd);
    process.stdout.once("error", (error: Error) => this.#fail(streamError("stdout", error)));
    process.stderr.once("error", (error: Error) => this.#fail(streamError("stderr", error)));
    process.stdin.once("error", (error: Error) => this.#fail(streamError("stdin", error)));
    process.once("error", (error: Error) => this.#fail(new Error(`App Server process error: ${error.message}`)));
    process.once("exit", (code, signal) => {
      this.#finish(
        new Error(
          signal
            ? `Zeta app-server exited from signal ${signal}`
            : `Zeta app-server exited with code ${code ?? "unknown"}`,
        ),
      );
    });
  }

  onFrame(listener: FrameListener): () => void {
    this.#frameListeners.add(listener);
    return () => this.#frameListeners.delete(listener);
  }

  onClose(listener: CloseListener): () => void {
    if (this.#terminalError) {
      const error = this.#terminalError;
      queueMicrotask(() => listener(error));
      return () => {};
    }
    this.#closeListeners.add(listener);
    return () => this.#closeListeners.delete(listener);
  }

  send(frame: string): Promise<void> {
    if (this.#terminalError) return Promise.reject(this.#terminalError);
    if (frame.includes("\n") || frame.includes("\r")) {
      return Promise.reject(new Error("JSONL frame must not contain CR or LF"));
    }
    if (Buffer.byteLength(frame, "utf8") > this.#maxFrameBytes) {
      return Promise.reject(new Error(`JSONL frame exceeds ${this.#maxFrameBytes} bytes`));
    }
    if (this.#pendingWrites >= this.#maxPendingWrites) {
      return Promise.reject(new Error("JSONL transport write queue is full"));
    }

    this.#pendingWrites += 1;
    const write = this.#writeTail.then(() => this.#writeFrame(`${frame}\n`));
    this.#writeTail = write.catch(() => {});
    return write.finally(() => {
      this.#pendingWrites -= 1;
    });
  }

  diagnostics(): string {
    return redactSecrets(this.#stderr.toString("utf8"));
  }

  close(): Promise<void> {
    if (this.#closePromise) return this.#closePromise;
    if (this.process.exitCode !== null || this.process.signalCode !== null) {
      this.#finish(new Error("Zeta app-server connection closed"));
      this.#closePromise = Promise.resolve();
      return this.#closePromise;
    }

    this.#closePromise = new Promise<void>((resolve) => {
      let settled = false;
      const finish = (): void => {
        if (settled) return;
        settled = true;
        clearTimeout(timeout);
        resolve();
      };
      const timeout = setTimeout(() => {
        this.process.kill("SIGKILL");
        finish();
      }, this.#closeTimeoutMs);
      timeout.unref();
      this.process.once("exit", finish);
      this.#finish(new Error("Zeta app-server connection closed"));
      if (!this.process.kill("SIGTERM")) finish();
    });
    return this.#closePromise;
  }

  readonly #onStdoutData = (chunk: Buffer | string): void => {
    if (this.#terminalError) return;
    const bytes = Buffer.isBuffer(chunk) ? chunk : Buffer.from(chunk, "utf8");
    let start = 0;
    for (let index = 0; index < bytes.length; index += 1) {
      if (bytes[index] !== 0x0a) continue;
      this.#appendFramePart(bytes.subarray(start, index));
      if (this.#terminalError) return;
      this.#emitFrame();
      if (this.#terminalError) return;
      start = index + 1;
    }
    this.#appendFramePart(bytes.subarray(start));
  };

  readonly #onStderrData = (chunk: Buffer | string): void => {
    const bytes = Buffer.isBuffer(chunk) ? chunk : Buffer.from(chunk, "utf8");
    if (bytes.length >= this.#maxStderrBytes) {
      this.#stderr = Buffer.from(bytes.subarray(bytes.length - this.#maxStderrBytes));
      return;
    }
    const retained = Math.max(0, this.#maxStderrBytes - bytes.length);
    const prior = this.#stderr.subarray(Math.max(0, this.#stderr.length - retained));
    this.#stderr = Buffer.concat([prior, bytes], prior.length + bytes.length);
  };

  readonly #onStdoutEnd = (): void => {
    if (this.#terminalError) return;
    const message =
      this.#frameBytes === 0
        ? "App Server stdout ended"
        : "App Server stdout ended with an unterminated JSONL frame";
    this.#fail(new Error(message));
  };

  #appendFramePart(part: Buffer): void {
    if (part.length === 0) return;
    if (this.#frameBytes + part.length > this.#maxFrameBytes) {
      this.#fail(new Error(`JSONL frame exceeds ${this.#maxFrameBytes} bytes`));
      return;
    }
    this.#frameParts.push(Buffer.from(part));
    this.#frameBytes += part.length;
  }

  #emitFrame(): void {
    if (this.#frameBytes === 0) {
      this.#fail(new Error("App Server emitted an empty JSONL frame"));
      return;
    }
    const bytes = Buffer.concat(this.#frameParts, this.#frameBytes);
    this.#frameParts = [];
    this.#frameBytes = 0;
    if (bytes.at(-1) === 0x0d) {
      this.#fail(new Error("App Server JSONL framing must use LF, not CRLF"));
      return;
    }

    let frame: string;
    try {
      frame = new TextDecoder("utf-8", { fatal: true }).decode(bytes);
    } catch {
      this.#fail(new Error("App Server emitted invalid UTF-8"));
      return;
    }
    for (const listener of this.#frameListeners) {
      try {
        listener(frame);
      } catch (error) {
        this.#fail(
          error instanceof Error ? error : new Error("JSONL frame listener failed"),
        );
        return;
      }
    }
  }

  #writeFrame(frame: string): Promise<void> {
    if (this.#terminalError) return Promise.reject(this.#terminalError);
    return new Promise<void>((resolve, reject) => {
      let callbackComplete = false;
      let drainComplete = true;
      let settled = false;
      const cleanup = (): void => {
        this.process.stdin.off("drain", onDrain);
        this.#writeRejectors.delete(rejectWrite);
      };
      const settle = (): void => {
        if (settled || !callbackComplete || !drainComplete) return;
        settled = true;
        cleanup();
        resolve();
      };
      const rejectWrite = (error: Error): void => {
        if (settled) return;
        settled = true;
        cleanup();
        reject(error);
      };
      const onDrain = (): void => {
        drainComplete = true;
        settle();
      };
      this.#writeRejectors.add(rejectWrite);
      try {
        const accepted = this.process.stdin.write(frame, "utf8", (error?: Error | null) => {
          if (error) {
            rejectWrite(streamError("stdin", error));
            return;
          }
          callbackComplete = true;
          settle();
        });
        if (!accepted) {
          drainComplete = false;
          this.process.stdin.once("drain", onDrain);
        }
      } catch (error) {
        rejectWrite(error instanceof Error ? error : new Error("App Server stdin write failed"));
      }
    });
  }

  #fail(error: Error): void {
    this.#finish(error);
    if (this.process.exitCode === null && this.process.signalCode === null) {
      this.process.kill("SIGTERM");
    }
  }

  #finish(error: Error): void {
    if (this.#terminalError) return;
    this.#terminalError = error;
    this.process.stdout.off("data", this.#onStdoutData);
    this.process.stderr.off("data", this.#onStderrData);
    for (const reject of this.#writeRejectors) reject(error);
    this.#writeRejectors.clear();
    for (const listener of this.#closeListeners) listener(error);
    this.#closeListeners.clear();
  }
}

function positiveInteger(value: number | undefined, fallback: number, name: string): number {
  const resolved = value ?? fallback;
  if (!Number.isSafeInteger(resolved) || resolved <= 0) {
    throw new Error(`${name} must be a positive safe integer`);
  }
  return resolved;
}

function streamError(stream: string, error: Error): Error {
  return new Error(`App Server ${stream} error: ${error.message}`);
}

function redactSecrets(value: string): string {
  return value
    .replace(/(bearer\s+)[^\s"',}]+/giu, "$1[REDACTED]")
    .replace(
      /((?:api[-_ ]?key|authorization|token|secret|password)["']?\s*[:=]\s*["']?)[^"'\s,}]+/giu,
      "$1[REDACTED]",
    )
    .replace(/\bsk-[A-Za-z0-9_-]{8,}\b/gu, "[REDACTED]");
}
