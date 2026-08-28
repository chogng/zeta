import { strict as assert } from "node:assert";
import type { ChildProcessWithoutNullStreams } from "node:child_process";
import { EventEmitter, once } from "node:events";
import test from "node:test";
import { PassThrough, Writable } from "node:stream";
import { isCancellationError } from "../../../../base/common/errors.js";
import {
	ChildProcessJsonlTransport,
	type ChildProcessJsonlTransportOptions,
} from "../../../../platform/app-server/electron-main/child-process-jsonl-transport.js";
import {
	JsonRpcPeer,
	RpcRequestCancelledError,
	type RpcMethodDefinition,
} from "../../../../platform/app-server/electron-main/json-rpc-peer.js";
import { AppServerRemoteError } from "../../../../platform/app-server/common/appServerError.js";
import {
	APP_SERVER_METHODS,
	APP_SERVER_NOTIFICATIONS,
} from "../../../../../../generated/app-server/types.js";

class FakeChildProcess extends EventEmitter {
	stdin: Writable = new PassThrough();
	readonly stdout = new PassThrough();
	readonly stderr = new PassThrough();
	exitCode: number | null = null;
	signalCode: NodeJS.Signals | null = null;

	kill(signal: NodeJS.Signals = "SIGTERM"): boolean {
		if (this.exitCode !== null || this.signalCode !== null) return false;
		this.signalCode = signal;
		queueMicrotask(() => this.emit("exit", null, signal));
		return true;
	}
}

function transport(
	child: FakeChildProcess,
	options?: ChildProcessJsonlTransportOptions,
): ChildProcessJsonlTransport {
	return new ChildProcessJsonlTransport(
		child as unknown as ChildProcessWithoutNullStreams,
		options,
	);
}

test("frames split UTF-8 code points incrementally on LF boundaries", async () => {
	const child = new FakeChildProcess();
	const jsonl = transport(child);
	const frames: string[] = [];
	jsonl.onFrame((frame) => frames.push(frame));
	const bytes = Buffer.from('{"message":"你好"}\n{"ok":true}\n', "utf8");

	child.stdout.write(bytes.subarray(0, 14));
	child.stdout.write(bytes.subarray(14, 17));
	child.stdout.write(bytes.subarray(17));

	assert.deepEqual(frames, ['{"message":"你好"}', '{"ok":true}']);
	await jsonl.close();
});

test("rejects an oversized unterminated frame before buffering more input", async () => {
	const child = new FakeChildProcess();
	const jsonl = transport(child, { maxFrameBytes: 4 });
	const closed = once(child, "exit");
	const failure = new Promise<Error>((resolve) => jsonl.onClose(resolve));

	child.stdout.write("12345");

	assert.match((await failure).message, /exceeds 4 bytes/);
	await closed;
});

test("rejects CRLF, empty frames, and invalid UTF-8", async () => {
	for (const [bytes, expected] of [
		[Buffer.from("{}\r\n"), /must use LF/],
		[Buffer.from("\n"), /empty JSONL frame/],
		[Buffer.from([0xff, 0x0a]), /invalid UTF-8/],
	] as const) {
		const child = new FakeChildProcess();
		const jsonl = transport(child);
		const failure = new Promise<Error>((resolve) => jsonl.onClose(resolve));
		child.stdout.write(bytes);
		assert.match((await failure).message, expected);
	}
});

test("waits for the stdin write callback and backpressure drain", async () => {
	const child = new FakeChildProcess();
	let written = "";
	child.stdin = new Writable({
		highWaterMark: 1,
		write(chunk: Buffer, _encoding, callback) {
			written += chunk.toString("utf8");
			setImmediate(callback);
		},
	});
	const jsonl = transport(child);

	await jsonl.send('{"id":1}');

	assert.equal(written, '{"id":1}\n');
	await jsonl.close();
});

test("bounds and redacts stderr diagnostics", async () => {
	const child = new FakeChildProcess();
	const jsonl = transport(child, { maxStderrBytes: 128 });
	child.stderr.write("discard-me-".repeat(20));
	child.stderr.write(" Authorization: Bearer super-secret-token\n");

	const diagnostics = jsonl.diagnostics();
	assert.ok(Buffer.byteLength(diagnostics, "utf8") < 160);
	assert.doesNotMatch(diagnostics, /super-secret-token/);
	assert.match(diagnostics, /\[REDACTED\]/);
	await jsonl.close();
});

