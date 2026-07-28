import { spawn, } from "node:child_process";
import { existsSync } from "node:fs";
import { isAbsolute } from "node:path";
import { DisposableSlot, markAsDisposed, setDisposableOwner, trackDisposable, toDisposable, } from "../../../base/common/lifecycle.js";
import { AppServerClient } from "./app-server-client.js";
import { AppServerSession, } from "./app-server-session.js";
import { JsonRpcPeer } from "./json-rpc-peer.js";
/**
 * Supervises App Server process/session replacement without replaying application requests.
 */
export class AppServerSupervisor {
    options;
    #spawnProcess;
    #fileExists;
    #wait;
    #stateListeners = new Set();
    #notificationListeners = new Set();
    #sessionNotification = new DisposableSlot();
    #maxRestartAttempts;
    #initialRestartDelayMs;
    #maxRestartDelayMs;
    #state = "stopped";
    #process;
    #session;
    #generation = 0;
    #restartAttempts = 0;
    #stopping = false;
    #disposed = false;
    #lastDiagnostics = "";
    constructor(options) {
        this.options = options;
        if (!isAbsolute(options.executable)) {
            throw new Error("App Server executable path must be absolute");
        }
        const allowedEnvironmentKeys = new Set(options.allowedEnvironmentKeys ?? ["PATH", "ZETA_STATE_ROOT"]);
        for (const key of Object.keys(options.environment)) {
            if (!allowedEnvironmentKeys.has(key)) {
                throw new Error(`App Server environment variable is not allowed: ${key}`);
            }
        }
        this.#maxRestartAttempts = nonNegativeInteger(options.maxRestartAttempts, 3, "maxRestartAttempts");
        this.#initialRestartDelayMs = positiveInteger(options.initialRestartDelayMs, 250, "initialRestartDelayMs");
        this.#maxRestartDelayMs = positiveInteger(options.maxRestartDelayMs, 2_000, "maxRestartDelayMs");
        this.#spawnProcess = options.spawnProcess ?? defaultSpawn;
        this.#fileExists = options.fileExists ?? existsSync;
        this.#wait = options.wait ?? wait;
        trackDisposable(this);
        setDisposableOwner(this.#sessionNotification, this);
    }
    get state() {
        return this.#state;
    }
    onStateChange(listener) {
        this.#stateListeners.add(listener);
        return toDisposable(() => this.#stateListeners.delete(listener));
    }
    onNotification(listener) {
        this.#notificationListeners.add(listener);
        return toDisposable(() => this.#notificationListeners.delete(listener));
    }
    async start() {
        if (this.#disposed) {
            throw new Error("Cannot start a disposed App Server supervisor");
        }
        if (this.#state !== "stopped") {
            throw new Error(`Cannot start App Server supervisor from ${this.#state}`);
        }
        if (!this.#fileExists(this.options.executable)) {
            throw new Error(`Packaged Zeta binary is missing: ${this.options.executable}`);
        }
        this.#stopping = false;
        this.#restartAttempts = 0;
        let lastError;
        for (let attempt = 0; attempt <= this.#maxRestartAttempts; attempt += 1) {
            if (attempt > 0) {
                this.#setState("restarting");
                await this.#wait(this.#restartDelay(attempt - 1));
            }
            try {
                await this.#launch();
                return;
            }
            catch (error) {
                lastError = error;
                this.#setState("crashed");
            }
        }
        throw lastError instanceof Error
            ? lastError
            : new Error("App Server failed to start");
    }
    request(definition, params, options) {
        if (this.#state !== "ready" || !this.#session) {
            return Promise.reject(new Error(`App Server is not ready: ${this.#state}`));
        }
        return this.#session.request(definition, params, options);
    }
    diagnostics() {
        return this.#session?.diagnostics() ?? this.#lastDiagnostics;
    }
    async stop() {
        if (this.#state === "stopped")
            return;
        this.#stopping = true;
        this.#generation += 1;
        this.#setState("stopping");
        const session = this.#session;
        this.#session = undefined;
        this.#process = undefined;
        this.#sessionNotification.clear();
        await session?.close();
        this.#setState("stopped");
    }
    async #launch() {
        this.#setState("starting");
        const generation = ++this.#generation;
        const child = this.#spawnProcess(this.options.executable, this.options.args, { environment: this.options.environment });
        this.#process = child;
        child.once("exit", () => {
            if (this.#process !== child || this.#generation !== generation)
                return;
            const restart = !this.#stopping && this.#state === "ready";
            const exitedSession = this.#session;
            this.#lastDiagnostics = exitedSession?.diagnostics() ?? this.#lastDiagnostics;
            this.#sessionNotification.clear();
            this.#process = undefined;
            this.#session = undefined;
            if (exitedSession) {
                queueMicrotask(() => exitedSession.dispose());
            }
            if (restart) {
                this.#setState("crashed");
                void this.#restartAfterCrash();
            }
        });
        const session = new AppServerSession(new AppServerClient(new JsonRpcPeer(child)), this.options.session);
        setDisposableOwner(session, this);
        this.#session = session;
        this.#sessionNotification.replace(session.onAnyNotification((notification) => {
            if (this.#session !== session)
                return;
            for (const listener of this.#notificationListeners) {
                try {
                    listener(notification);
                }
                catch {
                    // One host consumer cannot prevent delivery to other notification consumers.
                }
            }
        }));
        this.#setState("initializing");
        try {
            await session.initialize();
        }
        catch (error) {
            this.#lastDiagnostics = session.diagnostics();
            if (this.#session === session) {
                this.#sessionNotification.clear();
                this.#session = undefined;
            }
            if (this.#process === child)
                this.#process = undefined;
            this.#generation += 1;
            await session.close();
            throw error;
        }
        if (this.#stopping || this.#generation !== generation) {
            if (this.#session === session)
                this.#sessionNotification.clear();
            await session.close();
            throw new Error("App Server startup was superseded");
        }
        this.#setState("ready");
    }
    async #restartAfterCrash() {
        while (!this.#stopping && this.#restartAttempts < this.#maxRestartAttempts) {
            const attempt = this.#restartAttempts++;
            this.#setState("restarting");
            await this.#wait(this.#restartDelay(attempt));
            if (this.#stopping)
                return;
            try {
                await this.#launch();
                return;
            }
            catch {
                this.#setState("crashed");
            }
        }
    }
    #restartDelay(attempt) {
        return Math.min(this.#initialRestartDelayMs * 2 ** attempt, this.#maxRestartDelayMs);
    }
    #setState(state) {
        if (this.#state === state)
            return;
        this.#state = state;
        for (const listener of this.#stateListeners) {
            try {
                listener(state);
            }
            catch {
                // Connection state observers are isolated from supervisor lifecycle.
            }
        }
    }
    dispose() {
        if (this.#disposed)
            return;
        this.#disposed = true;
        this.#stateListeners.clear();
        this.#notificationListeners.clear();
        try {
            const stopping = this.stop();
            this.#sessionNotification.dispose();
            void stopping.catch(() => {
                // Explicit stop callers observe errors; disposal is best-effort.
            });
        }
        finally {
            markAsDisposed(this);
        }
    }
    [Symbol.dispose]() {
        this.dispose();
    }
}
function defaultSpawn(executable, args, options) {
    return spawn(executable, [...args], {
        env: { ...options.environment },
        shell: false,
        stdio: "pipe",
    });
}
function wait(milliseconds) {
    return new Promise((resolve) => {
        const timeout = setTimeout(resolve, milliseconds);
        timeout.unref();
    });
}
function positiveInteger(value, fallback, name) {
    const resolved = value ?? fallback;
    if (!Number.isSafeInteger(resolved) || resolved <= 0) {
        throw new Error(`${name} must be a positive safe integer`);
    }
    return resolved;
}
function nonNegativeInteger(value, fallback, name) {
    const resolved = value ?? fallback;
    if (!Number.isSafeInteger(resolved) || resolved < 0) {
        throw new Error(`${name} must be a non-negative safe integer`);
    }
    return resolved;
}
