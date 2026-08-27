import "./media/fontZoom.css";
import { registerEditorContribution } from "../../../browser/editorExtensions.js";
import { addDisposableListener, stopEvent } from "../../../../base/browser/dom.js";
import { Disposable } from "../../../../base/common/lifecycle.js";
import { type EditorViewport } from "../../../browser/view.js";

export interface FontZoomControllerOptions { readonly baseFontSize?: number; readonly baseLineHeight?: number; readonly initialScale?: number; }

/** Owns per-editor font zoom state and invalidates browser measurements after each change. */
export class FontZoomController extends Disposable {
	private readonly baseLineHeight: number;
	private readonly baseFontSize: number;
	private scale: number;

	constructor(private readonly input: HTMLElement, private readonly viewport: EditorViewport, options: FontZoomControllerOptions = {}) {
		super();
		this.baseLineHeight = readPositive(options.baseLineHeight ?? viewport.viewportLayout.lineHeight, "baseLineHeight");
		this.baseFontSize = readPositive(options.baseFontSize ?? readFontSize(viewport.element), "baseFontSize");
		this.scale = readScale(options.initialScale ?? 1);
		this.apply();
		this._register(addDisposableListener(input, "keydown", event => this.handleKeydown(event), true));
	}

	get zoomScale(): number { return this.scale; }

	setZoomScale(scale: number): void {
		this.scale = readScale(scale);
		this.apply();
	}

	private handleKeydown(event: KeyboardEvent): void {
		if (event.defaultPrevented || event.isComposing || (!event.ctrlKey && !event.metaKey) || event.altKey || event.shiftKey) return;
		if (event.key === "+" || event.key === "=") { stopEvent(event, { immediate: true }); this.setZoomScale(this.scale + 0.1); }
		else if (event.key === "-") { stopEvent(event, { immediate: true }); this.setZoomScale(this.scale - 0.1); }
		else if (event.key === "0") { stopEvent(event, { immediate: true }); this.setZoomScale(1); }
	}

	private apply(): void {
		this.viewport.element.style.setProperty("--stanza-editor-font-scale", String(this.scale));
		this.viewport.element.style.fontSize = `${this.baseFontSize * this.scale}px`;
		this.viewport.setLineHeight(Math.max(1, Math.round(this.baseLineHeight * this.scale)));
		this.viewport.refreshFontMetrics();
		this.viewport.announceAccessibilityStatus(`Editor font size ${Math.round(this.scale * 100)} percent`);
	}
}

function readScale(value: number): number { if (!Number.isFinite(value) || value < 0.5 || value > 3) throw new RangeError("Stanza font zoom scale must be between 0.5 and 3"); return Math.round(value * 10) / 10; }
function readPositive(value: number, name: string): number { if (!Number.isFinite(value) || value <= 0) throw new RangeError(`Stanza ${name} must be positive`); return value; }
function readFontSize(element: HTMLElement): number { return Number.parseFloat(element.ownerDocument.defaultView?.getComputedStyle(element).fontSize ?? ""); }

registerEditorContribution({
	id: "editor.contrib.fontZoom",
	install: context => {
		if (context.kind !== "text") return;
		context.register(new FontZoomController(context.view.element, context.viewport, { baseFontSize: context.options.fontSize, initialScale: context.options.fontZoom?.initialScale }));
	},
});
