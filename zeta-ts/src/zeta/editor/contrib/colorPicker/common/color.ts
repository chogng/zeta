import { type Event } from '../../../../base/common/event.js';
import { type URI } from '../../../../base/common/uri.js';
import { type TextEdit, TextRange } from '../../../common/core/text.js';
import { RGBA8 } from '../../../common/core/misc/rgba.js';
import { createLanguageFeatureRequest, isLanguageFeatureRequestCurrent, type LanguageFeatureRequest } from '../../../common/languages/languageFeatureRequest.js';
import { LanguageFeatureProviderRegistry, type LanguageFeatureProviderMetadata } from '../../../common/languageFeatureRegistry.js';
import { type TextModel } from '../../../common/model/textModel.js';

export interface LanguageColorInformation {
	readonly range: TextRange;
	readonly color: RGBA8;
}

export interface LanguageColorPresentation {
	readonly label: string;
	readonly textEdit?: TextEdit;
	readonly additionalTextEdits?: readonly TextEdit[];
}

export interface LanguageColorRequest extends LanguageFeatureRequest {
	readonly resource?: URI;
}

export interface LanguageColorPresentationRequest extends LanguageFeatureRequest {
	readonly color: RGBA8;
	readonly range: TextRange;
	readonly resource?: URI;
}

export interface LanguageColorProvider extends LanguageFeatureProviderMetadata {
	provideDocumentColors(request: LanguageColorRequest, signal: AbortSignal): readonly LanguageColorInformation[] | undefined | Promise<readonly LanguageColorInformation[] | undefined>;
	provideColorPresentations(request: LanguageColorPresentationRequest, signal: AbortSignal): readonly LanguageColorPresentation[] | undefined | Promise<readonly LanguageColorPresentation[] | undefined>;
}

export interface ColorData {
	readonly information: LanguageColorInformation;
	readonly provider: LanguageColorProvider;
}

export type DefaultColorDecoratorsEnablement = 'auto' | 'always' | 'never';

/** Resolves version-bound document colors while retaining the provider that owns each presentation. */
export class ColorService {
	private readonly defaultProvider = new DefaultDocumentColorProvider();
	readonly onDidChange: Event<void>;

	constructor(
		private readonly model: TextModel,
		private readonly providers: LanguageFeatureProviderRegistry<LanguageColorProvider>,
		private readonly resource?: URI,
		private readonly onError: (error: unknown) => void = error => console.error('Stanza document color provider failed', error),
	) {
		this.onDidChange = providers.onDidChange;
	}

	async provideDocumentColors(languageId: string, enablement: DefaultColorDecoratorsEnablement, signal: AbortSignal): Promise<readonly ColorData[]> {
		const request = this.createRequest(languageId, signal);
		const result: ColorData[] = [];
		let validProviderFound = false;
		for (const provider of this.providers.getProviders(languageId)) {
			const colors = await this.requestDocumentColors(provider, request, signal);
			if (!isLanguageFeatureRequestCurrent(request)) return Object.freeze([]);
			if (!colors) continue;
			validProviderFound = true;
			result.push(...this.normalizeColors(colors, provider));
		}
		if (enablement === 'always' || enablement === 'auto' && !validProviderFound) {
			const colors = await this.defaultProvider.provideDocumentColors(request, signal);
			if (!isLanguageFeatureRequestCurrent(request)) return Object.freeze([]);
			result.push(...this.normalizeColors(colors, this.defaultProvider));
		}
		return Object.freeze(result);
	}

	async provideColorPresentations(languageId: string, data: ColorData, color: RGBA8, signal: AbortSignal): Promise<readonly LanguageColorPresentation[]> {
		const request: LanguageColorPresentationRequest = Object.freeze({
			...this.createRequest(languageId, signal),
			color,
			range: data.information.range,
		});
		try {
			const values = await data.provider.provideColorPresentations(request, signal);
			if (!isLanguageFeatureRequestCurrent(request)) return Object.freeze([]);
			return Object.freeze((values?.length ? values : createColorPresentations(request.range, color)).map(normalizePresentation));
		} catch (error) {
			if (signal.aborted) return Object.freeze([]);
			this.onError(error);
			return Object.freeze(createColorPresentations(request.range, color).map(normalizePresentation));
		}
	}

	private createRequest(languageId: string, signal: AbortSignal): LanguageColorRequest {
		return Object.freeze({
			...createLanguageFeatureRequest(this.model, languageId, signal),
			...(this.resource ? { resource: this.resource } : {}),
		});
	}

	private async requestDocumentColors(provider: LanguageColorProvider, request: LanguageColorRequest, signal: AbortSignal): Promise<readonly LanguageColorInformation[] | undefined> {
		try {
			return await provider.provideDocumentColors(request, signal);
		} catch (error) {
			if (!signal.aborted) this.onError(error);
			return undefined;
		}
	}

