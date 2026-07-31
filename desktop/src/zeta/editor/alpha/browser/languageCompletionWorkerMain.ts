import { DisposableStore } from "../../../base/common/lifecycle.js";
import { LanguageCompletionCatalogWirePublisher } from "../common/languageCompletionCatalogWire.js";
import { LanguageCompletionProviderRegistry } from "../common/languageCompletionProviders.js";
import { LanguageCompletionProviderModuleHost, LanguageCompletionProviderModuleRegistry } from "../common/languageCompletionProviderModules.js";
import { LanguageCompletionProviderModuleWireServer } from "../common/languageCompletionProviderModuleWire.js";
import { LanguageCompletionResolveWireServer } from "../common/languageCompletionResolveWire.js";
import { LanguageCompletionProviderWorker } from "../common/languageCompletionService.js";
import { languageCompletionWireCodec } from "../common/languageCompletionWire.js";
import { createLanguageWordCompletionProvider } from "../common/languageWordCompletionProvider.js";
import { LanguageWorkerWireServer } from "../common/languageWorkerWire.js";
import { createDedicatedWorkerLanguagePort } from "./dedicatedWorkerLanguagePort.js";

const resources = new DisposableStore();
const registry = resources.add(new LanguageCompletionProviderRegistry());
const modules = resources.add(new LanguageCompletionProviderModuleRegistry());
resources.add(modules.register({
  id: "alpha.word",
  load: () => [createLanguageWordCompletionProvider()],
}));
const moduleHost = resources.add(new LanguageCompletionProviderModuleHost(modules, registry));
const port = createDedicatedWorkerLanguagePort();
const providerWorker = new LanguageCompletionProviderWorker(registry);
resources.add(new LanguageWorkerWireServer(
  port,
  languageCompletionWireCodec,
  providerWorker,
));
resources.add(new LanguageCompletionCatalogWirePublisher(port, registry));
resources.add(new LanguageCompletionProviderModuleWireServer(port, modules, moduleHost));
resources.add(new LanguageCompletionResolveWireServer(port, providerWorker));
