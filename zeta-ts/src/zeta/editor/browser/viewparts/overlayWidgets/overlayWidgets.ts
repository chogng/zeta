import './overlayWidgets.css';
import { createFastDomNode, type FastDomNode } from '../../../../base/browser/fastDomNode.js';
import { getDomNodePagePosition, type IRectangle } from '../../../../base/browser/geometry.js';
import { Disposable, DisposableMap, toDisposable } from '../../../../base/common/lifecycle.js';
import { type IOverlayWidget, type IOverlayWidgetPosition, type IOverlayWidgetPositionCoordinates, OverlayWidgetPositionPreference } from '../../editorBrowser.js';
import { PartFingerprint, PartFingerprints, type EditorRenderingContext, EditorViewPart } from '../../view/viewPart.js';

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

/** Owns widgets positioned against the editor viewport rather than document content. */
export class ViewOverlayWidgets extends EditorViewPart {
	public readonly domNode: FastDomNode<HTMLDivElement>;
	public readonly overflowingOverlayWidgetsDomNode: FastDomNode<HTMLDivElement>;
	private readonly widgets = this._register(new DisposableMap<string, OverlayWidget>());
	private viewDomNodePagePosition: IRectangle | null = null;

	constructor(private readonly options: ViewOverlayWidgetsOptions) {
		super();
		this.domNode = createFastDomNode(options.viewDomNode.ownerDocument.createElement('div'));
		PartFingerprints.write(this.domNode, PartFingerprint.OverlayWidgets);
		this.domNode.setClassName('stanza-editor-overlay-widgets');
		this.domNode.setPosition('absolute');
		this.domNode.setTop(0);
		this.domNode.setAttribute('role', 'presentation');
		this.overflowingOverlayWidgetsDomNode = createFastDomNode(options.viewDomNode.ownerDocument.createElement('div'));
		PartFingerprints.write(this.overflowingOverlayWidgetsDomNode, PartFingerprint.OverflowingOverlayWidgets);
		this.overflowingOverlayWidgetsDomNode.setClassName('stanza-editor-overflowing-overlay-widgets');
		this.overflowingOverlayWidgetsDomNode.setAttribute('role', 'presentation');
		this._register(toDisposable(() => {
			this.domNode.domNode.remove();
			this.overflowingOverlayWidgetsDomNode.domNode.remove();
		}));
	}

	public addWidget(widget: IOverlayWidget): void {
		const id = widget.getId();
		if (!id || this.widgets.has(id)) throw new RangeError(`Overlay widget '${id}' is already registered`);
		const overlayWidget = this.widgets.set(id, new OverlayWidget(this.options, widget));
		const container = overlayWidget.allowEditorOverflow ? this.overflowingOverlayWidgetsDomNode : this.domNode;
		container.domNode.append(overlayWidget.domNode.domNode);
		this.updateMinimumContentWidth();
	}

	public setWidgetPosition(widget: IOverlayWidget, position: IOverlayWidgetPosition | null): void {
		this.widget(widget)?.setPosition(position);
	}

	public removeWidget(widget: IOverlayWidget): void {
		const id = widget.getId();
		if (!this.widget(widget)) return;
		this.widgets.deleteAndDispose(id);
		this.updateMinimumContentWidth();
	}

	public override prepareRender(): void {
		this.viewDomNodePagePosition = getDomNodePagePosition(this.options.viewDomNode);
	}

	public render(context: EditorRenderingContext): void {
		const stacks = new Map<OverlayWidgetPositionPreference, number>();
		for (const preference of [OverlayWidgetPositionPreference.TOP_RIGHT_CORNER, OverlayWidgetPositionPreference.BOTTOM_RIGHT_CORNER, OverlayWidgetPositionPreference.TOP_CENTER]) {
			stacks.set(preference, 0);
		}
		const widgets = [...this.widgets].map(([, widget]) => widget);
		widgets.sort((left, right) => (left.position?.stackOrdinal ?? 0) - (right.position?.stackOrdinal ?? 0));
		for (const widget of widgets) {
			const preference = widget.position?.preference;
			const stackOffset = typeof preference === 'number' ? stacks.get(preference) ?? 0 : 0;
			widget.render(context, this.viewDomNodePagePosition, stackOffset);
			if (typeof preference === 'number') stacks.set(preference, stackOffset + widget.height);
		}
	}

