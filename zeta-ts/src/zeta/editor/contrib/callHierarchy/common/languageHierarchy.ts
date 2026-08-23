import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { type URI } from "../../../../base/common/uri.js";
import { type TextPosition, TextRange } from "../../../common/core/text.js";
import { createLanguageFeatureRequest, isLanguageFeatureRequestCurrent, type LanguageFeatureRequest } from "../../../common/languages/languageFeatureRequest.js";
import { LanguageFeatureProviderRegistry, type LanguageFeatureProviderMetadata } from "../../../common/languages/languageFeatureRegistry.js";
import { type TextModel } from "../../../common/model/textModel.js";

export interface LanguageHierarchyItem {
	readonly name: string;
	readonly symbolKind: number;
	readonly detail?: string;
	readonly resource: URI;
	readonly range: TextRange;
	readonly selectionRange: TextRange;
	readonly data?: unknown;
}

export interface LanguageCallHierarchyEntry {
	readonly item: LanguageHierarchyItem;
	readonly fromResource?: URI;
	readonly fromRanges: readonly TextRange[];
}

export interface LanguageHierarchyRequest extends LanguageFeatureRequest {
	readonly resource: URI;
	readonly position: TextPosition;
}

export interface LanguageHierarchyFollowupRequest extends LanguageFeatureRequest {
	readonly resource: URI;
	readonly item: LanguageHierarchyItem;
}

export interface LanguageCallHierarchyProvider extends LanguageFeatureProviderMetadata {
	prepareCallHierarchy(request: LanguageHierarchyRequest, signal: AbortSignal): readonly LanguageHierarchyItem[] | Promise<readonly LanguageHierarchyItem[]>;
	provideIncomingCalls(request: LanguageHierarchyFollowupRequest, signal: AbortSignal): readonly LanguageCallHierarchyEntry[] | Promise<readonly LanguageCallHierarchyEntry[]>;
	provideOutgoingCalls(request: LanguageHierarchyFollowupRequest, signal: AbortSignal): readonly LanguageCallHierarchyEntry[] | Promise<readonly LanguageCallHierarchyEntry[]>;
}

export interface LanguageTypeHierarchyProvider extends LanguageFeatureProviderMetadata {
	prepareTypeHierarchy(request: LanguageHierarchyRequest, signal: AbortSignal): readonly LanguageHierarchyItem[] | Promise<readonly LanguageHierarchyItem[]>;
	provideSupertypes(request: LanguageHierarchyFollowupRequest, signal: AbortSignal): readonly LanguageHierarchyItem[] | Promise<readonly LanguageHierarchyItem[]>;
	provideSubtypes(request: LanguageHierarchyFollowupRequest, signal: AbortSignal): readonly LanguageHierarchyItem[] | Promise<readonly LanguageHierarchyItem[]>;
}

export interface PreparedCallHierarchy {
	readonly roots: readonly LanguageHierarchyItem[];
	incoming(item: LanguageHierarchyItem): Promise<readonly LanguageCallHierarchyEntry[]>;
	outgoing(item: LanguageHierarchyItem): Promise<readonly LanguageCallHierarchyEntry[]>;
}

export interface PreparedTypeHierarchy {
	readonly roots: readonly LanguageHierarchyItem[];
	supertypes(item: LanguageHierarchyItem): Promise<readonly LanguageHierarchyItem[]>;
	subtypes(item: LanguageHierarchyItem): Promise<readonly LanguageHierarchyItem[]>;
}

/** Coordinates prepare/follow-up hierarchy requests while preserving provider identity and revision freshness. */
export class LanguageHierarchyService extends DisposableOwner {
	constructor(private readonly model: TextModel, private readonly resource: URI, private readonly callProviders: LanguageFeatureProviderRegistry<LanguageCallHierarchyProvider>, private readonly typeProviders: LanguageFeatureProviderRegistry<LanguageTypeHierarchyProvider>) { super(); }

