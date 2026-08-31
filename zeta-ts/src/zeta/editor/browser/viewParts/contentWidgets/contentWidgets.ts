import * as dom from '../../../../base/browser/dom.js';
import { createFastDomNode, type FastDomNode } from '../../../../base/browser/fastDomNode.js';
import { getClientArea, getDomNodePagePosition } from '../../../../base/browser/dom.js';
import { Disposable, DisposableMap, toDisposable } from '../../../../base/common/lifecycle.js';
import { ContentWidgetPositionPreference, type IContentWidget, type IContentWidgetPosition, type IContentWidgetRenderedCoordinate } from '../../editorBrowser.js';
import { type IPosition, Position } from '../../../common/core/position.js';
import { type IDimension } from '../../../common/core/2d/dimension.js';
import { PositionAffinity } from '../../../common/model.js';
import { PartFingerprint, PartFingerprints, type EditorRenderingContext, ViewPart } from '../../view/viewPart.js';
import { type ViewContext } from '../../../common/viewModel/viewContext.js';

interface ViewContentWidgetsOptions {
	readonly viewDomNode: HTMLElement;
	readonly allowOverflow: boolean;
	readonly fixedOverflowWidgets: boolean;
	readonly readContentLeft: () => number;
	readonly readContentWidth: () => number;
}

interface BoxLayoutResult {
	readonly fitsAbove: boolean;
	readonly aboveTop: number;
	readonly fitsBelow: boolean;
	readonly belowTop: number;
	readonly left: number;
}

interface OffViewportRenderData {
	readonly kind: 'offViewport';
	readonly preserveFocus: boolean;
}

interface InViewportRenderData {
	readonly kind: 'inViewport';
	readonly coordinate: RenderedCoordinate;
	readonly position: ContentWidgetPositionPreference;
}

type RenderData = InViewportRenderData | OffViewportRenderData;

export class ViewContentWidgets extends ViewPart {
	public readonly domNode: FastDomNode<HTMLDivElement>;
	public readonly overflowingContentWidgetsDomNode: FastDomNode<HTMLDivElement>;
	private readonly widgets = this._register(new DisposableMap<string, ContentWidget>());

	constructor(context: ViewContext, private readonly options: ViewContentWidgetsOptions) {
		super(context);
		this.domNode = createFastDomNode(options.viewDomNode.ownerDocument.createElement('div'));
		PartFingerprints.write(this.domNode, PartFingerprint.ContentWidgets);
		this.domNode.setClassName('stanza-editor-content-widgets');
		this.domNode.setPosition('absolute');
		this.domNode.setTop(0);
		this.domNode.setAttribute('role', 'presentation');
		this.overflowingContentWidgetsDomNode = createFastDomNode(options.viewDomNode.ownerDocument.createElement('div'));
		PartFingerprints.write(this.overflowingContentWidgetsDomNode, PartFingerprint.OverflowingContentWidgets);
		this.overflowingContentWidgetsDomNode.setClassName('stanza-editor-overflowing-content-widgets');
		this.overflowingContentWidgetsDomNode.setAttribute('role', 'presentation');
		this._register(toDisposable(() => {
			this.domNode.domNode.remove();
			this.overflowingContentWidgetsDomNode.domNode.remove();
		}));
	}

	public addWidget(widget: IContentWidget): void {
		const id = widget.getId();
		if (!id || this.widgets.has(id)) throw new RangeError(`Content widget '${id}' is already registered`);
		const contentWidget = this.widgets.set(id, new ContentWidget(this.options, widget));
		if (contentWidget.allowEditorOverflow) {
			this.overflowingContentWidgetsDomNode.domNode.append(contentWidget.domNode.domNode);
		} else {
			this.domNode.domNode.append(contentWidget.domNode.domNode);
		}
	}

	public setWidgetPosition(widget: IContentWidget, position: IContentWidgetPosition | null): void {
		this.widget(widget)?.setPosition(position);
	}

	public removeWidget(widget: IContentWidget): void {
		const id = widget.getId();
		if (!this.widget(widget)) return;
		this.widgets.deleteAndDispose(id);
	}

