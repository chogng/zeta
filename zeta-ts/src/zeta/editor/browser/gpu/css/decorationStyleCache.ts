export interface IDecorationStyleSet {
	readonly color: number | undefined;
	readonly bold: boolean | undefined;
	readonly opacity: number | undefined;
	readonly strikethrough: boolean | undefined;
	readonly strikethroughThickness: number | undefined;
	readonly strikethroughColor: number | undefined;
}

export interface IDecorationStyleCacheEntry extends IDecorationStyleSet {
	readonly id: number;
}

export class DecorationStyleCache {
	private nextId = 1;
	private readonly cacheById = new Map<number, IDecorationStyleCacheEntry>();
	private readonly cacheByStyle = new Map<string, IDecorationStyleCacheEntry>();

	public getOrCreateEntry(color: number | undefined, bold: boolean | undefined, opacity: number | undefined, strikethrough: boolean | undefined, strikethroughThickness: number | undefined, strikethroughColor: number | undefined): number {
		if (color === undefined && bold === undefined && opacity === undefined && strikethrough === undefined && strikethroughThickness === undefined && strikethroughColor === undefined) return 0;
		const key = [color, bold, opacity, strikethrough, strikethroughThickness, strikethroughColor].join('|');
		const existing = this.cacheByStyle.get(key);
		if (existing) return existing.id;
		const entry = Object.freeze({ id: this.nextId++, color, bold, opacity, strikethrough, strikethroughThickness, strikethroughColor });
		this.cacheById.set(entry.id, entry);
		this.cacheByStyle.set(key, entry);
		return entry.id;
	}

	public getStyleSet(id: number): IDecorationStyleSet | undefined {
		return id === 0 ? undefined : this.cacheById.get(id);
	}
}
