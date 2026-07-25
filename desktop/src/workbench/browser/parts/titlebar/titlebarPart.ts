import { ActionBar } from "../../../../base/browser/ui/index.js";
import { WorkbenchPart } from "../../part.js";
import type { TitlebarAction } from "./titlebarActions.js";

/** The persistent workbench title area and its host-level actions. */
export class TitlebarPart extends WorkbenchPart {
  readonly #label: HTMLHeadingElement;
  readonly #actions: ActionBar;

  constructor(title = "Zeta", actions: readonly TitlebarAction[] = []) {
    super("titlebar");
    this.#label = document.createElement("h1");
    this.#label.className = "zeta-titlebar-label";
    this.#label.textContent = title;
    this.#actions = new ActionBar(actions.map((action) => ({
      label: action.label,
      title: action.title,
      enabled: action.enabled,
      onClick: () => action.run(),
    })));
    this.titleElement.append(this.#label);
    this.contentElement.append(this.#actions.element);
  }

  setTitle(title: string): void { this.#label.textContent = title; }
}