	public override prepareRender(context: EditorRenderingContext): void {
		for (const [, widget] of this.widgets) widget.prepareRender(context);
	}

	public render(context: EditorRenderingContext): void {
		for (const [, widget] of this.widgets) widget.render(context);
	}

	private widget(widget: IContentWidget): ContentWidget | undefined {
		for (const [id, candidate] of this.widgets) {
			if (id === widget.getId() && candidate.actual === widget) return candidate;
		}
		return undefined;
	}
}

class ContentWidget extends Disposable {
	public readonly domNode: FastDomNode<HTMLElement>;
	public readonly id: string;
	public readonly allowEditorOverflow: boolean;
	public readonly useDisplayNone: boolean;
	public readonly actual: IContentWidget;
	private readonly fixedOverflowWidgets: boolean;
	private position: IContentWidgetPosition | null = null;
	private cachedWidth = -1;
	private cachedHeight = -1;
	private maxWidth = -1;
	private visible = false;
	private renderData: RenderData | null = null;

	constructor(private readonly options: ViewContentWidgetsOptions, actual: IContentWidget) {
		super();
		this.actual = actual;
		this.id = actual.getId();
		this.allowEditorOverflow = Boolean(actual.allowEditorOverflow) && options.allowOverflow;
		this.useDisplayNone = Boolean(actual.useDisplayNone);
		this.fixedOverflowWidgets = options.fixedOverflowWidgets;
		this.domNode = createFastDomNode(actual.getDomNode());
		this.domNode.setPosition(this.fixedOverflowWidgets && this.allowEditorOverflow ? 'fixed' : 'absolute');
		this.domNode.setDisplay('none');
		this.domNode.setVisibility('hidden');
		this.domNode.setAttribute('widgetId', this.id);
		if (actual.suppressMouseDown) this._register(dom.addDisposableListener(this.domNode.domNode, 'mousedown', event => event.preventDefault()));
		this._register(toDisposable(() => {
			this.domNode.domNode.remove();
			this.domNode.removeAttribute('widgetId');
			this.domNode.removeAttribute('data-stanza-visible-content-widget');
		}));
	}

	public setPosition(position: IContentWidgetPosition | null): void {
		this.position = position;
		const wantsDisplay = !this.useDisplayNone && position?.position !== null && position?.position !== undefined && position.preference.length > 0;
		this.domNode.setDisplay(wantsDisplay ? 'block' : 'none');
		this.cachedWidth = -1;
		this.cachedHeight = -1;
	}

	public prepareRender(context: EditorRenderingContext): void {
		this.renderData = this.prepareRenderData(context);
	}

	public render(_context: EditorRenderingContext): void {
		const renderData = this.renderData;
		if (!renderData || renderData.kind === 'offViewport') {
			if (renderData?.preserveFocus) {
				this.domNode.setTop(-1000);
				this.domNode.setVisibility('inherit');
			} else {
				this.domNode.setVisibility('hidden');
			}
			if (this.visible) {
				this.domNode.removeAttribute('data-stanza-visible-content-widget');
				this.visible = false;
			}
			safeInvoke(this.actual.afterRender, this.actual, null, null);
			return;
		}

		this.domNode.setTop(renderData.coordinate.top);
		this.domNode.setLeft(renderData.coordinate.left);
		if (!this.visible) {
			this.domNode.setVisibility('inherit');
			this.domNode.setAttribute('data-stanza-visible-content-widget', 'true');
			this.visible = true;
		}
		safeInvoke(this.actual.afterRender, this.actual, renderData.position, renderData.coordinate);
	}

