import { URI } from "../../src/zeta/base/common/uri.js";
import { EmbeddedTextEditorFactory } from "../../src/zeta/editor/alpha/browser/embeddedTextEditor.js";
import { createDefaultDocumentSchema } from "../../src/zeta/editor/gama/editor.main.js";
import { createTextNode } from "../../src/zeta/editor/gama/editor.main.js";
import { DocumentModel } from "../../src/zeta/editor/gama/editor.main.js";
import { EditorPane } from "../../src/zeta/editor/gama/browser/editorPane.js";
import { serializeDocument } from "../../src/zeta/editor/gama/common/model/documentSerialization.js";
import { MemoryTextFiles } from "./memoryTextFiles.js";

interface IntegrationHarness {
  readonly apiDocumentType: string;
  getTextBlockText(): string | undefined;
  getStructuredBlockTexts(): readonly string[];
  getStructuredFirstTextMarks(): readonly { readonly type: string; readonly attrs: Readonly<Record<string, string | number | boolean | null>> }[];
  getStructuredSelection(): unknown;
  saveTextBlock(): Promise<void>;
  getSavedTextBlock(): string;
  dispose(): void;
}

declare global {
  interface Window {
    zetaGamaIntegration: IntegrationHarness;
  }
}

const schema = createDefaultDocumentSchema();
const apiDocument = schema.createDocument([schema.createNode("paragraph", { content: [schema.createText("gama-api")] })]);
const apiModel = new DocumentModel(schema, apiDocument);
const textBlockResource = URI.parse("inmemory://gama/gama-text-block.zeta-academic");
const structuredResource = URI.parse("inmemory://gama/gama-structured.zeta-academic");
const textBlockDocument = schema.createDocument([schema.createNode("textBlock", {
  attrs: { language: "typescript" },
  content: [createTextNode("gama-text", "const gama = 1;")],
  id: "gama-text-block",
})], "gama-text-document");
const textBlockFiles = new MemoryTextFiles(textBlockResource, serializeDocument(textBlockDocument, schema));
const structuredFiles = new MemoryTextFiles(structuredResource, "Title\nBody");
const textBlockPane = new EditorPane(textBlockFiles, { embeddedTextEditorFactory: new EmbeddedTextEditorFactory() });
const structuredPane = new EditorPane(structuredFiles);

textBlockPane.create(requiredElement("#gama-text-block"));
structuredPane.create(requiredElement("#gama-structured"));
textBlockPane.layout({ width: 900, height: 300 });
structuredPane.layout({ width: 900, height: 300 });
await textBlockPane.setInput({ resource: textBlockResource, label: "snippet.ts" }, new AbortController().signal);
await structuredPane.setInput({ resource: structuredResource, label: "paper" }, new AbortController().signal);

window.zetaGamaIntegration = {
  apiDocumentType: apiModel.document.type,
  getTextBlockText: () => textBlockPane.getDocument().content[0]?.content[0]?.text,
  getStructuredBlockTexts: () => structuredPane.getDocument().content.map(block => block.content.find(child => child.text !== undefined)?.text ?? ""),
  getStructuredFirstTextMarks: () => structuredPane.getDocument().content[0]?.content[0]?.marks ?? [],
  getStructuredSelection: () => structuredPane.getDocumentSelection(),
  saveTextBlock: () => textBlockPane.save(),
  getSavedTextBlock: () => textBlockFiles.read(textBlockResource),
  dispose: () => {
    apiModel.dispose();
    textBlockPane.dispose();
    structuredPane.dispose();
    textBlockFiles.dispose();
    structuredFiles.dispose();
  },
};

function requiredElement(selector: string): HTMLElement {
  const element = document.querySelector<HTMLElement>(selector);
  if (!element) throw new Error(`Missing Gama integration root '${selector}'`);
  return element;
}
