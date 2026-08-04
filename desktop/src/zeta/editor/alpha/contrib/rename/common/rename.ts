import { DisposableOwner } from "../../../../../base/common/lifecycle.js";
import { type TextPosition, type TextRange } from "../../../common/core/text.js";
import { createLanguageFeatureRequest, isLanguageFeatureRequestCurrent, type LanguageFeatureRequest } from "../../../common/languages/languageFeatureRequest.js";
import { LanguageFeatureProviderRegistry, type LanguageFeatureProviderMetadata } from "../../../common/languages/languageFeatureRegistry.js";
import { type TextModel } from "../../../common/model/textModel.js";
import { type LanguageWorkspaceEdit } from "../../codeAction/common/codeAction.js";

export interface LanguageRenameRequest extends LanguageFeatureRequest {
  readonly position: TextPosition;
  readonly newName?: string;
}

export interface LanguageRenamePreparation {
  readonly range: TextRange;
  readonly placeholder: string;
}

export interface LanguageRenameProvider extends LanguageFeatureProviderMetadata {
  prepareRename?(request: LanguageRenameRequest, signal: AbortSignal): LanguageRenamePreparation | undefined | Promise<LanguageRenamePreparation | undefined>;
  provideRenameEdits(request: LanguageRenameRequest, signal: AbortSignal): LanguageWorkspaceEdit | Promise<LanguageWorkspaceEdit>;
}

/** Separates rename preparation/UI from the eventual workspace edit transaction. */
export class RenameService extends DisposableOwner {
  constructor(private readonly model: TextModel, private readonly providers: LanguageFeatureProviderRegistry<LanguageRenameProvider>) {
    super();
  }

  async prepareRename(languageId: string, position: TextPosition, signal: AbortSignal = new AbortController().signal): Promise<LanguageRenamePreparation | undefined> {
    const request = { ...createLanguageFeatureRequest(this.model, languageId, signal), position };
    for (const provider of this.providers.getProviders(languageId)) {
      if (!provider.prepareRename) continue;
      const result = await provider.prepareRename(request, signal);
      if (!isLanguageFeatureRequestCurrent(request)) return undefined;
      if (result) return Object.freeze({ range: result.range, placeholder: result.placeholder });
    }
    return undefined;
  }

  async provideRenameEdits(languageId: string, position: TextPosition, newName: string, signal: AbortSignal = new AbortController().signal): Promise<LanguageWorkspaceEdit> {
    if (newName.trim().length === 0) throw new TypeError("Rename name must not be empty");
    const request = { ...createLanguageFeatureRequest(this.model, languageId, signal), position, newName };
    for (const provider of this.providers.getProviders(languageId)) {
      const edit = await provider.provideRenameEdits(request, signal);
      if (!isLanguageFeatureRequestCurrent(request)) throw new Error("Rename result became stale");
      return Object.freeze({ edits: Object.freeze([...edit.edits]) });
    }
    return Object.freeze({ edits: Object.freeze([]) });
  }
}
