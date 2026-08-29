import { h } from '../../../../base/browser/dom.js';
import { AbstractDisposable, toDisposable, type IDisposable } from '../../../../base/common/lifecycle.js';
import { isFiniteNumber } from '../../../../base/common/numbers.js';
import { type EditorViewportLayout, ViewLayout } from '../../../common/viewLayout/viewLayout.js';
import { EditorViewPart, type EditorRenderingContext } from '../../view/viewPart.js';

export interface EditorViewZone {
	afterLineIndex: number;
	heightInPixels: number;
	ordinal?: number;
	readonly domNode: HTMLElement;
}

export interface EditorViewZoneHandle extends IDisposable {
	readonly top: number;
	readonly heightInPixels: number;
	layout(): void;
}

/** Owns caller-provided DOM roots placed in reserved vertical editor space. */
export class ViewZones extends EditorViewPart {
	public readonly domNode: HTMLDivElement;
	private readonly viewLayout: ViewLayout;
	private readonly readVisualLineCount: () => number;
	private readonly zones = new Map<string, EditorViewZone>();
	private readonly zoneLayouts = new Map<string, { readonly top: number; readonly heightInPixels: number }>();

	constructor(host: HTMLElement, viewLayout: ViewLayout, readVisualLineCount: () => number) {
		super();
		this.viewLayout = viewLayout;
		this.readVisualLineCount = readVisualLineCount;
		this.domNode = h(host.ownerDocument, 'div');
		this.domNode.className = 'stanza-editor-view-zones';
		this._register(toDisposable(() => {
			for (const zone of this.zones.values()) {
				zone.domNode.remove();
			}
			this.zones.clear();
			this.zoneLayouts.clear();
			this.domNode.remove();
		}));
	}

	public addZone(zone: EditorViewZone): EditorViewZoneHandle {
		this.assertNotDisposed();
		this.validateZone(zone);
		const id = this.viewLayout.addViewZone(zone.afterLineIndex, zone.heightInPixels, zone.ordinal);
		this.zones.set(id, zone);
		zone.domNode.classList.add('stanza-editor-view-zone');
		this.domNode.append(zone.domNode);
		this.layoutZones(this.viewLayout.layout);
		return new ViewZoneHandle(
			() => this.layoutZone(id),
			() => this.removeZone(id),
			() => this.zoneLayouts.get(id),
		);
	}

	public render(context: EditorRenderingContext): void {
		this.layoutZones(context.layout);
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
		this.zoneLayouts.delete(id);
		zone.domNode.remove();
		if (!this.isDisposed) {
			this.layoutZones(this.viewLayout.removeViewZone(id));
		}
	}

	private layoutZones(layout: EditorViewportLayout): void {
		this.zoneLayouts.clear();
		for (const geometry of layout.viewZones ?? []) {
			this.zoneLayouts.set(geometry.id, geometry);
			const zone = this.zones.get(geometry.id);
			if (!zone) {
				continue;
			}
			zone.domNode.style.top = `${geometry.top}px`;
			zone.domNode.style.height = `${geometry.heightInPixels}px`;
		}
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
