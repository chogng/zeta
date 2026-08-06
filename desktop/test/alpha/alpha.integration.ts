import { URI } from "../../src/zeta/base/common/uri.js";
import { AlphaEditorPane } from "../../src/zeta/editor/alpha/browser/alphaEditorPane.js";
import { createBrowserAlphaEditorSession } from "../../src/zeta/editor/alpha/browser/browserAlphaEditorSession.js";
import { BrowserTextModelService } from "../../src/zeta/editor/alpha/browser/services/browserTextModelService.js";
import { BrowserTextResourceStore } from "../../src/zeta/editor/alpha/browser/services/browserTextResourceStore.js";
import { TextModel } from "../../src/zeta/editor/alpha/editor.main.js";
import { MemoryTextFiles } from "./memoryTextFiles.js";

interface AlphaIntegrationHarness {
  readonly apiText: string;
  getValue(): string;
  save(): Promise<void>;
  getSavedText(): string;
  dispose(): void;
}

declare global {
  interface Window {
    zetaAlphaIntegration: AlphaIntegrationHarness;
  }
}

const root = requiredElement("#alpha-root");
const resource = URI.parse("inmemory://alpha/alpha.ts");
const files = new MemoryTextFiles(resource, "const answer = 42;\nconsole.log(answer);");
const resourceStore = new BrowserTextResourceStore(files);
const models = new BrowserTextModelService(resourceStore);
const pane = new AlphaEditorPane(resourceStore, { modelService: models, createSession: createBrowserAlphaEditorSession });
const apiModel = new TextModel("alpha-api");

pane.create(root);
pane.layout({ width: 900, height: 420 });
await pane.setInput({ resource, label: "alpha.ts" }, new AbortController().signal);

window.zetaAlphaIntegration = {
  apiText: apiModel.getText(),
  getValue: () => pane.getValue(),
  save: () => pane.save(),
  getSavedText: () => files.read(resource),
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
