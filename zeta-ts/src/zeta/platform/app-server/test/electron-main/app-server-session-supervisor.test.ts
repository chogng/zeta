import { strict as assert } from "node:assert";
import type { ChildProcessWithoutNullStreams } from "node:child_process";
import { EventEmitter } from "node:events";
import test from "node:test";
import { PassThrough } from "node:stream";
import {
	APP_SERVER_METHODS,
	APP_SERVER_PROTOCOL_MAJOR,
	APP_SERVER_SCHEMA_HASH,
} from "../../../../../../generated/app-server/types.js";
import { AppServerClient } from "../../../../platform/app-server/electron-main/app-server-client.js";
import { AppServerSession } from "../../../../platform/app-server/electron-main/app-server-session.js";
import { AppServerProtocolIncompatibleError } from "../../../../platform/app-server/electron-main/app-server-session.js";
import {
	AppServerSupervisor,
	type AppServerSupervisorOptions,
} from "../../../../platform/app-server/electron-main/app-server-supervisor.js";
import { JsonRpcPeer } from "../../../../platform/app-server/electron-main/json-rpc-peer.js";
import { LocalAppServerProcessLauncher } from "../../../../platform/app-server/electron-main/localAppServerProcessLauncher.js";

class ProtocolChildProcess extends EventEmitter {
	readonly stdin = new PassThrough();
	readonly stdout = new PassThrough();
	readonly stderr = new PassThrough();
	readonly requests: Array<Record<string, unknown>> = [];
	exitCode: number | null = null;
	signalCode: NodeJS.Signals | null = null;
	private stdinBuffer = "";

	constructor(
		readonly schemaHash: string = APP_SERVER_SCHEMA_HASH,
		readonly respondToInitialize = true,
		readonly serverName = "zeta-test",
		readonly protocolMajor: number = APP_SERVER_PROTOCOL_MAJOR,
		readonly contracts: Readonly<Record<string, { readonly version: number }>> = {
			sessions: { version: 1 },
			threads: { version: 1 },
			turns: { version: 1 },
		},
		readonly includeProtocolVersion = true,
	) {
		super();
		this.stdin.on("data", (chunk: Buffer) => this.onStdin(chunk));
	}

	kill(signal: NodeJS.Signals = "SIGTERM"): boolean {
		if (this.exitCode !== null || this.signalCode !== null) return false;
		this.signalCode = signal;
		queueMicrotask(() => this.emit("exit", null, signal));
		return true;
	}

	crash(): void {
		if (this.exitCode !== null || this.signalCode !== null) return;
		this.exitCode = 1;
		this.emit("exit", 1, null);
	}

	respond(id: unknown, result: unknown): void {
		this.stdout.write(`${JSON.stringify({ jsonrpc: "2.0", id, result })}\n`);
	}

	requestHost(id: string, method: string, params: unknown): void {
		this.stdout.write(`${JSON.stringify({ jsonrpc: "2.0", id, method, params })}\n`);
	}

	private onStdin(chunk: Buffer): void {
		this.stdinBuffer += chunk.toString("utf8");
		const frames = this.stdinBuffer.split("\n");
		this.stdinBuffer = frames.pop() ?? "";
		for (const frame of frames) {
			const request = JSON.parse(frame) as Record<string, unknown>;
			this.requests.push(request);
			if (request.method === "initialize" && this.respondToInitialize) {
				this.respond(request.id, {
					serverInfo: { name: this.serverName, version: "1" },
					...(this.includeProtocolVersion ? { protocolVersion: { major: this.protocolMajor, revision: 1 } } : {}),
					schemaHash: this.schemaHash,
					capabilities: {
						agentInteractions: true,
						documentCollaboration: true,
						sessions: true,
						threads: true,
						turns: true,
						resources: true,
						attachments: true,
						fileSystem: true,
						git: true,
						workspaceSearch: true,
						codebase: true,
						cloudCodebase: false,
						terminal: true,
						debugAdapter: true,
						typst: true,
						updateReplay: true,
						extensions: true,
						extensionHost: true,
						connectors: true,
						plugins: true,
						marketplace: true,
						mcp: true,
						mcpOAuth: true,
						contracts: this.contracts,
					},
					slashCommands: [{ name: "diagnose", description: "Inspect workspace", argumentMode: "optional" }],
				});
			} else if (request.method === "workspace/switch") {
				const params = request.params as { readonly root: string };
				this.respond(request.id, { root: params.root, trust: "restricted" });
			}
		}
	}
}

