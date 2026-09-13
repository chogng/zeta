import { type ILanguageDiagnosticsService as EditorLanguageDiagnosticsService } from "../../../../editor/common/services/languageDiagnosticsService.js";
import { createServiceIdentifier } from "../../../../platform/instantiation/common/instantiation.js";

/** Workbench DI view of the editor diagnostic aggregation contract. */
export interface ILanguageDiagnosticsService extends EditorLanguageDiagnosticsService {}

export const ILanguageDiagnosticsService = createServiceIdentifier<ILanguageDiagnosticsService>("languageDiagnosticsService");

export type { LanguageDiagnosticSnapshot, LanguageDiagnosticsPublisher, LanguageDiagnosticsRepository, LanguageDiagnosticsSource } from "../../../../editor/common/services/languageDiagnosticsService.js";
