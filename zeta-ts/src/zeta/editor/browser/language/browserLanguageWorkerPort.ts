import { Emitter, type Event } from "../../../base/common/event.js";
import { DisposableOwner } from "../../../base/common/lifecycle.js";
import { type LanguageWorkerWireClientPort } from "../../common/languages/languageWorkerWire.js";

/** Owns one browser Worker and adapts it to the editor's structural wire port. */
export class BrowserLanguageWorkerPort extends DisposableOwner implements LanguageWorkerWireClientPort {
  private readonly messageEmitter = this.own(new Emitter<unknown>());
  private readonly failureEmitter = this.own(new Emitter<unknown>());
  private disposed = false;

  readonly onMessage: Event<unknown> = this.messageEmitter.event;
  readonly onFailure: Event<unknown> = this.failureEmitter.event;

  constructor(private readonly worker: Worker) {
    super();
    const onMessage = (event: MessageEvent<unknown>): void => this.messageEmitter.fire(event.data);
    const onError = (event: ErrorEvent): void => this.failureEmitter.fire(event.error ?? new Error(event.message));
    const onMessageError = (): void => this.failureEmitter.fire(new TypeError("Language Worker returned an unreadable message"));
    worker.addEventListener("message", onMessage);
    worker.addEventListener("error", onError);
    worker.addEventListener("messageerror", onMessageError);
    this.defer(() => {
      this.disposed = true;
      worker.removeEventListener("message", onMessage);
      worker.removeEventListener("error", onError);
      worker.removeEventListener("messageerror", onMessageError);
      worker.terminate();
    });
  }

  send(message: unknown): void {
    if (this.disposed) {
      throw new ReferenceError("BrowserLanguageWorkerPort is already disposed");
    }
    this.worker.postMessage(message);
  }
}
