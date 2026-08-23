import type { ChildProcessWithoutNullStreams } from "node:child_process";
import type {
	AppServerMethod,
	AppServerMethodDefinition,
	MethodParams,
	MethodResult,
	ServerNotification,
} from "../../../../../generated/app-server/types.js";
import type { AppServerConnectionState } from "../common/appServerApi.js";
import {
	DisposableSlot,
	type IDisposable,
	markAsDisposed,
	setDisposableOwner,
	trackDisposable,
	toDisposable,
} from "../../../base/common/lifecycle.js";
import { AppServerClient } from "./app-server-client.js";
import type { IAppServerProcessLauncher } from "./appServerProcessLauncher.js";
import {
	AppServerSession,
	type AppServerSessionOptions,
} from "./app-server-session.js";
import { JsonRpcPeer, type RpcMethodDefinition, type RpcRequestContext, type RpcRequestOptions } from "./json-rpc-peer.js";

export interface AppServerSupervisorOptions {
	processLauncher: IAppServerProcessLauncher;
	session: AppServerSessionOptions;
	maxRestartAttempts?: number;
	initialRestartDelayMs?: number;
	maxRestartDelayMs?: number;
	wait?: (milliseconds: number) => Promise<void>;
}

type StateListener = (state: AppServerConnectionState) => void;
type NotificationListener = (notification: ServerNotification) => void;

interface RegisteredRequestHandler {
	readonly definition: RpcMethodDefinition<unknown, unknown>;
	readonly handler: (params: unknown, context: RpcRequestContext) => unknown | Promise<unknown>;
	active?: IDisposable;
}

/**
 * Supervises App Server process/session replacement without replaying application requests.
 */
export class AppServerSupervisor implements IDisposable {
	private readonly wait: (milliseconds: number) => Promise<void>;
	private readonly stateListeners = new Set<StateListener>();
	private readonly notificationListeners = new Set<NotificationListener>();
	private readonly requestHandlers = new Map<string, RegisteredRequestHandler>();
	private readonly sessionNotification = new DisposableSlot<IDisposable>();
	private readonly maxRestartAttempts: number;
	private readonly initialRestartDelayMs: number;
	private readonly maxRestartDelayMs: number;
	private _state: AppServerConnectionState = "stopped";
	private process?: ChildProcessWithoutNullStreams;
	private session?: AppServerSession;
	private generationValue = 0;
	private restartAttempts = 0;
	private stopping = false;
	private disposed = false;
	private lastDiagnostics = "";

	constructor(readonly options: AppServerSupervisorOptions) {
		this.maxRestartAttempts = nonNegativeInteger(
			options.maxRestartAttempts,
			3,
			"maxRestartAttempts",
		);
		this.initialRestartDelayMs = positiveInteger(
			options.initialRestartDelayMs,
			250,
			"initialRestartDelayMs",
		);
		this.maxRestartDelayMs = positiveInteger(
			options.maxRestartDelayMs,
			2_000,
			"maxRestartDelayMs",
		);
		this.wait = options.wait ?? wait;
		trackDisposable(this);
		setDisposableOwner(this.sessionNotification, this);
	}

	get state(): AppServerConnectionState {
		return this._state;
	}

	/** Monotonic identity of the current or most recently attempted process connection. */
	get generation(): number {
		return this.generationValue;
	}

	get slashCommands() {
		if (this._state !== "ready" || !this.session) {
			throw new Error(`App Server is not ready: ${this._state}`);
		}
		return this.session.slashCommands;
	}

	get capabilities() {
		if (this._state !== "ready" || !this.session) return undefined;
		return this.session.capabilities;
	}

	onStateChange(listener: StateListener): IDisposable {
		this.stateListeners.add(listener);
		return toDisposable(() => this.stateListeners.delete(listener));
	}

	onNotification(listener: NotificationListener): IDisposable {
		this.notificationListeners.add(listener);
		return toDisposable(() => this.notificationListeners.delete(listener));
	}

	async start(): Promise<void> {
		if (this.disposed) {
			throw new Error("Cannot start a disposed App Server supervisor");
		}
		if (this._state !== "stopped") {
			throw new Error(`Cannot start App Server supervisor from ${this._state}`);
		}
		await this.options.processLauncher.validate();
		this.stopping = false;
		this.restartAttempts = 0;
		let lastError: unknown;
		let initializationRecovered = false;
		for (let attempt = 0; attempt <= this.maxRestartAttempts; attempt += 1) {
			if (attempt > 0) {
				this.setState("restarting");
				await this.wait(this.restartDelay(attempt - 1));
			}
			try {
				await this.launch();
				return;
			} catch (error) {
				lastError = error;
				this.setState("crashed");
				if (!initializationRecovered && this.options.processLauncher.recoverInitializationFailure) {
					try {
						if (await this.options.processLauncher.recoverInitializationFailure(error)) {
							initializationRecovered = true;
							attempt -= 1;
							continue;
						}
					} catch (recoveryError) {
						lastError = recoveryError;
					}
				}
			}
		}
		throw lastError instanceof Error
			? lastError
			: new Error("App Server failed to start");
	}

	request<M extends AppServerMethod>(
		definition: AppServerMethodDefinition<M>,
		params: MethodParams<M>,
		options?: RpcRequestOptions,
	): Promise<MethodResult<M>> {
		if (this._state !== "ready" || !this.session) {
			return Promise.reject(new Error(`App Server is not ready: ${this._state}`));
		}
		return this.session.request(definition, params, options);
	}

