import assert from "node:assert/strict";
import test from "node:test";
import {
  type DialogRequest,
  DialogResult,
  DialogService,
  DialogSeverity,
  type IDialogHandler,
} from "../src/platform/dialogs/common/dialogs.js";

class TestDialogHandler implements IDialogHandler {
  readonly calls: Array<{
    readonly request: DialogRequest;
    readonly signal: AbortSignal;
    readonly resolve: (result: DialogResult) => void;
    readonly reject: (error: unknown) => void;
  }> = [];

  showDialog(
    request: DialogRequest,
    signal: AbortSignal,
  ): Promise<DialogResult> {
    return new Promise((resolve, reject) => {
      this.calls.push({
        request,
        signal,
        resolve,
        reject,
      });
    });
  }
}

test("dialog service serializes modal requests", async () => {
  const handler = new TestDialogHandler();
  using service = new DialogService(handler);

  const confirmation = service.confirm({
    message: "Continue?",
  });
  const message = service.showMessage({
    severity: DialogSeverity.Info,
    message: "Finished",
  });

  assert.equal(handler.calls.length, 1);
  assert.equal(handler.calls[0]?.request.kind, "confirmation");

  handler.calls[0]?.resolve(DialogResult.Primary);
  assert.equal(await confirmation, true);
  assert.equal(handler.calls.length, 2);
  assert.equal(handler.calls[1]?.request.kind, "message");

  handler.calls[1]?.resolve(DialogResult.Primary);
  await message;
});

test("dialog service maps cancellation to a false confirmation", async () => {
  const handler = new TestDialogHandler();
  using service = new DialogService(handler);
  const confirmation = service.confirm({
    message: "Delete item?",
    primaryButton: "Delete",
  });

  handler.calls[0]?.resolve(DialogResult.Cancel);
  assert.equal(await confirmation, false);
});

test("disposing dialog service cancels active and queued requests", async () => {
  const handler = new TestDialogHandler();
  const service = new DialogService(handler);
  const active = service.confirm({ message: "Active" });
  const queued = service.showMessage({
    severity: DialogSeverity.Warning,
    message: "Queued",
  });
  const activeSignal = handler.calls[0]?.signal;

  service.dispose();

  assert.equal(activeSignal?.aborted, true);
  assert.equal(await active, false);
  await queued;
  assert.equal(handler.calls.length, 1);
});

test("dialog service continues after a handler failure", async () => {
  const handler = new TestDialogHandler();
  using service = new DialogService(handler);
  const failed = service.showMessage({
    severity: DialogSeverity.Error,
    message: "Failure",
  });
  const next = service.confirm({ message: "Retry?" });

  handler.calls[0]?.reject(new Error("render failed"));
  await assert.rejects(failed, /render failed/);
  assert.equal(handler.calls.length, 2);

  handler.calls[1]?.resolve(DialogResult.Primary);
  assert.equal(await next, true);
});
