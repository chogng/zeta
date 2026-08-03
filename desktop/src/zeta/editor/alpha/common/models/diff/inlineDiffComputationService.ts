import { DisposableOwner } from "../../../../../base/common/lifecycle.js";
import { computeLineDiff, type LineDiff } from "./lineDiff.js";
import { type DiffComputationRequest, type IDiffComputationService } from "./diffComputationService.js";

/** In-realm computation used by non-browser hosts and deterministic tests. */
export class InlineDiffComputationService extends DisposableOwner implements IDiffComputationService {
  async compute(request: DiffComputationRequest, signal: AbortSignal): Promise<LineDiff> {
    signal.throwIfAborted();
    await Promise.resolve();
    signal.throwIfAborted();
    return computeLineDiff(request.original.text, request.modified.text, request.options);
  }
}
