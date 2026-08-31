import { h, reset, fragment as createFragment } from '../../../../base/browser/dom.js';
import { FastDomNode } from '../../../../base/browser/fastDomNode.js';
import { toDisposable } from '../../../../base/common/lifecycle.js';
import { OverviewRulerZone, OverviewZoneManager } from '../../../common/viewModel/overviewZoneManager.js';
import { type RestrictedRenderingContext } from '../../view/renderingContext.js';
import { ViewPart } from '../../view/viewPart.js';
import { type ViewContext } from '../../../common/viewModel/viewContext.js';
import { type DecorationPresentation } from '../decorations/decorations.js';

export interface DiagnosticOverviewMarker {
	readonly startLineIndex: number;
	readonly endLineIndexExclusive: number;
	readonly presentation: DecorationPresentation;
	readonly hoverText: string | undefined;
}

export interface DiffOverviewMarker {
	readonly startLineIndex: number;
	readonly endLineIndexExclusive: number;
	readonly presentation: DecorationPresentation.DiffAdded | DecorationPresentation.DiffModified | DecorationPresentation.DiffDeleted;
	readonly hoverText: string | undefined;
}

export interface OverviewRulerEntry {
	readonly startLineIndex: number;
	readonly endLineIndexExclusive: number;
	readonly heightInLines?: number;
	readonly className: string;
	readonly hoverText?: string;
}

export interface OverviewRulerOptions {
	readonly host: HTMLElement;
	readonly className: string;
	readonly width: number;
	readonly verticalScrollbarWidth: number;
	readonly getVerticalOffsetForLineIndex: (lineIndex: number) => number;
	readonly readEntries: () => readonly OverviewRulerEntry[];
	readonly readEntriesRevision: () => number;
}

/** Owns overview-ruler layout and line-to-pixel zone projection. */
export class OverviewRuler extends ViewPart {
	public readonly domNode: HTMLDivElement;
	private readonly root: FastDomNode<HTMLDivElement>;
	private readonly zoneManager: OverviewZoneManager;
	private entries: readonly { readonly entry: OverviewRulerEntry; readonly zone: OverviewRulerZone }[] = [];
	private renderedRevision = -1;

	constructor(context: ViewContext, private readonly options: OverviewRulerOptions) {
		super(context);
		if (!Number.isFinite(options.width) || options.width <= 0) throw new RangeError('Overview ruler width must be finite and positive');
		this.zoneManager = new OverviewZoneManager(lineNumber => options.getVerticalOffsetForLineIndex(lineNumber - 1));
		this.domNode = h(options.host.ownerDocument, 'div');
		this._register(toDisposable(() => this.domNode.remove()));
		this.root = new FastDomNode(this.domNode);
		this.root.setClassName(options.className);
		this.domNode.setAttribute('role', 'presentation');
		this.domNode.setAttribute('aria-hidden', 'true');
	}

	public render(context: RestrictedRenderingContext): void {
		this.root.setLeft(context.scrollLeft + Math.max(
			0,
			context.viewportWidth - this.options.verticalScrollbarWidth + (this.options.verticalScrollbarWidth - this.options.width) / 2,
		));
		this.root.setTop(context.scrollTop);
		this.root.setWidth(this.options.width);
		this.root.setHeight(context.viewportHeight);
		let geometryChanged = this.zoneManager.setLineHeight(context.viewportData.lineHeight);
		geometryChanged = this.zoneManager.setDOMWidth(this.options.width) || geometryChanged;
		geometryChanged = this.zoneManager.setDOMHeight(context.viewportHeight) || geometryChanged;
		geometryChanged = this.zoneManager.setOuterHeight(context.scrollHeight) || geometryChanged;
		const revision = this.options.readEntriesRevision();
		if (revision !== this.renderedRevision) {
			this.entries = Object.freeze(this.options.readEntries().map(entry => Object.freeze({
				entry,
				zone: new OverviewRulerZone(entry.startLineIndex + 1, entry.endLineIndexExclusive, entry.heightInLines ?? 0, entry.className),
			})));
			this.zoneManager.setZones(this.entries.map(({ zone }) => zone));
		}
		if (!geometryChanged && revision === this.renderedRevision) return;
		this.zoneManager.resolveColorZones();
		const fragment = createFragment(this.domNode.ownerDocument);
		for (const { entry, zone } of this.entries) {
			const colorZone = zone.getColorZones();
			if (!colorZone) continue;
			const element = h(this.domNode.ownerDocument, 'span');
			element.className = 'stanza-editor-overview-marker';
			element.classList.add(entry.className);
			element.style.top = `${colorZone.from}px`;
			element.style.height = `${Math.max(1, colorZone.to - colorZone.from)}px`;
			if (entry.hoverText !== undefined) element.title = entry.hoverText;
			fragment.append(element);
		}
		reset(this.domNode, fragment);
		this.renderedRevision = revision;
	}
}
