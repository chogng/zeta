import { Disposable } from "../../../../base/common/lifecycle.js";
import { type TextRange } from "../../../common/core/text.js";
import { type LanguageDiagnostic } from "../../../common/languages/languageResults.js";
import { createLanguageFeatureRequest, isLanguageFeatureRequestCurrent, type LanguageFeatureRequest } from "../../../common/languages/languageFeatureRequest.js";
import { LanguageFeatureProviderRegistry, type LanguageFeatureProviderMetadata } from "../../../common/languageFeatureRegistry.js";
import { type TextModel } from "../../../common/model/textModel.js";
import { normalizeLanguageWorkspaceEdit, type LanguageWorkspaceEdit } from "../../../common/languages/languageWorkspaceEdit.js";
import { type URI } from "../../../../base/common/uri.js";

export type { LanguageWorkspaceEdit } from "../../../common/languages/languageWorkspaceEdit.js";

export interface LanguageCodeAction {
	readonly title: string;
	readonly kind?: string;
	readonly isPreferred?: boolean;
	readonly disabledReason?: string;
	readonly edit?: LanguageWorkspaceEdit;
	readonly data?: unknown;
}

export interface LanguageCodeActionRequest extends LanguageFeatureRequest {
	readonly resource: URI;
	readonly range: TextRange;
	readonly diagnostics: readonly LanguageDiagnostic[];
	readonly only?: readonly string[];
}

export interface LanguageCodeActionProvider extends LanguageFeatureProviderMetadata {
	provideCodeActions(request: LanguageCodeActionRequest, signal: AbortSignal): readonly LanguageCodeAction[] | Promise<readonly LanguageCodeAction[]>;
	resolveCodeAction?(action: LanguageCodeAction, request: LanguageCodeActionRequest, signal: AbortSignal): LanguageCodeAction | Promise<LanguageCodeAction>;
}

/** Collects code actions and keeps edit application in the editor command layer. */
export class CodeActionService extends Disposable {
	constructor(private readonly model: TextModel, private readonly resource: URI, private readonly providers: LanguageFeatureProviderRegistry<LanguageCodeActionProvider>) {
		super();
	}

	async provideCodeActions(languageId: string, range: TextRange, diagnostics: readonly LanguageDiagnostic[] = [], only?: readonly string[], signal: AbortSignal = new AbortController().signal): Promise<readonly LanguageCodeAction[]> {
		const request = { ...createLanguageFeatureRequest(this.model, languageId, signal), resource: this.resource, range, diagnostics, ...(only ? { only } : {}) };
		const result: LanguageCodeAction[] = [];
		for (const provider of this.providers.getProviders(languageId)) {
			if (!isLanguageFeatureRequestCurrent(request)) return Object.freeze([]);
			const actions = await provider.provideCodeActions(request, signal);
			if (!isLanguageFeatureRequestCurrent(request)) return Object.freeze([]);
			result.push(...actions.map(normalizeLanguageCodeAction));
		}
		return Object.freeze(result);
	}

	async resolveCodeAction(languageId: string, range: TextRange, action: LanguageCodeAction, diagnostics: readonly LanguageDiagnostic[] = [], signal: AbortSignal = new AbortController().signal): Promise<LanguageCodeAction> {
		const request = { ...createLanguageFeatureRequest(this.model, languageId, signal), resource: this.resource, range, diagnostics };
		for (const provider of this.providers.getProviders(languageId)) {
			if (!provider.resolveCodeAction) continue;
			const resolved = await provider.resolveCodeAction(action, request, signal);
			if (!isLanguageFeatureRequestCurrent(request)) throw new Error("Code action result became stale");
			return normalizeLanguageCodeAction(resolved);
		}
		return action;
	}
}

function normalizeLanguageCodeAction(action: LanguageCodeAction): LanguageCodeAction {
	if (!action || typeof action !== "object" || typeof action.title !== "string" || action.title.trim().length === 0) throw new TypeError("Code action title must be a non-empty string");
	return Object.freeze({
		title: action.title,
		...(action.kind !== undefined ? { kind: action.kind } : {}),
		...(action.isPreferred !== undefined ? { isPreferred: action.isPreferred } : {}),
		...(action.disabledReason !== undefined ? { disabledReason: action.disabledReason } : {}),
		...(action.edit ? { edit: normalizeLanguageWorkspaceEdit(action.edit) } : {}),
		...(action.data !== undefined ? { data: action.data } : {}),
	});
}
