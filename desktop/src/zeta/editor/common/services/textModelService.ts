import { createServiceIdentifier, type ServiceIdentifier } from "../../../platform/instantiation/common/instantiation.js";
import { type Event } from "../../../base/common/event.js";
import { type IDisposable } from "../../../base/common/lifecycle.js";
import { type URI } from "../../../base/common/uri.js";
import { type TextModel } from "../model/textModel.js";
import { type ITextResourceStore } from "./textResourceStore.js";

/** The minimum identity and bootstrap data needed to acquire a text model. */
export interface TextModelInput {
  readonly resource: URI;
  readonly initialText?: string;
}

/** A reference-counted text model plus its persisted-file state. */
export interface TextModelReference extends IDisposable {
  readonly resource: URI;
  readonly model: TextModel;
  readonly isDirty: boolean;
  readonly onDidChangeDirty: Event<void>;
  readonly hasExternalChange: boolean;
  readonly onDidChangeExternalChange: Event<void>;
  save(signal: AbortSignal): Promise<void>;
  revert(signal: AbortSignal): Promise<void>;
}

/** Owns the lifetime and persisted baseline of text models. */
export interface ITextModelService extends IDisposable {
  acquire(input: TextModelInput, signal: AbortSignal): Promise<TextModelReference>;
}

export type { ITextResourceStore } from "./textResourceStore.js";

/** Service key used by hosts that register a text model service. */
export const ITextModelService: ServiceIdentifier<ITextModelService> = createServiceIdentifier<ITextModelService>("textModelService");

/** Reports that a resource changed after the model established its saved baseline. */
export class TextModelConflictError extends Error {
  constructor(readonly resource: URI) {
    super(`Cannot save '${resource.toString()}' because it changed outside the editor`);
    this.name = "TextModelConflictError";
  }
}
