import assert from "node:assert/strict";
import test from "node:test";
import { Emitter } from "../../../../../base/common/event.js";
import { DisposableOwner } from "../../../../../base/common/lifecycle.js";
import { URI } from "../../../../../base/common/uri.js";
import { BrowserWorkingCopyService } from "../../browser/browserWorkingCopyService.js";
import type { IWorkingCopy } from "../../common/workingCopyService.js";

test("BrowserWorkingCopyService indexes and unregisters format-specific copies", () => {
  using service = new BrowserWorkingCopyService();
  using copy = new FakeWorkingCopy(URI.file("C:\\project\\paper.zeta-academic"));
  const registered: IWorkingCopy[] = [];
  const unregistered: IWorkingCopy[] = [];
  using registeredListener = service.onDidRegister(value => registered.push(value));
  using unregisteredListener = service.onDidUnregister(value => unregistered.push(value));

  using registration = service.register(copy);
  assert.deepEqual(service.get(copy.resource), [copy]);
  assert.deepEqual(registered, [copy]);

  registration.dispose();
  assert.deepEqual(service.get(copy.resource), []);
  assert.deepEqual(unregistered, [copy]);
});

class FakeWorkingCopy extends DisposableOwner implements IWorkingCopy {
  private readonly dirtyEmitter = this.own(new Emitter<void>());
  private readonly externalChangeEmitter = this.own(new Emitter<void>());
  readonly onDidChangeDirty = this.dirtyEmitter.event;
  readonly onDidChangeExternalChange = this.externalChangeEmitter.event;
  readonly onDidChangeContent = this.dirtyEmitter.event;
  readonly isDirty = false;
  readonly hasExternalChange = false;
  readonly backupKind = "text" as const;

  constructor(readonly resource: URI) {
    super();
  }

  async save(_signal: AbortSignal): Promise<void> {}
  backup(): string { return ""; }
  restoreBackup(): void {}
  async saveAs(_resource: URI, _signal: AbortSignal): Promise<void> {}
  async revert(_signal: AbortSignal): Promise<void> {}
}
