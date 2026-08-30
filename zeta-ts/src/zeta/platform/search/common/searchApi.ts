import type { ContentSearchCancelParams, ContentSearchReadParams, ContentSearchReadResult, ContentSearchStartParams, ContentSearchStartResult } from "../../../../../generated/app-server/types.js";

export interface IContentSearchApi {
	start(params: ContentSearchStartParams): Promise<ContentSearchStartResult>;
	read(params: ContentSearchReadParams): Promise<ContentSearchReadResult>;
	cancel(params: ContentSearchCancelParams): Promise<void>;
}
