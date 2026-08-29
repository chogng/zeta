import './media/colorPicker.css';
import { addDisposableListener, stopEvent } from '../../../../base/browser/dom.js';
import { disposableWindowTimeout } from '../../../../base/browser/scheduler.js';
import { Disposable, MutableDisposable, type IDisposable } from '../../../../base/common/lifecycle.js';
import { localize } from '../../../../nls.js';
import { createEditorEditCommand } from '../../../common/commands/editorCommand.js';
import { type EditorSelectionController } from '../../../common/cursor/editorSelectionController.js';
import { type TextPosition } from '../../../common/core/text.js';
import { RGBA8 } from '../../../common/core/misc/rgba.js';
import { type EditorViewport } from '../../../browser/view.js';
import { type EditorCapability, registerEditorContribution } from '../../../browser/editorExtensions.js';
import { ColorService, type ColorData } from '../common/color.js';
import { ColorDetector } from './colorDetector.js';
import { ColorPickerModel } from './colorPickerModel.js';
import { ColorPickerWidget } from './colorPickerWidget.js';

interface ColorPickerCapabilityValue {
	readonly service: ColorService;
	readonly detector: ColorDetector;
}

const ColorPickerCapability: EditorCapability<ColorPickerCapabilityValue> = Object.freeze({ id: 'editor.colorPicker' });

export type ColorDecoratorsActivatedOn = 'clickAndHover' | 'click' | 'hover';

/** Coordinates color detection, picker requests, focus, and one atomic editor edit. */
export class ColorPickerController extends Disposable {
	private readonly widget: ColorPickerWidget;
	private readonly model = this._register(new MutableDisposable<ColorPickerModel>());
	private readonly hoverTimer = this._register(new MutableDisposable<IDisposable>());
	private presentationRequest: AbortController | undefined;
	private documentColorRequest: AbortController | undefined;
	private activeData: ColorData | undefined;
	private originalText = '';

	constructor(
		private readonly editorInput: HTMLElement,
		private readonly viewport: EditorViewport,
		private readonly selections: EditorSelectionController,
		private readonly service: ColorService,
		private readonly detector: ColorDetector,
		private readonly languageId: string,
		private readonly activatedOn: ColorDecoratorsActivatedOn,
		private readonly readOnly: boolean,
		private readonly onError: (error: unknown) => void,
	) {
		super();
		if (viewport.textModel !== selections.textModel) throw new TypeError('Stanza color picker dependencies must share a text model');
		this.widget = this._register(new ColorPickerWidget(
			viewport.element,
			color => this.refreshPresentations(color),
			() => this.apply(),
			() => this.close(true),
		));
		viewport.element.append(this.widget.domNode);
		this._register(addDisposableListener(editorInput, 'keydown', event => this.handleEditorKeyDown(event), true));
		this._register(addDisposableListener(viewport.element, 'pointerdown', event => this.handlePointerDown(event), true));
		this._register(addDisposableListener(viewport.element, 'pointerover', event => this.handlePointerOver(event)));
		this._register(addDisposableListener(viewport.element, 'pointerout', event => this.handlePointerOut(event)));
		this._register(addDisposableListener(viewport.element.ownerDocument, 'pointerdown', event => {
			const target = event.target;
			const targetNode = target && typeof (target as Node).nodeType === 'number' ? target as Node : undefined;
			if (!this.widget.visible) return;
			if (targetNode && this.widget.domNode.contains(targetNode)) {
				event.stopPropagation();
				return;
			}
			if (colorSwatch(target)) return;
			this.close(false);
		}, true));
		this._register(viewport.textModel.onDidChange(() => this.close(false)));
		this._register(viewport.onDidChangeLayout(() => this.close(false)));
	}

	async showAtPosition(position: TextPosition, focus = true): Promise<void> {
		this.documentColorRequest?.abort();
		const request = this.documentColorRequest = new AbortController();
		try {
			let data = this.detector.findAtPosition(position);
			if (!data) {
				const colors = await this.service.provideDocumentColors(this.languageId, 'auto', request.signal);
				if (request.signal.aborted) return;
				data = colors.find(candidate => candidate.information.range.containsPosition(position));
			}
			if (!data) {
				this.viewport.announceAccessibilityStatus(localize('zeta.editor.colorPicker', 'noColorAtCursor', 'No color is available at the cursor.'));
				return;
			}
			await this.show(data, focus);
		} catch (error) {
			if (!request.signal.aborted) this.onError(error);
		}
	}

	hide(): void {
		this.close(true);
	}

	private handleEditorKeyDown(event: KeyboardEvent): void {
		if (event.defaultPrevented || event.isComposing || event.getModifierState('AltGraph')) return;
		if (event.key === 'Escape' && this.widget.visible) {
			stopEvent(event);
			this.close(true);
			return;
		}
		if (!event.shiftKey || (!event.ctrlKey && !event.metaKey) || event.altKey || event.key.toLowerCase() !== 'c') return;
		stopEvent(event);
		void this.showAtPosition(this.selections.selections.primary.active);
	}

	private handlePointerDown(event: PointerEvent): void {
		if (this.activatedOn === 'hover') return;
		const swatch = colorSwatch(event.target);
		if (!swatch) return;
		const data = this.detector.findByDecorationId(swatch.dataset.decorationId);
		if (!data) return;
		stopEvent(event);
		void this.show(data, true);
	}

