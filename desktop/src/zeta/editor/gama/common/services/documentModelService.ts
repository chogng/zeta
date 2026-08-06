import type { Event } from "../../../../base/common/event.js";
import type { IDisposable } from "../../../../base/common/lifecycle.js";
import type { URI } from "../../../../base/common/uri.js";
import type { DocumentNode } from "../model/document.js";
import type { DocumentModel } from "../model/documentModel.js";
import type { DocumentPlugin } from "../model/documentPlugin.js";
import type { DocumentSchema } from "../model/documentSchema.js";

/** Inputs required to acquire one structured Gama document model. */
export interface DocumentModelInput {
  readonly resource: URI;
  readonly initialText?: string;
  readonly schema: DocumentSchema;
  readonly plugins?: readonly DocumentPlugin<unknown>[];
  readonly createEmptyDocument?: () => DocumentNode;
  readonly onSave?: () => Promise<void | boolean>;
}

/** A lifetime-bound structured document model with its persistence state. */
export interface DocumentModelReference extends IDisposable {
  readonly resource: URI;
  readonly model: DocumentModel;
  readonly isDirty: boolean;
  readonly hasExternalChange: boolean;
  readonly onDidChangeDirty: Event<void>;
  readonly onDidChangeExternalChange: Event<void>;
  save(signal: AbortSignal): Promise<void>;
  saveAs(resource: URI, signal: AbortSignal): Promise<void>;
  revert(signal: AbortSignal): Promise<void>;
}

/** Acquires lifetime-bound structured document model references. */
export interface IDocumentModelService extends IDisposable {
  acquire(input: DocumentModelInput, signal: AbortSignal): Promise<DocumentModelReference>;
}
