import assert from "node:assert/strict";
import test from "node:test";
import { ServiceCollection, } from "../src/platform/instantiation/common/instantiation.js";
import { IRendererApiService } from "../src/workbench/common/services.js";
import { StartTurnCommandId, } from "../src/workbench/contrib/turn/common/turnCommands.js";
import "../src/workbench/contrib/turn/browser/turnActions.js";
import { CommandService, } from "../src/workbench/services/commands/common/commandService.js";
test("start turn command runs the session, thread, and turn pipeline", async () => {
    const calls = [];
    const api = {
        session: {
            async create() {
                calls.push("session.create");
                return {
                    session: {
                        sessionId: "session-1",
                        title: "New conversation",
                        status: "active",
                        sequence: 4,
                        threads: [],
                    },
                };
            },
            async createThread(params) {
                calls.push("session.createThread");
                assert.equal(params.sessionId, "session-1");
                assert.equal(params.expectedSequence, 4);
                return {
                    session: {
                        sessionId: "session-1",
                        title: "New conversation",
                        status: "active",
                        sequence: 5,
                        threads: [],
                    },
                    threadId: "thread-1",
                };
            },
        },
        thread: {
            async read(params) {
                calls.push("thread.read");
                assert.equal(params.threadId, "thread-1");
                return {
                    thread: {
                        sessionId: "session-1",
                        threadId: "thread-1",
                        title: "Main",
                        status: "active",
                        sequence: 7,
                        turns: [],
                    },
                };
            },
        },
        turn: {
            async start(params) {
                calls.push("turn.start");
                assert.equal(params.sessionId, "session-1");
                assert.equal(params.threadId, "thread-1");
                assert.equal(params.expectedSequence, 7);
                assert.deepEqual(params.input, [{ type: "text", text: "Hello" }]);
                return { turnId: "turn-1", sequence: 8 };
            },
        },
    };
    const services = new ServiceCollection();
    services.set(IRendererApiService, api);
    await new CommandService(services).executeCommand(StartTurnCommandId);
    assert.deepEqual(calls, [
        "session.create",
        "session.createThread",
        "thread.read",
        "turn.start",
    ]);
});
