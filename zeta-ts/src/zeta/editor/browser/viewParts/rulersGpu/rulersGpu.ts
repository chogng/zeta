import { DisposableStore } from '../../../../base/common/lifecycle.js';
import { type ViewGpuContext } from '../../gpu/viewGpuContext.js';
import { type EditorRenderingContext, ViewPart } from '../../view/viewPart.js';
import { type ViewContext } from '../../../common/viewModel/viewContext.js';
import { type EditorRuler, validateRuler } from '../rulers/rulers.js';

/** Projects configured rulers into the shared GPU rectangle buffer. */
export class RulersGpu extends ViewPart {
	private readonly entries = this._register(new DisposableStore());
	private readonly rulers: readonly EditorRuler[];

	constructor(context: ViewContext, private readonly gpuContext: ViewGpuContext, rulers: readonly EditorRuler[], private readonly measureColumn: (column: number) => number) {
		super(context);
		this.rulers = Object.freeze(rulers.map(validateRuler));
	}

	public render(context: EditorRenderingContext): void {
		this.entries.clear();
		if (this.gpuContext.status !== 'ready') return;
		const devicePixelRatio = this.gpuContext.devicePixelRatio;
		for (const ruler of this.rulers) {
			const color = parseColor(ruler.color);
			this.entries.add(this.gpuContext.rectangleRenderer.register(
				this.measureColumn(ruler.column) * devicePixelRatio,
				0,
				Math.max(1, Math.ceil(devicePixelRatio)),
				Math.min(context.layout.contentSize.height * devicePixelRatio, 1_000_000),
				color[0], color[1], color[2], color[3],
			));
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
