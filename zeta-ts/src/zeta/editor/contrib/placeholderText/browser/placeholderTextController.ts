import "./media/placeholderText.css";
import { createReactiveDom } from "../../../../base/browser/reactiveDom.js";
import { observableFromEvent } from "../../../../base/common/observable.js";
import { Disposable, toDisposable } from "../../../../base/common/lifecycle.js";
import { SyncDescriptor } from "../../../../platform/instantiation/common/instantiation.js";
import { type EditorViewport } from "../../../browser/view.js";
import { CodeEditorContributionInstantiation, registerCodeEditorContribution, type CodeEditorContributionContext } from "../../../browser/widget/codeEditor/codeEditorContributions.js";
import { TextPosition } from "../../../common/core/text.js";

/** Presents a non-editable hint when the shared model is empty. */
export class PlaceholderTextController extends Disposable {
	private readonly element: HTMLDivElement;

	constructor(private readonly viewport: EditorViewport, placeholder: string) {
		super();
		if (typeof placeholder !== "string" || placeholder.trim().length === 0) throw new TypeError("Stanza placeholder text must be non-empty");
		const isEmpty = observableFromEvent(this, viewport.textModel.onDidChange, () => viewport.textModel.length === 0);
		const n = createReactiveDom(viewport.element.ownerDocument);
		const view = this._register(n.div({
			className: "stanza-editor-placeholder-text",
			attributes: { "aria-hidden": "true" },
			properties: { hidden: isEmpty.map(empty => !empty) },
		}, placeholder).toLiveElement());
		this.element = view.element;
		viewport.element.append(this.element);
		this._register(toDisposable(() => this.element.remove()));
		this._register(viewport.onDidChangeLayout(() => this.updateLayout()));
		this.updateLayout();
	}

	private updateLayout(): void {
		const position = this.viewport.getPositionContentCoordinates(TextPosition.at(0, 0));
		this.element.style.left = `${position.left}px`;
		this.element.style.top = `${position.top}px`;
		this.element.style.lineHeight = `${position.height}px`;
	}
}

class PlaceholderTextContribution extends Disposable {
	constructor(context: CodeEditorContributionContext) {
		super();
		if (context.placeholder) this._register(new PlaceholderTextController(context.viewport, context.placeholder));
	}
}

registerCodeEditorContribution({
	id: "editor.contrib.placeholderText",
	instantiation: CodeEditorContributionInstantiation.Eager,
	descriptor: new SyncDescriptor(PlaceholderTextContribution),
});