	private prepareRenderData(context: EditorRenderingContext): RenderData | null {
		const position = this.position;
		if (this.useDisplayNone || !position || !position.position || position.preference.length === 0) return null;
		const primary = anchorCoordinate(context, position.position, position.positionAffinity);
		if (!primary) {
			return {
				kind: 'offViewport',
				preserveFocus: this.domNode.domNode.contains(this.domNode.domNode.ownerDocument.activeElement),
			};
		}
		const secondary = position.secondaryPosition ? anchorCoordinate(context, position.secondaryPosition, position.positionAffinity) : null;
		this.updateDimensions();
		const anchor = reduceAnchor(primary, secondary?.visualLineIndex === primary.visualLineIndex ? secondary : null, this.cachedWidth, context);
		const placement = this.allowEditorOverflow
			? this.layoutBoxInPage(anchor, context)
			: layoutBoxInViewport(anchor, this.cachedWidth, this.cachedHeight, context);
		for (let pass = 1; pass <= 2; pass += 1) {
			for (const preference of position.preference) {
				if (preference === ContentWidgetPositionPreference.ABOVE && placement && (pass === 2 || placement.fitsAbove)) {
					return { kind: 'inViewport', coordinate: new RenderedCoordinate(placement.aboveTop, placement.left), position: preference };
				}
				if (preference === ContentWidgetPositionPreference.BELOW && placement && (pass === 2 || placement.fitsBelow)) {
					return { kind: 'inViewport', coordinate: new RenderedCoordinate(placement.belowTop, placement.left), position: preference };
				}
				if (preference === ContentWidgetPositionPreference.EXACT) {
					return { kind: 'inViewport', coordinate: this.exactCoordinate(anchor, context), position: preference };
				}
			}
		}
		return null;
	}

	private updateDimensions(): void {
		const nextMaxWidth = this.allowEditorOverflow
			? getClientArea(this.options.viewDomNode.ownerDocument.body).width
			: Math.max(0, this.options.readContentWidth());
		if (nextMaxWidth !== this.maxWidth) {
			this.maxWidth = nextMaxWidth;
			this.domNode.setMaxWidth(nextMaxWidth);
			this.cachedWidth = -1;
			this.cachedHeight = -1;
		}
		if (this.cachedWidth >= 0 && this.cachedHeight >= 0) return;
		const preferred = safeInvoke(this.actual.beforeRender, this.actual);
		if (preferred && validDimension(preferred)) {
			this.cachedWidth = preferred.width;
			this.cachedHeight = preferred.height;
			return;
		}
		const rectangle = this.domNode.domNode.getBoundingClientRect();
		this.cachedWidth = Math.round(rectangle.width);
		this.cachedHeight = Math.round(rectangle.height);
	}

	private layoutBoxInPage(anchor: AnchorCoordinate, context: EditorRenderingContext): BoxLayoutResult {
		const viewPosition = getDomNodePagePosition(this.options.viewDomNode);
		const ownerWindow = this.options.viewDomNode.ownerDocument.defaultView;
		const windowScrollLeft = ownerWindow?.scrollX ?? 0;
		const windowScrollTop = ownerWindow?.scrollY ?? 0;
		const viewport = getClientArea(this.options.viewDomNode.ownerDocument.body);
		const fixed = this.fixedOverflowWidgets;
		const anchorLeft = viewPosition.left + this.options.readContentLeft() + anchor.left - context.layout.scrollPosition.left - (fixed ? windowScrollLeft : 0);
		const anchorTop = viewPosition.top + anchor.top - context.layout.scrollPosition.top - (fixed ? windowScrollTop : 0);
		const minimumLeft = (fixed ? 0 : windowScrollLeft) + 15;
		const maximumRight = (fixed ? viewport.width : windowScrollLeft + viewport.width) - 15;
		const left = Math.min(Math.max(anchorLeft, minimumLeft), Math.max(minimumLeft, maximumRight - this.cachedWidth));
		const viewportTop = (fixed ? 0 : windowScrollTop) + 22;
		const viewportBottom = (fixed ? viewport.height : windowScrollTop + viewport.height) - 22;
		return {
			fitsAbove: anchorTop - this.cachedHeight >= viewportTop,
			aboveTop: Math.max(viewportTop, anchorTop - this.cachedHeight),
			fitsBelow: anchorTop + anchor.height + this.cachedHeight <= viewportBottom,
			belowTop: anchorTop + anchor.height,
			left,
		};
	}

