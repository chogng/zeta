import { DisposableStore } from "../../../base/common/lifecycle.js";
import { createDedicatedWorkerLanguagePort } from "./dedicatedWorkerLanguagePort.js";
import type { LanguageWorkerWirePort } from "../../common/languages/languageWorkerWire.js";

/** Context supplied to one dedicated language-worker runtime. */
export interface LanguageWorkerContext {
  readonly port: LanguageWorkerWirePort;
  readonly resources: DisposableStore;
}

let activeResources: DisposableStore | undefined;

/** Starts one worker runtime over the editor's canonical language-wire port. */
export function start(bootstrap: (context: LanguageWorkerContext) => void): void {
  if (typeof bootstrap !== "function") throw new TypeError("Editor language-worker bootstrap must be a function");
  if (activeResources) throw new Error("Editor language worker has already started");
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
