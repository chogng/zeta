import { markAsDisposed, trackDisposable, toDisposable, } from "../../../base/common/lifecycle.js";
export const DEFAULT_MAX_JSONL_FRAME_BYTES = 1_048_576;
export const DEFAULT_MAX_STDERR_BYTES = 65_536;
export const DEFAULT_MAX_PENDING_WRITES = 128;
/**
 * Owns bounded JSONL framing and stream lifecycle for one spawned App Server process.
 *
 * This transport deliberately has no knowledge of JSON-RPC methods or request identifiers.
 */
export class ChildProcessJsonlTransport {
    process;
    #maxFrameBytes;
    #maxStderrBytes;
    #maxPendingWrites;
    #closeTimeoutMs;
    #frameListeners = new Set();
    #closeListeners = new Set();
    #writeRejectors = new Set();
    #frameParts = [];
    #frameBytes = 0;
    #stderr = Buffer.alloc(0);
    #terminalError;
    #pendingWrites = 0;
    #writeTail = Promise.resolve();
    #closePromise;
    constructor(process, options = {}) {
        this.process = process;
        this.#maxFrameBytes = positiveInteger(options.maxFrameBytes, DEFAULT_MAX_JSONL_FRAME_BYTES, "maxFrameBytes");
        this.#maxStderrBytes = positiveInteger(options.maxStderrBytes, DEFAULT_MAX_STDERR_BYTES, "maxStderrBytes");
        this.#maxPendingWrites = positiveInteger(options.maxPendingWrites, DEFAULT_MAX_PENDING_WRITES, "maxPendingWrites");
        this.#closeTimeoutMs = positiveInteger(options.closeTimeoutMs, 2_000, "closeTimeoutMs");
        process.stdout.on("data", this.#onStdoutData);
        process.stderr.on("data", this.#onStderrData);
        process.stdout.once("end", this.#onStdoutEnd);
        process.stdout.once("error", this.#onStdoutError);
        process.stderr.once("error", this.#onStderrError);
        process.stdin.once("error", this.#onStdinError);
        process.once("error", this.#onProcessError);
        process.once("exit", this.#onProcessExit);
        trackDisposable(this);
    }
    onFrame(listener) {
        this.#frameListeners.add(listener);
        return toDisposable(() => this.#frameListeners.delete(listener));
    }
    onClose(listener) {
        if (this.#terminalError) {
            const error = this.#terminalError;
            let active = true;
            queueMicrotask(() => {
                if (active)
                    listener(error);
            });
            return toDisposable(() => {
                active = false;
            });
        }
        this.#closeListeners.add(listener);
        return toDisposable(() => this.#closeListeners.delete(listener));
    }
    send(frame) {
        if (this.#terminalError)
            return Promise.reject(this.#terminalError);
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
        this.#writeTail = write.catch(() => { });
        return write.finally(() => {
            this.#pendingWrites -= 1;
        });
    }
    diagnostics() {
        return redactSecrets(this.#stderr.toString("utf8"));
    }
    close() {
        if (this.#closePromise)
            return this.#closePromise;
        if (this.process.exitCode !== null || this.process.signalCode !== null) {
            this.#finish(new Error("Zeta app-server connection closed"));
            this.#closePromise = Promise.resolve();
            return this.#closePromise;
        }
        this.#closePromise = new Promise((resolve) => {
            let settled = false;
            const finish = () => {
                if (settled)
                    return;
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
            if (!this.process.kill("SIGTERM"))
                finish();
        });
        return this.#closePromise;
    }
    dispose() {
        void this.close().catch(() => {
            // Explicit close callers can observe errors; disposal is best-effort.
        });
    }
    [Symbol.dispose]() {
        this.dispose();
    }
    #onStdoutData = (chunk) => {
        if (this.#terminalError)
            return;
        const bytes = Buffer.isBuffer(chunk) ? chunk : Buffer.from(chunk, "utf8");
        let start = 0;
        for (let index = 0; index < bytes.length; index += 1) {
            if (bytes[index] !== 0x0a)
                continue;
            this.#appendFramePart(bytes.subarray(start, index));
            if (this.#terminalError)
                return;
            this.#emitFrame();
            if (this.#terminalError)
                return;
            start = index + 1;
        }
        this.#appendFramePart(bytes.subarray(start));
    };
    #onStderrData = (chunk) => {
        const bytes = Buffer.isBuffer(chunk) ? chunk : Buffer.from(chunk, "utf8");
        if (bytes.length >= this.#maxStderrBytes) {
            this.#stderr = Buffer.from(bytes.subarray(bytes.length - this.#maxStderrBytes));
            return;
        }
        const retained = Math.max(0, this.#maxStderrBytes - bytes.length);
        const prior = this.#stderr.subarray(Math.max(0, this.#stderr.length - retained));
        this.#stderr = Buffer.concat([prior, bytes], prior.length + bytes.length);
    };
    #onStdoutEnd = () => {
        if (this.#terminalError)
            return;
        const message = this.#frameBytes === 0
            ? "App Server stdout ended"
            : "App Server stdout ended with an unterminated JSONL frame";
        this.#fail(new Error(message));
    };
    #onStdoutError = (error) => {
        this.#fail(streamError("stdout", error));
    };
    #onStderrError = (error) => {
        this.#fail(streamError("stderr", error));
    };
    #onStdinError = (error) => {
        this.#fail(streamError("stdin", error));
    };
    #onProcessError = (error) => {
        this.#fail(new Error(`App Server process error: ${error.message}`));
    };
    #onProcessExit = (code, signal) => {
        this.#finish(new Error(signal
            ? `Zeta app-server exited from signal ${signal}`
            : `Zeta app-server exited with code ${code ?? "unknown"}`));
    };
    #appendFramePart(part) {
        if (part.length === 0)
            return;
        if (this.#frameBytes + part.length > this.#maxFrameBytes) {
            this.#fail(new Error(`JSONL frame exceeds ${this.#maxFrameBytes} bytes`));
            return;
        }
        this.#frameParts.push(Buffer.from(part));
        this.#frameBytes += part.length;
    }
    #emitFrame() {
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
        let frame;
        try {
            frame = new TextDecoder("utf-8", { fatal: true }).decode(bytes);
        }
        catch {
            this.#fail(new Error("App Server emitted invalid UTF-8"));
            return;
        }
        for (const listener of this.#frameListeners) {
            try {
                listener(frame);
            }
            catch (error) {
                this.#fail(error instanceof Error ? error : new Error("JSONL frame listener failed"));
                return;
            }
        }
    }
    #writeFrame(frame) {
        if (this.#terminalError)
            return Promise.reject(this.#terminalError);
        return new Promise((resolve, reject) => {
            let callbackComplete = false;
            let drainComplete = true;
            let settled = false;
            const cleanup = () => {
                this.process.stdin.off("drain", onDrain);
                this.#writeRejectors.delete(rejectWrite);
            };
            const settle = () => {
                if (settled || !callbackComplete || !drainComplete)
                    return;
                settled = true;
                cleanup();
                resolve();
            };
            const rejectWrite = (error) => {
                if (settled)
                    return;
                settled = true;
                cleanup();
                reject(error);
            };
            const onDrain = () => {
                drainComplete = true;
                settle();
            };
            this.#writeRejectors.add(rejectWrite);
            try {
                const accepted = this.process.stdin.write(frame, "utf8", (error) => {
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
            }
            catch (error) {
                rejectWrite(error instanceof Error ? error : new Error("App Server stdin write failed"));
            }
        });
    }
    #fail(error) {
        this.#finish(error);
        if (this.process.exitCode === null && this.process.signalCode === null) {
            this.process.kill("SIGTERM");
        }
    }
    #finish(error) {
        if (this.#terminalError)
            return;
        this.#terminalError = error;
        this.process.stdout.off("data", this.#onStdoutData);
        this.process.stdout.off("end", this.#onStdoutEnd);
        this.process.stdout.off("error", this.#onStdoutError);
        this.process.stderr.off("data", this.#onStderrData);
        this.process.stderr.off("error", this.#onStderrError);
        this.process.stdin.off("error", this.#onStdinError);
        this.process.off("error", this.#onProcessError);
        this.process.off("exit", this.#onProcessExit);
        for (const reject of this.#writeRejectors)
            reject(error);
        this.#writeRejectors.clear();
        for (const listener of this.#closeListeners) {
            try {
                listener(error);
            }
            catch {
                // One close observer cannot block transport-wide teardown.
            }
        }
        this.#frameListeners.clear();
        this.#closeListeners.clear();
        markAsDisposed(this);
    }
}
function positiveInteger(value, fallback, name) {
    const resolved = value ?? fallback;
    if (!Number.isSafeInteger(resolved) || resolved <= 0) {
        throw new Error(`${name} must be a positive safe integer`);
    }
    return resolved;
}
function streamError(stream, error) {
    return new Error(`App Server ${stream} error: ${error.message}`);
}
function redactSecrets(value) {
    return value
        .replace(/(bearer\s+)[^\s"',}]+/giu, "$1[REDACTED]")
        .replace(/((?:api[-_ ]?key|authorization|token|secret|password)["']?\s*[:=]\s*["']?)[^"'\s,}]+/giu, "$1[REDACTED]")
        .replace(/\bsk-[A-Za-z0-9_-]{8,}\b/gu, "[REDACTED]");
}
