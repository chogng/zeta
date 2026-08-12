import { addDisposableListener, stopEvent } from "../../base/browser/dom.js";
import { DisposableOwner } from "../../base/common/lifecycle.js";

export interface SaveControllerOptions {
  readonly save: () => Promise<void | boolean>;
  readonly beforeSave?: () => void | Promise<void>;
  readonly onSaveSuccess?: () => void;
  readonly onSaveError?: (error: unknown) => void;
}

/** Routes the focused editor's standard save shortcut without overlapping writes. */
export class SaveController extends DisposableOwner {
  private readonly save: () => Promise<void | boolean>;
  private readonly beforeSave: () => void | Promise<void>;
  private readonly onSaveSuccess: () => void;
  private readonly onSaveError: (error: unknown) => void;
  private saving = false;

  constructor(input: HTMLTextAreaElement, options: SaveControllerOptions) {
    super();
    if (!options || typeof options.save !== "function") {
      this.dispose();
      throw new TypeError("Alpha save controller requires a save operation");
    }
    this.save = options.save;
    this.beforeSave = options.beforeSave ?? (() => {});
    this.onSaveSuccess = options.onSaveSuccess ?? (() => {});
    this.onSaveError = options.onSaveError ?? reportSaveError;
    this.own(addDisposableListener(input, "keydown", event => this.handleKeydown(event)));
  }

  private handleKeydown(event: KeyboardEvent): void {
    if (event.defaultPrevented || event.isComposing || event.getModifierState("AltGraph")) return;
    if ((!event.ctrlKey && !event.metaKey) || event.shiftKey || event.altKey || event.key.toLowerCase() !== "s") return;
    stopEvent(event);
    if (this.saving) return;
    this.saving = true;
    void Promise.resolve()
      .then(() => this.beforeSave())
      .then(() => this.save())
      .then(result => {
        if (result !== false) this.onSaveSuccess();
      })
      .catch(error => this.onSaveError(error))
      .finally(() => {
        this.saving = false;
      });
  }
}

function reportSaveError(error: unknown): void {
  console.error("Alpha editor save failed", error);
}
