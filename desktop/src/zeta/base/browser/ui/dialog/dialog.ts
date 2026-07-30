import { addDisposableListener } from "../../dom.js";
import { DisposableOwner } from "../../../common/lifecycle.js";
import {
  focusFirst,
  trapTabFocus,
} from "../../focus.js";
import { setAriaAttribute } from "../aria/aria.js";

let nextDialogId = 1;

export interface DialogOptions {
  readonly title: string;
  readonly content: Element | string;
  readonly ownerDocument?: Document;
}

/** A modal dialog backed by the browser's native dialog element. */
export class Dialog extends DisposableOwner {
  readonly element: HTMLDialogElement;
  private resolve: ((result: string) => void) | undefined;
  private shown = false;

  constructor(options: DialogOptions) {
    super();
    const ownerDocument = options.ownerDocument ?? document;
    const element = ownerDocument.createElement("dialog");
    this.element = element;
    this.defer(() => element.remove());
    element.className = "zeta-dialog";
    const heading = ownerDocument.createElement("h2");
    heading.className = "zeta-dialog-title";
    heading.id = `zeta-dialog-title-${nextDialogId++}`;
    heading.textContent = options.title;
    setAriaAttribute(element, "labelledby", heading.id);
    const body = ownerDocument.createElement("div");
    body.className = "zeta-dialog-body";
    if (typeof options.content === "string") {
      body.textContent = options.content;
    } else {
      body.append(options.content);
    }
    element.append(heading, body);
    this.own(addDisposableListener(element, "close", () => {
      this.finish(element.returnValue);
    }));
    this.own(trapTabFocus(element));
    this.defer(() => {
      if (element.open) element.close();
      this.finish("");
    });
  }

  show(): Promise<string> {
    if (this.shown) {
      throw new Error("Dialog instances can only be shown once");
    }
    this.shown = true;
    const result = new Promise<string>((resolve) => {
      this.resolve = resolve;
    });
    try {
      this.element.showModal();
      focusFirst(this.element);
    } catch (error) {
      this.resolve = undefined;
      throw error;
    }
    return result;
  }

  close(result = ""): void {
    if (!this.element.open) return;
    this.element.close(result);
    this.finish(result);
  }

  private finish(result: string): void {
    const resolve = this.resolve;
    if (!resolve) return;
    this.resolve = undefined;
    resolve(result);
  }
}
