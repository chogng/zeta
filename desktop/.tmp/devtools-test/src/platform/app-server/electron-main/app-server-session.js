import { APP_SERVER_METHODS, } from "../../../../generated/app-server/types.js";
import { markAsDisposed, setDisposableOwner, trackDisposable, } from "../../../base/common/lifecycle.js";
/**
 * Owns one initialized App Server connection and its negotiated immutable capabilities.
 */
export class AppServerSession {
    client;
    options;
    #state = "created";
    #initializeResult;
    constructor(client, options) {
        this.client = client;
        this.options = options;
        trackDisposable(this);
        setDisposableOwner(client, this);
    }
    get state() {
        return this.#state;
    }
    get capabilities() {
        if (!this.#initializeResult) {
            throw new Error("App Server session is not initialized");
        }
        return this.#initializeResult.capabilities;
    }
    get serverInfo() {
        if (!this.#initializeResult) {
            throw new Error("App Server session is not initialized");
        }
        return this.#initializeResult.serverInfo;
    }
    async initialize() {
        if (this.#state !== "created") {
            throw new Error(`Cannot initialize App Server session from ${this.#state}`);
        }
        this.#state = "initializing";
        try {
            const initialized = await this.client.request(APP_SERVER_METHODS.initialize, {
                clientInfo: {
                    name: this.options.clientName,
                    version: this.options.clientVersion,
                },
                capabilities: { notifications: true },
            }, { timeoutMs: this.options.initializeTimeoutMs });
            validateInitializeResult(initialized);
            if (this.options.expectedServerName &&
                initialized.serverInfo.name !== this.options.expectedServerName) {
                throw new Error(`Unexpected App Server identity: ${initialized.serverInfo.name}`);
            }
            if (initialized.schemaHash !== this.options.schemaHash) {
                throw new Error(`Zeta app-server schema mismatch: expected ${this.options.schemaHash}, received ${initialized.schemaHash}`);
            }
            this.#initializeResult = initialized;
            this.#state = "ready";
            return initialized;
        }
        catch (error) {
            await this.close();
            throw error;
        }
    }
    request(definition, params, options) {
        if (this.#state !== "ready") {
            return Promise.reject(new Error(`App Server session is not ready: ${this.#state}`));
        }
        return this.client.request(definition, params, options);
    }
    onNotification(definition, listener) {
        return this.client.onNotification(definition, listener);
    }
    onAnyNotification(listener) {
        return this.client.onAnyNotification(listener);
    }
    registerRequestHandler(definition, handler) {
        if (this.#state === "closed") {
            throw new Error("Cannot register a handler on a closed App Server session");
        }
        return this.client.peer.registerRequestHandler(definition, handler);
    }
    diagnostics() {
        return this.client.diagnostics();
    }
    async close() {
        if (this.#state === "closed")
            return;
        this.#state = "closed";
        try {
            await this.client.close();
        }
        finally {
            markAsDisposed(this);
        }
    }
    dispose() {
        if (this.#state === "closed")
            return;
        this.#state = "closed";
        try {
            this.client.dispose();
        }
        finally {
            markAsDisposed(this);
        }
    }
    [Symbol.dispose]() {
        this.dispose();
    }
}
function validateInitializeResult(value) {
    if (!value ||
        typeof value !== "object" ||
        !value.serverInfo ||
        typeof value.serverInfo.name !== "string" ||
        value.serverInfo.name.trim().length === 0 ||
        typeof value.serverInfo.version !== "string" ||
        value.serverInfo.version.trim().length === 0 ||
        typeof value.schemaHash !== "string" ||
        !value.capabilities ||
        typeof value.capabilities.sessions !== "boolean" ||
        typeof value.capabilities.threads !== "boolean" ||
        typeof value.capabilities.turns !== "boolean" ||
        typeof value.capabilities.resources !== "boolean" ||
        typeof value.capabilities.typst !== "boolean" ||
        typeof value.capabilities.updateReplay !== "boolean") {
        throw new Error("App Server initialize result is malformed");
    }
}
