const MINIMUM_HEIGHT = 4;

export class ColorZone {
	constructor(
		public readonly from: number,
		public readonly to: number,
		public readonly colorId: number,
	) {}

	public static compare(left: ColorZone, right: ColorZone): number {
		return left.colorId - right.colorId || left.from - right.from || left.to - right.to;
	}
}

/** A model-line interval projected into the overview ruler. */
export class OverviewRulerZone {
	private colorZone: ColorZone | null = null;

	constructor(
		public readonly startLineIndex: number,
		public readonly endLineIndexExclusive: number,
		public readonly heightInLines: number,
		public readonly color: string,
	) {
		if (!Number.isSafeInteger(startLineIndex) || startLineIndex < 0) throw new RangeError('Overview ruler start line index must be non-negative');
		if (!Number.isSafeInteger(endLineIndexExclusive) || endLineIndexExclusive <= startLineIndex) throw new RangeError('Overview ruler end line index must follow its start');
		if (!Number.isSafeInteger(heightInLines) || heightInLines < 0) throw new RangeError('Overview ruler height must be a non-negative safe integer');
		if (typeof color !== 'string' || color.length === 0) throw new TypeError('Overview ruler color must be a non-empty string');
	}

	public static compare(left: OverviewRulerZone, right: OverviewRulerZone): number {
		return left.color.localeCompare(right.color)
			|| left.startLineIndex - right.startLineIndex
			|| left.heightInLines - right.heightInLines
			|| left.endLineIndexExclusive - right.endLineIndexExclusive;
	}

	public setColorZone(colorZone: ColorZone): void {
		this.colorZone = colorZone;
	}

	public getColorZone(): ColorZone | null {
		return this.colorZone;
	}
}

/** Maps model-line zones to stable overview-ruler pixel intervals. */
export class OverviewZoneManager {
	private zones: OverviewRulerZone[] = [];
	private colorZonesInvalid = true;
	private lineHeight = 0;
	private domWidth = 0;
	private domHeight = 0;
	private outerHeight = 0;
	private pixelRatio = 1;
	private lastAssignedId = 0;
	private readonly colorToId = new Map<string, number>();
	private readonly idToColor: string[] = [];

	constructor(private readonly getVerticalOffsetForLineIndex: (lineIndex: number) => number) {}

	public getIdToColor(): readonly string[] {
		return this.idToColor;
	}

	public setZones(zones: readonly OverviewRulerZone[]): void {
		this.zones = [...zones].sort(OverviewRulerZone.compare);
		this.colorZonesInvalid = true;
	}

	public setLineHeight(lineHeight: number): boolean {
		return this.setDimension('lineHeight', lineHeight);
	}

	public setPixelRatio(pixelRatio: number): boolean {
		return this.setDimension('pixelRatio', pixelRatio);
	}

	public getDOMWidth(): number { return this.domWidth; }
	public getCanvasWidth(): number { return this.domWidth * this.pixelRatio; }
	public setDOMWidth(width: number): boolean { return this.setDimension('domWidth', width); }
	public getDOMHeight(): number { return this.domHeight; }
	public getCanvasHeight(): number { return this.domHeight * this.pixelRatio; }
	public setDOMHeight(height: number): boolean { return this.setDimension('domHeight', height); }
	public getOuterHeight(): number { return this.outerHeight; }
	public setOuterHeight(height: number): boolean { return this.setDimension('outerHeight', height); }

	public resolveColorZones(): readonly ColorZone[] {
		if (this.outerHeight <= 0 || this.domHeight <= 0) return [];
		const totalHeight = Math.floor(this.getCanvasHeight());
		const heightRatio = totalHeight / this.outerHeight;
		const halfMinimumHeight = Math.floor(MINIMUM_HEIGHT * this.pixelRatio / 2);
		const result: ColorZone[] = [];
		for (const zone of this.zones) {
			const cached = zone.getColorZone();
			if (!this.colorZonesInvalid && cached) {
				result.push(cached);
				continue;
			}
			const offsetStart = this.getVerticalOffsetForLineIndex(zone.startLineIndex);
			const offsetEnd = zone.heightInLines === 0
				? this.getVerticalOffsetForLineIndex(zone.endLineIndexExclusive)
				: offsetStart + zone.heightInLines * Math.floor(this.lineHeight);
			const first = Math.floor(heightRatio * offsetStart);
			const last = Math.floor(heightRatio * offsetEnd);
			let center = Math.floor((first + last) / 2);
			let halfHeight = Math.min(Math.max(last - center, halfMinimumHeight), Math.floor(totalHeight / 2));
			if (center - halfHeight < 0) center = halfHeight;
			if (center + halfHeight > totalHeight) center = totalHeight - halfHeight;
			const colorId = this.colorId(zone.color);
			const colorZone = new ColorZone(Math.max(0, center - halfHeight), Math.min(totalHeight, center + halfHeight), colorId);
			zone.setColorZone(colorZone);
			result.push(colorZone);
		}
		this.colorZonesInvalid = false;
		return result.sort(ColorZone.compare);
	}

	private colorId(color: string): number {
		const current = this.colorToId.get(color);
		if (current !== undefined) return current;
		const id = ++this.lastAssignedId;
		this.colorToId.set(color, id);
		this.idToColor[id] = color;
		return id;
	}

	private setDimension(property: 'lineHeight' | 'domWidth' | 'domHeight' | 'outerHeight' | 'pixelRatio', value: number): boolean {
		if (!Number.isFinite(value) || value < 0 || (property === 'pixelRatio' && value === 0)) throw new RangeError(`Overview ruler ${property} must be finite and non-negative`);
		if (this[property] === value) return false;
		this[property] = value;
		this.colorZonesInvalid = true;
		return true;
	}
}
