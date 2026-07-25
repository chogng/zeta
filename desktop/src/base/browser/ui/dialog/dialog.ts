import { Component } from "../common/component.js";

/** A modal dialog backed by the browser's native dialog element. */
export class Dialog extends Component<HTMLDialogElement> {
  constructor(title: string, content: Element | string) {
    const element = document.createElement("dialog");
    element.className = "zeta-dialog";
    super(element);
    const heading = document.createElement("h2");
    heading.textContent = title;
    const body = document.createElement("div");
    if (typeof content === "string") body.textContent = content;
    else body.append(content);
    element.append(heading, body);
  }

  show(): Promise<string> {
    this.element.showModal();
    return new Promise((resolve) => this.element.addEventListener("close", () => resolve(this.element.returnValue), { once: true }));
  }

  close(result = ""): void { this.element.close(result); }
}