	private normalizeColors(values: readonly LanguageColorInformation[], provider: LanguageColorProvider): readonly ColorData[] {
		return values.map(value => {
			this.model.offsetAt(value.range.start);
			this.model.offsetAt(value.range.end);
			if (!(value.color instanceof RGBA8)) throw new TypeError('Language color provider must return RGBA8 values');
			return Object.freeze({
				information: Object.freeze({ range: value.range, color: value.color }),
				provider,
			});
		});
	}
}

/** Detects common CSS color literals when a language does not provide document colors. */
export class DefaultDocumentColorProvider implements LanguageColorProvider {
	readonly languageIds = Object.freeze(['*']);
	readonly providerId = 'stanza.defaultDocumentColorProvider';

	provideDocumentColors(request: LanguageColorRequest, signal: AbortSignal): readonly LanguageColorInformation[] {
		const text = request.snapshot.getText();
		const colors: LanguageColorInformation[] = [];
		for (const match of text.matchAll(COLOR_LITERAL_PATTERN)) {
			signal.throwIfAborted();
			const value = parseColorLiteral(match[0]);
			if (!value || match.index === undefined) continue;
			colors.push(Object.freeze({
				range: TextRange.from(request.model.positionAt(match.index), request.model.positionAt(match.index + match[0].length)),
				color: value,
			}));
		}
		return Object.freeze(colors);
	}

	provideColorPresentations(request: LanguageColorPresentationRequest): readonly LanguageColorPresentation[] {
		return createColorPresentations(request.range, request.color);
	}
}

const COLOR_LITERAL_PATTERN = /#(?:[0-9a-f]{8}|[0-9a-f]{6}|[0-9a-f]{4}|[0-9a-f]{3})(?![0-9a-f])|\b(?:rgba?|hsla?)\([^)]*\)|\btransparent\b/giu;

function parseColorLiteral(value: string): RGBA8 | undefined {
	const normalized = value.trim().toLowerCase();
	if (normalized === 'transparent') return new RGBA8(0, 0, 0, 0);
	if (normalized.startsWith('#')) return parseHex(normalized);
	if (normalized.startsWith('rgb')) return parseRgb(normalized);
	if (normalized.startsWith('hsl')) return parseHsl(normalized);
	return undefined;
}

function parseHex(value: string): RGBA8 | undefined {
	const hex = value.slice(1);
	if (![3, 4, 6, 8].includes(hex.length) || !/^[0-9a-f]+$/u.test(hex)) return undefined;
	const expanded = hex.length <= 4 ? [...hex].map(digit => digit + digit).join('') : hex;
	return new RGBA8(
		Number.parseInt(expanded.slice(0, 2), 16),
		Number.parseInt(expanded.slice(2, 4), 16),
		Number.parseInt(expanded.slice(4, 6), 16),
		expanded.length === 8 ? Number.parseInt(expanded.slice(6, 8), 16) : 255,
	);
}

function parseRgb(value: string): RGBA8 | undefined {
	const parts = colorFunctionParts(value);
	if (!parts || parts.channels.length !== 3) return undefined;
	const channels = parts.channels.map(parseRgbChannel);
	const alpha = parseAlpha(parts.alpha);
	if (channels.some(channel => channel === undefined) || alpha === undefined) return undefined;
	return new RGBA8(channels[0]!, channels[1]!, channels[2]!, alpha);
}

function parseHsl(value: string): RGBA8 | undefined {
	const parts = colorFunctionParts(value);
	if (!parts || parts.channels.length !== 3) return undefined;
	const hue = parseHue(parts.channels[0]!);
	const saturation = parsePercentage(parts.channels[1]!);
	const lightness = parsePercentage(parts.channels[2]!);
	const alpha = parseAlpha(parts.alpha);
	if (hue === undefined || saturation === undefined || lightness === undefined || alpha === undefined) return undefined;
	const chroma = (1 - Math.abs(2 * lightness - 1)) * saturation;
	const section = hue / 60;
	const secondary = chroma * (1 - Math.abs(section % 2 - 1));
	const [red, green, blue] = section < 1 ? [chroma, secondary, 0]
		: section < 2 ? [secondary, chroma, 0]
			: section < 3 ? [0, chroma, secondary]
				: section < 4 ? [0, secondary, chroma]
					: section < 5 ? [secondary, 0, chroma]
						: [chroma, 0, secondary];
	const match = lightness - chroma / 2;
	return new RGBA8(
		Math.round((red + match) * 255),
		Math.round((green + match) * 255),
		Math.round((blue + match) * 255),
		alpha,
	);
}

