import { DisposableStore } from '../../../../base/common/lifecycle.js';
import { type RectangleRenderer } from '../../gpu/rectangleRenderer.js';
import { type EditorRenderingContext, EditorViewPart } from '../../view/viewPart.js';
import { type EditorRuler } from '../rulers/rulers.js';

/** Projects configured rulers into the shared GPU rectangle buffer. */
export class RulersGpu extends EditorViewPart {
	private readonly entries = this._register(new DisposableStore());

	constructor(private readonly renderer: RectangleRenderer, private readonly rulers: readonly EditorRuler[], private readonly measureColumn: (column: number) => number) {
		super();
	}

	public render(context: EditorRenderingContext): void {
		this.entries.clear();
		for (const ruler of this.rulers) {
			const color = parseColor(ruler.color);
			this.entries.add(this.renderer.register(this.measureColumn(ruler.column), 0, 1, context.layout.contentSize.height, color[0], color[1], color[2], color[3]));
		}
	}
}

function parseColor(value: string | undefined): readonly [number, number, number, number] {
	if (!value) {
		return [0.5, 0.5, 0.5, 0.35];
	}
	const match = /^#([0-9a-f]{6})([0-9a-f]{2})?$/iu.exec(value);
	if (!match) {
		throw new TypeError('GPU ruler colors must use hexadecimal RGB or RGBA');
	}
	const rgb = match[1]!;
	return [
		Number.parseInt(rgb.slice(0, 2), 16) / 255,
		Number.parseInt(rgb.slice(2, 4), 16) / 255,
		Number.parseInt(rgb.slice(4, 6), 16) / 255,
		match[2] ? Number.parseInt(match[2], 16) / 255 : 1,
	];
}
