import "./media/placeholderText.css";
import { createReactiveDom } from "../../../../base/browser/reactiveDom.js";
import { observableFromEvent } from "../../../../base/common/observable.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { type EditorViewport } from "../../../browser/view/editorViewport.js";
import { registerCodeEditorPlaceholderFactory } from "../../../browser/widget/codeEditor/codeEditorWidget.js";
import { TextPosition } from "../../../common/core/text.js";

/** Presents a non-editable hint when the shared model is empty. */
export class PlaceholderTextController extends DisposableOwner {
	private readonly element: HTMLDivElement;

	constructor(private readonly viewport: EditorViewport, placeholder: string) {
		super();
		if (typeof placeholder !== "string" || placeholder.trim().length === 0) throw new TypeError("Aster placeholder text must be non-empty");
		const isEmpty = observableFromEvent(this, viewport.textModel.onDidChange, () => viewport.textModel.length === 0);
		const n = createReactiveDom(viewport.element.ownerDocument);
		const view = this.own(n.div({
			className: "aster-editor-placeholder-text",
			attributes: { "aria-hidden": "true" },
			properties: { hidden: isEmpty.map(empty => !empty) },
		}, placeholder).toLiveElement());
		this.element = view.element;
		viewport.element.append(this.element);
		this.defer(() => this.element.remove());
		this.own(viewport.onDidChangeLayout(() => this.updateLayout()));
		this.updateLayout();
	}

	private updateLayout(): void {
		const position = this.viewport.getPositionContentCoordinates(TextPosition.at(0, 0));
		this.element.style.left = `${position.left}px`;
		this.element.style.top = `${position.top}px`;
		this.element.style.lineHeight = `${position.height}px`;
	}
}

registerCodeEditorPlaceholderFactory((viewport, placeholder) => new PlaceholderTextController(viewport, placeholder));