	async prepareCallHierarchy(languageId: string, position: TextPosition, signal: AbortSignal = new AbortController().signal): Promise<readonly PreparedCallHierarchy[]> {
		const request = { ...createLanguageFeatureRequest(this.model, languageId, signal), resource: this.resource, position };
		const prepared: PreparedCallHierarchy[] = [];
		for (const provider of this.callProviders.getProviders(languageId)) {
			const roots = normalizeItems(await provider.prepareCallHierarchy(request, signal));
			if (!isLanguageFeatureRequestCurrent(request)) return Object.freeze([]);
			if (roots.length === 0) continue;
			prepared.push(Object.freeze({
				roots,
				incoming: (item: LanguageHierarchyItem) => this.followCall(provider, languageId, item, signal, "incoming"),
				outgoing: (item: LanguageHierarchyItem) => this.followCall(provider, languageId, item, signal, "outgoing"),
			}));
		}
		return Object.freeze(prepared);
	}

	async prepareTypeHierarchy(languageId: string, position: TextPosition, signal: AbortSignal = new AbortController().signal): Promise<readonly PreparedTypeHierarchy[]> {
		const request = { ...createLanguageFeatureRequest(this.model, languageId, signal), resource: this.resource, position };
		const prepared: PreparedTypeHierarchy[] = [];
		for (const provider of this.typeProviders.getProviders(languageId)) {
			const roots = normalizeItems(await provider.prepareTypeHierarchy(request, signal));
			if (!isLanguageFeatureRequestCurrent(request)) return Object.freeze([]);
			if (roots.length === 0) continue;
			prepared.push(Object.freeze({
				roots,
				supertypes: (item: LanguageHierarchyItem) => this.followType(provider, languageId, item, signal, "supertypes"),
				subtypes: (item: LanguageHierarchyItem) => this.followType(provider, languageId, item, signal, "subtypes"),
			}));
		}
		return Object.freeze(prepared);
	}

	private async followCall(provider: LanguageCallHierarchyProvider, languageId: string, item: LanguageHierarchyItem, signal: AbortSignal, direction: "incoming" | "outgoing"): Promise<readonly LanguageCallHierarchyEntry[]> {
		const request = { ...createLanguageFeatureRequest(this.model, languageId, signal), resource: this.resource, item };
		const entries = direction === "incoming" ? await provider.provideIncomingCalls(request, signal) : await provider.provideOutgoingCalls(request, signal);
		if (!isLanguageFeatureRequestCurrent(request)) return Object.freeze([]);
		return Object.freeze(entries.map(entry => Object.freeze({ item: normalizeItem(entry.item), ...(entry.fromResource ? { fromResource: entry.fromResource } : {}), fromRanges: Object.freeze(entry.fromRanges.map(normalizeRange)) })));
	}

	private async followType(provider: LanguageTypeHierarchyProvider, languageId: string, item: LanguageHierarchyItem, signal: AbortSignal, direction: "supertypes" | "subtypes"): Promise<readonly LanguageHierarchyItem[]> {
		const request = { ...createLanguageFeatureRequest(this.model, languageId, signal), resource: this.resource, item };
		const items = direction === "supertypes" ? await provider.provideSupertypes(request, signal) : await provider.provideSubtypes(request, signal);
		return isLanguageFeatureRequestCurrent(request) ? normalizeItems(items) : Object.freeze([]);
	}
}

function normalizeItems(items: readonly LanguageHierarchyItem[]): readonly LanguageHierarchyItem[] { return Object.freeze(items.map(normalizeItem)); }
function normalizeRange(range: TextRange): TextRange { return TextRange.from(range.start, range.end); }
function normalizeItem(item: LanguageHierarchyItem): LanguageHierarchyItem {
	const range = normalizeRange(item.range);
	const selectionRange = normalizeRange(item.selectionRange);
	if (!range.containsRange(selectionRange)) throw new RangeError("Hierarchy selection range must be contained by its symbol range");
	return Object.freeze({ name: item.name, symbolKind: item.symbolKind, ...(item.detail ? { detail: item.detail } : {}), resource: item.resource, range, selectionRange, ...(item.data === undefined ? {} : { data: item.data }) });
}
