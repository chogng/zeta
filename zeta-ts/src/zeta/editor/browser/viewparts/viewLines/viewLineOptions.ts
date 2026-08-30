export enum ViewLineTextDirection {
	Auto = 'auto',
	LeftToRight = 'ltr',
	RightToLeft = 'rtl',
}

export interface ViewLineOptionsConfiguration {
	readonly textDirection: ViewLineTextDirection;
	readonly fontLigatures: boolean;
	readonly useGpu: boolean;
	readonly useMonospaceOptimizations: boolean;
	readonly lineHeight: number;
	readonly tabSize: number;
}

/** Immutable rendering configuration shared by DOM and GPU line renderers. */
export class ViewLineOptions {
	public readonly textDirection: ViewLineTextDirection;
	public readonly fontLigatures: boolean;
	public readonly useGpu: boolean;
	public readonly useMonospaceOptimizations: boolean;
	public readonly lineHeight: number;
	public readonly tabSize: number;

	constructor(configuration: ViewLineOptionsConfiguration) {
		if (!Object.values(ViewLineTextDirection).includes(configuration.textDirection)) {
			throw new TypeError('Unknown Stanza editor text direction');
		}
		if (typeof configuration.fontLigatures !== 'boolean' || typeof configuration.useGpu !== 'boolean' || typeof configuration.useMonospaceOptimizations !== 'boolean') {
			throw new TypeError('Stanza view-line flags must be boolean');
		}
		if (!Number.isFinite(configuration.lineHeight) || configuration.lineHeight <= 0) throw new RangeError('Stanza view-line height must be positive');
		if (!Number.isSafeInteger(configuration.tabSize) || configuration.tabSize < 1) throw new RangeError('Stanza view-line tab size must be a positive safe integer');
		this.textDirection = configuration.textDirection;
		this.fontLigatures = configuration.fontLigatures;
		this.useGpu = configuration.useGpu;
		this.useMonospaceOptimizations = configuration.useMonospaceOptimizations;
		this.lineHeight = configuration.lineHeight;
		this.tabSize = configuration.tabSize;
	}

	public equals(other: ViewLineOptions): boolean {
		return this.textDirection === other.textDirection &&
			this.fontLigatures === other.fontLigatures &&
			this.useGpu === other.useGpu &&
			this.useMonospaceOptimizations === other.useMonospaceOptimizations &&
			this.lineHeight === other.lineHeight &&
			this.tabSize === other.tabSize;
	}
}
