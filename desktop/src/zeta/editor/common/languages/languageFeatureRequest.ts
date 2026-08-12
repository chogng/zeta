import { type TextSnapshot } from "../core/text.js";
import { type TextModel } from "../model/textModel.js";

/** Common immutable request context passed to a language feature provider. */
export interface LanguageFeatureRequest {
  readonly model: TextModel;
  readonly snapshot: TextSnapshot;
  readonly languageId: string;
  readonly signal: AbortSignal;
}

/** Creates a request from the model's current snapshot and a caller-owned cancellation signal. */
export function createLanguageFeatureRequest(model: TextModel, languageId: string, signal: AbortSignal): LanguageFeatureRequest {
  return Object.freeze({ model, snapshot: model.createSnapshot(), languageId, signal });
}

/** Returns whether a provider result may still be applied to the request's model. */
export function isLanguageFeatureRequestCurrent(request: LanguageFeatureRequest): boolean {
  return !request.signal.aborted && request.model.version === request.snapshot.version;
}
