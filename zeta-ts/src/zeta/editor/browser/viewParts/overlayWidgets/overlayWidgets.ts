import './overlayWidgets.css';
import { createFastDomNode, type FastDomNode } from '../../../../base/browser/fastDomNode.js';
import { getDomNodePagePosition, type IDomNodePagePosition } from '../../../../base/browser/dom.js';
import { Disposable, DisposableMap, toDisposable } from '../../../../base/common/lifecycle.js';
import { type IOverlayWidget, type IOverlayWidgetPosition, type IOverlayWidgetPositionCoordinates, OverlayWidgetPositionPreference } from '../../editorBrowser.js';
import { PartFingerprint, PartFingerprints, type EditorRenderingContext, ViewPart } from '../../view/viewPart.js';
import { type ViewContext } from '../../../common/viewModel/viewContext.js';

interface ViewOverlayWidgetsOptions {
	readonly viewDomNode: HTMLElement;
	readonly allowOverflow: boolean;
	readonly fixedOverflowWidgets: boolean;
	readonly verticalScrollbarWidth: number;
	readonly horizontalScrollbarHeight: number;
	readonly readMinimapWidth: () => number;
	readonly setMinimumContentWidth: (width: number) => void;
	readonly requestRender: () => void;
}

interface OverlayWidgetLayout {
	readonly editorWidth: number;
	readonly editorHeight: number;
	readonly minimapWidth: number;
	readonly verticalScrollbarWidth: number;
	readonly horizontalScrollbarHeight: number;
}

/** Owns widgets positioned against the editor viewport rather than document content. */
export class ViewOverlayWidgets extends ViewPart {
	private readonly _viewDomNode: HTMLElement;
	private readonly _domNode: FastDomNode<HTMLDivElement>;
	public readonly overflowingOverlayWidgetsDomNode: FastDomNode<HTMLDivElement>;
	private readonly _widgets = this._register(new DisposableMap<string, OverlayWidget>());
	private _viewDomNodeRect: IDomNodePagePosition | null = null;
	private readonly _verticalScrollbarWidth: number;
	private _minimapWidth = 0;
	private readonly _horizontalScrollbarHeight: number;
	private _editorHeight = 0;
	private _editorWidth = 0;

	constructor(context: ViewContext, private readonly options: ViewOverlayWidgetsOptions) {
		super(context);
		this._viewDomNode = options.viewDomNode;
		this._verticalScrollbarWidth = options.verticalScrollbarWidth;
		this._horizontalScrollbarHeight = options.horizontalScrollbarHeight;
		this._domNode = createFastDomNode(options.viewDomNode.ownerDocument.createElement('div'));
		PartFingerprints.write(this._domNode, PartFingerprint.OverlayWidgets);
		this._domNode.setClassName('stanza-editor-overlay-widgets');
		this._domNode.setPosition('absolute');
		this._domNode.setTop(0);
		this._domNode.setAttribute('role', 'presentation');
		this.overflowingOverlayWidgetsDomNode = createFastDomNode(options.viewDomNode.ownerDocument.createElement('div'));
		PartFingerprints.write(this.overflowingOverlayWidgetsDomNode, PartFingerprint.OverflowingOverlayWidgets);
		this.overflowingOverlayWidgetsDomNode.setClassName('stanza-editor-overflowing-overlay-widgets');
		this.overflowingOverlayWidgetsDomNode.setAttribute('role', 'presentation');
		this._register(toDisposable(() => {
			this._domNode.domNode.remove();
			this.overflowingOverlayWidgetsDomNode.domNode.remove();
		}));
	}

	public override dispose(): void {
		super.dispose();
	}

	public getDomNode(): FastDomNode<HTMLElement> {
		return this._domNode;
	}

	private _widgetCanOverflow(widget: IOverlayWidget): boolean {
		return Boolean(widget.allowEditorOverflow) && this.options.allowOverflow;
	}

