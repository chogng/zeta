import { clamp } from "./numbers.js";

function byte(value: number): string {
	return Math.round(clamp(value, 0, 1) * 255).toString(16).padStart(2, "0");
}

/** Immutable RGBA color used by domain-agnostic rendering infrastructure. */
export class Color {
	readonly red: number;
	readonly green: number;
	readonly blue: number;
	readonly alpha: number;

	constructor(red: number, green: number, blue: number, alpha = 1) {
		this.red = clamp(red, 0, 1);
		this.green = clamp(green, 0, 1);
		this.blue = clamp(blue, 0, 1);
		this.alpha = clamp(alpha, 0, 1);
		Object.freeze(this);
	}

	static fromHex(value: string): Color {
		const hex = value.trim().replace(/^#/, "");
		if (![3, 4, 6, 8].includes(hex.length) || !/^[0-9a-f]+$/i.test(hex)) {
			throw new TypeError(`Invalid hexadecimal color '${value}'`);
		}
		const expanded = hex.length <= 4 ? [...hex].map((digit) => digit + digit).join("") : hex;
		const red = Number.parseInt(expanded.slice(0, 2), 16) / 255;
		const green = Number.parseInt(expanded.slice(2, 4), 16) / 255;
		const blue = Number.parseInt(expanded.slice(4, 6), 16) / 255;
		const alpha = expanded.length === 8 ? Number.parseInt(expanded.slice(6, 8), 16) / 255 : 1;
		return new Color(red, green, blue, alpha);
	}

	transparent(factor: number): Color {
		return new Color(this.red, this.green, this.blue, this.alpha * clamp(factor, 0, 1));
	}

	lighten(factor: number): Color {
		return this.mix(new Color(1, 1, 1), factor);
	}

	darken(factor: number): Color {
		return this.mix(new Color(0, 0, 0), factor);
	}

	mix(other: Color, factor: number): Color {
		const amount = clamp(factor, 0, 1);
		return new Color(
			this.red + (other.red - this.red) * amount,
			this.green + (other.green - this.green) * amount,
			this.blue + (other.blue - this.blue) * amount,
			this.alpha + (other.alpha - this.alpha) * amount,
		);
	}

	makeOpaque(background: Color): Color {
		if (this.alpha === 1) return this;
		return new Color(
			this.red * this.alpha + background.red * (1 - this.alpha),
			this.green * this.alpha + background.green * (1 - this.alpha),
			this.blue * this.alpha + background.blue * (1 - this.alpha),
		);
	}

	toString(): string {
		const rgb = `${byte(this.red)}${byte(this.green)}${byte(this.blue)}`;
		return `#${rgb}${this.alpha === 1 ? "" : byte(this.alpha)}`;
	}
}
