import { addDisposableListener, h } from '../../../../base/browser/dom.js';
import { AbstractDisposable, DisposableMap, toDisposable, type IDisposable } from '../../../../base/common/lifecycle.js';
import { isFiniteNumber } from '../../../../base/common/numbers.js';
import { type EditorViewportLayout, EditorViewportLayoutManager } from '../../../common/viewLayout/viewLayout.js';
import { type IViewZone, type IViewZoneChangeAccessor } from '../../editorBrowser.js';
import { EditorViewPart, type EditorRenderingContext } from '../../view/viewPart.js';

export type EditorViewZone = IViewZone;

export interface EditorViewZoneHandle extends IDisposable {
	readonly top: number;
	readonly heightInPixels: number;
	layout(): void;
}

interface ViewZonesOptions {
	readonly host: HTMLElement;
	readonly viewLayout: EditorViewportLayoutManager;
	readonly readVisualLineCount: () => number;
	readonly readVisualLineIndexAfterPosition: (lineNumber: number, column: number | undefined) => number;
	readonly readContentLeft: () => number;
	readonly readContentWidth: () => number;
	readonly setMinimumContentWidth: (width: number) => void;
}

/** Owns caller-provided DOM roots placed in reserved vertical editor space. */
export class ViewZones extends EditorViewPart {
	public readonly domNode: HTMLDivElement;
	public readonly marginDomNode: HTMLDivElement;
	private readonly viewLayout: EditorViewportLayoutManager;
	private readonly readVisualLineCount: () => number;
	private readonly zones = new Map<string, EditorViewZone>();
	private readonly mouseDownListeners = this._register(new DisposableMap<string>());
	private readonly zoneLayouts = new Map<string, { readonly top: number; readonly heightInPixels: number }>();
	private lineHeight: number;

	constructor(private readonly options: ViewZonesOptions) {
		super();
		this.viewLayout = options.viewLayout;
		this.readVisualLineCount = options.readVisualLineCount;
		this.lineHeight = options.viewLayout.layout.lineHeight;
		this.domNode = h(options.host.ownerDocument, 'div');
		this.domNode.className = 'stanza-editor-view-zones';
		this.domNode.setAttribute('role', 'presentation');
		this.domNode.setAttribute('aria-hidden', 'true');
		this.marginDomNode = h(options.host.ownerDocument, 'div');
		this.marginDomNode.className = 'stanza-editor-margin-view-zones';
		this.marginDomNode.setAttribute('role', 'presentation');
		this.marginDomNode.setAttribute('aria-hidden', 'true');
		this._register(toDisposable(() => {
			for (const zone of this.zones.values()) {
				zone.domNode.remove();
				zone.marginDomNode?.remove();
			}
			this.zones.clear();
			this.zoneLayouts.clear();
			this.domNode.remove();
			this.marginDomNode.remove();
		}));
	}

	public addZone(zone: EditorViewZone): EditorViewZoneHandle {
		this.assertNotDisposed();
		const id = this.addZoneData(zone);
		return new ViewZoneHandle(
			() => this.layoutZone(id),
			() => this.removeZone(id),
			() => this.zoneLayouts.get(id),
		);
	}

	public changeViewZones(callback: (accessor: IViewZoneChangeAccessor) => void): void {
		this.assertNotDisposed();
		if (typeof callback !== 'function') throw new TypeError('View zone changes require a callback');
		let valid = true;
		const assertValid = (): void => {
			if (!valid) throw new Error('View zone change accessor is no longer valid');
		};
		const accessor: IViewZoneChangeAccessor = {
			addZone: zone => {
				assertValid();
				return this.addZoneData(zone);
			},
			removeZone: id => {
				assertValid();
				this.removeZone(id);
			},
			layoutZone: id => {
				assertValid();
				this.layoutZone(id);
			},
		};
		try {
			callback(accessor);
		} finally {
			valid = false;
			this.layoutZones(this.viewLayout.layout);
		}
	}

	public render(context: EditorRenderingContext): void {
		this.layoutZones(context.layout);
	}

	public setLineHeight(lineHeight: number): void {
		if (lineHeight === this.lineHeight) return;
		this.lineHeight = lineHeight;
		for (const [id, zone] of this.zones) {
			if (zone.heightInPx !== undefined) continue;
			this.viewLayout.changeViewZone(id, this.zoneLineIndex(zone), this.zoneHeight(zone), zone.ordinal);
		}
	}

	private addZoneData(zone: EditorViewZone): string {
		this.validateZone(zone);
		const id = this.viewLayout.addViewZone(this.zoneLineIndex(zone), this.zoneHeight(zone), zone.ordinal);
		this.zones.set(id, zone);
		zone.domNode.classList.add('stanza-editor-view-zone');
		this.domNode.append(zone.domNode);
		if (zone.marginDomNode) {
			zone.marginDomNode.classList.add('stanza-editor-margin-view-zone');
			this.marginDomNode.append(zone.marginDomNode);
		}
		if (zone.suppressMouseDown) this.mouseDownListeners.set(id, addDisposableListener(zone.domNode, 'mousedown', event => event.preventDefault()));
		this.updateMinimumContentWidth();
		this.layoutZones(this.viewLayout.layout);
		return id;
	}