	public addWidget(widget: IOverlayWidget): void {
		const id = widget.getId();
		if (!id || this._widgets.has(id)) throw new RangeError(`Overlay widget '${id}' is already registered`);
		const overlayWidget = this._widgets.set(id, new OverlayWidget(this.options, widget));
		const container = this._widgetCanOverflow(widget) ? this.overflowingOverlayWidgetsDomNode : this._domNode;
		container.domNode.append(overlayWidget.domNode.domNode);
		this._updateMaxMinWidth();
	}

	public setWidgetPosition(widget: IOverlayWidget, position: IOverlayWidgetPosition | null): boolean {
		const candidate = findOverlayWidget(this._widgets, widget);
		if (candidate?.actual !== widget) return false;
		const changed = candidate.setPosition(position);
		this._updateMaxMinWidth();
		return changed;
	}

	public removeWidget(widget: IOverlayWidget): void {
		const id = widget.getId();
		const candidate = findOverlayWidget(this._widgets, widget);
		if (candidate?.actual !== widget) return;
		this._widgets.deleteAndDispose(id);
		this._updateMaxMinWidth();
	}

	public override prepareRender(): void {
		this._viewDomNodeRect = getDomNodePagePosition(this._viewDomNode);
	}

	public render(context: EditorRenderingContext): void {
		this._editorWidth = context.layout.viewportSize.width;
		this._editorHeight = context.layout.viewportSize.height;
		this._minimapWidth = this.options.readMinimapWidth();
		this._domNode.setWidth(this._editorWidth);
		const stacks = new Map<OverlayWidgetPositionPreference, number>();
		for (const preference of [OverlayWidgetPositionPreference.TOP_RIGHT_CORNER, OverlayWidgetPositionPreference.BOTTOM_RIGHT_CORNER, OverlayWidgetPositionPreference.TOP_CENTER]) {
			stacks.set(preference, 0);
		}
		const widgets = [...this._widgets].map(([, widget]) => widget);
		widgets.sort((left, right) => (left.position?.stackOrdinal ?? 0) - (right.position?.stackOrdinal ?? 0));
		for (const widget of widgets) {
			const preference = widget.position?.preference;
			const stackOffset = typeof preference === 'number' ? stacks.get(preference) ?? 0 : 0;
			this._renderWidget(widget, context, stackOffset);
			if (typeof preference === 'number') stacks.set(preference, stackOffset + widget.height);
		}
	}

	private _renderWidget(widget: OverlayWidget, context: EditorRenderingContext, stackOffset: number): void {
		widget.render(context, this._viewDomNodeRect, stackOffset, {
			editorWidth: this._editorWidth,
			editorHeight: this._editorHeight,
			minimapWidth: this._minimapWidth,
			verticalScrollbarWidth: this._verticalScrollbarWidth,
			horizontalScrollbarHeight: this._horizontalScrollbarHeight,
		});
	}

	private _updateMaxMinWidth(): void {
		let minimumContentWidth = 0;
		for (const [, widget] of this._widgets) minimumContentWidth = Math.max(minimumContentWidth, widget.minimumContentWidth);
		this.options.setMinimumContentWidth(minimumContentWidth);
	}
}

function findOverlayWidget(widgets: DisposableMap<string, OverlayWidget>, actual: IOverlayWidget): OverlayWidget | undefined {
	for (const [id, candidate] of widgets) {
		if (id === actual.getId() && candidate.actual === actual) return candidate;
	}
	return undefined;
}

class OverlayWidget extends Disposable {
	public readonly domNode: FastDomNode<HTMLElement>;
	public readonly allowEditorOverflow: boolean;
	public readonly actual: IOverlayWidget;
	public position: IOverlayWidgetPosition | null;
	public height = 0;

	constructor(private readonly options: ViewOverlayWidgetsOptions, actual: IOverlayWidget) {
		super();
		this.actual = actual;
		this.position = actual.getPosition();
		this.allowEditorOverflow = Boolean(actual.allowEditorOverflow) && options.allowOverflow;
		this.domNode = createFastDomNode(actual.getDomNode());
		this.domNode.setPosition(this.allowEditorOverflow && options.fixedOverflowWidgets ? 'fixed' : 'absolute');
		this.domNode.setAttribute('widgetId', actual.getId());
		if (actual.onDidLayout) this._register(actual.onDidLayout(() => options.requestRender()));
		this._register(toDisposable(() => {
			this.domNode.domNode.remove();
			this.domNode.removeAttribute('widgetId');
		}));
	}

