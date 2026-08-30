const privateSymbol = Symbol('TextModelEditSource');

export type ITextModelEditSourceMetadata = Readonly<{
	source: string;
	[key: string]: string | boolean | null | undefined;
}>;

/** Serializable identity describing why a text mutation was requested. */
export class TextModelEditSource {
	constructor(
		public readonly metadata: ITextModelEditSourceMetadata,
		_privateCtorGuard: typeof privateSymbol,
	) {}

	toString(): string {
		return this.metadata.source;
	}

	getType(): string {
		if (this.metadata.source === 'cursor') return String(this.metadata.kind ?? 'cursor');
		if (this.metadata.source === 'inlineCompletionAccept') return this.metadata.$nes ? 'inlineCompletionAccept:nes' : 'inlineCompletionAccept';
		if (this.metadata.source === 'unknown') return String(this.metadata.name ?? 'unknown');
		return this.metadata.source;
	}

	toKey(level: number, filter: Record<string, boolean> = {}): string {
		return Object.entries(this.metadata)
			.filter(([key, value]) => filter[key] ?? ((key.match(/\$/g)?.length ?? 0) <= level && value !== undefined && value !== null && value !== ''))
			.map(([key, value]) => `${key}:${String(value)}`)
			.join('-');
	}

	get props(): Record<string, string | undefined> {
		return Object.fromEntries(Object.entries(this.metadata).map(([key, value]) => [key, value === undefined ? undefined : String(value)]));
	}
}

function createEditSource(metadata: ITextModelEditSourceMetadata): TextModelEditSource {
	return new TextModelEditSource(metadata, privateSymbol);
}

export const EditSources = Object.freeze({
	unknown: (data: { readonly name?: string | null } = {}) => createEditSource({ source: 'unknown', name: data.name }),
	reloadFromDisk: () => createEditSource({ source: 'reloadFromDisk' }),
	setValue: () => createEditSource({ source: 'setValue' }),
	applyEdits: () => createEditSource({ source: 'applyEdits' }),
	eolChange: () => createEditSource({ source: 'eolChange' }),
});
