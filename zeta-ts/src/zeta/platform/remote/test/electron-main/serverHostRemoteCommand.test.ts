import { strict as assert } from "node:assert";
import test from "node:test";
import { isCancellationError } from "../../../../base/common/cancellation.js";
import { runServerHostRemoteCommand } from "../../../../platform/remote/electron-main/serverHostRemoteCommand.js";

test("command observer failures reject the command instead of escaping the process event handler", async () => {
	await assert.rejects(
		runServerHostRemoteCommand(process.execPath, ["-e", "process.stderr.write('progress')"], process.env, {
			onStderrData: () => { throw new Error("invalid progress"); },
		}),
		/invalid progress/,
	);
});

test("command cancellation terminates the active local process and preserves its reason", async () => {
	const cancellation = new AbortController();
	const command = runServerHostRemoteCommand(process.execPath, ["-e", "setInterval(() => {}, 1000)"], process.env, {
		onStderrData: () => {},
		signal: cancellation.signal,
	});
	cancellation.abort("user cancelled bootstrap");

	await assert.rejects(
		command,
		(error: unknown) => isCancellationError(error) && error.reason === "user cancelled bootstrap",
	);
});

test("an already-cancelled command never starts", () => {
	const cancellation = new AbortController();
	cancellation.abort("cancelled before spawn");

	assert.throws(
		() => runServerHostRemoteCommand(process.execPath, ["-e", "process.exit(99)"], process.env, {
			onStderrData: () => {},
			signal: cancellation.signal,
		}),
		(error: unknown) => isCancellationError(error) && error.reason === "cancelled before spawn",
	);
});
