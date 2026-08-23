/**
 * Holds one typed drag payload while it remains inside the current renderer.
 *
 * Browser DataTransfer is still populated for native compatibility. This
 * transfer preserves the in-memory identity required by same-renderer drops
 * without serializing product objects into a browser-visible format.
 */
export class LocalSelectionTransfer<T> {
	private static readonly instance = new LocalSelectionTransfer<unknown>();

	private data: readonly T[] | undefined;
	private token: object | undefined;

	private constructor() {}

	static getInstance<T>(): LocalSelectionTransfer<T> {
		return LocalSelectionTransfer.instance as LocalSelectionTransfer<T>;
	}

	hasData(token: object): boolean {
		return token === this.token;
	}

	getData(token: object): readonly T[] | undefined {
		return this.hasData(token) ? this.data : undefined;
	}

	setData(data: readonly T[], token: object): void {
		this.data = [...data];
		this.token = token;
	}

	clearData(token: object): void {
		if (!this.hasData(token)) return;
		this.data = undefined;
		this.token = undefined;
	}
}