	private widget(widget: IOverlayWidget): OverlayWidget | undefined {
		for (const [id, candidate] of this.widgets) {
			if (id === widget.getId() && candidate.actual === widget) return candidate;
		}
		return undefined;
	}

	private updateMinimumContentWidth(): void {
		let minimumContentWidth = 0;
		for (const [, widget] of this.widgets) minimumContentWidth = Math.max(minimumContentWidth, widget.minimumContentWidth);
		this.options.setMinimumContentWidth(minimumContentWidth);
	}
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

	public setPosition(position: IOverlayWidgetPosition | null): void {
		this.position = position;
	}

	public render(context: EditorRenderingContext, viewPagePosition: IRectangle | null, stackOffset: number): void {
		const preference = this.position?.preference;
		this.domNode.setDisplay(preference === null || preference === undefined ? 'none' : 'block');
		if (preference === null || preference === undefined) return;
		const rectangle = this.domNode.domNode.getBoundingClientRect();
		const width = rectangle.width || this.domNode.domNode.offsetWidth;
		this.height = rectangle.height || this.domNode.domNode.offsetHeight;
		const coordinates = isCoordinates(preference)
			? this.coordinatePosition(preference, context, viewPagePosition)
			: this.preferredPosition(preference, context, width, this.height, stackOffset, viewPagePosition);
		this.domNode.setLeft(coordinates.left);
		this.domNode.setTop(coordinates.top);
	}

	private coordinatePosition(position: IOverlayWidgetPositionCoordinates, context: EditorRenderingContext, viewPagePosition: IRectangle | null): IOverlayWidgetPositionCoordinates {
		if (!this.allowEditorOverflow || !viewPagePosition) return position;
		const targetWindow = this.options.viewDomNode.ownerDocument.defaultView;
		const fixed = this.options.fixedOverflowWidgets;
		return {
			left: viewPagePosition.left + position.left - (fixed ? targetWindow?.scrollX ?? 0 : 0),
			top: viewPagePosition.top + position.top - (fixed ? targetWindow?.scrollY ?? 0 : 0),
		};
	}

	private preferredPosition(preference: OverlayWidgetPositionPreference, context: EditorRenderingContext, width: number, height: number, stackOffset: number, viewPagePosition: IRectangle | null): IOverlayWidgetPositionCoordinates {
		const viewportWidth = context.layout.viewportSize.width;
		const viewportHeight = context.layout.viewportSize.height;
		const minimapWidth = this.options.readMinimapWidth();
		const right = Math.max(0, viewportWidth - this.options.verticalScrollbarWidth - minimapWidth);
		let left = 0;
		let top = 0;
		if (preference === OverlayWidgetPositionPreference.TOP_RIGHT_CORNER) {
			left = Math.max(0, right - width);
			top = stackOffset;
		} else if (preference === OverlayWidgetPositionPreference.BOTTOM_RIGHT_CORNER) {
			left = Math.max(0, right - width);
			top = Math.max(0, viewportHeight - this.options.horizontalScrollbarHeight - height - stackOffset);
		} else {
			left = Math.max(0, Math.round((viewportWidth - width) / 2));
			top = stackOffset;
		}
		return this.coordinatePosition({ left, top }, context, viewPagePosition);
	}
}

function isCoordinates(value: OverlayWidgetPositionPreference | IOverlayWidgetPositionCoordinates): value is IOverlayWidgetPositionCoordinates {
	return typeof value === 'object';
}
