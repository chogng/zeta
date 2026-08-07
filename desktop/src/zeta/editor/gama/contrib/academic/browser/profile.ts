import { ACADEMIC_DOCUMENT_CONTENT_TYPE } from "../../../../../product/common/documentTypes.js";
import { inlineNodeViews as citationInlineNodeViews, nodeViews as citationNodeViews } from "../../citation/browser/nodeViews.js";
import { citationToolbarActions } from "../../citation/browser/toolbarAction.js";
import { createReferenceIndexPlugin } from "../../citation/common/references.js";
import { nodeViews as profileNodeViews } from "./nodeViews.js";
import { createAcademicDocumentSchema, createEmptyAcademicDocument } from "../common/schema.js";
import type { EditorProfile } from "../../../browser/services/editorProfile.js";
import { GAMA_EDITOR_ID } from "../../../browser/editorInput.js";

/** Academic's Gama profile; shared editing semantics remain in Gama browser/common. */
export const academicProfile: EditorProfile = Object.freeze({
  id: "academic",
  editorId: GAMA_EDITOR_ID,
  editorName: "Structured Editor",
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