function session(
	child: ProtocolChildProcess,
	options: { initializeTimeoutMs?: number } = {},
): AppServerSession {
	return new AppServerSession(
		new AppServerClient(
			new JsonRpcPeer(child as unknown as ChildProcessWithoutNullStreams),
		),
		{
			clientName: "desktop-test",
			clientVersion: "1",
			initializeTimeoutMs: options.initializeTimeoutMs ?? 100,
			expectedServerName: "zeta-test",
		},
	);
}

function supervisorOptions(
	children: ProtocolChildProcess[],
): AppServerSupervisorOptions {
	return {
		processLauncher: {
			description: "test-app-server",
			validate() {},
			launch: () => {
				const child = new ProtocolChildProcess();
				children.push(child);
				return child as unknown as ChildProcessWithoutNullStreams;
			},
		},
		session: {
			clientName: "desktop-test",
			clientVersion: "1",
			initializeTimeoutMs: 100,
			expectedServerName: "zeta-test",
		},
		wait: async () => {},
	};
}

test("session becomes ready after protocol and required capability gates pass", async () => {
	const child = new ProtocolChildProcess();
	const appServer = session(child);

	const initialized = await appServer.initialize();

	assert.equal(appServer.state, "ready");
	assert.equal(initialized.schemaHash, APP_SERVER_SCHEMA_HASH);
	assert.equal(appServer.protocolDiagnostics.schemaMatches, true);
	assert.equal(appServer.capabilities.resources, true);
	assert.equal(appServer.serverInfo.name, "zeta-test");
	assert.equal(appServer.slashCommands[0]?.name, "diagnose");
	await appServer.close();
	assert.equal(appServer.state, "closed");
});

test("session keeps a schema mismatch as diagnostics", async () => {
	const child = new ProtocolChildProcess("sha256:wrong");
	const appServer = session(child);

	await appServer.initialize();

	assert.equal(appServer.state, "ready");
	assert.equal(appServer.protocolDiagnostics.schemaMatches, false);
	assert.match(appServer.diagnostics(), /sha256:wrong/);
	await appServer.close();
});

test("session closes a protocol-major-mismatched connection", async () => {
	const child = new ProtocolChildProcess(APP_SERVER_SCHEMA_HASH, true, "zeta-test", APP_SERVER_PROTOCOL_MAJOR + 1);
	const appServer = session(child);

	await assert.rejects(appServer.initialize(), /protocol major mismatch/);

	assert.equal(appServer.state, "closed");
	assert.equal(child.signalCode, "SIGTERM");
});

test("session classifies a legacy unversioned server as protocol-incompatible", async () => {
	const child = new ProtocolChildProcess(APP_SERVER_SCHEMA_HASH, true, "zeta-test", APP_SERVER_PROTOCOL_MAJOR, {
		sessions: { version: 1 },
		threads: { version: 1 },
		turns: { version: 1 },
	}, false);
	const appServer = session(child);

	await assert.rejects(appServer.initialize(), AppServerProtocolIncompatibleError);

	assert.equal(appServer.state, "closed");
});

test("session reports a missing required capability", async () => {
	const child = new ProtocolChildProcess(APP_SERVER_SCHEMA_HASH, true, "zeta-test", APP_SERVER_PROTOCOL_MAJOR, {});
	const appServer = session(child);

	await assert.rejects(appServer.initialize(), /missing required capability sessions/);

	assert.equal(appServer.state, "closed");
});

