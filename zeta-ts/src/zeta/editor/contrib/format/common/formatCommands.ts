import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { type TextEdit, type TextRange, type TextPosition } from "../../../common/core/text.js";
import { type TextSelectionSet } from "../../../common/core/selection.js";
import { type EditorEditCommand } from "../../../common/commands/editorEditCommand.js";
import { createEditorEditCommand } from "../../../common/commands/editorCommand.js";
import { createLanguageFeatureRequest, isLanguageFeatureRequestCurrent, type LanguageFeatureRequest } from "../../../common/languages/languageFeatureRequest.js";
import { LanguageFeatureProviderRegistry, type LanguageFeatureProviderMetadata } from "../../../common/languages/languageFeatureRegistry.js";
import { type TextModel } from "../../../common/model/textModel.js";
import { type URI } from "../../../../base/common/uri.js";

export interface LanguageFormattingOptions {
	readonly tabSize: number;
	readonly insertSpaces: boolean;
	readonly trimTrailingWhitespace?: boolean;
}

export interface LanguageFormattingRequest extends LanguageFeatureRequest {
	readonly resource?: URI;
	readonly range?: TextRange;
	readonly options: LanguageFormattingOptions;
	readonly position?: TextPosition;
	readonly ch?: string;
}

export interface LanguageFormattingProvider extends LanguageFeatureProviderMetadata {
	provideDocumentFormattingEdits?(request: LanguageFormattingRequest, signal: AbortSignal): readonly TextEdit[] | Promise<readonly TextEdit[]>;
	provideRangeFormattingEdits?(request: LanguageFormattingRequest, signal: AbortSignal): readonly TextEdit[] | Promise<readonly TextEdit[]>;
	provideOnTypeFormattingEdits?(request: LanguageFormattingRequest, signal: AbortSignal): readonly TextEdit[] | Promise<readonly TextEdit[]>;
}

/** Owns formatting provider dispatch; edit validation/application stays in TextModel and cursor. */
export class FormatService extends DisposableOwner {
	constructor(private readonly model: TextModel, private readonly providers: LanguageFeatureProviderRegistry<LanguageFormattingProvider>, private readonly resource?: URI) {
		super();
	}

	provideDocumentFormattingEdits(languageId: string, options: LanguageFormattingOptions, signal?: AbortSignal): Promise<readonly TextEdit[]> {
		return this.provide(languageId, { options }, "provideDocumentFormattingEdits", signal);
	}

	provideRangeFormattingEdits(languageId: string, range: TextRange, options: LanguageFormattingOptions, signal?: AbortSignal): Promise<readonly TextEdit[]> {
		return this.provide(languageId, { range, options }, "provideRangeFormattingEdits", signal);
	}

	provideOnTypeFormattingEdits(languageId: string, position: TextPosition, ch: string, options: LanguageFormattingOptions, signal?: AbortSignal): Promise<readonly TextEdit[]> {
		return this.provide(languageId, { position, ch, options }, "provideOnTypeFormattingEdits", signal);
	}

	private async provide(languageId: string, fields: Partial<LanguageFormattingRequest>, method: "provideDocumentFormattingEdits" | "provideRangeFormattingEdits" | "provideOnTypeFormattingEdits", signal = new AbortController().signal): Promise<readonly TextEdit[]> {
		const request = { ...createLanguageFeatureRequest(this.model, languageId, signal), ...(this.resource ? { resource: this.resource } : {}), ...fields } as LanguageFormattingRequest;
		for (const provider of this.providers.getProviders(languageId)) {
			const provide = provider[method];
			if (!provide || !isLanguageFeatureRequestCurrent(request)) continue;
			const edits = await provide.call(provider, request, signal);
			if (!isLanguageFeatureRequestCurrent(request)) return Object.freeze([]);
			if (edits.length > 0) return Object.freeze([...edits]);
		}
		return Object.freeze([]);
	}
}

/** Converts current-version formatting edits into the editor's canonical command contract. */
export function createFormattingCommand(model: TextModel, selections: TextSelectionSet, edits: readonly TextEdit[]): EditorEditCommand | undefined {
	return createEditorEditCommand(model, selections, edits);
}
