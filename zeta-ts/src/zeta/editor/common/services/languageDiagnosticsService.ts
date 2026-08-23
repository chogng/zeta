import { type Event } from "../../../base/common/event.js";
import { type IDisposable } from "../../../base/common/lifecycle.js";
import { type URI } from "../../../base/common/uri.js";
import { type LanguageDiagnostic } from "../languages/languageResults.js";
import { type TextModel } from "../model/textModel.js";

/** Current diagnostics; revision `0` is reserved for unopened workspace resources. */
export interface LanguageDiagnosticSnapshot {
  readonly resource: URI;
  readonly revision: number;
  readonly diagnostics: readonly LanguageDiagnostic[];
}

/** Read-only diagnostic source consumed by editor presentation. */
export interface LanguageDiagnosticsSource {
  readonly onDidChangeDiagnostics: Event<URI>;
  getDiagnostics(resource: URI): LanguageDiagnosticSnapshot | undefined;
}

/** Enumerable diagnostic source consumed by Workbench-wide presentation. */
export interface LanguageDiagnosticsRepository extends LanguageDiagnosticsSource {
  getAllDiagnostics(): readonly LanguageDiagnosticSnapshot[];
}

/** One editor-owned diagnostic producer registered with the shared repository. */
export interface LanguageDiagnosticsPublisher extends IDisposable {
  update(revision: number, diagnostics: readonly LanguageDiagnostic[]): void;
}

/** Owns open-model synchronization and aggregates every current diagnostic producer. */
export interface ILanguageDiagnosticsService extends LanguageDiagnosticsRepository {
  acquire(resource: URI, languageId: string, model: TextModel): IDisposable;
  createPublisher(resource: URI): LanguageDiagnosticsPublisher;
}