	registerRequestHandler<P, R>(definition: RpcMethodDefinition<P, R>, handler: (params: P, context: RpcRequestContext) => R | Promise<R>): IDisposable {
		if (this.disposed) throw new Error("Cannot register a handler on a disposed App Server supervisor");
		if (this.requestHandlers.has(definition.method)) {
			throw new Error(`App Server host request handler already registered: ${definition.method}`);
		}
		const entry: RegisteredRequestHandler = {
			definition: definition as RpcMethodDefinition<unknown, unknown>,
			handler: (params, context) => handler(params as P, context),
		};
		this.requestHandlers.set(definition.method, entry);
		if (this.session) entry.active = this.session.registerRequestHandler(entry.definition, entry.handler);
		return toDisposable(() => {
			if (this.requestHandlers.get(definition.method) !== entry) return;
			this.requestHandlers.delete(definition.method);
			entry.active?.dispose();
			entry.active = undefined;
		});
	}

	diagnostics(): string {
		return this.session?.diagnostics() ?? this.lastDiagnostics;
	}

	async stop(): Promise<void> {
		if (this._state === "stopped") return;
		this.stopping = true;
		this.generationValue += 1;
		this.setState("stopping");
		const session = this.session;
		this.session = undefined;
		this.process = undefined;
		this.sessionNotification.clear();
		this.clearActiveRequestHandlers();
		await session?.close();
		this.setState("stopped");
	}

	private async launch(): Promise<void> {
		const generation = ++this.generationValue;
		this.setState("starting");
		const child = this.options.processLauncher.launch();
		this.process = child;
		child.once("exit", () => {
			if (this.process !== child || this.generationValue !== generation) return;
			const restart = !this.stopping && this._state === "ready";
			const exitedSession = this.session;
			this.lastDiagnostics = exitedSession?.diagnostics() ?? this.lastDiagnostics;
			this.sessionNotification.clear();
			this.clearActiveRequestHandlers();
			this.process = undefined;
			this.session = undefined;
			if (exitedSession) {
				queueMicrotask(() => exitedSession.dispose());
			}
			if (restart) {
				this.setState("crashed");
				void this.restartAfterCrash();
			}
		});

		const session = new AppServerSession(
			new AppServerClient(new JsonRpcPeer(child)),
			this.options.session,
		);
		setDisposableOwner(session, this);
		this.session = session;
		this.activateRequestHandlers(session);
		this.sessionNotification.replace(session.onAnyNotification((notification) => {
			if (this.session !== session) return;
			for (const listener of this.notificationListeners) {
				try {
					listener(notification);
				} catch {
					// One host consumer cannot prevent delivery to other notification consumers.
				}
			}
		}));
		this.setState("initializing");
		try {
			await session.initialize();
			if (this.stopping || this.generationValue !== generation) throw new Error("App Server startup was superseded");
			await this.options.processLauncher.didInitialize?.();
		} catch (error) {
			this.lastDiagnostics = session.diagnostics();
			if (this.session === session) {
				this.sessionNotification.clear();
				this.clearActiveRequestHandlers();
				this.session = undefined;
			}
			if (this.process === child) this.process = undefined;
			this.generationValue += 1;
			await session.close();
			throw error;
		}
		if (this.stopping || this.generationValue !== generation) {
			if (this.session === session) this.sessionNotification.clear();
			await session.close();
			throw new Error("App Server startup was superseded");
		}
		this.setState("ready");
	}

	private async restartAfterCrash(): Promise<void> {
		while (!this.stopping && this.restartAttempts < this.maxRestartAttempts) {
			const attempt = this.restartAttempts++;
			this.setState("restarting");
			await this.wait(this.restartDelay(attempt));
			if (this.stopping) return;
			try {
				await this.launch();
				return;
			} catch {
				this.setState("crashed");
			}
		}
	}

	private restartDelay(attempt: number): number {
		return Math.min(
			this.initialRestartDelayMs * 2 ** attempt,
			this.maxRestartDelayMs,
		);
	}

	private activateRequestHandlers(session: AppServerSession): void {
		for (const entry of this.requestHandlers.values()) {
			entry.active?.dispose();
			entry.active = session.registerRequestHandler(entry.definition, entry.handler);
		}
	}

	private clearActiveRequestHandlers(): void {
		for (const entry of this.requestHandlers.values()) {
			entry.active?.dispose();
			entry.active = undefined;
		}
	}

	private setState(state: AppServerConnectionState): void {
		if (this._state === state) return;
		this._state = state;
		for (const listener of this.stateListeners) {
			try {
				listener(state);
			} catch {
				// Connection state observers are isolated from supervisor lifecycle.
			}
		}
	}

	dispose(): void {
		if (this.disposed) return;
		this.disposed = true;
		this.stateListeners.clear();
		this.notificationListeners.clear();
		this.clearActiveRequestHandlers();
		this.requestHandlers.clear();
		try {
			const stopping = this.stop();
			this.sessionNotification.dispose();
			void stopping.catch(() => {
				// Explicit stop callers observe errors; disposal is best-effort.
			});
		} finally {
			markAsDisposed(this);
		}
	}

	[Symbol.dispose](): void {
		this.dispose();
	}
}

function wait(milliseconds: number): Promise<void> {
	return new Promise((resolve) => {
		const timeout = setTimeout(resolve, milliseconds);
		timeout.unref();
	});
}

function positiveInteger(value: number | undefined, fallback: number, name: string): number {
	const resolved = value ?? fallback;
	if (!Number.isSafeInteger(resolved) || resolved <= 0) {
		throw new Error(`${name} must be a positive safe integer`);
	}
	return resolved;
}

function nonNegativeInteger(
	value: number | undefined,
	fallback: number,
	name: string,
): number {
	const resolved = value ?? fallback;
	if (!Number.isSafeInteger(resolved) || resolved < 0) {
		throw new Error(`${name} must be a non-negative safe integer`);
	}
	return resolved;
}
