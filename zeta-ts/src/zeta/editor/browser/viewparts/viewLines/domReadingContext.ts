/** Caches the line-root geometry shared by DOM range reads in one render pass. */
export class DomReadingContext {
	private didReadClientRect = false;
	private cachedClientRectDeltaLeft = 0;
	private cachedClientRectScale = 1;
	private domLayoutOccurred = false;

	constructor(
		private readonly domNode: HTMLElement,
		public readonly endNode: HTMLElement,
	) {}

	public get didDomLayout(): boolean {
		return this.domLayoutOccurred;
	}

	public get clientRectDeltaLeft(): number {
		this.readClientRect();
		return this.cachedClientRectDeltaLeft;
	}

	public get clientRectScale(): number {
		this.readClientRect();
		return this.cachedClientRectScale;
	}

	public markDidDomLayout(): void {
		this.domLayoutOccurred = true;
	}

	private readClientRect(): void {
		if (this.didReadClientRect) return;
		this.didReadClientRect = true;
		const rectangle = this.domNode.getBoundingClientRect();
		this.markDidDomLayout();
		this.cachedClientRectDeltaLeft = rectangle.left;
		this.cachedClientRectScale = this.domNode.offsetWidth > 0
			? rectangle.width / this.domNode.offsetWidth
			: 1;
	}
}
