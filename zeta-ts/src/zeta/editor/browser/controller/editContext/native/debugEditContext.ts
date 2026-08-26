import { DisposableOwner, type IDisposable } from "../../../../../base/common/lifecycle.js";
import type { NativeEditContextObject } from "./nativeEditContext.js";

const CONTROL_BOUNDS_COLOR = "blue";
const SELECTION_BOUNDS_COLOR = "red";
const CHARACTER_BOUNDS_COLOR = "green";

export interface DebugNativeEditContextOptions {
	readonly ownerDocument: Document;
	readonly enabled?: boolean;
}

/**
 * Development wrapper for inspecting the browser's native EditContext state.
 * It deliberately implements the same narrow local contract as the adapter,
 * so production code never needs to depend on browser-private fields.
 */
export class DebugEditContext extends DisposableOwner implements NativeEditContextObject {
	private debugging: boolean;
	private controlBounds: DOMRect | undefined;
	private selectionBounds: DOMRect | undefined;
	private characterBounds: { readonly rangeStart: number; readonly bounds: readonly DOMRect[] } | undefined;
	private readonly listenerMap = new Map<EventListenerOrEventListenerObject, { readonly type: string; readonly listener: EventListener }>();

	constructor(
		private readonly delegate: NativeEditContextObject,
		private readonly options: DebugNativeEditContextOptions,
	) {
		super();
		this.debugging = options.enabled ?? true;
		this.defer(() => this.clearMarkers());
	}

	get text(): string {
		return this.delegate.text;
	}

	get selectionStart(): number {
		return this.delegate.selectionStart;
	}

	get selectionEnd(): number {
		return this.delegate.selectionEnd;
	}

	updateText(start: number, end: number, text: string): void {
		this.delegate.updateText(start, end, text);
		this.renderDebug();
	}

	updateSelection(start: number, end: number): void {
		this.delegate.updateSelection(start, end);
		this.renderDebug();
	}

	updateControlBounds(bounds: DOMRect): void {
		this.delegate.updateControlBounds?.(bounds);
		this.controlBounds = bounds;
		this.renderDebug();
	}

	updateSelectionBounds(bounds: DOMRect): void {
		this.delegate.updateSelectionBounds?.(bounds);
		this.selectionBounds = bounds;
		this.renderDebug();
	}

	updateCharacterBounds(start: number, bounds: readonly DOMRect[]): void {
		this.delegate.updateCharacterBounds?.(start, bounds);
		this.characterBounds = { rangeStart: start, bounds: [...bounds] };
		this.renderDebug();
	}

	addEventListener(type: string, listener: EventListenerOrEventListenerObject | null, options?: boolean | AddEventListenerOptions): void {
		if (!listener) return;
		const debugListener: EventListener = event => {
			if (this.debugging) this.renderDebug();
			if (typeof listener === "function") listener.call(this, event);
			else listener.handleEvent(event);
		};
		this.listenerMap.set(listener, { type, listener: debugListener });
		this.delegate.addEventListener(type, debugListener, options);
	}

	removeEventListener(type: string, listener: EventListenerOrEventListenerObject | null, options?: boolean | EventListenerOptions): void {
		if (!listener) return;
		const entry = this.listenerMap.get(listener);
		if (!entry) return;
		this.delegate.removeEventListener(entry.type, entry.listener, options);
		this.listenerMap.delete(listener);
	}

	dispatchEvent(event: Event): boolean {
		return this.delegate.dispatchEvent(event);
	}

	startDebugging(): void {
		this.debugging = true;
		this.renderDebug();
	}

	endDebugging(): void {
		this.debugging = false;
		this.clearMarkers();
	}

	renderDebug(): void {
		if (!this.debugging) return;
		this.clearMarkers();
		if (this.controlBounds) this.addMarker(this.controlBounds, CONTROL_BOUNDS_COLOR);
		if (this.selectionBounds) this.addMarker(this.selectionBounds, SELECTION_BOUNDS_COLOR);
		for (const bounds of this.characterBounds?.bounds ?? []) this.addMarker(bounds, CHARACTER_BOUNDS_COLOR);
		this.addTextMarker();
	}

	private readonly markers: IDisposable[] = [];

	private addMarker(bounds: DOMRect, color: string): void {
		const element = this.options.ownerDocument.createElement("div");
		element.className = "stanza-debug-edit-context-marker";
		element.style.position = "fixed";
		element.style.zIndex = "999999999";
		element.style.pointerEvents = "none";
		element.style.outline = `2px solid ${color}`;
		element.style.left = `${bounds.left}px`;
		element.style.top = `${bounds.top}px`;
		element.style.width = `${bounds.width}px`;
		element.style.height = `${bounds.height}px`;
		this.options.ownerDocument.body.append(element);
		this.markers.push({
			dispose: () => element.remove(),
			[Symbol.dispose]: () => element.remove(),
		});
	}

	private addTextMarker(): void {
		const element = this.options.ownerDocument.createElement("div");
		element.className = "stanza-debug-edit-context-marker";
		element.textContent = `${this.text.slice(0, this.selectionStart)}|${this.text.slice(this.selectionEnd)}`;
		element.style.position = "fixed";
		element.style.left = "8px";
		element.style.bottom = "8px";
		element.style.zIndex = "999999999";
		element.style.padding = "4px";
		element.style.background = "white";
		element.style.color = "black";
		element.style.font = "12px monospace";
		element.style.pointerEvents = "none";
		this.options.ownerDocument.body.append(element);
		this.markers.push({
			dispose: () => element.remove(),
			[Symbol.dispose]: () => element.remove(),
		});
	}

	private clearMarkers(): void {
		const markers = this.markers.splice(0);
		for (const marker of markers) marker.dispose();
	}

	protected override disposeCore(): void {
		for (const [listener, entry] of this.listenerMap) {
			this.delegate.removeEventListener(entry.type, entry.listener);
			this.listenerMap.delete(listener);
		}
		super.disposeCore();
	}
}