function colorFunctionParts(value: string): { readonly channels: readonly string[]; readonly alpha?: string } | undefined {
	const start = value.indexOf('(');
	const end = value.lastIndexOf(')');
	if (start < 0 || end <= start) return undefined;
	const body = value.slice(start + 1, end).trim();
	if (!body) return undefined;
	if (body.includes(',')) {
		if (body.includes('/')) return undefined;
		const items = body.split(',').map(item => item.trim());
		if ((items.length !== 3 && items.length !== 4) || items.some(item => item.length === 0)) return undefined;
		return { channels: items.slice(0, 3), ...(items[3] ? { alpha: items[3] } : {}) };
	}
	const slashParts = body.split('/').map(item => item.trim());
	if (slashParts.length > 2 || slashParts.some(item => item.length === 0)) return undefined;
	const channels = slashParts[0]!.split(/\s+/u);
	if (channels.length !== 3) return undefined;
	return { channels, ...(slashParts[1] ? { alpha: slashParts[1] } : {}) };
}

function parseRgbChannel(value: string): number | undefined {
	if (value.endsWith('%')) {
		const percent = parseNumber(value.slice(0, -1));
		return Number.isFinite(percent) ? Math.round(Math.min(100, Math.max(0, percent)) * 2.55) : undefined;
	}
	const channel = parseNumber(value);
	return Number.isFinite(channel) ? Math.round(Math.min(255, Math.max(0, channel))) : undefined;
}

function parseAlpha(value: string | undefined): number | undefined {
	if (value === undefined) return 255;
	if (value.endsWith('%')) {
		const percent = parseNumber(value.slice(0, -1));
		return Number.isFinite(percent) ? Math.round(Math.min(100, Math.max(0, percent)) * 2.55) : undefined;
	}
	const alpha = parseNumber(value);
	return Number.isFinite(alpha) ? Math.round(Math.min(1, Math.max(0, alpha)) * 255) : undefined;
}

function parseHue(value: string): number | undefined {
	const unit = /(?:deg|grad|rad|turn)$/u.exec(value)?.[0] ?? '';
	const hue = parseNumber(unit ? value.slice(0, -unit.length) : value);
	if (!Number.isFinite(hue)) return undefined;
	const degrees = unit === 'turn' ? hue * 360 : unit === 'rad' ? hue * 180 / Math.PI : unit === 'grad' ? hue * 0.9 : hue;
	return (degrees % 360 + 360) % 360;
}

function parsePercentage(value: string): number | undefined {
	if (!value.endsWith('%')) return undefined;
	const percent = parseNumber(value.slice(0, -1));
	return Number.isFinite(percent) ? Math.min(100, Math.max(0, percent)) / 100 : undefined;
}

function parseNumber(value: string): number {
	return /^[+-]?(?:\d+\.?\d*|\.\d+)(?:e[+-]?\d+)?$/iu.test(value) ? Number(value) : Number.NaN;
}

function createColorPresentations(range: TextRange, color: RGBA8): readonly LanguageColorPresentation[] {
	const alpha = rounded(color.a / 255, 3);
	const rgb = color.a === 255 ? `rgb(${color.r}, ${color.g}, ${color.b})` : `rgba(${color.r}, ${color.g}, ${color.b}, ${alpha})`;
	const [hue, saturation, lightness] = rgbToHsl(color);
	const hsl = color.a === 255 ? `hsl(${hue}, ${saturation}%, ${lightness}%)` : `hsla(${hue}, ${saturation}%, ${lightness}%, ${alpha})`;
	const hex = `#${[color.r, color.g, color.b, ...(color.a === 255 ? [] : [color.a])].map(channel => channel.toString(16).padStart(2, '0')).join('')}`;
	return Object.freeze([rgb, hsl, hex].map(label => Object.freeze({ label, textEdit: Object.freeze({ range, text: label }) })));
}

function rgbToHsl(color: RGBA8): readonly [number, number, number] {
	const red = color.r / 255;
	const green = color.g / 255;
	const blue = color.b / 255;
	const maximum = Math.max(red, green, blue);
	const minimum = Math.min(red, green, blue);
	const delta = maximum - minimum;
	const lightness = (maximum + minimum) / 2;
	const saturation = delta === 0 ? 0 : delta / (1 - Math.abs(2 * lightness - 1));
	let hue = 0;
	if (delta !== 0) {
		if (maximum === red) hue = 60 * (((green - blue) / delta) % 6);
		else if (maximum === green) hue = 60 * ((blue - red) / delta + 2);
		else hue = 60 * ((red - green) / delta + 4);
	}
	return Object.freeze([Math.round((hue + 360) % 360), Math.round(saturation * 100), Math.round(lightness * 100)]);
}

function normalizePresentation(value: LanguageColorPresentation): LanguageColorPresentation {
	if (!value || typeof value.label !== 'string' || value.label.trim().length === 0) throw new TypeError('Language color presentation must provide a label');
	return Object.freeze({
		label: value.label,
		...(value.textEdit ? { textEdit: Object.freeze({ range: value.textEdit.range, text: value.textEdit.text }) } : {}),
		...(value.additionalTextEdits ? { additionalTextEdits: Object.freeze([...value.additionalTextEdits]) } : {}),
	});
}

function rounded(value: number, digits: number): number {
	const factor = 10 ** digits;
	return Math.round(value * factor) / factor;
}
