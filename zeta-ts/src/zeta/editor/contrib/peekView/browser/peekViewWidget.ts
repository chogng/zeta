import "./media/peekView.css";
import { Disposable, toDisposable } from "../../../../base/common/lifecycle.js";
import { type TextPosition } from "../../../common/core/text.js";
import { type EditorViewport } from "../../../browser/view.js";
import { h } from "../../../../base/browser/dom.js";

/** A lifecycle-safe preview surface anchored to an editor position. */
export class PeekViewWidget extends Disposable {
	readonly element: HTMLElement;
	private readonly body: HTMLDivElement;

	constructor(private readonly viewport: EditorViewport, anchor: TextPosition, title = "Preview") {
		super();
		viewport.textModel.offsetAt(anchor);
		const document = viewport.element.ownerDocument;
		this.element = h(document, "section");
		this.element.className = "stanza-editor-peek-view";
		this.element.hidden = true;
		const header = h(document, "header");
		header.className = "stanza-editor-peek-view-header";
		header.textContent = title;
		this.body = h(document, "div");
		this.body.className = "stanza-editor-peek-view-body";
		this.element.append(header, this.body);
		viewport.element.append(this.element);
		this._register(toDisposable(() => this.element.remove()));
		this._register(viewport.onDidChangeLayout(() => this.position(anchor)));
		this.position(anchor);
	}

	setBody(content: Node): void { this.body.replaceChildren(content); }
	show(): void { this.element.hidden = false; this.position(); }
	hide(): void { this.element.hidden = true; }

	private position(anchor = this.anchor): void {
		if (!anchor) return;
		const coordinates = this.viewport.getPositionContentCoordinates(anchor);
		const scroll = this.viewport.viewportLayout.scrollPosition;
		this.element.style.left = `${Math.max(4, coordinates.left - scroll.left)}px`;
		this.element.style.top = `${Math.max(4, coordinates.top - scroll.top + coordinates.height + 4)}px`;
		this.anchor = anchor;
	}

	private anchor: TextPosition | undefined;
}
