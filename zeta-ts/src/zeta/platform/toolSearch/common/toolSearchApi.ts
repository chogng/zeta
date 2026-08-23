import type { ConfigCommandResult, ConfigReadResult, ToolSearchConfigureParams } from "../../../../../generated/app-server/types.js";

/** Transport-only Tool Search operations. Product consumers use IToolSearchService. */
export interface IToolSearchApi {
	readConfig(): Promise<ConfigReadResult>;
	configure(params: ToolSearchConfigureParams): Promise<ConfigCommandResult>;
}
