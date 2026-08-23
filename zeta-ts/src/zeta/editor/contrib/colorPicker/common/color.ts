import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { type URI } from "../../../../base/common/uri.js";
import { type TextEdit, type TextRange } from "../../../common/core/text.js";
import { RGBA8 } from "../../../common/core/misc/rgba.js";
import { createLanguageFeatureRequest, isLanguageFeatureRequestCurrent, type LanguageFeatureRequest } from "../../../common/languages/languageFeatureRequest.js";
import { LanguageFeatureProviderRegistry, type LanguageFeatureProviderMetadata } from "../../../common/languages/languageFeatureRegistry.js";
import { type TextModel } from "../../../common/model/textModel.js";

export interface LanguageColorInformation {
	readonly range: TextRange;
	readonly color: RGBA8;
}

export interface LanguageColorPresentation {
	readonly label: string;
	readonly textEdit?: TextEdit;
	readonly additionalTextEdits?: readonly TextEdit[];
}

export interface LanguageColorRequest extends LanguageFeatureRequest {
	readonly resource?: URI;
}

export interface LanguageColorPresentationRequest extends LanguageFeatureRequest {
	readonly color: RGBA8;
	readonly range: TextRange;
	readonly resource?: URI;
}

export interface LanguageColorProvider extends LanguageFeatureProviderMetadata {
	provideDocumentColors(request: LanguageColorRequest, signal: AbortSignal): readonly LanguageColorInformation[] | Promise<readonly LanguageColorInformation[]>;
	provideColorPresentations(request: LanguageColorPresentationRequest, signal: AbortSignal): readonly LanguageColorPresentation[] | Promise<readonly LanguageColorPresentation[]>;
}

/** Provides color ranges and replacement presentations; opening a color widget is browser-owned. */
export class ColorService extends DisposableOwner {
	constructor(private readonly model: TextModel, private readonly providers: LanguageFeatureProviderRegistry<LanguageColorProvider>, private readonly resource?: URI) {
		super();
	}

	async provideDocumentColors(languageId: string, signal: AbortSignal = new AbortController().signal): Promise<readonly LanguageColorInformation[]> {
		const request: LanguageColorRequest = Object.freeze({ ...createLanguageFeatureRequest(this.model, languageId, signal), ...(this.resource ? { resource: this.resource } : {}) });
		const result: LanguageColorInformation[] = [];
		for (const provider of this.providers.getProviders(languageId)) {
			if (!isLanguageFeatureRequestCurrent(request)) return Object.freeze([]);
			const colors = await provider.provideDocumentColors(request, signal);
			if (!isLanguageFeatureRequestCurrent(request)) return Object.freeze([]);
			result.push(...colors.map(color => Object.freeze({ range: color.range, color: color.color })));
		}
		return Object.freeze(result);
	}

	async provideColorPresentations(languageId: string, range: TextRange, color: RGBA8, signal: AbortSignal = new AbortController().signal): Promise<readonly LanguageColorPresentation[]> {
		const request: LanguageColorPresentationRequest = Object.freeze({ ...createLanguageFeatureRequest(this.model, languageId, signal), range, color, ...(this.resource ? { resource: this.resource } : {}) });
		const result: LanguageColorPresentation[] = [];
		for (const provider of this.providers.getProviders(languageId)) {
			const presentations = await provider.provideColorPresentations(request, signal);
			if (!isLanguageFeatureRequestCurrent(request)) return Object.freeze([]);
			result.push(...presentations.map(presentation => Object.freeze({ label: presentation.label, ...(presentation.textEdit ? { textEdit: presentation.textEdit } : {}), ...(presentation.additionalTextEdits ? { additionalTextEdits: Object.freeze([...presentation.additionalTextEdits]) } : {}) })));
		}
		return Object.freeze(result);
	}
}