	public get minimumContentWidth(): number {
		const width = this.actual.getMinContentWidthInPx?.();
		return typeof width === 'number' && Number.isFinite(width) && width > 0 ? width : 0;
	}

	public setPosition(position: IOverlayWidgetPosition | null): boolean {
		if (samePosition(this.position, position)) return false;
		this.position = position;
		return true;
	}

	public render(context: EditorRenderingContext, viewPagePosition: IDomNodePagePosition | null, stackOffset: number, layout: OverlayWidgetLayout): void {
		const preference = this.position?.preference;
		this.domNode.setDisplay(preference === null || preference === undefined ? 'none' : 'block');
		if (preference === null || preference === undefined) return;
		const rectangle = this.domNode.domNode.getBoundingClientRect();
		const width = rectangle.width || this.domNode.domNode.offsetWidth;
		this.height = rectangle.height || this.domNode.domNode.offsetHeight;
		const coordinates = isCoordinates(preference)
			? this.coordinatePosition(preference, context, viewPagePosition)
			: this.preferredPosition(preference, context, width, this.height, stackOffset, viewPagePosition, layout);
		this.domNode.setLeft(coordinates.left);
		this.domNode.setTop(coordinates.top);
	}

	private coordinatePosition(position: IOverlayWidgetPositionCoordinates, context: EditorRenderingContext, viewPagePosition: IDomNodePagePosition | null): IOverlayWidgetPositionCoordinates {
		if (!this.allowEditorOverflow || !viewPagePosition) return position;
		const targetWindow = this.options.viewDomNode.ownerDocument.defaultView;
		const fixed = this.options.fixedOverflowWidgets;
		return {
			left: viewPagePosition.left + position.left - (fixed ? targetWindow?.scrollX ?? 0 : 0),
			top: viewPagePosition.top + position.top - (fixed ? targetWindow?.scrollY ?? 0 : 0),
		};
	}

	private preferredPosition(preference: OverlayWidgetPositionPreference, context: EditorRenderingContext, width: number, height: number, stackOffset: number, viewPagePosition: IDomNodePagePosition | null, layout: OverlayWidgetLayout): IOverlayWidgetPositionCoordinates {
		const right = Math.max(0, layout.editorWidth - layout.verticalScrollbarWidth - layout.minimapWidth);
		let left = 0;
		let top = 0;
		if (preference === OverlayWidgetPositionPreference.TOP_RIGHT_CORNER) {
			left = Math.max(0, right - width);
			top = stackOffset;
		} else if (preference === OverlayWidgetPositionPreference.BOTTOM_RIGHT_CORNER) {
			left = Math.max(0, right - width);
			top = Math.max(0, layout.editorHeight - layout.horizontalScrollbarHeight - height - stackOffset);
		} else {
			left = Math.max(0, Math.round((layout.editorWidth - width) / 2));
			top = stackOffset;
		}
		return this.coordinatePosition({ left, top }, context, viewPagePosition);
	}
}

function isCoordinates(value: OverlayWidgetPositionPreference | IOverlayWidgetPositionCoordinates): value is IOverlayWidgetPositionCoordinates {
	return typeof value === 'object';
}

function samePosition(left: IOverlayWidgetPosition | null, right: IOverlayWidgetPosition | null): boolean {
	if (left === right) return true;
	if (!left || !right || left.stackOrdinal !== right.stackOrdinal) return false;
	const leftPreference = left.preference;
	const rightPreference = right.preference;
	if (leftPreference === rightPreference) return true;
	if (leftPreference === null || rightPreference === null) return false;
	return isCoordinates(leftPreference) && isCoordinates(rightPreference)
		&& leftPreference.top === rightPreference.top
		&& leftPreference.left === rightPreference.left;
}
