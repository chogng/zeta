import { type Event } from "../../../../base/common/event.js";
import { createServiceIdentifier } from "../../../../platform/instantiation/common/instantiation.js";
import { type LanguageServerMessageSeverityDto } from "../../../../../../generated/app-server/types.js";

export interface LanguageServerLogEntry {
  readonly sequence: number;
  readonly server: string;
  readonly severity: LanguageServerMessageSeverityDto;
  readonly message: string;
}

export interface LanguageServerProgressState {
  readonly server: string;
  readonly token: string;
  readonly title: string;
  readonly message?: string;
  readonly percentage?: number;
}

/** Window-scoped language-server messages and active work-done progress. */
export interface ILanguageServerStatusService {
  readonly onDidChange: Event<void>;
  getLogEntries(): readonly LanguageServerLogEntry[];
  getProgress(): readonly LanguageServerProgressState[];
  clearLog(): void;
}

export const ILanguageServerStatusService = createServiceIdentifier<ILanguageServerStatusService>("languageServerStatusService");
