import assert from "node:assert/strict";
import test from "node:test";
import type { LogEntry } from "../../common/logService.js";
import { LogService } from "../../common/logServiceImpl.js";

test("LogService publishes structured entries and releases dynamic sinks", () => {
	const entries: LogEntry[] = [];
	using service = new LogService({ now: () => 42 });
	const registration = service.registerSink({ log: entry => entries.push(entry) });
	service.warn("workspace", "Could not restore state", new Error("invalid"));
	registration.dispose();
	service.info("workspace", "Ignored after disposal");
	assert.equal(entries.length, 1);
	assert.equal(entries[0]?.timestampMillis, 42);
	assert.equal(entries[0]?.level, "warning");
	assert.equal(entries[0]?.category, "workspace");
	assert.match(String(entries[0]?.error), /invalid/u);
});

test("LogService isolates a failing sink", () => {
	const entries: LogEntry[] = [];
	const sinkErrors: unknown[] = [];
	using service = new LogService({ sinks: [{ log: () => { throw new Error("sink failed"); } }, { log: entry => entries.push(entry) }], onSinkError: (_entry, error) => sinkErrors.push(error) });
	service.error("test", "failure");
	assert.equal(entries.length, 1);
	assert.equal(sinkErrors.length, 1);
});
