import type { Event } from "../../../../base/common/event.js";
import type { IDisposable } from "../../../../base/common/lifecycle.js";
import { createServiceIdentifier } from "../../../../platform/instantiation/common/instantiation.js";
import type { ExtensionCatalog, ExtensionDescriptor } from "../../../../platform/extensions/common/extensionApi.js";
import type { ExtensionThemeRegistry } from "./extensionTheme.js";
export type { ExtensionGrammarContribution, ExtensionLanguageContribution, ExtensionManifest, ExtensionSnippetContribution, ExtensionThemeContribution } from "./extensionManifest.js";
export type { ExtensionThemeCatalog, ExtensionThemeDefinition, ExtensionThemeTokenColorRule, ExtensionThemeTokenColorSettings } from "./extensionTheme.js";
export { ExtensionThemeRegistry, parseExtensionTheme } from "./extensionTheme.js";
export { parseExtensionManifest } from "./extensionManifest.js";

export interface ExtensionServiceFailure {
  readonly extension: ExtensionDescriptor | undefined;
  readonly error: unknown;
}

/** Workbench-owned extension lifecycle and declarative contribution boundary. */
export interface IExtensionService extends IDisposable {
  readonly currentCatalog: ExtensionCatalog;
  readonly themes: ExtensionThemeRegistry;
  readonly onDidChange: Event<ExtensionCatalog>;
  readonly onDidFail: Event<ExtensionServiceFailure>;
  start(): Promise<void>;
  reload(): Promise<void>;
}

export const IExtensionService = createServiceIdentifier<IExtensionService>("extensionService");
