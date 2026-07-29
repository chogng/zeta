import {
  ContextView,
  type ContextViewHideReason,
  type ContextViewOptions,
} from "../../../base/browser/ui/contextview/contextview.js";
import { DisposableOwner } from "../../../base/common/lifecycle.js";
import type { IContextViewService } from "./contextView.js";

/** Browser ContextView service scoped to one Workbench container. */
export class BrowserContextViewService
  extends DisposableOwner
  implements IContextViewService {
  readonly container: HTMLElement;
  readonly #contextView: ContextView;

  constructor(container: HTMLElement) {
    super();
    this.container = container;
    this.#contextView = this.own(new ContextView(container));
  }

  show(options: ContextViewOptions): boolean {
    return this.#contextView.show(options);
  }

  hide(reason?: ContextViewHideReason): void {
    this.#contextView.hide(reason);
  }

  layout(): void {
    this.#contextView.layout();
  }
}
