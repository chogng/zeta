import "./media/floatingMenu.css";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { type EditorViewport } from "../../../browser/view.js";
import { type EditorSelectionController } from "../../../common/cursor/editorSelectionController.js";
import { h } from "../../../../base/browser/dom.js";

export interface FloatingMenuAction { readonly label: string; readonly run: () => void | Promise<void>; }

/** Provides an opt-in selection-anchored action menu for embedding hosts. */
export class FloatingMenuController extends DisposableOwner {
	private readonly element: HTMLDivElement;

	constructor(private readonly viewport: EditorViewport, private readonly selections: EditorSelectionController, actions: readonly FloatingMenuAction[] = [], private readonly onError: (error: unknown) => void = error => console.error("Stanza floating menu failed", error)) {
		super();
		if (viewport.textModel !== selections.textModel) throw new TypeError("Stanza floating menu dependencies must share a text model");
		this.element = h(viewport.element.ownerDocument, "div");
		this.element.className = "stanza-editor-floating-menu";
		this.element.hidden = true;
		viewport.element.append(this.element);
		this.defer(() => this.element.remove());
		this.own(selections.onDidChange(() => this.update(actions)));
		this.own(viewport.onDidChangeLayout(() => this.position()));
		this.update(actions);
	}

	private update(actions: readonly FloatingMenuAction[]): void {
		const selection = this.selections.selections.primary;
		if (selection.range.empty || actions.length === 0) { this.element.hidden = true; return; }
		this.element.replaceChildren(...actions.map(action => {
			const button = h(this.element.ownerDocument, "button");
			button.type = "button";
			button.textContent = action.label;
			button.addEventListener("click", () => { try { const result = action.run(); if (result && typeof (result as Promise<void>).then === "function") void (result as Promise<void>).catch(this.onError); } catch (error) { this.onError(error); } });
			return button;
		}));
		this.element.hidden = false;
		this.position();
	}

	private position(): void {
		if (this.element.hidden) return;
		const coordinates = this.viewport.getPositionContentCoordinates(this.selections.selections.primary.range.start);
		const scroll = this.viewport.viewportLayout.scrollPosition;
		this.element.style.left = `${Math.max(4, coordinates.left - scroll.left)}px`;
		this.element.style.top = `${Math.max(4, coordinates.top - scroll.top - 30)}px`;
	}
}
