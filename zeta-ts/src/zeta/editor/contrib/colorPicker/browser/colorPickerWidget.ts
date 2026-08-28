import { addDisposableListener, h, stopEvent } from '../../../../base/browser/dom.js';
import { Disposable, DisposableStore } from '../../../../base/common/lifecycle.js';
import { localize } from '../../../../nls.js';
import { RGBA8 } from '../../../common/core/misc/rgba.js';
import { type ColorPickerModel } from './colorPickerModel.js';

export interface ColorPickerWidgetPosition {
	readonly left: number;
	readonly top: number;
}

/** Owns the retained color controls and projects one replaceable ColorPickerModel. */
export class ColorPickerWidget extends Disposable {
	readonly domNode: HTMLDivElement;
	private readonly preview: HTMLDivElement;
	private readonly presentationSelect: HTMLSelectElement;
	private readonly hueInput: HTMLInputElement;
	private readonly saturationInput: HTMLInputElement;
	private readonly lightnessInput: HTMLInputElement;
	private readonly alphaInput: HTMLInputElement;
	private readonly applyButton: HTMLButtonElement;
	private readonly modelListeners = this._register(new DisposableStore());
	private model: ColorPickerModel | undefined;
	private updating = false;

	constructor(ownerDocument: Document, private readonly onColorChange: (color: RGBA8) => void, private readonly onApply: () => void, private readonly onCancel: () => void) {
		super();
		this.preview = h(ownerDocument, 'div', {
			className: 'stanza-editor-color-picker-preview',
			attributes: { 'aria-hidden': 'true' },
		});
		this.presentationSelect = h(ownerDocument, 'select', {
			className: 'stanza-editor-color-picker-presentation',
			attributes: { 'aria-label': localize('zeta.editor.colorPicker', 'presentation', 'Color format') },
		});
		this.hueInput = colorRange(ownerDocument, 'hue', localize('zeta.editor.colorPicker', 'hue', 'Hue'), 0, 360);
		this.saturationInput = colorRange(ownerDocument, 'saturation', localize('zeta.editor.colorPicker', 'saturation', 'Saturation'), 0, 100);
		this.lightnessInput = colorRange(ownerDocument, 'lightness', localize('zeta.editor.colorPicker', 'lightness', 'Lightness'), 0, 100);
		this.alphaInput = colorRange(ownerDocument, 'alpha', localize('zeta.editor.colorPicker', 'opacity', 'Opacity'), 0, 255);
		const closeButton = h(ownerDocument, 'button', {
			className: 'stanza-editor-color-picker-close',
			attributes: { type: 'button', 'aria-label': localize('zeta.editor.colorPicker', 'close', 'Close color picker') },
		}, '×');
		this.applyButton = h(ownerDocument, 'button', {
			className: 'stanza-editor-color-picker-apply',
			attributes: { type: 'button' },
		}, localize('zeta.editor.colorPicker', 'apply', 'Apply'));
		const cancelButton = h(ownerDocument, 'button', {
			className: 'stanza-editor-color-picker-cancel',
			attributes: { type: 'button' },
		}, localize('zeta.editor.colorPicker', 'cancel', 'Cancel'));
		this.domNode = h(ownerDocument, 'div', {
			className: 'stanza-editor-color-picker',
			attributes: {
				role: 'dialog',
				'aria-label': localize('zeta.editor.colorPicker', 'dialog', 'Color picker'),
				'aria-modal': 'false',
			},
			properties: { hidden: true },
		},
			h(ownerDocument, 'div', { className: 'stanza-editor-color-picker-header' }, this.preview, this.presentationSelect, closeButton),
			h(ownerDocument, 'div', { className: 'stanza-editor-color-picker-controls' },
				colorControl(ownerDocument, localize('zeta.editor.colorPicker', 'hueShort', 'H'), this.hueInput),
				colorControl(ownerDocument, localize('zeta.editor.colorPicker', 'saturationShort', 'S'), this.saturationInput),
				colorControl(ownerDocument, localize('zeta.editor.colorPicker', 'lightnessShort', 'L'), this.lightnessInput),
				colorControl(ownerDocument, localize('zeta.editor.colorPicker', 'opacityShort', 'A'), this.alphaInput),
			),
			h(ownerDocument, 'div', { className: 'stanza-editor-color-picker-actions' }, cancelButton, this.applyButton),
		);
		for (const input of [this.hueInput, this.saturationInput, this.lightnessInput, this.alphaInput]) {
			this._register(addDisposableListener(input, 'input', () => this.handleColorInput()));
		}
		this._register(addDisposableListener(this.presentationSelect, 'change', () => this.model?.selectPresentation(this.presentationSelect.selectedIndex)));
		this._register(addDisposableListener(closeButton, 'click', () => this.onCancel()));
		this._register(addDisposableListener(cancelButton, 'click', () => this.onCancel()));
		this._register(addDisposableListener(this.applyButton, 'click', () => this.onApply()));
		this._register(addDisposableListener(this.domNode, 'keydown', event => {
			if (event.key === 'Escape') {
				stopEvent(event);
				this.onCancel();
			}
		}));
	}