	private handlePointerOver(event: PointerEvent): void {
		if (this.activatedOn === 'click') return;
		const swatch = colorSwatch(event.target);
		if (!swatch) return;
		const data = this.detector.findByDecorationId(swatch.dataset.decorationId);
		if (!data) return;
		this.hoverTimer.value = disposableWindowTimeout(this.widget.domNode.ownerDocument.defaultView!, () => {
			this.hoverTimer.clear();
			void this.show(data, false);
		}, 300);
	}

	private handlePointerOut(event: PointerEvent): void {
		const swatch = colorSwatch(event.target);
		const related = event.relatedTarget;
		if (!swatch || related && typeof (related as Node).nodeType === 'number' && swatch.contains(related as Node)) return;
		this.hoverTimer.clear();
	}

	private async show(data: ColorData, focus: boolean): Promise<void> {
		this.presentationRequest?.abort();
		this.activeData = data;
		this.originalText = this.viewport.textModel.getTextInRange(data.information.range);
		const model = new ColorPickerModel(data.information.color);
		this.widget.hide();
		this.model.value = model;
		this.widget.show(model, this.widgetPosition(data.information.range.start), focus);
		await this.loadPresentations(model, data.information.color);
	}

	private refreshPresentations(color: RGBA8): void {
		const model = this.model.value;
		if (!model) return;
		void this.loadPresentations(model, color);
	}

	private async loadPresentations(model: ColorPickerModel, color: RGBA8): Promise<void> {
		const data = this.activeData;
		if (!data) return;
		this.presentationRequest?.abort();
		const request = this.presentationRequest = new AbortController();
		try {
			const presentations = await this.service.provideColorPresentations(this.languageId, data, color, request.signal);
			if (request.signal.aborted || this.model.value !== model) return;
			model.setColorPresentations(presentations, this.originalText);
		} catch (error) {
			if (!request.signal.aborted) this.onError(error);
		}
	}

	private apply(): void {
		const data = this.activeData;
		const presentation = this.model.value?.selectedPresentation;
		if (!data || !presentation) return;
		if (this.readOnly) {
			this.viewport.announceAccessibilityStatus(localize('zeta.editor.colorPicker', 'readOnly', 'The editor is read-only.'));
			this.close(true);
			return;
		}
		const edits = [
			presentation.textEdit ?? { range: data.information.range, text: presentation.label },
			...(presentation.additionalTextEdits ?? []),
		].sort((left, right) => left.range.start.compareTo(right.range.start) || left.range.end.compareTo(right.range.end));
		try {
			const command = createEditorEditCommand(this.viewport.textModel, this.selections.selections, edits);
			if (command) this.selections.execute(command);
			this.close(true);
		} catch (error) {
			this.onError(error);
		}
	}

	private widgetPosition(position: TextPosition): { readonly left: number; readonly top: number } {
		const coordinates = this.viewport.getPositionContentCoordinates(position);
		const scroll = this.viewport.viewportLayout.scrollPosition;
		const layout = this.viewport.currentLayout.viewportSize;
		const left = Math.min(Math.max(4, coordinates.left - scroll.left), Math.max(4, layout.width - 324));
		const below = coordinates.top - scroll.top + coordinates.height + 4;
		const top = below + 238 <= layout.height ? below : Math.max(4, coordinates.top - scroll.top - 238);
		return Object.freeze({ left, top });
	}

	private close(restoreFocus: boolean): void {
		this.documentColorRequest?.abort();
		this.documentColorRequest = undefined;
		this.presentationRequest?.abort();
		this.presentationRequest = undefined;
		this.hoverTimer.clear();
		this.widget.hide();
		this.model.clear();
		this.activeData = undefined;
		this.originalText = '';
		if (restoreFocus) this.editorInput.focus({ preventScroll: true });
	}

	protected override disposeCore(): void {
		this.close(false);
		super.disposeCore();
	}
}

function colorSwatch(target: EventTarget | null): HTMLElement | undefined {
	return target && typeof (target as Element).closest === 'function'
		? (target as Element).closest<HTMLElement>('.stanza-editor-decoration.color-swatch') ?? undefined
		: undefined;
}

registerEditorContribution({
	id: 'editor.contrib.colorPicker',
	configure: context => {
		const service = new ColorService(context.model, context.languageFeaturesService.colorProvider, context.options.input.resource, context.onLanguageError);
		const targetWindow = context.options.container.ownerDocument.defaultView;
		if (!targetWindow) throw new Error('Color picker requires an attached browser window');
		const detector = context.register(new ColorDetector(
			context.model,
			service,
			context.languageId,
			targetWindow,
			{
				enabled: context.options.colorDecorators !== false,
				limit: context.options.colorDecoratorsLimit ?? 500,
				defaultColorDecorators: context.options.defaultColorDecorators ?? 'auto',
			},
			context.onLanguageError,
		));
		context.addDecorationSource(detector.decorationSource);
		context.provideCapability(ColorPickerCapability, Object.freeze({ service, detector }));
	},
	install: context => {
		if (context.kind !== 'text') return;
		const capability = context.getCapability(ColorPickerCapability);
		context.register(new ColorPickerController(
			context.view.element,
			context.viewport,
			context.selections,
			capability.service,
			capability.detector,
			context.languageId,
			context.options.colorDecoratorsActivatedOn ?? 'clickAndHover',
			context.options.input.readOnly === true,
			context.onLanguageError,
		));
	},
});
