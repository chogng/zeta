import assert from "node:assert/strict";
import test from "node:test";
import {
  DialogResult,
  DialogSeverity,
} from "../src/zeta/platform/dialogs/common/dialogs.js";
import {
  DialogService,
} from "../src/zeta/workbench/services/dialogs/common/dialogService.js";

test("dialog service publishes requests through its owned model", async () => {
  using service = new DialogService();
  const confirmation = service.confirm({
    message: "Continue?",
  });
  const item = service.model.dialogs[0];

  assert.equal(service.model.dialogs.length, 1);
  assert.equal(item?.request.kind, "confirmation");
  item?.close(DialogResult.Primary);

  assert.equal(await confirmation, true);
  assert.equal(service.model.dialogs.length, 0);
});

test("dialog service maps cancellation to a false confirmation", async () => {
  using service = new DialogService();
  const confirmation = service.confirm({
    message: "Delete item?",
    primaryButton: "Delete",
  });

  service.model.dialogs[0]?.cancel();

  assert.equal(await confirmation, false);
});

test("disposing dialog service cancels every queued model request", async () => {
  const service = new DialogService();
  const confirmation = service.confirm({ message: "Active" });
  const message = service.showMessage({
    severity: DialogSeverity.Warning,
    message: "Queued",
  });

  assert.equal(service.model.dialogs.length, 2);
  service.dispose();

  assert.equal(await confirmation, false);
  await message;
  assert.equal(service.model.dialogs.length, 0);
});

test("dialog service propagates model presentation failures", async () => {
  using service = new DialogService();
  const result = service.showMessage({
    severity: DialogSeverity.Error,
    message: "Failure",
  });

  service.model.dialogs[0]?.fail(new Error("render failed"));

  await assert.rejects(result, /render failed/);
  assert.equal(service.model.dialogs.length, 0);
});