test("session initialization deadline closes an unresponsive child", async () => {
	const child = new ProtocolChildProcess(APP_SERVER_SCHEMA_HASH, false);
	const appServer = session(child, { initializeTimeoutMs: 5 });

	await assert.rejects(appServer.initialize(), /timed out/);

	assert.equal(appServer.state, "closed");
	assert.equal(child.signalCode, "SIGTERM");
});

test("session rejects an unexpected server identity", async () => {
	const child = new ProtocolChildProcess(
		APP_SERVER_SCHEMA_HASH,
		true,
		"not-zeta",
	);
	const appServer = session(child);

	await assert.rejects(appServer.initialize(), /Unexpected App Server identity/);

	assert.equal(appServer.state, "closed");
});

test("supervisor restarts a crashed process with bounded lifecycle states", async () => {
	const children: ProtocolChildProcess[] = [];
	const supervisor = new AppServerSupervisor(supervisorOptions(children));
	const states: string[] = [];
	supervisor.onStateChange((state) => states.push(state));
	await supervisor.start();
	assert.equal(supervisor.state, "ready");
	assert.equal(supervisor.generation, 1);
	assert.equal(supervisor.slashCommands[0]?.name, "diagnose");

	const restarted = new Promise<void>((resolve) => {
		const dispose = supervisor.onStateChange((state) => {
			if (state === "ready" && children.length === 2) {
				dispose.dispose();
				resolve();
			}
		});
	});
	children[0].crash();
	await restarted;

	assert.equal(children.length, 2);
	assert.equal(supervisor.generation, 2);
	assert.ok(states.includes("crashed"));
	assert.ok(states.includes("restarting"));
	assert.equal(supervisor.state, "ready");
	await supervisor.stop();
	assert.equal(supervisor.state, "stopped");
	assert.equal(supervisor.generation, 3);
});

test("workspace switching keeps the current App Server process and connection", async () => {
	const children: ProtocolChildProcess[] = [];
	const supervisor = new AppServerSupervisor(supervisorOptions(children));
	const states: string[] = [];
	supervisor.onStateChange((state) => states.push(state));
	await supervisor.start();

	const switched = await supervisor.request(APP_SERVER_METHODS["workspace/switch"], {
		root: "/test/workspace",
		trust: { type: "userConfig" },
	});

	assert.deepEqual(switched, { root: "/test/workspace", trust: "restricted" });
	assert.equal(children.length, 1);
	assert.equal(children[0].signalCode, null);
	assert.equal(supervisor.state, "ready");
	assert.deepEqual(states, ["starting", "initializing", "ready"]);
	await supervisor.stop();
});

test("supervisor advertises and preserves client-hosted request handlers", async () => {
	const children: ProtocolChildProcess[] = [];
	const options = supervisorOptions(children);
	options.session.capabilities = {
		browser: { version: 1, observe: true, input: true },
	};
	const supervisor = new AppServerSupervisor(options);
	const registration = supervisor.registerRequestHandler(
		{ method: "browser/create" },
		(params: { url: string }) => ({ targetId: `target:${params.url}` }),
	);
	await supervisor.start();
	const initialize = children[0].requests.find(request => request.method === "initialize");
	assert.deepEqual((initialize?.params as { capabilities: unknown }).capabilities, {
		notifications: true,
		browser: { version: 1, observe: true, input: true },
	});

	children[0].requestHost("browser-host:1:1", "browser/create", { url: "https://example.test/" });
	await new Promise<void>(resolve => setImmediate(resolve));
	const response = children[0].requests.find(request => request.id === "browser-host:1:1");
	assert.deepEqual(response?.result, { targetId: "target:https://example.test/" });

	registration.dispose();
	await supervisor.stop();
});

