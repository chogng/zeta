import type { ConfigCommandResult, ConfigReadResult, ToolSearchConfigureParams } from "../../../../../generated/app-server/index.js";

/** Transport-only Tool Search operations. Product consumers use IToolSearchService. */
export interface IToolSearchApi {
	readConfig(): Promise<ConfigReadResult>;
	configure(params: ToolSearchConfigureParams): Promise<ConfigCommandResult>;
}
