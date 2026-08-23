import { Emitter } from "../../../../base/common/event.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import type { ISettingsService } from "./settings.js";

/** Owns Settings visibility independently of its browser presentation. */
export class SettingsService
  extends DisposableOwner
  implements ISettingsService {
  private readonly _onDidChangeVisibility = this.own(new Emitter<boolean>());
  private readonly _onDidChangeActiveSection = this.own(new Emitter<string>());
  private _isOpen = false;
  private _activeSectionId = "general";

  readonly onDidChangeVisibility = this._onDidChangeVisibility.event;
  readonly onDidChangeActiveSection = this._onDidChangeActiveSection.event;

  get isOpen(): boolean {
    return this._isOpen;
  }

  get activeSectionId(): string {
    return this._activeSectionId;
  }

  open(sectionId?: string): void {
    if (sectionId !== undefined && sectionId !== this._activeSectionId) {
      if (sectionId.length === 0) {
        throw new TypeError("Settings section ID must not be empty");
      }
      this._activeSectionId = sectionId;
      this._onDidChangeActiveSection.fire(sectionId);
    }
    if (this._isOpen) return;
    this._isOpen = true;
    this._onDidChangeVisibility.fire(true);
  }

  close(): void {
    if (!this._isOpen) return;
    this._isOpen = false;
    this._onDidChangeVisibility.fire(false);
  }
}
