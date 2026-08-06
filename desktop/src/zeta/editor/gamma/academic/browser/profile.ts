import { ACADEMIC_DOCUMENT_CONTENT_TYPE } from "../../../../product/common/documentTypes.js";
import { inlineNodeViews as citationInlineNodeViews, nodeViews as citationNodeViews } from "../../contrib/citation/browser/nodeViews.js";
import { citationToolbarActions } from "../../contrib/citation/browser/toolbarAction.js";
import { createReferenceIndexPlugin } from "../../contrib/citation/common/references.js";
import { nodeViews as profileNodeViews } from "./nodeViews.js";
import { createAcademicDocumentSchema, createEmptyAcademicDocument } from "../common/schema.js";
import type { DocumentEditorProfile } from "../../browser/profile.js";
import { DOCUMENT_EDITOR_ID } from "../../browser/documentEditorInput.js";

/** Academic's Gamma profile; all shared editing semantics stay in Gamma browser/common. */
export const academicProfile: DocumentEditorProfile = Object.freeze({
  id: "academic",
  editorId: DOCUMENT_EDITOR_ID,
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
