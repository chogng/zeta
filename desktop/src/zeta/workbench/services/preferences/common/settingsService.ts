import { Emitter } from "../../../../base/common/event.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import type { ISettingsService } from "./settings.js";

/** Owns Settings visibility independently of its browser presentation. */
export class SettingsService
  extends DisposableOwner
  implements ISettingsService {
  readonly #onDidChangeVisibility = this.own(new Emitter<boolean>());
  readonly #onDidChangeActiveSection = this.own(new Emitter<string>());
  #isOpen = false;
  #activeSectionId = "general";

  readonly onDidChangeVisibility = this.#onDidChangeVisibility.event;
  readonly onDidChangeActiveSection = this.#onDidChangeActiveSection.event;

  get isOpen(): boolean {
    return this.#isOpen;
  }

  get activeSectionId(): string {
    return this.#activeSectionId;
  }

  open(sectionId?: string): void {
    if (sectionId !== undefined && sectionId !== this.#activeSectionId) {
      if (sectionId.length === 0) {
        throw new TypeError("Settings section ID must not be empty");
      }
      this.#activeSectionId = sectionId;
      this.#onDidChangeActiveSection.fire(sectionId);
    }
    if (this.#isOpen) return;
    this.#isOpen = true;
    this.#onDidChangeVisibility.fire(true);
  }

  close(): void {
    if (!this.#isOpen) return;
    this.#isOpen = false;
    this.#onDidChangeVisibility.fire(false);
  }
}
