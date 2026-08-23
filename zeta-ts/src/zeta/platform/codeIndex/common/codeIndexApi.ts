import type { CodeIndexStatusResult, ConfigCommandResult, ConfigReadResult, ProviderConfigureParams, SemanticCodeIndexAuthorizeParams, SemanticCodeIndexConfigureParams } from "../../../../../generated/app-server/types.js";

/** Transport-only semantic code-index operations. Product consumers use ICodeIndexService. */
export interface ICodeIndexApi {
	readConfig(): Promise<ConfigReadResult>;
	configureProvider(params: ProviderConfigureParams): Promise<ConfigCommandResult>;
	configure(params: SemanticCodeIndexConfigureParams): Promise<ConfigCommandResult>;
	authorize(params: SemanticCodeIndexAuthorizeParams): Promise<ConfigCommandResult>;
	revoke(params: SemanticCodeIndexAuthorizeParams): Promise<ConfigCommandResult>;
	status(): Promise<CodeIndexStatusResult>;
	cancel(): Promise<CodeIndexStatusResult>;
	retry(): Promise<CodeIndexStatusResult>;
}
