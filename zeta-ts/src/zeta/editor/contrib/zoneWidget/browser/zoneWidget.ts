import './media/zoneWidget.css';
import { h } from '../../../../base/browser/dom.js';
import { Sash, SashState, type SashDragEvent } from '../../../../base/browser/ui/sash/sash.js';
import { Disposable, MutableDisposable } from '../../../../base/common/lifecycle.js';
import { type CodeEditorWidget } from '../../../browser/widget/codeEditor/codeEditorWidget.js';
import { type EditorViewZone, type EditorViewZoneHandle } from '../../../browser/view.js';
import { type Position } from '../../../common/core/position.js';
import { Range } from '../../../common/core/range.js';
import { TrackedRangeStickiness, type TrackedRange } from '../../../common/model/trackedRange.js';

export type ZoneWidgetEditor = Pick<CodeEditorWidget, 'viewport' | 'revealRange'>;

export interface ZoneWidgetOptions {
	readonly showFrame?: boolean;
	readonly showArrow?: boolean;
	readonly frameWidth?: number;
	readonly className?: string;
	readonly isAccessible?: boolean;
	readonly isResizable?: boolean;
	readonly frameColor?: string;
	readonly arrowColor?: string;
	readonly keepEditorSelection?: boolean;
	readonly ordinal?: number;
}

export interface ZoneWidgetStyles {
	readonly frameColor?: string | null;
	readonly arrowColor?: string | null;
}

interface ResolvedZoneWidgetOptions {
	readonly showFrame: boolean;
	readonly showArrow: boolean;
	readonly frameWidth: number | undefined;
	readonly className: string;
	readonly isAccessible: boolean;
	readonly isResizable: boolean;
	readonly keepEditorSelection: boolean;
	readonly ordinal: number | undefined;
}

/** Anchors an interactive widget in reserved editor space after a text position. */
export abstract class ZoneWidget extends Disposable {
	public readonly domNode: HTMLDivElement;
	protected containerDomNode: HTMLDivElement | undefined;
	protected readonly editor: ZoneWidgetEditor;

	private readonly options: ResolvedZoneWidgetOptions;
	private readonly anchor: MutableDisposable<TrackedRange>;
	private readonly viewZoneHandle: MutableDisposable<EditorViewZoneHandle>;
	private viewZone: EditorViewZone | undefined;
	private arrowDomNode: HTMLDivElement | undefined;
	private resizeSash: Sash | undefined;
	private frameColor: string | undefined;
	private arrowColor: string | undefined;
	private heightInLines = 0;
	private resizeStartHeightInLines: number | undefined;
	private created = false;
	private layingOut = false;

	constructor(editor: ZoneWidgetEditor, options: ZoneWidgetOptions = {}) {
		super();
		this.editor = editor;
		this.options = readOptions(options);
		this.frameColor = options.frameColor;
		this.arrowColor = options.arrowColor;
		this.anchor = this._register(new MutableDisposable<TrackedRange>());
		this.viewZoneHandle = this._register(new MutableDisposable<EditorViewZoneHandle>());
		this.domNode = h(editor.viewport.element.ownerDocument, 'div');
		this.domNode.className = 'stanza-editor-zone-widget';
		this.domNode.hidden = true;
		if (!this.options.isAccessible) {
			this.domNode.setAttribute('aria-hidden', 'true');
			this.domNode.setAttribute('role', 'presentation');
		}
		this._register(editor.viewport.onDidChangeLayout(() => this.layoutZone()));
	}

	public get position(): Position | undefined {
		return this.anchor.value?.range.getStartPosition();
	}

	protected get isShowing(): boolean {
		return this.viewZone !== undefined;
	}

