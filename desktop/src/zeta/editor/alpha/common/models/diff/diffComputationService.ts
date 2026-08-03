import { type IDisposable } from "../../../../../base/common/lifecycle.js";
import { type LineDiff, type LineDiffOptions } from "./lineDiff.js";

/** One immutable text document supplied to a diff computation. */
export interface DiffComputationDocument {
  readonly version: number;
  readonly text: string;
}

/** One version-pinned request for a presentation-independent text diff. */
export interface DiffComputationRequest {
  readonly original: DiffComputationDocument;
  readonly modified: DiffComputationDocument;
  readonly options: LineDiffOptions;
}

/** Computes a diff outside the widget while respecting caller cancellation. */
export interface IDiffComputationService extends IDisposable {
  compute(request: DiffComputationRequest, signal: AbortSignal): Promise<LineDiff>;
}
