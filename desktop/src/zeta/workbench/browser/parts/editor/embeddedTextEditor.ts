import type { IDimension } from "../../../../base/browser/geometry.js";
import type { Event } from "../../../../base/common/event.js";
import type { IDisposable } from "../../../../base/common/lifecycle.js";
import type { URI } from "../../../../base/common/uri.js";

/**
 * A line-oriented editor hosted inside a structured editor block.
 *
 * The host owns the block identity and transaction semantics. The embedded
 * editor only edits plain text and reports completed text snapshots.
 */
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

export interface IEmbeddedTextEditorFactory {
  create(options: EmbeddedTextEditorOptions): IEmbeddedTextEditor;
}