test("close is asynchronous and idempotent", async () => {
	const child = new FakeChildProcess();
	const jsonl = transport(child);

	const first = jsonl.close();
	const second = jsonl.close();

	assert.equal(first, second);
	await first;
	assert.equal(child.signalCode, "SIGTERM");
});

test("pairs typed requests and preserves remote error details", async () => {
	const child = new FakeChildProcess();
	const peer = new JsonRpcPeer(child as unknown as ChildProcessWithoutNullStreams);
	const firstFrame = once(child.stdin, "data");
	const read = peer.request(APP_SERVER_METHODS["session/thread/read"], { sessionId: "session_1", threadId: "thread_1" });
	const [{ id }] = (await firstFrame).map((chunk) =>
		JSON.parse((chunk as Buffer).toString("utf8")),
	);
	child.stdout.write(
		`${JSON.stringify({ jsonrpc: "2.0", id, result: { thread: { threadId: "thread_1", title: "one", sequence: 0, turns: [] } } })}\n`,
	);

	assert.equal((await read).thread.threadId, "thread_1");

	const secondFrame = once(child.stdin, "data");
	const failed = peer.request(APP_SERVER_METHODS["config/read"], {});
	const [{ id: secondId }] = (await secondFrame).map((chunk) =>
		JSON.parse((chunk as Buffer).toString("utf8")),
	);
	child.stdout.write(
		`${JSON.stringify({ jsonrpc: "2.0", id: secondId, error: { code: -32030, message: "ConfigUnavailable", data: null } })}\n`,
	);

	await assert.rejects(failed, (error: unknown) => {
		assert.ok(error instanceof AppServerRemoteError);
		assert.equal(error.code, -32030);
		assert.equal(error.errorName, "ConfigUnavailable");
		assert.equal(error.data, null);
		return true;
	});
	await peer.close();
});

test("rejects App Server errors with non-null protocol data", async () => {
	const child = new FakeChildProcess();
	const peer = new JsonRpcPeer(child as unknown as ChildProcessWithoutNullStreams);
	const frame = once(child.stdin, "data");
	const failed = peer.request(APP_SERVER_METHODS["config/read"], {});
	const [{ id }] = (await frame).map((chunk) => JSON.parse((chunk as Buffer).toString("utf8")));
	child.stdout.write(`${JSON.stringify({ jsonrpc: "2.0", id, error: { code: -32030, message: "ConfigUnavailable", data: {} } })}\n`);

	await assert.rejects(failed, /invalid JSON-RPC error/);
	peer.dispose();
});

test("times out locally and ignores the retired request's late response", async () => {
	const child = new FakeChildProcess();
	const peer = new JsonRpcPeer(child as unknown as ChildProcessWithoutNullStreams);
	const firstFrame = once(child.stdin, "data");
	const timedOut = peer.request(
		APP_SERVER_METHODS["session/thread/read"],
		{ sessionId: "session_1", threadId: "slow" },
		{ timeoutMs: 5 },
	);
	const [{ id }] = (await firstFrame).map((chunk) =>
		JSON.parse((chunk as Buffer).toString("utf8")),
	);

	await assert.rejects(timedOut, /timed out/);
	child.stdout.write(
		`${JSON.stringify({ jsonrpc: "2.0", id, result: { thread: { threadId: "slow" } } })}\n`,
	);

	const nextFrame = once(child.stdin, "data");
	const next = peer.request(APP_SERVER_METHODS["config/read"], {});
	const [{ id: nextId }] = (await nextFrame).map((chunk) =>
		JSON.parse((chunk as Buffer).toString("utf8")),
	);
	child.stdout.write(
		`${JSON.stringify({ jsonrpc: "2.0", id: nextId, result: { preferredModel: null, theme: null } })}\n`,
	);
	assert.equal((await next).preferredModel, null);
	await peer.close();
});

