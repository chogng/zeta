import { type URI } from "../../../../base/common/uri.js";
import { type EditorLineGutterDecoration } from "../../../../editor/browser/view/lineGutterDecoration.js";
import { type ServicesAccessor } from "../../../../platform/instantiation/common/instantiation.js";

export type EditorLineGutterDecorationFactory = (resource: URI, accessor: ServicesAccessor) => EditorLineGutterDecoration | undefined;

const factories: EditorLineGutterDecorationFactory[] = [];

/** Registers one product contribution that projects Workbench semantics into a generic editor gutter slot. */
export function registerEditorLineGutterDecorationFactory(factory: EditorLineGutterDecorationFactory): void {
  if (typeof factory !== "function") throw new TypeError("Editor gutter decoration factory must be a function");
  factories.push(factory);
}

export function createEditorLineGutterDecorations(resource: URI, accessor: ServicesAccessor): readonly EditorLineGutterDecoration[] {
  return Object.freeze(factories.flatMap(factory => factory(resource, accessor) ?? []));
}
