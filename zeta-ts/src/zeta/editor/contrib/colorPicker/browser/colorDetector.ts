import { disposableWindowTimeout } from '../../../../base/browser/scheduler.js';
import { MutableDisposable, Disposable, type IDisposable } from '../../../../base/common/lifecycle.js';
import { type Position } from '../../../common/core/position.js';
import { type TextDecorationId, TextDecorationCollection, type TextDecorationSnapshot } from '../../../common/model/decorationCollection.js';
import { type TextModel } from '../../../common/model/textModel.js';

import { createStanzaDecorationSource, DecorationPresentation, type DecorationSource } from '../../../browser/viewParts/decorations/decorations.js';
import { ColorService, type ColorData, type DefaultColorDecoratorsEnablement } from '../common/languageColors.js';
import { TrackedRangeStickiness } from '../../../common/model.js';

export interface ColorDetectorOptions {
	readonly enabled: boolean;
	readonly limit: number;
	readonly defaultColorDecorators: DefaultColorDecoratorsEnablement;
}

/** Owns provider refresh, version cancellation, tracked ranges, and color-swatch metadata. */
export class EditorColorDetector extends Disposable {
	private readonly decorations: TextDecorationCollection<ColorData>;
	private readonly refreshTimer = this._register(new MutableDisposable<IDisposable>());
	private request: AbortController | undefined;
	private detectedCount = 0;
	readonly decorationSource: DecorationSource;

	constructor(
		private readonly model: TextModel,
		private readonly service: ColorService,
		private readonly languageId: string,
		private readonly targetWindow: Window,
		private readonly options: ColorDetectorOptions,
		private readonly onError: (error: unknown) => void,
	) {
		super();
		if (!Number.isSafeInteger(options.limit) || options.limit < 0) throw new RangeError('Color decorator limit must be a non-negative integer');
		this.decorations = this._register(new TextDecorationCollection<ColorData>(model));
		this.decorationSource = createStanzaDecorationSource(this.decorations, decoration => ({
			presentation: DecorationPresentation.ColorSwatch,
			color: colorToHex8(decoration.metadata.information.color),
			overviewRuler: false,
			minimap: false,
		}), decoration => model.getTextInRange(decoration.range));
		this._register(model.onDidChangeContent(() => this.scheduleRefresh(250)));
		this._register(service.onDidChange(() => this.scheduleRefresh(0)));
		this.scheduleRefresh(0);
	}

	get totalColorCount(): number {
		return this.detectedCount;
	}

	get isLimited(): boolean {
		return this.detectedCount > this.options.limit;
	}

	findAtPosition(position: Position): ColorData | undefined {
		const decoration = this.decorations.decorations.find(candidate => candidate.range.containsPosition(position));
		return decoration ? colorDataAtCurrentRange(decoration) : undefined;
	}

	findByDecorationId(id: string | undefined): ColorData | undefined {
		if (!id || !/^\d+$/u.test(id)) return undefined;
		const decoration = this.decorations.get(Number(id) as TextDecorationId);
		return decoration ? colorDataAtCurrentRange(decoration) : undefined;
	}

	refresh(): void {
		this.assertNotDisposed();
		this.refreshTimer.clear();
		this.request?.abort();
		if (!this.options.enabled || this.model.largeFile.tooLargeForTokenization) {
			this.detectedCount = 0;
			this.decorations.clear();
			return;
		}
		const request = this.request = new AbortController();
		void this.service.provideDocumentColors(this.languageId, this.options.defaultColorDecorators, request.signal).then(colors => {
			if (request.signal.aborted || this.isDisposed) return;
			this.detectedCount = colors.length;
			this.decorations.replaceAll(colors.slice(0, this.options.limit).map(data => ({
				range: data.information.range,
				stickiness: TrackedRangeStickiness.NeverGrowsWhenTypingAtEdges,
				metadata: data,
			})));
		}, error => {
			if (!request.signal.aborted) this.onError(error);
		});
	}

	private scheduleRefresh(delay: number): void {
		this.refreshTimer.value = disposableWindowTimeout(this.targetWindow, () => {
			this.refreshTimer.clear();
			this.refresh();
		}, delay);
	}

	protected override disposeCore(): void {
		this.request?.abort();
		this.request = undefined;
		super.disposeCore();
	}
}

function colorToHex8(color: { readonly red: number; readonly green: number; readonly blue: number; readonly alpha: number }): string {
	return `#${[color.red, color.green, color.blue, color.alpha].map(channel => Math.round(channel * 255).toString(16).padStart(2, '0')).join('')}`;
}

function colorDataAtCurrentRange(decoration: TextDecorationSnapshot<ColorData>): ColorData {
	const data = decoration.metadata;
	if (data.information.range.equalsRange(decoration.range)) return data;
	return Object.freeze({
		provider: data.provider,
		information: Object.freeze({ color: data.information.color, range: decoration.range }),
	});
}
