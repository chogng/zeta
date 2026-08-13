import assert from "node:assert/strict";
import test from "node:test";
import { type IServerEventApi } from "../../../../../platform/app-server/common/appServerApi.js";
import { DialogService } from "../../../dialogs/common/dialogService.js";
import { StatusbarAlignment, StatusbarService } from "../../../statusbar/browser/statusbar.js";
import { AppServerLanguageServerStatusService } from "../../browser/appServerLanguageServerStatusService.js";
import { type ServerNotification } from "../../../../../../../generated/app-server/types.js";

test("language-server status service retains logs and projects work-done progress", () => {
  const events = new FakeServerEvents();
  using dialogs = new DialogService();
  using statusbar = new StatusbarService();
  using service = new AppServerLanguageServerStatusService(events, dialogs, statusbar);

  events.fire({ method: "language/serverMessage", params: { server: "rust-analyzer", severity: "warning", show: false, message: "  check failed  " } });
  assert.deepEqual(service.getLogEntries().map(entry => [entry.server, entry.severity, entry.message]), [["rust-analyzer", "warning", "check failed"]]);

  events.fire({ method: "language/serverProgress", params: { server: "rust-analyzer", token: "index", title: "Indexing", message: "1/2 files", percentage: 50, done: false } });
  assert.equal(service.getProgress()[0]?.title, "Indexing");
  assert.equal(statusbar.getEntries(StatusbarAlignment.Right)[0]?.entry.text, "Indexing 50%");
  events.fire({ method: "language/serverProgress", params: { server: "rust-analyzer", token: "index", title: null, message: "done", percentage: null, done: true } });
  assert.deepEqual(service.getProgress(), []);
  assert.equal(statusbar.getEntries(StatusbarAlignment.Right)[0]?.entry.text, "Language Servers");

  service.clearLog();
  assert.deepEqual(service.getLogEntries(), []);
});

class FakeServerEvents implements IServerEventApi {
  private listener: ((event: ServerNotification) => void) | undefined;
  subscribe(listener: (event: ServerNotification) => void) { this.listener = listener; return { dispose: () => { this.listener = undefined; } }; }
  fire(event: ServerNotification): void { this.listener?.(event); }
}
