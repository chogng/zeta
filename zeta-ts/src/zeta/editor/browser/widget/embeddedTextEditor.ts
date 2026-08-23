import type { IDimension } from "../../../base/browser/geometry.js";
import type { Event } from "../../../base/common/event.js";
import type { IDisposable } from "../../../base/common/lifecycle.js";
import type { URI } from "../../../base/common/uri.js";

/** A line-oriented editor surface hosted by another editor widget. */
export interface IEmbeddedTextEditor extends IDisposable {
  readonly onDidChange: Event<string>;
  create(parent: HTMLElement): void;
  setValue(value: string): void;
  getValue(): string;
  layout(dimension: IDimension): void;
  focus(): void;
}

export interface EmbeddedTextEditorOptions {
  readonly resource: URI;
  readonly label: string;
  readonly languageId?: string;
  readonly initialText: string;
  readonly readOnly?: boolean;
}

/** Creates an embedded line editor without exposing its host implementation. */
export interface IEmbeddedTextEditorFactory {
  create(options: EmbeddedTextEditorOptions): IEmbeddedTextEditor;
}
