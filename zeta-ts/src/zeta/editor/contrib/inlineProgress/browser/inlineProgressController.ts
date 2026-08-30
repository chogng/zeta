import "./media/inlineProgress.css";
import { registerTextEditorCapabilityContribution } from "../../../browser/editorExtensions.js";
import { createReactiveDom } from "../../../../base/browser/reactiveDom.js";
import { Disposable, toDisposable } from "../../../../base/common/lifecycle.js";
import { observableValue } from "../../../../base/common/observable.js";
import { type EditorViewport } from "../../../browser/view.js";

/** Provides a reusable inline progress presentation for asynchronous editor requests. */
export class InlineProgressController extends Disposable {
	private readonly element: HTMLDivElement;
	private readonly label = observableValue(this, "");
	private active = 0;

	constructor(private readonly viewport: EditorViewport) {
		super();
		const n = createReactiveDom(viewport.element.ownerDocument);
		const view = this._register(n.div({
			className: "stanza-editor-inline-progress",
			attributes: { role: "status", "aria-live": "polite" },
			properties: { hidden: this.label.map(label => label.length === 0) },
		}, this.label).toLiveElement());
		this.element = view.element;
		viewport.element.append(this.element);
		this._register(toDisposable(() => this.element.remove()));
	}

	async run<T>(label: string, task: Promise<T>): Promise<T> {
		if (typeof label !== "string" || label.trim().length === 0) throw new TypeError("Stanza inline progress label must be non-empty");
		const token = ++this.active;
		this.label.set(label.trim());
		try { return await task; } finally { if (token === this.active) this.label.set(""); }
	}
}

registerTextEditorCapabilityContribution({
	id: "editor.contrib.inlineProgress",
	install: context => {
		if (context.kind !== "text") return;
		context.register(new InlineProgressController(context.viewport));
	},
});