	public create(): void {
		this.assertNotDisposed();
		if (this.created) throw new Error('Zone widget has already been created');
		this.created = true;
		for (const className of this.options.className.split(/\s+/u).filter(Boolean)) this.domNode.classList.add(className);
		this.domNode.classList.toggle('show-frame', this.options.showFrame);
		this.domNode.classList.toggle('show-arrow', this.options.showArrow);

		if (this.options.showArrow) {
			this.arrowDomNode = h(this.domNode.ownerDocument, 'div');
			this.arrowDomNode.className = 'stanza-editor-zone-widget-arrow';
			this.arrowDomNode.setAttribute('aria-hidden', 'true');
			this.domNode.append(this.arrowDomNode);
		}

		this.containerDomNode = h(this.domNode.ownerDocument, 'div');
		this.containerDomNode.className = 'stanza-editor-zone-widget-container';
		this.domNode.append(this.containerDomNode);
		this.fillContainer(this.containerDomNode);
		this.createResizeSash();
		this.applyStyles();
	}

	public style(styles: ZoneWidgetStyles): void {
		this.assertNotDisposed();
		if (styles.frameColor !== undefined) this.frameColor = styles.frameColor ?? undefined;
		if (styles.arrowColor !== undefined) this.arrowColor = styles.arrowColor ?? undefined;
		this.applyStyles();
	}

	public show(rangeOrPosition: Range | Position, heightInLines: number): void {
		this.assertNotDisposed();
		this.assertCreated();
		const range = normalizeRange(rangeOrPosition);
		validateHeightInLines(heightInLines);
		this.editor.viewport.textModel.offsetAt(range.getStartPosition());
		this.editor.viewport.textModel.offsetAt(range.getEndPosition());
		this.hide();
		this.heightInLines = this.limitHeightInLines(heightInLines);
		this.anchor.value = this.editor.viewport.textModel.trackRange(range, TrackedRangeStickiness.NeverGrowsAtEdges);
		const viewZone: EditorViewZone = {
			afterLineIndex: this.anchorVisualLineIndex,
			heightInPixels: this.heightInPixels,
			ordinal: this.options.ordinal,
			domNode: this.domNode,
		};
		this.viewZone = viewZone;
		this.domNode.hidden = false;
		try {
			this.viewZoneHandle.value = this.editor.viewport.addViewZone(viewZone);
		} catch (error) {
			this.viewZone = undefined;
			this.anchor.clear();
			this.domNode.hidden = true;
			throw error;
		}
		this.layoutZone();
		if (this.options.keepEditorSelection) this.editor.viewport.revealPosition(range.getStartPosition());
		else this.editor.revealRange(range);
	}

	public updatePositionAndHeight(rangeOrPosition: Range | Position, heightInLines = this.heightInLines): void {
		this.assertNotDisposed();
		if (!this.viewZone) return;
		const range = normalizeRange(rangeOrPosition);
		validateHeightInLines(heightInLines);
		this.editor.viewport.textModel.offsetAt(range.getStartPosition());
		this.editor.viewport.textModel.offsetAt(range.getEndPosition());
		this.anchor.value = this.editor.viewport.textModel.trackRange(range, TrackedRangeStickiness.NeverGrowsAtEdges);
		this.heightInLines = this.limitHeightInLines(heightInLines);
		this.layoutZone();
	}

	public hide(): void {
		this.assertNotDisposed();
		this.viewZone = undefined;
		this.viewZoneHandle.clear();
		this.anchor.clear();
		this.domNode.hidden = true;
		this.heightInLines = 0;
		this.resizeStartHeightInLines = undefined;
		this.updateSashState();
	}

	public hasFocus(): boolean {
		return this.domNode.contains(this.domNode.ownerDocument.activeElement);
	}

	protected setCssClass(className: string, classToReplace?: string): void {
		if (!this.containerDomNode) return;
		if (classToReplace) this.containerDomNode.classList.remove(classToReplace);
		this.containerDomNode.classList.add(className);
	}

	protected maximumHeightInLines(): number {
		const layout = this.editor.viewport.viewportLayout;
		return Math.max(12, layout.viewportSize.height / layout.lineHeight * 0.8);
	}

