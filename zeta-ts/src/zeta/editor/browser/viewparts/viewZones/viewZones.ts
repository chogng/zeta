import { addDisposableListener, h } from '../../../../base/browser/dom.js';
import { AbstractDisposable, DisposableMap, toDisposable, type IDisposable } from '../../../../base/common/lifecycle.js';
import { isFiniteNumber } from '../../../../base/common/numbers.js';
import { type EditorViewportLayout, ViewLayout } from '../../../common/viewLayout/viewLayout.js';
import { type IViewZone, type IViewZoneChangeAccessor } from '../../editorBrowser.js';
import { EditorViewPart, PartFingerprint, PartFingerprints, type EditorRenderingContext } from '../../view/viewPart.js';

export type EditorViewZone = IViewZone;

export interface EditorViewZoneHandle extends IDisposable {
	readonly top: number;
	readonly heightInPixels: number;
	layout(): void;
}

interface ViewZonesOptions {
	readonly host: HTMLElement;
	readonly viewLayout: ViewLayout;
	readonly readVisualLineCount: () => number;
	readonly readContentLeft: () => number;
	readonly readContentWidth: () => number;
	readonly setMinimumContentWidth: (width: number) => void;
}

/** Owns caller-provided DOM roots placed in reserved vertical editor space. */
export class ViewZones extends EditorViewPart {
	public readonly domNode: HTMLDivElement;
	public readonly marginDomNode: HTMLDivElement;
	private readonly viewLayout: ViewLayout;
	private readonly readVisualLineCount: () => number;
	private readonly zones = new Map<string, EditorViewZone>();
	private readonly mouseDownListeners = this._register(new DisposableMap<string>());
	private readonly zoneLayouts = new Map<string, { readonly top: number; readonly heightInPixels: number }>();

	constructor(private readonly options: ViewZonesOptions) {
		super();
		this.viewLayout = options.viewLayout;
		this.readVisualLineCount = options.readVisualLineCount;
		this.domNode = h(options.host.ownerDocument, 'div');
		PartFingerprints.write(this.domNode, PartFingerprint.ViewZones);
		this.domNode.className = 'stanza-editor-view-zones';
		this.domNode.setAttribute('role', 'presentation');
		this.domNode.setAttribute('aria-hidden', 'true');
		this.marginDomNode = h(options.host.ownerDocument, 'div');
		PartFingerprints.write(this.marginDomNode, PartFingerprint.ViewZones);
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

	private addZoneData(zone: EditorViewZone): string {
		this.validateZone(zone);
		const id = this.viewLayout.addViewZone(zone.afterLineIndex, zone.heightInPixels, zone.ordinal);
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
		this.layoutZones(this.viewLayout.changeViewZone(id, zone.afterLineIndex, zone.heightInPixels, zone.ordinal));
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
		for (const zone of this.zones.values()) width = Math.max(width, zone.minWidthInPixels ?? 0);
		this.options.setMinimumContentWidth(width);
	}

	private validateZone(zone: EditorViewZone): void {
		if (!zone || !(zone.domNode instanceof this.domNode.ownerDocument.defaultView!.HTMLElement)) {
			throw new TypeError('Editor view zone requires a DOM root from the editor document');
		}
		if (!Number.isSafeInteger(zone.afterLineIndex) || zone.afterLineIndex < -1 || zone.afterLineIndex >= this.readVisualLineCount()) {
			throw new RangeError('Editor view zone line index is outside the visual line collection');
		}
		if (!isFiniteNumber(zone.heightInPixels) || zone.heightInPixels <= 0) {
			throw new RangeError('Editor view zone height must be finite and positive');
		}
		if (zone.ordinal !== undefined && !isFiniteNumber(zone.ordinal)) {
			throw new RangeError('Editor view zone ordinal must be finite');
		}
		if (zone.minWidthInPixels !== undefined && (!isFiniteNumber(zone.minWidthInPixels) || zone.minWidthInPixels < 0)) {
			throw new RangeError('Editor view zone minimum width must be finite and non-negative');
		}
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
