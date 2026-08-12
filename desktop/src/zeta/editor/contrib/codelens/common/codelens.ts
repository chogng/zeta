import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { type TextRange } from "../../../common/core/text.js";
import { createLanguageFeatureRequest, isLanguageFeatureRequestCurrent, type LanguageFeatureRequest } from "../../../common/languages/languageFeatureRequest.js";
import { LanguageFeatureProviderRegistry, type LanguageFeatureProviderMetadata } from "../../../common/languages/languageFeatureRegistry.js";
import { type TextModel } from "../../../common/model/textModel.js";

export interface LanguageCodeLensCommand {
  readonly id: string;
  readonly title: string;
  readonly arguments?: readonly unknown[];
}

export interface LanguageCodeLens {
  readonly range: TextRange;
  readonly command?: LanguageCodeLensCommand;
  readonly data?: unknown;
}

export interface LanguageCodeLensRequest extends LanguageFeatureRequest {}

export interface LanguageCodeLensProvider extends LanguageFeatureProviderMetadata {
  provideCodeLenses(request: LanguageCodeLensRequest, signal: AbortSignal): readonly LanguageCodeLens[] | Promise<readonly LanguageCodeLens[]>;
  resolveCodeLens?(lens: LanguageCodeLens, request: LanguageCodeLensRequest, signal: AbortSignal): LanguageCodeLens | Promise<LanguageCodeLens>;
}

/** Owns versioned code-lens discovery and resolve; command execution is host-owned. */
export class CodeLensService extends DisposableOwner {
  constructor(private readonly model: TextModel, private readonly providers: LanguageFeatureProviderRegistry<LanguageCodeLensProvider>) {
    super();
  }

  async provideCodeLenses(languageId: string, signal: AbortSignal = new AbortController().signal): Promise<readonly LanguageCodeLens[]> {
    const request = createLanguageFeatureRequest(this.model, languageId, signal);
    const result: LanguageCodeLens[] = [];
    for (const provider of this.providers.getProviders(languageId)) {
      if (!isLanguageFeatureRequestCurrent(request)) return Object.freeze([]);
      const lenses = await provider.provideCodeLenses(request, signal);
      if (!isLanguageFeatureRequestCurrent(request)) return Object.freeze([]);
      result.push(...lenses.map(normalizeLanguageCodeLens));
    }
    return Object.freeze(result);
  }

  async resolveCodeLens(languageId: string, lens: LanguageCodeLens, signal: AbortSignal = new AbortController().signal): Promise<LanguageCodeLens> {
    const request = createLanguageFeatureRequest(this.model, languageId, signal);
    for (const provider of this.providers.getProviders(languageId)) {
      if (!provider.resolveCodeLens) continue;
      const resolved = await provider.resolveCodeLens(lens, request, signal);
      if (!isLanguageFeatureRequestCurrent(request)) throw new Error("Code lens result became stale");
      return normalizeLanguageCodeLens(resolved);
    }
    return lens;
  }
}

function normalizeLanguageCodeLens(lens: LanguageCodeLens): LanguageCodeLens {
  if (!lens || typeof lens !== "object") throw new TypeError("Code lens must be an object");
  return Object.freeze({ range: lens.range, ...(lens.command ? { command: Object.freeze({ id: lens.command.id, title: lens.command.title, ...(lens.command.arguments ? { arguments: Object.freeze([...lens.command.arguments]) } : {}) }) } : {}), ...(lens.data !== undefined ? { data: lens.data } : {}) });
}
