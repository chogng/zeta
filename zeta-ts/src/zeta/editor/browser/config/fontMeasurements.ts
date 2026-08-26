import { type TextMeasurer as CommonTextMeasurer } from '../../common/viewModel/textMeasurer.js';
import { CanvasCharWidthReader, type CharWidthReader } from './charWidthReader.js';

/** Browser font state consumed by text measurement and viewport layout. */
export interface FontMeasurementSnapshot {
	readonly signature: string;
	readonly canvasFont: string;
	readonly letterSpacing: number;
	readonly spaceWidth: number;
	readonly tabSize: number;
	readonly horizontalPadding: number;
	readonly contentLeftPadding: number;
	readonly fallbackCharacterWidth: number;
}

/** Reads the effective font environment and configures its width reader. */
export function readFontMeasurements(referenceElement: HTMLElement, charWidthReader: CharWidthReader): FontMeasurementSnapshot {
	const ownerWindow = referenceElement.ownerDocument.defaultView;
	if (!ownerWindow) throw new ReferenceError('Stanza font measurement requires a browser window');
	const style = ownerWindow.getComputedStyle(referenceElement);
	const fontSize = positiveCssNumber(style.fontSize, 14);
	const letterSpacing = style.letterSpacing === 'normal' ? 0 : cssNumber(style.letterSpacing, 0);
	const tabSize = positiveCssNumber(style.tabSize, 4);
	const contentLeftPadding = cssNumber(style.paddingLeft, 0);
	const horizontalPadding = contentLeftPadding + cssNumber(style.paddingRight, 0);
	const canvasFont = [
		style.fontStyle || 'normal',
		style.fontVariant || 'normal',
		style.fontWeight || '400',
		style.fontStretch || 'normal',
		style.fontSize || `${fontSize}px`,
		style.fontFamily || 'monospace',
	].join(' ');
	const fallbackCharacterWidth = fontSize * 0.6;
	charWidthReader.setFont(canvasFont);
	const spaceWidth = positiveNumber(charWidthReader.measureText(' '), fallbackCharacterWidth);
	return Object.freeze({
		signature: JSON.stringify([
			canvasFont,
			letterSpacing,
			style.fontFeatureSettings,
			style.fontKerning,
			style.fontVariationSettings,
			tabSize,
			horizontalPadding,
			contentLeftPadding,
			spaceWidth,
		]),
		canvasFont,
		letterSpacing,
		spaceWidth,
		tabSize,
		horizontalPadding,
		contentLeftPadding,
		fallbackCharacterWidth,
	});
}

/** Browser-backed line measurer using one resolved font environment. */
export interface TextMeasurer extends CommonTextMeasurer {
	refresh(): boolean;
}

/** Measures line text with the current browser font and tab-stop configuration. */
export class DomTextMeasurer implements TextMeasurer {
	private readonly charWidthReader: CharWidthReader;
	private metrics: FontMeasurementSnapshot;

	public constructor(private readonly referenceElement: HTMLElement) {
		this.charWidthReader = new CanvasCharWidthReader(referenceElement.ownerDocument);
		this.metrics = readFontMeasurements(referenceElement, this.charWidthReader);
	}

	public get horizontalPadding(): number {
		return this.metrics.horizontalPadding;
	}

	public get contentLeftPadding(): number {
		return this.metrics.contentLeftPadding;
	}

	public refresh(): boolean {
		const next = readFontMeasurements(this.referenceElement, this.charWidthReader);
		if (next.signature === this.metrics.signature) return false;
		this.metrics = next;
		return true;
	}

	public measureLineWidth(text: string): number {
		if (!text.includes('\t')) return this.measureSegment(text);
		const tabStopWidth = this.metrics.spaceWidth * this.metrics.tabSize;
		let width = 0;
		const segments = text.split('\t');
		for (let index = 0; index < segments.length; index++) {
			width += this.measureSegment(segments[index] ?? '');
			if (index + 1 < segments.length) width = (Math.floor(width / tabStopWidth) + 1) * tabStopWidth;
		}
		return width;
	}

	private measureSegment(text: string): number {
		if (!text) return 0;
		const characterCount = [...text].length;
		const width = this.charWidthReader.measureText(text) ?? characterCount * this.metrics.fallbackCharacterWidth;
		return Math.max(0, width + characterCount * this.metrics.letterSpacing);
	}
}

function cssNumber(value: string, fallback: number): number {
	const parsed = Number.parseFloat(value);
	return Number.isFinite(parsed) ? parsed : fallback;
}

function positiveCssNumber(value: string, fallback: number): number {
	return positiveNumber(cssNumber(value, fallback), fallback);
}

function positiveNumber(value: number | undefined, fallback: number): number {
	return value !== undefined && Number.isFinite(value) && value > 0 ? value : fallback;
}