test("crash rejects an unknown-outcome side effect without replaying it", async () => {
	const children: ProtocolChildProcess[] = [];
	const supervisor = new AppServerSupervisor(supervisorOptions(children));
	await supervisor.start();

	const turn = supervisor.request(APP_SERVER_METHODS["session/request"], {
		commandId: "one",
		sessionId: "session_1",
		expectedSequence: 1,
		request: {
			type: "startTurn",
			threadId: "thread_1",
			input: [{ type: "text", text: "hello" }],
		},
	});
	await new Promise<void>((resolve) => setImmediate(resolve));
	assert.equal(children[0].requests.at(-1)?.method, "session/request");
	const restarted = new Promise<void>((resolve) => {
		const dispose = supervisor.onStateChange((state) => {
			if (state === "ready" && children.length === 2) {
				dispose.dispose();
				resolve();
			}
		});
	});
	children[0].crash();

	await assert.rejects(turn, /exited with code 1/);
	await restarted;
	assert.deepEqual(
		children[1].requests.map((request) => request.method),
		["initialize"],
	);
	await supervisor.stop();
});

test("supervisor stops restarting after its crash budget is exhausted", async () => {
	const children: ProtocolChildProcess[] = [];
	const options = supervisorOptions(children);
	options.maxRestartAttempts = 1;
	const supervisor = new AppServerSupervisor(options);
	await supervisor.start();

	const restarted = new Promise<void>((resolve) => {
		const dispose = supervisor.onStateChange((state) => {
			if (state === "ready" && children.length === 2) {
				dispose.dispose();
				resolve();
			}
		});
	});
	children[0].crash();
	await restarted;
	children[1].crash();
	await new Promise<void>((resolve) => setImmediate(resolve));

	assert.equal(children.length, 2);
	assert.equal(supervisor.state, "crashed");
	await supervisor.stop();
});

test("local launcher requires an absolute executable and rejects variables outside its environment allowlist", () => {
	assert.throws(
		() =>
			new LocalAppServerProcessLauncher({
				executable: "relative/zeta",
				args: [],
				environment: {},
			}),
		/must be absolute/,
	);
	assert.throws(
		() =>
			new LocalAppServerProcessLauncher({
				executable: "/test/zeta",
				args: [],
				environment: {
					PATH: "/test/bin",
					ZETA_PROFILE_ROOT: "/test/state",
					AWS_SECRET_ACCESS_KEY: "should-not-leak",
				},
			}),
		/AWS_SECRET_ACCESS_KEY/,
	);
	const allowedLauncher = new LocalAppServerProcessLauncher({
		executable: "/test/zeta",
		args: [],
		environment: {
			HOME: "/home/zeta",
			LANG: "C.UTF-8",
			PATH: "/test/bin",
			ZETA_PROFILE_ROOT: "/test/state",
		},
		fileExists: () => true,
	});
	allowedLauncher.validate();
});

test("local launcher applies a validated authority environment to the next connection", () => {
	const launchedExecutables: string[] = [];
	const launchedEnvironments: Readonly<Record<string, string>>[] = [];
	const launcher = new LocalAppServerProcessLauncher({
		executable: "/test/zeta-server",
		args: ["app-server", "connect"],
		environment: {
			ZETA_PROFILE_ROOT: "/profiles/zeta",
			ZETA_WORKSPACE_ROOT: "/workspaces/one",
		},
		fileExists: () => true,
		spawnProcess: (executable, _args, options) => {
			launchedExecutables.push(executable);
			launchedEnvironments.push(options.environment);
			return new ProtocolChildProcess(APP_SERVER_SCHEMA_HASH) as unknown as ChildProcessWithoutNullStreams;
		},
	});

	launcher.launch();
	launcher.replaceExecutable("/test/zeta-server.next");
	launcher.replaceEnvironment({
		ZETA_PROFILE_ROOT: "/profiles/zeta",
		ZETA_WORKSPACE_ROOT: "/workspaces/two",
		ZETA_WORKSPACE_TRUST_SOURCE: "userConfig",
	});
	launcher.launch();

	assert.deepEqual(launchedExecutables, ["/test/zeta-server", "/test/zeta-server.next"]);
	assert.equal(launcher.description, "/test/zeta-server.next");
	assert.equal(launchedEnvironments[0].ZETA_WORKSPACE_ROOT, "/workspaces/one");
	assert.equal(launchedEnvironments[1].ZETA_WORKSPACE_ROOT, "/workspaces/two");
	assert.throws(() => launcher.replaceEnvironment({ AWS_SECRET_ACCESS_KEY: "secret" }), /AWS_SECRET_ACCESS_KEY/);
	assert.throws(() => launcher.replaceExecutable("relative/zeta-server"), /must be absolute/u);
});

