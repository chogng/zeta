import { strict as assert } from "node:assert";
import test from "node:test";
import { CancellationError, isCancellationError, throwIfCancelled, } from "../../common/cancellation.js";
test("live AbortSignals do not cancel an operation", () => {
    const controller = new AbortController();
    assert.doesNotThrow(() => throwIfCancelled(controller.signal));
});
test("aborted signals produce a classified error and preserve their reason", () => {
    const controller = new AbortController();
    const reason = new Error("superseded");
    controller.abort(reason);
    assert.throws(() => throwIfCancelled(controller.signal, "Request cancelled"), (error) => {
        assert.ok(isCancellationError(error));
        assert.equal(error.message, "Request cancelled");
        assert.equal(error.reason, reason);
        assert.equal(error.cause, reason);
        return true;
    });
});
test("unrelated errors are not classified as cancellation", () => {
    assert.equal(isCancellationError(new CancellationError()), true);
    assert.equal(isCancellationError(new Error("cancelled")), false);
});