	get visible(): boolean {
		return !this.domNode.hidden;
	}

	show(model: ColorPickerModel, position: ColorPickerWidgetPosition, focus: boolean): void {
		this.modelListeners.clear();
		this.model = model;
		this.modelListeners.add(model.onDidChangeColor(color => this.renderColor(color)));
		this.modelListeners.add(model.onDidChangePresentations(() => this.renderPresentations()));
		this.modelListeners.add(model.onDidChangeSelectedPresentation(() => this.renderSelectedPresentation()));
		this.renderColor(model.color);
		this.renderPresentations();
		this.domNode.style.left = `${position.left}px`;
		this.domNode.style.top = `${position.top}px`;
		this.domNode.hidden = false;
		if (focus) this.hueInput.focus({ preventScroll: true });
	}

	hide(): void {
		this.modelListeners.clear();
		this.model = undefined;
		this.domNode.hidden = true;
	}

	private handleColorInput(): void {
		if (this.updating || !this.model) return;
		const color = hslToRgb(
			Number(this.hueInput.value),
			Number(this.saturationInput.value) / 100,
			Number(this.lightnessInput.value) / 100,
			Number(this.alphaInput.value),
		);
		this.model.setColor(color);
		this.onColorChange(color);
	}

	private renderColor(color: RGBA8): void {
		const [hue, saturation, lightness] = rgbToHsl(color);
		this.updating = true;
		this.hueInput.value = String(hue);
		this.saturationInput.value = String(saturation);
		this.lightnessInput.value = String(lightness);
		this.alphaInput.value = String(color.a);
		this.updating = false;
		const value = colorToHex8(color);
		this.preview.style.setProperty('--stanza-editor-color-picker-value', value);
		this.saturationInput.style.setProperty('--stanza-editor-color-picker-hue', colorToHex8(hslToRgb(hue, 1, 0.5, 255)));
		this.alphaInput.style.setProperty('--stanza-editor-color-picker-opaque', colorToHex8(new RGBA8(color.r, color.g, color.b, 255)));
	}

	private renderPresentations(): void {
		const presentations = this.model?.colorPresentations ?? [];
		this.presentationSelect.replaceChildren(...presentations.map(presentation => h(this.domNode.ownerDocument, 'option', {}, presentation.label)));
		this.applyButton.disabled = presentations.length === 0;
		this.renderSelectedPresentation();
	}

	private renderSelectedPresentation(): void {
		const selected = this.model?.selectedPresentation;
		if (!selected) return;
		const index = this.model!.colorPresentations.indexOf(selected);
		if (index >= 0) this.presentationSelect.selectedIndex = index;
	}

	protected override disposeCore(): void {
		this.domNode.remove();
		super.disposeCore();
	}
}

function colorRange(ownerDocument: Document, className: string, label: string, minimum: number, maximum: number): HTMLInputElement {
	return h(ownerDocument, 'input', {
		className: `stanza-editor-color-picker-${className}`,
		attributes: { type: 'range', min: minimum, max: maximum, step: 1, 'aria-label': label },
	});
}

function colorControl(ownerDocument: Document, label: string, input: HTMLInputElement): HTMLLabelElement {
	return h(ownerDocument, 'label', { className: 'stanza-editor-color-picker-control' }, h(ownerDocument, 'span', {}, label), input);
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

function hslToRgb(hue: number, saturation: number, lightness: number, alpha: number): RGBA8 {
	const normalizedHue = (hue % 360 + 360) % 360;
	const chroma = (1 - Math.abs(2 * lightness - 1)) * saturation;
	const section = normalizedHue / 60;
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

function colorToHex8(color: RGBA8): string {
	return `#${[color.r, color.g, color.b, color.a].map(channel => channel.toString(16).padStart(2, '0')).join('')}`;
}