test("cancels requests and enforces the pending request bound", async () => {
	const child = new FakeChildProcess();
	const peer = new JsonRpcPeer(
		child as unknown as ChildProcessWithoutNullStreams,
		{ maxPendingRequests: 1 },
	);
	const cancellation = new AbortController();
	const requestFrame = once(child.stdin, "data");
	const first = peer.request(
		APP_SERVER_METHODS["session/thread/read"],
		{ sessionId: "session_1", threadId: "thread_1" },
		{ signal: cancellation.signal },
	);
	const [requestChunk] = await requestFrame;
	const requestId = JSON.parse((requestChunk as Buffer).toString("utf8")).id;
	await assert.rejects(
		peer.request(APP_SERVER_METHODS["session/thread/read"], { sessionId: "session_1", threadId: "thread_2" }),
		/pending request limit/,
	);

	const cancellationFrame = once(child.stdin, "data");
	cancellation.abort();
	await assert.rejects(first, (error: unknown) => {
		assert.ok(error instanceof RpcRequestCancelledError);
		assert.equal(isCancellationError(error), true);
		return true;
	});
	const [cancellationChunk] = await cancellationFrame;
	assert.deepEqual(
		JSON.parse((cancellationChunk as Buffer).toString("utf8")),
		{
			jsonrpc: "2.0",
			method: "$/cancelRequest",
			params: { id: requestId },
		},
	);
	await peer.close();
});

test("isolates notification listeners", async () => {
	const child = new FakeChildProcess();
	const peer = new JsonRpcPeer(child as unknown as ChildProcessWithoutNullStreams);
	let observed = "";
	peer.onNotification(APP_SERVER_NOTIFICATIONS["session/thread/update"], () => {
		throw new Error("presentation listener failed");
	});
	peer.onNotification(APP_SERVER_NOTIFICATIONS["session/thread/update"], (params) => {
		observed = params.threadId;
	});

	child.stdout.write(
		`${JSON.stringify({
      jsonrpc: "2.0",
      method: "session/thread/update",
      params: {
        sessionId: "session_1",
        threadId: "thread_7",
        durableSequence: 1,
        update: {
          type: "committed",
          event: {
            type: "threadCreated",
            sessionId: "session_1",
            threadId: "thread_7",
            title: "Thread",
          },
        },
      },
    })}\n`,
	);

	assert.equal(observed, "thread_7");
	await peer.close();
});

test("cancels inbound request handlers and returns a cancellation error", async () => {
	const child = new FakeChildProcess();
	const peer = new JsonRpcPeer(child as unknown as ChildProcessWithoutNullStreams);
	const definition: RpcMethodDefinition<{ value: string }, string> = {
		method: "desktop/test",
	};
	peer.registerRequestHandler(definition, (_params, context) =>
		new Promise<string>((resolve) => {
			context.signal.addEventListener("abort", () => resolve("cancelled"), {
				once: true,
			});
		}),
	);
	const responseFrame = once(child.stdin, "data");

	child.stdout.write(
		`${JSON.stringify({ jsonrpc: "2.0", id: "server-1", method: "desktop/test", params: { value: "work" } })}\n`,
	);
	child.stdout.write(
		`${JSON.stringify({ jsonrpc: "2.0", method: "$/cancelRequest", params: { id: "server-1" } })}\n`,
	);

	const [chunk] = await responseFrame;
	const response = JSON.parse((chunk as Buffer).toString("utf8"));
	assert.equal(response.id, "server-1");
	assert.equal(response.error.code, -32800);
	await peer.close();
});

test("unknown and duplicate response IDs close the peer", async () => {
	for (const duplicate of [false, true]) {
		const child = new FakeChildProcess();
		const peer = new JsonRpcPeer(child as unknown as ChildProcessWithoutNullStreams);
		let id = 999;
		if (duplicate) {
			const requestFrame = once(child.stdin, "data");
			const request = peer.request(APP_SERVER_METHODS["config/read"], {});
			const [chunk] = await requestFrame;
			id = JSON.parse((chunk as Buffer).toString("utf8")).id;
			child.stdout.write(
				`${JSON.stringify({ jsonrpc: "2.0", id, result: { preferredModel: null, theme: null } })}\n`,
			);
			await request;
		}

		child.stdout.write(
			`${JSON.stringify({ jsonrpc: "2.0", id, result: { preferredModel: null, theme: null } })}\n`,
		);

		await assert.rejects(
			peer.request(APP_SERVER_METHODS["config/read"], {}),
			duplicate ? /duplicate response/ : /unknown request/,
		);
	}
});
