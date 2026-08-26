import { h } from '../../../base/browser/dom.js';

/** Reads shaped text widths from one browser-owned measurement environment. */
export interface CharWidthReader {
	setFont(font: string): void;
	measureText(text: string): number | undefined;
}

/** Canvas-backed width reader used by the current browser text measurer. */
export class CanvasCharWidthReader implements CharWidthReader {
	private readonly context: CanvasRenderingContext2D | undefined;

	public constructor(ownerDocument: Document) {
		this.context = createCanvasContext(ownerDocument);
	}

	public setFont(font: string): void {
		if (!this.context) return;
		this.context.font = font;
		this.context.textBaseline = 'alphabetic';
	}

	public measureText(text: string): number | undefined {
		return this.context?.measureText(text).width;
	}
}

function createCanvasContext(ownerDocument: Document): CanvasRenderingContext2D | undefined {
	try {
		return h(ownerDocument, 'canvas').getContext('2d') ?? undefined;
	} catch {
		return undefined;
	}
}