	private exactCoordinate(anchor: AnchorCoordinate, context: EditorRenderingContext): RenderedCoordinate {
		if (!this.allowEditorOverflow) return new RenderedCoordinate(anchor.top, anchor.left);
		const placement = this.layoutBoxInPage(anchor, context);
		const viewPosition = getDomNodePagePosition(this.options.viewDomNode);
		const ownerWindow = this.options.viewDomNode.ownerDocument.defaultView;
		return new RenderedCoordinate(
			viewPosition.top + anchor.top - context.layout.scrollPosition.top - (this.fixedOverflowWidgets ? ownerWindow?.scrollY ?? 0 : 0),
			placement.left,
		);
	}
}

class AnchorCoordinate {
	constructor(
		public readonly top: number,
		public readonly left: number,
		public readonly height: number,
		public readonly visualLineIndex: number,
	) {}
}

class RenderedCoordinate implements IContentWidgetRenderedCoordinate {
	constructor(
		public readonly top: number,
		public readonly left: number,
	) {}
}

function anchorCoordinate(context: EditorRenderingContext, position: IPosition, affinity: PositionAffinity | undefined): AnchorCoordinate | null {
	const overlay = context.overlay;
	if (!overlay) return null;
	let validPosition: Position;
	try {
		validPosition = Position.lift(position);
		overlay.model.offsetAt(validPosition);
	} catch {
		return null;
	}
	const visualLineIndex = overlay.visualLineProjection.visualLineIndexAt(validPosition);
	if (visualLineIndex < context.layout.visibleLines.startLineIndex || visualLineIndex >= context.layout.visibleLines.endLineIndexExclusive) return null;
	const visualLine = overlay.visualLineProjection.lineAt(visualLineIndex);
	if (!visualLine) return null;
	const renderedPosition = overlay.visibleRangeForPosition(validPosition);
	const left = validPosition.column === 1 && affinity === PositionAffinity.LeftOfInjectedText
		? 0
		: renderedPosition?.left ?? overlay.textLeft + (visualLine.wrappedTextIndentWidth ?? 0) + overlay.textMeasurer.measureLineWidth(
			overlay.model.getLineContent((visualLine.logicalLineIndex) + 1).slice(visualLine.startColumn, validPosition.column - 1),
		);
	return new AnchorCoordinate(context.viewportData.getLineTop(visualLineIndex), left, context.layout.lineHeight, visualLineIndex);
}

function reduceAnchor(primary: AnchorCoordinate, secondary: AnchorCoordinate | null, width: number, context: EditorRenderingContext): AnchorCoordinate {
	if (!secondary) return primary;
	const clearance = context.overlay?.textMeasurer.measureLineWidth('Ｍ') ?? 0;
	const left = secondary.left < primary.left
		? Math.max(secondary.left, primary.left - width + clearance)
		: Math.min(secondary.left, primary.left + width - clearance);
	return new AnchorCoordinate(primary.top, left, primary.height, primary.visualLineIndex);
}

function layoutBoxInViewport(anchor: AnchorCoordinate, width: number, height: number, context: EditorRenderingContext): BoxLayoutResult {
	const scrollTop = context.layout.scrollPosition.top;
	const scrollLeft = context.layout.scrollPosition.left;
	const viewportBottom = scrollTop + context.layout.viewportSize.height;
	const viewportRight = scrollLeft + context.layout.viewportSize.width;
	return {
		fitsAbove: anchor.top - scrollTop >= height,
		aboveTop: anchor.top - height,
		fitsBelow: viewportBottom - (anchor.top + anchor.height) >= height,
		belowTop: anchor.top + anchor.height,
		left: Math.max(scrollLeft, Math.min(anchor.left, viewportRight - width)),
	};
}

function validDimension(dimension: IDimension): boolean {
	return Number.isFinite(dimension.width) && dimension.width >= 0 && Number.isFinite(dimension.height) && dimension.height >= 0;
}

function safeInvoke<TArguments extends unknown[], TResult>(fn: ((...arguments_: TArguments) => TResult) | undefined, thisArgument: unknown, ...arguments_: TArguments): TResult | null {
	if (!fn) return null;
	try {
		return fn.apply(thisArgument, arguments_);
	} catch {
		return null;
	}
}
