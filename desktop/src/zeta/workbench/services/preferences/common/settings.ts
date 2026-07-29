import type { Event } from "../../../../base/common/event.js";
import { createServiceIdentifier } from "../../../../platform/instantiation/common/instantiation.js";

/** Controls the window-scoped Settings overlay. */
export interface ISettingsService {
  readonly onDidChangeVisibility: Event<boolean>;
  readonly onDidChangeActiveSection: Event<string>;
  readonly isOpen: boolean;
  readonly activeSectionId: string;

  open(sectionId?: string): void;
  close(): void;
}

export const ISettingsService =
  createServiceIdentifier<ISettingsService>("settingsService");
