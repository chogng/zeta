import { type URI } from '../../../../base/common/uri.js';
import { createLanguageFeatureRequest, isLanguageFeatureRequestCurrent } from '../../../common/languages/languageFeatureRequest.js';
import { type OwnedLanguageFeatureProviderRegistry } from '../../../common/ownedLanguageFeatureProviderRegistry.js';
import { type TextModel } from '../../../common/model/textModel.js';
import { type LanguageCodeLens, type LanguageCodeLensProvider, type LanguageCodeLensRequest } from '../common/languageCodeLenses.js';

export interface LanguageCodeLensItem {
	readonly symbol: LanguageCodeLens;
	readonly provider: LanguageCodeLensProvider;
}

export class LanguageCodeLensModel {
	public static readonly Empty = new LanguageCodeLensModel([]);

	public readonly lenses: readonly LanguageCodeLensItem[];

	public constructor(lenses: readonly LanguageCodeLensItem[]) {
		this.lenses = Object.freeze([...lenses]);
	}
}

export interface CodeLensRequestContext {
	readonly model: TextModel;
	readonly providers: OwnedLanguageFeatureProviderRegistry<LanguageCodeLensProvider>;
	readonly languageId: string;
	readonly resource?: URI;
	readonly signal: AbortSignal;
	readonly onError: (error: unknown) => void;
}

/** Collects code lenses while retaining the provider that owns each deferred resolve. */
export async function getLanguageCodeLensModel(context: CodeLensRequestContext): Promise<LanguageCodeLensModel> {
	const request = createRequest(context);
	const providers = context.providers.getProviders(context.languageId);
	const providerRanks = new Map(providers.map((provider, index) => [provider, index] as const));
	const results = await Promise.all(providers.map(async provider => {
		try {
			const values = await provider.provideCodeLenses(request, context.signal);
			if (!isLanguageFeatureRequestCurrent(request)) return [];
			if (!Array.isArray(values)) throw new TypeError('Code lens provider must return an array');
			return values.map(value => Object.freeze({
				symbol: normalizeLanguageCodeLens(context.model, value),
				provider,
			}));
		} catch (error) {
			if (!context.signal.aborted) context.onError(error);
			return [];
		}
	}));
	if (!isLanguageFeatureRequestCurrent(request)) return LanguageCodeLensModel.Empty;
	const lenses = results.flat();
	lenses.sort((left, right) => compareCodeLensItems(left, right, providerRanks));
	return new LanguageCodeLensModel(lenses);
}

/** Resolves one deferred lens with the provider that originally produced it. */
export async function resolveLanguageCodeLensItem(context: Omit<CodeLensRequestContext, 'providers'>, item: LanguageCodeLensItem): Promise<LanguageCodeLens | undefined> {
	if (item.symbol.command || !item.provider.resolveCodeLens) return item.symbol;
	const request = createRequest(context);
	try {
		const resolved = await item.provider.resolveCodeLens(item.symbol, request, context.signal);
		if (!isLanguageFeatureRequestCurrent(request)) return undefined;
		return normalizeLanguageCodeLens(context.model, resolved);
	} catch (error) {
		if (!context.signal.aborted) context.onError(error);
		return undefined;
	}
}

function createRequest(context: Pick<CodeLensRequestContext, 'model' | 'languageId' | 'resource' | 'signal'>): LanguageCodeLensRequest {
	return Object.freeze({
		...createLanguageFeatureRequest(context.model, context.languageId, context.signal),
		...(context.resource ? { resource: context.resource } : {}),
	});
}

function normalizeLanguageCodeLens(model: TextModel, lens: LanguageCodeLens): LanguageCodeLens {
	if (!lens || typeof lens !== 'object') throw new TypeError('Code lens must be an object');
	model.offsetAt(lens.range.getStartPosition());
	model.offsetAt(lens.range.getEndPosition());
	const command = lens.command;
	if (command && (typeof command.id !== 'string' || command.id.trim().length === 0 || typeof command.title !== 'string' || command.title.trim().length === 0)) {
		throw new TypeError('Code lens command must provide a non-empty ID and title');
	}
	return Object.freeze({
		range: lens.range,
		...(command ? {
			command: Object.freeze({
				id: command.id,
				title: command.title,
				...(command.arguments ? { arguments: Object.freeze([...command.arguments]) } : {}),
			}),
		} : {}),
		...(lens.data !== undefined ? { data: lens.data } : {}),
	});
}

function compareCodeLensItems(left: LanguageCodeLensItem, right: LanguageCodeLensItem, providerRanks: ReadonlyMap<LanguageCodeLensProvider, number>): number {
	const lineComparison = left.symbol.range.getStartPosition().lineNumber - right.symbol.range.getStartPosition().lineNumber;
	if (lineComparison !== 0) return lineComparison;
	const providerComparison = providerRanks.get(left.provider)! - providerRanks.get(right.provider)!;
	if (providerComparison !== 0) return providerComparison;
	return left.symbol.range.getStartPosition().column - right.symbol.range.getStartPosition().column;
}
