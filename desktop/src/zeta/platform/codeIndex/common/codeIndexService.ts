import type { CodeIndexStatusResult, ConfigCommandResult, ConfigReadResult, ProviderConfigDto, SemanticCodeIndexAutomaticContextDto, SemanticCodeIndexSelectionDto } from "../../../../../generated/app-server/types.js";
import { createServiceIdentifier } from "../../instantiation/common/instantiation.js";

/** Frontend-owned semantic code-index configuration contract for the active Workspace. */
export interface ICodeIndexService {
  readConfig(): Promise<ConfigReadResult>;
  configureProvider(config: ProviderConfigDto, expectedRevision: number): Promise<ConfigCommandResult>;
  configure(selection: SemanticCodeIndexSelectionDto, automaticContext: SemanticCodeIndexAutomaticContextDto, expectedRevision: number): Promise<ConfigCommandResult>;
  authorize(expectedRevision: number): Promise<ConfigCommandResult>;
  revoke(expectedRevision: number): Promise<ConfigCommandResult>;
  status(): Promise<CodeIndexStatusResult>;
  cancel(): Promise<CodeIndexStatusResult>;
  retry(): Promise<CodeIndexStatusResult>;
}

export const ICodeIndexService = createServiceIdentifier<ICodeIndexService>("codeIndexService");
