import "./media/middleScroll.css";
import { registerTextEditorCapabilityContribution } from "../../../browser/editorExtensions.js";
import { addDisposableListener } from "../../../../base/browser/dom.js";
import { Disposable, toDisposable } from "../../../../base/common/lifecycle.js";
import { type EditorViewport } from "../../../browser/view.js";

/** Implements editor-local middle-button panning without entering pointer selection mode. */
export class MiddleScrollController extends Disposable {
	private active: { readonly pointerId: number; readonly x: number; readonly y: number; readonly left: number; readonly top: number } | undefined;

	constructor(private readonly viewport: EditorViewport) {
		super();
		this._register(addDisposableListener<PointerEvent>(viewport.element, "pointerdown", event => {
			if (event.button !== 1) return;
			event.preventDefault();
			viewport.element.setPointerCapture?.(event.pointerId);
			const scroll = viewport.viewportLayout.scrollPosition;
			this.active = { pointerId: event.pointerId, x: event.clientX, y: event.clientY, left: scroll.left, top: scroll.top };
			viewport.element.classList.add("middle-scrolling");
		}));
		this._register(addDisposableListener<PointerEvent>(viewport.element, "pointermove", event => {
			if (!this.active || this.active.pointerId !== event.pointerId) return;
			event.preventDefault();
			viewport.scrollTo({ left: this.active.left - event.clientX + this.active.x, top: this.active.top - event.clientY + this.active.y });
		}));
		const end = (event: PointerEvent): void => { if (this.active?.pointerId !== event.pointerId) return; this.active = undefined; viewport.element.classList.remove("middle-scrolling"); viewport.element.releasePointerCapture?.(event.pointerId); };
		this._register(addDisposableListener<PointerEvent>(viewport.element, "pointerup", end));
		this._register(addDisposableListener<PointerEvent>(viewport.element, "pointercancel", end));
		this._register(toDisposable(() => { this.active = undefined; viewport.element.classList.remove("middle-scrolling"); }));
	}
}

registerTextEditorCapabilityContribution({
	id: "editor.contrib.middleScroll",
	install: context => {
		if (context.kind !== "text") return;
		context.register(new MiddleScrollController(context.viewport));
	},
});
