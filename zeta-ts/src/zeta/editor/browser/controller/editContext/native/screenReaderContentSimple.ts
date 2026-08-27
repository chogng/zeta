import { addDisposableListener, h } from "../../../../../base/browser/dom.js";
import { Disposable, toDisposable } from "../../../../../base/common/lifecycle.js";
import { clampScreenReaderOffset, domOffsetAtPoint, domPointAtOffset, modelOffsetAtContentOffset, type NativeScreenReaderContent, type ScreenReaderContentLayout, type ScreenReaderContentState } from "./screenReaderUtils.js";

/** Plain-text screen-reader projection used by the native EditContext. */
export class SimpleScreenReaderContent extends Disposable implements NativeScreenReaderContent {
	readonly element: HTMLDivElement;
	protected state: ScreenReaderContentState | undefined;

	constructor(private readonly host: HTMLElement) {
		super();
		this.element = h(host.ownerDocument, "div");
		this.element.className = "stanza-native-screen-reader-content";
		this.element.setAttribute("aria-hidden", "true");
		host.append(this.element);
		this._register(toDisposable(() => this.element.remove()));
		this._register(addDisposableListener(this.element, "mousedown", event => event.preventDefault()));
	}

	getState(): ScreenReaderContentState | undefined {
		return this.state;
	}

	sync(state: ScreenReaderContentState): void {
		this.state = state;
		this.renderText(state.text, state);
		this.element.setAttribute("aria-hidden", "false");
		this.setDomSelection(state);
	}

	clear(): void {
		this.state = undefined;
		this.element.replaceChildren();
		this.element.scrollTop = 0;
		this.resetLayout();
		this.element.setAttribute("aria-hidden", "true");
	}

	layout(layout: ScreenReaderContentLayout): void {
		this.element.style.left = `${layout.left}px`;
		this.element.style.top = `${layout.top}px`;
		this.element.style.width = `${layout.width}px`;
		this.element.style.height = `${layout.height}px`;
		this.element.style.lineHeight = `${layout.lineHeight}px`;
		this.element.scrollTop = Math.max(0, layout.scrollTop);
	}

	readSelection(): { readonly anchorOffset: number; readonly activeOffset: number } | undefined {
		const state = this.state;
		if (!state) return undefined;
		const selection = this.host.ownerDocument.getSelection();
		if (!selection) return undefined;
		const anchorOffset = domOffsetAtPoint(this.element, selection.anchorNode, selection.anchorOffset);
		const activeOffset = domOffsetAtPoint(this.element, selection.focusNode, selection.focusOffset);
		if (anchorOffset === undefined || activeOffset === undefined) return undefined;
		const backward = selection.direction === "backward";
		return {
			anchorOffset: modelOffsetAtContentOffset(state, clampScreenReaderOffset(anchorOffset, state.text.length), backward ? "end" : "start"),
			activeOffset: modelOffsetAtContentOffset(state, clampScreenReaderOffset(activeOffset, state.text.length), backward ? "start" : "end"),
		};
	}

	setIgnoreSelectionChange(): void {
		this.selectionChangeIgnoreUntil = Date.now() + 100;
	}

	shouldIgnoreSelectionChange(): boolean {
		return Date.now() < this.selectionChangeIgnoreUntil;
	}

	protected renderText(text: string, _state: ScreenReaderContentState): void {
		if (this.element.textContent === text && this.element.firstChild?.nodeType === 3) return;
		this.element.replaceChildren(this.element.ownerDocument.createTextNode(text));
	}

	protected setDomSelection(state: ScreenReaderContentState): void {
		const selection = this.host.ownerDocument.getSelection();
		const anchor = domPointAtOffset(this.element, state.anchorOffset);
		const active = domPointAtOffset(this.element, state.activeOffset);
		if (!selection || !anchor || !active) return;
		this.setIgnoreSelectionChange();
		selection.setBaseAndExtent(anchor.node, anchor.offset, active.node, active.offset);
	}

	private resetLayout(): void {
		this.element.style.removeProperty("left");
		this.element.style.removeProperty("top");
		this.element.style.removeProperty("width");
		this.element.style.removeProperty("height");
		this.element.style.removeProperty("line-height");
	}

	private selectionChangeIgnoreUntil = 0;
}
