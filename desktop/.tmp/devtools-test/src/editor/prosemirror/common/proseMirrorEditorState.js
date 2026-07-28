import { baseKeymap, splitBlock, toggleMark, } from "prosemirror-commands";
import { history, redo, undo, } from "prosemirror-history";
import { keymap, } from "prosemirror-keymap";
import { Schema, } from "prosemirror-model";
import { schema as basicSchema, } from "prosemirror-schema-basic";
import { addListNodes, } from "prosemirror-schema-list";
import { EditorState, } from "prosemirror-state";
/**
 * Canonical structured-document schema for the ProseMirror editor.
 *
 * Add paper-specific nodes and marks here so editing, serialization, agent
 * patches, and tests can share one schema without depending on browser code.
 */
export const proseMirrorDocumentSchema = new Schema({
    nodes: addListNodes(basicSchema.spec.nodes, "paragraph block*", "block"),
    marks: basicSchema.spec.marks,
});
/** Creates a new editor state with the subsystem's schema and base plugins. */
export function createProseMirrorEditorState(initialText) {
    return EditorState.create({
        schema: proseMirrorDocumentSchema,
        doc: documentFromText(initialText),
        plugins: [
            history(),
            keymap({
                "Mod-z": undo,
                "Shift-Mod-z": redo,
                "Mod-y": redo,
                "Mod-b": toggleMark(requiredMark("strong")),
                "Mod-i": toggleMark(requiredMark("em")),
                "Mod-Enter": splitBlock,
            }),
            keymap(baseKeymap),
        ],
    });
}
function requiredMark(name) {
    const mark = proseMirrorDocumentSchema.marks[name];
    if (!mark) {
        throw new ReferenceError(`ProseMirror document schema is missing mark '${name}'`);
    }
    return mark;
}
function documentFromText(text) {
    const lines = text.replaceAll("\r\n", "\n").split("\n");
    const paragraph = proseMirrorDocumentSchema.nodes.paragraph;
    if (!paragraph) {
        throw new ReferenceError("ProseMirror document schema is missing paragraph nodes");
    }
    const content = lines.map((line) => paragraph.create(undefined, line.length > 0
        ? proseMirrorDocumentSchema.text(line)
        : undefined));
    return proseMirrorDocumentSchema.topNodeType.create(undefined, content.length > 0 ? content : [paragraph.create()]);
}
