import { DisposableStore } from "../../base/common/lifecycle.js";
import { createDedicatedWorkerLanguagePort } from "./browser/language/dedicatedWorkerLanguagePort.js";
import type { LanguageWorkerWirePort } from "./common/languages/languageWorkerWire.js";

/** Context supplied to one Alpha dedicated-worker runtime. */
export interface AlphaEditorWorkerContext {
  readonly port: LanguageWorkerWirePort;
  readonly resources: DisposableStore;
}

let activeResources: DisposableStore | undefined;

/** Starts one Alpha worker runtime over the editor's canonical language-wire port. */
export function start(bootstrap: (context: AlphaEditorWorkerContext) => void): void {
  if (typeof bootstrap !== "function") throw new TypeError("Alpha editor worker bootstrap must be a function");
  if (activeResources) throw new Error("Alpha editor worker has already started");
  const resources = new DisposableStore();
  const port = resources.add(createDedicatedWorkerLanguagePort());
  activeResources = resources;
  try {
    bootstrap({ port, resources });
  } catch (error) {
    activeResources = undefined;
    resources.dispose();
    throw error;
  }
}
