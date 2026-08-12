import { ACADEMIC_DOCUMENT_CONTENT_TYPE } from "../../../../product/common/documentTypes.js";
import { inlineNodeViews as citationInlineNodeViews, nodeViews as citationNodeViews } from "../../citation/browser/nodeViews.js";
import { citationToolbarActions } from "../../citation/browser/toolbarAction.js";
import { createReferenceIndexPlugin } from "../../citation/common/references.js";
import { nodeViews as profileNodeViews } from "./nodeViews.js";
import { createAcademicDocumentSchema, createEmptyAcademicDocument } from "../common/schema.js";
import type { EditorProfile } from "../../../browser/services/editorProfile.js";
import { DOCUMENT_EDITOR_ID } from "../../../browser/documentEditorInput.js";

/** Academic profile; shared document editing semantics remain in editor browser/common. */
export const academicProfile: EditorProfile = Object.freeze({
  id: "academic",
  editorId: DOCUMENT_EDITOR_ID,
  editorName: "Structured Editor",
  collaborationSchemaId: "gama-academic-v1",
  input: Object.freeze({
    contentTypes: [ACADEMIC_DOCUMENT_CONTENT_TYPE],
    extensions: [".zeta-academic", ".zeta-paper"],
  }),
  createSchema: createAcademicDocumentSchema,
  createEmptyDocument: createEmptyAcademicDocument,
  outlineNavigator: true,
  inlineNodeViews: citationInlineNodeViews,
  toolbarActions: citationToolbarActions,
  nodeViews: { ...profileNodeViews, ...citationNodeViews },
  createPlugins: () => [createReferenceIndexPlugin()],
});