test("local launcher rejects a packaged binary that does not match its manifest digest", async () => {
	const expectedSha256 = "a".repeat(64);
	const launcher = new LocalAppServerProcessLauncher({
		executable: "/test/zeta-server",
		args: [],
		environment: {},
		expectedSha256,
		fileExists: () => true,
		fileSha256: async () => "b".repeat(64),
	});

	await assert.rejects(launcher.validate(), /failed integrity validation/u);
});

test("initialization failures consume exactly the bounded startup retry budget", async () => {
	const children: ProtocolChildProcess[] = [];
	const options = supervisorOptions(children);
	options.maxRestartAttempts = 1;
	options.session = { ...options.session, initializeTimeoutMs: 5 };
	options.processLauncher = {
		description: "unresponsive-test-app-server",
		validate() {},
		launch: () => {
			const child = new ProtocolChildProcess(APP_SERVER_SCHEMA_HASH, false);
			children.push(child);
			return child as unknown as ChildProcessWithoutNullStreams;
		},
	};
	const supervisor = new AppServerSupervisor(options);

	await assert.rejects(supervisor.start(), /timed out/);

	assert.equal(children.length, 2);
	assert.equal(supervisor.state, "crashed");
	await supervisor.stop();
});

test("supervisor can retry a failed startup gate after stopping", async () => {
	const children: ProtocolChildProcess[] = [];
	const options = supervisorOptions(children);
	options.maxRestartAttempts = 0;
	options.session = { ...options.session, initializeTimeoutMs: 5 };
	let respondToInitialize = false;
	options.processLauncher = {
		description: "recoverable-test-app-server",
		validate() {},
		launch: () => {
			const child = new ProtocolChildProcess(APP_SERVER_SCHEMA_HASH, respondToInitialize);
			children.push(child);
			return child as unknown as ChildProcessWithoutNullStreams;
		},
	};
	const supervisor = new AppServerSupervisor(options);

	await assert.rejects(supervisor.start(), /timed out/);
	assert.equal(supervisor.state, "crashed");

	await supervisor.stop();
	respondToInitialize = true;
	await supervisor.start();

	assert.equal(supervisor.state, "ready");
	assert.equal(children.length, 2);
	await supervisor.stop();
});

test("supervisor gives a launcher one typed initialization recovery without consuming restart budget", async () => {
	const children: ProtocolChildProcess[] = [];
	const failures: unknown[] = [];
	let initialized = 0;
	const options = supervisorOptions(children);
	options.maxRestartAttempts = 0;
	options.processLauncher = {
		description: "recovering-remote-app-server",
		validate() {},
		launch: () => {
			const child = children.length === 0
				? new ProtocolChildProcess(APP_SERVER_SCHEMA_HASH, true, "zeta-test", APP_SERVER_PROTOCOL_MAJOR + 1)
				: new ProtocolChildProcess();
			children.push(child);
			return child as unknown as ChildProcessWithoutNullStreams;
		},
		recoverInitializationFailure: error => {
			failures.push(error);
			return error instanceof AppServerProtocolIncompatibleError;
		},
		didInitialize: () => { initialized += 1; },
	};
	const supervisor = new AppServerSupervisor(options);

	await supervisor.start();

	assert.equal(children.length, 2);
	assert.equal(failures.length, 1);
	assert.equal(initialized, 1);
	assert.equal(supervisor.state, "ready");
	await supervisor.stop();
});
