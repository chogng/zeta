import { type OwnedDecorationSource } from "../../../../editor/browser/viewparts/decorations/decorationPresentation.js";
import { type TextModel } from "../../../../editor/common/model/textModel.js";
import { type ServicesAccessor } from "../../../../platform/instantiation/common/instantiation.js";
import { type URI } from "../../../../base/common/uri.js";

export interface EditorDecorationSourceFactoryContext {
	readonly accessor: ServicesAccessor;
	readonly model: TextModel;
	readonly resource: URI;
}

export type EditorDecorationSourceFactory = (context: EditorDecorationSourceFactoryContext) => OwnedDecorationSource | undefined;

const factories: EditorDecorationSourceFactory[] = [];

/** Registers one mode contribution that projects resource semantics into editor decorations. */
export function registerEditorDecorationSourceFactory(factory: EditorDecorationSourceFactory): void {
	if (typeof factory !== "function") throw new TypeError("Editor decoration source factory must be a function");
	factories.push(factory);
}

export function createEditorDecorationSources(context: EditorDecorationSourceFactoryContext): readonly OwnedDecorationSource[] {
	return Object.freeze(factories.flatMap(factory => factory(context) ?? []));
}
