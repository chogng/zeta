import { URI } from "../../src/zeta/base/common/uri.js";
import { EditorPane } from "../../src/zeta/editor/alpha/browser/editorPane.js";
import { createBrowserAlphaEditorSession } from "../../src/zeta/editor/alpha/browser/browserEditorSession.js";
import { BrowserTextModelService } from "../../src/zeta/editor/alpha/browser/services/browserTextModelService.js";
import { BrowserTextResourceStore } from "../../src/zeta/editor/alpha/browser/services/browserTextResourceStore.js";
import { TextModel } from "../../src/zeta/editor/alpha/editor.main.js";
import { MemoryTextFiles } from "./memoryTextFiles.js";

interface IntegrationHarness {
  readonly apiText: string;
  getValue(): string;
  save(): Promise<void>;
  getSavedText(): string;
  getSyntaxAnalysisCount(): number;
  dispose(): void;
}

declare global {
  interface Window {
    zetaAlphaIntegration: IntegrationHarness;
  }
}

const root = requiredElement("#alpha-root");
const resource = URI.parse("inmemory://alpha/alpha.rs");
const files = new MemoryTextFiles(resource, "fn main() {\n  answer();\n}\n");
const resourceStore = new BrowserTextResourceStore(files);
const models = new BrowserTextModelService(resourceStore);
let syntaxAnalysisCount = 0;
const pane = new EditorPane(resourceStore, {
  modelService: models,
  createSession: createBrowserAlphaEditorSession,
  syntaxApi: {
    analyze: async params => {
      syntaxAnalysisCount += 1;
      return {
        revision: params.revision,
        hasErrors: true,
        tokens: [
          { kind: "keyword", range: { start: { lineIndex: 0, columnIndex: 0 }, end: { lineIndex: 0, columnIndex: 2 } } },
          { kind: "function", range: { start: { lineIndex: 0, columnIndex: 3 }, end: { lineIndex: 0, columnIndex: 7 } } },
        ],
        foldingRanges: [{ range: { start: { lineIndex: 0, columnIndex: 0 }, end: { lineIndex: 2, columnIndex: 1 } } }],
        symbols: [{
          name: "main",
          kind: "function",
          range: { start: { lineIndex: 0, columnIndex: 0 }, end: { lineIndex: 2, columnIndex: 1 } },
          selectionRange: { start: { lineIndex: 0, columnIndex: 3 }, end: { lineIndex: 0, columnIndex: 7 } },
        }],
        diagnostics: [{ kind: "missing", range: { start: { lineIndex: 1, columnIndex: 2 }, end: { lineIndex: 1, columnIndex: 8 } } }],
      };
    },
  },
});
const apiModel = new TextModel("alpha-api");

pane.create(root);
pane.layout({ width: 900, height: 420 });
await pane.setInput({ resource, label: "alpha.rs" }, new AbortController().signal);

window.zetaAlphaIntegration = {
  apiText: apiModel.getText(),
  getValue: () => pane.getValue(),
  save: () => pane.save(),
  getSavedText: () => files.read(resource),
  getSyntaxAnalysisCount: () => syntaxAnalysisCount,
  dispose: () => {
    apiModel.dispose();
    pane.dispose();
    models.dispose();
    files.dispose();
  },
};

function requiredElement(selector: string): HTMLElement {
  const element = document.querySelector<HTMLElement>(selector);
  if (!element) throw new Error(`Missing Alpha integration root '${selector}'`);
  return element;
}