	private layoutZone(id: string): void {
		const zone = this.zones.get(id);
		if (!zone) {
			return;
		}
		this.validateZone(zone);
		this.layoutZones(this.viewLayout.changeViewZone(id, this.zoneLineIndex(zone), this.zoneHeight(zone), zone.ordinal));
	}

	private removeZone(id: string): void {
		const zone = this.zones.get(id);
		if (!zone) {
			return;
		}
		this.zones.delete(id);
		this.mouseDownListeners.deleteAndDispose(id);
		this.zoneLayouts.delete(id);
		zone.domNode.remove();
		zone.marginDomNode?.remove();
		this.updateMinimumContentWidth();
		if (!this.isDisposed) {
			this.layoutZones(this.viewLayout.removeViewZone(id));
		}
	}

	private layoutZones(layout: EditorViewportLayout): void {
		this.domNode.style.left = `${this.options.readContentLeft()}px`;
		this.domNode.style.width = `${this.options.readContentWidth()}px`;
		this.domNode.style.height = `${layout.contentSize.height}px`;
		this.marginDomNode.style.width = `${this.options.readContentLeft()}px`;
		this.marginDomNode.style.height = `${layout.contentSize.height}px`;
		this.zoneLayouts.clear();
		for (const geometry of layout.viewZones ?? []) {
			this.zoneLayouts.set(geometry.id, geometry);
			const zone = this.zones.get(geometry.id);
			if (!zone) {
				continue;
			}
			zone.domNode.style.top = `${geometry.top}px`;
			zone.domNode.style.height = `${geometry.heightInPixels}px`;
			zone.domNode.style.width = `${this.options.readContentWidth()}px`;
			if (zone.marginDomNode) {
				zone.marginDomNode.style.top = `${geometry.top}px`;
				zone.marginDomNode.style.height = `${geometry.heightInPixels}px`;
				zone.marginDomNode.style.width = `${this.options.readContentLeft()}px`;
			}
			zone.onDomNodeTop?.(geometry.top - layout.scrollPosition.top);
			zone.onComputedHeight?.(geometry.heightInPixels);
		}
	}

	private updateMinimumContentWidth(): void {
		let width = 0;
		for (const zone of this.zones.values()) width = Math.max(width, zone.minWidthInPx ?? 0);
		this.options.setMinimumContentWidth(width);
	}

	private validateZone(zone: EditorViewZone): void {
		if (!zone || !(zone.domNode instanceof this.domNode.ownerDocument.defaultView!.HTMLElement)) {
			throw new TypeError('Editor view zone requires a DOM root from the editor document');
		}
		if (!Number.isSafeInteger(zone.afterLineNumber) || zone.afterLineNumber < 0) {
			throw new RangeError('Editor view zone line number is outside the visual line collection');
		}
		this.zoneLineIndex(zone);
		if (zone.heightInPx !== undefined && (!isFiniteNumber(zone.heightInPx) || zone.heightInPx <= 0)) {
			throw new RangeError('Editor view zone height must be finite and positive');
		}
		if (zone.heightInPx === undefined && zone.heightInLines !== undefined && (!isFiniteNumber(zone.heightInLines) || zone.heightInLines <= 0)) {
			throw new RangeError('Editor view zone line height must be finite and positive');
		}
		if (zone.ordinal !== undefined && !isFiniteNumber(zone.ordinal)) {
			throw new RangeError('Editor view zone ordinal must be finite');
		}
		if (zone.minWidthInPx !== undefined && (!isFiniteNumber(zone.minWidthInPx) || zone.minWidthInPx < 0)) {
			throw new RangeError('Editor view zone minimum width must be finite and non-negative');
		}
	}

	private zoneHeight(zone: EditorViewZone): number {
		return zone.heightInPx ?? (zone.heightInLines ?? 1) * this.lineHeight;
	}

	private zoneLineIndex(zone: EditorViewZone): number {
		const index = this.options.readVisualLineIndexAfterPosition(zone.afterLineNumber, zone.afterColumn);
		if (!Number.isSafeInteger(index) || index < -1 || index >= this.readVisualLineCount()) {
			throw new RangeError('Editor view zone line number is outside the visual line collection');
		}
		return index;
	}
}

class ViewZoneHandle extends AbstractDisposable implements EditorViewZoneHandle {
	constructor(
		private readonly layoutCallback: () => void,
		private readonly removeCallback: () => void,
		private readonly readLayout: () => { readonly top: number; readonly heightInPixels: number } | undefined,
	) {
		super();
	}

	public get top(): number {
		return this.readLayout()?.top ?? 0;
	}

	public get heightInPixels(): number {
		return this.readLayout()?.heightInPixels ?? 0;
	}

	public layout(): void {
		this.assertNotDisposed();
		this.layoutCallback();
	}

	protected disposeCore(): void {
		this.removeCallback();
	}
}