	protected resizeBounds(): { readonly minLines: number; readonly maxLines: number } {
		return { minLines: 5, maxLines: 35 };
	}

	protected relayout(heightInLines: number, useMaximum = false): void {
		validateHeightInLines(heightInLines);
		if (!this.viewZone) return;
		this.heightInLines = useMaximum ? this.limitHeightInLines(heightInLines) : heightInLines;
		this.layoutZone();
	}

	protected onWidth(_widthInPixels: number): void {}

	protected abstract fillContainer(container: HTMLElement): void;
	protected abstract layoutContent(heightInPixels: number, widthInPixels: number): void;

	private get anchorVisualLineIndex(): number {
		const position = this.anchor.value?.range.getStartPosition();
		if (!position) throw new Error('Zone widget has no anchor');
		return this.editor.viewport.getVisualLineProjection().visualLineIndexAt(position);
	}

	private get heightInPixels(): number {
		return this.heightInLines * this.editor.viewport.viewportLayout.lineHeight;
	}

	private limitHeightInLines(heightInLines: number): number {
		return Math.min(heightInLines, this.maximumHeightInLines());
	}

	private assertCreated(): void {
		if (!this.created) throw new Error('Zone widget must be created before it is shown');
	}

	private layoutZone(): void {
		if (!this.viewZone || this.layingOut) return;
		this.layingOut = true;
		try {
			const nextAfterLineIndex = this.anchorVisualLineIndex;
			const nextHeightInPixels = this.heightInPixels;
			const needsZoneLayout = this.viewZone.afterLineIndex !== nextAfterLineIndex || this.viewZone.heightInPixels !== nextHeightInPixels;
			this.viewZone.afterLineIndex = nextAfterLineIndex;
			this.viewZone.heightInPixels = nextHeightInPixels;
			if (needsZoneLayout) this.viewZoneHandle.value?.layout();
			this.layoutDom(nextHeightInPixels);
			this.updateSashState();
		} finally {
			this.layingOut = false;
		}
	}

	private layoutDom(heightInPixels: number): void {
		const layout = this.editor.viewport.viewportLayout;
		const widthInPixels = layout.viewportSize.width;
		const arrowHeight = this.options.showArrow ? Math.round(layout.lineHeight / 3) : 0;
		const frameWidth = this.options.showFrame ? this.options.frameWidth ?? Math.round(layout.lineHeight / 9) : 0;
		const containerHeight = Math.max(0, heightInPixels - 2 * arrowHeight - 2 * frameWidth);
		this.domNode.style.left = `${layout.scrollPosition.left}px`;
		this.domNode.style.width = `${widthInPixels}px`;
		this.arrowDomNode?.style.setProperty('--stanza-zone-widget-arrow-size', `${arrowHeight}px`);
		if (this.arrowDomNode) {
			const coordinates = this.editor.viewport.getPositionContentCoordinates(this.anchor.value!.range.getStartPosition());
			const relativeLeft = Math.min(Math.max(arrowHeight, coordinates.left - layout.scrollPosition.left), Math.max(arrowHeight, widthInPixels - arrowHeight));
			this.arrowDomNode.style.left = `${relativeLeft}px`;
		}
		if (this.containerDomNode) {
			this.containerDomNode.style.top = `${arrowHeight}px`;
			this.containerDomNode.style.height = `${containerHeight}px`;
			this.containerDomNode.style.borderTopWidth = `${frameWidth}px`;
			this.containerDomNode.style.borderBottomWidth = `${frameWidth}px`;
		}
		if (this.resizeSash) {
			this.resizeSash.element.style.top = `${Math.max(0, heightInPixels - frameWidth)}px`;
			this.resizeSash.element.style.right = '0';
			this.resizeSash.element.style.left = '0';
		}
		this.onWidth(widthInPixels);
		this.layoutContent(containerHeight, widthInPixels);
	}

