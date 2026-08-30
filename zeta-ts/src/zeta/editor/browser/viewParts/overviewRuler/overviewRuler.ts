import { h, reset, fragment as createFragment } from '../../../../base/browser/dom.js';
import { FastDomNode } from '../../../../base/browser/fastDomNode.js';
import { toDisposable } from '../../../../base/common/lifecycle.js';
import { OverviewRulerZone, OverviewZoneManager } from '../../../common/viewModel/overviewZoneManager.js';
import { type EditorRenderingContext, EditorViewPart } from '../../view/viewPart.js';
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
export class OverviewRuler extends EditorViewPart {
	public readonly domNode: HTMLDivElement;
	private readonly root: FastDomNode<HTMLDivElement>;
	private readonly zoneManager: OverviewZoneManager;
	private entries: readonly { readonly entry: OverviewRulerEntry; readonly zone: OverviewRulerZone }[] = [];
	private renderedRevision = -1;

	constructor(private readonly options: OverviewRulerOptions) {
		super();
		if (!Number.isFinite(options.width) || options.width <= 0) throw new RangeError('Overview ruler width must be finite and positive');
		this.zoneManager = new OverviewZoneManager(lineNumber => options.getVerticalOffsetForLineIndex(lineNumber - 1));
		this.domNode = h(options.host.ownerDocument, 'div');
		this._register(toDisposable(() => this.domNode.remove()));
		this.root = new FastDomNode(this.domNode);
		this.root.setClassName(options.className);
		this.domNode.setAttribute('role', 'presentation');
		this.domNode.setAttribute('aria-hidden', 'true');
	}

	public render(context: EditorRenderingContext): void {
		const layout = context.layout;
		this.root.setLeft(layout.scrollPosition.left + Math.max(
			0,
			layout.viewportSize.width - this.options.verticalScrollbarWidth + (this.options.verticalScrollbarWidth - this.options.width) / 2,
		));
		this.root.setTop(layout.scrollPosition.top);
		this.root.setWidth(this.options.width);
		this.root.setHeight(layout.viewportSize.height);
		let geometryChanged = this.zoneManager.setLineHeight(layout.lineHeight);
		geometryChanged = this.zoneManager.setDOMWidth(this.options.width) || geometryChanged;
		geometryChanged = this.zoneManager.setDOMHeight(layout.viewportSize.height) || geometryChanged;
		geometryChanged = this.zoneManager.setOuterHeight(layout.contentSize.height) || geometryChanged;
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
