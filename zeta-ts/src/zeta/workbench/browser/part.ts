import "./media/part.css";
import { type IDimension } from "../../base/browser/geometry.js";
import { Emitter, type Event } from "../../base/common/event.js";
import { DisposableOwner } from "../../base/common/lifecycle.js";
import { h } from "../../base/browser/dom.js";

/**
 * Base class for a persistent visual region in the browser workbench shell.
 *
 * Parts own their layout constraints. WorkbenchLayout decides topology and
 * delegates the resulting pixel dimensions through `layout`.
 */
export abstract class WorkbenchPart extends DisposableOwner {
	readonly domNode: HTMLElement;
	protected readonly titleDomNode: HTMLDivElement;
	protected readonly contentDomNode: HTMLDivElement;
	private readonly _onDidChangeConstraints = this.own(new Emitter<void>());

	readonly onDidChangeConstraints: Event<void> =
		this._onDidChangeConstraints.event;

	protected constructor(container: HTMLElement, id: string) {
		super();
		const ownerDocument = container.ownerDocument;
		const domNode = h(ownerDocument, "section");
		this.domNode = domNode;
		this.defer(() => domNode.remove());
		domNode.className = `zeta-workbench-part zeta-workbench-${id}`;
		domNode.dataset.part = id;
		this.titleDomNode = h(ownerDocument, "div");
		this.titleDomNode.className = "zeta-workbench-part-title";
		this.contentDomNode = h(ownerDocument, "div");
		this.contentDomNode.className = "zeta-workbench-part-content";
		domNode.append(this.titleDomNode, this.contentDomNode);
		container.append(domNode);
	}

	get minimumWidth(): number { return 0; }
	get maximumWidth(): number { return Number.POSITIVE_INFINITY; }
	get minimumHeight(): number { return 0; }
	get maximumHeight(): number { return Number.POSITIVE_INFINITY; }

	layout(_dimension: IDimension): void {}

	setVisible(visible: boolean): void {
		this.domNode.hidden = !visible;
	}

	/** Notifies the runtime layout after a subclass changes its constraints. */
	protected notifyConstraintsChanged(): void {
		this._onDidChangeConstraints.fire();
	}
}