	private applyStyles(): void {
		setOptionalCustomProperty(this.domNode, '--stanza-zone-widget-frame-color', this.frameColor);
		setOptionalCustomProperty(this.domNode, '--stanza-zone-widget-arrow-color', this.arrowColor);
	}

	private createResizeSash(): void {
		const sash = this._register(new Sash(this.domNode, 'horizontal'));
		this.resizeSash = sash;
		this._register(sash.onDidStart(() => {
			this.resizeStartHeightInLines = this.heightInLines;
		}));
		this._register(sash.onDidChange(event => this.handleSashChange(event)));
		this._register(sash.onDidEnd(() => {
			this.resizeStartHeightInLines = undefined;
		}));
		this.updateSashState();
	}

	private handleSashChange(event: SashDragEvent): void {
		if (!this.options.isResizable || this.resizeStartHeightInLines === undefined || !this.viewZone) return;
		const lineDelta = Math.trunc(event.delta / this.editor.viewport.viewportLayout.lineHeight);
		const { minLines, maxLines } = this.resizeBounds();
		const heightInLines = Math.min(maxLines, Math.max(minLines, this.resizeStartHeightInLines + lineDelta));
		if (heightInLines !== this.heightInLines) this.relayout(heightInLines);
	}

	private updateSashState(): void {
		if (!this.resizeSash) return;
		const isEnabled = this.options.isResizable && this.viewZone !== undefined;
		this.resizeSash.element.hidden = !isEnabled;
		if (!isEnabled) {
			this.resizeSash.state = SashState.Disabled;
			return;
		}
		const { minLines, maxLines } = this.resizeBounds();
		if (minLines >= maxLines) this.resizeSash.state = SashState.Disabled;
		else if (this.heightInLines <= minLines) this.resizeSash.state = SashState.AtMinimum;
		else if (this.heightInLines >= maxLines) this.resizeSash.state = SashState.AtMaximum;
		else this.resizeSash.state = SashState.Enabled;
	}
}

function readOptions(options: ZoneWidgetOptions): ResolvedZoneWidgetOptions {
	if (!options || typeof options !== 'object') throw new TypeError('Zone widget options must be an object');
	if (options.frameWidth !== undefined && (!Number.isFinite(options.frameWidth) || options.frameWidth < 0)) throw new RangeError('Zone widget frame width must be finite and non-negative');
	if (options.ordinal !== undefined && !Number.isFinite(options.ordinal)) throw new RangeError('Zone widget ordinal must be finite');
	if (options.className !== undefined && typeof options.className !== 'string') throw new TypeError('Zone widget class name must be a string');
	if (options.frameColor !== undefined && typeof options.frameColor !== 'string') throw new TypeError('Zone widget frame color must be a string');
	if (options.arrowColor !== undefined && typeof options.arrowColor !== 'string') throw new TypeError('Zone widget arrow color must be a string');
	return Object.freeze({
		showFrame: options.showFrame ?? true,
		showArrow: options.showArrow ?? true,
		frameWidth: options.frameWidth,
		className: options.className ?? '',
		isAccessible: options.isAccessible ?? false,
		isResizable: options.isResizable ?? false,
		keepEditorSelection: options.keepEditorSelection ?? false,
		ordinal: options.ordinal,
	});
}

function normalizeRange(rangeOrPosition: Range | Position): Range {
	return Range.isIRange(rangeOrPosition) ? Range.fromPositions(rangeOrPosition.getStartPosition(), rangeOrPosition.getEndPosition()) : Range.fromPositions(rangeOrPosition);
}

function validateHeightInLines(heightInLines: number): void {
	if (!Number.isFinite(heightInLines) || heightInLines <= 0) throw new RangeError('Zone widget height must be finite and positive');
}

function setOptionalCustomProperty(element: HTMLElement, name: string, value: string | undefined): void {
	if (value === undefined) element.style.removeProperty(name);
	else element.style.setProperty(name, value);
}
