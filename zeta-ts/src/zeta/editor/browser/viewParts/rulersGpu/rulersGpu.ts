import { Color } from '../../../../base/common/color.js';
import { toDisposable } from '../../../../base/common/lifecycle.js';
import { type IObjectCollectionBufferEntry } from '../../gpu/objectCollectionBuffer.js';
import { type RectangleRenderer, type RectangleRendererEntrySpec } from '../../gpu/rectangleRenderer.js';
import { type ViewGpuContext } from '../../gpu/viewGpuContext.js';
import { type RenderingContext, type RestrictedRenderingContext } from '../../view/renderingContext.js';
import { ViewPart } from '../../view/viewPart.js';
import { EditorOption } from '../../../common/config/editorOptions.js';
import { type ViewContext } from '../../../common/viewModel/viewContext.js';
import * as viewEvents from '../../../common/viewEvents.js';
import { editorRulerForeground } from '../../../../platform/theme/common/colors/editorColors.js';
import { type IColorTheme } from '../../../../platform/theme/common/colorTheme.js';

/** Renders configured editor rulers into the shared GPU rectangle buffer. */
export class RulersGpu extends ViewPart {
	private readonly shapes: IObjectCollectionBufferEntry<RectangleRendererEntrySpec>[] = [];
	private entriesDirty = true;
	private lastDevicePixelRatio = Number.NaN;
	private lastTextLeft = Number.NaN;
	private lastTheme: IColorTheme | undefined;

	constructor(
		context: ViewContext,
		private readonly gpuContext: ViewGpuContext,
		private readonly readTextLeft: () => number,
	) {
		super(context);
		this._register(toDisposable(() => {
			while (this.shapes.length > 0) this.shapes.pop()!.dispose();
		}));
	}

	public override onConfigurationChanged(event: viewEvents.ViewConfigurationChangedEvent): boolean {
		const changed = event.hasChanged(EditorOption.rulers) || event.hasChanged(EditorOption.fontInfo);
		this.entriesDirty ||= changed;
		return changed;
	}

	public override prepareRender(_context: RenderingContext): void {
	}

	public render(_context: RestrictedRenderingContext): void {
		if (this.gpuContext.status !== 'ready') return;
		const devicePixelRatio = this.gpuContext.devicePixelRatio;
		const textLeft = this.readTextLeft();
		const theme = this._context.theme.value;
		if (!this.entriesDirty && devicePixelRatio === this.lastDevicePixelRatio && textLeft === this.lastTextLeft && theme === this.lastTheme) return;
		this.updateEntries(devicePixelRatio, textLeft);
		this.entriesDirty = false;
		this.lastDevicePixelRatio = devicePixelRatio;
		this.lastTextLeft = textLeft;
		this.lastTheme = theme;
	}

	private updateEntries(devicePixelRatio: number, textLeft: number): void {
		const editorOptions = this._context.configuration.options;
		const rulers = editorOptions.get(EditorOption.rulers);
		const typicalHalfwidthCharacterWidth = editorOptions.get(EditorOption.fontInfo).typicalHalfwidthCharacterWidth;
		for (let index = 0; index < rulers.length; index += 1) {
			const ruler = rulers[index]!;
			const color = ruler.color
				? Color.fromHex(ruler.color)
				: this._context.theme.getColor(editorRulerForeground) ?? Color.white;
			const entry: Parameters<RectangleRenderer['register']> = [
				(textLeft + ruler.column * typicalHalfwidthCharacterWidth) * devicePixelRatio,
				0,
				Math.max(1, Math.ceil(devicePixelRatio)),
				Number.MAX_SAFE_INTEGER,
				color.rgba.r / 255,
				color.rgba.g / 255,
				color.rgba.b / 255,
				color.rgba.a,
			];
			if (this.shapes[index]) this.shapes[index]!.setRaw(entry);
			else this.shapes.push(this.gpuContext.rectangleRenderer.register(...entry));
		}
		while (this.shapes.length > rulers.length) this.shapes.pop()!.dispose();
	}
}
