import type { WorkspaceSearchCancelParams, WorkspaceSearchReadParams, WorkspaceSearchReadResult, WorkspaceSearchStartParams, WorkspaceSearchStartResult } from "../../../../../generated/app-server/types.js";
import type { IWorkspaceSearchOptions, IWorkspaceSearchQuery, IWorkspaceSearchComplete, IWorkspaceSearchService } from "../common/search.js";

const DEFAULT_MAX_RESULTS = 2_000;
const RESULT_BATCH_SIZE = 100;
const IDLE_POLL_MILLIS = 20;

/** Narrow App Server capability consumed by the browser search adapter. */
export interface IWorkspaceSearchApi {
  start(params: WorkspaceSearchStartParams): Promise<WorkspaceSearchStartResult>;
  read(params: WorkspaceSearchReadParams): Promise<WorkspaceSearchReadResult>;
  cancel(params: WorkspaceSearchCancelParams): Promise<void>;
}

/** Pulls bounded backend batches and exposes one cancellable renderer search. */
export class BrowserWorkspaceSearchService implements IWorkspaceSearchService {
  private readonly api: IWorkspaceSearchApi;

  constructor(api: IWorkspaceSearchApi) {
    this.api = api;
  }

  async search(
    query: IWorkspaceSearchQuery,
    options: IWorkspaceSearchOptions = {},
  ): Promise<IWorkspaceSearchComplete> {
    throwIfAborted(options.signal);
    const started = await this.api.start({
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
          searchId: started.searchId,
          afterMatch: cursor,
          maxMatches: RESULT_BATCH_SIZE,
        });
        if (snapshot.matches.length > 0) {
          cursor = snapshot.nextMatch;
          options.onProgress?.(snapshot.matches);
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
