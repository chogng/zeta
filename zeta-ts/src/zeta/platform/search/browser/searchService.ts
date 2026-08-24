import type { IWorkspaceSearchApi } from "../common/searchApi.js";
import type { IWorkspaceSearchOptions, IWorkspaceSearchQuery, IWorkspaceSearchComplete, IWorkspaceSearchService } from "../common/search.js";
import type { IWorkspaceContextService, IWorkspaceFolder } from "../../workspace/common/workspace.js";

const DEFAULT_MAX_RESULTS = 2_000;
const RESULT_BATCH_SIZE = 100;
const IDLE_POLL_MILLIS = 20;

/** Pulls bounded backend batches and exposes one cancellable renderer search. */
export class BrowserWorkspaceSearchService implements IWorkspaceSearchService {
	private readonly api: IWorkspaceSearchApi;

	constructor(api: IWorkspaceSearchApi, private readonly workspaceContext?: IWorkspaceContextService) {
		this.api = api;
	}

	async search(
		query: IWorkspaceSearchQuery,
		options: IWorkspaceSearchOptions = {},
	): Promise<IWorkspaceSearchComplete> {
		const folders = this.workspaceContext?.getWorkspace().folders;
		if (!folders) return this.searchFolder(undefined, query, options);
		if (folders.length === 0) return { resultCount: 0, limitHit: false, error: "Open a folder to search files." };
		let resultCount = 0;
		let limitHit = false;
		let error: string | undefined;
		const maxResults = query.maxResults ?? DEFAULT_MAX_RESULTS;
		for (const folder of folders) {
			const complete = await this.searchFolder(folder, { ...query, maxResults: maxResults - resultCount }, options);
			resultCount += complete.resultCount;
			limitHit ||= complete.limitHit;
			error ??= complete.error;
			if (resultCount >= maxResults) {
				limitHit = true;
				break;
			}
		}
		return { resultCount, limitHit, error };
	}

	private async searchFolder(
		folder: IWorkspaceFolder | undefined,
		query: IWorkspaceSearchQuery,
		options: IWorkspaceSearchOptions,
	): Promise<IWorkspaceSearchComplete> {
		throwIfAborted(options.signal);
		const started = await this.api.start({
			...(folder ? { workspaceFolderId: folder.id } : {}),
			query: query.text,
			patternKind: query.patternKind,
			caseSensitivity: query.caseSensitivity,
			includePatterns: [...query.includePatterns],
			excludePatterns: [...query.excludePatterns],
			maxResults: query.maxResults ?? DEFAULT_MAX_RESULTS,
		});
		let cursor = 0;
		try {
			while (true) {
				throwIfAborted(options.signal);
				const snapshot = await this.api.read({
					...(folder ? { workspaceFolderId: folder.id } : {}),
					searchId: started.searchId,
					afterMatch: cursor,
					maxMatches: RESULT_BATCH_SIZE,
				});
				if (snapshot.matches.length > 0) {
					cursor = snapshot.nextMatch;
					options.onProgress?.(snapshot.matches.map((match) => ({
						...match,
						...(folder ? { workspaceFolderId: folder.id, workspaceFolderName: folder.name } : {}),
						ranges: match.ranges.map((range) => ({ ...range })),
					})));
				}
				if (snapshot.completed) {
					return {
						resultCount: cursor,
						limitHit: snapshot.limitHit,
						error: snapshot.error ?? undefined,
					};
				}
				await waitForNextPoll(options.signal);
			}
		} finally {
			await this.api.cancel({
				...(folder ? { workspaceFolderId: folder.id } : {}),
				searchId: started.searchId,
			}).catch(() => {
				// Cancellation is cleanup after completion or connection loss.
			});
		}
	}
}

function throwIfAborted(signal: AbortSignal | undefined): void {
	if (signal?.aborted) throw abortError();
}

function waitForNextPoll(signal: AbortSignal | undefined): Promise<void> {
	return new Promise((resolve, reject) => {
		if (signal?.aborted) {
			reject(abortError());
			return;
		}
		const onAbort = () => {
			clearTimeout(timeout);
			reject(abortError());
		};
		const timeout = setTimeout(() => {
			signal?.removeEventListener("abort", onAbort);
			resolve();
		}, IDLE_POLL_MILLIS);
		signal?.addEventListener("abort", onAbort, { once: true });
	});
}

function abortError(): DOMException {
	return new DOMException("Workspace search was cancelled", "AbortError");
}
