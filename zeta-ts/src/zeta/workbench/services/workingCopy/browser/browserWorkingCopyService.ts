import { Emitter } from "../../../../base/common/event.js";
import { DisposableOwner, toDisposable } from "../../../../base/common/lifecycle.js";
import { type URI } from "../../../../base/common/uri.js";
import { type IWorkingCopy, type IWorkingCopyService } from "../common/workingCopyService.js";

/** Browser registry for editor-domain working copies. */
export class BrowserWorkingCopyService extends DisposableOwner implements IWorkingCopyService {
  private readonly copies = new Map<string, Set<IWorkingCopy>>();
  private readonly _onDidRegister = this.own(new Emitter<IWorkingCopy>());
  private readonly _onDidUnregister = this.own(new Emitter<IWorkingCopy>());

  readonly onDidRegister = this._onDidRegister.event;
  readonly onDidUnregister = this._onDidUnregister.event;

  register(workingCopy: IWorkingCopy): ReturnType<typeof toDisposable> {
    validateWorkingCopy(workingCopy);
    const key = workingCopy.resource.toString();
    let copies = this.copies.get(key);
    if (!copies) {
      copies = new Set<IWorkingCopy>();
      this.copies.set(key, copies);
    }
    copies.add(workingCopy);
    this._onDidRegister.fire(workingCopy);
    let registered = true;
    return toDisposable(() => {
      if (!registered) return;
      registered = false;
      const current = this.copies.get(key);
      if (!current || !current.delete(workingCopy)) return;
      if (current.size === 0) this.copies.delete(key);
      this._onDidUnregister.fire(workingCopy);
    });
  }

  get(resource: URI): readonly IWorkingCopy[] {
    return [...this.copies.get(resource.toString()) ?? []];
  }

  getAll(): readonly IWorkingCopy[] {
    return [...this.copies.values()].flatMap(copies => [...copies]);
  }

  override dispose(): void {
    this.copies.clear();
    super.dispose();
  }
}

function validateWorkingCopy(workingCopy: IWorkingCopy): void {
  if (!workingCopy || typeof workingCopy !== "object" || !workingCopy.resource || typeof workingCopy.resource.toString !== "function") {
    throw new TypeError("Working copy registration requires a resource");
  }
  if (typeof workingCopy.onDidChangeContent !== "function" || typeof workingCopy.backup !== "function" || typeof workingCopy.restoreBackup !== "function" || typeof workingCopy.save !== "function" || typeof workingCopy.saveAs !== "function" || typeof workingCopy.revert !== "function") {
    throw new TypeError("Working copy registration requires content events, backup restoration, save, saveAs, and revert operations");
  }
}
