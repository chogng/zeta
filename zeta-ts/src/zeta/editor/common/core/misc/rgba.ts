/** A compact integer RGBA value used by pixel-oriented editor projections. */
export class RGBA8 {
	static readonly Empty = new RGBA8(0, 0, 0, 0);

	readonly r: number;
	readonly g: number;
	readonly b: number;
	readonly a: number;

	constructor(r: number, g: number, b: number, a: number) {
		this.r = clamp(r);
		this.g = clamp(g);
		this.b = clamp(b);
		this.a = clamp(a);
	}

	equals(other: RGBA8): boolean { return this.r === other.r && this.g === other.g && this.b === other.b && this.a === other.a; }
}

function clamp(value: number): number { return Math.min(255, Math.max(0, value | 0)); }
